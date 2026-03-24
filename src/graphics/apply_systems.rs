//! Apply Systems for Graphics Settings
//!
//! This module contains systems that apply `GraphicsSettings` changes to the
//! actual render configuration (cameras, lights, etc.).

use crate::graphics::*;
use bevy::{core_pipeline::tonemapping::Tonemapping, prelude::*, render::view::ColorGrading};
use bevy_light::{CascadeShadowConfig, DirectionalLight, DirectionalLightShadowMap, ShadowFilteringMethod};
use bevy_post_process::bloom::Bloom;

/// System that applies color grading settings (brightness, contrast, saturation, gamma)
/// to all cameras with ColorGrading components.
pub fn apply_color_grading_system(
    graphics_settings: Res<GraphicsSettings>,
    mut cameras: Query<&mut ColorGrading, With<Camera>>,
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
pub fn apply_shadow_quality_system(
    graphics_settings: Res<GraphicsSettings>,
    mut directional_lights: Query<(&mut DirectionalLight, Option<&mut CascadeShadowConfig>)>,
    mut shadow_map_resource: ResMut<DirectionalLightShadowMap>,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    let quality = &graphics_settings.shadow_quality;

    // Enable/disable shadows based on quality
    let shadows_enabled = *quality != ShadowQuality::Off;

    // Only update shadow map resolution when shadows are enabled.
    // wgpu requires non-zero texture dimensions, so we keep the previous/valid size
    // when shadows are disabled. The shadows_enabled flag on lights controls
    // whether shadows are actually rendered.
    if shadows_enabled {
        shadow_map_resource.size = quality.shadow_map_size();
    }

    for (mut light, cascade_config) in directional_lights.iter_mut() {
        // Enable/disable shadows based on quality
        light.shadows_enabled = shadows_enabled;

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
    mut cameras: Query<&mut Tonemapping, With<Camera>>,
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
pub fn apply_bloom_system(
    graphics_settings: Res<GraphicsSettings>,
    mut cameras: Query<&mut Bloom, With<Camera>>,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    for mut bloom in cameras.iter_mut() {
        if graphics_settings.bloom_enabled {
            bloom.intensity = graphics_settings.bloom_intensity;
        } else {
            bloom.intensity = 0.0;
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
    mut cameras: Query<&mut Msaa, With<Camera>>,
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

/// System that applies ambient lighting settings to the global AmbientLight resource.
/// The brightness is multiplied by a base value of 80.0 (Bevy's default ambient brightness).
pub fn apply_ambient_light_system(
    graphics_settings: Res<GraphicsSettings>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    // Apply ambient light color
    ambient_light.color = graphics_settings.ambient_light_color;

    // Apply ambient light brightness
    // Base brightness is 80.0 (Bevy's default), multiplier ranges from 0.0 to 2.0
    ambient_light.brightness =80.0 * graphics_settings.ambient_light_brightness;
}
