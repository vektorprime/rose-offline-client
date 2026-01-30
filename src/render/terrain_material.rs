use std::num::NonZeroU32;

use bevy::{
    asset::{load_internal_asset, AssetApp, Handle, UntypedHandle, Asset, UntypedAssetId},
    log::info_span,
    core_pipeline::core_3d::Opaque3d,
    ecs::{query::QueryItem, system::{lifetimeless::{Read, SRes}, SystemParamItem}},
    pbr::{
        DrawMesh, MeshPipeline, MeshPipelineKey, SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::{
        AlphaMode, App, Commands, Entity, FromWorld, IntoSystemConfigs, Mesh, Plugin, Query, Res, ResMut, Resource,
    },
    reflect::TypePath,
    render::{
        mesh::{MeshVertexBufferLayout, GpuBufferInfo},
        prelude::Shader,
        render_asset::RenderAssets,
        render_phase::{AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline, TrackedRenderPass},
        render_resource::{
            AsBindGroup, AsBindGroupError, BindGroup, BindGroupEntry,
            BindGroupLayout, BindGroupLayoutEntry, BindingResource,
            BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState,
            UnpreparedBindGroup, RenderPipelineDescriptor, SamplerBindingType,
            ShaderRef, ShaderStages, SpecializedMeshPipeline, SpecializedMeshPipelineError,
            SpecializedMeshPipelines, TextureSampleType, TextureViewDimension, VertexFormat,
            PipelineCache, CachedRenderPipelineId,
        },
        renderer::RenderDevice,
        texture::{FallbackImage, Image, GpuImage},
        view::{ExtractedView, ViewTarget},
        Extract, ExtractSchedule, Render, RenderApp, RenderSet,
    },
    utils::Uuid,
};

use crate::render::{
    zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta},
    MESH_ATTRIBUTE_UV_1,
};
use std::any::TypeId;

pub const TERRAIN_MATERIAL_SHADER_HANDLE: UntypedHandle =
    UntypedHandle::Weak(UntypedAssetId::Uuid { type_id: TypeId::of::<Shader>(), uuid: Uuid::from_u128(0x3d7939250aff89cb) });

pub const TERRAIN_MATERIAL_SHADER_HANDLE_TYPED: Handle<Shader> =
    Handle::weak_from_u128(0x3d7939250aff89cb);

pub const TERRAIN_MESH_ATTRIBUTE_TILE_INFO: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_TileInfo", 3855645392, VertexFormat::Uint32);

pub const TERRAIN_MATERIAL_MAX_TEXTURES: usize = 100;

#[derive(Default)]
pub struct TerrainMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TERRAIN_MATERIAL_SHADER_HANDLE_TYPED,
            "shaders/terrain_material.wgsl",
            Shader::from_wgsl
        );

        // Register the TerrainMaterial asset type (normally done by MaterialPlugin)
        app.init_asset::<TerrainMaterial>();

        // NOTE: We don't use MaterialPlugin since we handle bind groups manually
        // due to texture array limitations in Bevy 0.13's AsBindGroup trait

        // Initialize render resources and systems
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<TerrainMaterialBindGroupLayout>()
                .init_resource::<GpuTerrainMaterials>()
                .init_resource::<RenderTerrainMaterials>()
                .init_resource::<ExtractedTerrainMeshes>()
                .init_resource::<TerrainMaterialPipeline>()
                .init_resource::<SpecializedMeshPipelines<TerrainMaterialPipeline>>()
                .init_resource::<ExtractedMsaa>()
                .add_render_command::<Opaque3d, DrawTerrainMaterial>()
                .add_systems(
                    ExtractSchedule,
                    (extract_terrain_materials, extract_terrain_meshes, extract_msaa),
                )
                .add_systems(
                    Render,
                    init_terrain_material_bind_group_layout,
                )
                .add_systems(
                    Render,
                    prepare_terrain_material_bind_groups.after(init_terrain_material_bind_group_layout),
                )
                .add_systems(
                    Render,
                    queue_terrain_material_meshes.in_set(RenderSet::Queue),
                );
        }
    }

    fn finish(&self, _app: &mut App) {
        // Pipeline is initialized lazily in queue_terrain_material_meshes
    }
}

