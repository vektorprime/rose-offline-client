use bevy::pbr::MeshMaterial3d;
use bevy::{prelude::*, render::alpha::AlphaMode};
use rand::Rng;

use crate::components::{
    BoatState, BowSprayParticle, PlayerCharacter, WakeEmitter, WakeParticle, WakeSource,
};
use crate::graphics::GraphicsSettings;

#[derive(Resource)]
pub struct BoatWakeAssets {
    pub mesh: Handle<Mesh>,
    pub wake_material: Handle<StandardMaterial>,
    pub spray_material: Handle<StandardMaterial>,
}

pub fn setup_boat_wake_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Mesh::from(bevy::math::primitives::Plane3d::new(
        Vec3::Y,
        Vec2::splat(0.5),
    )));

    let wake_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.9, 0.95, 1.0, 0.4),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    let spray_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.96, 0.98, 1.0, 0.55),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    commands.insert_resource(BoatWakeAssets {
        mesh,
        wake_material,
        spray_material,
    });
}

pub fn ensure_boat_wake_emitter_system(
    mut commands: Commands,
    query: Query<(Entity, &BoatState, Option<&WakeEmitter>), With<PlayerCharacter>>,
) {
    for (entity, boat, wake_emitter) in query.iter() {
        if boat.active && wake_emitter.is_none() {
            commands.entity(entity).insert(WakeEmitter::default());
        } else if !boat.active && wake_emitter.is_some() {
            commands.entity(entity).remove::<WakeEmitter>();
        }
    }
}

fn clone_with_alpha(
    materials: &mut Assets<StandardMaterial>,
    base_handle: &Handle<StandardMaterial>,
    alpha: f32,
) -> Handle<StandardMaterial> {
    let mut material = materials.get(base_handle).cloned().unwrap_or_default();
    let rgba = material.base_color.to_srgba();
    material.base_color = Color::srgba(rgba.red, rgba.green, rgba.blue, alpha);
    materials.add(material)
}

pub fn boat_wake_spawn_system(
    time: Res<Time>,
    graphics_settings: Res<GraphicsSettings>,
    wake_assets: Res<BoatWakeAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut boat_query: Query<
        (Entity, &BoatState, &Transform, &mut WakeEmitter),
        With<PlayerCharacter>,
    >,
    wake_particles: Query<&WakeSource, With<WakeParticle>>,
    spray_particles: Query<&WakeSource, With<BowSprayParticle>>,
    mut commands: Commands,
) {
    let camera_position = camera_query.iter().next().map(|t| t.translation());

    for (boat_entity, boat, boat_transform, mut wake_emitter) in boat_query.iter_mut() {
        if !boat.active {
            continue;
        }

        if let Some(camera_position) = camera_position {
            let distance_to_camera = camera_position.distance(boat_transform.translation);
            if distance_to_camera > 50.0 {
                continue;
            }
        }

        let speed_ratio = (boat.speed / boat.max_speed.max(0.1)).clamp(0.0, 1.0);
        if speed_ratio <= 0.02 {
            continue;
        }

        wake_emitter.spawn_timer.tick(time.delta());
        wake_emitter.spray_spawn_timer.tick(time.delta());

        let current_wake_count = wake_particles
            .iter()
            .filter(|source| source.boat_entity == boat_entity)
            .count();
        let current_spray_count = spray_particles
            .iter()
            .filter(|source| source.boat_entity == boat_entity)
            .count();

        let forward = Vec3::new(boat.heading.sin(), 0.0, -boat.heading.cos()).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let center = boat_transform.translation;

        if graphics_settings.sailing.wake_particles_enabled
            && wake_emitter.spawn_timer.just_finished()
            && current_wake_count < wake_emitter.max_particles
        {
            let wake_scale = 0.3 + speed_ratio * 0.4;
            let wake_lifetime_secs = 2.0;

            for side in [-1.0f32, 1.0f32] {
                let spawn_pos =
                    center - forward * 1.5 + right * side * 0.9 + Vec3::new(0.0, -0.03, 0.0);
                let wake_dir = (-forward + right * side * 0.3).normalize_or_zero();
                let velocity = wake_dir * (boat.speed * 0.3);

                let particle_material = clone_with_alpha(
                    &mut materials,
                    &wake_assets.wake_material,
                    (0.18 + speed_ratio * 0.35).clamp(0.0, 1.0),
                );

                commands.spawn((
                    WakeSource { boat_entity },
                    WakeParticle {
                        velocity,
                        lifetime: Timer::from_seconds(wake_lifetime_secs, TimerMode::Once),
                        initial_alpha: (0.18 + speed_ratio * 0.35).clamp(0.0, 1.0),
                        initial_scale: wake_scale,
                    },
                    Mesh3d(wake_assets.mesh.clone()),
                    MeshMaterial3d(particle_material),
                    Transform::from_translation(spawn_pos)
                        .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                        .with_scale(Vec3::splat(wake_scale)),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                ));
            }
        }

        if graphics_settings.sailing.bow_spray_enabled
            && speed_ratio > 0.5
            && wake_emitter.spray_spawn_timer.just_finished()
            && current_spray_count < wake_emitter.max_spray_particles
        {
            let mut rng = rand::thread_rng();
            let spray_burst = rng.gen_range(3..=5);
            for _ in 0..spray_burst {
                if spray_particles
                    .iter()
                    .filter(|source| source.boat_entity == boat_entity)
                    .count()
                    >= wake_emitter.max_spray_particles
                {
                    break;
                }

                let side_offset = rng.gen_range(-0.35..0.35);
                let vertical_jitter = rng.gen_range(-0.02..0.06);
                let spawn_pos = center
                    + forward * 1.55
                    + right * side_offset
                    + Vec3::new(0.0, 0.03 + vertical_jitter, 0.0);

                let upward = rng.gen_range(2.0..4.0);
                let backward = boat.speed * rng.gen_range(0.25..0.45);
                let spray_velocity = Vec3::new(0.0, upward, 0.0) + (-forward * backward);

                let particle_material = clone_with_alpha(
                    &mut materials,
                    &wake_assets.spray_material,
                    rng.gen_range(0.35..0.65),
                );
                let initial_scale = rng.gen_range(0.1..0.2);
                let lifetime = rng.gen_range(0.3..0.6);

                commands.spawn((
                    WakeSource { boat_entity },
                    BowSprayParticle {
                        velocity: spray_velocity,
                        lifetime: Timer::from_seconds(lifetime, TimerMode::Once),
                        initial_alpha: rng.gen_range(0.35..0.65),
                        initial_scale,
                    },
                    Mesh3d(wake_assets.mesh.clone()),
                    MeshMaterial3d(particle_material),
                    Transform::from_translation(spawn_pos)
                        .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                        .with_scale(Vec3::splat(initial_scale)),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                ));
            }
        }
    }
}

