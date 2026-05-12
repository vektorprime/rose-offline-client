use bevy::prelude::*;

/// Marker for wake/spray emitters attached to sailing boats.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct WakeEmitter {
    /// Spawn interval for wake particles.
    pub spawn_timer: Timer,
    /// Maximum alive wake particles for this boat.
    pub max_particles: usize,
    /// Spawn interval for bow spray particles.
    pub spray_spawn_timer: Timer,
    /// Maximum alive spray particles for this boat.
    pub max_spray_particles: usize,
}

impl Default for WakeEmitter {
    fn default() -> Self {
        Self {
            spawn_timer: Timer::from_seconds(0.05, TimerMode::Repeating),
            max_particles: 100,
            spray_spawn_timer: Timer::from_seconds(0.08, TimerMode::Repeating),
            max_spray_particles: 30,
        }
    }
}

/// Individual water wake particle.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct WakeParticle {
    pub velocity: Vec3,
    pub lifetime: Timer,
    pub initial_alpha: f32,
    pub initial_scale: f32,
}

/// Individual bow spray particle.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct BowSprayParticle {
    pub velocity: Vec3,
    pub lifetime: Timer,
    pub initial_alpha: f32,
    pub initial_scale: f32,
}

/// Associates spawned effect particles with their source boat.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct WakeSource {
    pub boat_entity: Entity,
}