/// Resource holding the bind group layout for terrain materials (initialized lazily)
#[derive(Resource, Clone, Default)]
pub struct TerrainMaterialBindGroupLayout(pub Option<BindGroupLayout>);

/// Initialize the bind group layout on first frame when RenderDevice is available
fn init_terrain_material_bind_group_layout(
    mut layout: ResMut<TerrainMaterialBindGroupLayout>,
    render_device: Res<RenderDevice>,
) {
    if layout.0.is_some() {
        return;
    }

    let bind_group_layout = render_device.create_bind_group_layout(
        "terrain_material_bind_group_layout",
        &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: NonZeroU32::new(TERRAIN_MATERIAL_MAX_TEXTURES as u32),
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    );
    layout.0 = Some(bind_group_layout);
}

/// Resource to store prepared bind groups for terrain materials
#[derive(Resource, Default)]
pub struct GpuTerrainMaterials {
    bind_groups: bevy::utils::HashMap<bevy::asset::AssetId<TerrainMaterial>, BindGroup>,
}

impl GpuTerrainMaterials {
    pub fn get(&self, id: bevy::asset::AssetId<TerrainMaterial>) -> Option<&BindGroup> {
        self.bind_groups.get(&id)
    }

    pub fn insert(&mut self, id: bevy::asset::AssetId<TerrainMaterial>, bind_group: BindGroup) {
        self.bind_groups.insert(id, bind_group);
    }
}

/// Resource to hold extracted terrain materials for render world
/// Uses a HashMap for efficient incremental updates and change tracking
#[derive(Resource, Default)]
pub struct RenderTerrainMaterials {
    pub materials: bevy::utils::HashMap<bevy::asset::AssetId<TerrainMaterial>, TerrainMaterial>,
}

/// Resource to hold extracted terrain meshes for rendering
#[derive(Resource, Default)]
pub struct ExtractedTerrainMeshes {
    pub meshes: Vec<(Entity, Handle<TerrainMaterial>, Handle<Mesh>)>,
}

