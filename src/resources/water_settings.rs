//! Water rendering settings resource
//!
//! This resource stores configurable parameters for water rendering,
//! allowing real-time adjustment through the settings UI.

use bevy::prelude::Resource;

/// Resource for storing water rendering settings that can be modified at runtime.
#[derive(Resource, Debug, Clone)]
pub struct WaterSettings {
    /// Foam intensity (0.0-1.0) - controls how visible foam effects are on wave crests
    pub foam_intensity: f32,
    /// Foam threshold (0.0-1.0) - wave height at which foam starts appearing
    pub foam_threshold: f32,
    /// Subsurface scattering intensity (0.0-1.0) - light scattering through water
    pub sss_intensity: f32,
    /// Refraction strength (0.0-0.2) - UV distortion amount for pseudo-refraction
    pub refraction_strength: f32,
    /// Wave speed multiplier (0.1-5.0) - how fast waves animate
    pub wave_speed: f32,
    /// Fresnel strength (0.0-1.0) - angle-dependent reflectivity
    pub fresnel_strength: f32,
    /// Specular intensity (0.0-1.0) - sun highlight brightness
    pub specular_intensity: f32,
}

impl Default for WaterSettings {
    fn default() -> Self {
        Self {
            foam_intensity: 0.4,
            foam_threshold: 0.3,
            sss_intensity: 1.0,
            refraction_strength: 0.05,
            wave_speed: 1.0,
            fresnel_strength: 0.5,
            specular_intensity: 0.5,
        }
    }
}
