//! Components and data types for sea creatures: Kraken, Sharks, and Whales.
//!
//! Each creature type has its own component, state machine, and behavioral settings.
//! All creatures share common patterns with the pirate ship system for consistency.

use bevy::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════
// KRAKEN
// ═══════════════════════════════════════════════════════════════════════════

/// Main component for Kraken boss creatures.
/// The Kraken is a rare, large boss that emerges from deep water with multiple tentacles.
#[derive(Component, Debug, Clone)]
pub struct Kraken {
    /// Unique creature identifier within the zone.
    pub creature_id: u32,

    /// Current health.
    pub health: f32,

    /// Maximum health (massive pool for boss).
    pub max_health: f32,

    /// Attack damage per tentacle strike.
    pub attack: f32,

    /// Defense (damage reduction percentage, 0..1).
    pub defense: f32,

    /// Current AI state.
    pub state: KrakenState,

    /// Entity of the player boat currently targeted.
    pub target_entity: Option<Entity>,

    /// Time until next attack (seconds).
    pub attack_cooldown: f32,

    /// How long the Kraken has been in its current state (seconds).
    pub state_timer: f32,

    /// Spawn position (for respawn logic).
    pub spawn_position: Vec3,

    /// Whether the Kraken is currently aggro'd on the player.
    pub is_aggro: bool,

    /// Respawn timer (seconds) after being defeated.
    pub respawn_timer: f32,

    /// Whether the Kraken is dead and waiting to respawn.
    pub is_dead: bool,

    /// Number of active tentacles.
    pub tentacle_count: usize,

    /// Tentacle grab cooldown per tentacle (seconds).
    pub tentacle_grab_cooldown: f32,

    /// Current emergence phase (0.0 = fully submerged, 1.0 = fully emerged).
    pub emergence_phase: f32,

    /// Speed of emergence/emersion animation.
    pub emergence_speed: f32,
}

impl Kraken {
    /// Create a new Kraken at the given spawn position.
    pub fn new(creature_id: u32, spawn_position: Vec3) -> Self {
        Self {
            creature_id,
            health: 5000.0,
            max_health: 5000.0,
            attack: 80.0,
            defense: 0.4,
            state: KrakenState::Submerged,
            target_entity: None,
            attack_cooldown: 0.0,
            state_timer: 0.0,
            spawn_position,
            is_aggro: false,
            respawn_timer: 0.0,
            is_dead: false,
            tentacle_count: 8,
            tentacle_grab_cooldown: 0.0,
            emergence_phase: 0.0,
            emergence_speed: 0.15,
        }
    }
}

/// AI behavioral states for Kraken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KrakenState {
    #[default]
    /// Hidden beneath the waves, waiting for prey.
    Submerged,

    /// Rising to the surface with a dramatic emergence.
    Emerging,

    /// Fully visible, actively attacking boats with tentacles.
    Attacking,

    /// Grabbing a boat with tentacles, dragging it.
    Grabbing,

    /// Retreating back into the depths after low health.
    Retreating,

    /// Sinking back underwater after retreating.
    Submerging,

    /// Dead, waiting to respawn.
    Dead,
}

/// Marker component for Kraken tentacles.
#[derive(Component, Debug, Clone)]
pub struct KrakenTentacle {
    /// Index of this tentacle (0..tentacle_count-1).
    pub index: usize,

    /// Current grab target entity (if grabbing a boat).
    pub grab_target: Option<Entity>,

    /// Time until this tentacle can grab again (seconds).
    pub cooldown: f32,

    /// Current angle offset from the Kraken body.
    pub angle_offset: f32,

    /// Length of this tentacle.
    pub length: f32,

    /// Whether this tentacle is currently extended.
    pub is_extended: bool,
}

impl KrakenTentacle {
    pub fn new(index: usize, angle_offset: f32, length: f32) -> Self {
        Self {
            index,
            grab_target: None,
            cooldown: 0.0,
            angle_offset,
            length,
            is_extended: false,
        }
    }
}

/// Projectile fired by Kraken (ink blast or water jet).
#[derive(Component, Debug, Clone)]
pub struct KrakenProjectile {
    /// Damage this projectile deals.
    pub damage: f32,

