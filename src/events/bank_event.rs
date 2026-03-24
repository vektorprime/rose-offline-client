use bevy::prelude::Message;

use rose_game_common::messages::ClientEntityId;

#[derive(Message)]
pub enum BankEvent {
    OpenBankFromClientEntity { client_entity_id: ClientEntityId },
    Show,
}
