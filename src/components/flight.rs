use bevy::prelude::*;

/// Represents the current flight state of a character
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct FlightState {
    /// Whether the character is currently in flying mode
    pub is_flying: bool,
    /// Whether the character is actively thrusting forward (Space bar held)
    pub is_thrusting: bool,
    /// Current flight speed
    pub current_speed: f32,
    /// Last flight direction (normalized) - used for momentum when stopping thrust
    pub last_flight_direction: Vec3,
    /// Entity ID of the left wing
    pub wing_entity_left: Option<Entity>,
    /// Entity ID of the right wing
    pub wing_entity_right: Option<Entity>,
    /// Entity ID of the wind effect emitter
    pub wind_emitter_entity: Option<Entity>,
}