    /// Speed of the projectile in m/s.
    pub speed: f32,

    /// Source Kraken entity.
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

    /// Whether this is an ink blast (slows target) or water jet (pure damage).
    pub projectile_type: KrakenProjectileType,
}

impl KrakenProjectile {
    pub fn new(
        source_entity: Entity,
        target_entity: Entity,
        damage: f32,
        speed: f32,
        start_position: Vec3,
        direction: Vec3,
        projectile_type: KrakenProjectileType,
    ) -> Self {
        Self {
            damage,
            speed,
            source_entity,
            target_entity,
            lifetime: 4.0,
            age: 0.0,
            direction,
            start_position,
            projectile_type,
        }
    }
}

/// Types of Kraken projectiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KrakenProjectileType {
    /// Ink blast - deals damage and slows the target.
    InkBlast,

    /// Water jet - high damage, no secondary effect.
    WaterJet,
}

// ═══════════════════════════════════════════════════════════════════════════
// SHARKS
// ═══════════════════════════════════════════════════════════════════════════

/// Main component for Shark creatures.
/// Sharks are fast, aggressive predators that attack boats in groups.
#[derive(Component, Debug, Clone)]
pub struct Shark {
    /// Unique creature identifier within the zone.
    pub creature_id: u32,

    /// Current health.
    pub health: f32,

    /// Maximum health.
    pub max_health: f32,

    /// Bite damage to boat hull.
    pub attack: f32,

    /// Defense (damage reduction percentage, 0..1).
    pub defense: f32,

    /// Current AI state.
    pub state: SharkState,

    /// Entity of the player boat currently targeted.
    pub target_entity: Option<Entity>,

    /// Time until next bite attack (seconds).
    pub attack_cooldown: f32,

    /// How long the shark has been in its current state (seconds).
    pub state_timer: f32,

    /// Spawn position (for respawn logic).
    pub spawn_position: Vec3,

    /// Whether the shark is currently aggro'd on the player.
    pub is_aggro: bool,

    /// Respawn timer (seconds) after being defeated.
    pub respawn_timer: f32,

    /// Whether the shark is dead and waiting to respawn.
    pub is_dead: bool,

    /// Swim speed multiplier (individual variation).
    pub speed_multiplier: f32,

    /// Pack ID - sharks with the same pack_id coordinate their attacks.
    pub pack_id: u32,

    /// Role within the pack.
    pub pack_role: SharkPackRole,
}

impl Shark {
    /// Create a new shark at the given spawn position.
    pub fn new(creature_id: u32, spawn_position: Vec3, pack_id: u32, pack_role: SharkPackRole) -> Self {
        Self {
            creature_id,
            health: 150.0,
            max_health: 150.0,
            attack: 25.0,
            defense: 0.1,
            state: SharkState::Cruising,
            target_entity: None,
            attack_cooldown: 0.0,
            state_timer: 0.0,
            spawn_position,
            is_aggro: false,
            respawn_timer: 0.0,
            is_dead: false,
            speed_multiplier: 0.9 + (creature_id as f32 * 0.02),
            pack_id,
            pack_role,
        }
    }
}

/// AI behavioral states for Sharks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SharkState {
    #[default]
    /// Swimming in a relaxed cruising pattern.
    Cruising,

    /// Detected prey, swimming toward it aggressively.
    Hunting,

    /// In biting range, attacking the boat hull.
    Biting,

    /// Retreating after taking significant damage.
    Fleeing,

    /// Dead, waiting to respawn.
    Dead,
}

/// Role of a shark within its pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharkPackRole {
    /// Alpha shark - leads the pack, first to attack.
    Alpha,

    /// Flanker - approaches from the sides.
    Flanker,

    /// Harasser - keeps pressure on from range.
    Harasser,

    /// Ambusher - waits and strikes when the boat is distracted.
    Ambusher,
}

// ═══════════════════════════════════════════════════════════════════════════
// WHALES
// ═══════════════════════════════════════════════════════════════════════════

/// Main component for Whale creatures.
/// Whales are massive, peaceful creatures that roam the ocean.
#[derive(Component, Debug, Clone)]
pub struct Whale {
    /// Unique creature identifier within the zone.
    pub creature_id: u32,

