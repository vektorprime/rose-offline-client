//! Custom terrain material with texture array support for ROSE Online terrain
//!
//! This module implements a custom material that supports:
//! - Up to 100 tile textures in a binding_array
//! - Per-vertex tile_info for texture selection and rotation
//! - Two-layer blending with alpha
//! - Lightmap support via UV0

use std::num::NonZeroU32;

use bevy::{
    asset::{load_internal_asset, weak_handle, Asset, AssetApp, AssetId, Assets, Handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::{Material, MaterialPipeline, MaterialPipelineKey},
    prelude::{
        App, Color, ColorToComponents, LinearRgba, Local, Mesh, Plugin, Res, ResMut, Vec3, Vec4,
    },
    material::AlphaMode,
    reflect::TypePath,
    render::{
        render_asset::RenderAssets,
        render_resource::*,
        renderer::RenderDevice,
        texture::{FallbackImage, GpuImage},
    },
    shader::{Shader, ShaderRef},
};
use bevy_mesh::MeshVertexBufferLayoutRef;

use crate::graphics::GraphicsSettings;
use crate::render::zone_lighting::{
    moon_light_factor, sun_light_factor, DaylightSettings, MOON_COLOR, MOON_MAX_ILLUMINANCE,
    SUN_MAX_ILLUMINANCE,
};
use crate::render::{
    StarrySkySettings, ZoneLighting, MESH_ATTRIBUTE_UV_1, TERRAIN_MESH_ATTRIBUTE_TILE_INFO,
};

/// Shader handle for the terrain material shader
pub const TERRAIN_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("3d793925-0aff-89cb-0000-000000000000");

/// Maximum number of terrain tile textures supported
pub const TERRAIN_MATERIAL_MAX_TEXTURES: usize = 100;

/// Plugin that registers the terrain material
pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TERRAIN_MATERIAL_SHADER_HANDLE,
            "shaders/terrain_material.wgsl",
            Shader::from_wgsl
        );

        // Register the material asset
        app.init_asset::<TerrainMaterial>();

        // Add the material plugin for rendering
        // Note: prepass and shadows are controlled via enable_prepass() and enable_shadows() methods on Material trait
        app.add_plugins(bevy::pbr::MaterialPlugin::<TerrainMaterial>::default());

        log::info!("[TERRAIN MATERIAL] TerrainMaterialPlugin loaded");
    }
}

/// Terrain light units per `terrain_light_intensity` unit at the full sun
/// (default intensity 5 x 2.5 / 5 = 2.5, the old "Day" value).
const TERRAIN_SUN_SCALE: f32 = 2.5 / 5.0;