/// Extract terrain materials from main world to render world
/// Uses incremental updates with count tracking to avoid unnecessary work
fn extract_terrain_materials(
    mut render_materials: ResMut<RenderTerrainMaterials>,
    main_materials: Extract<Res<bevy::asset::Assets<TerrainMaterial>>>,
    mut last_count: bevy::ecs::system::Local<usize>,
) {
    let _span = info_span!("extract_terrain_materials").entered();
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

/// Extract terrain meshes from main world to render world
/// Uses incremental updates to avoid allocating every frame
fn extract_terrain_meshes(
    mut extracted_meshes: ResMut<ExtractedTerrainMeshes>,
    query: Extract<Query<(Entity, &Handle<TerrainMaterial>, &Handle<Mesh>)>>,
    mut local_meshes: bevy::ecs::system::Local<Vec<(Entity, Handle<TerrainMaterial>, Handle<Mesh>)>>,
    mut last_count: bevy::ecs::system::Local<usize>,
) {
    let _span = info_span!("extract_terrain_meshes").entered();
    // Quick check: count entities first without fully iterating
    let current_count = query.iter().count();
    
    // Skip if count hasn't changed and we processed the same number before
    // The actual comparison below will catch any handle changes
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

/// Extracted MSAA samples from main world
#[derive(Resource, Default, Clone, Copy)]
pub struct ExtractedMsaa {
    pub samples: u32,
}

/// Extract MSAA from main world to render world
fn extract_msaa(mut extracted: ResMut<ExtractedMsaa>, msaa: Extract<Res<bevy::prelude::Msaa>>) {
    extracted.samples = msaa.samples();
}

/// Prepares bind groups for terrain materials
fn prepare_terrain_material_bind_groups(
    mut gpu_materials: ResMut<GpuTerrainMaterials>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    render_materials: Res<RenderTerrainMaterials>,
    layout: Res<TerrainMaterialBindGroupLayout>,
) {
    let _span = info_span!("prepare_terrain_material_bind_groups").entered();
    
    // bevy::log::info!("[RENDER DEBUG] prepare_terrain_material_bind_groups called");
    // bevy::log::info!("[RENDER DEBUG]   Render materials count: {}", render_materials.materials.len());
    // bevy::log::info!("[RENDER DEBUG]   GPU materials count: {}", gpu_materials.bind_groups.len());
    // bevy::log::info!("[RENDER DEBUG]   Images loaded: {}", images.iter().count());
    
    // Skip if layout not initialized yet
    let Some(layout) = layout.0.as_ref() else {
        bevy::log::warn!("[RENDER DEBUG] Terrain material bind group layout NOT initialized!");
        return;
    };
    
    //bevy::log::info!("[RENDER DEBUG] Terrain material bind group layout is ready");

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

        // Collect all texture views for the tile array
        let mut tile_textures: Vec<_> = vec![&*fallback_image.d2.texture_view; TERRAIN_MATERIAL_MAX_TEXTURES];
        let mut all_textures_loaded = true;

        for (idx, handle) in material.textures.iter().take(TERRAIN_MATERIAL_MAX_TEXTURES).enumerate() {
            match images.get(handle) {
                Some(image) => tile_textures[idx] = &*image.texture_view,
                None => {
                    all_textures_loaded = false;
                    break;
                }
            }
        }

        if !all_textures_loaded {
            continue; // Skip this material until all textures are loaded
        }

        // Get detail texture view (use fallback)
        let detail_texture_view = &*fallback_image.d2.texture_view;

        // Create bind group with texture array
        let bind_group = render_device.create_bind_group(
            "terrain_material_bind_group",
            layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureViewArray(&tile_textures[..]),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&*fallback_image.d2.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(detail_texture_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&*fallback_image.d2.sampler),
                },
            ],
        );

        gpu_materials.insert(*id, bind_group);
    }
}

/// Pipeline key for terrain material specialization
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainMaterialPipelineKey {
    pub mesh_key: MeshPipelineKey,
}

/// Terrain material render pipeline (initialized lazily)
#[derive(Resource)]
pub struct TerrainMaterialPipeline {
    pub mesh_pipeline: Option<MeshPipeline>,
    pub material_layout: Option<BindGroupLayout>,
    pub zone_lighting_layout: Option<BindGroupLayout>,
}

impl Default for TerrainMaterialPipeline {
    fn default() -> Self {
        Self {
            mesh_pipeline: None,
            material_layout: None,
            zone_lighting_layout: None,
        }
    }
}

impl TerrainMaterialPipeline {
    /// Initialize the pipeline if not already done
    fn initialize(
        &mut self,
        mesh_pipeline: Option<&MeshPipeline>,
        material_layout: Option<&TerrainMaterialBindGroupLayout>,
        zone_lighting_meta: Option<&ZoneLightingUniformMeta>,
    ) {
        if self.mesh_pipeline.is_some() && self.material_layout.is_some() && self.zone_lighting_layout.is_some() {
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

        if self.zone_lighting_layout.is_none() {
            if let Some(meta) = zone_lighting_meta {
                self.zone_lighting_layout = Some(meta.bind_group_layout.clone());
            }
        }
    }

    fn is_ready(&self) -> bool {
        self.mesh_pipeline.is_some() && self.material_layout.is_some() && self.zone_lighting_layout.is_some()
    }
}

impl SpecializedMeshPipeline for TerrainMaterialPipeline {
    type Key = TerrainMaterialPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mesh_pipeline = self.mesh_pipeline.as_ref()
            .expect("TerrainMaterialPipeline should be initialized before specialize is called");
        let material_layout = self.material_layout.as_ref()
            .expect("TerrainMaterialPipeline should be initialized before specialize is called");
        let zone_lighting_layout = self.zone_lighting_layout.as_ref()
            .expect("TerrainMaterialPipeline should be initialized before specialize is called");
        
