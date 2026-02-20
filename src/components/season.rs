use bevy::prelude::*;

/// Season types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum Season {
    #[default]
    None,
    Fall,
    Spring,
    Winter,
}

/// Component attached to weather particle entities
#[derive(Component, Debug, Clone, Reflect)]
pub struct WeatherParticle {
    pub age: f32,
    pub lifetime: f32,
    pub velocity: Vec3,
    pub base_size: f32,
    pub rotation: f32,
    pub rotation_speed: f32,
    pub wobble_phase: f32,
    pub wobble_amplitude: f32,
}

/// Marker component for season-specific entities (for cleanup)
#[derive(Component, Debug, Clone, Reflect)]
pub struct SeasonMarker(pub Season);

/// Component for flower entities spawned in spring
#[derive(Component, Debug, Clone, Reflect)]
pub struct SpringFlower {
    pub spawn_time: f32,
}
