//! Underwater rendering effect for Rose Online client
//!
//! This module implements underwater post-processing effects including:
//! - Volumetric fog using Beer-Lambert law
//! - Depth-based color absorption (red absorbed fastest, blue penetrates)
//! - Procedural caustics effect
//!
//! Based on the implementation plan in `plans/underwater-rendering-fix.md`

use bevy::{
    asset::{load_internal_asset, weak_handle, Handle},
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::Camera,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_graph::{
            NodeRunError, RenderGraphApp as _, RenderGraphContext, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, DynamicUniformBuffer, FilterMode, FragmentState,
            Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, Shader,
            ShaderStages, ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines,
            TextureFormat, TextureSampleType,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::{ExtractedView, ViewTarget},
        Render, RenderApp, RenderSet,
    },
    time::Time,
    transform::components::GlobalTransform,
    utils::default,
};

use crate::resources::WaterSettings;

/// Shader handle for the underwater effect shader
pub const UNDERWATER_EFFECT_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("a1b2c3d4-e5f6-7890-abcd-ef1234567890");

// =============================================================================
// Main World Components and Resources
// =============================================================================

/// Resource for underwater effect settings
#[derive(Resource, Clone, Reflect, ExtractResource)]
#[reflect(Resource, Default)]
pub struct UnderwaterSettings {
    /// Fog density for underwater effect (higher = denser fog)
    pub fog_density: f32,
    /// Base fog color when underwater (RGBA)
    pub fog_color: Vec4,
    /// Maximum visibility distance underwater
    pub max_visibility: f32,
    /// Light absorption coefficients per channel (R, G, B)
    /// Based on real-world water absorption:
    /// Red: ~0.5 m^-1, Green: ~0.05 m^-1, Blue: ~0.01 m^-1
    pub absorption_coefficients: Vec3,
    /// Caustics intensity (0.0 = off, 1.0 = full)
    pub caustics_intensity: f32,
    /// Caustics pattern scale
    pub caustics_scale: f32,
    /// Caustics animation speed
    pub caustics_speed: f32,
    /// Whether underwater effects are enabled
    pub enabled: bool,
}

impl Default for UnderwaterSettings {
    fn default() -> Self {
        Self {
            // Exponential fog density - tuned for underwater visibility
            fog_density: 0.015,
            // Deep blue-green underwater color
            fog_color: Vec4::new(0.05, 0.15, 0.25, 1.0),
            // Maximum visibility ~100 meters underwater
            max_visibility: 100.0,
            // Realistic water absorption coefficients
            // Red is absorbed fastest, blue penetrates deepest
            absorption_coefficients: Vec3::new(0.5, 0.05, 0.01),
            // Caustics settings
            caustics_intensity: 0.3,
            caustics_scale: 0.1,
            caustics_speed: 0.5,
            enabled: true,
        }
    }
}

/// Component to track camera underwater state
/// Uses ExtractComponent derive to automatically extract to render world when underwater
#[derive(Component, Default, Reflect, Clone, ExtractComponent)]
#[reflect(Component, Default, Clone)]
#[extract_component_filter(With<Camera>)]
pub struct CameraUnderwaterState {
    /// Whether the camera is currently underwater
    pub is_underwater: bool,
    /// Y coordinate of the water surface
    pub water_surface_y: f32,
    /// How deep below the surface the camera is (0.0 if above water)
    pub depth_below_surface: f32,
}

// =============================================================================
// Render World Resources and Pipeline
// =============================================================================

/// GPU pipeline data for the underwater effect
#[derive(Resource)]
pub struct UnderwaterEffectPipeline {
    /// Bind group layout for the underwater effect
    bind_group_layout: BindGroupLayout,
    /// Sampler for reading the source texture
    source_sampler: Sampler,
}

/// A key that uniquely identifies an underwater effect pipeline
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnderwaterEffectPipelineKey {
    /// The format of the source and destination textures
    texture_format: TextureFormat,
}

/// Component attached to cameras in the render world storing the pipeline ID
#[derive(Component, Deref, DerefMut)]
pub struct UnderwaterEffectPipelineId(CachedRenderPipelineId);

