use std::num::NonZeroU32;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use bevy::{
    asset::{load_internal_asset, Handle, UntypedHandle, UntypedAssetId},
    ecs::{
        system::{lifetimeless::SRes, SystemParamItem},
    },
    pbr::{
        DrawMesh, MeshPipelineKey, SetMaterialBindGroup, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::{
        AlphaMode, App, Commands, FromWorld, Image, Material, MaterialPlugin, Mesh,
        Plugin, Res, Resource, Time, World,
    },
    asset::Asset,
    reflect::TypePath,
    render::{
        prelude::Shader,
        render_asset::RenderAssets,
        render_phase::{
            PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            encase, AddressMode, AsBindGroup, AsBindGroupError, BindGroup, BindGroupEntry,
            BindGroupLayout, BindGroupLayoutEntry,
            BindingResource, BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState,
            FilterMode, UnpreparedBindGroup, PushConstantRange, RenderPipelineDescriptor,
            SamplerBindingType, SamplerDescriptor, ShaderDefVal, ShaderRef, ShaderSize, ShaderStages,
            ShaderType, SpecializedMeshPipelineError, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
        texture::FallbackImage,
        Extract, ExtractSchedule, RenderApp,
    },
    utils::Uuid,
};
use dashmap::DashMap;
use lazy_static::lazy_static;
use log::info;

use crate::render::zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta};
use std::any::TypeId;

pub const WATER_MESH_MATERIAL_SHADER_HANDLE: UntypedHandle =
    UntypedHandle::Weak(UntypedAssetId::Uuid { type_id: TypeId::of::<Shader>(), uuid: Uuid::from_u128(0x333959e64b35d5d9) });

pub const WATER_MESH_MATERIAL_SHADER_HANDLE_TYPED: Handle<Shader> =
    Handle::weak_from_u128(0x333959e64b35d5d9);

pub const WATER_MATERIAL_NUM_TEXTURES: usize = 25;

// Bind group cache for WaterMaterial
lazy_static! {
    static ref WATER_BIND_GROUP_CACHE: DashMap<u64, BindGroup> = DashMap::new();
    static ref WATER_CACHE_STATS: CacheStats = CacheStats::new();
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
        let total_entries = WATER_BIND_GROUP_CACHE.len();
        (hits, misses, total_entries)
    }
}

#[derive(Default)]
pub struct WaterMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for WaterMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            WATER_MESH_MATERIAL_SHADER_HANDLE_TYPED,
            "shaders/water_material.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(MaterialPlugin::<WaterMaterial> {
            prepass_enabled: self.prepass_enabled,
            ..Default::default()
        });

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(ExtractSchedule, extract_water_push_constant_data);
        }
    }
}

#[derive(Clone, ShaderType, Resource)]
pub struct WaterPushConstantData {
    pub current_index: i32,
    pub next_index: i32,
    pub next_weight: f32,
}

