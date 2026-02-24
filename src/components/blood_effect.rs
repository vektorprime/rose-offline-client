//! Blood effect components for visual feedback during combat.
//!
//! This module provides components for:
//! - Blood spatter decals on terrain when entities are killed
//! - Gash wounds that appear on entities when HP drops below 50%

use bevy::{prelude::*, reflect::Reflect};

/// Marker for blood spatter decal entities.
///
/// Blood spatters are spawned on terrain when monsters/NPCs are killed.
/// They fade out over time based on the configured lifetime.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct BloodSpatter {
    /// Time remaining before this spatter fades out completely (in seconds).
    pub lifetime: f32,
    /// Current alpha transparency value (0.0 = invisible, 1.0 = fully opaque).
    pub alpha: f32,
    /// Size of the decal in world units.
    pub size: f32,
}

impl Default for BloodSpatter {
    fn default() -> Self {
        Self {
            lifetime: 30.0,
            alpha: 0.8,
            size: 1.0,
        }
    }
}

/// Tracks wound state for an entity that can show gash wounds.
///
/// Wounds appear when an entity's HP drops below 50% and remain visible
/// until the entity despawns.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct GashWounds {
    /// Number of wound visuals currently attached to this entity.
    pub wound_count: usize,
    /// Whether wounds are currently visible (HP < 50% threshold).
    pub wounds_visible: bool,
    /// The parent entity that owns these wounds (for cleanup tracking).
    pub parent_entity: Entity,
}

impl GashWounds {
    /// Creates a new GashWounds component for the given parent entity.
    pub fn new(parent_entity: Entity) -> Self {
        Self {
            wound_count: 0,
            wounds_visible: false,
            parent_entity,
        }
    }
}

/// Marker component for wound visual child entities.
///
/// These are spawned as children of entities with [`GashWounds`] when
/// wounds become visible.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct WoundVisual {
    /// The parent entity this wound visual is attached to.
    pub parent_entity: Entity,
}

impl WoundVisual {
    /// Creates a new WoundVisual marker for the given parent.
    pub fn new(parent_entity: Entity) -> Self {
        Self { parent_entity }
    }
}

/// Configuration for blood spatter appearance and behavior.
///
/// This can be attached to entities for custom blood configurations,
/// or used as a default via [`BloodEffectConfig`](crate::resources::BloodEffectConfig).
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct BloodSpatterConfig {
    /// Minimum random spatter size in world units.
    pub min_size: f32,
    /// Maximum random spatter size in world units.
    pub max_size: f32,
    /// Number of spatter decals to spawn on death.
    pub spatter_count: usize,
    /// How long spatters persist before fading (in seconds).
    pub spatter_lifetime: f32,
    /// Maximum distance from death position for spatter placement.
    pub spatter_radius: f32,
}

impl Default for BloodSpatterConfig {
    fn default() -> Self {
        Self {
            min_size: 0.3,
            max_size: 1.5,
            spatter_count: 5,
            spatter_lifetime: 30.0,
            spatter_radius: 2.0,
        }
    }
}
