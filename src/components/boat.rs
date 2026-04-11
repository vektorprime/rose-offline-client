use bevy::prelude::*;

/// Runtime sailing state attached to the player entity.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct BoatState {
    /// True when the player is currently sailing.
    pub active: bool,
    /// Rider entity (currently the local player entity).
    pub rider_entity: Option<Entity>,
    /// Heading in radians (0 = +Y in Position space).
    pub heading: f32,
    /// Current speed in meters/second.
    pub speed: f32,
    /// Maximum achievable speed in meters/second.
    pub max_speed: f32,
    /// Sail trim angle (0..PI).
    pub sail_trim: f32,
    /// Rudder input (-1..1).
    pub rudder: f32,
    /// Current hull health.
    pub hull_health: f32,
    /// Maximum hull health.
    pub hull_max_health: f32,
    /// Root entity for spawned visual boat model.
    pub model_root_entity: Option<Entity>,
    /// Baseline water height in game-space centimeters.
    pub water_height_cm: f32,
    /// Wave roll angle in radians.
    pub wave_roll: f32,
    /// Wave pitch angle in radians.
    pub wave_pitch: f32,
}

impl Default for BoatState {
    fn default() -> Self {
        Self {
            active: false,
            rider_entity: None,
            heading: 0.0,
            speed: 0.0,
            max_speed: 9.0,
            sail_trim: std::f32::consts::FRAC_PI_4,
            rudder: 0.0,
            hull_health: 100.0,
            hull_max_health: 100.0,
            model_root_entity: None,
            water_height_cm: 0.0,
            wave_roll: 0.0,
            wave_pitch: 0.0,
        }
    }
}

/// References to major visual parts of the spawned boat model.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct BoatModel {
    pub root_entity: Entity,
    pub hull_entity: Entity,
    pub mast_entity: Entity,
    pub sail_entity: Entity,
    pub rudder_entity: Entity,
    pub rider_seat_entity: Entity,
}

/// Marker for procedural sail mesh runtime data.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct SailMesh {
    /// 0 = limp, 1 = fully filled.
    pub billow: f32,
    pub side: SailSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum SailSide {
    Port,
    Starboard,
    Center,
}

