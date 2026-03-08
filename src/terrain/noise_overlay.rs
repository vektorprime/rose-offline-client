//! Procedural noise overlay for terrain enhancement.
//!
//! This module provides multi-octave Perlin/Simplex noise to add natural rolling
//! hills and terrain variation while preserving the original terrain's general shape.
//!
//! # Blend Zones
//!
//! Blend zones ensure smooth transitions between flat areas (near important game objects)
//! and enhanced terrain features. The blend factor reduces noise amplitude near:
//! - Spawn points
//! - NPCs
//! - Buildings/structures
//! - Any position marked as "important"
//!
//! The blending uses exponential falloff for natural-looking transitions.

use bevy::prelude::*;
use noise::{Perlin, NoiseFn, Seedable};
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
            noise_scale: 0.008,       // Low frequency for large rolling hills
            noise_amplitude: 2.0,     // Subtle height variation
            noise_octaves: 4,         // Multiple detail layers
            noise_persistence: 0.5,   // Standard roughness
            noise_seed: 42,           // Consistent seed for reproducibility
            
            // Blend zone defaults
            blend_near_objects: true,
            blend_distance: 20.0,     // 20 world units radius for flat zones
            blend_curve_power: 2.0,   // Quadratic falloff (smooth)
            
            // Elevation-based zone defaults
            elevation_zone_low: 5.0,  // Below 5 units = valley
            elevation_zone_high: 30.0, // Above 30 units = mountain
            valley_noise_multiplier: 0.3,  // Less noise in valleys (smoother)
            mountain_noise_multiplier: 1.5, // More noise at elevation (rougher)
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
            let noise_value = self.noise.get([world_x as f64 * frequency, world_z as f64 * frequency]);
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

/// Thread-local storage for the noise generator.
/// This allows `get_terrain_height` to access the noise without needing
/// direct access to the resource system.
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

    /// Update the generator with new settings
    pub fn update_settings(&mut self, settings: &TerrainEnhancementSettings) {
        init_thread_local_noise(settings);
        self.generator = TerrainNoiseGenerator::new(settings);
    }
}

/// Apply procedural noise to a base height value.
///
/// # Arguments
/// * `base_height` - The original terrain height from the heightmap
/// * `world_x` - World X coordinate for noise sampling
/// * `world_z` - World Z coordinate for noise sampling
/// * `noise_generator` - The global noise generator resource
///
/// # Returns
/// The modified height with noise applied
pub fn apply_noise_to_height(
    base_height: f32,
    world_x: f32,
    world_z: f32,
    noise_generator: &GlobalTerrainNoise,
) -> f32 {
    let noise_offset = noise_generator.get_noise(world_x, world_z);
    base_height + noise_offset
}

/// Apply procedural noise to a base height value with elevation-based zones.
/// This creates smoother valleys and rougher mountain terrain.
///
/// # Arguments
/// * `base_height` - The original terrain height from the heightmap
/// * `world_x` - World X coordinate for noise sampling
/// * `world_z` - World Z coordinate for noise sampling
/// * `noise_generator` - The global noise generator resource
/// * `settings` - Terrain enhancement settings with elevation zone configuration
///
/// # Returns
/// The modified height with noise applied, adjusted based on elevation
pub fn apply_noise_to_height_with_elevation(
    base_height: f32,
    world_x: f32,
    world_z: f32,
    noise_generator: &GlobalTerrainNoise,
    settings: &TerrainEnhancementSettings,
) -> f32 {
    let noise_offset = noise_generator.get_noise(world_x, world_z);
    
    // Calculate elevation-based multiplier
    let elevation_multiplier = calculate_elevation_multiplier(
        base_height,
        settings.elevation_zone_low,
        settings.elevation_zone_high,
        settings.valley_noise_multiplier,
        settings.mountain_noise_multiplier,
        settings.elevation_transition_smoothness,
    );
    
    // Apply noise with elevation multiplier
    base_height + (noise_offset * elevation_multiplier)
}