/// The on-GPU version of the underwater settings
#[derive(ShaderType)]
pub struct UnderwaterEffectUniform {
    /// Whether camera is underwater (0.0 or 1.0)
    is_underwater: f32,
    /// Water surface Y coordinate
    water_surface_y: f32,
    /// Fog density
    fog_density: f32,
    /// Maximum visibility distance
    max_visibility: f32,
    /// Fog color (RGBA)
    fog_color: Vec4,
    /// Light absorption coefficients (RGB)
    absorption: Vec3,
    /// Caustics intensity
    caustics_intensity: f32,
    /// Caustics scale
    caustics_scale: f32,
    /// Caustics speed
    caustics_speed: f32,
    /// Time for animation
    time: f32,
    /// Padding for alignment
    _padding: Vec3,
}

impl Default for UnderwaterEffectUniform {
    fn default() -> Self {
        Self {
            is_underwater: 0.0,
            water_surface_y: 0.0,
            fog_density: 0.015,
            max_visibility: 100.0,
            fog_color: Vec4::new(0.05, 0.15, 0.25, 1.0),
            absorption: Vec3::new(0.5, 0.05, 0.01),
            caustics_intensity: 0.3,
            caustics_scale: 0.1,
            caustics_speed: 0.5,
            time: 0.0,
            _padding: Vec3::ZERO,
        }
    }
}

/// Resource storing uniform buffers for underwater effects
#[derive(Resource, Deref, DerefMut, Default)]
pub struct UnderwaterEffectUniformBuffers {
    #[deref]
    buffer: DynamicUniformBuffer<UnderwaterEffectUniform>,
}

/// Component storing the uniform buffer offset for a view
#[derive(Component, Deref, DerefMut)]
pub struct UnderwaterEffectUniformOffset(u32);

// =============================================================================
// Render Node
// =============================================================================

/// The render node that runs the underwater effect
#[derive(Default)]
pub struct UnderwaterEffectNode;

impl ViewNode for UnderwaterEffectNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static UnderwaterEffectPipelineId,
        &'static CameraUnderwaterState,
        &'static UnderwaterEffectUniformOffset,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view_target, pipeline_id, _underwater_state, uniform_offset): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let underwater_pipeline = world.resource::<UnderwaterEffectPipeline>();
        let underwater_uniform_buffers = world.resource::<UnderwaterEffectUniformBuffers>();

        // Get the pipeline
        let Some(pipeline) = pipeline_cache.get_render_pipeline(**pipeline_id) else {
            return Ok(());
        };

        // Get the uniform buffer binding
        let Some(uniform_buffer_binding) = underwater_uniform_buffers.buffer.binding() else {
            return Ok(());
        };

        // Use post_process_write for full-screen pass
        let post_process = view_target.post_process_write();

        let pass_descriptor = RenderPassDescriptor {
            label: Some("underwater_effect pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        // Create bind group with source texture, sampler, and uniforms
        let bind_group = render_context.render_device().create_bind_group(
            Some("underwater_effect bind group"),
            &underwater_pipeline.bind_group_layout,
            &BindGroupEntries::sequential((
                post_process.source,
                &underwater_pipeline.source_sampler,
                uniform_buffer_binding,
            )),
        );

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[**uniform_offset]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

// =============================================================================
// Plugin
// =============================================================================

/// Plugin that adds underwater rendering effects
pub struct UnderwaterEffectPlugin;

impl Plugin for UnderwaterEffectPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            UNDERWATER_EFFECT_SHADER_HANDLE,
            "shaders/underwater_effect.wgsl",
            Shader::from_wgsl
        );

        // Register types and extract resources to render world
        app.register_type::<UnderwaterSettings>()
            .register_type::<CameraUnderwaterState>()
            .init_resource::<UnderwaterSettings>()
            .add_plugins((
                ExtractResourcePlugin::<UnderwaterSettings>::default(),
                ExtractComponentPlugin::<CameraUnderwaterState>::default(),
            ));

        // Add the underwater detection system
        app.add_systems(Update, detect_underwater_camera);

        // Setup render app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<UnderwaterEffectPipeline>>()
            .init_resource::<UnderwaterEffectUniformBuffers>()
            .add_systems(
                Render,
                (
                    prepare_underwater_effect_pipelines,
                    prepare_underwater_effect_uniforms,
                )
                    .in_set(RenderSet::Prepare),
            )
            .add_render_graph_node::<ViewNodeRunner<UnderwaterEffectNode>>(
                Core3d,
                Node3d::PostProcessing, // Run after main post-processing
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<UnderwaterEffectPipeline>();
    }
}

