//! Angelic Wing Spawn System
//!
//! This system listens for flight mode toggles. Wing model spawning is currently
//! DISABLED - flying functionality (movement, animation) works without visual wings.

use bevy::prelude::*;

use crate::components::PlayerCharacter;
use crate::events::FlightToggleEvent;
use crate::render::wing_material::WingMaterialPlugin;

/// Plugin for wing systems
pub struct WingSpawnPlugin;

impl Plugin for WingSpawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WingMaterialPlugin::default())
            .add_systems(Update, wing_spawn_system);
    }
}

/// System that handles wing spawning when flight is enabled.
///
/// Wing model spawning is currently DISABLED - the system only logs when flight
/// is toggled without spawning visual wings.
pub fn wing_spawn_system(
    mut flight_events: MessageReader<FlightToggleEvent>,
    player_query: Query<Entity, With<PlayerCharacter>>,
) {
    for event in flight_events.read() {
        if player_query.contains(event.entity) {
            log::info!(
                "[WingSpawn] Flight toggled for entity {:?} - wing spawning disabled",
                event.entity
            );
        }
    }
}
