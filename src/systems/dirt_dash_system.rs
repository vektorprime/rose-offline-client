use bevy::{
    log::debug,
    math::Vec3Swizzles,
    prelude::*,
};
use rand::Rng;

use crate::components::{
    Command, CommandMove, DirtDashEffect, DirtDashParticle, DirtDashSettings, Position,
};

/// Plugin for the dirt dash effect system
pub struct DirtDashPlugin;

impl Plugin for DirtDashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DirtDashSettings>()
            .add_systems(Update, dirt_dash_spawn_system)
            .add_systems(Update, dirt_dash_particle_update_system);
    }
}

/// System that detects moving characters and spawns dirt particles at their feet.
/// Uses position delta to calculate speed and triggers particle emission when
/// the character is moving faster than the minimum speed threshold.
pub fn dirt_dash_spawn_system(
    time: Res<Time>,
    settings: Res<DirtDashSettings>,
    mut commands: Commands,
    mut query: Query<(
        &Position,
        &Command,
        &mut DirtDashEffect,
    )>,
    particle_count: Query<(), With<DirtDashParticle>>,
) {
    let delta_time = time.delta_secs();
    let mut rng = rand::thread_rng();

    // Performance check: skip if too many particles exist
    let current_particle_count = particle_count.iter().count();
    if current_particle_count >= settings.max_particles {
        return;
    }

    for (position, command, mut dirt_dash) in query.iter_mut() {
        // Check if the entity is moving
        let Command::Move(CommandMove { destination, .. }) = *command else {
            // Not moving, reset timer
            dirt_dash.spawn_timer = 0.0;
            continue;
        };

        // Calculate speed based on distance to destination and movement
        let direction = destination.xy() - position.xy();
        let speed = direction.length();

        // Only spawn particles if moving fast enough
        if speed < dirt_dash.min_speed {
            dirt_dash.spawn_timer = 0.0;
            continue;
        }

        // Accumulate time
        dirt_dash.spawn_timer += delta_time;

        // Spawn particles at intervals
        while dirt_dash.spawn_timer >= dirt_dash.spawn_interval {
            dirt_dash.spawn_timer -= dirt_dash.spawn_interval;

            // Spawn a burst of particles
            for _ in 0..dirt_dash.particles_per_burst {
                // Check particle limit again
                if particle_count.iter().count() >= settings.max_particles {
                    break;
                }

                // Calculate spawn position with random spread
                let spread_x = rng.gen_range(-dirt_dash.spread_radius..dirt_dash.spread_radius);
                let spread_z = rng.gen_range(-dirt_dash.spread_radius..dirt_dash.spread_radius);

                let spawn_position = Vec3::new(
                    position.x + spread_x,
                    position.y,
                    position.z + dirt_dash.feet_offset + spread_z,
                );

                // Calculate velocity - mostly upward with some backward motion
                let upward_velocity = rng.gen_range(settings.min_upward_velocity..settings.max_upward_velocity);
                
                // Add some velocity in the opposite direction of movement
                let horizontal_velocity = if speed > 0.0 {
                    let move_dir = direction.normalize();
                    Vec3::new(
                        -move_dir.x * speed * settings.horizontal_velocity_factor * rng.gen_range(0.5..1.5),
                        -move_dir.y * speed * settings.horizontal_velocity_factor * rng.gen_range(0.5..1.5),
                        0.0,
                    )
                } else {
                    Vec3::ZERO
                };

                let velocity = horizontal_velocity + Vec3::new(0.0, 0.0, upward_velocity);

                // Random lifetime and size
                let lifetime = rng.gen_range(settings.min_lifetime..settings.max_lifetime);
                let size = rng.gen_range(settings.min_size..settings.max_size);

                // Spawn the particle entity
                commands.spawn((
                    DirtDashParticle::new(lifetime, velocity, size, settings.gravity, settings.particle_color.w),
                    Transform::from_translation(spawn_position),
                ));
            }
        }
    }
}

/// System that updates dirt dash particles each frame.
/// Handles physics simulation, lifetime, and despawning.
pub fn dirt_dash_particle_update_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut DirtDashParticle, &mut Transform)>,
) {
    let delta_time = time.delta_secs();

    for (entity, mut particle, mut transform) in query.iter_mut() {
        // Update age
        particle.age += delta_time;

        // Check if particle should die
        if particle.age >= particle.lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        // Apply gravity to velocity
        particle.velocity.z -= particle.gravity * delta_time;

        // Update position based on velocity
        transform.translation += particle.velocity * delta_time;

        // Shrink particle over lifetime
        let t = particle.normalized_age();
        particle.current_size = particle.initial_size * (1.0 - t * 0.5);
    }
}
