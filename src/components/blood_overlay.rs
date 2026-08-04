//! Blood overlay component for model texture blood effects.
//!
//! This module provides the [`BloodOverlay`] component which tracks blood stains
//! on entity models. Blood is rendered as an overlay texture sampled in a custom
//! material extension, providing realistic blood appearance that deforms with
//! skeletal animation.
//!
//! Stains are tracked per-material (identified by entity ID of the mesh part)
//! to avoid UV space mismatch across multi-material entities (Root Cause #1).
//! Each material gets its own overlay texture with stains painted only in that
//! material's UV space.

use bevy::{prelude::*, reflect::Reflect};
use std::collections::HashMap;

/// UV-space blood stain position.
#[derive(Clone, Debug, Reflect)]
pub struct BloodStain {
    /// UV space position (center of stain) in [0, 1] range.
    pub uv_center: Vec2,
    /// UV space size of the stain (width, height).
    pub uv_size: Vec2,
    /// Rotation in UV space (radians).
    pub rotation: f32,
    /// Alpha intensity of the stain (0.0 = invisible, 1.0 = fully opaque).
    pub alpha: f32,
    /// Which blood texture variant to use (0-7 for different stain shapes).
    pub texture_variant: usize,
    /// Whether this stain is currently visible.
    pub visible: bool,
    /// The material entity this stain belongs to.
    /// When None, the stain applies to all materials (legacy behavior).
    /// When Some(entity), the stain only applies to that specific material's overlay.
    pub material_entity: Option<Entity>,
}

impl BloodStain {
    /// Creates a new blood stain at the given UV position for a specific material.
    pub fn new(uv_center: Vec2, uv_size: f32, variant: usize) -> Self {
        Self::new_inner(uv_center, uv_size, variant, None)
    }

    /// Creates a new blood stain at the given UV position for a specific material entity.
    pub fn new_for_material(
        uv_center: Vec2,
        uv_size: f32,
        variant: usize,
        material_entity: Entity,
    ) -> Self {
        Self::new_inner(uv_center, uv_size, variant, Some(material_entity))
    }

    fn new_inner(uv_center: Vec2, uv_size: f32, variant: usize, material_entity: Option<Entity>) -> Self {
        Self {
            uv_center,
            uv_size: Vec2::splat(uv_size),
            rotation: rand::random::<f32>() * std::f32::consts::TAU,
            alpha: 0.92,
            texture_variant: variant % 8,
            visible: true,
            material_entity,
        }
    }
}

/// Component that tracks blood overlay state for an entity.
///
/// This component is attached to entities that can have blood stains on their
/// model textures. Blood stains are accumulated over time during combat and
/// rendered via the [`BloodOverlayExtension`](crate::render::BloodOverlayExtension)
/// material extension.
///
/// Stains are organized per-material to avoid UV space mismatch across
/// multi-material entities (e.g., Head, Body, Arms have different UV layouts).
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct BloodOverlay {
    /// List of blood stains on this entity.
    pub stains: Vec<BloodStain>,
    /// Whether the overlay texture needs to be regenerated.
    pub texture_dirty: bool,
    /// Whether this entity currently has visible blood.
    pub is_bloodied: bool,
    /// Maximum number of stains before old ones are removed.
    pub max_stains: usize,
    /// Per-material dirty flags. When a material entity is present here,
    /// its overlay texture needs regeneration.
    pub material_dirty: HashMap<Entity, bool>,
    /// Cache of material-bearing entities in this entity's model subtree.
    /// Refreshed when the overlay textures are regenerated; clean frames use it
    /// to re-bind overlay handles without walking the subtree again.
    pub material_part_entities: Vec<Entity>,
}

impl BloodOverlay {
    /// Creates a new empty blood overlay for the given entity.
    pub fn new() -> Self {
        Self {
            stains: Vec::new(),
            texture_dirty: false,
            is_bloodied: false,
            max_stains: 20,
            material_dirty: HashMap::new(),
            material_part_entities: Vec::new(),
        }
    }

    /// Adds a blood stain at the given UV position (applies to all materials).
    pub fn add_stain(&mut self, uv_center: Vec2, uv_size: f32, variant: usize) {
        if self.stains.len() >= self.max_stains {
            // Remove oldest stain
            self.stains.remove(0);
        }
        self.stains
            .push(BloodStain::new(uv_center, uv_size, variant));
        self.texture_dirty = true;
        self.is_bloodied = true;
    }

    /// Adds a blood stain at the given UV position for a specific material entity.
    /// This avoids the UV space mismatch problem (Root Cause #1) by only painting
    /// the stain on the material that was actually hit.
    pub fn add_stain_for_material(
        &mut self,
        uv_center: Vec2,
        uv_size: f32,
        variant: usize,
        material_entity: Entity,
    ) {
        if self.stains.len() >= self.max_stains {
            // Remove oldest stain
            self.stains.remove(0);
        }
        self.stains.push(BloodStain::new_for_material(
            uv_center,
            uv_size,
            variant,
            material_entity,
        ));
        self.texture_dirty = true;
        self.is_bloodied = true;
        self.material_dirty.insert(material_entity, true);
    }

    /// Returns stains that should be painted on the given material entity.
    /// Stains with no material_entity (legacy) are included for all materials.
    /// Stains with a matching material_entity are included only for that material.
    pub fn stains_for_material(&self, material_entity: Entity) -> Vec<&BloodStain> {
        self.stains
            .iter()
            .filter(|s| {
                s.visible
                    && (s.material_entity.is_none() || s.material_entity == Some(material_entity))
            })
            .collect()
    }

    /// Removes all blood stains from this entity.
    pub fn clear_stains(&mut self) {
        if !self.stains.is_empty() {
            self.stains.clear();
            self.texture_dirty = true;
            self.is_bloodied = false;
            self.material_dirty.clear();
        }
    }

    /// Returns the number of active stains.
    pub fn stain_count(&self) -> usize {
        self.stains.iter().filter(|s| s.visible).count()
    }

    /// Returns whether a specific material's overlay needs regeneration.
    pub fn is_material_dirty(&self, material_entity: Entity) -> bool {
        self.texture_dirty
            || self
                .material_dirty
                .get(&material_entity)
                .copied()
                .unwrap_or(false)
    }

    /// Marks a specific material's overlay as clean after regeneration.
    pub fn mark_material_clean(&mut self, material_entity: Entity) {
        self.material_dirty.insert(material_entity, false);
    }
}
