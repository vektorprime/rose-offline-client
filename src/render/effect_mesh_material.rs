use bevy::{
    asset::{load_internal_asset, AssetApp, Handle, UntypedHandle, Asset, UntypedAssetId},
    log::info_span,
    core_pipeline::core_3d::{Opaque3d, Transparent3d},
    ecs::{query::QueryItem, system::{lifetimeless::{Read, SRes}, SystemParamItem}},
    pbr::{
        MeshPipeline, MeshPipelineKey, SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::{
        AlphaMode, App, Commands, Component, Entity, FromWorld, IntoSystemConfigs, Mesh, Plugin, Query, Res, ResMut, Resource, With, World,
    },
    reflect::{Reflect, TypePath},
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{GpuBufferInfo, MeshVertexBufferLayout},
        prelude::Shader,
        render_asset::RenderAssets,
        render_phase::{AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline, TrackedRenderPass},
        render_resource::{
            AsBindGroup, AsBindGroupError, AsBindGroupShaderType, BindGroup, BindGroupEntry,
            BindGroupLayout, BindGroupLayoutEntry, BindingResource,
            BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState,
            Buffer, BufferUsages, CompareFunction, UnpreparedBindGroup, RenderPipelineDescriptor, SamplerBindingType,
            ShaderRef, ShaderStages, SpecializedMeshPipeline, SpecializedMeshPipelineError,
            SpecializedMeshPipelines, TextureSampleType, TextureViewDimension, VertexFormat,
            PipelineCache, CachedRenderPipelineId, PushConstantRange, ShaderDefVal, ShaderSize,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{FallbackImage, Image, GpuImage},
        view::{ExtractedView, ViewTarget},
        Extract, ExtractSchedule, Render, RenderApp, RenderSet,
    },
    utils::Uuid,
};
use std::any::TypeId;

use crate::render::{
    custom_mesh_pipeline::{CustomMeshPipelineKey, HashableAlphaMode, MeshVertexLayoutBuilder, PipelineDescriptorBuilder},
    zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta},
};

pub const EFFECT_MESH_MATERIAL_SHADER_HANDLE: UntypedHandle =
    UntypedHandle::Weak(UntypedAssetId::Uuid {
        type_id: TypeId::of::<Shader>(),
        uuid: Uuid::from_u128(0x90d5233c3001d33e),
    });