/// System that updates terrain material lighting from ZoneLighting and the
/// live sun/moon.
///
/// The legacy terrain shader has its own unit-scale lighting (not lux), so it
/// mirrors the PBR lights instead of a time-of-day table:
/// - sun: `zone_lighting.light_direction` (toward the sun), strength follows
///   `sun_light_factor` (the same ramp as the sun's illuminance) and the Sky
///   tab's sun brightness;
/// - moon: a second directional light toward `moon_direction`, fading in with
///   `moon_light_factor` at MOON/SUN of the sun's strength.
///
/// Both are continuous in the sun's elevation, so there are no jumps at
/// Morning/Day/Evening/Night changes (was a 2.0/2.5/2.0/1.0 step table).
///
/// Runs every frame without a change gate: the targets are a handful of float
/// ops, and a gate would leave a newly spawned zone's material (created with
/// placeholder lighting) stale until the next lighting change.
pub fn update_terrain_lighting_system(
    zone_lighting: Res<ZoneLighting>,
    graphics_settings: Res<GraphicsSettings>,
    daylight: Res<DaylightSettings>,
    starry_sky_settings: Option<Res<StarrySkySettings>>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut stale_materials: Local<Vec<AssetId<TerrainMaterial>>>,
) {
    // Compute targets first; write only on actual difference.
    let base_scale = graphics_settings.terrain_light_intensity * TERRAIN_SUN_SCALE;
    let light_direction = zone_lighting.light_direction;
    let sun_height = light_direction.y;
    let sun_strength = base_scale
        * (daylight.sun_illuminance.max(0.0) / SUN_MAX_ILLUMINANCE)
        * sun_light_factor(sun_height);
    // Sun color matches the PBR sun (sync_zone_lighting_to_bevy_lights_system
    // sets it to character_diffuse_color).
    let char_diffuse = zone_lighting.character_diffuse_color;
    let light_color = Color::from(LinearRgba::new(
        char_diffuse.x * sun_strength,
        char_diffuse.y * sun_strength,
        char_diffuse.z * sun_strength,
        1.0,
    ));

    let moon_strength = base_scale
        * (MOON_MAX_ILLUMINANCE / SUN_MAX_ILLUMINANCE)
        * moon_light_factor(sun_height);
    let moon_tint = MOON_COLOR.to_linear();
    let moon_color = Color::from(LinearRgba::new(
        moon_tint.red * moon_strength,
        moon_tint.green * moon_strength,
        moon_tint.blue * moon_strength,
        1.0,
    ));
    // StarrySkySettings::moon_direction points TOWARD the moon (the moon light
    // sits at camera + moon_direction and looks back at the camera).
    let moon_direction = starry_sky_settings
        .as_ref()
        .map(|settings| settings.moon_direction)
        .unwrap_or(DEFAULT_MOON_DIRECTION)
        .normalize_or(Vec3::Y);

    let map_ambient = zone_lighting.map_ambient_color;
    let ambient_color = Color::from(LinearRgba::new(
        map_ambient.x,
        map_ambient.y,
        map_ambient.z,
        1.0,
    ));

    // Find stale materials read-only, then `get_mut` only those. `Assets::iter_mut`
    // queues AssetEvent::Modified for EVERY asset it visits, written or not, and
    // each Modified TerrainMaterial is re-prepared by the render world. On Bevy
    // 0.19.1's CreateBindGroupDirectly path that re-prepare also never frees the
    // previous bind group (bevy_pbr material.rs prepare_asset), so it leaks.
    stale_materials.clear();
    stale_materials.extend(
        terrain_materials
            .iter()
            .filter(|(_, material)| {
                material.light_direction != light_direction
                    || material.light_color != light_color
                    || material.ambient_color != ambient_color
                    || material.moon_direction != moon_direction
                    || material.moon_color != moon_color
            })
            .map(|(id, _)| id),
    );

    for id in stale_materials.drain(..) {
        if let Some(mut material) = terrain_materials.get_mut(id) {
            material.light_direction = light_direction;
            material.light_color = light_color;
            material.ambient_color = ambient_color;
            material.moon_direction = moon_direction;
            material.moon_color = moon_color;
        }
    }
}

/// Fallback moon direction (toward the moon) when StarrySkySettings is absent;
/// matches `StarrySkySettings::default().moon_direction`.
pub const DEFAULT_MOON_DIRECTION: Vec3 = Vec3::new(0.3, 0.8, 0.5);

/// Custom terrain material supporting multiple tile textures via texture array
#[derive(Asset, Debug, Clone, TypePath)]
pub struct TerrainMaterial {
    /// Array of tile texture handles (up to TERRAIN_MATERIAL_MAX_TEXTURES)
    pub textures: Vec<Handle<bevy::image::Image>>,

    /// Terrain sun direction, pointing TOWARD the sun.
    ///
    /// Uploaded to a read-only storage buffer in the material bind group to remain
    /// compatible with wgpu 27's binding-array restrictions.
    pub light_direction: Vec3,
    /// Terrain sun color (pre-scaled by strength; black when the sun is down).
    pub light_color: Color,
    /// Terrain ambient light color.
    pub ambient_color: Color,
    /// Terrain moon direction, pointing TOWARD the moon.
    pub moon_direction: Vec3,
    /// Terrain moon color (pre-scaled by strength; black by day).
    pub moon_color: Color,
}

/// Data stored alongside the prepared bind group
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TerrainMaterialKey {
    pub texture_count: u32,
}

impl From<&TerrainMaterial> for TerrainMaterialKey {
    fn from(material: &TerrainMaterial) -> Self {
        TerrainMaterialKey {
            texture_count: material.textures.len() as u32,
        }
    }
}

impl Material for TerrainMaterial {
    fn vertex_shader() -> ShaderRef {
        TERRAIN_MATERIAL_SHADER_HANDLE.into()
    }

    fn fragment_shader() -> ShaderRef {
        TERRAIN_MATERIAL_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }

    /// Disable prepass for custom material
    fn enable_prepass() -> bool {
        false
    }

