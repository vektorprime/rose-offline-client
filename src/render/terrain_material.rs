//! Custom terrain material with texture array support for ROSE Online terrain
//!
//! This module implements a custom material that supports:
//! - Up to 100 tile textures in a binding_array
//! - Per-vertex tile_info for texture selection and rotation
//! - Two-layer blending with alpha
//! - Lightmap support via UV0

use std::num::NonZeroU32;

use bevy::{
    asset::{load_internal_asset, weak_handle, Asset, AssetApp, Assets, Handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::{Material, MaterialPipeline, MaterialPipelineKey, MeshPipelineKey},
    prelude::{
        App, Color, ColorToComponents, Component, DetectChanges, GlobalTransform, LinearRgba, Mesh,
        Plugin, Query, Res, ResMut, Resource, Vec3, Vec4, With, World,
    },
    reflect::TypePath,
    render::{
        alpha::AlphaMode,
        render_asset::RenderAssets,
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
        renderer::RenderDevice,
        texture::{FallbackImage, GpuImage},
    },
    shader::{Shader, ShaderRef},
};
use bevy_mesh::MeshVertexBufferLayoutRef;

use crate::graphics::GraphicsSettings;
use crate::render::{ZoneLighting, MESH_ATTRIBUTE_UV_1, TERRAIN_MESH_ATTRIBUTE_TILE_INFO};

/// Shader handle for the terrain material shader
pub const TERRAIN_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("3d793925-0aff-89cb-0000-000000000000");

/// Maximum number of terrain tile textures supported
pub const TERRAIN_MATERIAL_MAX_TEXTURES: usize = 100;

/// Per-entity terrain lighting values extracted from ZoneLighting.
///
/// Kept on the mesh entity so we don't need uniform buffers in the material bind group,
/// which is disallowed by wgpu when using binding arrays.
#[derive(Component, Debug, Clone, Copy)]
pub struct TerrainLighting {
    pub light_direction: Vec4,
    pub light_color: Vec4,
    pub ambient_color: Vec4,
}

impl Default for TerrainLighting {
    fn default() -> Self {
        Self {
            light_direction: Vec4::new(0.0, -1.0, 0.0, 0.0),
            light_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            ambient_color: Vec4::new(0.2, 0.2, 0.2, 1.0),
        }
    }
}

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

/// System that updates terrain material lighting based on ZoneLighting and time of day.
///
/// The terrain lighting intensity is adjusted based on the time of day:
/// | Time State | Intensity Multiplier | Time Period  |
/// |------------|---------------------|--------------|
/// | Morning    | 2.0                 | 6:00-12:00   |
/// | Day        | 2.5                 | 12:00-17:00  |
/// | Evening    | 2.0                 | 17:00-19:00  |
/// | Night      | 1.0                 | 19:00-6:00   |
pub fn update_terrain_lighting_system(
    zone_lighting: Res<ZoneLighting>,
    graphics_settings: Res<GraphicsSettings>,
    zone_time: Res<crate::resources::ZoneTime>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
) {
    // Only update if zone_lighting, graphics_settings, or zone_time has changed
    if !zone_lighting.is_changed() && !graphics_settings.is_changed() && !zone_time.is_changed() {
        return;
    }

    // Get the terrain light intensity scale from graphics settings
    let base_intensity = graphics_settings.terrain_light_intensity;

    // Apply time-of-day multiplier to terrain lighting intensity
    // This creates more realistic lighting transitions throughout the day
    let time_multiplier = match zone_time.state {
        crate::resources::ZoneTimeState::Morning => 2.0, // 6:00-12:00: Moderate morning light
        crate::resources::ZoneTimeState::Day => 2.5,     // 12:00-17:00: Bright daylight
        crate::resources::ZoneTimeState::Evening => 2.0, // 17:00-19:00: Dimming evening light
        crate::resources::ZoneTimeState::Night => 1.0,   // 19:00-6:00: Dim night light
    };

    // Combine base intensity with time multiplier
    // Scale down by dividing by 5.0 to keep values in reasonable range (base is 5.0)
    let intensity_scale = (base_intensity * time_multiplier) / 5.0;

    for (_, material) in terrain_materials.iter_mut() {
        material.light_direction = zone_lighting.light_direction;
        let char_diffuse = zone_lighting.character_diffuse_color;
        // Scale the light color to match the perceptual brightness of DirectionalLight's HDR illuminance
        material.light_color = Color::from(LinearRgba::new(
            char_diffuse.x * intensity_scale,
            char_diffuse.y * intensity_scale,
            char_diffuse.z * intensity_scale,
            1.0,
        ));
        let map_ambient = zone_lighting.map_ambient_color;
        material.ambient_color = Color::from(LinearRgba::new(
            map_ambient.x,
            map_ambient.y,
            map_ambient.z,
            1.0,
        ));
    }
}

/// Sync terrain lighting data to terrain mesh entities for shader consumption.
pub fn sync_terrain_lighting_component_system(
    zone_lighting: Res<ZoneLighting>,
    graphics_settings: Res<GraphicsSettings>,
    zone_time: Res<crate::resources::ZoneTime>,
    mut terrain_entities: Query<
        &mut TerrainLighting,
        With<bevy::pbr::MeshMaterial3d<TerrainMaterial>>,
    >,
) {
    if !zone_lighting.is_changed() && !graphics_settings.is_changed() && !zone_time.is_changed() {
        return;
    }

    let base_intensity = graphics_settings.terrain_light_intensity;
    let time_multiplier = match zone_time.state {
        crate::resources::ZoneTimeState::Morning => 2.0,
        crate::resources::ZoneTimeState::Day => 2.5,
        crate::resources::ZoneTimeState::Evening => 2.0,
        crate::resources::ZoneTimeState::Night => 1.0,
    };
    let intensity_scale = (base_intensity * time_multiplier) / 5.0;

    let char_diffuse = zone_lighting.character_diffuse_color;
    let map_ambient = zone_lighting.map_ambient_color;
    let values = TerrainLighting {
        light_direction: zone_lighting.light_direction.extend(0.0),
        light_color: Vec4::new(
            char_diffuse.x * intensity_scale,
            char_diffuse.y * intensity_scale,
            char_diffuse.z * intensity_scale,
            1.0,
        ),
        ambient_color: Vec4::new(map_ambient.x, map_ambient.y, map_ambient.z, 1.0),
    };

    for mut lighting in terrain_entities.iter_mut() {
        *lighting = values;
    }
}

/// Custom terrain material supporting multiple tile textures via texture array
#[derive(Asset, Debug, Clone, TypePath)]
pub struct TerrainMaterial {
    /// Array of tile texture handles (up to TERRAIN_MATERIAL_MAX_TEXTURES)
    pub textures: Vec<Handle<bevy::image::Image>>,

    /// Terrain directional light direction.
    ///
    /// Uploaded to a read-only storage buffer in the material bind group to remain
    /// compatible with wgpu 27's binding-array restrictions.
    pub light_direction: Vec3,
    /// Terrain directional light color.
    pub light_color: Color,
    /// Terrain ambient light color.
    pub ambient_color: Color,
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

        // Configure blending for terrain
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
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        // Lighting payload for shader consumption.
        // NOTE: wgpu 27 disallows mixing binding arrays and uniform buffers in one bind group,
        // so terrain lighting is provided via a read-only storage buffer instead of uniforms.
        let light_color = self.light_color.to_linear().to_f32_array();
        let ambient_color = self.ambient_color.to_linear().to_f32_array();
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