#[derive(Default)]
pub struct EffectMeshMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for EffectMeshMaterialPlugin {
    fn build(&self, app: &mut App) {
        bevy::log::info!("[EFFECT MESH MATERIAL] Building EffectMeshMaterialPlugin, prepass_enabled={}", self.prepass_enabled);
        
        // Load the internal asset using the Bevy 0.13 API
        load_internal_asset!(
            app,
            EFFECT_MESH_MATERIAL_SHADER_HANDLE.typed::<Shader>(),
            "shaders/effect_mesh_material.wgsl",
            Shader::from_wgsl
        );
        bevy::log::info!("[EFFECT MESH MATERIAL] Shader loaded successfully");

        // Register the EffectMeshMaterial asset type (normally done by MaterialPlugin)
        app.init_asset::<EffectMeshMaterial>();

        // Register EffectMeshAnimationRenderState component
        app.add_plugins(ExtractComponentPlugin::<EffectMeshAnimationRenderState>::extract_visible());
        app.register_type::<EffectMeshAnimationRenderState>();
        bevy::log::info!("[EFFECT MESH MATERIAL] ExtractComponentPlugin<EffectMeshAnimationRenderState> added");

        // Initialize render resources and systems
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<RenderEffectMeshMaterials>()
                .init_resource::<GpuEffectMeshMaterials>()
                .init_resource::<ExtractedEffectMeshes>()
                .init_resource::<EffectMeshMaterialPipeline>()
                .init_resource::<SpecializedMeshPipelines<EffectMeshMaterialPipeline>>()
                .add_render_command::<Opaque3d, DrawEffectMeshMaterial>()
                .add_render_command::<Transparent3d, DrawEffectMeshMaterial>()
                .add_systems(
                    ExtractSchedule,
                    (extract_effect_mesh_materials, extract_effect_mesh_meshes),
                )
                .add_systems(
                    Render,
                    prepare_effect_mesh_material_bind_groups,
                )
                .add_systems(
                    Render,
                    queue_effect_mesh_material_meshes.in_set(RenderSet::Queue),
                );
        }
        bevy::log::info!("[EFFECT MESH MATERIAL] EffectMeshMaterialPlugin build complete");
    }

    fn finish(&self, app: &mut App) {
        // Pipeline is initialized lazily in queue_effect_mesh_material_meshes
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct EffectMeshMaterialFlags: u32 {
        const ALPHA_MODE_OPAQUE         = (1 << 0);
        const ALPHA_MODE_MASK           = (1 << 1);
        const NONE                      = 0;
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct EffectMeshAnimationFlags: u32 {
        const ANIMATE_POSITION          = (1 << 0);
        const ANIMATE_NORMALS           = (1 << 1);
        const ANIMATE_UV                = (1 << 2);
        const ANIMATE_ALPHA             = (1 << 3);
        const NONE                      = 0;
    }
}

#[derive(Copy, Clone, Default, Component, bevy::render::render_resource::encase::ShaderType, Reflect)]
pub struct EffectMeshAnimationRenderState {
    pub flags: u32,
    pub current_next_frame: u32,
    pub next_weight: f32,
    pub alpha: f32,
}

impl ExtractComponent for EffectMeshAnimationRenderState {
    type QueryData = &'static Self;
    type QueryFilter = With<Handle<EffectMeshMaterial>>;
    type Out = Self;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self> {
        Some(*item)
    }
}

/// Resource to hold extracted effect mesh materials for render world
#[derive(Resource, Default)]
pub struct RenderEffectMeshMaterials {
    pub materials: bevy::utils::HashMap<bevy::asset::AssetId<EffectMeshMaterial>, EffectMeshMaterial>,
}

/// Resource to hold extracted effect mesh meshes for rendering
#[derive(Resource, Default)]
pub struct ExtractedEffectMeshes {
    pub meshes: Vec<(Entity, Handle<EffectMeshMaterial>, Handle<Mesh>)>,
}

/// Extract effect mesh materials from main world to render world
fn extract_effect_mesh_materials(
    mut render_materials: ResMut<RenderEffectMeshMaterials>,
    main_materials: Extract<Res<bevy::asset::Assets<EffectMeshMaterial>>>,
    mut last_count: bevy::ecs::system::Local<usize>,
) {
    let _span = info_span!("extract_effect_mesh_materials").entered();
    let current_count = main_materials.len();
    
    // Skip if material count hasn't changed and we have materials
    if current_count == *last_count && current_count > 0 && !render_materials.materials.is_empty() {
        return;
    }
    
    *last_count = current_count;

    // Track which materials still exist for cleanup
    let mut active_ids = bevy::utils::HashSet::default();
    
    // Iterate over all materials to track active IDs and update changed ones
    for (id, material) in main_materials.iter() {
        active_ids.insert(id);
        
        // Only clone if material is new or changed
        let should_update = match render_materials.materials.get(&id) {
            Some(existing) => existing != material,
            None => true, // New material
        };
        
        if should_update {
            render_materials.materials.insert(id, material.clone());
        }
    }
    
    // Remove materials that no longer exist in main world
    render_materials.materials.retain(|id, _| active_ids.contains(id));
}

/// Extract effect mesh meshes from main world to render world
fn extract_effect_mesh_meshes(
    mut extracted_meshes: ResMut<ExtractedEffectMeshes>,
    query: Extract<Query<(Entity, &Handle<EffectMeshMaterial>, &Handle<Mesh>)>>,
    mut local_meshes: bevy::ecs::system::Local<Vec<(Entity, Handle<EffectMeshMaterial>, Handle<Mesh>)>>,
    mut last_count: bevy::ecs::system::Local<usize>,
) {
    let _span = info_span!("extract_effect_mesh_meshes").entered();
    // Quick check: count entities first without fully iterating
    let current_count = query.iter().count();
    
    // Skip if count hasn't changed and we processed the same number before
    if current_count == 0 && extracted_meshes.meshes.is_empty() {
        return;
    }

    // Clear and reuse the local buffer to avoid allocations
    local_meshes.clear();
    
    // Build a new list of active meshes using weak handles
    for (entity, material_handle, mesh_handle) in query.iter() {
        local_meshes.push((entity, material_handle.clone_weak(), mesh_handle.clone_weak()));
    }
    
    // Only update if the list has actually changed
    let has_changed = extracted_meshes.meshes.len() != local_meshes.len() ||
        extracted_meshes.meshes.iter().zip(local_meshes.iter()).any(|(a, b)| {
            a.0 != b.0 || a.1.id() != b.1.id() || a.2.id() != b.2.id()
        });
    
    if has_changed {
        // Clone from the local buffer instead of taking ownership
        extracted_meshes.meshes.clone_from(&*local_meshes);
    }
    
    *last_count = current_count;
}

/// Resource to store prepared bind groups for effect mesh materials
#[derive(Resource, Default)]
pub struct GpuEffectMeshMaterials {
    bind_groups: bevy::utils::HashMap<bevy::asset::AssetId<EffectMeshMaterial>, BindGroup>,
}

impl GpuEffectMeshMaterials {
    pub fn get(&self, id: bevy::asset::AssetId<EffectMeshMaterial>) -> Option<&BindGroup> {
        self.bind_groups.get(&id)
    }

    pub fn insert(&mut self, id: bevy::asset::AssetId<EffectMeshMaterial>, bind_group: BindGroup) {
        self.bind_groups.insert(id, bind_group);
    }
}

/// Prepares bind groups for effect mesh materials
fn prepare_effect_mesh_material_bind_groups(
    mut gpu_materials: ResMut<GpuEffectMeshMaterials>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    render_materials: Res<RenderEffectMeshMaterials>,
    mut pipeline: ResMut<EffectMeshMaterialPipeline>,
    mesh_pipeline: Option<Res<MeshPipeline>>,
    zone_lighting_meta: Res<ZoneLightingUniformMeta>,
) {
    let _span = info_span!("prepare_effect_mesh_material_bind_groups").entered();

    // Lazy initialization of the pipeline if not already done
    pipeline.initialize(mesh_pipeline.as_deref(), &render_device, &zone_lighting_meta);

    // Skip if pipeline layout not initialized yet
    let material_layout = match pipeline.material_layout.as_ref() {
        Some(layout) => layout,
        None => {
            bevy::log::debug!("[EFFECT MESH MATERIAL] Material bind group layout not yet initialized, will retry next frame");
            return;
        }
    };

    // Cleanup: Remove bind groups for materials that no longer exist
    let active_material_ids: std::collections::HashSet<_> = render_materials
        .materials
        .keys()
        .copied()
        .collect();
    gpu_materials
        .bind_groups
        .retain(|id, _| active_material_ids.contains(id));

    for (id, material) in render_materials.materials.iter() {
        // Only create bind group if it doesn't exist yet
        if gpu_materials.get(*id).is_some() {
            continue;
        }

        // Use AsBindGroup to prepare the uniform buffer and get bindings
        let unprepared = match material.as_bind_group(material_layout, &render_device, &images, &fallback_image) {
            Ok(unprepared) => unprepared,
            Err(AsBindGroupError::RetryNextUpdate) => {
                // Textures are still loading - this is expected, log at debug level only
                bevy::log::debug!(
                    "[EFFECT MESH MATERIAL] Textures not ready for material {:?}: base={:?} animation={:?}",
                    id,
                    material.base_texture.is_some(),
                    material.animation_texture.is_some()
                );
                continue;
            }
            Err(e) => {
                bevy::log::warn!("[EFFECT MESH MATERIAL] Failed to prepare bind group for material {:?}: {:?}", id, e);
                continue;
            }
        };

        // Get texture views (use fallback if not loaded)
        let base_texture_view = match material.base_texture.as_ref().and_then(|h| images.get(h)) {
            Some(image) => &*image.texture_view,
            None => &*fallback_image.d2.texture_view,
        };

        let animation_texture_view = match material.animation_texture.as_ref().and_then(|h| images.get(h)) {
            Some(image) => &*image.texture_view,
            None => &*fallback_image.d2.texture_view,
        };

        // Extract the uniform buffer from the unprepared bindings
        let uniform_buffer = unprepared.bindings.iter()
            .find(|(binding, _)| *binding == 0)
            .and_then(|(_, resource)| {
                if let bevy::render::render_resource::OwnedBindingResource::Buffer(buffer) = resource {
                    Some(buffer.clone())
                } else {
                    None
                }
            })
            .expect("Uniform buffer not found in unprepared bindings");

        // Create bind group with texture array
        let bind_group = render_device.create_bind_group(
            "effect_mesh_material_bind_group",
            material_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(base_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&*fallback_image.d2.sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(animation_texture_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&*fallback_image.d2.sampler),
                },
            ],
        );

        gpu_materials.insert(*id, bind_group);
    }
}

