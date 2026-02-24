use bevy::prelude::*;

/// Event sent when movement speed should be set for an entity
#[derive(Event, Clone, Debug)]
pub struct MoveSpeedSetEvent {
    /// The entity to set the movement speed for
    pub entity: Entity,
    /// The new movement speed value
    pub speed: f32,
}