impl FromWorld for UnderwaterEffectPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Create bind group layout
        let bind_group_layout = render_device.create_bind_group_layout(
            Some("underwater_effect bind group layout"),
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    // Source texture
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // Source sampler
                    sampler(SamplerBindingType::Filtering),
                    // Uniform buffer
                    uniform_buffer::<UnderwaterEffectUniform>(true),
                ),
            ),
        );

        // Create sampler
        let source_sampler = render_device.create_sampler(&SamplerDescriptor {
            mipmap_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..default()
        });

        UnderwaterEffectPipeline {
            bind_group_layout,
            source_sampler,
        }
    }
}

impl SpecializedRenderPipeline for UnderwaterEffectPipeline {
    type Key = UnderwaterEffectPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("underwater_effect".into()),
            layout: vec![self.bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: UNDERWATER_EFFECT_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "fragment_main".into(),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: default(),
            depth_stencil: None,
            multisample: default(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: false,
        }
    }
}

// =============================================================================
// Systems
// =============================================================================

/// System to detect when camera is underwater
pub fn detect_underwater_camera(
    mut camera_query: Query<(&GlobalTransform, &mut CameraUnderwaterState), With<Camera>>,
    water_settings: Res<WaterSettings>,
    underwater_settings: Res<UnderwaterSettings>,
) {
    // Skip if underwater effects are disabled
    if !underwater_settings.enabled {
        return;
    }

    // Get water surface Y from water settings
    // The water surface is typically at a fixed height in the zone
    let water_surface_y = water_settings.water_surface_y;

    for (transform, mut underwater_state) in camera_query.iter_mut() {
        let camera_y = transform.translation().y;
        let is_underwater = camera_y < water_surface_y;
        
        underwater_state.is_underwater = is_underwater;
        underwater_state.water_surface_y = water_surface_y;
        underwater_state.depth_below_surface = if is_underwater {
            (water_surface_y - camera_y).max(0.0)
        } else {
            0.0
        };
    }
}

/// Prepares underwater effect pipelines for views that need them
pub fn prepare_underwater_effect_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UnderwaterEffectPipeline>>,
    underwater_pipeline: Res<UnderwaterEffectPipeline>,
    views: Query<(Entity, &ExtractedView), With<CameraUnderwaterState>>,
) {
    for (entity, view) in views.iter() {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &underwater_pipeline,
            UnderwaterEffectPipelineKey {
                texture_format: if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
            },
        );

        commands
            .entity(entity)
            .insert(UnderwaterEffectPipelineId(pipeline_id));
    }
}

/// Prepares and uploads underwater effect uniforms to the GPU
pub fn prepare_underwater_effect_uniforms(
    mut commands: Commands,
    mut uniform_buffers: ResMut<UnderwaterEffectUniformBuffers>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    time: Res<Time>,
    underwater_settings: Res<UnderwaterSettings>,
    views: Query<(Entity, &CameraUnderwaterState)>,
) {
    uniform_buffers.clear();

    let current_time = time.elapsed_secs();

    for (view_entity, underwater_state) in views.iter() {
        let uniform = UnderwaterEffectUniform {
            is_underwater: if underwater_state.is_underwater { 1.0 } else { 0.0 },
            water_surface_y: underwater_state.water_surface_y,
            fog_density: underwater_settings.fog_density,
            max_visibility: underwater_settings.max_visibility,
            fog_color: underwater_settings.fog_color,
            absorption: underwater_settings.absorption_coefficients,
            caustics_intensity: underwater_settings.caustics_intensity,
            caustics_scale: underwater_settings.caustics_scale,
            caustics_speed: underwater_settings.caustics_speed,
            time: current_time,
            _padding: Vec3::ZERO,
        };

        let offset = uniform_buffers.buffer.push(&uniform);
        commands
            .entity(view_entity)
            .insert(UnderwaterEffectUniformOffset(offset));
    }

    // Upload to GPU
    uniform_buffers.buffer.write_buffer(&render_device, &render_queue);
}
