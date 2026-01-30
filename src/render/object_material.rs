use bevy::{
    asset::{load_internal_asset, AssetApp, Handle, UntypedHandle, Asset, UntypedAssetId},
    log::info_span,
    core_pipeline::core_3d::{Opaque3d, Transparent3d},
    ecs::{query::QueryItem, system::{lifetimeless::{Read, SRes}, SystemParamItem}},
    pbr::{
        DrawMesh, MeshPipeline, MeshPipelineKey, SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::{
        AlphaMode, App, Commands, Component, Entity, FromWorld, IntoSystemConfigs, Mesh, Plugin, Query, Res, ResMut, Resource, Vec3, With, World,
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
            PipelineCache, CachedRenderPipelineId,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{FallbackImage, Image, GpuImage},
        view::{ExtractedView, ViewTarget},
        Extract, ExtractSchedule, Render, RenderApp, RenderSet,
    },
    math::Vec2,
    utils::Uuid,
};

use rose_file_readers::{ZscMaterialBlend, ZscMaterialGlow};

use crate::render::{
    custom_mesh_pipeline::{CustomMeshPipelineKey, HashableAlphaMode, MeshVertexLayoutBuilder, PipelineDescriptorBuilder},
    zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta},
    MESH_ATTRIBUTE_UV_1,
};

use std::any::TypeId;

pub const OBJECT_MATERIAL_SHADER_HANDLE: UntypedHandle =
    UntypedHandle::Weak(UntypedAssetId::Uuid {
        type_id: TypeId::of::<Shader>(),
        uuid: Uuid::from_u128(0xb7ebbc00ea16d3c7),
    });

#[derive(Default)]
pub struct ObjectMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for ObjectMaterialPlugin {
    fn build(&self, app: &mut App) {
        //bevy::log::info!("[OBJECT MATERIAL] Building ObjectMaterialPlugin, prepass_enabled={}", self.prepass_enabled);
        
        // Load the internal asset using the Bevy 0.13 API
        load_internal_asset!(
            app,
            OBJECT_MATERIAL_SHADER_HANDLE.typed::<Shader>(),
            "shaders/object_material.wgsl",
            bevy::render::render_resource::Shader::from_wgsl
        );
        //bevy::log::info!("[OBJECT MATERIAL] Shader loaded successfully");

        // Register the ObjectMaterial asset type (normally done by MaterialPlugin)
        app.init_asset::<ObjectMaterial>();

        // Register ObjectMaterialClipFace component
        app.add_plugins(ExtractComponentPlugin::<ObjectMaterialClipFace>::extract_visible());
        //bevy::log::info!("[OBJECT MATERIAL] ExtractComponentPlugin<ObjectMaterialClipFace> added");

        // Initialize render resources and systems
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<RenderObjectMaterials>()
                .init_resource::<GpuObjectMaterials>()
                .init_resource::<ExtractedObjectMeshes>()
                .init_resource::<ObjectMaterialPipeline>()
                .init_resource::<SpecializedMeshPipelines<ObjectMaterialPipeline>>()
                .add_render_command::<Opaque3d, DrawObjectMaterial>()
                .add_render_command::<Transparent3d, DrawObjectMaterial>()
                .add_systems(
                    ExtractSchedule,
                    (extract_object_materials, extract_object_meshes),
                )
                .add_systems(
                    Render,
                    prepare_object_material_bind_groups,
                )
                .add_systems(
                    Render,
                    queue_object_material_meshes.in_set(RenderSet::Queue),
                );
        }
        //bevy::log::info!("[OBJECT MATERIAL] ObjectMaterialPlugin build complete");
    }

    fn finish(&self, app: &mut App) {
        // Pipeline is initialized lazily in queue_object_material_meshes
    }
}

#[derive(Copy, Clone, Component, Reflect)]
pub enum ObjectMaterialClipFace {
    First(u32),
    Last(u32),
}

impl ExtractComponent for ObjectMaterialClipFace {
    type QueryData = &'static Self;
    type QueryFilter = With<Handle<ObjectMaterial>>;
    type Out = Self;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(*item)
    }
}

/// Resource to hold extracted object materials for render world
#[derive(Resource, Default)]
pub struct RenderObjectMaterials {
    pub materials: bevy::utils::HashMap<bevy::asset::AssetId<ObjectMaterial>, ObjectMaterial>,
}

