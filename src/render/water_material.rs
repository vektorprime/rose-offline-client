//! Custom procedural water material (no texture dependencies)
//!
//! This module implements a custom material that supports:
//! - Fully procedural wave/color generation in WGSL
//! - Physically-plausible alpha blending for water transparency
//! - Depth write disabled for proper water rendering
//! - Zone lighting integration
//! - Configurable water settings via WaterSettings resource

use bevy::{
    asset::{load_internal_asset, weak_handle, Asset, AssetApp, Handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    image::Image,
    math::{Vec3, Vec4},
    pbr::{Material, MaterialPipeline, MaterialPipelineKey},
    prelude::{App, Plugin},
    reflect::TypePath,
    render::{
        alpha::AlphaMode,
        render_asset::RenderAssets,
        render_resource::*,
        renderer::RenderDevice,
        texture::{FallbackImage, GpuImage},
    },
};
use bevy_mesh::{Mesh, MeshVertexBufferLayoutRef};
use bevy_shader::{Shader, ShaderRef};

use crate::resources::WaterSettings;

/// Shader handle for the water material shader
pub const WATER_MATERIAL_SHADER_HANDLE: Handle<bevy_shader::Shader> =
    weak_handle!("333959e6-4b35-d5d9-0000-000000000000");

/// Plugin that registers the water material
pub struct WaterMaterialPlugin;

impl Plugin for WaterMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            WATER_MATERIAL_SHADER_HANDLE,
            "shaders/water_material.wgsl",
            Shader::from_wgsl
        );

        // Register the material asset
        app.init_asset::<WaterMaterial>();

        // Add the material plugin for rendering
        // Note: prepass and shadows are controlled via enable_prepass() and enable_shadows() methods on Material trait
        app.add_plugins(bevy::pbr::MaterialPlugin::<WaterMaterial>::default());

        log::info!("[WATER MATERIAL] WaterMaterialPlugin loaded");
    }
}

/// Custom water material for fully procedural water shading
#[derive(Asset, Debug, Clone, TypePath)]
pub struct WaterMaterial {
    /// Light direction for specular highlights (normalized, pointing towards light)
    pub light_direction: Vec3,
    /// Ambient light color
    pub ambient_color: Vec4,
    /// Diffuse light color
    pub diffuse_color: Vec4,
    /// Water rendering settings
    pub settings: WaterSettings,
    /// Fog color for distance blending (from zone lighting)
    pub fog_color: Vec4,
    /// Fog density for exponential fog (from zone lighting)
    pub fog_density: f32,
    /// Fog minimum density (from zone lighting)
    pub fog_min_density: f32,
    /// Fog maximum density (from zone lighting)
    pub fog_max_density: f32,
    /// Off-screen texture containing the mirrored scene rendered by the
    /// reflection camera. Sampled in the fragment shader for planar reflections.
    pub reflection_texture: Handle<Image>,
    /// DEBUG: reflection camera status written by the water reflection plugin
    /// (0 = camera disabled, 1 = active but no entities visible, 2 = ok)
    pub reflection_status: u32,
}

/// Default implementation for WaterMaterial
impl Default for WaterMaterial {
    fn default() -> Self {
        Self {
            // Default light direction pointing down and slightly forward
            light_direction: Vec3::new(0.3, -0.8, 0.5).normalize(),
            // Default ambient color (warm daylight)
            ambient_color: Vec4::new(0.4, 0.4, 0.45, 1.0),
            // Default diffuse color (bright sunlight)
            diffuse_color: Vec4::new(0.8, 0.75, 0.7, 1.0),
            // Default water settings
            settings: WaterSettings::default(),
            // Default fog settings (will be overridden by zone lighting)
            fog_color: Vec4::new(0.2, 0.2, 0.2, 1.0),
            fog_density: 0.0018,
            fog_min_density: 0.0,
            fog_max_density: 0.75,
            // Default handle; the water reflection plugin replaces it with the
            // actual reflection render target once it exists.
            reflection_texture: Handle::default(),
            reflection_status: 0,
        }
    }
}

/// Data stored alongside the prepared bind group
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct WaterMaterialKey;

impl From<&WaterMaterial> for WaterMaterialKey {
    fn from(material: &WaterMaterial) -> Self {
        let _ = material;
        WaterMaterialKey
    }
}

impl Material for WaterMaterial {
    fn vertex_shader() -> ShaderRef {
        WATER_MATERIAL_SHADER_HANDLE.into()
    }

    fn fragment_shader() -> ShaderRef {
        WATER_MATERIAL_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    /// Disable prepass for transparent water
    fn enable_prepass() -> bool {
        false
    }

    /// Water doesn't cast shadows
    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Disable depth write for transparent water
        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = false;

        // Set up vertex buffer layout
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        // Configure additive blending for water
        // Use alpha blending for more realistic water compositing.
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = Some(BlendState::ALPHA_BLENDING);
            }
        }

        // Render water from both above and below so the surface remains visible underwater.
        descriptor.primitive.cull_mode = None;

        Ok(())
    }
}

impl AsBindGroup for WaterMaterial {
    type Data = WaterMaterialKey;
    type Param = (SRes<RenderAssets<GpuImage>>, SRes<FallbackImage>);

