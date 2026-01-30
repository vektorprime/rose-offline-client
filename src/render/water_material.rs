use std::num::NonZeroU32;

use bevy::{
    asset::{load_internal_asset, AssetApp, Handle, UntypedHandle, Asset, UntypedAssetId},
    log::info_span,
    core_pipeline::core_3d::Transparent3d,
    ecs::{
        query::{QueryItem, ROQueryItem},
        system::{lifetimeless::{Read, SRes}, SystemParamItem},
    },
    pbr::{
        DrawMesh, MeshPipeline, MeshPipelineKey, SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::{
        AlphaMode, App, Commands, Entity, FromWorld, Image, IntoSystemConfigs, Mesh,
        Plugin, Query, Res, ResMut, Resource, Time,
    },
    reflect::TypePath,
    render::{
        mesh::{MeshVertexBufferLayout, GpuBufferInfo},
        prelude::Shader,
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, 
            RenderPhase, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            encase, AddressMode, AsBindGroup, AsBindGroupError, BindGroup, BindGroupEntry,
            BindGroupLayout, BindGroupLayoutEntry,
            BindingResource, BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState,
            FilterMode, UnpreparedBindGroup, PushConstantRange, RenderPipelineDescriptor,
            SamplerBindingType, SamplerDescriptor, ShaderDefVal, ShaderRef, ShaderSize, ShaderStages,
            ShaderType, SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
            TextureSampleType, TextureViewDimension, PipelineCache, CachedRenderPipelineId,
        },
        renderer::RenderDevice,
        texture::FallbackImage,
        Extract, ExtractSchedule, Render, RenderApp, RenderSet,
    },
    utils::Uuid,
};

use crate::render::zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta};
use std::any::TypeId;

pub const WATER_MESH_MATERIAL_SHADER_HANDLE: UntypedHandle =
    UntypedHandle::Weak(UntypedAssetId::Uuid { type_id: TypeId::of::<Shader>(), uuid: Uuid::from_u128(0x333959e64b35d5d9) });

pub const WATER_MESH_MATERIAL_SHADER_HANDLE_TYPED: Handle<Shader> =
    Handle::weak_from_u128(0x333959e64b35d5d9);

pub const WATER_MATERIAL_NUM_TEXTURES: usize = 25;

#[derive(Default)]
pub struct WaterMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for WaterMaterialPlugin {
    fn build(&self, app: &mut App) {
        bevy::log::info!("[WATER MATERIAL] Building WaterMaterialPlugin");
        
        load_internal_asset!(
            app,
            WATER_MESH_MATERIAL_SHADER_HANDLE_TYPED,
            "shaders/water_material.wgsl",
            Shader::from_wgsl
        );

        // Register the WaterMaterial asset type (normally done by MaterialPlugin)
        app.init_asset::<WaterMaterial>();

        // NOTE: We don't use MaterialPlugin since we handle bind groups manually
        // due to texture array limitations in Bevy 0.13's AsBindGroup trait

        // Initialize render resources and systems
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            bevy::log::info!("[WATER MATERIAL] Initializing render app resources and systems");
            render_app
                .init_resource::<WaterMaterialBindGroupLayout>()
                .init_resource::<WaterSampler>()
                .init_resource::<GpuWaterMaterials>()
                .init_resource::<RenderWaterMaterials>()
                .init_resource::<ExtractedWaterMeshes>()
                .init_resource::<WaterMaterialPipeline>()
                .init_resource::<SpecializedMeshPipelines<WaterMaterialPipeline>>()
                .add_render_command::<Transparent3d, DrawWaterMaterial>()
                .init_resource::<ExtractedMsaa>()
                .add_systems(
                    ExtractSchedule,
                    (extract_water_materials, extract_water_meshes, extract_water_push_constant_data, extract_msaa),
                )
                .add_systems(
                    Render,
                    (init_water_material_bind_group_layout, init_water_sampler),
                )
                .add_systems(
                    Render,
                    prepare_water_material_bind_groups
                        .after(init_water_material_bind_group_layout)
                        .after(init_water_sampler),
                )
                .add_systems(
                    Render,
                    queue_water_material_meshes,
                );
            bevy::log::info!("[WATER MATERIAL] Render app systems initialized successfully");
        } else {
            bevy::log::error!("[WATER MATERIAL] FAILED to get render app - water rendering will not work!");
        }
    }

    fn finish(&self, _app: &mut App) {
        bevy::log::info!("[WATER MATERIAL] WaterMaterialPlugin finish called");
        // Pipeline is initialized lazily in queue_water_material_meshes
    }
}

