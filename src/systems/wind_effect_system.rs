use bevy::{
    prelude::*,
    pbr::{MeshMaterial3d, StandardMaterial},
    render::alpha::AlphaMode,
};
use rand::Rng;

use crate::components::{
    FacingDirection, FlightState, PlayerCharacter, Position, WindEffectEmitter, WindEffectParticle,
};
use crate::resources::FlightSettings;

/// Resource holding the shared mesh and material handles for wind particles
#[derive(Resource)]
pub struct WindEffectAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

/// Plugin for the wind effect system
pub struct WindEffectPlugin;

impl Plugin for WindEffectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_wind_effect_assets)
            .add_systems(Update, wind_emitter_spawn_system)
            .add_systems(Update, wind_particle_spawn_system)
            .add_systems(Update, wind_particle_update_system);
    }
}

/// System to create the shared mesh and material assets for wind particles
fn setup_wind_effect_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create a thin capsule mesh for horizontal wind streaks
    // Capsule3d is oriented along the Y axis by default, we'll rotate it when spawning
    let mesh = meshes.add(Mesh::from(bevy::math::primitives::Capsule3d {
        radius: 0.1,      // Thin radius for streamlined look
        half_length: 0.9, // Long body for streak effect
    }));

    // Create semi-transparent white/light-blue material for wind streaks
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.85, 0.92, 1.0, 0.5), // Light blue, semi-transparent
        emissive: LinearRgba::new(0.3, 0.5, 0.7, 1.0),   // Soft blue glow
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.2,
        metallic: 0.1,
        cull_mode: None, // Double-sided for better visibility
        ..default()
    });

    commands.insert_resource(WindEffectAssets { mesh, material });

    log::info!("[WIND_EFFECT] Initialized wind effect particle assets");
}

/// System that spawns a wind effect emitter when flight starts
/// and despawns it when flight ends
pub fn wind_emitter_spawn_system(
    mut commands: Commands,
    mut flight_states: Query<(Entity, &mut FlightState), With<PlayerCharacter>>,
) {
    for (entity, mut flight_state) in flight_states.iter_mut() {
        if flight_state.is_flying {
            // Spawn emitter if not already present
            if flight_state.wind_emitter_entity.is_none() {
                let emitter_entity = commands
                    .spawn((
                        WindEffectEmitter::default(),
                        Transform::default(),
                        GlobalTransform::default(),
                    ))
                    .id();

                flight_state.wind_emitter_entity = Some(emitter_entity);

                // Make emitter a child of the player entity so it follows them
                commands.entity(entity).add_child(emitter_entity);

                log::info!(
                    "[WIND_EFFECT] Spawned wind emitter for flying player"
                );
            }
        } else {
            // Despawn emitter if present when not flying
            if let Some(emitter_entity) = flight_state.wind_emitter_entity {
                if commands.get_entity(emitter_entity).is_ok() {
                    commands.entity(emitter_entity).despawn();
                }
                flight_state.wind_emitter_entity = None;

                log::info!(
                    "[WIND_EFFECT] Despawned wind emitter - flight ended"
                );
            }
        }
    }
}

