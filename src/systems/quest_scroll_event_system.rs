use bevy::prelude::{MessageReader, MessageWriter, Res};
use rose_game_common::{components::ItemSlot, messages::client::ClientMessage};

use crate::{
    events::{QuestScrollEvent, QuestTriggerEvent},
    resources::GameConnection,
};

/// System to handle QuestScrollEvent::Confirm
/// This is triggered when the user confirms the quest scroll dialog
pub fn quest_scroll_event_system(
    mut quest_scroll_events: MessageReader<QuestScrollEvent>,
    mut quest_trigger_events: MessageWriter<QuestTriggerEvent>,
    game_connection: Option<Res<GameConnection>>,
) {
    for event in quest_scroll_events.read() {
        match event {
            QuestScrollEvent::Confirm {
                quest_trigger,
                item_slot,
            } => {
                log::info!(
                    "QuestScroll confirmed for trigger {}, item slot {:?}",
                    quest_trigger,
                    item_slot
                );

                // Send UseItem message to server - server will process the quest trigger
                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::UseItem {
                            item_slot: *item_slot,
                            target_entity_id: None, // QuestScroll doesn't target entities
                        })
                        .ok();
                }

                // Also dispatch DoTrigger event for client-side condition checking
                // This allows the client to validate quest conditions before the server responds
                quest_trigger_events.write(QuestTriggerEvent::DoTrigger(
                    quest_trigger.as_str().into()
                ));
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