/// Resource holding the bind group layout for water materials (initialized lazily)
#[derive(Resource, Clone, Default)]
pub struct WaterMaterialBindGroupLayout(pub Option<BindGroupLayout>);

/// Initialize the bind group layout on first frame when RenderDevice is available
fn init_water_material_bind_group_layout(
    mut layout: ResMut<WaterMaterialBindGroupLayout>,
    render_device: Res<RenderDevice>,
) {
    if layout.0.is_some() {
        return;
    }

    let bind_group_layout = render_device.create_bind_group_layout(
        "water_material_bind_group_layout",
        &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: NonZeroU32::new(WATER_MATERIAL_NUM_TEXTURES as u32),
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    );
    layout.0 = Some(bind_group_layout);
}

/// Resource holding the water material sampler (initialized lazily)
#[derive(Resource, Clone, Default)]
pub struct WaterSampler(pub Option<bevy::render::render_resource::Sampler>);

/// Initialize the sampler on first frame when RenderDevice is available
fn init_water_sampler(
    mut sampler: ResMut<WaterSampler>,
    render_device: Res<RenderDevice>,
) {
    if sampler.0.is_some() {
        return;
    }

    let new_sampler = render_device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::Repeat,
        address_mode_v: AddressMode::Repeat,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    });
    sampler.0 = Some(new_sampler);
}

/// Resource to store prepared bind groups for water materials
#[derive(Resource, Default)]
pub struct GpuWaterMaterials {
    bind_groups: bevy::utils::HashMap<bevy::asset::AssetId<WaterMaterial>, BindGroup>,
}

impl GpuWaterMaterials {
    pub fn get(&self, id: bevy::asset::AssetId<WaterMaterial>) -> Option<&BindGroup> {
        self.bind_groups.get(&id)
    }

    pub fn insert(&mut self, id: bevy::asset::AssetId<WaterMaterial>, bind_group: BindGroup) {
        self.bind_groups.insert(id, bind_group);
    }
}

/// Resource to hold extracted water materials for render world
/// Uses a HashMap for efficient incremental updates and change tracking
#[derive(Resource, Default)]
pub struct RenderWaterMaterials {
    pub materials: bevy::utils::HashMap<bevy::asset::AssetId<WaterMaterial>, WaterMaterial>,
}

/// Resource to hold extracted water meshes for rendering
#[derive(Resource, Default)]
pub struct ExtractedWaterMeshes {
    pub meshes: Vec<(Entity, Handle<WaterMaterial>, Handle<Mesh>)>,
}

/// Extract water materials from main world to render world
/// Uses incremental updates with count tracking to avoid unnecessary work
fn extract_water_materials(
    mut render_materials: ResMut<RenderWaterMaterials>,
    main_materials: Extract<Res<bevy::asset::Assets<WaterMaterial>>>,
    mut last_count: bevy::ecs::system::Local<usize>,
) {
    let _span = info_span!("extract_water_materials").entered();
    let current_count = main_materials.len();
    
    // Skip if material count hasn't changed and we have materials
    // This avoids iterating over all materials every frame
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
            Some(existing) => existing.textures.len() != material.textures.len() ||
                existing.textures.iter().zip(material.textures.iter()).any(|(a, b)| a != b),
            None => true, // New material
        };
        
        if should_update {
            render_materials.materials.insert(id, material.clone());
        }
    }
    
    // Remove materials that no longer exist in main world
    render_materials.materials.retain(|id, _| active_ids.contains(id));
}