/// Pipeline key for effect mesh material specialization
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EffectMeshMaterialPipelineKey {
    pub mesh_key: MeshPipelineKey,
    pub custom_key: CustomMeshPipelineKey,
    pub has_animation_texture: bool,
    pub blend_op: BlendOperation,
    pub src_blend_factor: BlendFactor,
    pub dst_blend_factor: BlendFactor,
}

/// Effect mesh material render pipeline (initialized lazily)
#[derive(Resource, Default)]
pub struct EffectMeshMaterialPipeline {
    pub mesh_pipeline: Option<MeshPipeline>,
    pub material_layout: Option<BindGroupLayout>,
    pub zone_lighting_layout: Option<BindGroupLayout>,
}

impl EffectMeshMaterialPipeline {
    /// Initialize the pipeline if not already done
    fn initialize(
        &mut self,
        mesh_pipeline: Option<&MeshPipeline>,
        render_device: &RenderDevice,
        zone_lighting_meta: &ZoneLightingUniformMeta,
    ) {
        if self.mesh_pipeline.is_some() {
            return;
        }

        // Create material bind group layout
        let material_layout = render_device.create_bind_group_layout(
            "effect_mesh_material_bind_group_layout",
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: bevy::render::render_resource::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        );

        // Get zone lighting bind group layout
        let zone_lighting_layout = zone_lighting_meta.bind_group_layout.clone();

        if let Some(pipeline) = mesh_pipeline {
            self.mesh_pipeline = Some(pipeline.clone());
        }
        self.material_layout = Some(material_layout);
        self.zone_lighting_layout = Some(zone_lighting_layout);
    }

