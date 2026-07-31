//! Procedural noise overlay for terrain enhancement.
//!
//! This module provides multi-octave Perlin/Simplex noise to add natural rolling
//! hills and terrain variation while preserving the original terrain's general shape.

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use std::cell::RefCell;

/// Resource for configuring terrain enhancement settings.
/// Controls how procedural noise is applied to terrain height.
#[derive(Resource, Debug, Clone)]
pub struct TerrainEnhancementSettings {
    /// Whether noise overlay is enabled
    pub noise_enabled: bool,
    /// Frequency/scale of the noise (lower = larger features)
    pub noise_scale: f32,
    /// Maximum height change from noise in world units
    pub noise_amplitude: f32,
    /// Number of noise layers (octaves) for detail
    pub noise_octaves: usize,
    /// How much each octave contributes (0.0-1.0)
    pub noise_persistence: f32,
    /// Random seed for noise generation
    pub noise_seed: u32,

    // Blend Zone Settings
    /// Whether to reduce noise near important game objects
    pub blend_near_objects: bool,
    /// Distance from important objects where blending starts (world units)
    pub blend_distance: f32,
    /// Power for the blend falloff curve (higher = sharper transition)
    /// 1.0 = linear, 2.0 = quadratic (exponential), 3.0 = cubic
    pub blend_curve_power: f32,

    // Elevation-based zones
    /// Elevation threshold below which terrain is considered "valley" (world units)
    /// Below this height, noise is reduced for smoother terrain
    pub elevation_zone_low: f32,
    /// Elevation threshold above which terrain is considered "mountain" (world units)
    /// Above this height, noise is increased for rougher terrain
    pub elevation_zone_high: f32,
    /// Noise multiplier for valley areas (below elevation_zone_low)
    /// Lower values = smoother terrain in valleys (default: 0.3)
    pub valley_noise_multiplier: f32,
    /// Noise multiplier for mountain areas (above elevation_zone_high)
    /// Higher values = rougher terrain at elevation (default: 1.5)
    pub mountain_noise_multiplier: f32,
    /// Smoothness of the transition between elevation zones (0.0-1.0)
    /// Higher values = wider transition zone, lower = sharper transition
    pub elevation_transition_smoothness: f32,
}

impl Default for TerrainEnhancementSettings {
    fn default() -> Self {
        Self {
            noise_enabled: false,
            noise_scale: 0.008,     // Low frequency for large rolling hills
            noise_amplitude: 2.0,   // Subtle height variation
            noise_octaves: 4,       // Multiple detail layers
            noise_persistence: 0.5, // Standard roughness
            noise_seed: 42,         // Consistent seed for reproducibility

            // Blend zone defaults
            blend_near_objects: true,
            blend_distance: 20.0,   // 20 world units radius for flat zones
            blend_curve_power: 2.0, // Quadratic falloff (smooth)

            // Elevation-based zone defaults
            elevation_zone_low: 5.0,              // Below 5 units = valley
            elevation_zone_high: 30.0,            // Above 30 units = mountain
            valley_noise_multiplier: 0.3,         // Less noise in valleys (smoother)
            mountain_noise_multiplier: 1.5,       // More noise at elevation (rougher)
            elevation_transition_smoothness: 0.5, // Smooth transition between zones
        }
    }
}

/// Internal noise generator that caches the Perlin noise instance.
/// This is created once and reused for all noise queries.
pub struct TerrainNoiseGenerator {
    noise: Perlin,
    settings: TerrainEnhancementSettings,
}

impl TerrainNoiseGenerator {
    /// Create a new noise generator with the given settings
    pub fn new(settings: &TerrainEnhancementSettings) -> Self {
        let noise = Perlin::new(settings.noise_seed);
        Self {
            noise,
            settings: settings.clone(),
        }
    }

    /// Generate fractal Brownian motion (fBm) noise at the given world coordinates.
    /// This combines multiple octaves of Perlin noise for natural-looking terrain.
    pub fn get_noise(&self, world_x: f32, world_z: f32) -> f32 {
        if !self.settings.noise_enabled {
            return 0.0;
        }

        let scale = self.settings.noise_scale as f64;
        let persistence = self.settings.noise_persistence as f64;

        // Sample noise at multiple octaves
        let mut total = 0.0f64;
        let mut amplitude = 1.0f64;
        let mut frequency = scale;
        let mut max_value = 0.0f64;

        for _ in 0..self.settings.noise_octaves {
            // Use 2D noise with x and z coordinates
            let noise_value = self
                .noise
                .get([world_x as f64 * frequency, world_z as f64 * frequency]);
            total += noise_value * amplitude;
            max_value += amplitude;

            amplitude *= persistence;
            frequency *= 2.0;
        }

        // Normalize to -1 to 1 range, then scale by amplitude
        let normalized = if max_value > 0.0 {
            (total / max_value) as f32
        } else {
            0.0
        };

        normalized * self.settings.noise_amplitude
    }
}

// Thread-local storage for the noise generator.
// This allows `get_terrain_height` to access the noise without needing
// direct access to the resource system.
thread_local! {
    static TERRAIN_NOISE: RefCell<Option<TerrainNoiseGenerator>> = RefCell::new(None);
}

/// Initialize the thread-local noise generator with the given settings.
/// This should be called when the zone is loaded.
pub fn init_thread_local_noise(settings: &TerrainEnhancementSettings) {
    TERRAIN_NOISE.with(|cell| {
        *cell.borrow_mut() = Some(TerrainNoiseGenerator::new(settings));
    });
}

/// Get the noise offset for a world position using the thread-local generator.
/// Returns 0.0 if the generator hasn't been initialized.
pub fn get_thread_local_noise(world_x: f32, world_z: f32) -> f32 {
    TERRAIN_NOISE.with(|cell| {
        if let Some(ref generator) = *cell.borrow() {
            generator.get_noise(world_x, world_z)
        } else {
            0.0
        }
    })
}

/// Global terrain noise generator resource.
/// This is stored as a resource to avoid recreating the noise generator each frame.
#[derive(Resource)]
pub struct GlobalTerrainNoise {
    generator: TerrainNoiseGenerator,
}

impl GlobalTerrainNoise {
    pub fn new(settings: &TerrainEnhancementSettings) -> Self {
        // Also initialize the thread-local version
        init_thread_local_noise(settings);

        Self {
            generator: TerrainNoiseGenerator::new(settings),
        }
    }

    /// Get the noise value at world coordinates
    pub fn get_noise(&self, world_x: f32, world_z: f32) -> f32 {
        self.generator.get_noise(world_x, world_z)
    }
}

/// Plugin that sets up terrain enhancement resources
pub struct TerrainEnhancementPlugin;

impl Plugin for TerrainEnhancementPlugin {
    fn build(&self, app: &mut App) {
        // Initialize with default settings
        let settings = TerrainEnhancementSettings::default();
        let noise = GlobalTerrainNoise::new(&settings);

        app.insert_resource(settings).insert_resource(noise);
    }
}