    /// Current health.
    pub health: f32,

    /// Maximum health (very high due to size).
    pub max_health: f32,

    /// Attack damage (only when provoked).
    pub attack: f32,

    /// Defense (very high due to thick skin).
    pub defense: f32,

    /// Current AI state.
    pub state: WhaleState,

    /// Entity of the player boat currently targeted (only if provoked).
    pub target_entity: Option<Entity>,

    /// Time until next defensive action (seconds).
    pub attack_cooldown: f32,

    /// How long the whale has been in its current state (seconds).
    pub state_timer: f32,

    /// Spawn position (for respawn logic).
    pub spawn_position: Vec3,

    /// Whether the whale has been provoked (becomes aggressive).
    pub is_provoked: bool,

    /// Respawn timer (seconds) after being defeated.
    pub respawn_timer: f32,

    /// Whether the whale is dead and waiting to respawn.
    pub is_dead: bool,

    /// Current waypoint index for migration route.
    pub current_waypoint_index: usize,

    /// Migration waypoints.
    pub waypoints: Vec<WhaleWaypoint>,

    /// Breathing timer - time since last surface breath (seconds).
    pub breath_timer: f32,

    /// Time between breaths (seconds).
    pub breath_interval: f32,

    /// Whether the whale is currently at the surface breathing.
    pub is_breathing: bool,

    /// Dive depth (0.0 = surface, 1.0 = deep).
    pub dive_depth: f32,

    /// Whether the whale can be ridden.
    pub can_be_ridden: bool,

    /// Entity of the rider (if being ridden).
    pub rider_entity: Option<Entity>,

    /// Whale size variant.
    pub size_variant: WhaleSizeVariant,
}

impl Whale {
    /// Create a new whale at the given spawn position.
    pub fn new(creature_id: u32, spawn_position: Vec3, waypoints: Vec<WhaleWaypoint>) -> Self {
        Self {
            creature_id,
            health: 10000.0,
            max_health: 10000.0,
            attack: 120.0,
            defense: 0.6,
            state: WhaleState::Migrating,
            target_entity: None,
            attack_cooldown: 0.0,
            state_timer: 0.0,
            spawn_position,
            is_provoked: false,
            respawn_timer: 0.0,
            is_dead: false,
            current_waypoint_index: 0,
            waypoints,
            breath_timer: 0.0,
            breath_interval: 15.0,
            is_breathing: false,
            dive_depth: 0.0,
            can_be_ridden: true,
            rider_entity: None,
            size_variant: WhaleSizeVariant::Blue,
        }
    }
}

/// AI behavioral states for Whales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhaleState {
    #[default]
    /// Following migration route peacefully.
    Migrating,

    /// Surfacing to breathe.
    Breathing,

    /// Diving deep underwater.
    Diving,

    /// Spouting water (breath display).
    Spouting,

    /// Defending itself after being attacked.
    Defending,

    /// Fleeing from a threat.
    Fleeing,

    /// Dead, waiting to respawn.
    Dead,
}

/// A waypoint for whale migration routes.
#[derive(Debug, Clone, Copy)]
pub struct WhaleWaypoint {
    /// Position in game-space centimeters.
    pub position: Vec3,

    /// Depth at this waypoint (0.0 = surface, 1.0 = deep).
    pub depth: f32,

    /// How long to pause at this waypoint (seconds).
    pub pause_duration: f32,
}

impl WhaleWaypoint {
    pub fn new(position: Vec3, depth: f32, pause_duration: f32) -> Self {
        Self {
            position,
            depth,
            pause_duration,
        }
    }
}

/// Size variants for whales.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhaleSizeVariant {
    /// Blue whale - largest, slowest.
    Blue,

    /// Humpback whale - medium, more active.
    Humpback,

    /// Sperm whale - smaller, more aggressive when provoked.
    Sperm,
}

// ═══════════════════════════════════════════════════════════════════════════
// SPAWN POINTS
// ═══════════════════════════════════════════════════════════════════════════

/// Marker for Kraken spawn points in the sailing zone (rare, 1 per zone).
#[derive(Component, Debug, Clone)]
pub struct KrakenSpawnPoint {
    /// Spawn position in game-space centimeters.
    pub position: Vec3,