/// Resource to hold extracted object meshes for rendering
#[derive(Resource, Default)]
pub struct ExtractedObjectMeshes {
    pub meshes: Vec<(Entity, Handle<ObjectMaterial>, Handle<Mesh>)>,
}

/// Extract object materials from main world to render world
fn extract_object_materials(
    mut render_materials: ResMut<RenderObjectMaterials>,
    main_materials: Extract<Res<bevy::asset::Assets<ObjectMaterial>>>,
    mut last_count: bevy::ecs::system::Local<usize>,
) {
    let _span = info_span!("extract_object_materials").entered();
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

/// Extract object meshes from main world to render world
fn extract_object_meshes(
    mut extracted_meshes: ResMut<ExtractedObjectMeshes>,
    query: Extract<Query<(Entity, &Handle<ObjectMaterial>, &Handle<Mesh>)>>,
    mut local_meshes: bevy::ecs::system::Local<Vec<(Entity, Handle<ObjectMaterial>, Handle<Mesh>)>>,
    mut last_count: bevy::ecs::system::Local<usize>,
) {
    let _span = info_span!("extract_object_meshes").entered();
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

/// Resource to store prepared bind groups for object materials
#[derive(Resource, Default)]
pub struct GpuObjectMaterials {
    bind_groups: bevy::utils::HashMap<bevy::asset::AssetId<ObjectMaterial>, BindGroup>,
}

impl GpuObjectMaterials {
    pub fn get(&self, id: bevy::asset::AssetId<ObjectMaterial>) -> Option<&BindGroup> {
        self.bind_groups.get(&id)
    }

    pub fn insert(&mut self, id: bevy::asset::AssetId<ObjectMaterial>, bind_group: BindGroup) {
        self.bind_groups.insert(id, bind_group);
    }
}

/// Prepares bind groups for object materials
fn prepare_object_material_bind_groups(
    mut gpu_materials: ResMut<GpuObjectMaterials>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    render_materials: Res<RenderObjectMaterials>,
    mut pipeline: ResMut<ObjectMaterialPipeline>,
    zone_lighting_meta: Res<ZoneLightingUniformMeta>,
    mesh_pipeline: Option<Res<MeshPipeline>>,
) {
    let _span = info_span!("prepare_object_material_bind_groups").entered();

    // Lazy initialization of the pipeline if not already done
    pipeline.initialize(mesh_pipeline.as_deref(), &render_device, &zone_lighting_meta);

    // Skip if pipeline layout not initialized yet
    let material_layout = match pipeline.material_layout.as_ref() {
        Some(layout) => layout,
        None => {
            bevy::log::debug!("[OBJECT MATERIAL] Material bind group layout not yet initialized, will retry next frame");
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
            Err(e) => {
                bevy::log::warn!("[OBJECT MATERIAL] Failed to prepare bind group for material {:?}: {:?}", id, e);
                continue;
            }
        };

        // Get texture views (use fallback if not loaded)
        let base_texture_view = match material.base_texture.as_ref().and_then(|h| images.get(h)) {
            Some(image) => &*image.texture_view,
            None => &*fallback_image.d2.texture_view,
        };

        let lightmap_texture_view = match material.lightmap_texture.as_ref().and_then(|h| images.get(h)) {
            Some(image) => &*image.texture_view,
            None => &*fallback_image.d2.texture_view,
        };

        let specular_texture_view = match material.specular_texture.as_ref().and_then(|h| images.get(h)) {
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
            "object_material_bind_group",
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
                    resource: BindingResource::TextureView(lightmap_texture_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&*fallback_image.d2.sampler),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(specular_texture_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::Sampler(&*fallback_image.d2.sampler),
                },
            ],
        );

        gpu_materials.insert(*id, bind_group);
    }
}

/// Pipeline key for object material specialization
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ObjectMaterialPipelineKey {
    pub mesh_key: MeshPipelineKey,
    pub custom_key: CustomMeshPipelineKey,
}

/// Object material render pipeline (initialized lazily)
#[derive(Resource, Default)]
pub struct ObjectMaterialPipeline {
    pub mesh_pipeline: Option<MeshPipeline>,
    pub material_layout: Option<BindGroupLayout>,
    pub zone_lighting_layout: Option<BindGroupLayout>,
}

impl ObjectMaterialPipeline {
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
            "object_material_bind_group_layout",
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
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 6,
                    visibility: ShaderStages::FRAGMENT,
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

impl SpecializedMeshPipeline for ObjectMaterialPipeline {
    type Key = ObjectMaterialPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mesh_pipeline = self.mesh_pipeline.as_ref()
            .expect("ObjectMaterialPipeline should be initialized before specialize is called");
        let material_layout = self.material_layout.as_ref()
            .expect("ObjectMaterialPipeline should be initialized before specialize is called");
        let zone_lighting_layout = self.zone_lighting_layout.as_ref()
            .expect("ObjectMaterialPipeline should be initialized before specialize is called");
        
        // Build base descriptor using shared infrastructure
        let mut descriptor = PipelineDescriptorBuilder::build_base_descriptor(
            mesh_pipeline,
            key.mesh_key,
            &key.custom_key,
            layout,
        )?;
        
        // Check if mesh actually has UV_1 attribute at the expected location
        // This must match what the shader expects when VERTEX_UVS_LIGHTMAP is defined
        let mesh_has_uv_1 = layout.get_layout(&[MESH_ATTRIBUTE_UV_1.at_shader_location(3)]).is_ok();
        
        // Only enable lightmap UVs if:
        // 1. The material says it has lightmaps (key.custom_key.vertex_uvs_lightmap)
        // 2. The mesh actually has UV_1 data
        // This prevents pipeline validation errors when the shader expects data the mesh doesn't have
        let vertex_uvs_lightmap = key.custom_key.vertex_uvs_lightmap && mesh_has_uv_1;
        
        // Build the correct vertex attributes list based on actual mesh capabilities
        let mut custom_key_for_build = key.custom_key;
        custom_key_for_build.vertex_uvs_lightmap = vertex_uvs_lightmap;
        
        // Rebuild the vertex buffer layout to ensure correct attribute locations
        let vertex_attributes = MeshVertexLayoutBuilder::build_vertex_attributes(&custom_key_for_build);
        let vertex_buffer_layout = MeshVertexLayoutBuilder::get_layout(layout, &vertex_attributes)?;
        
        // Replace the vertex buffer layout with our custom one
        // to ensure the shader gets the attributes at the expected locations
        descriptor.vertex.buffers = vec![vertex_buffer_layout];

        // Apply object material shader
        descriptor.vertex.shader = OBJECT_MATERIAL_SHADER_HANDLE.typed().into();
        if let Some(ref mut fragment) = descriptor.fragment {
            fragment.shader = OBJECT_MATERIAL_SHADER_HANDLE.typed().into();
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

        if vertex_uvs_lightmap {
            descriptor
                .vertex
                .shader_defs
                .push("VERTEX_UVS_LIGHTMAP".into());
            if let Some(fragment) = descriptor.fragment.as_mut() {
                fragment
                    .shader_defs
                    .push("VERTEX_UVS_LIGHTMAP".into());
            }
        } else if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment
                .shader_defs
                .push("ZONE_LIGHTING_CHARACTER".into());
        }

        if key.custom_key.skinned {
            descriptor
                .vertex
                .shader_defs
                .push("SKINNED".into());
        }

        Ok(descriptor)
    }
}

/// Custom render command to set the object material bind group
pub struct SetObjectMaterialBindGroup<const I: u32>;

impl<P: PhaseItem, const I: u32> RenderCommand<P> for SetObjectMaterialBindGroup<I> {
    type Param = SRes<GpuObjectMaterials>;
    type ViewQuery = ();
    type ItemQuery = Read<Handle<ObjectMaterial>>;

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
                bevy::log::warn!("[RENDER CMD] SetObjectMaterialBindGroup: No material handle!");
                return RenderCommandResult::Failure;
            }
        };

        let bind_group = match gpu_materials.into_inner().get(material_handle.id()) {
            Some(bind_group) => bind_group,
            None => {
                bevy::log::warn!("[RENDER CMD] SetObjectMaterialBindGroup: No bind group for material {:?}!", material_handle.id());
                return RenderCommandResult::Failure;
            }
        };

        pass.set_bind_group(I as usize, bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawObjectMesh;
impl<P: PhaseItem> RenderCommand<P> for DrawObjectMesh {
    type Param = SRes<RenderAssets<Mesh>>;
    type ViewQuery = ();
    type ItemQuery = (Read<Handle<Mesh>>, Option<Read<ObjectMaterialClipFace>>);

    #[inline]
    fn render<'w>(
        _: &P,
        _: bevy::ecs::query::ROQueryItem<'_, Self::ViewQuery>,
        item: Option<bevy::ecs::query::ROQueryItem<'_, Self::ItemQuery>>,
        meshes: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (mesh_handle, clip_face) = match item {
            Some((mesh_handle, clip_face)) => (mesh_handle, clip_face),
            None => return RenderCommandResult::Failure,
        };
        let (start_index_offset, end_index_offset) = if let Some(clip_face) = clip_face {
            match clip_face {
                ObjectMaterialClipFace::First(num_faces) => (num_faces * 3, 0),
                ObjectMaterialClipFace::Last(num_faces) => (0, num_faces * 3),
            }
        } else {
            (0, 0)
        };

        if let Some(gpu_mesh) = meshes.into_inner().get(mesh_handle) {
            pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
            match &gpu_mesh.buffer_info {
                GpuBufferInfo::Indexed {
                    buffer,
                    index_format,
                    count,
                } => {
                    let start_index = start_index_offset;
                    let end_index = *count - end_index_offset;
                    pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                    pass.draw_indexed(start_index..end_index, 0, 0..1);
                }
                GpuBufferInfo::NonIndexed => {
                    let start_vertex = start_index_offset;
                    let end_vertex = gpu_mesh.vertex_count - end_index_offset;
                    pass.draw(start_vertex..end_vertex, 0..1);
                }
            }
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}

type DrawObjectMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetObjectMaterialBindGroup<2>,
    SetZoneLightingBindGroup<3>,
    DrawObjectMesh,
);

// NOTE: These must match the bit flags in shaders/object_material.wgsl!
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct ObjectMaterialFlags: u32 {
        const ALPHA_MODE_OPAQUE          = (1 << 0);
        const ALPHA_MODE_MASK            = (1 << 1);
        const ALPHA_MODE_BLEND           = (1 << 2);
        const HAS_ALPHA_VALUE            = (1 << 3);
        const SPECULAR                   = (1 << 4);
        const NONE                       = 0;
    }
}

