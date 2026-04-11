use bevy::prelude::*;

#[derive(Message, Clone, Debug)]
pub struct BoardBoatEvent {
    pub entity: Entity,
}

#[derive(Message, Clone, Debug)]
pub struct DisembarkBoatEvent {
    pub entity: Entity,
}

