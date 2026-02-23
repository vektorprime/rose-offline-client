use bevy::prelude::*;
use crate::components::Season;

/// Global season settings
#[derive(Resource, Debug, Clone, Reflect)]
pub struct SeasonSettings {
    pub enabled: bool,
    pub current_season: Season,
    pub max_particles: usize,
    pub spawn_rate: f32, // particles per second
    pub wind_strength: f32,
    pub wind_direction: Vec2,
}

impl Default for SeasonSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            current_season: Season::None,
            max_particles: 2000,   // Maximum particles for season weather effects
            spawn_rate: 100.0,     // Particles per second
            wind_strength: 1.0,
            wind_direction: Vec2::X,
        }
    }
}

/// Fall-specific settings
#[derive(Resource, Debug, Clone, Reflect)]
pub struct FallSettings {
    pub leaf_colors: Vec<Color>,
    pub fall_speed: f32,
    pub drift_factor: f32,
    pub wobble_frequency: f32,
    pub leaf_size_range: (f32, f32),
    pub lifetime_range: (f32, f32),
}

impl Default for FallSettings {
    fn default() -> Self {
        Self {
            leaf_colors: vec![
                Color::srgb(0.8, 0.3, 0.1),  // Orange-red
                Color::srgb(0.9, 0.5, 0.0),  // Orange
                Color::srgb(0.8, 0.6, 0.1),  // Gold
                Color::srgb(0.6, 0.2, 0.0),  // Brown
            ],
            fall_speed: 2.0,
            drift_factor: 1.5,
            wobble_frequency: 2.0,
            leaf_size_range: (0.5, 1.5), // Increased from (0.3, 0.8) for visibility
            lifetime_range: (8.0, 15.0),
        }
    }
}

/// Spring-specific settings
#[derive(Resource, Debug, Clone, Reflect)]
pub struct SpringSettings {
    pub rain_drop_size: f32,
    pub rain_speed: f32,
    pub rain_color: Color,
    pub flower_spawn_chance: f32,
    pub flower_lifetime: f32,
    pub flower_colors: Vec<Color>,
}

impl Default for SpringSettings {
    fn default() -> Self {
        Self {
            rain_drop_size: 0.5, // Increased from 0.2 for better visibility
            rain_speed: 15.0,
            rain_color: Color::srgba(0.6, 0.75, 0.9, 0.8), // More opaque and visible
            flower_spawn_chance: 0.01,
            flower_lifetime: 30.0,
            flower_colors: vec![
                Color::srgb(1.0, 0.5, 0.8),  // Pink
                Color::srgb(0.8, 0.5, 1.0),  // Purple
                Color::srgb(1.0, 1.0, 0.5),  // Yellow
                Color::srgb(1.0, 1.0, 1.0),  // White
            ],
        }
    }
}

/// Winter-specific settings
#[derive(Resource, Debug, Clone, Reflect)]
pub struct WinterSettings {
    pub snowflake_size_range: (f32, f32),
    pub fall_speed: f32,
    pub turbulence: f32,
    pub snow_color: Color,
    pub lifetime_range: (f32, f32),
}

impl Default for WinterSettings {
    fn default() -> Self {
        Self {
            snowflake_size_range: (0.2, 0.6), // Increased from (0.05, 0.15) for visibility
            fall_speed: 1.0,
            turbulence: 0.5,
            snow_color: Color::srgba(1.0, 1.0, 1.0, 0.9),
            lifetime_range: (10.0, 20.0),
        }
    }
}

/// Summer-specific settings
#[derive(Resource, Debug, Clone, Reflect)]
pub struct SummerSettings {
    /// Maximum number of grass blades to spawn
    pub max_grass_blades: usize,
    /// Maximum number of flowers to spawn
    pub max_flowers: usize,
    /// Spawn radius around player for vegetation
    pub spawn_radius: f32,
    /// Grass blade height range
    pub grass_height_range: (f32, f32),
    /// Grass blade width
    pub grass_width: f32,
    /// Grass sway speed
    pub grass_sway_speed: f32,
    /// Grass sway amplitude (radians)
    pub grass_sway_amplitude: f32,
    /// Flower spawn chance per frame
    pub flower_spawn_chance: f32,
    /// Flower colors
    pub flower_colors: Vec<Color>,
    /// Flower stem height range
    pub flower_stem_height_range: (f32, f32),
    /// Flower head size
    pub flower_head_size: f32,
    /// Wind intensity for vegetation sway
    pub wind_intensity: f32,
}

impl Default for SummerSettings {
    fn default() -> Self {
        Self {
            max_grass_blades: 50000,    // Increased to 50000
            max_flowers: 1000,          // Increased 10x from 100 for visibility
            spawn_radius: 50.0,
            grass_height_range: (0.6, 1.6),  // 2x original (0.3, 0.8)
            grass_width: 0.2,           // 2x original (0.1)
            grass_sway_speed: 1.5,
            grass_sway_amplitude: 0.1,
            flower_spawn_chance: 0.02,
            flower_colors: vec![
                Color::srgb(1.0, 0.9, 0.3),  // Yellow (sunflower-like)
                Color::srgb(1.0, 0.5, 0.2),  // Orange
                Color::srgb(0.9, 0.3, 0.3),  // Red
                Color::srgb(0.8, 0.4, 0.8),  // Purple
                Color::srgb(0.3, 0.6, 1.0),  // Blue
            ],
            flower_stem_height_range: (4.0, 7.0),  // Increased 10x from (0.4, 0.7) for visibility
            flower_head_size: 1.5,       // Increased 10x from 0.15 for visibility
            wind_intensity: 1.0,
        }
    }
}
