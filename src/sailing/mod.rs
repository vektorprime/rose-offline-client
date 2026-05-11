//! Sailing Zone Features Module
//!
//! This module contains all the sailing-specific gameplay features:
//! - Pirate ship NPCs with AI navigation and combat
//! - Sea creatures (kraken, sharks, whales)
//! - Dynamic storms system
//! - Boat cannons and projectiles
//! - Treasure chests and underwater exploration
//! - Sailing zone management

mod mod_pirates;
mod mod_sea_creatures;
mod mod_storms;
mod mod_cannons;
mod mod_treasure;
mod mod_sailing_zone;

pub use mod_pirates::*;
pub use mod_sea_creatures::*;
pub use mod_storms::*;
pub use mod_cannons::*;
pub use mod_treasure::*;
pub use mod_sailing_zone::*;

/// Plugin that registers all sailing zone features
pub struct SailingFeaturesPlugin;

impl Plugin for SailingFeaturesPlugin {
    fn build(&self, app: &mut App) {
        // Register resources
        app.init_resource::<SailingZoneConfig>()
            .init_resource::<StormState>();

        // Register sailing zone systems
        app.add_systems(
            Update,
            (
                // Pirate ship systems
                pirate_ship_spawn_system
                    .after(crate::systems::boat_spawn_system::ensure_boat_state_system),
                pirate_ship_ai_system
                    .after(sailing_movement_system),
                pirate_ship_combat_system
                    .after(pirate_ship_ai_system),
                pirate_ship_despawn_system
                    .after(pirate_ship_combat_system),

                // Sea creature systems
                sea_creature_spawn_system
                    .after(pirate_ship_spawn_system),
                kraken_behavior_system
                    .after(sea_creature_spawn_system),
                shark_behavior_system
                    .after(kraken_behavior_system),
                whale_behavior_system
                    .after(shark_behavior_system),
                sea_creature_despawn_system
                    .after(whale_behavior_system),

                // Storm systems
                storm_update_system
                    .after(crate::resources::wind_state::wind_update_system),
                storm_visual_system
                    .after(storm_update_system),
                storm_effect_on_boats_system
                    .after(storm_visual_system),

                // Cannon systems
                cannon_fire_system
                    .after(pirate_ship_combat_system),
                cannon_projectile_system
                    .after(cannon_fire_system),
                cannon_impact_system
                    .after(cannon_projectile_system),

                // Treasure systems
                treasure_spawn_system
                    .after(sea_creature_spawn_system),
                treasure_loot_system
                    .after(treasure_spawn_system),
            ),
        );

        log::info!("[SailingFeaturesPlugin] Sailing zone features initialized");
    }
}
