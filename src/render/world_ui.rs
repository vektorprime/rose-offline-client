use std::{cmp::Ordering, ops::Range};
use std::ops::Deref;

use bevy::{
    asset::{AssetId, Handle, UntypedAssetId, UntypedHandle, load_internal_asset},
    math::Mat4,
    core_pipeline::core_3d::Transparent3d,
    ecs::{
        query::ROQueryItem,
        system::{
            SystemParamItem, lifetimeless::{Read, SRes}
        },
    },
    pbr::MeshPipelineKey,
    prelude::{
        App, Assets, Color, Commands, Component, Entity, FromWorld, GlobalTransform, InheritedVisibility, IntoSystemConfigs, Msaa, Plugin, Query, Res, ResMut, Resource, Vec2, Vec3, ViewVisibility, World, Image
    },
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderSet,
        render_asset::RenderAssets,
        texture::GpuImage,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
            SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{
            BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferUsages, RawBufferVec, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, FragmentState, FrontFace, MultisampleState, PipelineCache, PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipelineDescriptor, SamplerBindingType, Shader, ShaderStages, ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines, StencilFaceState, StencilState, TextureFormat, TextureSampleType, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode
        },
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms}
    },
    utils::HashMap,
    color::ColorToComponents,
};
use uuid::Uuid;
use std::any::TypeId;
use bytemuck::{Pod, Zeroable};

use crate::render::zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta, ZoneLightingUniformData};
// use crate::diagnostics::render_diagnostics::{
//     log_pipeline_cache_access,
//     log_pipeline_creation,
//     PipelineType,
// };

pub const WORLD_UI_SHADER_HANDLE: UntypedHandle =
    UntypedHandle::Weak(UntypedAssetId::Uuid { type_id: TypeId::of::<Shader>(), uuid: Uuid::from_u128(0xd5cdda11c713e3a7) });

pub const WORLD_UI_SHADER_HANDLE_TYPED: Handle<Shader> =
    Handle::weak_from_u128(0xd5cdda11c713e3a7);

#[derive(Default)]
pub struct WorldUiRenderPlugin;

impl Plugin for WorldUiRenderPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            WORLD_UI_SHADER_HANDLE_TYPED,
            "shaders/world_ui.wgsl",
            Shader::from_wgsl
        );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedWorldUi>()
                .init_resource::<WorldUiMeta>()
                .init_resource::<ImageBindGroups>()
                .add_render_command::<Transparent3d, DrawWorldUi>()
                .init_resource::<SpecializedRenderPipelines<WorldUiPipeline>>()
                .add_systems(ExtractSchedule, extract_world_ui_rects)
                .add_systems(Render, (queue_world_ui_meshes).in_set(RenderSet::Queue));
        }
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
 
        render_app.init_resource::<WorldUiPipeline>();
    }
}

#[derive(Component, Clone)]
pub struct WorldUiRect {
    pub image: Handle<Image>,
    pub screen_offset: Vec2,
    pub screen_size: Vec2,
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub color: Color,
    pub order: u8,
}

pub struct ExtractedRect {
    pub world_position: Vec3,
    pub screen_offset: Vec2,
    pub screen_size: Vec2,
    pub image_handle_id: AssetId<Image>,
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub color: Color,
    pub order: u8,
}

#[derive(Resource)]
pub struct ExtractedWorldUi {
    pub rects: Vec<ExtractedRect>,
}

impl Default for ExtractedWorldUi {
    fn default() -> Self {
        Self {
            rects: Vec::with_capacity(1024),
        }
    }
}

