use bevy::prelude::{MessageReader, MessageWriter, Res};
use rose_game_common::{components::ItemSlot, messages::client::ClientMessage};

use crate::{events::QuestScrollEvent, resources::GameConnection};

/// System to handle QuestScrollEvent::Confirm
/// This is triggered when the user confirms the quest scroll dialog
pub fn quest_scroll_event_system(
    mut quest_scroll_events: MessageReader<QuestScrollEvent>,
    game_connection: Option<Res<GameConnection>>,
) {
    for event in quest_scroll_events.read() {
        match event {
            QuestScrollEvent::Confirm {
                quest_trigger: _quest_trigger,
                item_slot,
            } => {
                log::info!("QuestScroll confirmed for item slot {:?}", item_slot);

                // Send UseItem message to server - server will authoritatively process the quest trigger
                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::UseItem {
                            item_slot: *item_slot,
                            target_entity_id: None, // QuestScroll doesn't target entities
                        })
                        .ok();
                }

                // Client no longer performs local quest condition checking
                // Server will evaluate conditions and return success/failure response
            }
            QuestScrollEvent::Cancel => {
                log::debug!("QuestScroll dialog cancelled");
                // No action needed on cancel - the dialog just closes
            }
            QuestScrollEvent::Show { .. } => {
                // Show events are handled by the UI system
                log::trace!("QuestScroll dialog show event passed through");
            }
        }
    }
}