        let mut descriptor = mesh_pipeline.specialize(key.mesh_key, layout)?;

        // Apply terrain material shader
        descriptor.vertex.shader = TERRAIN_MATERIAL_SHADER_HANDLE.typed().into();
        if let Some(ref mut fragment) = descriptor.fragment {
            fragment.shader = TERRAIN_MATERIAL_SHADER_HANDLE.typed().into();
        }

        // Add material bind group layout at index 2 (after view at 0, mesh at 1)
        descriptor.layout.insert(2, material_layout.clone());

        // Add zone lighting bind group layout at index 3
        descriptor.layout.push(zone_lighting_layout.clone());

        // Apply blend state
        if let Some(ref mut fragment) = descriptor.fragment {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = Some(BlendState {
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
                });
            }
        }

        Ok(descriptor)
    }
}

/// Custom render command to set the terrain material bind group
pub struct SetTerrainMaterialBindGroup<const I: u32>;

impl<P: PhaseItem, const I: u32> RenderCommand<P> for SetTerrainMaterialBindGroup<I> {
    type Param = SRes<GpuTerrainMaterials>;
    type ViewQuery = ();
    type ItemQuery = Read<Handle<TerrainMaterial>>;

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
                bevy::log::warn!("[RENDER CMD] SetTerrainMaterialBindGroup: No material handle!");
                return RenderCommandResult::Failure;
            }
        };

        let bind_group = match gpu_materials.into_inner().get(material_handle.id()) {
            Some(bind_group) => bind_group,
            None => {
                bevy::log::warn!("[RENDER CMD] SetTerrainMaterialBindGroup: No bind group for material {:?}!", material_handle.id());
                return RenderCommandResult::Failure;
            }
        };

        pass.set_bind_group(I as usize, bind_group, &[]);
        RenderCommandResult::Success
    }
}

type DrawTerrainMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetTerrainMaterialBindGroup<2>,
    SetZoneLightingBindGroup<3>,
    DrawMesh,
);

