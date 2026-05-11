//! Pirate ship NPC system for sailing zone 200.
//!
//! This module provides:
//! - `PirateShip` component with health, stats, and AI state
//! - Waypoint-based AI navigation around islands
//! - Combat system with cannon projectiles
//! - Spawning tied to pirate spawn points in Zone 200
//! - Loot drop on defeat

use bevy::prelude::*;
use rand::Rng;

use crate::components::{BoatState, ClientEntity, ClientEntityName, Position, Zone};
use crate::events::ZoneEvent;
use crate::resources::WindState;

// ─── Re-exports ─────────────────────────────────────────────────────────────

pub use self::components::*;
pub use self::plugin::PirateShipPlugin;
pub use self::spawn_system::pirate_ship_spawn_system;
pub use self::ai_system::pirate_ship_ai_system;
pub use self::combat_system::pirate_ship_combat_system;
pub use self::cleanup_system::pirate_ship_cleanup_system;

mod components;
mod plugin;
mod spawn_system;
mod ai_system;
mod combat_system;
mod cleanup_system;
