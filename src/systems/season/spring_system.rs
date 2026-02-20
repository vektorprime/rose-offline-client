use bevy::{
    pbr::MeshMaterial3d,
    prelude::*,
    render::mesh::Mesh3d,
};
use crate::components::{Season, SeasonMarker, WeatherParticle, SpringFlower};
use crate::resources::{SeasonMaterials, SeasonSettings, SpringSettings};

/// Spawns rain particles for spring season
pub fn spring_rain_system(
    mut commands: Commands,
    settings: Res<SeasonSettings>,
    spring_settings: Res<SpringSettings>,
    season_materials: Res<SeasonMaterials>,
    camera_query: Query<&Transform, With<Camera3d>>,
    mut query: Query<(Entity, &mut Transform, &mut WeatherParticle), (Without<SpringFlower>, Without<Camera3d>)>,
    flower_query: Query<(Entity, &SpringFlower)>,
    time: Res<Time>,
) {
    if !settings.enabled || settings.current_season != Season::Spring {
        return;
    }

    let dt = time.delta_secs();

    // Get camera position for camera-relative spawning
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    let camera_pos = camera_transform.translation;

    // Spawn new rain drops
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

            // Use pre-created elongated rain mesh
            let rain_mesh = season_materials.rain_mesh.clone();

            // Use pre-created rain material
            let rain_material = season_materials.rain_material.clone();

            commands.spawn((
                Mesh3d(rain_mesh),
                MeshMaterial3d(rain_material),
                Transform::from_translation(position).with_scale(Vec3::new(
                    spring_settings.rain_drop_size,
                    spring_settings.rain_drop_size * 2.0, // Elongate rain drops
                    spring_settings.rain_drop_size,
                )),
                WeatherParticle {
                    age: 0.0,
                    lifetime: 2.0 + rand::random::<f32>() * 1.0,
                    velocity: Vec3::new(
                        settings.wind_direction.x * settings.wind_strength * 0.5,
                        -spring_settings.rain_speed,
                        settings.wind_direction.y * settings.wind_strength * 0.5,
                    ),
                    base_size: spring_settings.rain_drop_size,
                    rotation: 0.0,
                    rotation_speed: 0.0,
                    wobble_phase: 0.0,
                    wobble_amplitude: 0.0,
                },
                SeasonMarker(Season::Spring),
            ));
        }
    }

    // Update existing particles
    for (entity, mut transform, mut particle) in query.iter_mut() {
        particle.age += dt;

        // Despawn if below ground level (relative to camera) or lifetime exceeded
        if particle.age >= particle.lifetime || transform.translation.y < camera_pos.y - 50.0 {
            commands.entity(entity).despawn();
            continue;
        }

        transform.translation += particle.velocity * dt;
    }

    // Update flowers
    let current_time = time.elapsed_secs();
    for (entity, flower) in flower_query.iter() {
        if current_time - flower.spawn_time > spring_settings.flower_lifetime {
            commands.entity(entity).despawn();
        }
    }
}

/// Spawns flowers on the ground during spring
#[allow(dead_code)]
pub fn spawn_flower_system(
    mut commands: Commands,
    settings: Res<SeasonSettings>,
    spring_settings: Res<SpringSettings>,
    season_materials: Res<SeasonMaterials>,
    camera_query: Query<&Transform, With<Camera3d>>,
    time: Res<Time>,
) {
    if !settings.enabled || settings.current_season != Season::Spring {
        return;
    }

    if rand::random::<f32>() > spring_settings.flower_spawn_chance {
        return;
    }

    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    let camera_pos = camera_transform.translation;

    // Larger spawn area for flowers (100 units radius)
    let offset_x = (rand::random::<f32>() - 0.5) * 200.0;
    let offset_z = (rand::random::<f32>() - 0.5) * 200.0;

    let position = Vec3::new(
        camera_pos.x + offset_x,
        0.1, // Ground level
        camera_pos.z + offset_z,
    );

    // Get random flower material from pre-created materials
    let flower_material = season_materials.flower_materials
        [rand::random::<usize>() % season_materials.flower_materials.len()].clone();

    // Use pre-created circle mesh for flower
    let flower_mesh = season_materials.flower_mesh.clone();

    commands.spawn((
        Mesh3d(flower_mesh),
        MeshMaterial3d(flower_material),
        Transform::from_translation(position)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::splat(2.0)),
        SpringFlower {
            spawn_time: time.elapsed_secs(),
        },
        SeasonMarker(Season::Spring),
    ));
}
