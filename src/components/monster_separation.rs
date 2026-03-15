use bevy::prelude::{Component, Reflect};

/// Component for hostile monster collision separation.
/// This is only added to monsters (ClientEntityType::Monster), not NPCs.
#[derive(Component, Reflect)]
pub struct MonsterSeparation {
    /// Radius for separation detection in meters
    pub separation_radius: f32,
    /// Force strength when overlapping
    pub separation_force: f32,
    /// Maximum separation per frame in meters
    pub max_separation: f32,
}

impl Default for MonsterSeparation {
    fn default() -> Self {
        Self {
            separation_radius: 1.0,  // 1.5 meters
            separation_force: 5.0,
            max_separation: 2.0,     // 2 meters per second max
        }
    }
}
