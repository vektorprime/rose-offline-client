use bevy::{
    pbr::MeshMaterial3d,
    prelude::*,
    render::mesh::Mesh3d,
};
use crate::components::{Season, SeasonMarker, WeatherParticle};
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
pub fn fall_particle_system(
    mut commands: Commands,
    settings: Res<SeasonSettings>,
    fall_settings: Res<FallSettings>,
    season_materials: Res<SeasonMaterials>,
    camera_query: Query<&Transform, With<Camera3d>>,
    mut query: Query<(Entity, &mut Transform, &mut WeatherParticle), Without<Camera3d>>,
    time: Res<Time>,
) {
    if !settings.enabled || settings.current_season != Season::Fall {
        return;
    }

    let dt = time.delta_secs();

    // Get camera position for camera-relative spawning
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    let camera_pos = camera_transform.translation;

    // Spawn new leaf particles
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

        // Update rotation
        particle.rotation += particle.rotation_speed * dt;
        transform.rotation = Quat::from_rotation_z(particle.rotation);

        // Note: Fade-out effect removed to avoid ResMut<Assets<StandardMaterial>> conflict
        // Particles will simply disappear at end of lifetime
    }
}
