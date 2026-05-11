//! Pirate Ship NPCs - AI navigation, combat, spawning
//!
//! Pirate ships are NPC-controlled boats that sail around the zone,
//! patrol waypoints, and engage player boats in combat.

use bevy::prelude::*;

use crate::components::{BoatState, FacingDirection, Position};

/// Marker component for pirate ship entities
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct PirateShip {
    /// Unique pirate ship ID
    pub id: u32,
    /// Current health
    pub health: f32,
    /// Maximum health
    pub max_health: f32,
    /// Attack damage per cannon shot
    pub attack_damage: f32,
    /// Defense reduction factor (0.0-1.0)
    pub defense: f32,
    /// Current AI state
    pub ai_state: PirateShipAiState,
    /// Index of current waypoint
    pub current_waypoint: usize,
    /// Timer for state transitions
    pub state_timer: Timer,
    /// Combat cooldown timer
    pub combat_cooldown: Timer,
    /// Detection range for player boats (meters)
    pub detection_range: f32,
    /// Combat range for cannon fire (meters)
    pub combat_range: f32,
    /// Whether this ship is currently engaged in combat
    pub in_combat: bool,
    /// Entity of the target player boat (if in combat)
    pub target_entity: Option<Entity>,
}

impl Default for PirateShip {
    fn default() -> Self {
        Self {
            id: 0,
            health: 200.0,
            max_health: 200.0,
            attack_damage: 15.0,
            defense: 0.3,
            ai_state: PirateShipAiState::Patrol,
            current_waypoint: 0,
            state_timer: Timer::from_seconds(2.0, TimerMode::Once),
            combat_cooldown: Timer::from_seconds(3.0, TimerMode::Once),
            detection_range: 200.0,
            combat_range: 100.0,
            in_combat: false,
            target_entity: None,
        }
    }
}

/// AI states for pirate ships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum PirateShipAiState {
    /// Sailing between waypoints
    Patrol,
    /// Chasing player boat
    Chase,
    /// Firing cannons at player
    Combat,
    /// Fleeing when low health
    Flee,
    /// Dead/despawning
    Dead,
}

/// Waypoint for pirate ship patrol routes
#[derive(Debug, Clone, Reflect)]
pub struct PirateWaypoint {
    /// Position in game coordinates (cm)
    pub position: Vec2,
    /// Dwell time at this waypoint (seconds)
    pub dwell_time: f32,
}

/// Resource storing pirate ship spawn points and waypoints
#[derive(Resource, Debug, Clone)]
pub struct PirateShipSpawns {
    /// Spawn positions for pirate ships
    pub spawn_points: Vec<Vec2>,
    /// Patrol waypoints for each ship
    pub waypoints: Vec<Vec<PirateWaypoint>>,
    /// Spawn interval timer
    pub spawn_timer: Timer,
    /// Maximum active pirate ships
    pub max_active: usize,
}

impl Default for PirateShipSpawns {
    fn default() -> Self {
        // Default spawn points based on Zone 200 pirate locations
        let spawn_points = vec![
            Vec2::new(3000.0 * 100.0, -3000.0 * 100.0),
            Vec2::new(7000.0 * 100.0, -7000.0 * 100.0),
            Vec2::new(5200.0 * 100.0, -3000.0 * 100.0),
        ];

        // Generate patrol waypoints around each spawn
        let mut waypoints = Vec::new();
        for spawn in &spawn_points {
            let mut wp = Vec::new();
            // Create a patrol route: 4 waypoints forming a rectangle around spawn
            for (dx, dz) in [(100.0, 0.0), (0.0, 100.0), (-100.0, 0.0), (0.0, -100.0)] {
                wp.push(PirateWaypoint {
                    position: Vec2::new(spawn.x + dx * 100.0, spawn.y + dz * 100.0),
                    dwell_time: 5.0,
                });
            }
            waypoints.push(wp);
        }

        Self {
            spawn_points,
            waypoints,
            spawn_timer: Timer::from_seconds(30.0, TimerMode::Repeating),
            max_active: 6,
        }
    }
}

/// Spawn pirate ships at designated spawn points
pub fn pirate_ship_spawn_system(
    time: Res<Time>,
    mut spawns: ResMut<PirateShipSpawns>,
    commands: &mut Commands,
    active_ships: Query<Entity, With<PirateShip>>,
) {
    // Check if we need to spawn more ships
    let active_count = active_ships.iter().len();
    if active_count >= spawns.max_active {
        return;
    }

    spawns.spawn_timer.tick(time.delta());
    if !spawns.spawn_timer.just_finished() {
        return;
    }

    // Find next spawn point to use
    let spawn_idx = active_count % spawns.spawn_points.len();
    let spawn_pos = spawns.spawn_points[spawn_idx];

    // Spawn the pirate ship
    let pirate = PirateShip::default();
    commands.spawn((
        Name::new(format!("Pirate Ship {}", pirate.id)),
        pirate,
        Position::new(Vec3::new(spawn_pos.x, spawn_pos.y, 0.0)),
        FacingDirection::default(),
        BoatState::default(),
        Transform::default(),
        GlobalTransform::default(),
    ));

    log::debug!("[pirate_ship_spawn_system] Spawned pirate ship at ({}, {})", spawn_pos.x, spawn_pos.y);
}