    /// Terrain doesn't cast shadows
    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Set up vertex buffer layout with our custom attributes
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2), // Lightmap UVs
            MESH_ATTRIBUTE_UV_1.at_shader_location(3),  // Tile texture UVs
            TERRAIN_MESH_ATTRIBUTE_TILE_INFO.at_shader_location(4), // Tile info
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        // No blending: terrain_material.wgsl always outputs alpha 1.0, so the former
        // SrcAlpha/OneMinusSrcAlpha blend reduced to src * 1 + dst * 0 = src. Writing
        // src directly is identical and skips the destination read on the HDR target.
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = None;
            }
        }

        Ok(())
    }
}

impl AsBindGroup for TerrainMaterial {
    type Data = TerrainMaterialKey;
    type Param = (SRes<RenderAssets<GpuImage>>, SRes<FallbackImage>);

    fn label() -> &'static str {
        "terrain_material"
    }

    fn bind_group_data(&self) -> Self::Data {
        TerrainMaterialKey {
            texture_count: self.textures.len() as u32,
        }
    }

    /// Override as_bind_group to create bind group with texture array
    /// This is needed because UnpreparedBindGroup doesn't support texture arrays
    fn as_bind_group(
        &self,
        layout_descriptor: &BindGroupLayoutDescriptor,
        render_device: &RenderDevice,
        pipeline_cache: &PipelineCache,
        (image_assets, fallback_image): &mut SystemParamItem<'_, '_, Self::Param>,
    ) -> Result<PreparedBindGroup, AsBindGroupError> {
        use std::ops::Deref;

        // Get the actual bind group layout from the pipeline cache
        let layout = pipeline_cache.get_bind_group_layout(layout_descriptor);

        // Collect loaded textures
        let mut images = vec![];
        for handle in self.textures.iter().take(TERRAIN_MATERIAL_MAX_TEXTURES) {
            match image_assets.get(handle) {
                Some(image) => images.push(image),
                None => return Err(AsBindGroupError::RetryNextUpdate),
            }
        }

        // Build texture view array using raw wgpu views (accessed via Deref), with fallback for missing slots
        // The TextureView type from bevy::render::render_resource derefs to wgpu::TextureView
        let fallback_view = &*fallback_image.d2.texture_view;
        let mut textures: Vec<&_> = vec![fallback_view; TERRAIN_MATERIAL_MAX_TEXTURES];
        for (id, image) in images.into_iter().enumerate() {
            textures[id] = &*image.texture_view;
        }

        // Create sampler
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        });

        // Lighting payload for shader consumption.
        // NOTE: wgpu 27 disallows mixing binding arrays and uniform buffers in one bind group,
        // so terrain lighting is provided via a read-only storage buffer instead of uniforms.
        let light_color = self.light_color.to_linear().to_f32_array();
        let ambient_color = self.ambient_color.to_linear().to_f32_array();
        let moon_color = self.moon_color.to_linear().to_f32_array();
        // Layout must match `terrain_lighting` in terrain_material.wgsl.
        let lighting_data = [
            Vec4::new(
                self.light_direction.x,
                self.light_direction.y,
                self.light_direction.z,
                0.0,
            ),
            Vec4::new(
                light_color[0],
                light_color[1],
                light_color[2],
                light_color[3],
            ),
            Vec4::new(
                ambient_color[0],
                ambient_color[1],
                ambient_color[2],
                ambient_color[3],
            ),
            Vec4::new(
                self.moon_direction.x,
                self.moon_direction.y,
                self.moon_direction.z,
                0.0,
            ),
            Vec4::new(moon_color[0], moon_color[1], moon_color[2], moon_color[3]),
        ];
        let lighting_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("terrain_lighting_buffer"),
            contents: bytemuck::cast_slice(&lighting_data),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        // Create bind group entries
        let entries = vec![
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
                resource: lighting_buffer.as_entire_binding(),
            },
        ];

        // Create bind group
        let bind_group = render_device.create_bind_group(Self::label(), &layout, &entries);

        Ok(PreparedBindGroup {
            bindings: BindingResources(vec![]),
            bind_group,
        })
    }

    /// Required by trait even though we override as_bind_group
    fn unprepared_bind_group(
        &self,
        _layout: &BindGroupLayout,
        _render_device: &RenderDevice,
        _param: &mut SystemParamItem<'_, '_, Self::Param>,
        _bindless: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        // Signal that we want as_bind_group to be called instead
        Err(AsBindGroupError::CreateBindGroupDirectly)
    }

    fn bind_group_layout_entries(
        _render_device: &RenderDevice,
        _bindless: bool,
    ) -> Vec<BindGroupLayoutEntry> {
        vec![
            // Texture array binding
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
            // Sampler binding
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
            // Terrain lighting data in read-only storage buffer
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ]
    }
}
