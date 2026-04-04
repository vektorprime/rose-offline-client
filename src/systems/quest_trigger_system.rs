use bevy::prelude::MessageReader;
use rose_game_common::messages::client::ClientMessage;

use crate::{
    events::QuestTriggerEvent,
    scripting::{
        quest_apply_rewards, quest_check_conditions, ScriptFunctionContext, ScriptFunctionResources,
    },
};

pub fn quest_trigger_system(
    mut quest_trigger_events: MessageReader<QuestTriggerEvent>,
    mut script_context: ScriptFunctionContext,
    script_resources: ScriptFunctionResources,
) {
    for event in quest_trigger_events.read() {
        match *event {
            QuestTriggerEvent::ApplyRewards(trigger_hash) => {
                // Authority migrated to server. Client no longer applies rewards locally.
            }
            QuestTriggerEvent::DoTrigger(trigger_hash) => {
                if let Some(game_connection) = script_resources.game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::QuestTrigger {
                            trigger: trigger_hash,
                        })
                        .ok();
                }
            }
            QuestTriggerEvent::UseQuestScroll(item_reference, trigger_hash) => {
                // This event is dispatched when a quest scroll item is used.
                // It allows for client-side dialog showing before triggering the quest.

                // Note: Full implementation requires confile parsing to extract quest information
                // (title, description) and display in a modal dialog. Since confile parsing isn't
                // implemented yet, we fall back to DoTrigger behavior which sends QuestTrigger to server.

                log::info!(
                    "QuestScroll item {:?} used for trigger {:?}, showing dialog not yet implemented",
                    item_reference,
                    trigger_hash
                );

                // Fall back to DoTrigger behavior - send QuestTrigger to server for validation
                if let Some(game_connection) = script_resources.game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::QuestTrigger {
                            trigger: trigger_hash,
                        })
                        .ok();
                }
            }
        }
    }
}
