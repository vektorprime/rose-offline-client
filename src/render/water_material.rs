//! Custom water material with texture array support for animated water
//!
//! This module implements a custom material that supports:
//! - Up to 25 water animation frames in a binding_array
//! - Frame blending based on time for smooth animation
//! - Additive blending (SrcAlpha + One) for water transparency
//! - Depth write disabled for proper water rendering
//! - Zone lighting integration

use std::num::NonZeroU32;

use bevy::{
    asset::{load_internal_asset, Asset, AssetApp, Handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::{
        Material, MaterialPipeline, MaterialPipelineKey, MeshPipelineKey,
    },
    prelude::{App, Mesh, Plugin, Res, ResMut, Resource, Time, World},
    reflect::TypePath,
    render::{
        alpha::AlphaMode,
        mesh::MeshVertexBufferLayoutRef,
        render_asset::RenderAssets,
        render_resource::*,
        renderer::RenderDevice,
        texture::{FallbackImage, GpuImage},
    },
};

/// Shader handle for the water material shader
pub const WATER_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x333959e64b35d5d9);

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
        app.add_plugins(bevy::pbr::MaterialPlugin::<WaterMaterial> {
            prepass_enabled: false,  // Disable prepass for transparent water
            shadows_enabled: false,  // Water doesn't cast shadows
            ..Default::default()
        });
        
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

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
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

    fn label() -> Option<&'static str> {
        Some("water_material")
    }

    /// Override as_bind_group to create bind group with texture array
    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        (image_assets, fallback_image): &mut SystemParamItem<'_, '_, Self::Param>,
    ) -> Result<PreparedBindGroup<Self::Data>, AsBindGroupError> {
        use std::ops::Deref;
        
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
        ];

        // Create bind group
        let bind_group = render_device.create_bind_group(Self::label(), layout, &entries);

        Ok(PreparedBindGroup {
            bindings: vec![],
            bind_group,
            data: WaterMaterialKey {
                texture_count: self.textures.len() as u32,
            },
        })
    }

    /// Required by trait even though we override as_bind_group
    fn unprepared_bind_group(
        &self,
        _layout: &BindGroupLayout,
        _render_device: &RenderDevice,
        _param: &mut SystemParamItem<'_, '_, Self::Param>,
    ) -> Result<UnpreparedBindGroup<Self::Data>, AsBindGroupError> {
        // This should never be called since we override as_bind_group
        Err(AsBindGroupError::RetryNextUpdate)
    }

    fn bind_group_layout_entries(
        _render_device: &RenderDevice,
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
