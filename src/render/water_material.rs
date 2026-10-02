//! Custom procedural water material (no texture dependencies)
//!
//! This module implements a custom material that supports:
//! - Fully procedural, physically based water shading in WGSL (Fresnel,
//!   absorption, sun/moon glints, caustics, foam), lit by the view's actual
//!   directional/ambient lights so day and night match the scene
//! - Planar reflections from the mirrored reflection camera's render target
//! - Premultiplied-alpha blending over the opaque scene, depth write disabled
//! - Configurable water settings via WaterSettings resource

use bevy::{
    asset::{load_internal_asset, weak_handle, Asset, AssetApp, AssetId, Assets, Handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    image::Image,
    material::AlphaMode,
    math::Vec4,
    pbr::{Material, MaterialPipeline, MaterialPipelineKey},
    prelude::{App, Local, Plugin, Res, ResMut, Update},
    reflect::TypePath,
    render::{
        render_asset::RenderAssets,
        render_resource::*,
        renderer::RenderDevice,
        texture::{FallbackImage, GpuImage},
    },
};
use bevy_mesh::{Mesh, MeshVertexBufferLayoutRef};
use bevy_shader::{Shader, ShaderRef};

use crate::{render::starry_sky_material::StarrySkySettings, resources::WaterSettings};

/// Shader handle for the water material shader
pub const WATER_MATERIAL_SHADER_HANDLE: Handle<bevy_shader::Shader> =
    weak_handle!("333959e6-4b35-d5d9-0000-000000000000");

/// Number of vec4s in the material data storage buffer. Must match
/// `water_material_data` in `water_material.wgsl`.
const WATER_MATERIAL_DATA_LEN: usize = 8;

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

        app.add_systems(Update, sync_water_sky_night_factor);

        log::info!("[WATER MATERIAL] WaterMaterialPlugin loaded");
    }
}

/// Custom water material for fully procedural water shading.
///
/// Lighting (sun, moon, sky fill, ambient) and exposure come from Bevy's view
/// bindings in the shader, so the material only carries the settings and the
/// reflection inputs.
#[derive(Asset, Debug, Clone, TypePath)]
pub struct WaterMaterial {
    /// Water rendering settings
    pub settings: WaterSettings,
    /// Off-screen texture containing the mirrored scene rendered by the
    /// reflection camera. Sampled in the fragment shader for planar reflections.
    pub reflection_texture: Handle<Image>,
    /// Reflection camera status written by the water reflection plugin
    /// (0 = camera disabled: the texture is stale and must not be sampled,
    /// 1 = active but no entities visible, 2 = few entities, 3 = ok)
    pub reflection_status: u32,
    /// `StarrySkySettings::night_factor`, quantized (see
    /// `sync_water_sky_night_factor`): how much the starry sky covers the
    /// atmosphere, so the water's own sky estimate goes dark at night.
    pub sky_night_factor: f32,
}

/// Default implementation for WaterMaterial
impl Default for WaterMaterial {
    fn default() -> Self {
        Self {
            // Default water settings
            settings: WaterSettings::default(),
            // Default handle; the water reflection plugin replaces it with the
            // actual reflection render target once it exists.
            reflection_texture: Handle::default(),
            reflection_status: 0,
            sky_night_factor: 0.0,
        }
    }
}

/// Steps per unit of night factor. The night factor ramps every frame during
/// evening and morning, and every change re-prepares the water bind group, so
/// it is quantized (the steps are invisible in the water's sky reflection).
const SKY_NIGHT_FACTOR_STEPS: f32 = 32.0;