/// Apply procedural noise to a base height value with blend zone support.
///
/// # Arguments
/// * `base_height` - The original terrain height from the heightmap
/// * `world_x` - World X coordinate for noise sampling
/// * `world_z` - World Z coordinate for noise sampling
/// * `noise_generator` - The global noise generator resource
/// * `important_positions` - Positions of important game objects for blending
/// * `settings` - Terrain enhancement settings for blend configuration
///
/// # Returns
/// The modified height with noise applied, blended based on proximity to important objects
pub fn apply_noise_to_height_with_blend(
    base_height: f32,
    world_x: f32,
    world_z: f32,
    noise_generator: &GlobalTerrainNoise,
    important_positions: &ImportantPositions,
    settings: &TerrainEnhancementSettings,
) -> f32 {
    let noise_offset = noise_generator.get_noise(world_x, world_z);
    
    // Calculate blend factor (0.0 near objects = flat, 1.0 far from objects = full noise)
    let blend_factor = if settings.blend_near_objects {
        calculate_blend_factor(
            world_x,
            world_z,
            &important_positions.positions,
            settings.blend_distance,
            settings.blend_curve_power,
        )
    } else {
        1.0 // No blending - full noise everywhere
    };
    
    // Calculate elevation-based multiplier for varied terrain
    // Uses BASE height (before noise) to determine zone
    let elevation_multiplier = calculate_elevation_multiplier(
        base_height,
        settings.elevation_zone_low,
        settings.elevation_zone_high,
        settings.valley_noise_multiplier,
        settings.mountain_noise_multiplier,
        settings.elevation_transition_smoothness,
    );
    
    // Apply noise with blend factor and elevation multiplier
    // Final noise = base_noise * blend_factor * elevation_multiplier
    base_height + (noise_offset * blend_factor * elevation_multiplier)
}

/// Get just the noise offset for a world position.
/// Useful when you need to know how much noise contributes without the base height.
pub fn get_terrain_noise(
    world_x: f32,
    world_z: f32,
    noise_generator: &GlobalTerrainNoise,
) -> f32 {
    noise_generator.get_noise(world_x, world_z)
}

// =============================================================================
// Blend Zone Implementation
// =============================================================================

/// Resource storing positions of important game objects that should have flatter terrain nearby.
///
/// This resource is populated during zone loading with positions of:
/// - Spawn points
/// - NPCs
/// - Buildings/structures
/// - Warps/teleporters
/// - Any other gameplay-critical locations
#[derive(Resource, Debug, Clone, Default)]
pub struct ImportantPositions {
    /// List of important world positions (X, Z coordinates)
    pub positions: Vec<Vec2>,
}

impl ImportantPositions {
    /// Create a new empty ImportantPositions resource
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an ImportantPositions resource with pre-filled positions
    pub fn with_positions(positions: Vec<Vec2>) -> Self {
        Self { positions }
    }

    /// Add a single important position
    pub fn add_position(&mut self, position: Vec2) {
        self.positions.push(position);
    }

    /// Add multiple important positions
    pub fn add_positions(&mut self, positions: &[Vec2]) {
        self.positions.extend(positions.iter().copied());
    }

    /// Clear all important positions (call when loading a new zone)
    pub fn clear(&mut self) {
        self.positions.clear();
    }