    fn is_ready(&self) -> bool {
        self.mesh_pipeline.is_some() && self.material_layout.is_some() && self.zone_lighting_layout.is_some()
    }
}

impl SpecializedMeshPipeline for EffectMeshMaterialPipeline {
    type Key = EffectMeshMaterialPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mesh_pipeline = self.mesh_pipeline.as_ref()
            .expect("EffectMeshMaterialPipeline should be initialized before specialize is called");
        let material_layout = self.material_layout.as_ref()
            .expect("EffectMeshMaterialPipeline should be initialized before specialize is called");
        let zone_lighting_layout = self.zone_lighting_layout.as_ref()
            .expect("EffectMeshMaterialPipeline should be initialized before specialize is called");
        
        // Build base descriptor using shared infrastructure
        let mut descriptor = PipelineDescriptorBuilder::build_base_descriptor(
            mesh_pipeline,
            key.mesh_key,
            &key.custom_key,
            layout,
        )?;

        // Apply effect mesh material shader
        descriptor.vertex.shader = EFFECT_MESH_MATERIAL_SHADER_HANDLE.typed().into();
        if let Some(ref mut fragment) = descriptor.fragment {
            fragment.shader = EFFECT_MESH_MATERIAL_SHADER_HANDLE.typed().into();
        }

        // Add material bind group layout at index 2 (after view at 0, mesh at 1)
        descriptor.layout.insert(2, material_layout.clone());

        // Add zone lighting bind group layout at index 3
        descriptor.layout.push(zone_lighting_layout.clone());

        // Apply depth/stencil configuration
        descriptor.depth_stencil.as_mut().unwrap().depth_write_enabled = 
            key.custom_key.alpha_mode.to_alpha_mode() != AlphaMode::Blend;