    fn label() -> &'static str {
        "water_material"
    }

    fn bind_group_data(&self) -> Self::Data {
        WaterMaterialKey
    }

    /// Override as_bind_group to create bind group with packed per-material data.
    fn as_bind_group(
        &self,
        layout_descriptor: &BindGroupLayoutDescriptor,
        render_device: &RenderDevice,
        pipeline_cache: &PipelineCache,
        (image_assets, fallback_image): &mut SystemParamItem<'_, '_, Self::Param>,
    ) -> Result<PreparedBindGroup, AsBindGroupError> {
        // Get the actual bind group layout from the pipeline cache
        let layout = pipeline_cache.get_bind_group_layout(layout_descriptor);

        // Pack all per-material values into a read-only storage buffer.
        // Layout:
        // [0] light_direction (vec4)
        // [1] ambient_color (vec4)
        // [2] diffuse_color (vec4)
        // [3] settings_1: foam_intensity, foam_threshold, sss_intensity, refraction_strength
        // [4] settings_2: wave_speed, fresnel_strength, specular_intensity, wave_amplitude
        // [5] fog_color (vec4)
        // [6] fog_params: density, min_density, max_density, wave_frequency
        // [7] depth_1: min_depth, max_depth, shallow_threshold, bottom_visibility
        // [8] deep_color (vec4)
        // [9] shallow_color (vec4)
        // [10] depth_scale: x, y, wave_layers (as float), caustics_intensity
        // [11] caustics: scale, speed, water_surface_y, padding
        let water_material_data = [
            Vec4::new(
                self.light_direction.x,
                self.light_direction.y,
                self.light_direction.z,
                0.0,
            ),
            self.ambient_color,
            self.diffuse_color,
            Vec4::new(
                self.settings.foam_intensity,
                self.settings.foam_threshold,
                self.settings.sss_intensity,
                self.settings.refraction_strength,
            ),
            Vec4::new(
                self.settings.wave_speed,
                self.settings.fresnel_strength,
                self.settings.specular_intensity,
                self.settings.wave_amplitude,
            ),
            self.fog_color,
            Vec4::new(
                self.fog_density,
                self.fog_min_density,
                self.fog_max_density,
                self.settings.wave_frequency,
            ),
            Vec4::new(
                self.settings.min_depth,
                self.settings.max_depth,
                self.settings.shallow_threshold,
                self.settings.bottom_visibility,
            ),
            self.settings.deep_color,
            self.settings.shallow_color,
            Vec4::new(
                self.settings.depth_gradient_scale[0],
                self.settings.depth_gradient_scale[1],
                self.settings.wave_layers as f32,
                self.settings.caustics_intensity,
            ),
            Vec4::new(
                self.settings.caustics_scale,
                self.settings.caustics_speed,
                self.settings.water_surface_y,
                0.0,
            ),
            // [12] reflection plane: normal (0,1,0) + distance (water surface y)
            Vec4::new(0.0, 1.0, 0.0, self.settings.water_surface_y),
            // [13] reflection params: enabled, debug_show_reflection, status, padding
            Vec4::new(
                if self.settings.reflection_enabled {
                    1.0
                } else {
                    0.0
                },
                if self.settings.debug_show_reflection {
                    1.0
                } else {
                    0.0
                },
                self.reflection_status as f32,
                0.0,
            ),
        ];
        let water_material_data_buffer =
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("water_material_data_buffer"),
                contents: bytemuck::cast_slice(&water_material_data),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            });

        // Reflection render target produced by the mirrored reflection camera.
        // Falls back to the fallback image until the real texture is available.
        use std::ops::Deref;
        let reflection_view = match image_assets.get(&self.reflection_texture) {
            Some(image) => &*image.texture_view,
            None => {
                log::warn!(
                    "[WATER MATERIAL] Reflection texture {:?} not ready, binding fallback image",
                    self.reflection_texture.id()
                );
                &*fallback_image.d2.texture_view
            }
        };
        let reflection_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("water_reflection_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        // Create bind group entries
        let entries = vec![
            BindGroupEntry {
                binding: 0,
                resource: water_material_data_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(reflection_view),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::Sampler(&reflection_sampler),
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
        // This should never be called since we override as_bind_group
        Err(AsBindGroupError::CreateBindGroupDirectly)
    }

    fn bind_group_layout_entries(
        _render_device: &RenderDevice,
        _bindless: bool,
    ) -> Vec<BindGroupLayoutEntry> {
        vec![
            // Water material data in read-only storage buffer
            // [0] light_direction
            // [1] ambient_color
            // [2] diffuse_color
            // [3] water_settings_1: foam_intensity, foam_threshold, sss_intensity, refraction_strength
            // [4] water_settings_2: wave_speed, fresnel_strength, specular_intensity, wave_amplitude
            // [5] fog_color
            // [6] fog_params: density, min_density, max_density, wave_frequency
            // [7] depth_1: min_depth, max_depth, shallow_threshold, bottom_visibility
            // [8] deep_color
            // [9] shallow_color
            // [10] depth_scale: x, y, wave_layers, caustics_intensity
            // [11] caustics: scale, speed, water_surface_y, padding
            // [12] reflection_plane: normal xyz, distance
            // [13] reflection_params: enabled, padding
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Reflection render target texture (planar reflection)
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
            // Reflection texture sampler
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ]
    }
}
