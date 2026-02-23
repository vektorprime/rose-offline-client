use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Marker component for bird entities
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct Bird {
    /// Movement speed
    pub speed: f32,
    /// Current target position to fly towards
    pub target_position: Vec3,
    /// Center of roam area
    pub roam_center: Vec3,
    /// Radius of roam area
    pub roam_radius: f32,
    /// Wing flap animation phase (0 to 2π)
    pub flap_phase: f32,
    /// Vertical bobbing phase
    pub bob_phase: f32,
}

impl Default for Bird {
    fn default() -> Self {
        Self {
            speed: 5.0,
            target_position: Vec3::ZERO,
            roam_center: Vec3::ZERO,
            roam_radius: 100.0,
            flap_phase: 0.0,
            bob_phase: 0.0,
        }
    }
}

/// Resource for bird configuration settings
#[derive(Resource, Reflect, Clone, Debug, Serialize, Deserialize)]
#[reflect(Resource, Default, Serialize, Deserialize)]
pub struct BirdSettings {
    /// Whether birds are enabled
    pub enabled: bool,
    /// Number of birds per zone
    pub birds_per_zone: usize,
    /// Minimum flying altitude
    pub min_altitude: f32,
    /// Maximum flying altitude
    pub max_altitude: f32,
    /// Minimum movement speed
    pub min_speed: f32,
    /// Maximum movement speed
    pub max_speed: f32,
    /// Roam radius from spawn point
    pub roam_radius: f32,
    /// Wing flap speed (radians per second)
    pub flap_speed: f32,
    /// Vertical bobbing amplitude
    pub bob_amplitude: f32,
    /// Vertical bobbing speed
    pub bob_speed: f32,
}

impl Default for BirdSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            birds_per_zone: 400,       // Default 400 birds per zone
            min_altitude: 20.0,       // Lower altitude for better visibility
            max_altitude: 50.0,       // Lower max altitude
            min_speed: 3.0,
            max_speed: 8.0,
            roam_radius: 200.0,       // Smaller roam radius so birds are more concentrated
            flap_speed: 12.0,
            bob_amplitude: 0.5,
            bob_speed: 2.0,
        }
    }
}

/// Marker component for the bird mesh child entity
#[derive(Component)]
pub struct BirdMesh;
