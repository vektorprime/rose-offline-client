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
            spawn_interval: 0.08,      // Spawn every 80ms when running (less frequent)
            particles_per_burst: 1,    // Spawn 1 particle per burst (more subtle)
            feet_offset: 0.0,          // At feet level
            spread_radius: 0.15,       // Slightly larger spread for dust cloud effect
        }
    }
}

/// Component for individual dust/smoke particles
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
    /// Random drift direction (for wandering motion)
    pub drift_direction: Vec3,
    /// Random phase offset for vertical oscillation
    pub oscillation_phase: f32,
    /// Initial Y position (to calculate oscillation from)
    pub base_y: f32,
}

impl DirtDashParticle {
    pub fn new(
        lifetime: f32,
        velocity: Vec3,
        size: f32,
        gravity: f32,
        initial_alpha: f32,
        drift_direction: Vec3,
        oscillation_phase: f32,
        base_y: f32,
    ) -> Self {
        Self {
            age: 0.0,
            lifetime,
            velocity,
            initial_size: size,
            current_size: size,
            gravity,
            initial_alpha,
            drift_direction,
            oscillation_phase,
            base_y,
        }
    }

    /// Returns normalized age (0.0 to 1.0)
    pub fn normalized_age(&self) -> f32 {
        (self.age / self.lifetime).clamp(0.0, 1.0)
    }

    /// Returns current alpha based on age (fades out over lifetime)
    pub fn current_alpha(&self) -> f32 {
        let t = self.normalized_age();
        // Smooth fade out - start fading earlier for more subtle effect
        if t > 0.3 {
            self.initial_alpha * (1.0 - (t - 0.3) / 0.7)
        } else {
            // Fade in slightly at the start
            self.initial_alpha * (t / 0.3).min(1.0)
        }
    }
}

/// Resource for dust effect settings (smoke/fog that hovers near player)
#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct DirtDashSettings {
    /// Base color for dust particles (light gray/white for smoke effect)
    pub particle_color: Vec4,
    /// Minimum particle lifetime
    pub min_lifetime: f32,
    /// Maximum particle lifetime
    pub max_lifetime: f32,
    /// Minimum particle size
    pub min_size: f32,
    /// Maximum particle size
    pub max_size: f32,
    /// Minimum upward velocity (very low for hovering effect)
    pub min_upward_velocity: f32,
    /// Maximum upward velocity (very low for hovering effect)
    pub max_upward_velocity: f32,
    /// Horizontal velocity multiplier (minimal to stay near player)
    pub horizontal_velocity_factor: f32,
    /// Gravity applied to particles (negative = float up slightly, positive = fall)
    pub gravity: f32,
    /// Maximum number of active dust particles (performance limit)
    pub max_particles: usize,
    /// Horizontal drift speed (random wandering motion)
    pub drift_speed: f32,
    /// Vertical oscillation amplitude (for floating effect)
    pub vertical_oscillation: f32,
}

impl Default for DirtDashSettings {
    fn default() -> Self {
        Self {
            // Light gray dust/smoke color with low opacity for subtlety
            particle_color: Vec4::new(0.7, 0.68, 0.65, 0.25),
            min_lifetime: 0.1,          // Can be instant
            max_lifetime: 0.6,          // Up to 0.8 seconds
            min_size: 0.01,             // Very small particles
            max_size: 0.1,              // Can grow larger for smoke effect
            min_upward_velocity: 0.05,  // Very low upward velocity (hovering)
            max_upward_velocity: 0.15,  // Very low upward velocity (hovering)
            horizontal_velocity_factor: 0.0, // No horizontal spread - stays near player
            gravity: 0.1,               // Very low gravity for floating effect
            max_particles: 300,
            drift_speed: 0.1,           // Gentle random drift
            vertical_oscillation: 0.02, // Subtle bobbing motion
        }
    }
}