    /// Get the nearest important position and its distance squared
    pub fn nearest_position(&self, world_x: f32, world_z: f32) -> Option<(Vec2, f32)> {
        let query_pos = Vec2::new(world_x, world_z);
        self.positions
            .iter()
            .map(|&pos| {
                let dist_sq = pos.distance_squared(query_pos);
                (pos, dist_sq)
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    }
}

/// Calculate the blend factor for terrain noise based on proximity to important objects.
///
/// # Arguments
/// * `world_x` - World X coordinate to check
/// * `world_z` - World Z coordinate to check
/// * `object_positions` - Positions of nearby important objects
/// * `blend_distance` - Distance at which full noise starts (world units)
/// * `blend_curve_power` - Power for the falloff curve (1.0 = linear, 2.0 = quadratic)
///
/// # Returns
/// A blend factor from 0.0 (no noise, flat terrain) to 1.0 (full noise):
/// - 0.0 when very close to important objects (flat terrain for gameplay)
/// - 1.0 when far from all important objects (full natural terrain)
/// - Smooth transition in between
pub fn calculate_blend_factor(
    world_x: f32,
    world_z: f32,
    object_positions: &[Vec2],
    blend_distance: f32,
    blend_curve_power: f32,
) -> f32 {
    if object_positions.is_empty() {
        // No important objects - full noise everywhere
        return 1.0;
    }

    let query_pos = Vec2::new(world_x, world_z);
    let blend_distance_sq = blend_distance * blend_distance;

    // Find the minimum distance to any important object
    let min_dist_sq = object_positions
        .iter()
        .map(|&pos| pos.distance_squared(query_pos))
        .fold(f32::MAX, |a, b| a.min(b));

    // If we're at or inside an important object position, no noise
    if min_dist_sq <= 0.001 {
        return 0.0;
    }

    // If we're beyond the blend distance, full noise
    if min_dist_sq >= blend_distance_sq {
        return 1.0;
    }

    // Calculate normalized distance (0.0 at object, 1.0 at blend_distance)
    let min_dist = min_dist_sq.sqrt();
    let normalized_distance = min_dist / blend_distance;

    // Apply exponential curve for smoother falloff
    // Using powf with the curve power for adjustable falloff
    normalized_distance.powf(blend_curve_power)
}

/// Smoothstep function for even smoother transitions.
/// Returns a smooth interpolation between 0 and 1 based on edge distances.
///
/// # Arguments
/// * `x` - Input value (typically normalized distance)
/// * `edge0` - Lower edge of the transition
/// * `edge1` - Upper edge of the transition
///
/// # Returns
/// Smoothly interpolated value between 0 and 1
#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    // Clamp x to [0, 1] range based on edges
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    // Smoothstep formula: 3t² - 2t³
    t * t * (3.0 - 2.0 * t)
}

/// Smootherstep function (Ken Perlin's improved version).
/// Provides even smoother transitions than smoothstep.
///
/// # Arguments
/// * `x` - Input value (typically normalized distance)
/// * `edge0` - Lower edge of the transition
/// * `edge1` - Upper edge of the transition
///
/// # Returns
/// Very smoothly interpolated value between 0 and 1
#[inline]
pub fn smootherstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    // Smootherstep formula: 6t⁵ - 15t⁴ + 10t³
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Calculate blend factor using smoothstep for ultra-smooth transitions.
/// This is an alternative to `calculate_blend_factor` that uses smoothstep instead of power curves.
///
/// # Arguments
/// * `world_x` - World X coordinate to check
/// * `world_z` - World Z coordinate to check
/// * `object_positions` - Positions of nearby important objects
/// * `blend_distance` - Distance at which full noise starts
/// * `inner_distance` - Distance inside which terrain is completely flat (default 0.0)
///
/// # Returns
/// A blend factor from 0.0 to 1.0 with smoothstep interpolation
pub fn calculate_blend_factor_smoothstep(
    world_x: f32,
    world_z: f32,
    object_positions: &[Vec2],
    blend_distance: f32,
    inner_distance: f32,
) -> f32 {
    if object_positions.is_empty() {
        return 1.0;
    }

    let query_pos = Vec2::new(world_x, world_z);

    // Find minimum distance to any important object
    let min_dist = object_positions
        .iter()
        .map(|&pos| pos.distance(query_pos))
        .fold(f32::MAX, |a, b| a.min(b));

    // Use smoothstep for smooth transition
    smoothstep(inner_distance, blend_distance, min_dist)
}

/// Calculate blend factor using smootherstep for ultra-smooth transitions.
///
/// # Arguments
/// * `world_x` - World X coordinate to check
/// * `world_z` - World Z coordinate to check
/// * `object_positions` - Positions of nearby important objects
/// * `blend_distance` - Distance at which full noise starts
/// * `inner_distance` - Distance inside which terrain is completely flat (default 0.0)
///
/// # Returns
/// A blend factor from 0.0 to 1.0 with smootherstep interpolation
pub fn calculate_blend_factor_smootherstep(
    world_x: f32,
    world_z: f32,
    object_positions: &[Vec2],
    blend_distance: f32,
    inner_distance: f32,
) -> f32 {
    if object_positions.is_empty() {
        return 1.0;
    }

    let query_pos = Vec2::new(world_x, world_z);

    // Find minimum distance to any important object
    let min_dist = object_positions
        .iter()
        .map(|&pos| pos.distance(query_pos))
        .fold(f32::MAX, |a, b| a.min(b));

    // Use smootherstep for ultra-smooth transition
    smootherstep(inner_distance, blend_distance, min_dist)
}

/// Height-based blend factor calculation.
/// Reduces noise at lower elevations and increases it at higher elevations.
/// This creates flatter valleys and more varied highlands.
///
/// # Arguments
/// * `height` - Current terrain height
/// * `min_height` - Minimum expected terrain height
/// * `max_height` - Maximum expected terrain height
/// * `blend_curve_power` - Power for the curve (higher = more pronounced effect)
///
/// # Returns
/// A blend factor from 0.0 (low terrain, less noise) to 1.0 (high terrain, full noise)
pub fn calculate_height_based_blend(
    height: f32,
    min_height: f32,
    max_height: f32,
    blend_curve_power: f32,
) -> f32 {
    let height_range = max_height - min_height;
    if height_range <= 0.0 {
        return 1.0;
    }

    // Normalize height to 0-1 range
    let normalized_height = ((height - min_height) / height_range).clamp(0.0, 1.0);

    // Apply power curve for more control
    normalized_height.powf(blend_curve_power)
}

// =============================================================================
// Elevation-Based Zone Implementation
// =============================================================================

/// Calculates noise multiplier based on elevation.
/// Returns a value that modifies noise amplitude based on height, creating
/// more dramatic terrain features at higher elevations and smoother terrain at lower elevations.
///
/// # Arguments
/// * `base_height` - The base terrain height (before noise is applied)
/// * `low_threshold` - Elevation below this is considered "valley" (world units)
/// * `high_threshold` - Elevation above this is considered "mountain" (world units)
/// * `valley_multiplier` - Noise multiplier for valley areas (typically < 1.0)
/// * `mountain_multiplier` - Noise multiplier for mountain areas (typically > 1.0)
/// * `smoothness` - Smoothness of transition between zones (0.0-1.0)
///
/// # Returns
/// A multiplier for the noise amplitude:
/// - `valley_multiplier` when base_height <= low_threshold
/// - `mountain_multiplier` when base_height >= high_threshold
/// - Smoothly interpolated value between the two when in transition zone
///
/// # Example
/// ```
/// let multiplier = calculate_elevation_multiplier(
///     15.0,   // base height
///     5.0,    // low threshold (valley)
///     30.0,   // high threshold (mountain)
///     0.3,    // valley multiplier (less noise)
///     1.5,    // mountain multiplier (more noise)
///     0.5,    // smooth transition
/// );
/// // Returns ~0.78 (between valley and mid-range)
/// ```
pub fn calculate_elevation_multiplier(
    base_height: f32,
    low_threshold: f32,
    high_threshold: f32,
    valley_multiplier: f32,
    mountain_multiplier: f32,
    smoothness: f32,
) -> f32 {
    // Handle edge case where thresholds are the same or inverted
    if high_threshold <= low_threshold {
        // Use average of multipliers if thresholds are invalid
        return (valley_multiplier + mountain_multiplier) * 0.5;
    }
    
    // Below low threshold: valley
    if base_height <= low_threshold {
        return valley_multiplier;
    }
    
    // Above high threshold: mountain
    if base_height >= high_threshold {
        return mountain_multiplier;
    }
    
    // In transition zone: interpolate between valley and mountain
    let normalized = (base_height - low_threshold) / (high_threshold - low_threshold);
    
    // Apply smoothness factor to control transition width
    // smoothness of 0.0 = linear interpolation
    // smoothness of 1.0 = very smooth (smootherstep) interpolation
    let t = if smoothness <= 0.0 {
        normalized
    } else if smoothness >= 1.0 {
        // Use smootherstep for very smooth transitions
        normalized * normalized * normalized * (normalized * (normalized * 6.0 - 15.0) + 10.0)
    } else {
        // Blend between linear and smoothstep based on smoothness parameter
        let smooth_t = normalized * normalized * (3.0 - 2.0 * normalized);
        normalized * (1.0 - smoothness) + smooth_t * smoothness
    };
    
    // Interpolate between valley and mountain multipliers
    valley_multiplier + (mountain_multiplier - valley_multiplier) * t
}

/// Zone-center based blend factor calculation.
/// Creates a flat zone in the center of the map with increasing noise toward edges.
/// This is useful when you don't have specific object positions but want a central flat area.
///
/// # Arguments
/// * `world_x` - World X coordinate
/// * `world_z` - World Z coordinate
/// * `zone_center` - Center of the zone (typically (0, 0) or zone midpoint)
/// * `flat_radius` - Radius of completely flat area around center
/// * `blend_distance` - Distance from flat_radius to full noise
///
/// # Returns
/// A blend factor from 0.0 (center, flat) to 1.0 (edges, full noise)
pub fn calculate_zone_center_blend(
    world_x: f32,
    world_z: f32,
    zone_center: Vec2,
    flat_radius: f32,
    blend_distance: f32,
) -> f32 {
    let query_pos = Vec2::new(world_x, world_z);
    let dist_from_center = query_pos.distance(zone_center);

    if dist_from_center <= flat_radius {
        0.0 // Completely flat in center
    } else if dist_from_center >= flat_radius + blend_distance {
        1.0 // Full noise at edges
    } else {
        // Smooth transition
        let t = (dist_from_center - flat_radius) / blend_distance;
        smoothstep(0.0, 1.0, t)
    }
}

/// Flat zone rectangle for defining areas with reduced noise.
/// Useful for defining specific gameplay areas that should remain flat.
#[derive(Debug, Clone, Copy)]
pub struct FlatZone {
    /// Minimum corner of the rectangle (world coordinates)
    pub min: Vec2,
    /// Maximum corner of the rectangle (world coordinates)
    pub max: Vec2,
    /// Blend distance around the rectangle edges
    pub blend_distance: f32,
}

impl FlatZone {
    /// Create a new flat zone rectangle
    pub fn new(min_x: f32, min_z: f32, max_x: f32, max_z: f32, blend_distance: f32) -> Self {
        Self {
            min: Vec2::new(min_x, min_z),
            max: Vec2::new(max_x, max_z),
            blend_distance,
        }
    }

