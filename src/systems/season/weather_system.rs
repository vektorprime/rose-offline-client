use crate::components::{PlayerCharacter, Season, SeasonMarker, WeatherParticle};
use crate::resources::{
    FallSettings, SeasonMaterials, SeasonSettings, SpringSettings, WinterSettings,
};
use bevy::{pbr::MeshMaterial3d, prelude::*};
use bevy_mesh::Mesh3d;

/// Per-particle spawn data generated for the active season
struct WeatherParticleSpawn {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    scale: Vec3,
    velocity: Vec3,
    lifetime: f32,
    base_size: f32,
    rotation: f32,
    rotation_speed: f32,
    wobble_phase: f32,
    wobble_amplitude: f32,
}

/// Spawns and updates weather particles (rain/snow/leaves) for the current season
/// Particles use billboard behavior to always face the camera
pub fn weather_particle_system(
    mut commands: Commands,
    settings: Res<SeasonSettings>,
    fall_settings: Res<FallSettings>,
    winter_settings: Res<WinterSettings>,
    spring_settings: Res<SpringSettings>,
    season_materials: Res<SeasonMaterials>,
    player_query: Query<&GlobalTransform, With<PlayerCharacter>>,
    camera_query: Query<
        &GlobalTransform,
        (With<Camera3d>, Without<crate::render::WaterReflectionCamera>),
    >,
    mut query: Query<
        (Entity, &mut Transform, &mut WeatherParticle),
        (Without<PlayerCharacter>, Without<Camera3d>),
    >,
    time: Res<Time>,
) {
    if !settings.enabled {
        return;
    }

    let dt = time.delta_secs();

    // Get player position for player-relative spawning
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation();

    // Spawn new particles
    let current_count = query.iter().len();
    if current_count < settings.max_particles {
        let particles_this_frame = ((settings.spawn_rate * dt) as usize).max(10);
        for _ in 0..particles_this_frame {
            let Some(spawn) = particle_spawn(
                settings.current_season,
                &settings,
                &fall_settings,
                &winter_settings,
                &spring_settings,
                &season_materials,
            ) else {
                break;
            };

            // Spawn in a circle around player using radius
            let spawn_radius = 100.0; // Distance from player
            let angle = rand::random::<f32>() * std::f32::consts::TAU;
            let radius_offset = rand::random::<f32>() * spawn_radius;
            let offset_x = angle.cos() * radius_offset;
            let offset_z = angle.sin() * radius_offset;
            // Spawn 15-25 units above player
            let spawn_y = player_pos.y + 15.0 + rand::random::<f32>() * 10.0;

            let position = Vec3::new(player_pos.x + offset_x, spawn_y, player_pos.z + offset_z);

            commands.spawn((
                Mesh3d(spawn.mesh),
                MeshMaterial3d(spawn.material),
                Transform::from_translation(position).with_scale(spawn.scale),
                WeatherParticle {
                    age: 0.0,
                    lifetime: spawn.lifetime,
                    velocity: spawn.velocity,
                    base_size: spawn.base_size,
                    rotation: spawn.rotation,
                    rotation_speed: spawn.rotation_speed,
                    wobble_phase: spawn.wobble_phase,
                    wobble_amplitude: spawn.wobble_amplitude,
                },
                SeasonMarker(settings.current_season),
            ));
        }
    }

    // Get camera transform for billboard behavior
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    let camera_pos = camera_transform.translation();

    // Update existing particles
    for (entity, mut transform, mut particle) in query.iter_mut() {
        particle.age += dt;

        // Despawn if below ground level (ground is at y=0 in most zones) or lifetime exceeded
        if particle.age >= particle.lifetime || transform.translation.y < 0.5 {
            commands.entity(entity).despawn();
            continue;
        }

        update_particle_movement(
            settings.current_season,
            &settings,
            &fall_settings,
            &mut transform,
            &mut particle,
            dt,
        );

        // Billboard: Make particle face the camera
        let to_camera = camera_pos - transform.translation;
        if to_camera.length_squared() > 0.001 {
            let forward = to_camera.normalize();
            let up = Vec3::Y;
            let right = up.cross(forward).normalize();
            let corrected_up = forward.cross(right).normalize();

            // Build rotation matrix and convert to quaternion
            let look_rotation = Quat::from_mat3(&Mat3::from_cols(right, corrected_up, forward));

            if matches!(
                settings.current_season,
                Season::Winter | Season::Fall
            ) {
                // Apply particle's own rotation on top (for visual variety)
                particle.rotation += particle.rotation_speed * dt;
                let particle_rotation = Quat::from_rotation_z(particle.rotation);

                transform.rotation = look_rotation * particle_rotation;
            } else {
                transform.rotation = look_rotation;
            }
        }
    }
}

