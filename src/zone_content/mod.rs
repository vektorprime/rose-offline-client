//! Zone content spawners for zone 200 (ocean).
//!
//! The ocean zone's IFO contains no NPCs, docks, or content, so everything is
//! spawned client-side when the zone loads:
//! - `npcs`: dock NPCs (boat vendor, ferryman, quest giver, traders)
//! - `monsters`: sea sharks with swim AI and boat hull combat
//! - `boats`: decorative civilian boats patrolling open-water routes
//! - `docks`: procedural wooden docks at the island shores

pub mod boats;
pub mod docks;
pub mod monsters;
pub mod npcs;
