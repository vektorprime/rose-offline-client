use bevy::prelude::*;

/// Season types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum Season {
    #[default]
    None,
    Spring,
    Summer,
    Fall,
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

/// Component for grass blade entities spawned in summer
#[derive(Component, Debug, Clone, Reflect)]
pub struct GrassBlade {
    /// Initial rotation offset for varied swaying
    pub sway_offset: f32,
    /// Sway speed multiplier
    pub sway_speed: f32,
    /// Maximum sway amplitude in radians
    pub sway_amplitude: f32,
    /// Height of the grass blade (for scaling)
    pub height: f32,
}

/// Component for flower entities spawned in summer
#[derive(Component, Debug, Clone, Reflect)]
pub struct SummerFlower {
    /// Initial rotation offset for varied swaying
    pub sway_offset: f32,
    /// Sway speed multiplier
    pub sway_speed: f32,
    /// Flower color index for material selection
    pub color_index: usize,
    /// Height of the flower stem
    pub stem_height: f32,
}
