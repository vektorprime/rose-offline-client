use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use bevy::{
    asset::{Asset, Handle, UntypedAssetId, UntypedHandle, load_internal_asset},
    ecs::{
        query::QueryItem,
        system::{
            SystemParamItem, lifetimeless::{Read, SRes}
        },
    },
    math::Vec2,
    pbr::{
        AlphaMode, MeshPipelineKey, SetMaterialBindGroup, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::{
        App, Component, FromWorld, Material, MaterialPlugin, Mesh, Plugin,
        Vec3, With, World,
    },
    reflect::Reflect,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{GpuBufferInfo, MeshVertexBufferLayout},
        render_asset::RenderAssets,
        render_phase::{
            PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            AsBindGroup, AsBindGroupError, BindGroup, BindGroupLayout, BlendComponent, BlendFactor, BlendOperation, BlendState, CompareFunction, RenderPipelineDescriptor, Shader, ShaderDefVal, ShaderRef, SpecializedMeshPipelineError, encase::ShaderType, UnpreparedBindGroup
        },
        renderer::RenderDevice,
        texture::{Image, FallbackImage},
    },
    utils::Uuid,
};
use dashmap::DashMap;
use lazy_static::lazy_static;
use log::info;
use encase;

use rose_file_readers::{ZscMaterialBlend, ZscMaterialGlow};

use crate::render::{
    zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta},
    MESH_ATTRIBUTE_UV_1,
};

use std::any::TypeId;

// Bind group cache for ObjectMaterial
lazy_static! {
    static ref OBJECT_BIND_GROUP_CACHE: DashMap<u64, BindGroup> = DashMap::new();
    static ref OBJECT_CACHE_STATS: CacheStats = CacheStats::new();
}

struct CacheStats {
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
}

