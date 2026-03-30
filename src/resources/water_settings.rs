//! Water rendering settings resource
//!
//! This resource stores configurable parameters for water rendering,
//! allowing real-time adjustment through the settings UI.

use bevy::prelude::{Resource, Vec4};

/// Resource for storing water rendering settings that can be modified at runtime.
#[derive(Resource, Debug, Clone)]
pub struct WaterSettings {
    // === Existing settings (kept for compatibility) ===
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
    /// Y coordinate of the water surface in world space
    /// Used for underwater detection and effects
    pub water_surface_y: f32,

    // === New depth-related settings ===
    /// Minimum water depth in meters (shallow areas)
    pub min_depth: f32,
    /// Maximum water depth in meters (deep areas)
    pub max_depth: f32,
    /// Depth below which the bottom becomes visible (meters)
    pub shallow_threshold: f32,
    /// Color of deep water (RGBA)
    pub deep_color: Vec4,
    /// Color of shallow water (RGBA)
    pub shallow_color: Vec4,
    /// How much the bottom shows through in shallow water (0.0-1.0)
    pub bottom_visibility: f32,
    /// Scale for depth variation pattern (XZ)
    pub depth_gradient_scale: [f32; 2],

    // === New wave settings ===
    /// Height of waves (amplitude)
    pub wave_amplitude: f32,
    /// How many waves per unit distance (frequency)
    pub wave_frequency: f32,
    /// Number of wave layers for complexity (1-4)
    pub wave_layers: u32,

    // === New caustics settings ===
    /// Caustics intensity (0.0-1.0)
    pub caustics_intensity: f32,
    /// Caustics pattern scale
    pub caustics_scale: f32,
    /// Caustics animation speed
    pub caustics_speed: f32,
}

impl Default for WaterSettings {
    fn default() -> Self {
        Self {
            // Existing settings
            foam_intensity: 0.5,
            foam_threshold: 0.8,
            sss_intensity: 1.0,
            refraction_strength: 0.05,
            wave_speed: 1.0,
            fresnel_strength: 0.5,
            specular_intensity: 0.5,
            water_surface_y: 0.0,

            // Depth settings
            min_depth: 0.5,      // Shallow water at edges
            max_depth: 10.0,     // Deep water in center
            shallow_threshold: 2.0, // Bottom visible below 2m depth
            deep_color: Vec4::new(0.0, 0.2, 0.4, 0.9),    // Deep blue
            shallow_color: Vec4::new(0.3, 0.6, 0.7, 0.5), // Light turquoise
            bottom_visibility: 0.6,
            depth_gradient_scale: [0.02, 0.02],

            // Wave settings
            wave_amplitude: 0.5,
            wave_frequency: 2.0,
            wave_layers: 3,

            // Caustics settings
            caustics_intensity: 0.3,
            caustics_scale: 0.1,
            caustics_speed: 0.5,
        }
    }
}