fn extract_world_ui_rects(
    mut extracted_world_ui: ResMut<ExtractedWorldUi>,
    images: Extract<Res<Assets<Image>>>,
    query: Extract<Query<(&ViewVisibility, &InheritedVisibility, &GlobalTransform, &WorldUiRect)>>,
) {
    extracted_world_ui.rects.clear();
    let mut visible_count = 0;
    let mut extracted_count = 0;
    for (visible, _inherited_visibility, global_transform, rect) in query.iter() {
        if !visible.get() {
            continue;
        }

        if !images.contains(rect.image.id()) {
            continue;
        }

        extracted_world_ui.rects.push(ExtractedRect {
            world_position: global_transform.translation(),
            screen_offset: rect.screen_offset,
            screen_size: rect.screen_size,
            image_handle_id: rect.image.id(),
            uv_min: rect.uv_min,
            uv_max: rect.uv_max,
            color: rect.color,
            order: rect.order,
        });
        visible_count += 1;
        extracted_count += 1;
    }

    if extracted_count > 0 {
        log::info!("[RENDER] Extracted {} world UI rects ({} visible)", extracted_count, visible_count);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct WorldUiVertex {
    world_position: [f32; 3],
    screen_position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

#[derive(Resource)]
pub struct WorldUiMeta {
    vertices: RawBufferVec<WorldUiVertex>,
    view_bind_group: Option<BindGroup>,
}

impl Default for WorldUiMeta {
    fn default() -> Self {
        Self {
            vertices: RawBufferVec::new(BufferUsages::VERTEX),
            view_bind_group: None,
        }
    }
}

#[derive(Resource)]
pub struct WorldUiPipeline {
    view_layout: BindGroupLayout,
    vertex_shader: Handle<Shader>,
    fragment_shader: Handle<Shader>,
    material_layout: BindGroupLayout,
    zone_lighting_layout: BindGroupLayout,
}

impl SpecializedRenderPipeline for WorldUiPipeline {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.vertex_shader.clone(),
                entry_point: "vertex".into(),
                buffers: vec![VertexBufferLayout {
                    array_stride: 3 * 4 + 2 * 4 + 2 * 4 + 4 * 4,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![
                        // World Position
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        // Screen Position
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 3 * 4,
                            shader_location: 1,
                        },
                        // UV
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 3 * 4 + 2 * 4,
                            shader_location: 2,
                        },
                        // Color
                        VertexAttribute {
                            format: VertexFormat::Float32x4,
                            offset: 3 * 4 + 2 * 4 + 2 * 4,
                            shader_location: 3,
                        },
                    ],
                }],
                shader_defs: vec!["ZONE_LIGHTING_GROUP_2".into()],
            },
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs: vec!["ZONE_LIGHTING_GROUP_2".into()],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: match key.contains(MeshPipelineKey::HDR) {
                        true => ViewTarget::TEXTURE_FORMAT_HDR,
                        false => TextureFormat::Bgra8UnormSrgb,
                    },
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![
                self.view_layout.clone(),
                self.material_layout.clone(),
                self.zone_lighting_layout.clone(),
            ],
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("world_ui_pipeline".into()),
            push_constant_ranges: Vec::default(),
            zero_initialize_workgroup_memory: false,
        }
    }
}

impl FromWorld for WorldUiPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let view_layout = render_device.create_bind_group_layout(
            None,
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(ViewUniform::min_size()),
                },
                count: None,
            }],
        );
        let material_layout = render_device.create_bind_group_layout(
            Some("world_ui_material_layout"),
            &[
                // Base Texture
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Base Texture Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        );

        // Get zone lighting layout, or create a fallback if resource doesn't exist
        let zone_lighting_layout = match world.get_resource::<ZoneLightingUniformMeta>() {
            Some(meta) => meta.bind_group_layout.clone(),
            None => {
                // Create a fallback bind group layout if ZoneLightingUniformMeta is not available
                log::warn!("[WORLD UI] ZoneLightingUniformMeta not found, creating fallback bind group layout");
                render_device.create_bind_group_layout(
                    Some("fallback_zone_lighting_layout"),
                    &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(ZoneLightingUniformData::min_size()),
                        },
                        count: None,
                    }],
                )
            }
        };

        // // DIAGNOSTIC: Log pipeline creation (after layouts are created)
        // let mut diagnostics_state = world.resource_mut::<crate::diagnostics::render_diagnostics::RenderDiagnosticsState>();
        // log_pipeline_creation(
        //     &mut diagnostics_state,
        //     "world_ui_pipeline",
        //     PipelineType::Render,
        //     3, // Number of bind groups (view, material, zone_lighting)
        // );

        WorldUiPipeline {
            view_layout,
            vertex_shader: WORLD_UI_SHADER_HANDLE_TYPED,
            fragment_shader: WORLD_UI_SHADER_HANDLE_TYPED,
            material_layout,
            zone_lighting_layout,
        }
    }
}