        // Apply face culling
        descriptor.primitive.cull_mode = if key.custom_key.two_sided {
            None
        } else {
            Some(bevy::render::render_resource::Face::Back)
        };

        // Add shader defines
        descriptor
            .vertex
            .shader_defs
            .push("VERTEX_UVS".into());
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment
                .shader_defs
                .push("VERTEX_UVS".into());
        }

        if key.has_animation_texture {
            descriptor
                .vertex
                .shader_defs
                .push(ShaderDefVal::Bool("HAS_ANIMATION_TEXTURE".into(), true));
            if let Some(fragment) = descriptor.fragment.as_mut() {
                fragment
                    .shader_defs
                    .push(ShaderDefVal::Bool("HAS_ANIMATION_TEXTURE".into(), true));
            }
            descriptor.push_constant_ranges.push(PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..EffectMeshAnimationRenderState::SHADER_SIZE.get() as u32,
            });
        }

        // Apply custom blend state
        if let Some(ref mut fragment) = descriptor.fragment {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: key.src_blend_factor,
                        dst_factor: key.dst_blend_factor,
                        operation: key.blend_op,
                    },
                    alpha: BlendComponent {
                        src_factor: key.src_blend_factor,
                        dst_factor: key.dst_blend_factor,
                        operation: key.blend_op,
                    },
                });
            }
        }

        // Disable color fog for additive blending
        // IMPORTANT: Shader defs MUST be applied consistently to BOTH vertex and fragment stages
        // to prevent pipeline validation errors due to signature mismatches.
        if matches!(key.blend_op, BlendOperation::Add) {
            let fog_def = ShaderDefVal::Bool("ZONE_LIGHTING_DISABLE_COLOR_FOG".into(), true);
            descriptor.vertex.shader_defs.push(fog_def.clone());
            if let Some(fragment) = descriptor.fragment.as_mut() {
                fragment.shader_defs.push(fog_def);
            }
        }

        Ok(descriptor)
    }
}

/// Custom render command to set the effect mesh material bind group
pub struct SetEffectMeshMaterialBindGroup<const I: u32>;

impl<P: PhaseItem, const I: u32> RenderCommand<P> for SetEffectMeshMaterialBindGroup<I> {
    type Param = SRes<GpuEffectMeshMaterials>;
    type ViewQuery = ();
    type ItemQuery = Read<Handle<EffectMeshMaterial>>;

    fn render<'w>(
        _item: &P,
        _view: bevy::ecs::query::ROQueryItem<'w, Self::ViewQuery>,
        material_handle: Option<bevy::ecs::query::ROQueryItem<'w, Self::ItemQuery>>,
        gpu_materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material_handle = match material_handle {
            Some(handle) => handle,
            None => {
                bevy::log::warn!("[RENDER CMD] SetEffectMeshMaterialBindGroup: No material handle!");
                return RenderCommandResult::Failure;
            }
        };

        let bind_group = match gpu_materials.into_inner().get(material_handle.id()) {
            Some(bind_group) => bind_group,
            None => {
                bevy::log::warn!("[RENDER CMD] SetEffectMeshMaterialBindGroup: No bind group for material {:?}!", material_handle.id());
                return RenderCommandResult::Failure;
            }
        };

        pass.set_bind_group(I as usize, bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawEffectMeshMesh;
impl<P: PhaseItem> RenderCommand<P> for DrawEffectMeshMesh {
    type Param = SRes<RenderAssets<Mesh>>;
    type ViewQuery = ();
    type ItemQuery = (
        Read<Handle<Mesh>>,
        Option<Read<EffectMeshAnimationRenderState>>,
    );

    #[inline]
    fn render<'w>(
        _: &P,
        _: bevy::ecs::query::ROQueryItem<'_, Self::ViewQuery>,
        item: Option<bevy::ecs::query::ROQueryItem<'_, Self::ItemQuery>>,
        meshes: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (mesh_handle, animation_state) = match item {
            Some((mesh_handle, animation_state)) => (mesh_handle, animation_state),
            None => return RenderCommandResult::Failure,
        };
        
        // Set animation state as push constant if present
        if let Some(animation_state) = animation_state {
            let byte_buffer = [0u8; EffectMeshAnimationRenderState::SHADER_SIZE.get() as usize];
            let mut buffer = bevy::render::render_resource::encase::StorageBuffer::new(byte_buffer);
            buffer.write(animation_state).unwrap();
            pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, buffer.as_ref());
        }

        if let Some(gpu_mesh) = meshes.into_inner().get(mesh_handle) {
            pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));

