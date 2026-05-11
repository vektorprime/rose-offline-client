//! Plugin that registers all pirate ship systems.

use bevy::prelude::*;

use super::components::{PirateShip, PirateShipSettings, PirateCannonProjectile};
use super::spawn_system::pirate_ship_spawn_system;
use super::ai_system::pirate_ship_ai_system;
use super::combat_system::pirate_ship_combat_system;
use super::cleanup_system::pirate_ship_cleanup_system;

/// Plugin for pirate ship NPC systems.
pub struct PirateShipPlugin;

impl Plugin for PirateShipPlugin {
    fn build(&self, app: &mut App) {
        log::info!("[PIRATE] PirateShipPlugin::build() called");

        app
            // Register types
            .register_type::<PirateShip>()
            .register_type::<PirateCannonProjectile>()

            // Initialize settings resource
            .init_resource::<PirateShipSettings>()

            // Spawn system - runs when zone 200 is loaded
            .add_systems(Update, pirate_ship_spawn_system)

            // AI navigation - runs every frame
            .add_systems(Update, pirate_ship_ai_system)

            // Combat system - runs every frame
            .add_systems(Update, pirate_ship_combat_system)

            // Cleanup dead ships and projectiles
            .add_systems(Update, pirate_ship_cleanup_system);
    }
}
