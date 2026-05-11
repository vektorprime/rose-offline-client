//! Sea Creatures - Kraken, Sharks, Whales
//!
//! Various sea creatures that inhabit the sailing zone with unique behaviors.

use bevy::prelude::*;

use crate::components::{FacingDirection, Position};

/// Marker component for all sea creatures
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct SeaCreature {
    /// Creature type
    pub creature_type: SeaCreatureType,
    /// Current health
    pub health: f32,
    /// Maximum health
    pub max_health: f32,
    /// Whether this creature is active
    pub active: bool,
    /// Spawn timer for respawning
    pub spawn_timer: Timer,
}

/// Types of sea creatures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum SeaCreatureType {
    Kraken,
    Shark,
    Whale,
}

/// Kraken-specific component
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Kraken {
    /// Number of tentacles
    pub tentacle_count: u32,
    /// Current tentacle state (0=hidden, 1=emerging, 2=active, 3=grabbing)
    pub tentacle_state: u32,
    /// Grab cooldown timer
    pub grab_cooldown: Timer,
    /// Whether kraken is currently grabbing a boat
    pub is_grabbing: bool,
    /// Target boat entity
    pub target_boat: Option<Entity>,
    /// Emergence progress (0.0-1.0)
    pub emergence_progress: f32,
}

impl Default for Kraken {
    fn default() -> Self {
        Self {
            tentacle_count: 8,
            tentacle_state: 0,
            grab_cooldown: Timer::from_seconds(5.0, TimerMode::Once),
            is_grabbing: false,
            target_boat: None,
            emergence_progress: 0.0,
        }
    }
}

/// Shark-specific component
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Shark {
    /// Shark speed multiplier
    pub speed_multiplier: f32,
    /// Bite damage
    pub bite_damage: f32,
    /// Whether shark is currently attacking
    pub is_attacking: bool,
    /// Attack cooldown timer
    pub attack_cooldown: Timer,
    /// Shark pack ID (for group behavior)
    pub pack_id: u32,
}

impl Default for Shark {
    fn default() -> Self {
        Self {
            speed_multiplier: 1.5,
            bite_damage: 5.0,
            is_attacking: false,
            attack_cooldown: Timer::from_seconds(2.0, TimerMode::Once),
            pack_id: 0,
        }
    }
}

/// Whale-specific component
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Whale {
    /// Whether whale is peaceful
    pub is_peaceful: bool,
    /// Whether player can ride this whale
    pub is_ridable: bool,
    /// Current rider entity (if any)
    pub rider_entity: Option<Entity>,
    /// Breathing timer
    pub breathing_timer: Timer,
    /// Whether whale is currently breathing
    pub is_breathing: bool,
}

impl Default for Whale {
    fn default() -> Self {
        Self {
            is_peaceful: true,
            is_ridable: true,
            rider_entity: None,
            breathing_timer: Timer::from_seconds(10.0, TimerMode::Repeating),
            is_breathing: false,
        }
    }
}

/// Resource for sea creature spawn points
#[derive(Resource, Debug, Clone)]
pub struct SeaCreatureSpawns {
    /// Kraken spawn points
    pub kraken_spawns: Vec<Vec2>,
    /// Shark spawn points
    pub shark_spawns: Vec<Vec2>,
    /// Whale spawn points
    pub whale_spawns: Vec<Vec2>,
    /// Spawn interval timer
    pub spawn_timer: Timer,
}

impl Default for SeaCreatureSpawns {
    fn default() -> Self {
        Self {
            kraken_spawns: vec![
                Vec2::new(2000.0 * 100.0, -7000.0 * 100.0),
            ],
            shark_spawns: vec![
                Vec2::new(8000.0 * 100.0, -5200.0 * 100.0),
                Vec2::new(3000.0 * 100.0, -3000.0 * 100.0),
                Vec2::new(7000.0 * 100.0, -7000.0 * 100.0),
            ],
            whale_spawns: vec![
                Vec2::new(5200.0 * 100.0, -9000.0 * 100.0),
                Vec2::new(5200.0 * 100.0, -2000.0 * 100.0),
            ],
            spawn_timer: Timer::from_seconds(60.0, TimerMode::Repeating),
        }
    }
}

/// Spawn sea creatures at designated points
pub fn sea_creature_spawn_system(
    time: Res<Time>,
    mut spawns: ResMut<SeaCreatureSpawns>,
    commands: &mut Commands,
    active_creatures: Query<Entity, With<SeaCreature>>,
) {
    spawns.spawn_timer.tick(time.delta());
    if !spawns.spawn_timer.just_finished() {
        return;
    }

    // Spawn sharks in packs
    for (idx, spawn) in spawns.shark_spawns.iter().enumerate() {
        let pack_size = 3;
        for i in 0..pack_size {
            let offset_x = (i as f32 - 1.0) * 50.0 * 100.0;
            let offset_y = (i as f32 - 1.0) * 30.0 * 100.0;
            
            commands.spawn((
                Name::new(format!("Shark Pack {}-{}", idx, i)),
                SeaCreature {
                    creature_type: SeaCreatureType::Shark,
                    health: 50.0,
                    max_health: 50.0,
                    active: true,
                    spawn_timer: Timer::from_seconds(30.0, TimerMode::Once),
                },
                Shark {
                    pack_id: idx as u32,
                    ..Default::default()
                },
                Position::new(Vec3::new(
                    spawn.x + offset_x,
                    spawn.y + offset_y,
                    -200.0, // Below water surface
                )),
                FacingDirection::default(),
                Transform::default(),
                GlobalTransform::default(),
            ));
        }
    }

    // Spawn whales
    for (idx, spawn) in spawns.whale_spawns.iter().enumerate() {
        commands.spawn((
            Name::new(format!("Whale {}", idx)),
            SeaCreature {
                creature_type: SeaCreatureType::Whale,
                health: 1000.0,
                max_health: 1000.0,
                active: true,
                spawn_timer: Timer::from_seconds(120.0, TimerMode::Once),
            },
            Whale::default(),
            Position::new(Vec3::new(spawn.x, spawn.y, -500.0)),
            FacingDirection::default(),
            Transform::default(),
            GlobalTransform::default(),
        ));
    }

    log::debug!("[sea_creature_spawn_system] Spawned sea creatures");
}

