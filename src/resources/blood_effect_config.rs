//! Configuration resource for the blood effects system.
//!
//! This resource controls global blood effect settings including:
//! - Enable/disable blood effects
//! - Maximum spatter count
//! - Spatter lifetime and fade behavior
//! - Wound visibility settings

use bevy::{prelude::*, reflect::Reflect};

/// Global configuration for blood effects.
///
/// This resource controls the behavior and limits of the blood effects system.
/// Insert this resource into your app to customize blood effect behavior.
#[derive(Resource, Reflect, Clone, Debug)]
#[reflect(Resource)]
pub struct BloodEffectConfig {
    /// Whether blood effects are enabled globally.
    ///
    /// When disabled, no blood spatters or wounds will be spawned.
    pub enable_blood: bool,

    /// Maximum number of blood spatters allowed in the scene at once.
    ///
    /// When this limit is reached, oldest spatters are removed to make room
    /// for new ones (LRU eviction). Set to 0 for unlimited spatters (not recommended).
    pub max_spatters: usize,

    /// How long blood spatters persist before fading out (in seconds).
    ///
    /// Default is 30 seconds. Spatters will begin fading in the last 5 seconds.
    pub spatter_lifetime: f32,

    /// Blood intensity multiplier (0.0 - 1.0).
    ///
    /// This affects:
    /// - Number of spatters spawned
    /// - Size of spatters
    /// - Alpha/opacity of blood
    pub intensity: f32,

    /// Whether to show gash wounds on damaged entities.
    ///
    /// When enabled, wounds appear on entities when HP drops below 50%.
    pub show_wounds: bool,

    /// HP percentage threshold below which wounds become visible.
    ///
    /// Default is 0.5 (50%). Valid range is 0.0 - 1.0.
    pub wound_visibility_threshold: f32,

    /// Base color tint for blood effects.
    ///
    /// Default is a dark red color.
    pub blood_color: Color,

    /// Minimum size for blood spatters in world units.
    pub min_spatter_size: f32,

    /// Maximum size for blood spatters in world units.
    pub max_spatter_size: f32,

    /// Number of spatter decals to spawn on a killing blow.
    pub spatter_count_on_kill: usize,

    /// Number of spatter decals to spawn on a non-lethal hit.
    pub spatter_count_on_hit: usize,

    /// Maximum radius around death position for spatter placement.
    pub spatter_radius: f32,
}

impl Default for BloodEffectConfig {
    fn default() -> Self {
        Self {
            enable_blood: true,
            max_spatters: 100,
            spatter_lifetime: 30.0,
            intensity: 0.7,
            show_wounds: true,
            wound_visibility_threshold: 0.5,
            blood_color: Color::srgb(0.6, 0.0, 0.0),
            min_spatter_size: 0.3,
            max_spatter_size: 1.5,
            spatter_count_on_kill: 5,
            spatter_count_on_hit: 1,
            spatter_radius: 2.0,
        }
    }
}

impl BloodEffectConfig {
    /// Creates a new config with blood effects disabled.
    pub fn disabled() -> Self {
        Self {
            enable_blood: false,
            ..default()
        }
    }

    /// Creates a new config with minimal/low intensity blood effects.
    pub fn low_intensity() -> Self {
        Self {
            intensity: 0.3,
            spatter_count_on_kill: 2,
            spatter_count_on_hit: 0,
            show_wounds: false,
            ..default()
        }
    }

    /// Creates a new config with high intensity blood effects.
    pub fn high_intensity() -> Self {
        Self {
            intensity: 1.0,
            spatter_count_on_kill: 8,
            spatter_count_on_hit: 2,
            max_spatters: 200,
            show_wounds: true,
            ..default()
        }
    }

    /// Returns the effective spatter count for a killing blow, adjusted by intensity.
    pub fn effective_kill_spatter_count(&self) -> usize {
        ((self.spatter_count_on_kill as f32 * self.intensity).ceil() as usize).max(1)
    }

    /// Returns the effective spatter count for a non-lethal hit, adjusted by intensity.
    pub fn effective_hit_spatter_count(&self) -> usize {
        (self.spatter_count_on_hit as f32 * self.intensity).ceil() as usize
    }

    /// Returns the effective spatter size range, adjusted by intensity.
    pub fn effective_spatter_size_range(&self) -> (f32, f32) {
        let min = self.min_spatter_size * self.intensity;
        let max = self.max_spatter_size * self.intensity;
        (min.max(0.1), max.max(min))
    }
}
