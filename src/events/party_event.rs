use bevy::prelude::{Entity, Message};

#[derive(Message)]
pub enum PartyEvent {
    InvitedCreate(Entity),
    InvitedJoin(Entity),
}
