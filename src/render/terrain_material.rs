use std::num::NonZeroU32;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use bevy::{
    asset::{load_internal_asset, Handle, UntypedHandle, Asset, UntypedAssetId},
    pbr::{
        DrawMesh, MeshPipelineKey, SetMaterialBindGroup, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::{
        AlphaMode, App, FromWorld, Material, MaterialPlugin, Mesh, Plugin, World,
    },
    reflect::TypePath,
    render::{
        mesh::MeshVertexAttribute,
        prelude::Shader,
        render_asset::RenderAssets,
        render_phase::SetItemPipeline,
        render_resource::{
            AddressMode, AsBindGroup, AsBindGroupError, BindGroup, BindGroupEntry,
            BindGroupLayout, BindGroupLayoutEntry, BindingResource,
            BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState, FilterMode,
            UnpreparedBindGroup, RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor,
            ShaderRef, ShaderStages, SpecializedMeshPipelineError, TextureSampleType,
            TextureViewDimension, VertexFormat,
        },
        renderer::RenderDevice,
        texture::{FallbackImage, Image},
    },
    utils::Uuid,
};
use dashmap::DashMap;
use lazy_static::lazy_static;
use log::info;

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

// Bind group cache for TerrainMaterial
lazy_static! {
    static ref TERRAIN_BIND_GROUP_CACHE: DashMap<u64, BindGroup> = DashMap::new();
    static ref TERRAIN_CACHE_STATS: CacheStats = CacheStats::new();
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
        let total_entries = TERRAIN_BIND_GROUP_CACHE.len();
        (hits, misses, total_entries)
    }
}

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

        app.add_plugins(MaterialPlugin::<TerrainMaterial> {
            prepass_enabled: self.prepass_enabled,
            ..Default::default()
        });
    }
}

#[derive(Clone)]
pub struct TerrainMaterialPipelineData {
    pub zone_lighting_layout: BindGroupLayout,
}

impl FromWorld for TerrainMaterialPipelineData {
    fn from_world(world: &mut World) -> Self {
        TerrainMaterialPipelineData {
            zone_lighting_layout: world
                .resource::<ZoneLightingUniformMeta>()
                .bind_group_layout
                .clone(),
        }
    }
}

#[derive(Asset, Debug, Clone, TypePath)]
pub struct TerrainMaterial {
    pub textures: Vec<Handle<Image>>,
}

impl Material for TerrainMaterial {
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }

    fn vertex_shader() -> ShaderRef {
        TERRAIN_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn fragment_shader() -> ShaderRef {
        TERRAIN_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &bevy::render::mesh::MeshVertexBufferLayout,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
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

        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            MESH_ATTRIBUTE_UV_1.at_shader_location(3),
            TERRAIN_MESH_ATTRIBUTE_TILE_INFO.at_shader_location(4),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        Ok(())
    }
}

impl AsBindGroup for TerrainMaterial {
    type Data = ();

    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &bevy::render::renderer::RenderDevice,
        images: &bevy::render::render_asset::RenderAssets<bevy::prelude::Image>,
        fallback_image: &bevy::render::texture::FallbackImage,
    ) -> Result<UnpreparedBindGroup<Self::Data>, AsBindGroupError> {
        // Generate cache key from texture handles
        let mut hasher = DefaultHasher::new();
        for handle in self.textures.iter().take(TERRAIN_MATERIAL_MAX_TEXTURES) {
            handle.id().hash(&mut hasher);
        }
        let cache_key = hasher.finish();

        // Check cache
        if let Some(cached_bind_group) = TERRAIN_BIND_GROUP_CACHE.get(&cache_key) {
            TERRAIN_CACHE_STATS.record_hit();
            let (hits, misses, total_entries) = TERRAIN_CACHE_STATS.get_stats();
            let hit_rate = if hits + misses > 0 {
                (hits as f64 / (hits + misses) as f64) * 100.0
            } else {
                0.0
            };
            info!("[TERRAIN BIND GROUP CACHE] Hit! Total: {} unique, Hits: {}, Misses: {}, Hit rate: {:.1}%",
                total_entries, hits, misses, hit_rate);
            return Ok(UnpreparedBindGroup {
                bindings: vec![],
                data: (),
            });
        }

        // Cache miss - create new bind group
        TERRAIN_CACHE_STATS.record_miss();
        info!("[MATERIAL LIFECYCLE] Creating TerrainMaterial bind group with {} textures (cache miss)", self.textures.len());

        let mut images_vec = vec![];
        for handle in self.textures.iter().take(TERRAIN_MATERIAL_MAX_TEXTURES) {
            match images.get(handle) {
                Some(image) => images_vec.push(image),
                None => return Err(AsBindGroupError::RetryNextUpdate),
            }
        }

        let mut textures = vec![&*fallback_image.d2.texture_view; TERRAIN_MATERIAL_MAX_TEXTURES];
        for (id, image) in images_vec.into_iter().enumerate() {
            textures[id] = &*image.texture_view;
        }

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let detail_sampler = render_device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = render_device.create_bind_group(
            "terrain_material_bind_group",
            layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureViewArray(&textures[..]),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&fallback_image.d2.texture_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&detail_sampler),
                },
            ],
        );

        // Store in cache
        TERRAIN_BIND_GROUP_CACHE.insert(cache_key, bind_group);

        let (hits, misses, total_entries) = TERRAIN_CACHE_STATS.get_stats();
        let hit_rate = if hits + misses > 0 {
            (hits as f64 / (hits + misses) as f64) * 100.0
        } else {
            0.0
        };
        info!("[TERRAIN BIND GROUP CACHE] Created new entry. Total: {} unique, Hits: {}, Misses: {}, Hit rate: {:.1}%",
            total_entries, hits, misses, hit_rate);

        Ok(UnpreparedBindGroup {
            bindings: vec![],
            data: (),
        })
    }

    fn bind_group_layout_entries(_render_device: &bevy::render::renderer::RenderDevice) -> Vec<BindGroupLayoutEntry> {
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

type DrawTerrainMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<TerrainMaterial, 1>,
    SetMeshBindGroup<2>,
    SetZoneLightingBindGroup<3>,
    DrawMesh,
);