/// Copies the starry sky's night factor into every water material, only when
/// the quantized value changes.
fn sync_water_sky_night_factor(
    starry_sky_settings: Option<Res<StarrySkySettings>>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    mut stale_materials: Local<Vec<AssetId<WaterMaterial>>>,
) {
    let night_factor = starry_sky_settings
        .map(|settings| settings.night_factor)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let night_factor = (night_factor * SKY_NIGHT_FACTOR_STEPS).round() / SKY_NIGHT_FACTOR_STEPS;

    // Read-only scan first: any write through `get_mut` marks a material
    // modified (re-prepared), even when the written value is identical.
    stale_materials.clear();
    stale_materials.extend(
        water_materials
            .iter()
            .filter(|(_, material)| material.sky_night_factor != night_factor)
            .map(|(id, _)| id),
    );
    for id in stale_materials.drain(..) {
        if let Some(mut material) = water_materials.get_mut(id) {
            material.sky_night_factor = night_factor;
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

    /// The shader outputs premultiplied color: reflection, glints and foam are
    /// added at full strength while alpha only says how much of the lake bed
    /// behind the surface is hidden.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
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
            .depth_write_enabled = Some(false);

        // Set up vertex buffer layout
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        // Premultiplied alpha (explicit, matching alpha_mode): out = src + dst * (1 - src.a).
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING);
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

    /// Builds the bindings with packed per-material data.
    ///
    /// Returned unprepared (instead of overriding `as_bind_group` and returning
    /// `CreateBindGroupDirectly`) so Bevy's material allocator frees the previous
    /// bind group when the material is modified. On Bevy 0.19.1 the
    /// `CreateBindGroupDirectly` path never frees it, which leaked a bind group,
    /// buffer and sampler per modification.
    fn unprepared_bind_group(
        &self,
        _layout: &BindGroupLayout,
        render_device: &RenderDevice,
        (image_assets, fallback_image): &mut SystemParamItem<'_, '_, Self::Param>,
        _bindless: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        let settings = &self.settings;
        // Pack all per-material values into a read-only storage buffer.
        // Layout (must match the accessors in water_material.wgsl):
        // [0] waves: wave_amplitude, wave_frequency, wave_speed, wave_layers
        // [1] surface: fresnel_strength, specular_intensity, sss_intensity, refraction_strength
        // [2] foam/caustics: foam_intensity, foam_threshold, caustics_intensity, caustics_scale
        // [3] depth: min_depth, max_depth, shallow_threshold, bottom_visibility
        // [4] deep_color
        // [5] shallow_color
        // [6] depth_gradient_scale.xy, caustics_speed, water_surface_y
        // [7] reflection: enabled, debug_show_reflection, status, sky night factor
        let water_material_data: [Vec4; WATER_MATERIAL_DATA_LEN] = [
            Vec4::new(
                settings.wave_amplitude,
                settings.wave_frequency,
                settings.wave_speed,
                settings.wave_layers as f32,
            ),
            Vec4::new(
                settings.fresnel_strength,
                settings.specular_intensity,
                settings.sss_intensity,
                settings.refraction_strength,
            ),
            Vec4::new(
                settings.foam_intensity,
                settings.foam_threshold,
                settings.caustics_intensity,
                settings.caustics_scale,
            ),
            Vec4::new(
                settings.min_depth,
                settings.max_depth,
                settings.shallow_threshold,
                settings.bottom_visibility,
            ),
            settings.deep_color,
            settings.shallow_color,
            Vec4::new(
                settings.depth_gradient_scale[0],
                settings.depth_gradient_scale[1],
                settings.caustics_speed,
                settings.water_surface_y,
            ),
            Vec4::new(
                if settings.reflection_enabled {
                    1.0
                } else {
                    0.0
                },
                if settings.debug_show_reflection {
                    1.0
                } else {
                    0.0
                },
                self.reflection_status as f32,
                self.sky_night_factor,
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
        let reflection_view = match image_assets.get(&self.reflection_texture) {
            Some(image) => image.texture_view.clone(),
            None => {
                log::warn!(
                    "[WATER MATERIAL] Reflection texture {:?} not ready, binding fallback image",
                    self.reflection_texture.id()
                );
                fallback_image.d2.texture_view.clone()
            }
        };
        let reflection_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("water_reflection_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        });

        Ok(UnpreparedBindGroup {
            bindings: BindingResources(vec![
                (0, OwnedBindingResource::Buffer(water_material_data_buffer)),
                (
                    1,
                    OwnedBindingResource::TextureView(TextureViewDimension::D2, reflection_view),
                ),
                (
                    2,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Filtering,
                        reflection_sampler,
                    ),
                ),
            ]),
        })
    }

    fn bind_group_layout_entries(
        _render_device: &RenderDevice,
        _bindless: bool,
    ) -> Vec<BindGroupLayoutEntry> {
        vec![
            // Water material data in read-only storage buffer (layout documented
            // in unprepared_bind_group)
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
