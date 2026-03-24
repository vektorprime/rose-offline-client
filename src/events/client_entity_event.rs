use bevy::prelude::{Entity, Message};

#[derive(Message, Copy, Clone, Debug)]
pub enum ClientEntityEvent {
    Die(Entity),
    LevelUp(Entity, Option<u32>),
}