    /// Create a flat zone from center and size
    pub fn from_center(center_x: f32, center_z: f32, width: f32, depth: f32, blend_distance: f32) -> Self {
        Self {
            min: Vec2::new(center_x - width / 2.0, center_z - depth / 2.0),
            max: Vec2::new(center_x + width / 2.0, center_z + depth / 2.0),
            blend_distance,
        }
    }

    /// Check if a point is inside the flat zone (ignoring blend distance)
    pub fn contains(&self, world_x: f32, world_z: f32) -> bool {
        world_x >= self.min.x && world_x <= self.max.x &&
        world_z >= self.min.y && world_z <= self.max.y
    }

    /// Calculate the blend factor for a position relative to this flat zone
    pub fn blend_factor(&self, world_x: f32, world_z: f32) -> f32 {
        let pos = Vec2::new(world_x, world_z);

        // If inside the rectangle, no noise
        if self.contains(world_x, world_z) {
            return 0.0;
        }

        // Calculate distance to rectangle edge
        let dist_to_edge = self.distance_to_edge(pos);

        if dist_to_edge >= self.blend_distance {
            1.0 // Full noise far from zone
        } else {
            // Smooth transition based on distance
            smoothstep(0.0, self.blend_distance, dist_to_edge)
        }
    }

    /// Calculate the distance from a point to the nearest edge of the rectangle
    fn distance_to_edge(&self, pos: Vec2) -> f32 {
        // Calculate distance to each edge
        let dx_left = (self.min.x - pos.x).max(0.0);
        let dx_right = (pos.x - self.max.x).max(0.0);
        let dz_bottom = (self.min.y - pos.y).max(0.0);
        let dz_top = (pos.y - self.max.y).max(0.0);

        // If outside in both dimensions, use diagonal distance
        if dx_left > 0.0 || dx_right > 0.0 || dz_bottom > 0.0 || dz_top > 0.0 {
            (dx_left.max(dx_right).powi(2) + dz_bottom.max(dz_top).powi(2)).sqrt()
        } else {
            // Inside the rectangle - distance to nearest edge
            let dx = (self.min.x - pos.x).abs().min((pos.x - self.max.x).abs());
            let dz = (self.min.y - pos.y).abs().min((pos.y - self.max.y).abs());
            dx.min(dz)
        }
    }
}

/// Calculate blend factor considering multiple flat zones
pub fn calculate_flat_zones_blend(world_x: f32, world_z: f32, flat_zones: &[FlatZone]) -> f32 {
    if flat_zones.is_empty() {
        return 1.0;
    }

    // Find the minimum blend factor from all zones
    flat_zones
        .iter()
        .map(|zone| zone.blend_factor(world_x, world_z))
        .fold(1.0, |a, b| a.min(b))
}

/// Combined blend factor calculation using both object positions and flat zones.
/// This is the most flexible approach for complex terrain blending.
///
/// # Arguments
/// * `world_x` - World X coordinate
/// * `world_z` - World Z coordinate
/// * `object_positions` - Positions of important game objects
/// * `flat_zones` - Rectangular zones that should remain flat
/// * `blend_distance` - Blend distance for object positions
/// * `blend_curve_power` - Power curve for object position blending
///
/// # Returns
/// Combined blend factor from 0.0 to 1.0
pub fn calculate_combined_blend_factor(
    world_x: f32,
    world_z: f32,
    object_positions: &[Vec2],
    flat_zones: &[FlatZone],
    blend_distance: f32,
    blend_curve_power: f32,
) -> f32 {
    // Calculate blend from object positions
    let object_blend = calculate_blend_factor(
        world_x,
        world_z,
        object_positions,
        blend_distance,
        blend_curve_power,
    );

    // Calculate blend from flat zones
    let zone_blend = calculate_flat_zones_blend(world_x, world_z, flat_zones);

    // Use minimum to ensure both constraints are respected
    object_blend.min(zone_blend)
}

/// Plugin that sets up terrain enhancement resources
pub struct TerrainEnhancementPlugin;

impl Plugin for TerrainEnhancementPlugin {
    fn build(&self, app: &mut App) {
        // Initialize with default settings
        let settings = TerrainEnhancementSettings::default();
        let noise = GlobalTerrainNoise::new(&settings);
        let important_positions = ImportantPositions::new();
        
        app.insert_resource(settings)
            .insert_resource(noise)
            .insert_resource(important_positions);
    }
}