pub struct SetWorldUiMaterialBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetWorldUiMaterialBindGroup<I> {
    type Param = SRes<ImageBindGroups>;
    type ViewQuery = ();
    type ItemQuery = Read<WorldUiBatch>;

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewQuery>,
        sprite_batch: Option<ROQueryItem<'w, Self::ItemQuery>>,
        image_bind_groups: SystemParamItem<'w, 'w, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let sprite_batch = match sprite_batch {
            Some(sprite_batch) => sprite_batch,
            None => return RenderCommandResult::Success,
        };
        let image_bind_groups = image_bind_groups.into_inner();
        pass.set_bind_group(
                I,
                image_bind_groups
                    .values
                    .get(&sprite_batch.image_handle_id)
                    .unwrap(),
                &[],
            );
        RenderCommandResult::Success
    }
}

type DrawWorldUi = (
    SetItemPipeline,
    SetWorldUiViewBindGroup<0>,
    SetWorldUiMaterialBindGroup<1>,
    SetZoneLightingBindGroup<2>,
    DrawWorldUiBatch,
);

struct DrawWorldUiBatch;
impl<P: PhaseItem> RenderCommand<P> for DrawWorldUiBatch {
    type Param = SRes<WorldUiMeta>;
    type ViewQuery = ();
    type ItemQuery = Read<WorldUiBatch>;

    #[inline]
    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewQuery>,
        batch: Option<ROQueryItem<'w, Self::ItemQuery>>,
        sprite_meta: SystemParamItem<'w, 'w, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let batch = match batch {
            Some(batch) => batch,
            None => return RenderCommandResult::Success,
        };
        let sprite_meta = sprite_meta.into_inner();
        pass.set_vertex_buffer(0, sprite_meta.vertices.buffer().unwrap().slice(..));
        pass.draw(batch.vertex_range.clone(), 0..1);
        RenderCommandResult::Success
    }
}

struct SetWorldUiViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetWorldUiViewBindGroup<I> {
    type Param = SRes<WorldUiMeta>;
    type ViewQuery = Option<Read<ViewUniformOffset>>;
    type ItemQuery = Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<()>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>;

