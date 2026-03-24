use bevy::prelude::*;

/// Message sent when flight mode should be toggled for an entity
#[derive(Message, Clone, Debug)]
pub struct FlightToggleEvent {
    /// The entity to toggle flight for
    pub entity: Entity,
}
