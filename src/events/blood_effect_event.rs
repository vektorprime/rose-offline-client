//! Events for the blood effects system.
//!
//! These events are used to trigger blood effect spawning, updates, and cleanup.

use bevy::{prelude::*, reflect::Reflect};

/// Event triggered when blood effects should be spawned or updated.
///
/// This event enum covers all blood-related actions:
/// - Spawning blood spatter decals on terrain
/// - Showing gash wounds on damaged entities
/// - Updating wound visibility based on HP changes
/// - Cleaning up wounds when entities despawn
#[derive(Event, Reflect, Clone, Debug)]
pub enum BloodEffectEvent {
    /// Spawn blood spatter decals on terrain at the specified position.
    ///
    /// This is typically triggered when an entity is killed.
    SpawnSpatter {
        /// World position where the spatter should appear.
        position: Vec3,
        /// Surface normal for the spatter orientation (usually Vec3::Y for terrain).
        normal: Vec3,
        /// Amount of damage that caused this spatter (affects size/intensity).
        damage_amount: u32,
        /// Whether this is a killing blow (triggers more spatters).
        is_kill: bool,
    },

    /// Show a gash wound on an entity.
    ///
    /// Triggered when an entity's HP drops below the wound visibility threshold.
    ShowWound {
        /// Entity to attach the wound visual to.
        entity: Entity,
        /// Local position on the entity for the wound.
        wound_position: Vec3,
        /// Normal direction for wound orientation.
        wound_normal: Vec3,
    },

    /// Update wound visibility based on current HP percentage.
    ///
    /// This allows the system to show/hide wounds as HP changes.
    UpdateWoundVisibility {
        /// Entity whose wound visibility should be updated.
        entity: Entity,
        /// Current HP as a percentage (0.0 - 1.0).
        health_percent: f32,
    },

    /// Clean up all wound visuals for an entity.
    ///
    /// Should be triggered when an entity is about to despawn.
    CleanupWounds {
        /// Entity whose wounds should be cleaned up.
        entity: Entity,
    },
}

impl BloodEffectEvent {
    /// Creates a new SpawnSpatter event for a killing blow.
    pub fn kill_spatter(position: Vec3, normal: Vec3, damage_amount: u32) -> Self {
        Self::SpawnSpatter {
            position,
            normal,
            damage_amount,
            is_kill: true,
        }
    }

    /// Creates a new SpawnSpatter event for a non-lethal hit.
    pub fn hit_spatter(position: Vec3, normal: Vec3, damage_amount: u32) -> Self {
        Self::SpawnSpatter {
            position,
            normal,
            damage_amount,
            is_kill: false,
        }
    }

    /// Creates a new ShowWound event.
    pub fn show_wound(entity: Entity, wound_position: Vec3, wound_normal: Vec3) -> Self {
        Self::ShowWound {
            entity,
            wound_position,
            wound_normal,
        }
    }

    /// Creates a new UpdateWoundVisibility event.
    pub fn update_visibility(entity: Entity, health_percent: f32) -> Self {
        Self::UpdateWoundVisibility {
            entity,
            health_percent,
        }
    }

    /// Creates a new CleanupWounds event.
    pub fn cleanup(entity: Entity) -> Self {
        Self::CleanupWounds { entity }
    }
}