/// Extract water meshes from main world to render world
/// Uses incremental updates to avoid allocating every frame
fn extract_water_meshes(
    mut extracted_meshes: ResMut<ExtractedWaterMeshes>,
    query: Extract<Query<(Entity, &Handle<WaterMaterial>, &Handle<Mesh>)>>,
    mut local_meshes: bevy::ecs::system::Local<Vec<(Entity, Handle<WaterMaterial>, Handle<Mesh>)>>,
) {
    let _span = info_span!("extract_water_meshes").entered();
    // Quick check: count entities first
    let current_count = query.iter().count();
    
    // Skip if no water meshes and none previously extracted
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
}

/// Prepares bind groups for water materials
fn prepare_water_material_bind_groups(
    mut gpu_materials: ResMut<GpuWaterMaterials>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    render_materials: Res<RenderWaterMaterials>,
    layout: Res<WaterMaterialBindGroupLayout>,
    water_sampler: Res<WaterSampler>,
) {
    let _span = info_span!("prepare_water_material_bind_groups").entered();
    // Skip if resources not initialized yet
    let Some(layout) = layout.0.as_ref() else {
        return;
    };
    let Some(sampler) = water_sampler.0.as_ref() else {
        return;
    };

    // Cleanup: Remove bind groups for materials that no longer exist
    // to prevent memory growth when materials are destroyed
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

        // Collect all texture views for the water animation array
        let mut textures: Vec<_> = vec![&*fallback_image.d2.texture_view; WATER_MATERIAL_NUM_TEXTURES];
        let mut all_textures_loaded = true;

        for (idx, handle) in material.textures.iter().take(WATER_MATERIAL_NUM_TEXTURES).enumerate() {
            match images.get(handle) {
                Some(image) => textures[idx] = &*image.texture_view,
                None => {
                    all_textures_loaded = false;
                    break;
                }
            }
        }

        if !all_textures_loaded {
            continue; // Skip this material until all textures are loaded
        }

        // Create bind group with texture array
        let bind_group = render_device.create_bind_group(
            "water_material_bind_group",
            layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureViewArray(&textures[..]),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        );

        gpu_materials.insert(*id, bind_group);
    }
}

#[derive(Clone, ShaderType, Resource)]
pub struct WaterPushConstantData {
    pub current_index: i32,
    pub next_index: i32,
    pub next_weight: f32,
}

/// Extracted MSAA samples from main world
#[derive(Resource, Default, Clone, Copy)]
pub struct ExtractedMsaa {
    pub samples: u32,
}

fn extract_water_push_constant_data(mut commands: Commands, time: Extract<Res<Time>>) {
    let _span = info_span!("extract_water_push_constant_data").entered();
    let time = time.elapsed_seconds_wrapped() * 10.0;
    let current_index = (time as i32) % WATER_MATERIAL_NUM_TEXTURES as i32;
    let next_index = (current_index + 1) % WATER_MATERIAL_NUM_TEXTURES as i32;
    let next_weight = time.fract();

    commands.insert_resource(WaterPushConstantData {
        current_index,
        next_index,
        next_weight,
    });
}

/// Extract MSAA from main world to render world
fn extract_msaa(mut extracted: ResMut<ExtractedMsaa>, msaa: Extract<Res<bevy::prelude::Msaa>>) {
    extracted.samples = msaa.samples();
}

/// Pipeline key for water material specialization
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct WaterMaterialPipelineKey {
    pub mesh_key: MeshPipelineKey,
}

/// Water material render pipeline (initialized lazily)
#[derive(Resource)]
pub struct WaterMaterialPipeline {
    pub mesh_pipeline: Option<MeshPipeline>,
    pub material_layout: Option<BindGroupLayout>,
}

impl Default for WaterMaterialPipeline {
    fn default() -> Self {
        Self {
            mesh_pipeline: None,
            material_layout: None,
        }
    }
}

impl WaterMaterialPipeline {
    /// Initialize the pipeline if not already done
    fn initialize(
        &mut self,
        mesh_pipeline: Option<&MeshPipeline>,
        material_layout: Option<&WaterMaterialBindGroupLayout>,
    ) {
        if self.mesh_pipeline.is_some() && self.material_layout.is_some() {
            return;
        }

        if self.mesh_pipeline.is_none() {
            if let Some(pipeline) = mesh_pipeline {
                self.mesh_pipeline = Some(pipeline.clone());
            }
        }

        if self.material_layout.is_none() {
            if let Some(layout_wrapper) = material_layout {
                self.material_layout = layout_wrapper.0.clone();
            }
        }
    }

