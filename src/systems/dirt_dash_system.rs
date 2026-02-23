use bevy::{
    math::Vec3Swizzles,
    prelude::*,
    pbr::{MeshMaterial3d, StandardMaterial},
    render::alpha::AlphaMode,
};
use rand::Rng;

use crate::components::{
    Command, CommandMove, DirtDashEffect, DirtDashParticle, DirtDashSettings, Position,
};

/// Resource holding the shared mesh and material handles for dirt particles
#[derive(Resource)]
pub struct DirtDashAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

/// Plugin for the dirt dash effect system
pub struct DirtDashPlugin;

impl Plugin for DirtDashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DirtDashSettings>()
            .add_systems(Startup, setup_dirt_dash_assets)
            .add_systems(Update, dirt_dash_spawn_system)
            .add_systems(Update, dirt_dash_particle_update_system);
    }
}

/// System to create the shared mesh and material assets for dirt particles
fn setup_dirt_dash_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<DirtDashSettings>,
) {
    // Create a simple sphere mesh for particles
    let mesh = meshes.add(Mesh::from(bevy::math::primitives::Sphere { radius: 1.0 }));

    // Create material with the dirt color from settings
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(
            settings.particle_color.x,
            settings.particle_color.y,
            settings.particle_color.z,
            settings.particle_color.w,
        ),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.9,
        metallic: 0.0,
        cull_mode: None, // Double-sided for better visibility
        ..default()
    });

    commands.insert_resource(DirtDashAssets { mesh, material });

    log::info!("[DIRT_DASH] Initialized dirt dash particle assets");
}

/// System that detects moving characters and spawns dirt particles at their feet.
/// Uses position delta to calculate speed and triggers particle emission when
/// the character is moving faster than the minimum speed threshold.
pub fn dirt_dash_spawn_system(
    time: Res<Time>,
    settings: Res<DirtDashSettings>,
    assets: Res<DirtDashAssets>,
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
                    position.x / 100.0 + spread_x,
                    position.z / 100.0 + dirt_dash.feet_offset,
                    -position.y / 100.0 + spread_z,
                );

                // Calculate velocity - minimal upward velocity for hovering effect
                // Clamp min/max to prevent crash if settings are invalid
                let min_upward = settings.min_upward_velocity.min(settings.max_upward_velocity);
                let max_upward = settings.max_upward_velocity.max(settings.min_upward_velocity);
                let upward_velocity = rng.gen_range(min_upward..max_upward);

                // Minimal horizontal velocity - dust stays near player
                let horizontal_velocity = if speed > 0.0 {
                    let move_dir = direction.normalize();
                    Vec3::new(
                        -move_dir.x * speed * settings.horizontal_velocity_factor * rng.gen_range(0.3..1.0),
                        -move_dir.y * speed * settings.horizontal_velocity_factor * rng.gen_range(0.3..1.0),
                        0.0,
                    )
                } else {
                    Vec3::ZERO
                };

                let velocity = horizontal_velocity + Vec3::new(0.0, upward_velocity, 0.0);

                // Random lifetime and size - clamp min/max to prevent crash
                let min_lifetime = settings.min_lifetime.min(settings.max_lifetime);
                let max_lifetime = settings.max_lifetime.max(settings.min_lifetime);
                let lifetime = rng.gen_range(min_lifetime..max_lifetime);
                
                let min_size = settings.min_size.min(settings.max_size);
                let max_size = settings.max_size.max(settings.min_size);
                let size = rng.gen_range(min_size..max_size);

                // Random drift direction for wandering motion
                let drift_angle = rng.gen_range(0.0..std::f32::consts::TAU);
                let drift_direction = Vec3::new(
                    drift_angle.cos() * settings.drift_speed,
                    0.0,
                    drift_angle.sin() * settings.drift_speed,
                );

                // Random oscillation phase
                let oscillation_phase = rng.gen_range(0.0..std::f32::consts::TAU);

                // Spawn the particle entity with mesh and material
                commands.spawn((
                    DirtDashParticle::new(
                        lifetime,
                        velocity,
                        size,
                        settings.gravity,
                        settings.particle_color.w,
                        drift_direction,
                        oscillation_phase,
                        spawn_position.y,
                    ),
                    Mesh3d(assets.mesh.clone()),
                    MeshMaterial3d(assets.material.clone()),
                    Transform::from_translation(spawn_position).with_scale(Vec3::splat(size)),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                ));
            }
        }
    }
}

/// System that updates dust particles each frame.
/// Handles floating/hovering physics, lifetime, and despawning.
pub fn dirt_dash_particle_update_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut DirtDashParticle, &mut Transform)>,
    settings: Res<DirtDashSettings>,
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

        // Apply very gentle gravity (creates floating effect)
        particle.velocity.y -= particle.gravity * delta_time;

        // Apply drift motion (random wandering)
        transform.translation.x += particle.drift_direction.x * delta_time;
        transform.translation.z += particle.drift_direction.z * delta_time;

        // Apply vertical oscillation (gentle bobbing)
        let oscillation = (particle.age * 3.0 + particle.oscillation_phase).sin()
            * settings.vertical_oscillation;
        transform.translation.y = particle.base_y + oscillation;

        // Update position based on velocity
        transform.translation += particle.velocity * delta_time;

        // Update base_y to track vertical movement from velocity
        particle.base_y += particle.velocity.y * delta_time;

        // Grow slightly then shrink over lifetime for smoke effect
        let t = particle.normalized_age();
        let size_factor = if t < 0.2 {
            // Grow slightly at start
            1.0 + t * 2.5
        } else {
            // Shrink after initial growth
            1.5 - (t - 0.2) * 0.8
        };
        particle.current_size = particle.initial_size * size_factor;
        
        // Update transform scale
        transform.scale = Vec3::splat(particle.current_size);
    }
}
