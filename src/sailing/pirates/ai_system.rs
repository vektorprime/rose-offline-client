//! Pirate ship AI navigation system.
//!
//! Handles waypoint-based patrol routing, player detection, and state transitions
//! for pirate ship NPCs in the sailing zone.

use bevy::prelude::*;

use crate::components::{BoatState, PlayerCharacter, Position};
use crate::resources::WindState;

use super::components::{PirateShip, PirateShipState, PirateShipSettings};

/// Minimum distance to a waypoint to consider it "reached" (in centimeters).
const WAYPOINT_REACH_DISTANCE_CM: f32 = 500.0;

/// System that updates pirate ship AI: navigation, detection, state transitions.
pub fn pirate_ship_ai_system(
    time: Res<Time>,
    wind: Res<WindState>,
    settings: Res<PirateShipSettings>,
    mut pirate_query: Query<(&mut PirateShip, &mut BoatState, &mut Position)>,
    player_query: Query<(&Position, &BoatState), With<PlayerCharacter>>,
) {
    let dt = time.delta_secs();

    // Find active player boat position
    let player_position = player_query.iter().find_map(|(pos, boat)| {
        if boat.active {
            Some(pos.position)
        } else {
            None
        }
    });

    for (mut pirate, mut boat, mut position) in pirate_query.iter_mut() {
        // Skip dead ships
        if pirate.is_dead {
            continue;
        }

        pirate.state_timer += dt;

        // ─── Player detection ──────────────────────────────────────────
        if let Some(player_pos) = player_position {
            let dx = position.position.x - player_pos.x;
            let dy = position.position.y - player_pos.y;
            let distance_m = (dx * dx + dy * dy).sqrt() / 100.0;

            if distance_m <= settings.detection_range_m && !pirate.is_aggro {
                // Player detected - enter chase state
                pirate.state = PirateShipState::Chasing;
                pirate.is_aggro = true;
                pirate.target_entity = find_player_entity();
                pirate.state_timer = 0.0;
                log::info!(
                    "[PIRATE] Ship #{} detected player at {:.1}m, entering chase",
                    pirate.ship_id,
                    distance_m
                );
            }

            // Check if player is out of range while aggro
            if pirate.is_aggro && distance_m > settings.detection_range_m * 1.5 {
                pirate.state = PirateShipState::Patrolling;
                pirate.is_aggro = false;
                pirate.target_entity = None;
                pirate.state_timer = 0.0;
                log::info!(
                    "[PIRATE] Ship #{} player out of range, returning to patrol",
                    pirate.ship_id
                );
            }
        }

        // ─── State machine ─────────────────────────────────────────────
        match pirate.state {
            PirateShipState::Patrolling => update_patrol(&mut pirate, &mut boat, &position, &wind, dt, &settings),
            PirateShipState::Chasing => update_chasing(&mut pirate, &mut boat, &position, player_position, &wind, dt, &settings),
            PirateShipState::Attacking => update_attacking(&mut pirate, &mut boat, &position, player_position, &wind, dt, &settings),
            PirateShipState::Retreating => update_retreating(&mut pirate, &mut boat, &position, player_position, &wind, dt, &settings),
            PirateShipState::Dead => {} // Handled by cleanup system
        }

        // ─── Health-based state transitions ────────────────────────────
        if !pirate.is_dead && pirate.health > 0.0 {
            let health_pct = pirate.health / pirate.max_health;
            if health_pct <= settings.retreat_threshold && pirate.state != PirateShipState::Retreating {
                pirate.state = PirateShipState::Retreating;
                pirate.state_timer = 0.0;
                log::info!(
                    "[PIRATE] Ship #{} health low ({:.0}%), retreating",
                    pirate.ship_id,
                    health_pct * 100.0
                );
            }
        }

        // ─── Sync boat heading to position facing ──────────────────────
        position.position.z = boat.water_height_cm;
    }
}