            match &gpu_mesh.buffer_info {
                GpuBufferInfo::Indexed {
                    buffer,
                    index_format,
                    count,
                } => {
                    pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                    pass.draw_indexed(0..*count, 0, 0..1);
                }
                GpuBufferInfo::NonIndexed => {
                    pass.draw(0..gpu_mesh.vertex_count, 0..1);
                }
            }
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}

type DrawEffectMeshMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetEffectMeshMaterialBindGroup<2>,
    SetZoneLightingBindGroup<3>,
    DrawEffectMeshMesh,
);

#[derive(Clone, bevy::render::render_resource::encase::ShaderType)]
pub struct EffectMeshMaterialUniformData {
    pub flags: u32,
    pub alpha_cutoff: f32,
}

#[derive(Asset, AsBindGroup, Debug, Clone, PartialEq, TypePath)]
#[bind_group_data(EffectMeshMaterialKey)]
#[uniform(0, EffectMeshMaterialUniformData)]
pub struct EffectMeshMaterial {
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Option<Handle<Image>>,

    #[texture(3)]
    #[sampler(4)]
    pub animation_texture: Option<Handle<Image>>,

    pub alpha_enabled: bool,
    pub alpha_test: bool,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
    pub blend_op: BlendOperation,
    pub src_blend_factor: BlendFactor,
    pub dst_blend_factor: BlendFactor,
}