    fn is_ready(&self) -> bool {
        self.mesh_pipeline.is_some() && self.material_layout.is_some()
    }
}

impl SpecializedMeshPipeline for WaterMaterialPipeline {
    type Key = WaterMaterialPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mesh_pipeline = self.mesh_pipeline.as_ref()
            .expect("WaterMaterialPipeline should be initialized before specialize is called");
        let material_layout = self.material_layout.as_ref()
            .expect("WaterMaterialPipeline should be initialized before specialize is called");
        
        let mut descriptor = mesh_pipeline.specialize(key.mesh_key, layout)?;

        // Apply water material shader
        descriptor.vertex.shader = WATER_MESH_MATERIAL_SHADER_HANDLE.typed().into();
        if let Some(ref mut fragment) = descriptor.fragment {
            fragment.shader = WATER_MESH_MATERIAL_SHADER_HANDLE.typed().into();
        }

        // Add material bind group layout at index 2 (after view at 0, mesh at 1)
        descriptor.layout.insert(2, material_layout.clone());

        // Disable depth write for transparent water
        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = false;

        // Apply blend state
        if let Some(ref mut fragment) = descriptor.fragment {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                });
            }

            // Do not apply color fog to additive blended water
            fragment.shader_defs.push(ShaderDefVal::Bool(
                "ZONE_LIGHTING_DISABLE_COLOR_FOG".into(),
                true,
            ));
        }

        // Add push constants for water animation
        descriptor.push_constant_ranges.push(PushConstantRange {
            stages: ShaderStages::FRAGMENT,
            range: 0..WaterPushConstantData::SHADER_SIZE.get() as u32,
        });

        Ok(descriptor)
    }
}

/// Custom render command to set the water material bind group
pub struct SetWaterMaterialBindGroup<const I: u32>;

impl<P: PhaseItem, const I: u32> RenderCommand<P> for SetWaterMaterialBindGroup<I> {
    type Param = SRes<GpuWaterMaterials>;
    type ViewQuery = ();
    type ItemQuery = Read<Handle<WaterMaterial>>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        material_handle: Option<ROQueryItem<'w, Self::ItemQuery>>,
        gpu_materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material_handle = match material_handle {
            Some(handle) => handle,
            None => return RenderCommandResult::Failure,
        };

        let bind_group = match gpu_materials.into_inner().get(material_handle.id()) {
            Some(bind_group) => bind_group,
            None => return RenderCommandResult::Failure,
        };

        pass.set_bind_group(I as usize, bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct SetWaterMaterialPushConstants<const OFFSET: u32>;
impl<P: PhaseItem, const OFFSET: u32> RenderCommand<P> for SetWaterMaterialPushConstants<OFFSET> {
    type Param = SRes<WaterPushConstantData>;
    type ViewQuery = ();
    type ItemQuery = ();

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewQuery>,
        _: Option<ROQueryItem<'w, Self::ItemQuery>>,
        water_uniform_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let byte_buffer = [0u8; WaterPushConstantData::SHADER_SIZE.get() as usize];
        let mut buffer = encase::StorageBuffer::new(byte_buffer);
        buffer.write(water_uniform_data.as_ref()).unwrap();
        pass.set_push_constants(ShaderStages::FRAGMENT, OFFSET, buffer.as_ref());
        RenderCommandResult::Success
    }
}

type DrawWaterMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetWaterMaterialBindGroup<2>,
    SetZoneLightingBindGroup<3>,
    SetWaterMaterialPushConstants<0>,
    DrawMesh,
);

