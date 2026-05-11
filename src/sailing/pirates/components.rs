//! Components and data types for pirate ship NPCs.

use bevy::prelude::*;

/// Main component for pirate ship NPCs.
/// Attached to the ship entity and drives AI, combat, and spawning.
#[derive(Component, Debug, Clone)]
pub struct PirateShip {
    /// Unique ship identifier within the zone.
    pub ship_id: u32,

    /// Current health of the ship.
    pub health: f32,

    /// Maximum health.
    pub max_health: f32,

    /// Attack damage per cannon shot.
    pub attack: f32,

    /// Defense (damage reduction percentage, 0..1).
    pub defense: f32,

    /// Current AI state.
    pub state: PirateShipState,

    /// Index of the current waypoint in the patrol route.
    pub current_waypoint_index: usize,

    /// Entity of the player boat currently targeted (if in combat).
    pub target_entity: Option<Entity>,

    /// Time until next cannon shot (seconds).
    pub fire_cooldown: f32,

    /// How long the ship has been in its current state (seconds).
    pub state_timer: f32,

    /// Spawn position (for respawn logic).
    pub spawn_position: Vec3,

    /// Patrol waypoints for this ship.
    pub waypoints: Vec<PirateShipWaypoint>,

    /// Whether this ship is currently aggro'd on the player.
    pub is_aggro: bool,

    /// Respawn timer (seconds) after being defeated.
    pub respawn_timer: f32,

    /// Whether the ship is dead and waiting to respawn.
    pub is_dead: bool,
}

impl PirateShip {
    /// Create a new pirate ship at the given spawn position.
    pub fn new(ship_id: u32, spawn_position: Vec3, waypoints: Vec<PirateShipWaypoint>) -> Self {
        Self {
            ship_id,
            health: 200.0,
            max_health: 200.0,
            attack: 15.0,
            defense: 0.2,
            state: PirateShipState::Patrolling,
            current_waypoint_index: 0,
            target_entity: None,
            fire_cooldown: 0.0,
            state_timer: 0.0,
            spawn_position,
            waypoints,
            is_aggro: false,
            respawn_timer: 0.0,
            is_dead: false,
        }
    }
}

/// AI behavioral states for pirate ships.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PirateShipState {
    #[default]
    /// Sailing between waypoints in patrol route.
    Patrolling,

    /// Chasing the player boat.
    Chasing,

    /// Actively firing cannons at the player.
    Attacking,

    /// Retreating after low health.
    Retreating,

    /// Dead, waiting to respawn.
    Dead,
}

/// A waypoint for pirate ship patrol routes.
#[derive(Debug, Clone, Copy)]
pub struct PirateShipWaypoint {
    /// Position in game-space centimeters.
    pub position: Vec3,
    /// How long to pause at this waypoint (seconds).
    pub pause_duration: f32,
}

impl PirateShipWaypoint {
    pub fn new(position: Vec3, pause_duration: f32) -> Self {
        Self {
            position,
            pause_duration,
        }
    }
}

/// Marker component for cannon projectiles fired by pirate ships.
#[derive(Component, Debug, Clone)]
pub struct PirateCannonProjectile {
    /// Damage this projectile deals.
    pub damage: f32,

    /// Speed of the projectile in m/s.
    pub speed: f32,

    /// Source pirate ship entity.
    pub source_entity: Entity,

    /// Target entity.
    pub target_entity: Entity,

    /// Lifetime of the projectile (seconds).
    pub lifetime: f32,

    /// Current age of the projectile (seconds).
    pub age: f32,

    /// Direction of travel.
    pub direction: Vec3,

    /// Start position.
    pub start_position: Vec3,
}

impl PirateCannonProjectile {
    pub fn new(
        source_entity: Entity,
        target_entity: Entity,
        damage: f32,
        speed: f32,
        start_position: Vec3,
        direction: Vec3,
    ) -> Self {
        Self {
            damage,
            speed,
            source_entity,
            target_entity,
            lifetime: 5.0,
            age: 0.0,
            direction,
            start_position,
        }
    }
}

/// Global settings for pirate ship spawning and behavior in sailing zones.
#[derive(Resource, Debug, Clone)]
pub struct PirateShipSettings {
    /// Maximum number of pirate ships active at once.
    pub max_ships: usize,

    /// Detection range for player boats (meters).
    pub detection_range_m: f32,

    /// Attack range for cannon fire (meters).
    pub attack_range_m: f32,

    /// Retreat threshold (health percentage).
    pub retreat_threshold: f32,

    /// Cooldown between cannon shots (seconds).
    pub fire_cooldown: f32,

    /// Projectile speed (m/s).
    pub projectile_speed: f32,

    /// Respawn time after defeat (seconds).
    pub respawn_time: f32,

    /// Patrol speed multiplier relative to wind speed.
    pub patrol_speed_multiplier: f32,

    /// Chase speed multiplier.
    pub chase_speed_multiplier: f32,

    /// Whether pirate ships are enabled.
    pub enabled: bool,
}

impl Default for PirateShipSettings {
    fn default() -> Self {
        Self {
            max_ships: 6,
            detection_range_m: 500.0,
            attack_range_m: 200.0,
            retreat_threshold: 0.25,
            fire_cooldown: 3.0,
            projectile_speed: 30.0,
            respawn_time: 60.0,
            patrol_speed_multiplier: 0.6,
            chase_speed_multiplier: 1.0,
            enabled: true,
        }
    }
}

/// Marker for pirate spawn points in the sailing zone.
#[derive(Component, Debug, Clone)]
pub struct PirateSpawnPoint {
    /// Spawn position in game-space centimeters.
    pub position: Vec3,
    /// Index of this spawn point.
    pub index: u32,
}

impl PirateSpawnPoint {
    pub fn new(position: Vec3, index: u32) -> Self {
        Self { position, index }
    }
}