impl AsBindGroupShaderType<EffectMeshMaterialUniformData> for EffectMeshMaterial {
    fn as_bind_group_shader_type(
        &self,
        _images: &RenderAssets<Image>,
    ) -> EffectMeshMaterialUniformData {
        let mut flags = EffectMeshMaterialFlags::NONE;
        if self.alpha_test {
            flags |= EffectMeshMaterialFlags::ALPHA_MODE_MASK;
        } else if !self.alpha_enabled {
            flags |= EffectMeshMaterialFlags::ALPHA_MODE_OPAQUE;
        }

        EffectMeshMaterialUniformData {
            flags: flags.bits(),
            alpha_cutoff: 0.5,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EffectMeshMaterialKey {
    has_animation_texture: bool,
    alpha_enabled: bool,
    alpha_test: bool,
    two_sided: bool,
    z_test_enabled: bool,
    z_write_enabled: bool,
    blend_op: BlendOperation,
    src_blend_factor: BlendFactor,
    dst_blend_factor: BlendFactor,
}

impl From<&EffectMeshMaterial> for EffectMeshMaterialKey {
    fn from(material: &EffectMeshMaterial) -> Self {
        EffectMeshMaterialKey {
            has_animation_texture: material.animation_texture.is_some(),
            alpha_enabled: material.alpha_enabled,
            alpha_test: material.alpha_test,
            two_sided: material.two_sided,
            z_test_enabled: material.z_test_enabled,
            z_write_enabled: material.z_write_enabled,
            blend_op: material.blend_op,
            src_blend_factor: material.src_blend_factor,
            dst_blend_factor: material.dst_blend_factor,
        }
    }
}

/// Queue effect mesh material meshes for rendering
#[allow(clippy::too_many_arguments)]
fn queue_effect_mesh_material_meshes(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    mut effect_pipeline: ResMut<EffectMeshMaterialPipeline>,
    mut pipelines: ResMut<bevy::render::render_resource::SpecializedMeshPipelines<EffectMeshMaterialPipeline>>,
    mut opaque_phases: Query<&mut RenderPhase<Opaque3d>>,
    mut transparent_phases: Query<&mut RenderPhase<Transparent3d>>,
    extracted_meshes: Res<ExtractedEffectMeshes>,
    gpu_materials: Res<GpuEffectMeshMaterials>,
    render_materials: Res<RenderEffectMeshMaterials>,
    render_meshes: Res<RenderAssets<Mesh>>,
    pipeline_cache: Res<PipelineCache>,
    mesh_pipeline: Option<Res<MeshPipeline>>,
    render_device: Res<RenderDevice>,
    zone_lighting_meta: Res<ZoneLightingUniformMeta>,
) {
    let _span = info_span!("queue_effect_mesh_material_meshes").entered();
    
    // Lazy initialization of the pipeline
    effect_pipeline.initialize(mesh_pipeline.as_deref(), &render_device, &zone_lighting_meta);
    
    // Skip if pipeline is not ready yet
    if !effect_pipeline.is_ready() {
        bevy::log::warn!("[EFFECT MESH MATERIAL] Pipeline NOT READY - cannot queue meshes!");
        return;
    }
    
    let opaque_draw_function = opaque_draw_functions
        .read()
        .get_id::<DrawEffectMeshMaterial>()
        .unwrap();
    let transparent_draw_function = transparent_draw_functions
        .read()
        .get_id::<DrawEffectMeshMaterial>()
        .unwrap();

    let mut total_queued = 0u32;
    let mut skipped_no_material = 0u32;
    let mut skipped_no_mesh = 0u32;
    let mut pipeline_failures = 0u32;

    for (entity, material_handle, mesh_handle) in extracted_meshes.meshes.iter() {
        // Only queue if material bind group is ready
        if gpu_materials.get(material_handle.id()).is_none() {
            skipped_no_material += 1;
            continue;
        }

        // Get the material to determine alpha mode
        let material = match render_materials.materials.get(&material_handle.id()) {
            Some(mat) => mat,
            None => {
                skipped_no_material += 1;
                continue;
            }
        };

        // Get the mesh to access its layout
        let gpu_mesh = match render_meshes.get(mesh_handle) {
            Some(mesh) => mesh,
            None => {
                skipped_no_mesh += 1;
                continue;
            }
        };

        // Determine alpha mode
        let alpha_mode = if material.alpha_enabled || !material.z_write_enabled {
            AlphaMode::Blend
        } else if material.alpha_test {
            AlphaMode::Mask(0.5)
        } else {
            AlphaMode::Opaque
        };

        // Create mesh vertex buffer layout from the GPU mesh
        let mesh_vertex_layout = MeshVertexBufferLayout::new((*gpu_mesh.layout).clone());

        // Create pipeline key
        let mesh_key = MeshPipelineKey::from_primitive_topology(
            bevy::render::render_resource::PrimitiveTopology::TriangleList
        );
        let custom_key = CustomMeshPipelineKey::new()
            .with_two_sided(material.two_sided)
            .with_alpha_mode(alpha_mode);
        let pipeline_key = EffectMeshMaterialPipelineKey {
            mesh_key,
            custom_key,
            has_animation_texture: material.animation_texture.is_some(),
            blend_op: material.blend_op,
            src_blend_factor: material.src_blend_factor,
            dst_blend_factor: material.dst_blend_factor,
        };

        // Get the specialized pipeline
        let pipeline = match pipelines.specialize(
            &pipeline_cache,
            &effect_pipeline,
            pipeline_key,
            &mesh_vertex_layout,
        ) {
            Ok(pipeline) => pipeline,
            Err(e) => {
                bevy::log::error!("[EFFECT MESH MATERIAL] Failed to specialize pipeline: {:?}", e);
                pipeline_failures += 1;
                continue;
            }
        };

        // Queue to appropriate phase based on alpha mode
        if alpha_mode == AlphaMode::Blend {
            for mut transparent_phase in transparent_phases.iter_mut() {
                transparent_phase.add(Transparent3d {
                    distance: 0.0, // Will be updated by sorting
                    pipeline,
                    entity: *entity,
                    draw_function: transparent_draw_function,
                    batch_range: 0..0,
                    dynamic_offset: None,
                });
            }
        } else {
            for mut opaque_phase in opaque_phases.iter_mut() {
                opaque_phase.add(Opaque3d {
                    asset_id: mesh_handle.id(),
                    pipeline,
                    entity: *entity,
                    draw_function: opaque_draw_function,
                    batch_range: 0..0,
                    dynamic_offset: None,
                });
            }
        }
        total_queued += 1;
    }
}
