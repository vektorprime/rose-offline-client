//! Fish component for swimming fish in water
//!
//! Fish swim around in water areas using simple AI behavior.
//! They spawn when water is created and stay within water bounds.

use bevy::prelude::*;
use bevy::reflect::Reflect;

/// Component attached to fish entities for swimming behavior
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct Fish {
    /// Swimming speed in units per second
    pub speed: f32,
    /// How fast the fish can turn (radians per second)
    pub turn_speed: f32,
    /// Target position the fish is swimming towards
    pub target_position: Vec3,
    /// How deep in the water (negative Y from water surface)
    pub depth: f32,
    /// School ID for grouping fish together (optional schooling behavior)
    pub school_id: u32,
    /// Center position of the water area this fish belongs to
    pub water_center: Vec3,
    /// Half-extents of the water area (for boundary checking)
    pub water_half_extents: Vec2,
    /// Time accumulator for swimming animation wobble
    pub wobble_time: f32,
}

impl Default for Fish {
    fn default() -> Self {
        Self {
            speed: 2.0,
            turn_speed: 3.0,
            target_position: Vec3::ZERO,
            depth: -1.0,
            school_id: 0,
            water_center: Vec3::ZERO,
            water_half_extents: Vec2::splat(10.0),
            wobble_time: 0.0,
        }
    }
}

/// Resource for fish spawning settings
///
/// Fish type variety (colors, sizes, speeds, schooling) is defined in
/// `src/systems/fish_system.rs` (`FishType` + `default_fish_type_distribution`);
/// all existing fields here drive the shared spawning parameters.
///
/// Fish count scales with the water plane area so small ponds get a few fish
/// while large lakes get more, instead of every plane receiving the same
/// fixed count (which over-packed small planes into static clumps).
#[derive(Resource, Reflect, Debug, Clone)]
#[reflect(Resource)]
pub struct FishSettings {
    /// Fish density: number of fish per 1000 square meters of water surface.
    /// Set to 0 to disable fish entirely.
    pub fish_per_1000_sqm: f32,
    /// Minimum fish to spawn per water plane (when density is non-zero)
    pub min_fish_per_water: usize,
    /// Maximum fish to spawn per water plane
    pub max_fish_per_water: usize,
    /// Minimum depth below water surface
    pub min_depth: f32,
    /// Maximum depth below water surface
    pub max_depth: f32,
    /// Minimum swimming speed
    pub min_speed: f32,
    /// Maximum swimming speed
    pub max_speed: f32,
    /// Minimum distance to target before picking new target
    pub target_reach_distance: f32,
    /// How far fish can swim from water center (as fraction of water size)
    pub boundary_margin: f32,
    /// Fish farther than this from the camera are not simulated (meters)
    pub simulation_distance: f32,
}

impl Default for FishSettings {
    fn default() -> Self {
        Self {
            fish_per_1000_sqm: 50.0,
            min_fish_per_water: 8,
            max_fish_per_water: 150,
            min_depth: 0.5,
            max_depth: 3.0,
            min_speed: 0.5,
            max_speed: 2.0,
            target_reach_distance: 0.4,
            boundary_margin: 0.8, // Stay 80% within water bounds
            simulation_distance: 80.0,
        }
    }
}

/// Marker component for fish mesh entities (children of fish entities)
#[derive(Component, Debug, Clone, Copy)]
pub struct FishMesh;

/// Component to track which water entity a fish belongs to
#[derive(Component, Debug, Clone, Copy)]
pub struct FishWaterRef {
    pub water_entity: Entity,
}

/// Message sent when water is spawned, triggering fish spawning
#[derive(Message, Debug, Clone)]
pub struct WaterSpawnedEvent {
    pub water_entity: Entity,
    pub zone_entity: Entity,
    pub water_center: Vec3,
    pub water_half_extents: Vec2,
}
