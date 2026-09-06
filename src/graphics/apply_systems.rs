//! Apply Systems for Graphics Settings
//!
//! This module contains systems that apply `GraphicsSettings` changes to the
//! actual render configuration (cameras, lights, etc.).
//!
//! INVARIANT: every camera query here excludes `WaterReflectionCamera`. Post
//! components must never be inserted on the reflection camera: `#[require]`
//! chains (SSAO pulls DepthPrepass+NormalPrepass, MotionBlur pulls
//! DepthPrepass+MotionVectorPrepass) would give it prepass phases without
//! deferred phases, and Bevy 0.18.1 panics in `queue_prepass_material_meshes`
//! (prepass/mod.rs unwrap) once a deferred material is visible to it.

use crate::graphics::*;
use bevy::{
    anti_alias::{
        fxaa::Fxaa,
        smaa::{Smaa, SmaaPreset},
    },
    core_pipeline::tonemapping::Tonemapping,
    image::Image,
    pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel},
    post_process::motion_blur::MotionBlur,
    prelude::*,
    render::view::ColorGrading,
};
use bevy_light::{
    CascadeShadowConfig, DirectionalLight, DirectionalLightShadowMap, ShadowFilteringMethod,
};
use bevy_post_process::bloom::Bloom;
use bevy_post_process::dof::DepthOfField;

/// System that applies color grading settings (brightness, contrast, saturation, gamma)
/// to all cameras with ColorGrading components.
pub fn apply_color_grading_system(
    graphics_settings: Res<GraphicsSettings>,
    mut cameras: Query<
        &mut ColorGrading,
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    for mut color_grading in cameras.iter_mut() {
        // Apply brightness through exposure
        // Map 0.0-2.0 to -2 to +2 EV stops (1.0 = neutral)
        color_grading.global.exposure = (graphics_settings.brightness - 1.0) * 2.0;

        // Apply contrast to all sections
        color_grading.shadows.contrast = graphics_settings.contrast;
        color_grading.midtones.contrast = graphics_settings.contrast;
        color_grading.highlights.contrast = graphics_settings.contrast;

        // Apply saturation through post_saturation
        color_grading.global.post_saturation = graphics_settings.saturation;

        // Apply gamma to all sections
        color_grading.shadows.gamma = graphics_settings.gamma;
        color_grading.midtones.gamma = graphics_settings.gamma;
        color_grading.highlights.gamma = graphics_settings.gamma;
    }
}

/// System that applies shadow quality settings to directional lights.
/// Skips MoonLight: the time-of-day table owns moon shadows (always off for perf).
/// Skips SkyFillLight: the sky-bounce fill never casts shadows by design.
pub fn apply_shadow_quality_system(
    graphics_settings: Res<GraphicsSettings>,
    mut directional_lights: Query<
        (&mut DirectionalLight, Option<&mut CascadeShadowConfig>),
        (
            Without<crate::render::MoonLight>,
            Without<crate::render::zone_lighting::SkyFillLight>,
        ),
    >,
    mut shadow_map_resource: ResMut<DirectionalLightShadowMap>,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    let quality = &graphics_settings.shadow_quality;

    // Enable/disable shadows based on quality
    let shadow_maps_enabled = *quality != ShadowQuality::Off;

    // Only update shadow map resolution when shadows are enabled.
    // wgpu requires non-zero texture dimensions, so we keep the previous/valid size
    // when shadows are disabled. The shadow_maps_enabled flag on lights controls
    // whether shadows are actually rendered.
    if shadow_maps_enabled {
        shadow_map_resource.size = quality.shadow_map_size();
    }

    for (mut light, cascade_config) in directional_lights.iter_mut() {
        // Enable/disable shadows based on quality.
        // MoonLight is owned by the time-of-day table (always off); don't force it on here.
        // (Query can't filter by MoonLight without importing it; time-of-day corrects any
        // transient on the next ZoneTime change, so this stays a bounded one-frame effect.)
        light.shadow_maps_enabled = shadow_maps_enabled;

        // Apply cascade configuration if present
        if let Some(mut config) = cascade_config {
            let cascade_count = quality.cascade_count();
            if cascade_count > 0 {
                let max_distance = graphics_settings
                    .shadow_max_distance
                    .min(quality.max_distance());

                // Calculate bounds for cascades
                let first_bound = max_distance / cascade_count as f32;
                let bounds: Vec<f32> = (0..cascade_count)
                    .map(|i| first_bound * (i + 1) as f32)
                    .collect();

                config.bounds = bounds;
                config.overlap_proportion = 0.2;
                config.minimum_distance = 0.1;
            }
        }
    }
}