    /// Index of this spawn point.
    pub index: u32,
}

impl KrakenSpawnPoint {
    pub fn new(position: Vec3, index: u32) -> Self {
        Self { position, index }
    }
}

/// Marker for Shark spawn points in the sailing zone (common).
#[derive(Component, Debug, Clone)]
pub struct SharkSpawnPoint {
    /// Spawn position in game-space centimeters.
    pub position: Vec3,

    /// Index of this spawn point.
    pub index: u32,

    /// Number of sharks to spawn at this point.
    pub pack_size: usize,
}

impl SharkSpawnPoint {
    pub fn new(position: Vec3, index: u32, pack_size: usize) -> Self {
        Self { position, index, pack_size }
    }
}

/// Marker for Whale spawn points in the sailing zone (rare).
#[derive(Component, Debug, Clone)]
pub struct WhaleSpawnPoint {
    /// Spawn position in game-space centimeters.
    pub position: Vec3,

    /// Index of this spawn point.
    pub index: u32,
}

impl WhaleSpawnPoint {
    pub fn new(position: Vec3, index: u32) -> Self {
        Self { position, index }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SETTINGS
// ═══════════════════════════════════════════════════════════════════════════

/// Global settings for sea creature spawning and behavior in sailing zones.
#[derive(Resource, Debug, Clone)]
pub struct SeaCreatureSettings {
    // ── Kraken settings ──
    /// Maximum number of Krakens active at once (typically 1).
    pub max_krakens: usize,

    /// Detection range for Kraken to notice player boats (meters).
    pub kraken_detection_range_m: f32,

    /// Attack range for Kraken tentacles (meters).
    pub kraken_attack_range_m: f32,

    /// Kraken retreat threshold (health percentage).
    pub kraken_retreat_threshold: f32,

    /// Kraken attack cooldown (seconds).
    pub kraken_attack_cooldown: f32,

    /// Kraken respawn time after defeat (seconds).
    pub kraken_respawn_time: f32,

    // ── Shark settings ──
    /// Maximum number of sharks active at once.
    pub max_sharks: usize,

    /// Detection range for sharks (meters).
    pub shark_detection_range_m: f32,

    /// Attack range for shark bites (meters).
    pub shark_attack_range_m: f32,

    /// Shark retreat threshold (health percentage).
    pub shark_retreat_threshold: f32,

    /// Shark attack cooldown (seconds).
    pub shark_attack_cooldown: f32,

    /// Shark respawn time after defeat (seconds).
    pub shark_respawn_time: f32,

    /// Number of sharks per pack.
    pub shark_pack_size: usize,

    // ── Whale settings ──
    /// Maximum number of whales active at once.
    pub max_whales: usize,

    /// Whale migration speed multiplier.
    pub whale_migration_speed_multiplier: f32,

    /// Whale provoked aggression range (meters).
    pub whale_provoked_range_m: f32,

    /// Whale respawn time after defeat (seconds).
    pub whale_respawn_time: f32,

    /// Whether whales can be ridden.
    pub whales_ridable: bool,

    // ── General ──
    /// Whether sea creatures are enabled.
    pub enabled: bool,
}

impl Default for SeaCreatureSettings {
    fn default() -> Self {
        Self {
            // Kraken
            max_krakens: 1,
            kraken_detection_range_m: 600.0,
            kraken_attack_range_m: 250.0,
            kraken_retreat_threshold: 0.15,
            kraken_attack_cooldown: 4.0,
            kraken_respawn_time: 300.0,

            // Sharks
            max_sharks: 12,
            shark_detection_range_m: 300.0,
            shark_attack_range_m: 15.0,
            shark_retreat_threshold: 0.2,
            shark_attack_cooldown: 2.0,
            shark_respawn_time: 45.0,
            shark_pack_size: 4,

            // Whales
            max_whales: 2,
            whale_migration_speed_multiplier: 0.4,
            whale_provoked_range_m: 150.0,
            whale_respawn_time: 600.0,
            whales_ridable: true,

            // General
            enabled: true,
        }
    }
}
