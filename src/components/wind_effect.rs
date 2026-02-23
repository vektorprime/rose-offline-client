use bevy::prelude::*;

/// Component for individual wind particles during flight
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct WindEffectParticle {
    /// Current velocity of the particle
    pub velocity: Vec3,
    /// Lifetime timer for the particle
    pub lifetime: Timer,
    /// Initial alpha value for fading calculations
    pub initial_alpha: f32,
}

/// Component for the wind effect emitter attached to flying characters
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct WindEffectEmitter {
    /// Timer for controlling particle spawn rate
    pub spawn_timer: Timer,
}

impl Default for WindEffectEmitter {
    fn default() -> Self {
        Self {
            spawn_timer: Timer::from_seconds(0.033, TimerMode::Repeating),
        }
    }
}
