use bevy::prelude::{Entity, Message};

use rose_data::ItemReference;

#[derive(Message)]
pub struct UseItemEvent {
    pub entity: Entity,
    pub item: ItemReference,
}
