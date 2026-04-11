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

    /// Dried blood tint used for wet-to-dry evolution.
    pub dry_blood_color: Color,

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

    /// Global quality scalar for blood visuals (0.0 - 1.0).
    pub quality_scale: f32,

    /// Maximum number of spatter decals spawned in a single frame.
    pub max_spatters_per_frame: usize,

    /// Fraction of lifetime at which alpha fade begins (0.0 - 1.0).
    pub fade_start_fraction: f32,

    /// Depth fade factor for forward decal blending.
    pub decal_depth_fade_factor: f32,

    /// Minimum wound overlay size in local units.
    pub wound_min_size: f32,

    /// Maximum wound overlay size in local units.
    pub wound_max_size: f32,

    /// Maximum wound overlays to show on a single entity.
    pub max_wounds_per_entity: usize,

    /// Distance from the player where full blood quality is used.
    pub lod_near_distance: f32,

    /// Distance from the player where blood is strongly reduced.
    pub lod_far_distance: f32,

    /// Enable layered blood rendering (mist + droplets + decals).
    pub enable_layered_effects: bool,

    /// Enable lightweight diagnostics logging counters.
    pub enable_diagnostics: bool,
}

impl Default for BloodEffectConfig {
    fn default() -> Self {
        Self {
            enable_blood: true,
            max_spatters: 100,
            spatter_lifetime: 30.0,
            intensity: 1.5,
            show_wounds: true,
            wound_visibility_threshold: 0.5,
            blood_color: Color::srgb(0.6, 0.0, 0.0),
            dry_blood_color: Color::srgb(0.28, 0.06, 0.04),
            min_spatter_size: 0.3,
            max_spatter_size: 1.5,
            spatter_count_on_kill: 5,
            spatter_count_on_hit: 1,
            spatter_radius: 2.0,
            quality_scale: 1.0,
            max_spatters_per_frame: 24,
            fade_start_fraction: 0.7,
            decal_depth_fade_factor: 0.65,
            wound_min_size: 0.12,
            wound_max_size: 0.28,
            max_wounds_per_entity: 4,
            lod_near_distance: 40.0,
            lod_far_distance: 140.0,
            enable_layered_effects: true,
            enable_diagnostics: false,
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
            quality_scale: 0.5,
            max_spatters_per_frame: 8,
            enable_layered_effects: false,
            enable_diagnostics: false,
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
            quality_scale: 1.0,
            max_spatters_per_frame: 48,
            max_wounds_per_entity: 6,
            enable_layered_effects: true,
            enable_diagnostics: true,
            ..default()
        }
    }

    /// Returns a clamped quality multiplier.
    pub fn effective_quality_scale(&self) -> f32 {
        self.quality_scale.clamp(0.1, 1.0)
    }

    /// LOD scale by distance from the camera/player.
    pub fn distance_lod_scale(&self, distance: f32) -> f32 {
        let near = self.lod_near_distance.max(1.0);
        let far = self.lod_far_distance.max(near + 1.0);
        if distance <= near {
            1.0
        } else if distance >= far {
            0.2
        } else {
            let t = (distance - near) / (far - near);
            (1.0 - t * 0.8).clamp(0.2, 1.0)
        }
    }

    /// Returns the effective spatter count for a killing blow, adjusted by intensity.
    pub fn effective_kill_spatter_count(&self) -> usize {
        ((self.spatter_count_on_kill as f32 * self.intensity * self.effective_quality_scale())
            .ceil() as usize)
            .max(1)
    }

    /// Returns the effective spatter count for a non-lethal hit, adjusted by intensity.
    pub fn effective_hit_spatter_count(&self) -> usize {
        (self.spatter_count_on_hit as f32 * self.intensity * self.effective_quality_scale()).ceil()
            as usize
    }

    /// Returns the per-frame spawn budget for blood spatters.
    pub fn effective_spatter_spawn_budget(&self) -> usize {
        ((self.max_spatters_per_frame as f32) * self.effective_quality_scale()).round() as usize
    }

    /// Returns the effective spatter size range, adjusted by intensity.
    pub fn effective_spatter_size_range(&self) -> (f32, f32) {
        let min = self.min_spatter_size * self.intensity * self.effective_quality_scale();
        let max = self.max_spatter_size * self.intensity * self.effective_quality_scale();
        (min.max(0.1), max.max(min))
    }
}