/// System that spawns wind particles while flying
/// Particles streak backward to show motion, with more particles when thrusting
pub fn wind_particle_spawn_system(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    assets: Res<WindEffectAssets>,
    mut commands: Commands,
    flight_query: Query<(&FlightState, &Position, &FacingDirection), With<PlayerCharacter>>,
    mut emitter_query: Query<&mut WindEffectEmitter>,
    particle_count: Query<(), With<WindEffectParticle>>,
) {
    let delta_time = time.delta_secs();
    let mut rng = rand::thread_rng();

    // Performance limit for particles
    const MAX_WIND_PARTICLES: usize = 200;
    let current_particle_count = particle_count.iter().count();
    if current_particle_count >= MAX_WIND_PARTICLES {
        return;
    }

    for (flight_state, position, facing_direction) in flight_query.iter() {
        // Only spawn particles when flying AND moving
        if !flight_state.is_flying {
            continue;
        }

        // Don't spawn particles when stopped (no speed)
        if flight_state.current_speed < 0.1 {
            continue;
        }

        // Get the emitter for this flight state
        let Some(emitter_entity) = flight_state.wind_emitter_entity else {
            continue;
        };

        let Ok(mut emitter) = emitter_query.get_mut(emitter_entity) else {
            continue;
        };

        // Tick the spawn timer
        emitter.spawn_timer.tick(time.delta());

        // Only spawn when timer finishes
        if !emitter.spawn_timer.finished() {
            continue;
        }

        // Calculate spawn rate based on thrusting state
        // More particles when thrusting (Space bar held)
        let base_rate = settings.wind_particle_spawn_rate;
        let spawn_rate = if flight_state.is_thrusting {
            base_rate * 2.5 // 2.5x more particles when thrusting
        } else {
            base_rate
        };

        // Adjust timer duration based on spawn rate
        emitter
            .spawn_timer
            .set_duration(std::time::Duration::from_secs_f32(1.0 / spawn_rate));

        // Number of particles to spawn this frame
        let particles_to_spawn = if flight_state.is_thrusting { 3 } else { 1 };

        for _ in 0..particles_to_spawn {
            // Check particle limit
            if particle_count.iter().count() >= MAX_WIND_PARTICLES {
                break;
            }

            // Convert Position to world coordinates (ROSE uses centimeters, Bevy uses meters)
            let player_pos = Vec3::new(
                position.x / 100.0,
                position.z / 100.0, // Z is up in Bevy
                -position.y / 100.0,
            );

            // Spawn particles around the character's body (torso/chest area, not feet)
            // Player height is approximately 1.8m, so torso is around 0.8-1.2m above feet
            let body_offset = rng.gen_range(0.8..1.4); // Torso/chest height offset
            let side_offset = rng.gen_range(-0.5..0.5); // Random side offset
            let front_back_offset = rng.gen_range(-0.3..0.3); // Random front/back offset

            let spawn_position = player_pos + Vec3::new(side_offset, body_offset, front_back_offset);

            // Calculate backward direction based on facing direction
            // FacingDirection stores angle in radians
            let facing_angle = facing_direction.actual;
            let forward_dir = Vec3::new(facing_angle.cos(), 0.0, -facing_angle.sin()).normalize();
            let backward_dir = -forward_dir;

            // Particle velocity - streak backward with some randomness
            let speed = if flight_state.is_thrusting {
                rng.gen_range(8.0..15.0) // Faster when thrusting
            } else {
                rng.gen_range(4.0..8.0) // Normal speed when gliding
            };

            let velocity = backward_dir * speed
                + Vec3::new(
                    rng.gen_range(-0.5..0.5),
                    rng.gen_range(-0.3..0.3),
                    rng.gen_range(-0.5..0.5),
                );

            // Random lifetime (0.5 to 1.0 seconds)
            let lifetime = rng.gen_range(0.5..1.0);

            // Particle size - thin for streamlined streak effect
            let size = rng.gen_range(0.02..0.05);

            // Calculate rotation to align particle with velocity direction
            // The capsule mesh is oriented along Y-axis by default
            // We need to rotate it to align with the backward (velocity) direction
            let velocity_normalized = velocity.normalize();
            let up = Vec3::Y;
            
            // Calculate rotation from Y-axis to velocity direction
            let rotation = if velocity_normalized.abs().dot(up) > 0.999 {
                // Nearly parallel or anti-parallel to Y axis
                if velocity_normalized.y > 0.0 {
                    Quat::IDENTITY
                } else {
                    Quat::from_rotation_x(std::f32::consts::PI)
                }
            } else {
                // Use rotation from Y-axis to velocity direction
                Quat::from_rotation_arc(up, velocity_normalized)
            };

            // Spawn the particle entity with horizontal orientation
            commands.spawn((
                WindEffectParticle {
                    velocity,
                    lifetime: Timer::from_seconds(lifetime, TimerMode::Once),
                    initial_alpha: 0.5,
                },
                Mesh3d(assets.mesh.clone()),
                MeshMaterial3d(assets.material.clone()),
                Transform::from_translation(spawn_position)
                    .with_rotation(rotation)
                    .with_scale(Vec3::new(size, size * 3.0, size)), // Elongated along velocity
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ));
        }
    }
}

/// System that updates wind particles each frame
/// Handles movement, lifetime, fading, and despawning
pub fn wind_particle_update_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut WindEffectParticle, &mut Transform)>,
) {
    let delta_time = time.delta_secs();

    for (entity, mut particle, mut transform) in query.iter_mut() {
        // Tick lifetime timer
        particle.lifetime.tick(time.delta());

        // Check if particle should die
        if particle.lifetime.finished() {
            commands.entity(entity).despawn();
            continue;
        }

        // Update position based on velocity
        transform.translation += particle.velocity * delta_time;

        // Calculate fade based on remaining lifetime
        let remaining_ratio = particle.lifetime.remaining_secs() / particle.lifetime.duration().as_secs_f32();

        // Scale down the particle as it fades for a shrinking effect
        let scale_factor = 0.3 + 0.7 * remaining_ratio;
        let base_size = transform.scale.x.max(0.001);
        transform.scale = Vec3::new(
            base_size * scale_factor,
            base_size * 3.0 * scale_factor, // Maintain elongation ratio
            base_size * scale_factor,
        );

        // Slow down slightly over time (air resistance effect)
        particle.velocity *= 0.98;
    }
}

/// Cleanup system to despawn all wind particles when leaving flight mode
pub fn cleanup_wind_particles_on_flight_end(
    mut commands: Commands,
    flight_states: Query<&FlightState, With<PlayerCharacter>>,
    particles: Query<Entity, With<WindEffectParticle>>,
) {
    // Check if any player is not flying
    for flight_state in flight_states.iter() {
        if !flight_state.is_flying {
            // Despawn all wind particles
            for entity in particles.iter() {
                commands.entity(entity).despawn();
            }
            break;
        }
    }
}