fn extract_water_push_constant_data(mut commands: Commands, time: Extract<Res<Time>>) {
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

#[derive(Clone)]
pub struct WaterMaterialPipelineData {
    pub zone_lighting_layout: BindGroupLayout,
}

impl FromWorld for WaterMaterialPipelineData {
    fn from_world(world: &mut World) -> Self {
        WaterMaterialPipelineData {
            zone_lighting_layout: world
                .resource::<ZoneLightingUniformMeta>()
                .bind_group_layout
                .clone(),
        }
    }
}

#[derive(Asset, Debug, Clone, TypePath)]
pub struct WaterMaterial {
    pub textures: Vec<Handle<Image>>,
}

impl Material for WaterMaterial {
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn vertex_shader() -> ShaderRef {
        WATER_MESH_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn fragment_shader() -> ShaderRef {
        WATER_MESH_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &bevy::render::mesh::MeshVertexBufferLayout,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = false;

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

        // Note: Zone lighting layout removed as PipelineData is no longer available
        // This will need to be handled differently if zone lighting is required

        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        descriptor.push_constant_ranges.push(PushConstantRange {
            stages: ShaderStages::FRAGMENT,
            range: 0..WaterPushConstantData::SHADER_SIZE.get() as u32,
        });

        Ok(())
    }
}

impl AsBindGroup for WaterMaterial {
    type Data = ();

    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        image_assets: &RenderAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> Result<UnpreparedBindGroup<Self::Data>, AsBindGroupError> {
        // Generate cache key from texture handles
        let mut hasher = DefaultHasher::new();
        for handle in self.textures.iter().take(WATER_MATERIAL_NUM_TEXTURES) {
            handle.id().hash(&mut hasher);
        }
        let cache_key = hasher.finish();

        // Check cache
        if let Some(_cached_bind_group) = WATER_BIND_GROUP_CACHE.get(&cache_key) {
            WATER_CACHE_STATS.record_hit();
            let (hits, misses, total_entries) = WATER_CACHE_STATS.get_stats();
            let hit_rate = if hits + misses > 0 {
                (hits as f64 / (hits + misses) as f64) * 100.0
            } else {
                0.0
            };
            info!("[WATER BIND GROUP CACHE] Hit! Total: {} unique, Hits: {}, Misses: {}, Hit rate: {:.1}%",
                total_entries, hits, misses, hit_rate);
            return Ok(UnpreparedBindGroup {
                bindings: vec![],
                data: (),
            });
        }

        // Cache miss - create new bind group
        WATER_CACHE_STATS.record_miss();
        info!("[MATERIAL LIFECYCLE] Creating WaterMaterial bind group with {} textures (cache miss)", self.textures.len());

        let mut images = vec![];
        for handle in self.textures.iter().take(WATER_MATERIAL_NUM_TEXTURES) {
            match image_assets.get(handle) {
                Some(image) => images.push(image),
                None => return Err(AsBindGroupError::RetryNextUpdate),
            }
        }

        let mut textures = vec![&*fallback_image.d2.texture_view; WATER_MATERIAL_NUM_TEXTURES];
        for (id, image) in images.into_iter().enumerate() {
            textures[id] = &*image.texture_view;
        }

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

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
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        );

        // Store in cache
        WATER_BIND_GROUP_CACHE.insert(cache_key, bind_group);

        let (hits, misses, total_entries) = WATER_CACHE_STATS.get_stats();
        let hit_rate = if hits + misses > 0 {
            (hits as f64 / (hits + misses) as f64) * 100.0
        } else {
            0.0
        };
        info!("[WATER BIND GROUP CACHE] Created new entry. Total: {} unique, Hits: {}, Misses: {}, Hit rate: {:.1}%",
            total_entries, hits, misses, hit_rate);

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
pub struct SetWaterMaterialPushConstants<const OFFSET: u32>;
impl<P: PhaseItem, const OFFSET: u32> RenderCommand<P> for SetWaterMaterialPushConstants<OFFSET> {
    type Param = SRes<WaterPushConstantData>;
    type ViewQuery = ();
    type ItemQuery = ();

    fn render<'w>(
        _: &P,
        _: bevy::ecs::query::ROQueryItem<'w, Self::ViewQuery>,
        _: Option<()>,
        water_uniform_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let byte_buffer = [0u8; WaterPushConstantData::SHADER_SIZE.get() as usize];
        let mut buffer = encase::StorageBuffer::new(byte_buffer);
        buffer.write(water_uniform_data.as_ref()).unwrap();
        pass.set_push_constants(ShaderStages::FRAGMENT, 0, buffer.as_ref());
        RenderCommandResult::Success
    }
}

type DrawWaterMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<WaterMaterial, 1>,
    SetMeshBindGroup<2>,
    SetZoneLightingBindGroup<3>,
    SetWaterMaterialPushConstants<0>,
    DrawMesh,
);
