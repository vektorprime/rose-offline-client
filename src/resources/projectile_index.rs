use bevy::prelude::{Entity, Resource};
use std::collections::HashMap;

/// Index of in-flight projectile entities grouped by their source (attacker).
///
/// Lets pending-damage kill resolution check only the attacker's own projectiles
/// instead of scanning every projectile in the world. Entries are pruned when a
/// source's last projectile is removed.
#[derive(Default, Resource)]
pub struct ProjectileIndex {
    by_source: HashMap<Entity, Vec<Entity>>,
}

impl ProjectileIndex {
    pub fn add(&mut self, source: Entity, projectile: Entity) {
        self.by_source.entry(source).or_default().push(projectile);
    }

    pub fn remove(&mut self, source: Entity, projectile: Entity) {
        if let Some(projectiles) = self.by_source.get_mut(&source) {
            if let Some(index) = projectiles.iter().position(|&candidate| candidate == projectile)
            {
                projectiles.swap_remove(index);
            }
            if projectiles.is_empty() {
                self.by_source.remove(&source);
            }
        }
    }

    pub fn get(&self, source: &Entity) -> Option<&Vec<Entity>> {
        self.by_source.get(source)
    }
}
