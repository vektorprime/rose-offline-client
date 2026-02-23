use bevy::prelude::*;

/// Resource holding global flight system settings
#[derive(Resource, Reflect, Clone)]
#[reflect(Resource)]
pub struct FlightSettings {
    /// Maximum flight speed
    pub max_speed: f32,
    /// Acceleration when starting to fly
    pub acceleration: f32,
    /// Deceleration when stopping
    pub deceleration: f32,
    /// Speed of wing flapping animation
    pub wing_flap_speed: f32,
    /// Duration for wings to fully spread (seconds)
    pub wing_spread_duration: f32,
    /// Wind particle spawn rate (particles per second)
    pub wind_particle_spawn_rate: f32,
}

impl Default for FlightSettings {
    fn default() -> Self {
        Self {
            max_speed: 15.0,
            acceleration: 8.0,
            deceleration: 5.0,
            wing_flap_speed: 3.0,
            wing_spread_duration: 0.5,
            wind_particle_spawn_rate: 30.0,
        }
    }
}
