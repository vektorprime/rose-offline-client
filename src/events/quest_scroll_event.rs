use bevy::prelude::Message;
use rose_game_common::components::ItemSlot;

/// Message for quest scroll dialog interaction
#[derive(Message)]
pub enum QuestScrollEvent {
    /// Open a quest scroll dialog for the given item
    Show {
        /// The slot location of the quest scroll item being used
        item_slot: ItemSlot,
        /// Quest trigger name (derived from confile)
        quest_trigger: String,
    },
    /// User confirmed the quest scroll dialog
    Confirm {
        /// Quest trigger to execute
        quest_trigger: String,
        /// Item slot that was used
        item_slot: ItemSlot,
    },
    /// User cancelled the quest scroll dialog
    Cancel,
}