/// System that applies tonemapping settings to cameras.
pub fn apply_tonemapping_system(
    graphics_settings: Res<GraphicsSettings>,
    mut cameras: Query<
        &mut Tonemapping,
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    for mut tonemapping in cameras.iter_mut() {
        *tonemapping = match graphics_settings.tonemapping {
            TonemappingMode::None => Tonemapping::None,
            TonemappingMode::Reinhard => Tonemapping::Reinhard,
            TonemappingMode::ReinhardLuminance => Tonemapping::ReinhardLuminance,
            TonemappingMode::AcesFitted => Tonemapping::AcesFitted,
            TonemappingMode::AgX => Tonemapping::AgX,
            TonemappingMode::SomewhatBoringDisplayTransform => {
                Tonemapping::SomewhatBoringDisplayTransform
            }
            TonemappingMode::TonyMcMapface => Tonemapping::TonyMcMapface,
            TonemappingMode::BlenderFilmic => Tonemapping::BlenderFilmic,
        };
    }
}

/// System that applies bloom settings to cameras.
/// Disabling REMOVES the component so the bloom pyramid pass is skipped entirely.
/// Previously intensity was set to 0.0, which still dispatched the full pass.
pub fn apply_bloom_system(
    graphics_settings: Res<GraphicsSettings>,
    mut commands: Commands,
    cameras: Query<
        (Entity, Option<&Bloom>),
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    for (entity, bloom) in cameras.iter() {
        if graphics_settings.bloom_enabled {
            if let Some(_existing) = bloom {
                // Update intensity in place via separate query-less path: re-insert
                // preserves settings while keeping code simple (change-gated, rare).
                commands.entity(entity).insert(Bloom {
                    intensity: graphics_settings.bloom_intensity,
                    ..Bloom::NATURAL
                });
            } else {
                commands.entity(entity).insert(Bloom {
                    intensity: graphics_settings.bloom_intensity,
                    ..Bloom::NATURAL
                });
            }
        } else if bloom.is_some() {
            commands.entity(entity).remove::<Bloom>();
        }
    }
}

/// System that applies shadow filtering method to lights.
pub fn apply_shadow_filtering_system(
    graphics_settings: Res<GraphicsSettings>,
    mut lights: Query<&mut ShadowFilteringMethod, With<DirectionalLight>>,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    for mut filtering in lights.iter_mut() {
        *filtering = match graphics_settings.shadow_filtering {
            GraphicsShadowFilteringMethod::Hardware2x2 => ShadowFilteringMethod::Hardware2x2,
            GraphicsShadowFilteringMethod::Gaussian => ShadowFilteringMethod::Gaussian,
            GraphicsShadowFilteringMethod::Temporal => ShadowFilteringMethod::Temporal,
        };
    }
}

/// System that applies MSAA settings to cameras.
pub fn apply_msaa_system(
    graphics_settings: Res<GraphicsSettings>,
    mut cameras: Query<
        &mut Msaa,
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    let new_msaa = match graphics_settings.msaa_samples {
        MsaaSamples::X1 => Msaa::Off,
        MsaaSamples::X2 => Msaa::Sample2,
        MsaaSamples::X4 => Msaa::Sample4,
        MsaaSamples::X8 => Msaa::Sample8,
    };

    for mut msaa in cameras.iter_mut() {
        *msaa = new_msaa;
    }
}

