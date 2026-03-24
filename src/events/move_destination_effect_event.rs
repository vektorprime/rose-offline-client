use bevy::prelude::{Message, Vec3};

#[derive(Message)]
pub enum MoveDestinationEffectEvent {
    Show { position: Vec3 },
    Hide,
}
