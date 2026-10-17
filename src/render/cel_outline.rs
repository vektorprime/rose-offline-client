//! Screen-space cel outlines.
//!
//! [`CelOutline`] on a camera enables a fullscreen pass (`shaders/cel_outline.wgsl`)
//! that finds silhouettes and creases in the prepass depth and normal textures and
//! alpha-blends a line color into the camera's main texture.
//!
//! The pass runs in the main pass, after `main_opaque_pass_3d` and before
//! `main_transparent_pass_3d` (the slot Bevy's own `render_sky` and transmission passes
//! use), and draws into the current main texture through
//! `ViewTarget::get_color_attachment()`. It never calls `post_process_write()`, so it
//! takes no part in the post-process ping-pong ordering (see
//! `pitfalls/postprocess-pass-race.md`).
//!
//! Requirements on the camera: `DepthPrepass` and `NormalPrepass` (views without both
//! prepass textures are skipped), and `Msaa::Off` (multisampled prepass textures are not
//! supported; such views are skipped too). The terrain writes the prepass through
//! `terrain_prepass.wgsl`; the ROSE object materials write the outline class into the
//! normal alpha (`rose_object_extension.wgsl`).

use bevy::{
    asset::{load_internal_asset, weak_handle},
    camera::{MainPassResolutionOverride, Viewport},
    core_pipeline::{
        core_3d::{main_opaque_pass_3d, main_transparent_pass_3d},
        prepass::ViewPrepassTextures,
        Core3d, Core3dSystems, FullscreenShader,
    },
    ecs::query::With,
    math::Vec4,
    prelude::{
        App, Commands, Component, Entity, FromWorld, Handle, IntoScheduleConfigs, Msaa, Plugin,
        Query, Res, ResMut, Resource, Shader, World,
    },
    render::{
        camera::ExtractedCamera,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_resource::{
            binding_types::{texture_2d, texture_depth_2d, uniform_buffer},
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            BlendComponent, BlendFactor, BlendOperation, BlendState, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, PipelineCache, RenderPassDescriptor,
            RenderPipelineDescriptor, ShaderStages, ShaderType, SpecializedRenderPipeline,
            SpecializedRenderPipelines, TextureFormat, TextureSampleType, VertexState,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
        Render, RenderApp, RenderSystems,
    },
};

pub const CEL_OUTLINE_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("c3e1a7d2-5b4f-4e8a-0000-000000000002");

/// Cel outline settings of a camera. Adding it enables the outline pass for that camera;
/// removing it disables the pass. Matches `CelOutlineSettings` in `cel_outline.wgsl`.
/// On the main camera it is owned by `apply_cel_shading_settings_system`
/// (from `CelShadingSettings`); do not insert it elsewhere.
#[derive(Component, Clone, Copy, Debug, PartialEq, ExtractComponent, ShaderType)]
pub struct CelOutline {
    /// Line color in linear RGB (as written to the HDR target); alpha is its opacity.
    pub color: Vec4,
    /// Line width in pixels at [`Self::reference_height`]; scaled with the vertical
    /// resolution and rounded to whole pixels (minimum 1).
    pub thickness: f32,
    /// Silhouette threshold: how much farther (relative to this pixel's view distance)
    /// a neighbor must be, beyond what the pixel's own surface slope explains.
    pub depth_threshold: f32,
    /// Crease threshold: 1 - cos(angle) between neighboring normals (0.3 = ~45 deg).
    pub normal_threshold: f32,
    /// Multiplier on `depth_threshold` for alpha-masked parts (foliage), which only get
    /// silhouettes against a much farther background.
    pub masked_depth_scale: f32,
    /// View distance (m) where lines start fading out, and where they are gone.
    pub fade_start: f32,
    pub fade_end: f32,
    /// Vertical resolution `thickness` is given for.
    pub reference_height: f32,
    pub _padding: f32,
}

impl Default for CelOutline {
    fn default() -> Self {
        Self {
            color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            thickness: 2.5,
            depth_threshold: 0.04,
            normal_threshold: 0.3,
            masked_depth_scale: 5.0,
            fade_start: 80.0,
            fade_end: 300.0,
            reference_height: 1080.0,
            _padding: 0.0,
        }
    }
}

pub struct CelOutlinePlugin;

impl Plugin for CelOutlinePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            CEL_OUTLINE_SHADER_HANDLE,
            "shaders/cel_outline.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins((
            ExtractComponentPlugin::<CelOutline>::default(),
            UniformComponentPlugin::<CelOutline>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<CelOutlinePipeline>>()
            .add_systems(
                Render,
                (
                    prepare_cel_outline_pipelines.in_set(RenderSystems::Prepare),
                    prepare_cel_outline_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            )
            .add_systems(
                Core3d,
                cel_outline_pass
                    .in_set(Core3dSystems::MainPass)
                    .after(main_opaque_pass_3d)
                    .before(main_transparent_pass_3d),
            );
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<CelOutlinePipeline>();
        }
    }
}