/// Generates per-particle spawn values for the active season
fn particle_spawn(
    season: Season,
    settings: &SeasonSettings,
    fall_settings: &FallSettings,
    winter_settings: &WinterSettings,
    spring_settings: &SpringSettings,
    season_materials: &SeasonMaterials,
) -> Option<WeatherParticleSpawn> {
    match season {
        Season::Spring => Some(WeatherParticleSpawn {
            // Use pre-created elongated rain mesh
            mesh: season_materials.rain_mesh.clone(),
            // Use pre-created rain material
            material: season_materials.rain_material.clone(),
            scale: Vec3::new(
                spring_settings.rain_drop_size,
                spring_settings.rain_drop_size * 2.0, // Elongate rain drops
                spring_settings.rain_drop_size,
            ),
            velocity: Vec3::new(
                settings.wind_direction.x * settings.wind_strength * 0.5,
                -spring_settings.rain_speed,
                settings.wind_direction.y * settings.wind_strength * 0.5,
            ),
            lifetime: 2.0 + rand::random::<f32>() * 1.0,
            base_size: spring_settings.rain_drop_size,
            rotation: 0.0,
            rotation_speed: 0.0,
            wobble_phase: 0.0,
            wobble_amplitude: 0.0,
        }),
        Season::Winter => {
            let size_range = winter_settings.snowflake_size_range;
            let size = size_range.0 + rand::random::<f32>() * (size_range.1 - size_range.0);
            let lifetime_range = winter_settings.lifetime_range;
            let lifetime =
                lifetime_range.0 + rand::random::<f32>() * (lifetime_range.1 - lifetime_range.0);

            Some(WeatherParticleSpawn {
                // Use pre-created hexagon mesh for snowflake
                mesh: season_materials.snow_mesh.clone(),
                // Use pre-created snow material
                material: season_materials.snow_material.clone(),
                scale: Vec3::splat(size),
                velocity: Vec3::new(
                    (rand::random::<f32>() - 0.5) * 0.5,
                    -winter_settings.fall_speed,
                    (rand::random::<f32>() - 0.5) * 0.5,
                ),
                lifetime,
                base_size: size,
                rotation: rand::random::<f32>() * std::f32::consts::TAU,
                rotation_speed: (rand::random::<f32>() - 0.5) * 0.5,
                wobble_phase: rand::random::<f32>() * std::f32::consts::TAU,
                wobble_amplitude: winter_settings.turbulence,
            })
        }
        Season::Fall => {
            let size_range = fall_settings.leaf_size_range;
            let size = size_range.0 + rand::random::<f32>() * (size_range.1 - size_range.0);
            let lifetime_range = fall_settings.lifetime_range;
            let lifetime =
                lifetime_range.0 + rand::random::<f32>() * (lifetime_range.1 - lifetime_range.0);

            // Get random leaf material from pre-created materials
            let leaf_material = season_materials.leaf_materials
                [rand::random::<usize>() % season_materials.leaf_materials.len()]
            .clone();

            Some(WeatherParticleSpawn {
                // Use pre-created diamond mesh for the leaf particle
                mesh: season_materials.leaf_mesh.clone(),
                material: leaf_material,
                scale: Vec3::splat(size),
                velocity: Vec3::new(
                    (rand::random::<f32>() - 0.5) * fall_settings.drift_factor,
                    -fall_settings.fall_speed,
                    (rand::random::<f32>() - 0.5) * fall_settings.drift_factor,
                ),
                lifetime,
                base_size: size,
                rotation: rand::random::<f32>() * std::f32::consts::TAU,
                rotation_speed: (rand::random::<f32>() - 0.5) * 2.0,
                wobble_phase: rand::random::<f32>() * std::f32::consts::TAU,
                wobble_amplitude: 0.5 + rand::random::<f32>() * 0.5,
            })
        }
        _ => None,
    }
}

/// Updates particle movement based on the active season
fn update_particle_movement(
    season: Season,
    settings: &SeasonSettings,
    fall_settings: &FallSettings,
    transform: &mut Transform,
    particle: &mut WeatherParticle,
    dt: f32,
) {
    match season {
        Season::Spring => {
            transform.translation += particle.velocity * dt;
        }
        Season::Winter => {
            // Turbulent swirling motion
            particle.wobble_phase += dt * 3.0;
            let swirl_x =
                (particle.wobble_phase.sin() * particle.wobble_amplitude) * settings.wind_strength;
            let swirl_z = (particle.wobble_phase.cos() * particle.wobble_amplitude * 0.7)
                * settings.wind_strength;

            // Apply wind
            let wind = Vec3::new(
                settings.wind_direction.x * settings.wind_strength * 0.5,
                0.0,
                settings.wind_direction.y * settings.wind_strength * 0.5,
            );

            transform.translation += (particle.velocity + wind + Vec3::new(swirl_x, 0.0, swirl_z))
                * dt;
        }
        Season::Fall => {
            // Update wobble
            particle.wobble_phase += dt * fall_settings.wobble_frequency;
            let wobble =
                (particle.wobble_phase.sin() * particle.wobble_amplitude) * settings.wind_strength;

            // Apply wind and wobble
            let wind = Vec3::new(
                settings.wind_direction.x * settings.wind_strength,
                0.0,
                settings.wind_direction.y * settings.wind_strength,
            );

            transform.translation +=
                (particle.velocity + wind + Vec3::new(wobble, 0.0, wobble * 0.5)) * dt;
        }
        _ => {}
    }
}
