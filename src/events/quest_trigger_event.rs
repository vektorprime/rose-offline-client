use bevy::prelude::Event;

use rose_data::{ItemReference, QuestTriggerHash};

#[derive(Event)]
pub enum QuestTriggerEvent {
    ApplyRewards(QuestTriggerHash),
    DoTrigger(QuestTriggerHash),
    /// Triggered when a quest scroll item is used. Shows dialog before triggering quest.
    UseQuestScroll(ItemReference, QuestTriggerHash),
}