/// Update patrol behavior: sail to next waypoint.
fn update_patrol(
    pirate: &mut PirateShip,
    boat: &mut BoatState,
    position: &Position,
    wind: &WindState,
    dt: f32,
    settings: &PirateShipSettings,
) {
    if pirate.waypoints.is_empty() {
        return;
    }

    let current_wp = &pirate.waypoints[pirate.current_waypoint_index];

    // Check if we reached the waypoint
    let dx = position.position.x - current_wp.position.x;
    let dy = position.position.y - current_wp.position.y;
    let distance_cm = (dx * dx + dy * dy).sqrt();

    if distance_cm < WAYPOINT_REACH_DISTANCE_CM {
        // Reached waypoint - pause then move to next
        if pirate.state_timer >= current_wp.pause_duration {
            pirate.current_waypoint_index = (pirate.current_waypoint_index + 1) % pirate.waypoints.len();
            pirate.state_timer = 0.0;
        }
        // Slow down near waypoint
        boat.speed = boat.speed * 0.95;
    } else {
        // Navigate toward waypoint
        let target_heading = calculate_heading(position.position, current_wp.position);
        steer_toward_heading(boat, target_heading, dt);

        // Apply wind-based speed
        let speed_factor = sail_speed_factor(boat.heading, wind.angle);
        let target_speed = boat.max_speed * settings.patrol_speed_multiplier * speed_factor;
        boat.speed += (target_speed - boat.speed) * 2.0 * dt;
        boat.speed = boat.speed.clamp(0.0, boat.max_speed);
    }

    // Move the ship
    move_ship(boat, position, dt);
}

/// Update chasing behavior: pursue the player boat.
fn update_chasing(
    pirate: &mut PirateShip,
    boat: &mut BoatState,
    position: &Position,
    player_position: Option<Vec3>,
    wind: &WindState,
    dt: f32,
    settings: &PirateShipSettings,
) {
    let Some(player_pos) = player_position else {
        // Lost player - return to patrol
        pirate.state = PirateShipState::Patrolling;
        pirate.is_aggro = false;
        pirate.target_entity = None;
        return;
    };

    let dx = position.position.x - player_pos.x;
    let dy = position.position.y - player_pos.y;
    let distance_m = (dx * dx + dy * dy).sqrt() / 100.0;

    // If in attack range, switch to attacking state
    if distance_m <= settings.attack_range_m {
        pirate.state = PirateShipState::Attacking;
        pirate.state_timer = 0.0;
        return;
    }

    // Navigate toward player
    let target_heading = calculate_heading(position.position, player_pos);
    steer_toward_heading(boat, target_heading, dt);

    // Chase speed
    let speed_factor = sail_speed_factor(boat.heading, wind.angle);
    let target_speed = boat.max_speed * settings.chase_speed_multiplier * speed_factor;
    boat.speed += (target_speed - boat.speed) * 3.0 * dt;
    boat.speed = boat.speed.clamp(0.0, boat.max_speed);

    move_ship(boat, position, dt);
}

/// Update attacking behavior: maintain position and fire cannons.
fn update_attacking(
    pirate: &mut PirateShip,
    boat: &mut BoatState,
    position: &Position,
    player_position: Option<Vec3>,
    wind: &WindState,
    dt: f32,
    settings: &PirateShipSettings,
) {
    let Some(player_pos) = player_position else {
        pirate.state = PirateShipState::Patrolling;
        pirate.is_aggro = false;
        pirate.target_entity = None;
        return;
    };

    let dx = position.position.x - player_pos.x;
    let dy = position.position.y - player_pos.y;
    let distance_m = (dx * dx + dy * dy).sqrt() / 100.0;

    // If player moved out of attack range, chase them
    if distance_m > settings.attack_range_m * 1.2 {
        pirate.state = PirateShipState::Chasing;
        pirate.state_timer = 0.0;
        return;
    }

    // Face the player but maintain position
    let target_heading = calculate_heading(position.position, player_pos);
    steer_toward_heading(boat, target_heading, dt);

    // Slow speed while attacking (maintain firing position)
    let speed_factor = sail_speed_factor(boat.heading, wind.angle);
    let target_speed = boat.max_speed * 0.3 * speed_factor;
    boat.speed += (target_speed - boat.speed) * 2.0 * dt;
    boat.speed = boat.speed.clamp(0.0, boat.max_speed * 0.5);

    // Update fire cooldown
    pirate.fire_cooldown -= dt;
    if pirate.fire_cooldown <= 0.0 {
        pirate.fire_cooldown = settings.fire_cooldown;
        // Signal that ship should fire (handled by combat system)
    }

    move_ship(boat, position, dt);
}

