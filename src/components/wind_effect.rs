use bevy::prelude::*;

/// Resource for global wind sway settings that can be tweaked at runtime
#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource, Default)]
pub struct WindSwaySettings {
    /// Whether wind sway is enabled
    pub enabled: bool,
    /// Global wind intensity multiplier (affects all vegetation)
    pub global_intensity: f32,
    /// Speed multiplier for grass sway
    pub grass_speed: f32,
    /// Amplitude multiplier for grass sway (radians)
    pub grass_amplitude: f32,
    /// Speed multiplier for tree leaf sway
    pub tree_speed: f32,
    /// Amplitude multiplier for tree leaf sway (radians)
    pub tree_amplitude: f32,
    /// Debug: Log count of entities with WindSway (for troubleshooting)
    pub debug_log_count: bool,
}

impl Default for WindSwaySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            global_intensity: 0.1,
            grass_speed: 2.0,
            grass_amplitude: 0.2,  // ~11 degrees - clearly visible
            tree_speed: 1.5,
            tree_amplitude: 0.15, // ~8 degrees - visible but gentler
            debug_log_count: false,
        }
    }
}

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

/// Component for vegetation wind sway effect (grass, leaves, etc.)
/// Applies a subtle swaying motion to simulate wind
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct WindSway {
    /// Phase offset to desynchronize multiple objects
    pub phase_offset: f32,
    /// Axis to rotate around (normalized)
    pub sway_axis: Vec3,
    /// Whether this is grass (sways from base) or tree leaves (sways from branch)
    pub is_grass: bool,
    /// Original rotation to apply sway relative to
    pub base_rotation: Quat,
}

impl Default for WindSway {
    fn default() -> Self {
        Self {
            phase_offset: 0.0,
            sway_axis: Vec3::X,
            is_grass: true,
            base_rotation: Quat::IDENTITY,
        }
    }
}

impl WindSway {
    /// Create a new WindSway for grass
    pub fn for_grass() -> Self {
        Self {
            phase_offset: 0.0,
            sway_axis: Vec3::X,
            is_grass: true,
            base_rotation: Quat::IDENTITY,
        }
    }

    /// Create a new WindSway for tree leaves
    pub fn for_tree_leaves() -> Self {
        Self {
            phase_offset: 0.0,
            sway_axis: Vec3::X,
            is_grass: false,
            base_rotation: Quat::IDENTITY,
        }
    }

    /// Set the phase offset (useful for randomizing appearance)
    pub fn with_phase_offset(mut self, offset: f32) -> Self {
        self.phase_offset = offset;
        self
    }

    /// Set the base rotation
    pub fn with_base_rotation(mut self, rotation: Quat) -> Self {
        self.base_rotation = rotation;
        self
    }
}

/// System that applies wind sway to vegetation
/// Uses a combination of sine waves for natural-looking movement
pub fn wind_sway_system(
    time: Res<Time>,
    settings: Res<WindSwaySettings>,
    mut query: Query<(&WindSway, &mut Transform)>,
) {
    // Early return if disabled
    if !settings.enabled {
        return;
    }
    
    // Debug logging if enabled
    if settings.debug_log_count {
        let count = query.iter().len();
        log::info!("[WIND SWAY] {} entities with WindSway component", count);
    }
    
    let time_seconds = time.elapsed_secs();
    
    for (wind_sway, mut transform) in query.iter_mut() {
        // Get speed and amplitude from settings based on type
        let (speed, amplitude) = if wind_sway.is_grass {
            (settings.grass_speed, settings.grass_amplitude)
        } else {
            (settings.tree_speed, settings.tree_amplitude)
        };
        
        // Multi-frequency sine wave for more natural movement
        // Primary wave
        let primary_wave = (time_seconds * speed + wind_sway.phase_offset).sin();
        // Secondary faster wave for flutter effect
        let secondary_wave = (time_seconds * speed * 2.3 + wind_sway.phase_offset * 1.5).sin() * 0.3;
        // Slow large movement
        let slow_wave = (time_seconds * speed * 0.3 + wind_sway.phase_offset * 0.7).sin() * 0.2;
        
        // Combine waves with settings
        let combined_sway = (primary_wave + secondary_wave + slow_wave) * amplitude * settings.global_intensity;
        
        // Create rotation quaternion around the sway axis
        let sway_rotation = Quat::from_axis_angle(wind_sway.sway_axis, combined_sway);
        
        // Apply sway on top of base rotation
        // For grass: rotate around the base (X-axis primarily)
        // For leaves: rotate around the attachment point
        if wind_sway.is_grass {
            // Grass sways more dramatically
            let sway_z = Quat::from_axis_angle(Vec3::Z, combined_sway * 0.5);
            transform.rotation = wind_sway.base_rotation * sway_rotation * sway_z;
        } else {
            // Tree leaves sway more gently with slight flutter
            let flutter = (time_seconds * 8.0 + wind_sway.phase_offset).sin() * 0.02 * settings.global_intensity;
            let flutter_rot = Quat::from_axis_angle(Vec3::Y, flutter);
            transform.rotation = wind_sway.base_rotation * sway_rotation * flutter_rot;
        }
    }
}

/// Plugin for vegetation wind sway effects (grass, trees, leaves)
pub struct VegetationSwayPlugin;

impl Plugin for VegetationSwayPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WindEffectParticle>()
            .register_type::<WindEffectEmitter>()
            .register_type::<WindSway>()
            .register_type::<WindSwaySettings>()
            .init_resource::<WindSwaySettings>()
            .add_systems(Update, wind_sway_system);
    }
}