pub fn boat_wake_update_system(
    time: Res<Time>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut wake_query: Query<
        (
            Entity,
            &mut WakeParticle,
            &mut Transform,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Without<BowSprayParticle>,
    >,
    mut spray_query: Query<
        (
            Entity,
            &mut BowSprayParticle,
            &mut Transform,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Without<WakeParticle>,
    >,
) {
    let dt = time.delta_secs();

    for (entity, mut particle, mut transform, material_handle) in wake_query.iter_mut() {
        particle.lifetime.tick(time.delta());
        if particle.lifetime.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        transform.translation += particle.velocity * dt;
        particle.velocity *= 0.97;

        let life_t = particle.lifetime.fraction();
        let current_alpha = (particle.initial_alpha * (1.0 - life_t)).clamp(0.0, 1.0);
        let current_scale = particle.initial_scale * (1.0 + life_t * 0.5);
        transform.scale = Vec3::splat(current_scale);

        if let Some(material) = materials.get_mut(&material_handle.0) {
            let rgba = material.base_color.to_srgba();
            material.base_color = Color::srgba(rgba.red, rgba.green, rgba.blue, current_alpha);
        }
    }

    for (entity, mut particle, mut transform, material_handle) in spray_query.iter_mut() {
        particle.lifetime.tick(time.delta());
        if particle.lifetime.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        particle.velocity.y -= 9.81 * 0.45 * dt;
        transform.translation += particle.velocity * dt;
        particle.velocity *= 0.93;

        let life_t = particle.lifetime.fraction();
        let current_alpha = (particle.initial_alpha * (1.0 - life_t)).clamp(0.0, 1.0);
        let current_scale = particle.initial_scale * (1.0 + life_t * 0.35);
        transform.scale = Vec3::splat(current_scale);

        if let Some(material) = materials.get_mut(&material_handle.0) {
            let rgba = material.base_color.to_srgba();
            material.base_color = Color::srgba(rgba.red, rgba.green, rgba.blue, current_alpha);
        }
    }
}