impl CacheStats {
    fn new() -> Self {
        Self {
            hits: std::sync::atomic::AtomicU64::new(0),
            misses: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn record_hit(&self) {
        self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn record_miss(&self) {
        self.misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn get_stats(&self) -> (u64, u64, usize) {
        let hits = self.hits.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.misses.load(std::sync::atomic::Ordering::Relaxed);
        let total_entries = OBJECT_BIND_GROUP_CACHE.len();
        (hits, misses, total_entries)
    }
}

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
        // Load the internal asset using the Bevy 0.13 API
        load_internal_asset!(
            app,
            OBJECT_MATERIAL_SHADER_HANDLE.typed::<Shader>(),
            "shaders/object_material.wgsl",
            bevy::render::render_resource::Shader::from_wgsl
        );

        app.add_plugins(ExtractComponentPlugin::<ObjectMaterialClipFace>::extract_visible());

        app.register_type::<ObjectMaterial>();

        app.add_plugins(MaterialPlugin::<ObjectMaterial> {
            prepass_enabled: self.prepass_enabled,
            ..Default::default()
        });

        app.register_type::<ObjectMaterialClipFace>();
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
    SetMaterialBindGroup<ObjectMaterial, 1>,
    SetMeshBindGroup<2>,
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

#[derive(Clone, ShaderType)]
pub struct ObjectMaterialUniformData {
    pub flags: u32,
    pub alpha_cutoff: f32,
    pub alpha_value: f32,
    pub lightmap_uv_offset: Vec2,
    pub lightmap_uv_scale: f32,
}

impl From<&ObjectMaterial> for ObjectMaterialUniformData {
    fn from(material: &ObjectMaterial) -> ObjectMaterialUniformData {
        let mut flags = ObjectMaterialFlags::NONE;
        let mut alpha_cutoff = 0.5;
        let mut alpha_value = 1.0;

        if material.specular_texture.is_some() {
            flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE | ObjectMaterialFlags::SPECULAR;
            alpha_cutoff = 1.0;
        } else {
            if material.alpha_enabled {
                flags |= ObjectMaterialFlags::ALPHA_MODE_BLEND;

                if let Some(alpha_ref) = material.alpha_test {
                    flags |= ObjectMaterialFlags::ALPHA_MODE_MASK;
                    alpha_cutoff = alpha_ref;
                }
            } else {
                flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE;
            }

            if let Some(material_alpha_value) = material.alpha_value {
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
            lightmap_uv_offset: material.lightmap_uv_offset,
            lightmap_uv_scale: material.lightmap_uv_scale,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Reflect, Hash)]
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

#[derive(Copy, Clone, Debug, Reflect)]
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

#[derive(Asset, Debug, Clone, Reflect)]
pub struct ObjectMaterial {
    pub base_texture: Option<Handle<Image>>,
    pub lightmap_texture: Option<Handle<Image>>,
    pub lightmap_uv_offset: Vec2,
    pub lightmap_uv_scale: f32,
    pub specular_texture: Option<Handle<Image>>,
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

#[derive(Clone)]
pub struct ObjectMaterialPipelineData {
    pub zone_lighting_layout: BindGroupLayout,
}

impl FromWorld for ObjectMaterialPipelineData {
    fn from_world(world: &mut World) -> Self {
        ObjectMaterialPipelineData {
            zone_lighting_layout: world
                .resource::<ZoneLightingUniformMeta>()
                .bind_group_layout
                .clone(),
        }
    }
}

impl Material for ObjectMaterial {
    fn vertex_shader() -> ShaderRef {
        OBJECT_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn fragment_shader() -> ShaderRef {
        OBJECT_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        OBJECT_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        OBJECT_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn alpha_mode(&self) -> bevy::prelude::AlphaMode {
        // Count texture handles for logging
        let texture_count = [
            self.base_texture.is_some(),
            self.lightmap_texture.is_some(),
            self.specular_texture.is_some(),
        ].iter().filter(|&&x| x).count();

        info!("[MATERIAL LIFECYCLE] ObjectMaterial alpha_mode called with {} textures (base: {}, lightmap: {}, specular: {})",
            texture_count,
            self.base_texture.is_some(),
            self.lightmap_texture.is_some(),
            self.specular_texture.is_some());

        let mut alpha_mode;

        if self.specular_texture.is_some() {
            alpha_mode = AlphaMode::Opaque;
        } else {
            if self.alpha_enabled {
                alpha_mode = AlphaMode::Blend;

                if let Some(alpha_ref) = self.alpha_test {
                    alpha_mode = AlphaMode::Mask(alpha_ref);
                }
            } else {
                alpha_mode = AlphaMode::Opaque;
            }

            if let Some(material_alpha_value) = self.alpha_value {
                if material_alpha_value == 1.0 {
                    alpha_mode = AlphaMode::Opaque;
                } else {
                    alpha_mode = AlphaMode::Blend;
                }
            }
        }

        alpha_mode
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = key.bind_group_data.z_write_enabled;

        if !key.bind_group_data.z_test_enabled {
            descriptor.depth_stencil.as_mut().unwrap().depth_compare = CompareFunction::Always;
        }

        if key.bind_group_data.two_sided {
            descriptor.primitive.cull_mode = None;
        }

        let mut vertex_attributes = vec![
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ];

        if key.bind_group_data.has_lightmap {
            descriptor
                .vertex
                .shader_defs
                .push(ShaderDefVal::Bool("VERTEX_UVS_LIGHTMAP".into(), true));

            if let Some(fragment) = descriptor.fragment.as_mut() {
                fragment
                    .shader_defs
                    .push(ShaderDefVal::Bool("VERTEX_UVS_LIGHTMAP".into(), true));
            }

            vertex_attributes.push(MESH_ATTRIBUTE_UV_1.at_shader_location(3));
        } else if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment
                .shader_defs
                .push(ShaderDefVal::Bool("ZONE_LIGHTING_CHARACTER".into(), true));
        }

        if layout.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
            && layout.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        {
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(4));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(5));
        }

        descriptor.vertex.buffers = vec![layout.get_layout(&vertex_attributes)?];

        if key.mesh_key.contains(MeshPipelineKey::DEPTH_PREPASS)
            || key.mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS)
        {
            return Ok(());
        }

        if let Some(fragment) = descriptor.fragment.as_mut() {
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

        Ok(())
    }
}

impl AsBindGroup for ObjectMaterial {
    type Data = ObjectMaterialKey;

    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        images: &RenderAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> Result<UnpreparedBindGroup<Self::Data>, AsBindGroupError> {
        // Generate cache key from texture handles and material properties
        let mut hasher = DefaultHasher::new();

        // Log texture handle IDs for debugging
        let base_id = self.base_texture.as_ref().map(|h| h.id());
        let lightmap_id = self.lightmap_texture.as_ref().map(|h| h.id());
        let specular_id = self.specular_texture.as_ref().map(|h| h.id());

        info!("[CACHE KEY DEBUG] Texture IDs - base: {:?}, lightmap: {:?}, specular: {:?}",
            base_id, lightmap_id, specular_id);

        if let Some(ref handle) = self.base_texture {
            handle.id().hash(&mut hasher);
        }
        if let Some(ref handle) = self.lightmap_texture {
            handle.id().hash(&mut hasher);
        }
        if let Some(ref handle) = self.specular_texture {
            handle.id().hash(&mut hasher);
        }

        // Log material properties for debugging
        info!("[CACHE KEY DEBUG] Material properties - alpha_enabled: {}, alpha_value: {:?}, alpha_test: {:?}, two_sided: {}, z_test: {}, z_write: {}, skinned: {}, blend: {:?}",
            self.alpha_enabled, self.alpha_value, self.alpha_test, self.two_sided, self.z_test_enabled, self.z_write_enabled, self.skinned, self.blend);

        info!("[CACHE KEY DEBUG] UV properties - offset: ({}, {}), scale: {}",
            self.lightmap_uv_offset.x, self.lightmap_uv_offset.y, self.lightmap_uv_scale);

        // Hash other material properties
        if let Some(alpha_value) = self.alpha_value {
            alpha_value.to_bits().hash(&mut hasher);
        }
        self.alpha_enabled.hash(&mut hasher);
        if let Some(alpha_test) = self.alpha_test {
            alpha_test.to_bits().hash(&mut hasher);
        }
        self.two_sided.hash(&mut hasher);
        self.z_test_enabled.hash(&mut hasher);
        self.z_write_enabled.hash(&mut hasher);
        self.skinned.hash(&mut hasher);
        self.blend.hash(&mut hasher);
        if let Some(ref glow) = self.glow {
            match glow {
                ObjectMaterialGlow::Simple(v) => {
                    v.to_array().iter().for_each(|x| x.to_bits().hash(&mut hasher));
                    0u8.hash(&mut hasher);
                }
                ObjectMaterialGlow::Light(v) => {
                    v.to_array().iter().for_each(|x| x.to_bits().hash(&mut hasher));
                    1u8.hash(&mut hasher);
                }
                ObjectMaterialGlow::Texture(v) => {
                    v.to_array().iter().for_each(|x| x.to_bits().hash(&mut hasher));
                    2u8.hash(&mut hasher);
                }
                ObjectMaterialGlow::TextureLight(v) => {
                    v.to_array().iter().for_each(|x| x.to_bits().hash(&mut hasher));
                    3u8.hash(&mut hasher);
                }
                ObjectMaterialGlow::Alpha(v) => {
                    v.to_array().iter().for_each(|x| x.to_bits().hash(&mut hasher));
                    4u8.hash(&mut hasher);
                }
            }
        }
        self.lightmap_uv_offset.to_array().iter().for_each(|x| x.to_bits().hash(&mut hasher));
        self.lightmap_uv_scale.to_bits().hash(&mut hasher);

        let cache_key = hasher.finish();

        // Log the generated cache key
        info!("[CACHE KEY DEBUG] Generated cache key: 0x{:016x}", cache_key);

        // Check cache
        if let Some(_cached_bind_group) = OBJECT_BIND_GROUP_CACHE.get(&cache_key) {
            OBJECT_CACHE_STATS.record_hit();
            let (hits, misses, total_entries) = OBJECT_CACHE_STATS.get_stats();
            let hit_rate = if hits + misses > 0 {
                (hits as f64 / (hits + misses) as f64) * 100.0
            } else {
                0.0
            };
            info!("[OBJECT BIND GROUP CACHE] Hit! Total: {} unique, Hits: {}, Misses: {}, Hit rate: {:.1}%",
                total_entries, hits, misses, hit_rate);
            return Ok(UnpreparedBindGroup {
                bindings: vec![],
                data: ObjectMaterialKey::from(self),
            });
        }

        // Cache miss - create new bind group
        OBJECT_CACHE_STATS.record_miss();

        let texture_count = [
            self.base_texture.is_some(),
            self.lightmap_texture.is_some(),
            self.specular_texture.is_some(),
        ].iter().filter(|&&x| x).count();

        info!("[MATERIAL LIFECYCLE] Creating ObjectMaterial bind group with {} textures (cache miss)", texture_count);

        // Create uniform buffer
        let uniform_data = ObjectMaterialUniformData::from(self);
        let mut buffer = bevy::render::render_resource::encase::UniformBuffer::new(Vec::new());
        buffer.write(&uniform_data).unwrap();
        let uniform_buffer = render_device.create_buffer_with_data(&bevy::render::render_resource::BufferInitDescriptor {
            label: Some("object_material_uniform_buffer"),
            usage: bevy::render::render_resource::BufferUsages::UNIFORM | bevy::render::render_resource::BufferUsages::COPY_DST,
            contents: buffer.as_ref(),
        });

        // Create samplers
        let sampler = render_device.create_sampler(&bevy::render::render_resource::SamplerDescriptor {
            address_mode_u: bevy::render::render_resource::AddressMode::Repeat,
            address_mode_v: bevy::render::render_resource::AddressMode::Repeat,
            mag_filter: bevy::render::render_resource::FilterMode::Linear,
            min_filter: bevy::render::render_resource::FilterMode::Linear,
            ..Default::default()
        });

        // Create bind group entries
        let mut entries = vec![
            bevy::render::render_resource::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }
        ];

        // Add base texture and sampler
        if let Some(ref base_texture) = self.base_texture {
            let image = images.get(base_texture).ok_or(AsBindGroupError::RetryNextUpdate)?;
            entries.push(bevy::render::render_resource::BindGroupEntry {
                binding: 1,
                resource: bevy::render::render_resource::BindingResource::TextureView(&image.texture_view),
            });
            entries.push(bevy::render::render_resource::BindGroupEntry {
                binding: 2,
                resource: bevy::render::render_resource::BindingResource::Sampler(&sampler),
            });
        } else {
            entries.push(bevy::render::render_resource::BindGroupEntry {
                binding: 1,
                resource: bevy::render::render_resource::BindingResource::TextureView(&fallback_image.d2.texture_view),
            });
            entries.push(bevy::render::render_resource::BindGroupEntry {
                binding: 2,
                resource: bevy::render::render_resource::BindingResource::Sampler(&sampler),
            });
        }

        // Add lightmap texture and sampler
        if let Some(ref lightmap_texture) = self.lightmap_texture {
            let image = images.get(lightmap_texture).ok_or(AsBindGroupError::RetryNextUpdate)?;
            entries.push(bevy::render::render_resource::BindGroupEntry {
                binding: 3,
                resource: bevy::render::render_resource::BindingResource::TextureView(&image.texture_view),
            });
            entries.push(bevy::render::render_resource::BindGroupEntry {
                binding: 4,
                resource: bevy::render::render_resource::BindingResource::Sampler(&sampler),
            });
        }

        // Add specular texture and sampler
        if let Some(ref specular_texture) = self.specular_texture {
            let image = images.get(specular_texture).ok_or(AsBindGroupError::RetryNextUpdate)?;
            entries.push(bevy::render::render_resource::BindGroupEntry {
                binding: 5,
                resource: bevy::render::render_resource::BindingResource::TextureView(&image.texture_view),
            });
            entries.push(bevy::render::render_resource::BindGroupEntry {
                binding: 6,
                resource: bevy::render::render_resource::BindingResource::Sampler(&sampler),
            });
        }

        let bind_group = render_device.create_bind_group(
            "object_material_bind_group",
            layout,
            &entries,
        );

        // Store in cache
        OBJECT_BIND_GROUP_CACHE.insert(cache_key, bind_group);

        let (hits, misses, total_entries) = OBJECT_CACHE_STATS.get_stats();
        let hit_rate = if hits + misses > 0 {
            (hits as f64 / (hits + misses) as f64) * 100.0
        } else {
            0.0
        };
        info!("[OBJECT BIND GROUP CACHE] Created new entry. Total: {} unique, Hits: {}, Misses: {}, Hit rate: {:.1}%",
            total_entries, hits, misses, hit_rate);

        Ok(UnpreparedBindGroup {
            bindings: vec![],
            data: ObjectMaterialKey::from(self),
        })
    }

    fn bind_group_layout_entries(_render_device: &RenderDevice) -> Vec<bevy::render::render_resource::BindGroupLayoutEntry> {
        vec![
            bevy::render::render_resource::BindGroupLayoutEntry {
                binding: 0,
                visibility: bevy::render::render_resource::ShaderStages::FRAGMENT | bevy::render::render_resource::ShaderStages::VERTEX,
                ty: bevy::render::render_resource::BindingType::Buffer {
                    ty: bevy::render::render_resource::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            bevy::render::render_resource::BindGroupLayoutEntry {
                binding: 1,
                visibility: bevy::render::render_resource::ShaderStages::FRAGMENT,
                ty: bevy::render::render_resource::BindingType::Texture {
                    sample_type: bevy::render::render_resource::TextureSampleType::Float { filterable: true },
                    view_dimension: bevy::render::render_resource::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            bevy::render::render_resource::BindGroupLayoutEntry {
                binding: 2,
                visibility: bevy::render::render_resource::ShaderStages::FRAGMENT,
                ty: bevy::render::render_resource::BindingType::Sampler(bevy::render::render_resource::SamplerBindingType::Filtering),
                count: None,
            },
            bevy::render::render_resource::BindGroupLayoutEntry {
                binding: 3,
                visibility: bevy::render::render_resource::ShaderStages::FRAGMENT,
                ty: bevy::render::render_resource::BindingType::Texture {
                    sample_type: bevy::render::render_resource::TextureSampleType::Float { filterable: true },
                    view_dimension: bevy::render::render_resource::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            bevy::render::render_resource::BindGroupLayoutEntry {
                binding: 4,
                visibility: bevy::render::render_resource::ShaderStages::FRAGMENT,
                ty: bevy::render::render_resource::BindingType::Sampler(bevy::render::render_resource::SamplerBindingType::Filtering),
                count: None,
            },
            bevy::render::render_resource::BindGroupLayoutEntry {
                binding: 5,
                visibility: bevy::render::render_resource::ShaderStages::FRAGMENT,
                ty: bevy::render::render_resource::BindingType::Texture {
                    sample_type: bevy::render::render_resource::TextureSampleType::Float { filterable: true },
                    view_dimension: bevy::render::render_resource::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            bevy::render::render_resource::BindGroupLayoutEntry {
                binding: 6,
                visibility: bevy::render::render_resource::ShaderStages::FRAGMENT,
                ty: bevy::render::render_resource::BindingType::Sampler(bevy::render::render_resource::SamplerBindingType::Filtering),
                count: None,
            },
        ]
    }
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