    fn render<'w>(
        _: &P,
        view_uniform: ROQueryItem<'w, Self::ViewQuery>,
        _item_query: Option<ROQueryItem<'w, Self::ItemQuery>>,
        world_ui_meta: SystemParamItem<'w, 'w, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(view_uniform) = view_uniform else {
            return RenderCommandResult::Success;
        };
        pass.set_bind_group(
            I,
            world_ui_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}

#[derive(Component, Eq, PartialEq, Clone)]
pub struct WorldUiBatch {
    pub image_handle_id: AssetId<Image>,
    pub vertex_range: Range<u32>,
}

#[derive(Default, Resource)]
pub struct ImageBindGroups {
    pub values: HashMap<AssetId<Image>, BindGroup>,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_world_ui_meshes(
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    world_ui_pipeline: Res<WorldUiPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<WorldUiPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    views: Query<(Entity, &ExtractedView, Option<&Msaa>)>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    mut extracted_world_ui: ResMut<ExtractedWorldUi>,
    mut world_ui_meta: ResMut<WorldUiMeta>,
    mut commands: Commands,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    render_images: Res<RenderAssets<GpuImage>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    view_uniforms: Res<ViewUniforms>,
    // mut diagnostics_state: ResMut<crate::diagnostics::render_diagnostics::RenderDiagnosticsState>,
) {
    if view_uniforms.uniforms.is_empty() {
        return;
    }

    // DIAGNOSTIC: Log pipeline cache access before using it
    // log_pipeline_cache_access(
    //     &mut diagnostics_state,
    //     None,
    //     PipelineType::Render,
    //     0, // Cache size not directly accessible, will be logged on access
    // );

    let draw_alpha_mask = transparent_draw_functions
        .read()
        .get_id::<DrawWorldUi>()
        .unwrap();

    if let Some(view_bindings) = view_uniforms.uniforms.binding() {
        world_ui_meta.view_bind_group.get_or_insert_with(|| {
            render_device.create_bind_group(
                "world_ui_view_bind_group",
                &world_ui_pipeline.view_layout,
                &[BindGroupEntry {
                    binding: 0,
                    resource: view_bindings,
                }],
            )
        });
    }

    // Query for views and look up their phases from the resource
    for (view_entity, view, msaa) in views.iter() {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view_entity) else {
            continue;
        };

        let msaa_samples = msaa.map(|m| m.samples()).unwrap_or(1);
        let view_key = MeshPipelineKey::from_msaa_samples(msaa_samples)
            | MeshPipelineKey::from_hdr(view.hdr);
        
        // DIAGNOSTIC: Log pipeline cache access before specialization
        // log_pipeline_cache_access(
        //     &mut diagnostics_state,
        //     None,
        //     PipelineType::Render,
        //     0, // Cache size not directly accessible
        // );
        
        let pipeline = pipelines.specialize(&pipeline_cache, &world_ui_pipeline, view_key);
        let view_matrix = view.world_from_view.compute_matrix();
        let inverse_view_transform = view_matrix.inverse();
        let inverse_view_row_2 = inverse_view_transform.row(2);
        let view_proj = view.clip_from_world.unwrap_or(Mat4::IDENTITY);
        let view_width = view.viewport.z as f32;
        let view_height = view.viewport.w as f32;

        extracted_world_ui.rects.sort_unstable_by(|a, b| {
            match view_proj
                .project_point3(a.world_position)
                .z
                .partial_cmp(&view_proj.project_point3(b.world_position).z)
            {
                Some(Ordering::Equal) | None => a.order.cmp(&b.order),
                Some(other) => other,
            }
        });

        world_ui_meta.vertices.clear();
        world_ui_meta
            .vertices
            .reserve(extracted_world_ui.rects.len() * 6, &render_device);

        for rect in extracted_world_ui.rects.iter() {
            let gpu_image =
                if let Some(gpu_image) = render_images.get(rect.image_handle_id) {
                    gpu_image
                } else {
                    // Image not ready yet, ignore
                    continue;
                };

            let clip_pos = view_proj.project_point3(rect.world_position);
            if clip_pos.z < 0.0 || clip_pos.z > 1.0 {
                // Outside frustum depth, ignore
                continue;
            }
            let screen_pos =
                (clip_pos.truncate() + Vec2::ONE) / 2.0 * Vec2::new(view_width, view_height);

            let min_screen_pos = screen_pos + rect.screen_offset;
            let max_screen_pos = screen_pos + rect.screen_offset + rect.screen_size;
            if max_screen_pos.x < 0.0
                || max_screen_pos.y < 0.0
                || min_screen_pos.x >= view_width
                || min_screen_pos.y >= view_height
            {
                // Not visible on screen
                continue;
            }

            let positions = [
                [rect.screen_offset.x, rect.screen_offset.y],
                [
                    rect.screen_offset.x + rect.screen_size.x,
                    rect.screen_offset.y,
                ],
                [
                    rect.screen_offset.x + rect.screen_size.x,
                    rect.screen_offset.y + rect.screen_size.y,
                ],
                [
                    rect.screen_offset.x,
                    rect.screen_offset.y + rect.screen_size.y,
                ],
            ];
            let uvs = [
                [rect.uv_min.x, rect.uv_max.y],
                [rect.uv_max.x, rect.uv_max.y],
                [rect.uv_max.x, rect.uv_min.y],
                [rect.uv_min.x, rect.uv_min.y],
            ];

            const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];
            let color = rect.color.to_linear().to_f32_array();

            let item_start = world_ui_meta.vertices.len() as u32;
            for i in QUAD_INDICES {
                world_ui_meta.vertices.push(WorldUiVertex {
                    world_position: rect.world_position.to_array(),
                    screen_position: positions[i],
                    uv: uvs[i],
                    color,
                });
            }
            let item_end = world_ui_meta.vertices.len() as u32;

            let visible_entity = commands
                .spawn(WorldUiBatch {
                    image_handle_id: rect.image_handle_id,
                    vertex_range: item_start..item_end,
                })
                .id();

            image_bind_groups
                .values
                .entry(rect.image_handle_id)
                .or_insert_with(|| {
                    render_device.create_bind_group(
                        "world_ui_bind_group",
                        &world_ui_pipeline.material_layout,
                        &[
                            BindGroupEntry {
                                binding: 0,
                                resource: BindingResource::TextureView(&gpu_image.texture_view),
                            },
                            BindGroupEntry {
                                binding: 1,
                                resource: BindingResource::Sampler(&gpu_image.sampler),
                            },
                        ],
                    )
                });

            transparent_phase.items.push(Transparent3d {
                entity: (visible_entity, visible_entity.into()),
                draw_function: draw_alpha_mask,
                pipeline,
                distance: inverse_view_row_2.dot(rect.world_position.extend(1.0)) + 999999.0,
                batch_range: 0..1,
                extra_index: bevy::render::render_phase::PhaseItemExtraIndex(0),
            });
        }
    }
}
