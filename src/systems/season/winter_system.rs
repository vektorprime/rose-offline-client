use bevy::{
    pbr::MeshMaterial3d,
    prelude::*,
    render::mesh::Mesh3d,
};
use crate::components::{Season, SeasonMarker, WeatherParticle};
use crate::resources::{SeasonMaterials, SeasonSettings, WinterSettings};

/// Spawns and updates snow particles for winter season
pub fn winter_snow_system(
    mut commands: Commands,
    settings: Res<SeasonSettings>,
    winter_settings: Res<WinterSettings>,
    season_materials: Res<SeasonMaterials>,
    camera_query: Query<&Transform, With<Camera3d>>,
    mut query: Query<(Entity, &mut Transform, &mut WeatherParticle), Without<Camera3d>>,
    time: Res<Time>,
) {
    if !settings.enabled || settings.current_season != Season::Winter {
        return;
    }

    let dt = time.delta_secs();

    // Get camera position for camera-relative spawning
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    let camera_pos = camera_transform.translation;

    // Spawn new snowflakes
    let current_count = query.iter().len();
    if current_count < settings.max_particles {
        let particles_this_frame = ((settings.spawn_rate * dt) as usize).max(10);
        for _ in 0..particles_this_frame {
            // Spawn in large radius around camera (500 unit radius = 1000 unit diameter)
            let large_radius = 500.0;
            let offset_x = (rand::random::<f32>() - 0.5) * large_radius * 2.0;
            let offset_z = (rand::random::<f32>() - 0.5) * large_radius * 2.0;
            let spawn_y = camera_pos.y + 30.0 + rand::random::<f32>() * 30.0; // Spawn above camera (30-60 units)

            let position = Vec3::new(
                camera_pos.x + offset_x,
                spawn_y,
                camera_pos.z + offset_z,
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

    // Update existing snowflakes
    for (entity, mut transform, mut particle) in query.iter_mut() {
        particle.age += dt;

        if particle.age >= particle.lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        // Despawn if below ground level (relative to camera)
        if transform.translation.y < camera_pos.y - 50.0 {
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

        // Gentle rotation
        particle.rotation += particle.rotation_speed * dt;
        transform.rotation = Quat::from_rotation_z(particle.rotation);

        // Note: Fade-out effect removed to avoid ResMut<Assets<StandardMaterial>> conflict
        // Particles will simply disappear at end of lifetime
    }
}
