use bevy::{
    pbr::MeshMaterial3d,
    prelude::*,
    render::mesh::Mesh3d,
};
use crate::components::{PlayerCharacter, Season, SeasonMarker, WeatherParticle};
use crate::resources::{SeasonMaterials, SeasonSettings, WinterSettings};

/// Spawns and updates snow particles for winter season
/// Particles use billboard behavior to always face the camera
pub fn winter_snow_system(
    mut commands: Commands,
    settings: Res<SeasonSettings>,
    winter_settings: Res<WinterSettings>,
    season_materials: Res<SeasonMaterials>,
    player_query: Query<&GlobalTransform, With<PlayerCharacter>>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut query: Query<(Entity, &mut Transform, &mut WeatherParticle), (Without<PlayerCharacter>, Without<Camera3d>)>,
    time: Res<Time>,
) {
    if !settings.enabled || settings.current_season != Season::Winter {
        return;
    }

    let dt = time.delta_secs();

    // Get player position for player-relative spawning
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    let player_pos = player_transform.translation();

    // Spawn new snowflakes
    let current_count = query.iter().len();
    if current_count < settings.max_particles {
        let particles_this_frame = ((settings.spawn_rate * dt) as usize).max(10);
        for _ in 0..particles_this_frame {
            // Spawn in a circle around player using radius
            let spawn_radius = 100.0; // Distance from player
            let angle = rand::random::<f32>() * std::f32::consts::TAU;
            let radius_offset = rand::random::<f32>() * spawn_radius;
            let offset_x = angle.cos() * radius_offset;
            let offset_z = angle.sin() * radius_offset;
            // Spawn 15-25 units above player
            let spawn_y = player_pos.y + 15.0 + rand::random::<f32>() * 10.0;

            let position = Vec3::new(
                player_pos.x + offset_x,
                spawn_y,
                player_pos.z + offset_z,
            );

            let size_range = winter_settings.snowflake_size_range;
            let size = size_range.0 + rand::random::<f32>() * (size_range.1 - size_range.0);

            let lifetime_range = winter_settings.lifetime_range;
            let lifetime = lifetime_range.0 + rand::random::<f32>() * (lifetime_range.1 - lifetime_range.0);

            // Use pre-created hexagon mesh for snowflake
            let snow_mesh = season_materials.snow_mesh.clone();

            // Use pre-created snow material
            let snow_material = season_materials.snow_material.clone();

            commands.spawn((
                Mesh3d(snow_mesh),
                MeshMaterial3d(snow_material),
                Transform::from_translation(position).with_scale(Vec3::splat(size)),
                WeatherParticle {
                    age: 0.0,
                    lifetime,
                    velocity: Vec3::new(
                        (rand::random::<f32>() - 0.5) * 0.5,
                        -winter_settings.fall_speed,
                        (rand::random::<f32>() - 0.5) * 0.5,
                    ),
                    base_size: size,
                    rotation: rand::random::<f32>() * std::f32::consts::TAU,
                    rotation_speed: (rand::random::<f32>() - 0.5) * 0.5,
                    wobble_phase: rand::random::<f32>() * std::f32::consts::TAU,
                    wobble_amplitude: winter_settings.turbulence,
                },
                SeasonMarker(Season::Winter),
            ));
        }
    }

    // Get camera transform for billboard behavior
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    let camera_pos = camera_transform.translation();

    // Update existing snowflakes with billboard behavior
    for (entity, mut transform, mut particle) in query.iter_mut() {
        particle.age += dt;

        if particle.age >= particle.lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        // Despawn if below ground level (ground is at y=0 in most zones)
        if transform.translation.y < 0.5 {
            commands.entity(entity).despawn();
            continue;
        }

        // Turbulent swirling motion
        particle.wobble_phase += dt * 3.0;
        let swirl_x = (particle.wobble_phase.sin() * particle.wobble_amplitude)
            * settings.wind_strength;
        let swirl_z = (particle.wobble_phase.cos() * particle.wobble_amplitude * 0.7)
            * settings.wind_strength;

        // Apply wind
        let wind = Vec3::new(
            settings.wind_direction.x * settings.wind_strength * 0.5,
            0.0,
            settings.wind_direction.y * settings.wind_strength * 0.5,
        );

        transform.translation += (particle.velocity + wind + Vec3::new(swirl_x, 0.0, swirl_z)) * dt;

        // Billboard: Make particle face the camera
        // Calculate direction from particle to camera
        let to_camera = camera_pos - transform.translation;
        if to_camera.length_squared() > 0.001 {
            let forward = to_camera.normalize();
            // Create a rotation that faces the camera (billboard look-at)
            // Use up vector (0, 1, 0) to maintain consistent orientation
            let up = Vec3::Y;
            let right = up.cross(forward).normalize();
            let corrected_up = forward.cross(right).normalize();
            
            // Build rotation matrix and convert to quaternion
            let look_rotation = Quat::from_mat3(&Mat3::from_cols(right, corrected_up, forward));
            
            // Apply particle's own rotation on top (for visual variety)
            particle.rotation += particle.rotation_speed * dt;
            let particle_rotation = Quat::from_rotation_z(particle.rotation);
            
            transform.rotation = look_rotation * particle_rotation;
        }

        // Note: Fade-out effect removed to avoid ResMut<Assets<StandardMaterial>> conflict
        // Particles will simply disappear at end of lifetime
    }
}