/// System that applies SSAO enable/quality to cameras via insert/remove.
/// Previously `ssao_enabled` was never read here and the camera always kept SSAO
/// (downgraded to Low at best), so Low settings still paid the full SSAO pass.
pub fn apply_ssao_system(
    graphics_settings: Res<GraphicsSettings>,
    mut commands: Commands,
    cameras: Query<
        (Entity, Option<&ScreenSpaceAmbientOcclusion>),
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    if !graphics_settings.is_changed() {
        return;
    }

    for (entity, ssao) in cameras.iter() {
        if !graphics_settings.ssao_enabled || graphics_settings.ssao_quality == SsaoQuality::Off
        {
            if ssao.is_some() {
                commands
                    .entity(entity)
                    .remove::<ScreenSpaceAmbientOcclusion>();
            }
            continue;
        }
        let level = match graphics_settings.ssao_quality {
            SsaoQuality::Off => continue, // handled above
            SsaoQuality::Low => ScreenSpaceAmbientOcclusionQualityLevel::Low,
            SsaoQuality::Medium => ScreenSpaceAmbientOcclusionQualityLevel::Medium,
            SsaoQuality::High => ScreenSpaceAmbientOcclusionQualityLevel::High,
            SsaoQuality::Ultra => ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
        };
        commands.entity(entity).insert(ScreenSpaceAmbientOcclusion {
            quality_level: level,
            ..Default::default()
        });
    }
}

/// System that applies SMAA quality to cameras via insert/remove.
/// The camera no longer spawns SMAA by default; this adds it only when requested.
pub fn apply_smaa_system(
    graphics_settings: Res<GraphicsSettings>,
    mut commands: Commands,
    cameras: Query<
        (Entity, Option<&Smaa>),
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    if !graphics_settings.is_changed() {
        return;
    }

    for (entity, smaa) in cameras.iter() {
        let preset = match graphics_settings.smaa_quality {
            SmaaQuality::Disabled => None,
            SmaaQuality::Low => Some(SmaaPreset::Low),
            SmaaQuality::Medium => Some(SmaaPreset::Medium),
            SmaaQuality::High => Some(SmaaPreset::High),
            SmaaQuality::Ultra => Some(SmaaPreset::Ultra),
        };
        match preset {
            None => {
                if smaa.is_some() {
                    commands.entity(entity).remove::<Smaa>();
                }
            }
            Some(preset) => {
                let needs_insert = match smaa {
                    Some(existing) => existing.preset != preset,
                    None => true,
                };
                if needs_insert {
                    commands.entity(entity).insert(Smaa {
                        preset,
                        ..Default::default()
                    });
                }
            }
        }
    }
}

/// System that applies motion-blur enable to cameras via insert/remove.
/// Previously the camera hard-added MotionBlur with no removal path.
pub fn apply_motion_blur_system(
    graphics_settings: Res<GraphicsSettings>,
    mut commands: Commands,
    cameras: Query<
        (Entity, Option<&MotionBlur>),
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    if !graphics_settings.is_changed() {
        return;
    }

    // motion_blur_intensity (0-1 slider) maps to shutter_angle (0-2.0 rad scale).
    let shutter_angle = (graphics_settings.motion_blur_intensity.clamp(0.0, 1.0) * 2.0).max(0.05);
    for (entity, blur) in cameras.iter() {
        if graphics_settings.motion_blur_enabled {
            let needs_insert = match blur {
                Some(existing) => (existing.shutter_angle - shutter_angle).abs() > 0.01,
                None => true,
            };
            if needs_insert {
                commands.entity(entity).insert(MotionBlur {
                    shutter_angle,
                    ..Default::default()
                });
            }
        } else if blur.is_some() {
            commands.entity(entity).remove::<MotionBlur>();
        }
    }
}

/// System that applies depth-of-field enable from GraphicsSettings.
/// The richer DepthOfFieldSettings resource owns the parameters; this only ensures
/// `dof_enabled=false` removes the component (insert path is owned by
/// apply_depth_of_field_settings so parameters stay in one place).
pub fn apply_dof_enabled_system(
    graphics_settings: Res<GraphicsSettings>,
    mut commands: Commands,
    cameras: Query<
        (Entity, Option<&DepthOfField>),
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    if !graphics_settings.is_changed() {
        return;
    }

    if graphics_settings.dof_enabled {
        return;
    }
    for (entity, dof) in cameras.iter() {
        if dof.is_some() {
            commands.entity(entity).remove::<DepthOfField>();
        }
    }
}

