//! Custom water material with texture array support for animated water
//!
//! This module implements a custom material that supports:
//! - Up to 25 water animation frames in a binding_array
//! - Frame blending based on time for smooth animation
//! - Additive blending (SrcAlpha + One) for water transparency
//! - Depth write disabled for proper water rendering
//! - Zone lighting integration
//! - Configurable water settings via WaterSettings resource

use std::num::NonZeroU32;

use bevy::{
    asset::{load_internal_asset, Asset, AssetApp, Handle, weak_handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    math::{Vec3, Vec4},
    pbr::{
        Material, MaterialPipeline, MaterialPipelineKey, MeshPipelineKey,
    },
    prelude::{App, Plugin, Res, ResMut, Resource, Time, World},
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

/// Number of water animation frames
pub const WATER_MATERIAL_NUM_TEXTURES: usize = 25;

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
        
        // Register the water time resource
        app.init_resource::<WaterAnimationTime>();
        
        log::info!("[WATER MATERIAL] WaterMaterialPlugin loaded");
    }
}

/// Resource to track water animation time
#[derive(Resource, Default, Clone, Copy)]
pub struct WaterAnimationTime {
    /// Current frame index (0-24)
    pub current_index: u32,
    /// Next frame index for blending (0-24)
    pub next_index: u32,
    /// Blend factor between frames (0.0-1.0)
    pub blend: f32,
}

/// Custom water material supporting animated water textures via texture array
#[derive(Asset, Debug, Clone, TypePath)]
pub struct WaterMaterial {
    /// Array of water texture handles (25 frames for animation)
    pub textures: Vec<Handle<bevy::image::Image>>,
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
}

/// Default implementation for WaterMaterial
impl Default for WaterMaterial {
    fn default() -> Self {
        Self {
            textures: Vec::new(),
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
        }
    }
}

/// Data stored alongside the prepared bind group
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct WaterMaterialKey {
    pub texture_count: u32,
}

impl From<&WaterMaterial> for WaterMaterialKey {
    fn from(material: &WaterMaterial) -> Self {
        WaterMaterialKey {
            texture_count: material.textures.len() as u32,
        }
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
        // SrcAlpha + One gives the glowing water effect
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
        }

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
        WaterMaterialKey {
            texture_count: self.textures.len() as u32,
        }
    }

    /// Override as_bind_group to create bind group with texture array and lighting uniforms
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
        for handle in self.textures.iter().take(WATER_MATERIAL_NUM_TEXTURES) {
            match image_assets.get(handle) {
                Some(image) => images.push(image),
                None => return Err(AsBindGroupError::RetryNextUpdate),
            }
        }

        // Build texture view array with fallback for missing slots
        let fallback_view = &*fallback_image.d2.texture_view;
        let mut textures: Vec<&_> = vec![fallback_view; WATER_MATERIAL_NUM_TEXTURES];
        for (id, image) in images.into_iter().enumerate() {
            textures[id] = &*image.texture_view;
        }

        // Create sampler with repeat address mode for tiling water
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        // wgpu 27 forbids mixing binding arrays and uniform buffers in one bind group.
        // Pack all non-texture data into a read-only storage buffer.
        // Layout:
        // [0] light_direction (vec4)
        // [1] ambient_color (vec4)
        // [2] diffuse_color (vec4)
        // [3] settings_1: foam_intensity, foam_threshold, sss_intensity, refraction_strength
        // [4] settings_2: wave_speed, fresnel_strength, specular_intensity, padding
        // [5] fog_color (vec4)
        // [6] fog_params: density, min_density, max_density, padding
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
                0.0,
            ),
            self.fog_color,
            Vec4::new(
                self.fog_density,
                self.fog_min_density,
                self.fog_max_density,
                0.0,
            ),
        ];
        let water_material_data_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("water_material_data_buffer"),
            contents: bytemuck::cast_slice(&water_material_data),
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
                resource: water_material_data_buffer.as_entire_binding(),
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
            // Texture array binding (25 water frames)
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
            // Sampler binding
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
            // Water material data in read-only storage buffer
            // [0] light_direction
            // [1] ambient_color
            // [2] diffuse_color
            // [3] water_settings_1
            // [4] water_settings_2
            // [5] fog_color
            // [6] fog_params
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

/// System to update water animation time
pub fn update_water_animation_time(
    time: Res<Time>,
    mut water_time: ResMut<WaterAnimationTime>,
) {
    // Animate at 10 frames per second for smooth water movement
    let elapsed = time.elapsed_secs_wrapped() * 10.0;
    let current_index = (elapsed as u32) % WATER_MATERIAL_NUM_TEXTURES as u32;
    let next_index = (current_index + 1) % WATER_MATERIAL_NUM_TEXTURES as u32;
    let blend = elapsed.fract();
    
    water_time.current_index = current_index;
    water_time.next_index = next_index;
    water_time.blend = blend;
}
