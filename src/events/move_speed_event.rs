use bevy::prelude::*;

/// Message sent when movement speed should be set for an entity
#[derive(Message, Clone, Debug)]
pub struct MoveSpeedSetEvent {
    /// The entity to set the movement speed for
    pub entity: Entity,
    /// The new movement speed value
    pub speed: f32,
}
