//! Treasure Chests and Underwater Exploration
//!
//! Treasure chests scattered around the zone and underwater exploration features.

use bevy::prelude::*;

use crate::components::{FacingDirection, Position};

/// Treasure chest component
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct TreasureChest {
    /// Whether chest is opened
    pub opened: bool,
    /// Loot table index
    pub loot_table: u32,
    /// Chest value (gold)
    pub value: u32,
    /// Whether chest is underwater
    pub underwater: bool,
    /// Spawn timer for respawning
    pub respawn_timer: Timer,
}

impl Default for TreasureChest {
    fn default() -> Self {
        Self {
            opened: false,
            loot_table: 0,
            value: 100,
            underwater: false,
            respawn_timer: Timer::from_seconds(300.0, TimerMode::Once),
        }
    }
}

/// Underwater exploration area marker
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct UnderwaterArea {
    /// Area center position
    pub center: Vec3,
    /// Area radius
    pub radius: f32,
    /// Depth
    pub depth: f32,
    /// Whether area has treasure
    pub has_treasure: bool,
}

/// Resource for treasure spawn points
#[derive(Resource, Debug, Clone)]
pub struct TreasureSpawns {
    /// Treasure chest spawn locations
    pub spawn_points: Vec<Vec3>,
    /// Underwater exploration areas
    pub underwater_areas: Vec<UnderwaterArea>,
    /// Spawn timer
    pub spawn_timer: Timer,
}

impl Default for TreasureSpawns {
    fn default() -> Self {
        // Spawn points around islands and underwater
        let spawn_points = vec![
            // On islands
            Vec3::new(2000.0 * 100.0, -2000.0 * 100.0, 100.0),
            Vec3::new(8000.0 * 100.0, -2000.0 * 100.0, 100.0),
            Vec3::new(2000.0 * 100.0, -8000.0 * 100.0, 100.0),
            Vec3::new(8000.0 * 100.0, -8000.0 * 100.0, 100.0),
            // Underwater
            Vec3::new(5200.0 * 100.0, -5200.0 * 100.0, -500.0),
            Vec3::new(3000.0 * 100.0, -3000.0 * 100.0, -800.0),
            Vec3::new(7000.0 * 100.0, -7000.0 * 100.0, -600.0),
        ];

        let underwater_areas = vec![
            UnderwaterArea {
                center: Vec3::new(5200.0 * 100.0, -5200.0 * 100.0, -500.0),
                radius: 200.0 * 100.0,
                depth: 500.0,
                has_treasure: true,
            },
            UnderwaterArea {
                center: Vec3::new(3000.0 * 100.0, -3000.0 * 100.0, -800.0),
                radius: 150.0 * 100.0,
                depth: 800.0,
                has_treasure: true,
            },
        ];

        Self {
            spawn_points,
            underwater_areas,
            spawn_timer: Timer::from_seconds(600.0, TimerMode::Repeating),
        }
    }
}

/// Spawn treasure chests
pub fn treasure_spawn_system(
    time: Res<Time>,
    mut spawns: ResMut<TreasureSpawns>,
    commands: &mut Commands,
    active_chests: Query<Entity, With<TreasureChest>>,
) {
    spawns.spawn_timer.tick(time.delta());
    if !spawns.spawn_timer.just_finished() {
        return;
    }

    // Spawn chests at designated locations
    for (idx, spawn) in spawns.spawn_points.iter().enumerate() {
        let underwater = spawn.z < 0.0;
        
        commands.spawn((
            Name::new(format!("Treasure Chest {}", idx)),
            TreasureChest {
                value: if underwater { 500 } else { 100 },
                underwater,
                ..Default::default()
            },
            Position::new(*spawn),
            FacingDirection::default(),
            Transform::default(),
            GlobalTransform::default(),
        ));
    }

    log::debug!("[treasure_spawn_system] Spawned treasure chests");
}

/// Handle treasure chest loot
pub fn treasure_loot_system(
    time: Res<Time>,
    commands: &mut Commands,
    mut chest_query: Query<(Entity, &mut TreasureChest, &Position)>,
    player_query: Query<&Position, With<crate::components::PlayerCharacter>>,
) {
    for (chest_entity, mut chest, chest_position) in chest_query.iter_mut() {
        if chest.opened {
            // Check if ready to respawn
            chest.respawn_timer.tick(time.delta());
            if chest.respawn_timer.just_finished() {
                chest.opened = false;
                log::debug!("[treasure_loot_system] Treasure chest respawned");
            }
            continue;
        }

        // Check if player is nearby to open
        for player_pos in player_query.iter() {
            let dx = chest_position.position.x - player_pos.position.x;
            let dy = chest_position.position.y - player_pos.position.y;
            let dz = chest_position.position.z - player_pos.position.z;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();

            if dist < 500.0 { // 5m interaction range
                chest.opened = true;
                log::info!("[treasure_loot_system] Player opened treasure chest worth {} gold", chest.value);
                break;
            }
        }
    }
}
