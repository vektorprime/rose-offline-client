use bevy::prelude::*;

use crate::components::PlayerCharacter;
use crate::events::MoveSpeedSetEvent;
use rose_game_common::components::MoveSpeed;

/// System that handles move speed set events.
///
/// This system listens for [`MoveSpeedSetEvent`] events and updates the
/// [`MoveSpeed`] component on the target entity.
pub fn move_speed_set_system(
    mut events: EventReader<MoveSpeedSetEvent>,
    mut query: Query<&mut MoveSpeed, With<PlayerCharacter>>,
) {
    for event in events.read() {
        // Try to get the MoveSpeed component for the entity
        if let Ok(mut move_speed) = query.get_mut(event.entity) {
            move_speed.speed = event.speed;
            info!(
                "Set move speed for entity {:?} to {}",
                event.entity, event.speed
            );
        } else {
            warn!(
                "MoveSpeedSetEvent received for entity {:?} but it has no MoveSpeed component",
                event.entity
            );
        }
    }
}
