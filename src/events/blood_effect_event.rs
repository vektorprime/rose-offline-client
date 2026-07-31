//! Messages for the blood effects system.
//!
//! These messages are used to trigger blood effect spawning, updates, and cleanup.

use bevy::{prelude::*, reflect::Reflect};

/// Blood impact profile used to tune layered blood behavior.
#[derive(Reflect, Clone, Copy, Debug, Default)]
pub enum BloodImpactProfile {
    #[default]
    Slash,
    Pierce,
    Blunt,
    SkillMagic,
    Projectile,
}

/// Message triggered when blood effects should be spawned or updated.
#[derive(Message, Reflect, Clone, Debug)]
pub enum BloodEffectEvent {
    /// Spawn blood spatter decals on terrain at the specified position.
    SpawnSpatter {
        position: Vec3,
        normal: Vec3,
        impact_direction: Vec3,
        damage_amount: u32,
        is_kill: bool,
        profile: BloodImpactProfile,
    },

    /// Show a gash wound on an entity.
    ShowWound {
        entity: Entity,
        wound_position: Vec3,
        wound_normal: Vec3,
    },

    /// Update wound visibility based on current HP percentage.
    UpdateWoundVisibility {
        entity: Entity,
        health_percent: f32,
    },

    /// Clean up all wound visuals for an entity.
    CleanupWounds { entity: Entity },
}

impl BloodEffectEvent {
    /// Creates a new SpawnSpatter event for a killing blow with a profile.
    pub fn kill_spatter_with_profile(
        position: Vec3,
        normal: Vec3,
        damage_amount: u32,
        impact_direction: Vec3,
        profile: BloodImpactProfile,
    ) -> Self {
        Self::SpawnSpatter {
            position,
            normal,
            impact_direction,
            damage_amount,
            is_kill: true,
            profile,
        }
    }

    /// Creates a new SpawnSpatter event for a non-lethal hit with a profile.
    pub fn hit_spatter_with_profile(
        position: Vec3,
        normal: Vec3,
        damage_amount: u32,
        impact_direction: Vec3,
        profile: BloodImpactProfile,
    ) -> Self {
        Self::SpawnSpatter {
            position,
            normal,
            impact_direction,
            damage_amount,
            is_kill: false,
            profile,
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
}
