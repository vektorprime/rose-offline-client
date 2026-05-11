//! Dynamic Storms System
//!
//! Storms that form, move across the zone, and affect sailing conditions.

use bevy::prelude::*;

use crate::components::BoatState;
use crate::resources::WindState;

/// Current storm state resource
#[derive(Resource, Debug, Clone)]
pub struct StormState {
    /// Whether a storm is currently active
    pub active: bool,
    /// Storm center position (meters)
    pub center: Vec2,
    /// Storm radius (meters)
    pub radius: f32,
    /// Storm intensity (0.0-1.0)
    pub intensity: f32,
    /// Storm movement direction
    pub movement: Vec2,
    /// Storm formation timer
    pub formation_timer: Timer,
    /// Storm duration timer
    pub duration_timer: Timer,
    /// Time since last storm
    pub time_since_storm: f32,
    /// Lightning strike timer
    pub lightning_timer: Timer,
    /// Whether lightning is currently striking
    pub lightning_active: bool,
}

impl Default for StormState {
    fn default() -> Self {
        Self {
            active: false,
            center: Vec2::ZERO,
            radius: 500.0,
            intensity: 0.0,
            movement: Vec2::new(1.0, -1.0).normalize(),
            formation_timer: Timer::from_seconds(30.0, TimerMode::Once),
            duration_timer: Timer::from_seconds(120.0, TimerMode::Once),
            time_since_storm: 0.0,
            lightning_timer: Timer::from_seconds(5.0, TimerMode::Repeating),
            lightning_active: false,
        }
    }
}

/// Storm visual effect component
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct StormVisual {
    /// Cloud layer entity
    pub cloud_entity: Option<Entity>,
    /// Rain particle entity
    pub rain_entity: Option<Entity>,
    /// Lightning flash timer
    pub flash_timer: Timer,
}

impl Default for StormVisual {
    fn default() -> Self {
        Self {
            cloud_entity: None,
            rain_entity: None,
            flash_timer: Timer::from_seconds(0.1, TimerMode::Once),
        }
    }
}

/// Update storm state and behavior
pub fn storm_update_system(
    time: Res<Time>,
    mut storm: ResMut<StormState>,
    mut wind: ResMut<WindState>,
) {
    storm.time_since_storm += time.delta_secs();

    if !storm.active {
        // Check if we should form a new storm
        storm.formation_timer.tick(time.delta());
        
        // Form storm after random interval (3-10 minutes)
        if storm.time_since_storm > 180.0 && storm.formation_timer.just_finished() {
            storm.active = true;
            storm.intensity = 0.0;
            storm.center = Vec2::new(
                (2000.0..8000.0).gen(),
                -(2000.0..8000.0).gen(),
            );
            storm.radius = 500.0;
            storm.duration_timer.reset();
            log::info!("[storm_update_system] Storm formed at ({}, {})", storm.center.x, storm.center.y);
        }
        return;
    }

    // Active storm behavior
    storm.duration_timer.tick(time.delta());
    
    // Build up intensity
    if storm.intensity < 1.0 {
        storm.intensity = (storm.intensity + time.delta_secs() * 0.01).min(1.0);
    }

    // Move storm
    storm.center += storm.movement * 10.0 * time.delta_secs();

    // Expand radius
    storm.radius = (storm.radius + time.delta_secs() * 2.0).min(1000.0);

    // Affect wind
    wind.speed *= (1.0 + storm.intensity * 2.0);
    wind.gust_factor = storm.intensity;

    // Lightning
    storm.lightning_timer.tick(time.delta());
    if storm.lightning_timer.just_finished() && storm.intensity > 0.5 {
        storm.lightning_active = true;
        storm.flash_timer.reset();
        log::debug!("[storm_update_system] Lightning strike!");
    }

    // Check if storm should dissipate
    if storm.duration_timer.just_finished() {
        storm.intensity = (storm.intensity - time.delta_secs() * 0.02).max(0.0);
        if storm.intensity <= 0.0 {
            storm.active = false;
            storm.time_since_storm = 0.0;
            log::info!("[storm_update_system] Storm dissipated");
        }
    }
}

/// Visual effects for storms
pub fn storm_visual_system(
    time: Res<Time>,
    storm: Res<StormState>,
    mut commands: &mut Commands,
) {
    if !storm.active {
        return;
    }

    // Update lightning flash
    if storm.lightning_active {
        // Would spawn lightning visual effect here
        storm.lightning_active = false;
    }

    // Spawn/update storm clouds
    // Would create cloud entities at storm center
}

/// Apply storm effects to boats
pub fn storm_effect_on_boats_system(
    time: Res<Time>,
    storm: Res<StormState>,
    mut boat_query: Query<(&mut BoatState, &Position)>,
) {
    if !storm.active {
        return;
    }

    for (mut boat, position) in boat_query.iter_mut() {
        // Check if boat is within storm radius
        let boat_pos = Vec2::new(
            position.position.x / 100.0,
            position.position.y / 100.0,
        );
        
        let dist = (boat_pos - storm.center).length();
        
        if dist < storm.radius {
            // Apply storm effects
            let effect_strength = (1.0 - dist / storm.radius) * storm.intensity;
            
            // Reduce boat speed
            boat.speed *= (1.0 - effect_strength * 0.3);
            
            // Add wave roll/pitch
            boat.wave_roll += effect_strength * time.delta_secs() * 2.0;
            boat.wave_pitch += effect_strength * time.delta_secs() * 1.5;
            
            // Clamp wave effects
            boat.wave_roll = boat.wave_roll.sin().clamp(-0.2, 0.2);
            boat.wave_pitch = boat.wave_pitch.sin().clamp(-0.15, 0.15);
        }
    }
}
