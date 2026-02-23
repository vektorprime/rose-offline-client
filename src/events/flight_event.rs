use bevy::prelude::*;

/// Event sent when flight mode should be toggled for an entity
#[derive(Event, Clone, Debug)]
pub struct FlightToggleEvent {
    /// The entity to toggle flight for
    pub entity: Entity,
}
