use bevy::prelude::*;

/// Component marker for entities that should produce dirt/dash effects when moving.
/// Attach this to characters that should spawn dirt particles when running.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct DirtDashEffect {
    /// Minimum speed (units per second) to trigger the effect
    pub min_speed: f32,
    /// Time accumulator for particle spawning
    pub spawn_timer: f32,
    /// Interval between particle spawns when running
    pub spawn_interval: f32,
    /// Number of particles to spawn per burst
    pub particles_per_burst: usize,
    /// Vertical offset from entity position to spawn particles (feet level)
    pub feet_offset: f32,
    /// Random spread radius for particle spawn position
    pub spread_radius: f32,
}

impl Default for DirtDashEffect {
    fn default() -> Self {
        Self {
            min_speed: 100.0,          // Minimum speed to trigger effect
            spawn_timer: 0.0,
            spawn_interval: 0.05,      // Spawn every 50ms when running
            particles_per_burst: 2,    // Spawn 2 particles per burst
            feet_offset: 0.0,          // At feet level
            spread_radius: 10.0,       // Small spread around feet
        }
    }
}

/// Component for individual dirt/dash particles
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct DirtDashParticle {
    /// Current age of the particle in seconds
    pub age: f32,
    /// Total lifetime of the particle in seconds
    pub lifetime: f32,
    /// Current velocity of the particle
    pub velocity: Vec3,
    /// Initial size of the particle
    pub initial_size: f32,
    /// Current size (interpolated over lifetime)
    pub current_size: f32,
    /// Gravity applied to the particle
    pub gravity: f32,
    /// Initial opacity
    pub initial_alpha: f32,
}

impl DirtDashParticle {
    pub fn new(
        lifetime: f32,
        velocity: Vec3,
        size: f32,
        gravity: f32,
        initial_alpha: f32,
    ) -> Self {
        Self {
            age: 0.0,
            lifetime,
            velocity,
            initial_size: size,
            current_size: size,
            gravity,
            initial_alpha,
        }
    }

    /// Returns normalized age (0.0 to 1.0)
    pub fn normalized_age(&self) -> f32 {
        (self.age / self.lifetime).clamp(0.0, 1.0)
    }

    /// Returns current alpha based on age (fades out over lifetime)
    pub fn current_alpha(&self) -> f32 {
        let t = self.normalized_age();
        // Fade out in the second half of lifetime
        if t > 0.5 {
            self.initial_alpha * (1.0 - (t - 0.5) * 2.0)
        } else {
            self.initial_alpha
        }
    }
}

/// Resource for dirt dash effect settings
#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct DirtDashSettings {
    /// Base color for dirt particles (brownish)
    pub particle_color: Vec4,
    /// Minimum particle lifetime
    pub min_lifetime: f32,
    /// Maximum particle lifetime
    pub max_lifetime: f32,
    /// Minimum particle size
    pub min_size: f32,
    /// Maximum particle size
    pub max_size: f32,
    /// Minimum upward velocity
    pub min_upward_velocity: f32,
    /// Maximum upward velocity
    pub max_upward_velocity: f32,
    /// Horizontal velocity multiplier (based on character speed)
    pub horizontal_velocity_factor: f32,
    /// Gravity applied to particles
    pub gravity: f32,
    /// Maximum number of active dirt particles (performance limit)
    pub max_particles: usize,
}

impl Default for DirtDashSettings {
    fn default() -> Self {
        Self {
            // Brownish dirt color with some transparency
            particle_color: Vec4::new(0.45, 0.35, 0.25, 0.8),
            min_lifetime: 0.2,
            max_lifetime: 0.4,
            min_size: 3.0,
            max_size: 8.0,
            min_upward_velocity: 20.0,
            max_upward_velocity: 50.0,
            horizontal_velocity_factor: 0.3,
            gravity: 150.0,
            max_particles: 500,
        }
    }
}