/// Kraken behavior system
pub fn kraken_behavior_system(
    time: Res<Time>,
    mut kraken_query: Query<(
        &mut Kraken,
        &mut SeaCreature,
        &mut Position,
        &mut FacingDirection,
    )>,
) {
    for (mut kraken, mut creature, mut position, mut facing) in kraken_query.iter_mut() {
        if !creature.active {
            continue;
        }

        kraken.grab_cooldown.tick(time.delta());

        // Emergence behavior
        if kraken.tentacle_state == 0 {
            // Hidden - slowly emerge
            kraken.emergence_progress += time.delta_secs() * 0.01;
            if kraken.emergence_progress >= 1.0 {
                kraken.tentacle_state = 1; // Emerging
            }
        } else if kraken.tentacle_state == 1 {
            // Emerging - rise to surface
            position.position.z += time.delta_secs() * 50.0;
            if position.position.z >= -100.0 {
                kraken.tentacle_state = 2; // Active
                position.position.z = -100.0;
            }
        } else if kraken.tentacle_state == 2 {
            // Active - look for boats to grab
            // Would detect nearby boats here
            if kraken.grab_cooldown.just_finished() && !kraken.is_grabbing {
                // Attempt to grab a boat
                kraken.is_grabbing = true;
                kraken.tentacle_state = 3;
                kraken.grab_cooldown.reset();
            }
        } else if kraken.tentacle_state == 3 {
            // Grabbing - hold for a few seconds then release
            if kraken.grab_cooldown.just_finished() {
                kraken.is_grabbing = false;
                kraken.tentacle_state = 2;
                kraken.grab_cooldown.reset();
            }
        }

        // Gentle floating movement
        let t = time.elapsed_secs();
        position.position.x += t.sin() * 10.0 * time.delta_secs();
        position.position.y += t.cos() * 5.0 * time.delta_secs();

        facing.desired = position.position.x.atan2(position.position.y);
    }
}

/// Shark behavior system
pub fn shark_behavior_system(
    time: Res<Time>,
    mut shark_query: Query<(
        &mut Shark,
        &mut SeaCreature,
        &mut Position,
        &mut FacingDirection,
    )>,
) {
    for (mut shark, mut creature, mut position, mut facing) in shark_query.iter_mut() {
        if !creature.active {
            continue;
        }

        shark.attack_cooldown.tick(time.delta());

        // Swim in pack formation
        let t = time.elapsed_secs();
        let pack_offset = shark.pack_id as f32 * 100.0;
        
        // Circular swimming pattern
        let radius = 100.0 * 100.0; // 100m radius
        let speed = shark.speed_multiplier * 2.0;
        
        let angle = t * speed + pack_offset;
        let target_x = position.position.x + angle.sin() * radius * time.delta_secs();
        let target_y = position.position.y + angle.cos() * radius * time.delta_secs();
        
        position.position.x = target_x;
        position.position.y = target_y;

        // Stay below surface
        position.position.z = (-200.0..-500.0).contains(&position.position.z) 
            ? position.position.z 
            : -300.0;

        // Attack cooldown management
        if shark.is_attacking && shark.attack_cooldown.just_finished() {
            shark.is_attacking = false;
        }

        facing.desired = (target_y - position.position.y).atan2(target_x - position.position.x);
    }
}

/// Whale behavior system
pub fn whale_behavior_system(
    time: Res<Time>,
    mut whale_query: Query<(
        &mut Whale,
        &mut SeaCreature,
        &mut Position,
        &mut FacingDirection,
    )>,
) {
    for (mut whale, mut creature, mut position, mut facing) in whale_query.iter_mut() {
        if !creature.active {
            continue;
        }

        whale.breathing_timer.tick(time.delta());

        // Breathing behavior
        if whale.breathing_timer.just_finished() {
            whale.is_breathing = !whale.is_breathing;
            if whale.is_breathing {
                // Rise to surface
                position.position.z = (-100.0..0.0).contains(&position.position.z)
                    ? position.position.z
                    : -50.0;
            } else {
                // Dive back down
                position.position.z = (-500.0..-1000.0).contains(&position.position.z)
                    ? position.position.z
                    : -800.0;
            }
        }

        // Gentle swimming movement
        let t = time.elapsed_secs();
        let swim_speed = 0.5;
        
        position.position.x += t.sin() * swim_speed * 100.0 * time.delta_secs();
        position.position.y += t.cos() * swim_speed * 50.0 * time.delta_secs();

        facing.desired = position.position.x.atan2(position.position.y);
    }
}

/// Despawn dead sea creatures
pub fn sea_creature_despawn_system(
    commands: &mut Commands,
    mut creature_query: Query<(Entity, &mut SeaCreature)>,
) {
    for (entity, mut creature) in creature_query.iter_mut() {
        if creature.health <= 0.0 {
            creature.active = false;
            commands.entity(entity).despawn_recursive();
        }
    }
}