/// Queue water material meshes for rendering
#[allow(clippy::too_many_arguments)]
fn queue_water_material_meshes(
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    mut water_pipeline: ResMut<WaterMaterialPipeline>,
    mut pipelines: ResMut<bevy::render::render_resource::SpecializedMeshPipelines<WaterMaterialPipeline>>,
    mut transparent_phases: Query<&mut RenderPhase<Transparent3d>>,
    extracted_meshes: Res<ExtractedWaterMeshes>,
    gpu_materials: Res<GpuWaterMaterials>,
    render_meshes: Res<RenderAssets<Mesh>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<ExtractedMsaa>,
    mesh_pipeline: Option<Res<MeshPipeline>>,
    material_layout: Option<Res<WaterMaterialBindGroupLayout>>,
) where
    WaterMaterialPipeline: bevy::render::render_resource::SpecializedMeshPipeline,
{
    let _span = info_span!("queue_water_material_meshes").entered();
    // Lazy initialization of the pipeline
    water_pipeline.initialize(
        mesh_pipeline.as_deref(),
        material_layout.as_deref(),
    );
    
    // Skip if pipeline is not ready yet
    if !water_pipeline.is_ready() {
        return;
    }
    let draw_function = transparent_draw_functions
        .read()
        .get_id::<DrawWaterMaterial>()
        .unwrap();

    for mut transparent_phase in transparent_phases.iter_mut() {
        for (entity, material_handle, mesh_handle) in extracted_meshes.meshes.iter() {
            // Only queue if material bind group is ready
            if gpu_materials.get(material_handle.id()).is_none() {
                continue;
            }

            // Get the mesh to access its layout
            let gpu_mesh = match render_meshes.get(mesh_handle) {
                Some(mesh) => mesh,
                None => continue,
            };

            // Create mesh vertex buffer layout from the GPU mesh
            let mesh_vertex_layout = MeshVertexBufferLayout::new((*gpu_mesh.layout).clone());

            let mesh_key = MeshPipelineKey::from_msaa_samples(msaa.samples);
            let pipeline_key = WaterMaterialPipelineKey { mesh_key };

            // Get the specialized pipeline - specialize returns a Result in Bevy 0.13
            let pipeline = match pipelines.specialize(
                &pipeline_cache,
                &water_pipeline,
                pipeline_key,
                &mesh_vertex_layout,
            ) {
                Ok(pipeline) => pipeline,
                Err(e) => {
                    bevy::log::error!("Failed to specialize water pipeline: {:?}", e);
                    continue;
                }
            };

            // Add to transparent phase (water is transparent)
            transparent_phase.add(Transparent3d {
                distance: 0.0_f32, // Water doesn't need distance sorting for now
                pipeline,
                entity: *entity,
                draw_function,
                batch_range: 0..0,
                dynamic_offset: None,
            });
        }
    }
}

/// Water material asset - does NOT implement Material trait since we use custom pipeline
#[derive(Asset, Debug, Clone, TypePath)]
pub struct WaterMaterial {
    pub textures: Vec<Handle<Image>>,
}

impl Default for WaterMaterial {
    fn default() -> Self {
        Self {
            textures: Vec::new(),
        }
    }
}

// Note: We don't implement Material trait here since we're using a completely custom pipeline.
// The Material trait requires MaterialPlugin which doesn't support texture arrays.
// Instead, we use our own render systems and custom render commands (DrawWaterMaterial).

impl AsBindGroup for WaterMaterial {
    type Data = ();

    fn unprepared_bind_group(
        &self,
        _layout: &BindGroupLayout,
        _render_device: &RenderDevice,
        _image_assets: &RenderAssets<Image>,
        _fallback_image: &FallbackImage,
    ) -> Result<UnpreparedBindGroup<Self::Data>, AsBindGroupError> {
        // Bind groups are prepared separately in prepare_water_material_bind_groups
        // and stored in GpuWaterMaterials resource.
        // The actual bind group is set by SetWaterMaterialBindGroup render command.
        Ok(UnpreparedBindGroup {
            bindings: vec![],
            data: (),
        })
    }

    fn bind_group_layout_entries(_render_device: &RenderDevice) -> Vec<BindGroupLayoutEntry> {
        vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: NonZeroU32::new(WATER_MATERIAL_NUM_TEXTURES as u32),
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ]
    }
}
