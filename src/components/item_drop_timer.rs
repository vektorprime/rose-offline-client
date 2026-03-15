use bevy::ecs::prelude::Component;
use std::time::Duration;

use crate::components::ClientEntityId;

/// Component for tracking the remaining time before an item drop despawns
#[derive(Component, Clone, Debug)]
pub struct ItemDropRemainingTime {
    pub remaining_time: Duration,
}

impl ItemDropRemainingTime {
    pub fn new(remaining_time: Duration) -> Self {
        Self { remaining_time }
    }
}

/// Component for tracking the owner of an item drop
#[derive(Component, Clone, Debug)]
pub struct ItemDropOwner {
    pub owner_entity_id: Option<ClientEntityId>,
}

impl ItemDropOwner {
    pub fn new(owner_entity_id: Option<ClientEntityId>) -> Self {
        Self { owner_entity_id }
    }
}