/// Queue terrain material meshes for rendering
#[allow(clippy::too_many_arguments)]
fn queue_terrain_material_meshes(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    mut terrain_pipeline: ResMut<TerrainMaterialPipeline>,
    mut pipelines: ResMut<bevy::render::render_resource::SpecializedMeshPipelines<TerrainMaterialPipeline>>,
    mut opaque_phases: Query<&mut RenderPhase<Opaque3d>>,
    extracted_meshes: Res<ExtractedTerrainMeshes>,
    gpu_materials: Res<GpuTerrainMaterials>,
    render_meshes: Res<RenderAssets<Mesh>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<ExtractedMsaa>,
    mesh_pipeline: Option<Res<MeshPipeline>>,
    material_layout: Option<Res<TerrainMaterialBindGroupLayout>>,
    zone_lighting_meta: Option<Res<ZoneLightingUniformMeta>>,
) {
    let _span = info_span!("queue_terrain_material_meshes").entered();
    
    // bevy::log::info!("[RENDER DEBUG] queue_terrain_material_meshes called");
    // bevy::log::info!("[RENDER DEBUG]   Extracted meshes: {}", extracted_meshes.meshes.len());
    // bevy::log::info!("[RENDER DEBUG]   GPU materials: {}", gpu_materials.bind_groups.len());
    // bevy::log::info!("[RENDER DEBUG]   Render meshes: {}", render_meshes.iter().count());
    // bevy::log::info!("[RENDER DEBUG]   Mesh pipeline available: {}", mesh_pipeline.is_some());
    // bevy::log::info!("[RENDER DEBUG]   Material layout available: {}", material_layout.as_ref().map(|l| l.0.is_some()).unwrap_or(false));
    
    // Lazy initialization of the pipeline
    terrain_pipeline.initialize(
        mesh_pipeline.as_deref(),
        material_layout.as_deref(),
        zone_lighting_meta.as_deref(),
    );
    
    // Skip if pipeline is not ready yet
    if !terrain_pipeline.is_ready() {
        bevy::log::warn!("[RENDER DEBUG] Terrain pipeline NOT READY - cannot queue meshes!");
        return;
    }
    
    //bevy::log::info!("[RENDER DEBUG] Terrain pipeline is ready");
    let draw_function = opaque_draw_functions
        .read()
        .get_id::<DrawTerrainMaterial>()
        .unwrap();

    let mut total_queued = 0u32;
    let mut skipped_no_material = 0u32;
    let mut skipped_no_mesh = 0u32;
    let mut pipeline_failures = 0u32;

    for mut opaque_phase in opaque_phases.iter_mut() {
        for (entity, material_handle, mesh_handle) in extracted_meshes.meshes.iter() {
            // Only queue if material bind group is ready
            if gpu_materials.get(material_handle.id()).is_none() {
                skipped_no_material += 1;
                continue;
            }

            // Get the mesh to access its layout
            let gpu_mesh = match render_meshes.get(mesh_handle) {
                Some(mesh) => mesh,
                None => {
                    skipped_no_mesh += 1;
                    continue;
                }
            };

            // Create mesh vertex buffer layout from the GPU mesh
            // In Bevy 0.13, GpuMesh::layout is Hashed<InnerMeshVertexBufferLayout>
            // We dereference to get the inner value and clone it
            let mesh_vertex_layout = MeshVertexBufferLayout::new((*gpu_mesh.layout).clone());

            let mesh_key = MeshPipelineKey::from_msaa_samples(msaa.samples);
            let pipeline_key = TerrainMaterialPipelineKey { mesh_key };

            // Get the specialized pipeline - specialize returns a Result in Bevy 0.13
            let pipeline = match pipelines.specialize(
                &pipeline_cache,
                &terrain_pipeline,
                pipeline_key,
                &mesh_vertex_layout,
            ) {
                Ok(pipeline) => pipeline,
                Err(e) => {
                    bevy::log::error!("[RENDER DEBUG] Failed to specialize terrain pipeline: {:?}", e);
                    pipeline_failures += 1;
                    continue;
                }
            };

            // In Bevy 0.13, Opaque3d uses asset_id for sorting
            opaque_phase.add(Opaque3d {
                asset_id: mesh_handle.id(),
                pipeline,
                entity: *entity,
                draw_function,
                batch_range: 0..0,
                dynamic_offset: None,
            });
            total_queued += 1;
        }
    }
    
    // bevy::log::info!("[RENDER DEBUG] Terrain queue summary: queued={}, skipped_no_material={}, skipped_no_mesh={}, pipeline_failures={}",
    //     total_queued, skipped_no_material, skipped_no_mesh, pipeline_failures);
}

/// Terrain material asset - does NOT implement Material trait since we use custom pipeline
#[derive(Asset, Debug, Clone, TypePath)]
pub struct TerrainMaterial {
    pub textures: Vec<Handle<Image>>,
}

impl Default for TerrainMaterial {
    fn default() -> Self {
        Self {
            textures: Vec::new(),
        }
    }
}

// Note: We don't implement Material trait here since we're using a completely custom pipeline.
// The Material trait requires MaterialPlugin which doesn't support texture arrays.
// Instead, we use our own render systems (extract_terrain_materials, prepare_terrain_material_bind_groups,
// queue_terrain_material_meshes) and custom render commands (DrawTerrainMaterial).

impl AsBindGroup for TerrainMaterial {
    type Data = ();

    fn unprepared_bind_group(
        &self,
        _layout: &BindGroupLayout,
        _render_device: &RenderDevice,
        _images: &RenderAssets<Image>,
        _fallback_image: &FallbackImage,
    ) -> Result<UnpreparedBindGroup<Self::Data>, AsBindGroupError> {
        // Bind groups are prepared separately in prepare_terrain_material_bind_groups
        // and stored in GpuTerrainMaterials resource.
        // The actual bind group is set by SetTerrainMaterialBindGroup render command.
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
                count: NonZeroU32::new(TERRAIN_MATERIAL_MAX_TEXTURES as u32),
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ]
    }
}

use bevy::render::mesh::MeshVertexAttribute;