#[derive(Resource)]
pub struct CelOutlinePipeline {
    layout: BindGroupLayoutDescriptor,
    fullscreen_vertex: VertexState,
}

impl FromWorld for CelOutlinePipeline {
    fn from_world(world: &mut World) -> Self {
        let layout = BindGroupLayoutDescriptor::new(
            "cel_outline_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    // Prepass depth
                    texture_depth_2d(),
                    // Prepass normals (xyz) + outline class (a)
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<CelOutline>(true),
                ),
            ),
        );

        Self {
            layout,
            fullscreen_vertex: world.resource::<FullscreenShader>().to_vertex_state(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CelOutlinePipelineKey {
    /// The view's main texture format (HDR or LDR), from `ExtractedView::target_format`.
    target_format: TextureFormat,
}

impl SpecializedRenderPipeline for CelOutlinePipeline {
    type Key = CelOutlinePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("cel_outline_pipeline".into()),
            layout: vec![self.layout.clone()],
            vertex: self.fullscreen_vertex.clone(),
            fragment: Some(FragmentState {
                shader: CEL_OUTLINE_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format: key.target_format,
                    // Line color over the scene; the target's alpha is left as is.
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            ..Default::default()
        }
    }
}

#[derive(Component)]
pub struct CelOutlinePipelineId(CachedRenderPipelineId);

#[derive(Component)]
pub struct CelOutlineBindGroup(BindGroup);

fn prepare_cel_outline_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<CelOutlinePipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<CelOutlinePipeline>>,
    views: Query<(Entity, &ExtractedView, Option<&Msaa>), With<CelOutline>>,
) {
    for (entity, view, msaa) in &views {
        if msaa.is_some_and(|msaa| msaa.samples() > 1) {
            commands.entity(entity).remove::<CelOutlinePipelineId>();
            continue;
        }

        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            CelOutlinePipelineKey {
                target_format: view.target_format,
            },
        );
        commands
            .entity(entity)
            .insert(CelOutlinePipelineId(pipeline_id));
    }
}

/// Rebuilt every frame: the prepass texture views change with the window size.
fn prepare_cel_outline_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<CelOutlinePipeline>,
    view_uniforms: Res<ViewUniforms>,
    outline_uniforms: Res<ComponentUniforms<CelOutline>>,
    views: Query<(Entity, &ViewPrepassTextures), With<CelOutline>>,
) {
    let (Some(view_binding), Some(outline_binding)) = (
        view_uniforms.uniforms.binding(),
        outline_uniforms.uniforms().binding(),
    ) else {
        return;
    };

    for (entity, prepass_textures) in &views {
        let (Some(depth_view), Some(normal_view)) =
            (prepass_textures.depth_view(), prepass_textures.normal_view())
        else {
            commands.entity(entity).remove::<CelOutlineBindGroup>();
            continue;
        };

        let bind_group = render_device.create_bind_group(
            "cel_outline_bind_group",
            &pipeline_cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((
                depth_view,
                normal_view,
                view_binding.clone(),
                outline_binding.clone(),
            )),
        );
        commands
            .entity(entity)
            .insert(CelOutlineBindGroup(bind_group));
    }
}

fn cel_outline_pass(
    view: ViewQuery<
        (
            &ExtractedCamera,
            &ViewTarget,
            &CelOutlinePipelineId,
            &CelOutlineBindGroup,
            &ViewUniformOffset,
            &DynamicUniformIndex<CelOutline>,
            Option<&MainPassResolutionOverride>,
        ),
        With<CelOutline>,
    >,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (
        camera,
        view_target,
        pipeline_id,
        bind_group,
        view_uniform_offset,
        outline_uniform_index,
        resolution_override,
    ) = view.into_inner();

    let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
        return;
    };

    let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("cel_outline_pass"),
        color_attachments: &[Some(view_target.get_color_attachment())],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

    if let Some(viewport) =
        Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
    {
        render_pass.set_camera_viewport(&viewport);
    }

    render_pass.set_render_pipeline(pipeline);
    render_pass.set_bind_group(
        0,
        &bind_group.0,
        &[view_uniform_offset.offset, outline_uniform_index.index()],
    );
    render_pass.draw(0..3, 0..1);
}