/// Update retreating behavior: sail away from player.
fn update_retreating(
    pirate: &mut PirateShip,
    boat: &mut BoatState,
    position: &Position,
    player_position: Option<Vec3>,
    wind: &WindState,
    dt: f32,
    settings: &PirateShipSettings,
) {
    // Navigate away from player or back to spawn point
    let flee_target = player_position.map(|pp| {
        // Flee in opposite direction from player
        let dx = position.position.x - pp.x;
        let dy = position.position.y - pp.y;
        let dist = (dx * dx + dy * dy).sqrt().max(1.0);
        Vec3::new(
            position.position.x + dx / dist * 200000.0, // 2km away
            position.position.y + dy / dist * 200000.0,
            position.position.z,
        )
    }).unwrap_or(pirate.spawn_position);

    let target_heading = calculate_heading(position.position, flee_target);
    steer_toward_heading(boat, target_heading, dt);

    // Full speed retreat
    let speed_factor = sail_speed_factor(boat.heading, wind.angle);
    let target_speed = boat.max_speed * speed_factor;
    boat.speed += (target_speed - boat.speed) * 4.0 * dt;
    boat.speed = boat.speed.clamp(0.0, boat.max_speed);

    move_ship(boat, position, dt);

    // Check if far enough from player to stop retreating
    if let Some(player_pos) = player_position {
        let dx = position.position.x - player_pos.x;
        let dy = position.position.y - player_pos.y;
        let distance_m = (dx * dx + dy * dy).sqrt() / 100.0;

        if distance_m > settings.detection_range_m * 2.0 {
            pirate.state = PirateShipState::Patrolling;
            pirate.is_aggro = false;
            pirate.target_entity = None;
            pirate.current_waypoint_index = 0;
            pirate.state_timer = 0.0;
            log::info!(
                "[PIRATE] Ship #{} retreated far enough, resuming patrol",
                pirate.ship_id
            );
        }
    }
}

/// Calculate heading angle from one position to another.
fn calculate_heading(from: Vec3, to: Vec3) -> f32 {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    dy.atan2(dx)
}

/// Steer the boat toward a target heading.
fn steer_toward_heading(boat: &mut BoatState, target_heading: f32, dt: f32) {
    let mut diff = target_heading - boat.heading;
    // Normalize to -PI..PI
    while diff > std::f32::consts::PI {
        diff -= std::f32::consts::TAU;
    }
    while diff < -std::f32::consts::PI {
        diff += std::f32::consts::TAU;
    }

    let turn_rate = 1.5;
    let max_turn = turn_rate * dt;
    let turn = diff.clamp(-max_turn, max_turn);
    boat.heading += turn;
    boat.heading = boat.heading.rem_euclid(std::f32::consts::TAU);

    boat.rudder = if diff.abs() > 0.01 {
        diff.signum()
    } else {
        0.0
    };
}

/// Calculate sail speed factor based on angle to wind.
fn sail_speed_factor(heading: f32, wind_angle: f32) -> f32 {
    let mut angle_to_wind = (heading - wind_angle).rem_euclid(std::f32::consts::TAU);
    if angle_to_wind > std::f32::consts::PI {
        angle_to_wind = std::f32::consts::TAU - angle_to_wind;
    }

    if angle_to_wind < 0.78 {
        (angle_to_wind / 0.78).powf(2.0) * 0.3
    } else if angle_to_wind < 1.57 {
        let t = (angle_to_wind - 0.78) / (1.57 - 0.78);
        0.3 + t * 0.7
    } else if angle_to_wind < 2.36 {
        let t = (angle_to_wind - 1.57) / (2.36 - 1.57);
        1.0 - t * 0.2
    } else {
        let t = (angle_to_wind - 2.36) / (std::f32::consts::PI - 2.36);
        0.8 - t * 0.3
    }
}

/// Move the ship based on current heading and speed.
fn move_ship(boat: &BoatState, position: &mut Position, dt: f32) {
    let forward = Vec3::new(boat.heading.sin(), boat.heading.cos(), 0.0);
    let movement = forward * boat.speed * dt * 100.0; // Convert m/s to cm/s
    position.position.x += movement.x;
    position.position.y += movement.y;
}

/// Find the player entity with an active boat.
fn find_player_entity() -> Option<Entity> {
    // This is a placeholder - in the full implementation, we'd query the world
    // For now, the combat system handles target resolution
    None
}
