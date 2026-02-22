use bevy::{
    pbr::MeshMaterial3d,
    prelude::*,
    render::mesh::Mesh3d,
};
use crate::components::{PlayerCharacter, Season, SeasonMarker, WeatherParticle};
use crate::resources::{SeasonMaterials, SeasonSettings, FallSettings};

/// Spawns falling leaf particles for fall season
#[allow(dead_code)]
pub fn fall_particle_spawn_system(
    mut commands: Commands,
    settings: Res<SeasonSettings>,
    fall_settings: Res<FallSettings>,
    season_materials: Res<SeasonMaterials>,
    particle_count: Query<(), With<WeatherParticle>>,
    time: Res<Time>,
) {
    if !settings.enabled || settings.current_season != Season::Fall {
        return;
    }

    let current_count = particle_count.iter().len();
    if current_count >= settings.max_particles {
        return;
    }

    // Spawn particles across zone
    let spawn_rate = settings.spawn_rate as f64;
    let particles_to_spawn = (time.elapsed_secs_f64() * spawn_rate).fract();

    if particles_to_spawn < 0.1 {
        return;
    }

    // Spawn across entire zone (2000 unit diameter)
    let zone_size = 2000.0;
    let spawn_x = (rand::random::<f32>() - 0.5) * zone_size;
    let spawn_z = (rand::random::<f32>() - 0.5) * zone_size;
    let spawn_y = 50.0 + rand::random::<f32>() * 50.0; // Spawn high up (50-100 units)

    let position = Vec3::new(spawn_x, spawn_y, spawn_z);

    let size_range = fall_settings.leaf_size_range;
    let size = size_range.0 + rand::random::<f32>() * (size_range.1 - size_range.0);

    let lifetime_range = fall_settings.lifetime_range;
    let lifetime = lifetime_range.0 + rand::random::<f32>() * (lifetime_range.1 - lifetime_range.0);

    // Get random leaf material from pre-created materials
    let leaf_material = season_materials.leaf_materials
        [rand::random::<usize>() % season_materials.leaf_materials.len()].clone();

    // Use pre-created diamond mesh for the leaf particle
    let leaf_mesh = season_materials.leaf_mesh.clone();

    // Create leaf entity with 3D mesh
    commands.spawn((
        Mesh3d(leaf_mesh),
        MeshMaterial3d(leaf_material),
        Transform::from_translation(position).with_scale(Vec3::splat(size)),
        WeatherParticle {
            age: 0.0,
            lifetime,
            velocity: Vec3::new(
                (rand::random::<f32>() - 0.5) * fall_settings.drift_factor,
                -fall_settings.fall_speed,
                (rand::random::<f32>() - 0.5) * fall_settings.drift_factor,
            ),
            base_size: size,
            rotation: rand::random::<f32>() * std::f32::consts::TAU,
            rotation_speed: (rand::random::<f32>() - 0.5) * 2.0,
            wobble_phase: rand::random::<f32>() * std::f32::consts::TAU,
            wobble_amplitude: 0.5 + rand::random::<f32>() * 0.5,
        },
        SeasonMarker(Season::Fall),
    ));
}

/// Spawns and updates fall leaf particles
/// Particles use billboard behavior to always face the camera
pub fn fall_particle_system(
    mut commands: Commands,
    settings: Res<SeasonSettings>,
    fall_settings: Res<FallSettings>,
    season_materials: Res<SeasonMaterials>,
    player_query: Query<&GlobalTransform, With<PlayerCharacter>>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut query: Query<(Entity, &mut Transform, &mut WeatherParticle), (Without<PlayerCharacter>, Without<Camera3d>)>,
    time: Res<Time>,
) {
    if !settings.enabled || settings.current_season != Season::Fall {
        return;
    }

    let dt = time.delta_secs();

    // Get player position for player-relative spawning
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    let player_pos = player_transform.translation();

    // Spawn new leaf particles
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

            let size_range = fall_settings.leaf_size_range;
            let size = size_range.0 + rand::random::<f32>() * (size_range.1 - size_range.0);

            let lifetime_range = fall_settings.lifetime_range;
            let lifetime = lifetime_range.0 + rand::random::<f32>() * (lifetime_range.1 - lifetime_range.0);

            // Get random leaf material from pre-created materials
            let leaf_material = season_materials.leaf_materials
                [rand::random::<usize>() % season_materials.leaf_materials.len()].clone();

            // Use pre-created diamond mesh for the leaf particle
            let leaf_mesh = season_materials.leaf_mesh.clone();

            // Create leaf entity with 3D mesh
            commands.spawn((
                Mesh3d(leaf_mesh),
                MeshMaterial3d(leaf_material),
                Transform::from_translation(position).with_scale(Vec3::splat(size)),
                WeatherParticle {
                    age: 0.0,
                    lifetime,
                    velocity: Vec3::new(
                        (rand::random::<f32>() - 0.5) * fall_settings.drift_factor,
                        -fall_settings.fall_speed,
                        (rand::random::<f32>() - 0.5) * fall_settings.drift_factor,
                    ),
                    base_size: size,
                    rotation: rand::random::<f32>() * std::f32::consts::TAU,
                    rotation_speed: (rand::random::<f32>() - 0.5) * 2.0,
                    wobble_phase: rand::random::<f32>() * std::f32::consts::TAU,
                    wobble_amplitude: 0.5 + rand::random::<f32>() * 0.5,
                },
                SeasonMarker(Season::Fall),
            ));
        }
    }

    // Get camera transform for billboard behavior
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    let camera_pos = camera_transform.translation();

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

        // Update wobble
        particle.wobble_phase += dt * fall_settings.wobble_frequency;
        let wobble = (particle.wobble_phase.sin() * particle.wobble_amplitude)
            * settings.wind_strength;

        // Apply wind and wobble
        let wind = Vec3::new(
            settings.wind_direction.x * settings.wind_strength,
            0.0,
            settings.wind_direction.y * settings.wind_strength,
        );

        transform.translation +=
            (particle.velocity + wind + Vec3::new(wobble, 0.0, wobble * 0.5)) * dt;

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