/// System that applies FXAA enable to cameras via insert/remove.
/// Previously `fxaa_enabled` (including Low preset's `fxaa:true` fallback) had no
/// consumer, so the preset promised AA it never got.
pub fn apply_fxaa_system(
    graphics_settings: Res<GraphicsSettings>,
    mut commands: Commands,
    cameras: Query<
        (Entity, Option<&Fxaa>),
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    if !graphics_settings.is_changed() {
        return;
    }

    for (entity, fxaa) in cameras.iter() {
        if graphics_settings.fxaa_enabled {
            if fxaa.is_none() {
                commands.entity(entity).insert(Fxaa::default());
            }
        } else if fxaa.is_some() {
            commands.entity(entity).remove::<Fxaa>();
        }
    }
}

/// System that wires view_distance to camera far plane.
/// Previously the slider (100-2000m) had no consumer: users thought they lowered cost
/// but nothing changed. Mapping far = clamp(view_distance * 16, 6000, 12000):
/// default 500 -> 8000 (matches startup), Low 300 -> 6000 (tighter depth precision),
/// High/Ultra saturate at 12000 (sky radius 4000 + margin). Sky follows the camera so
/// it is never clipped within this range.
pub fn apply_view_distance_system(
    graphics_settings: Res<GraphicsSettings>,
    mut cameras: Query<&mut Projection, (With<Camera>, Without<crate::render::WaterReflectionCamera>)>,
) {
    if !graphics_settings.is_changed() {
        return;
    }
    let far = (graphics_settings.view_distance * 16.0).clamp(6000.0, 12000.0);
    for mut projection in cameras.iter_mut() {
        if let Projection::Perspective(ref mut perspective) = *projection {
            if (perspective.far - far).abs() > 1.0 {
                perspective.far = far;
            }
        }
    }
}

/// System that wires texture_quality to image sampler LOD clamps.
/// Previously `mip_bias()` was dead code (never applied). Mapping bias to
/// lod_min_clamp forces smaller mips on Low (faster, blurrier) and full res on
/// High/Ultra. Only runs on settings change; converts Default samplers to explicit
/// descriptors preserving all other fields.
pub fn apply_texture_quality_system(
    graphics_settings: Res<GraphicsSettings>,
    mut images: ResMut<Assets<Image>>,
) {
    if !graphics_settings.is_changed() {
        return;
    }
    let bias = graphics_settings.texture_quality.mip_bias();
    // bias 2.0/1.0 (Low/Medium) -> force mip >= 2/1; 0.0/-0.5 (High/Ultra) -> full res.
    let lod_min = bias.max(0.0);
    for (_, image) in images.iter_mut() {
        let descriptor = image.sampler.get_or_init_descriptor();
        if (descriptor.lod_min_clamp - lod_min).abs() > f32::EPSILON {
            descriptor.lod_min_clamp = lod_min;
        }
    }
}

/// System that applies ambient lighting settings to the global AmbientLight resource.
/// The brightness is multiplied by a base value of 80.0 (Bevy's default ambient brightness).
///
/// NOTE: `sync_zone_lighting_to_bevy_lights_system` (Update) is the authority for
/// the ambient color/brightness each frame: it blends the zone's
/// `map_ambient_color` with the user's color and scales brightness by daylight
/// so backlit faces stay readable at noon. This system (PostUpdate) only seeds
/// the same blend when settings change so there is no one-frame flash of flat
/// white ambient and no ping-pong between the two writers.
pub fn apply_ambient_light_system(
    graphics_settings: Res<GraphicsSettings>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
    zone_lighting: Option<Res<crate::render::ZoneLighting>>,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    let user_color = graphics_settings.ambient_light_color.to_linear();
    if let Some(zone_lighting) = zone_lighting {
        let map = zone_lighting.map_ambient_color;
        ambient_light.color = Color::from(LinearRgba::new(
            map.x * user_color.red,
            map.y * user_color.green,
            map.z * user_color.blue,
            1.0,
        ));
    } else {
        // No zone loaded yet (menus): fall back to the plain user color.
        ambient_light.color = graphics_settings.ambient_light_color;
    }

    // Apply ambient light brightness
    // Base brightness is 80.0 (Bevy's default), multiplier ranges from 0.0 to 2.0.
    // Daylight scaling itself is applied by the sync system in Update.
    ambient_light.brightness = 80.0 * graphics_settings.ambient_light_brightness;
}