/// AI system for pirate ship navigation and behavior
pub fn pirate_ship_ai_system(
    time: Res<Time>,
    mut pirate_query: Query<(
        &mut PirateShip,
        &mut Position,
        &mut FacingDirection,
        &mut BoatState,
    )>,
    player_query: Query<&Position, With<crate::components::PlayerCharacter>>,
) {
    for (mut pirate, mut position, mut facing, mut boat) in pirate_query.iter_mut() {
        if pirate.ai_state == PirateShipAiState::Dead {
            continue;
        }

        pirate.state_timer.tick(time.delta());
        pirate.combat_cooldown.tick(time.delta());

        match pirate.ai_state {
            PirateShipAiState::Patrol => {
                // Navigate to current waypoint
                if let Some(waypoint) = pirate.current_waypoint.min(pirate.current_waypoint).checked_add(0) {
                    // Simple patrol: move toward next waypoint
                    let target_x = position.position.x;
                    let target_y = position.position.y;
                    
                    // Move in a pattern based on time
                    let t = time.elapsed_secs();
                    let patrol_radius = 50.0 * 100.0; // 50m in cm
                    let dx = (t * 0.5 + pirate.id as f32).sin() * patrol_radius;
                    let dy = (t * 0.3 + pirate.id as f32).cos() * patrol_radius;
                    
                    let spawn_x = position.position.x;
                    let spawn_y = position.position.y;
                    
                    let move_x = (spawn_x + dx - position.position.x).min(1.0);
                    let move_y = (spawn_y + dy - position.position.y).min(1.0);
                    
                    if move_x.abs() > 0.1 || move_y.abs() > 0.1 {
                        boat.heading = move_x.atan2(move_y);
                        boat.speed = (boat.speed + 0.5 * time.delta_secs()).min(boat.max_speed * 0.5);
                    }
                }

                // Check for player boats in detection range
                for player_pos in player_query.iter() {
                    let dx = position.position.x - player_pos.position.x;
                    let dy = position.position.y - player_pos.position.y;
                    let dist = (dx * dx + dy * dy).sqrt() / 100.0; // Convert to meters

                    if dist < pirate.detection_range {
                        pirate.ai_state = PirateShipAiState::Chase;
                        pirate.in_combat = true;
                        pirate.target_entity = None; // Would be set from player entity
                        log::debug!("[pirate_ship_ai_system] Pirate {} detected player at {}m", pirate.id, dist);
                        break;
                    }
                }
            }
            PirateShipAiState::Chase => {
                // Chase player - move toward detected position
                boat.speed = (boat.speed + 1.0 * time.delta_secs()).min(boat.max_speed * 0.8);
                
                // Check if in combat range
                for player_pos in player_query.iter() {
                    let dx = position.position.x - player_pos.position.x;
                    let dy = position.position.y - player_pos.position.y;
                    let dist = (dx * dx + dy * dy).sqrt() / 100.0;

                    if dist < pirate.combat_range {
                        pirate.ai_state = PirateShipAiState::Combat;
                        break;
                    }

                    // Chase direction
                    if dist > pirate.combat_range * 1.5 {
                        // Too far, return to patrol
                        pirate.ai_state = PirateShipAiState::Patrol;
                        pirate.in_combat = false;
                        break;
                    }
                }
            }
            PirateShipAiState::Combat => {
                // Maintain combat range, fire cannons
                boat.speed = boat.max_speed * 0.4; // Slow movement during combat
                
                // Check health - flee if low
                if pirate.health < pirate.max_health * 0.25 {
                    pirate.ai_state = PirateShipAiState::Flee;
                }

                // Check if target is still in range
                let mut target_in_range = false;
                for player_pos in player_query.iter() {
                    let dx = position.position.x - player_pos.position.x;
                    let dy = position.position.y - player_pos.position.y;
                    let dist = (dx * dx + dy * dy).sqrt() / 100.0;

                    if dist < pirate.combat_range * 2.0 {
                        target_in_range = true;
                        break;
                    }
                }

                if !target_in_range {
                    pirate.ai_state = PirateShipAiState::Chase;
                }
            }
            PirateShipAiState::Flee => {
                // Flee from player
                boat.speed = boat.max_speed;
                
                // Return to patrol if far enough or health recovered
                if pirate.health > pirate.max_health * 0.5 {
                    pirate.ai_state = PirateShipAiState::Patrol;
                    pirate.in_combat = false;
                }
            }
            PirateShipAiState::Dead => {}
        }

        facing.desired = boat.heading;
    }
}

/// Combat system for pirate ships
pub fn pirate_ship_combat_system(
    time: Res<Time>,
    mut pirate_query: Query<(&mut PirateShip, &Position)>,
) {
    for (mut pirate, position) in pirate_query.iter_mut() {
        if pirate.ai_state != PirateShipAiState::Combat {
            continue;
        }

        // Fire cannon shot when cooldown is ready
        if pirate.combat_cooldown.just_finished() {
            // Would spawn cannon projectile here
            log::debug!("[pirate_ship_combat_system] Pirate {} fired cannon", pirate.id);
            
            // Reset cooldown
            pirate.combat_cooldown.reset();
        }
    }
}

/// Despawn dead pirate ships
pub fn pirate_ship_despawn_system(
    commands: &mut Commands,
    mut pirate_query: Query<(Entity, &mut PirateShip)>,
) {
    for (entity, mut pirate) in pirate_query.iter_mut() {
        if pirate.health <= 0.0 && pirate.ai_state != PirateShipAiState::Dead {
            pirate.ai_state = PirateShipAiState::Dead;
            log::debug!("[pirate_ship_despawn_system] Pirate {} destroyed", pirate.id);
        }

        if pirate.ai_state == PirateShipAiState::Dead {
            commands.entity(entity).despawn_recursive();
        }
    }
}