/// Uniform data for ObjectMaterial - matches the shader layout
#[derive(Clone, Copy, Debug, Default, Reflect, bevy::render::render_resource::encase::ShaderType)]
pub struct ObjectMaterialUniformData {
    pub flags: u32,
    pub alpha_cutoff: f32,
    pub alpha_value: f32,
    pub lightmap_uv_offset_x: f32,
    pub lightmap_uv_offset_y: f32,
    pub lightmap_uv_scale: f32,
    // Padding to ensure 16-byte alignment
    pub _padding: f32,
}

impl AsBindGroupShaderType<ObjectMaterialUniformData> for ObjectMaterial {
    fn as_bind_group_shader_type(&self, _images: &RenderAssets<Image>) -> ObjectMaterialUniformData {
        let mut flags = ObjectMaterialFlags::NONE;
        let mut alpha_cutoff = 0.5;
        let mut alpha_value = 1.0;

        if self.specular_texture.is_some() {
            flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE | ObjectMaterialFlags::SPECULAR;
            alpha_cutoff = 1.0;
        } else {
            if self.alpha_enabled {
                flags |= ObjectMaterialFlags::ALPHA_MODE_BLEND;

                if let Some(alpha_ref) = self.alpha_test {
                    flags |= ObjectMaterialFlags::ALPHA_MODE_MASK;
                    alpha_cutoff = alpha_ref;
                }
            } else {
                flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE;
            }

            if let Some(material_alpha_value) = self.alpha_value {
                if material_alpha_value == 1.0 {
                    flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE;
                } else {
                    flags |= ObjectMaterialFlags::HAS_ALPHA_VALUE;
                    alpha_value = material_alpha_value;
                }
            }
        }

        ObjectMaterialUniformData {
            flags: flags.bits(),
            alpha_cutoff,
            alpha_value,
            lightmap_uv_offset_x: self.lightmap_uv_offset.x,
            lightmap_uv_offset_y: self.lightmap_uv_offset.y,
            lightmap_uv_scale: self.lightmap_uv_scale,
            _padding: 0.0,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Reflect, Hash, PartialEq)]
pub enum ObjectMaterialBlend {
    #[default]
    Normal,
    Lighten,
}

impl From<ZscMaterialBlend> for ObjectMaterialBlend {
    fn from(zsc: ZscMaterialBlend) -> Self {
        match zsc {
            ZscMaterialBlend::Normal => ObjectMaterialBlend::Normal,
            ZscMaterialBlend::Lighten => ObjectMaterialBlend::Lighten,
        }
    }
}

#[derive(Copy, Clone, Debug, Reflect, PartialEq)]
pub enum ObjectMaterialGlow {
    Simple(Vec3),
    Light(Vec3),
    Texture(Vec3),
    TextureLight(Vec3),
    Alpha(Vec3),
}

impl From<ZscMaterialGlow> for ObjectMaterialGlow {
    fn from(zsc: ZscMaterialGlow) -> Self {
        match zsc {
            ZscMaterialGlow::Simple(value) => {
                ObjectMaterialGlow::Simple(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Light(value) => {
                ObjectMaterialGlow::Light(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Texture(value) => {
                ObjectMaterialGlow::Texture(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::TextureLight(value) => {
                ObjectMaterialGlow::TextureLight(Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Alpha(value) => {
                ObjectMaterialGlow::Alpha(Vec3::new(value.x, value.y, value.z))
            }
        }
    }
}

#[derive(Asset, AsBindGroup, Debug, Clone, Reflect, PartialEq)]
#[bind_group_data(ObjectMaterialKey)]
#[uniform(0, ObjectMaterialUniformData)]
pub struct ObjectMaterial {
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Option<Handle<Image>>,

    #[texture(3)]
    #[sampler(4)]
    pub lightmap_texture: Option<Handle<Image>>,

    #[texture(5)]
    #[sampler(6)]
    pub specular_texture: Option<Handle<Image>>,

    pub lightmap_uv_offset: Vec2,
    pub lightmap_uv_scale: f32,
    pub alpha_value: Option<f32>,
    pub alpha_enabled: bool,
    pub alpha_test: Option<f32>,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
    pub skinned: bool,
    pub blend: ObjectMaterialBlend,
    pub glow: Option<ObjectMaterialGlow>,
}

impl Default for ObjectMaterial {
    fn default() -> Self {
        Self {
            base_texture: None,
            alpha_value: None,
            alpha_enabled: false,
            alpha_test: None,
            two_sided: false,
            z_test_enabled: true,
            z_write_enabled: true,
            specular_texture: None,
            skinned: false,
            blend: ObjectMaterialBlend::Normal,
            glow: None,
            lightmap_texture: None,
            lightmap_uv_offset: Vec2::new(0.0, 0.0),
            lightmap_uv_scale: 1.0,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ObjectMaterialKey {
    has_lightmap: bool,
    two_sided: bool,
    z_test_enabled: bool,
    z_write_enabled: bool,
}

impl From<&ObjectMaterial> for ObjectMaterialKey {
    fn from(material: &ObjectMaterial) -> Self {
        ObjectMaterialKey {
            has_lightmap: material.lightmap_texture.is_some(),
            two_sided: material.two_sided,
            z_test_enabled: material.z_test_enabled,
            z_write_enabled: material.z_write_enabled,
        }
    }
}

/// Queue object material meshes for rendering
#[allow(clippy::too_many_arguments)]
fn queue_object_material_meshes(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    mut object_pipeline: ResMut<ObjectMaterialPipeline>,
    mut pipelines: ResMut<bevy::render::render_resource::SpecializedMeshPipelines<ObjectMaterialPipeline>>,
    mut opaque_phases: Query<&mut RenderPhase<Opaque3d>>,
    mut transparent_phases: Query<&mut RenderPhase<Transparent3d>>,
    extracted_meshes: Res<ExtractedObjectMeshes>,
    gpu_materials: Res<GpuObjectMaterials>,
    render_materials: Res<RenderObjectMaterials>,
    render_meshes: Res<RenderAssets<Mesh>>,
    pipeline_cache: Res<PipelineCache>,
    mesh_pipeline: Option<Res<MeshPipeline>>,
    render_device: Res<RenderDevice>,
    zone_lighting_meta: Res<ZoneLightingUniformMeta>,
) {
    let _span = info_span!("queue_object_material_meshes").entered();
    
    // Lazy initialization of the pipeline
    object_pipeline.initialize(mesh_pipeline.as_deref(), &render_device, &zone_lighting_meta);
    
    // Skip if pipeline is not ready yet
    if !object_pipeline.is_ready() {
        bevy::log::warn!("[OBJECT MATERIAL] Pipeline NOT READY - cannot queue meshes!");
        return;
    }
    
    let opaque_draw_function = opaque_draw_functions
        .read()
        .get_id::<DrawObjectMaterial>()
        .unwrap();
    let transparent_draw_function = transparent_draw_functions
        .read()
        .get_id::<DrawObjectMaterial>()
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

        // Get the mesh to access its layout
        let gpu_mesh = match render_meshes.get(mesh_handle) {
            Some(mesh) => mesh,
            None => {
                skipped_no_mesh += 1;
                continue;
            }
        };

        // Get the material to determine alpha mode
        let material = match render_materials.materials.get(&material_handle.id()) {
            Some(mat) => mat,
            None => {
                skipped_no_material += 1;
                continue;
            }
        };

        // Determine alpha mode based on material properties
        // Follow the same logic as in ObjectMaterial::as_bind_group_shader_type
        let alpha_mode = if material.specular_texture.is_some() {
            AlphaMode::Opaque
        } else if material.alpha_enabled {
            if material.alpha_test.is_some() {
                AlphaMode::Mask(material.alpha_test.unwrap_or(0.5))
            } else {
                AlphaMode::Blend
            }
        } else {
            // alpha_enabled is false
            if let Some(alpha_value) = material.alpha_value {
                if alpha_value == 1.0 {
                    AlphaMode::Opaque
                } else {
                    AlphaMode::Blend
                }
            } else {
                AlphaMode::Opaque
            }
        };

        // Check if mesh has lightmap UVs (UV_1 attribute)
        // This is required for the shader to properly match vertex outputs with fragment inputs
        // The shader expects lightmap_uv at location 3 when VERTEX_UVS_LIGHTMAP is defined
        let has_lightmap_uv = (*gpu_mesh.layout)
            .get_layout(&[MESH_ATTRIBUTE_UV_1.at_shader_location(3)])
            .is_ok();

        // DIAGNOSTIC: Log mesh UV_1 detection
        //bevy::log::info!("[QUEUE DEBUG] Entity {:?}: has_lightmap_uv={}", entity, has_lightmap_uv);

        // Create mesh vertex buffer layout from the GPU mesh
        let mesh_vertex_layout = MeshVertexBufferLayout::new((*gpu_mesh.layout).clone());
        
        // DIAGNOSTIC: Check if the created layout has UV_1 at location 3
        let layout_has_uv_1_loc3 = mesh_vertex_layout.get_layout(&[MESH_ATTRIBUTE_UV_1.at_shader_location(3)]).is_ok();
        //bevy::log::info!("[QUEUE DEBUG]   Mesh vertex layout has UV_1 at location 3: {}", layout_has_uv_1_loc3);
        
        // // DIAGNOSTIC: Check what attributes are actually in the mesh layout
        // if let Ok(vertex_buffer_layout) = mesh_vertex_layout.get_layout(&[]) {
        //     //bevy::log::info!("[QUEUE DEBUG]   Vertex buffer layout has {} attributes", vertex_buffer_layout.attributes.len());
        //     for (i, attr) in vertex_buffer_layout.attributes.iter().enumerate() {
        //         //bevy::log::info!("[QUEUE DEBUG]     Attr[{}] location={} format={:?}", i, attr.shader_location, attr.format);
        //     }
        // }

        // Create pipeline key
        let mesh_key = MeshPipelineKey::from_primitive_topology(
            bevy::render::render_resource::PrimitiveTopology::TriangleList
        );
        let custom_key = CustomMeshPipelineKey::new()
            .with_two_sided(material.two_sided)
            .with_alpha_mode(alpha_mode)
            .with_vertex_uvs_lightmap(has_lightmap_uv);
        let pipeline_key = ObjectMaterialPipelineKey { mesh_key, custom_key };

        // Get the specialized pipeline
        let pipeline = match pipelines.specialize(
            &pipeline_cache,
            &object_pipeline,
            pipeline_key,
            &mesh_vertex_layout,
        ) {
            Ok(pipeline) => pipeline,
            Err(e) => {
                bevy::log::error!("[OBJECT MATERIAL] Failed to specialize pipeline: {:?}", e);
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
