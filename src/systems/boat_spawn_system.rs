use bevy::prelude::*;
use bevy::render::alpha::AlphaMode;
use std::collections::HashSet;

use crate::components::{
    BoatModel, BoatState, FacingDirection, PlayerCharacter, Position, SailMesh, SailSide,
};
use crate::events::{BoardBoatEvent, DisembarkBoatEvent};

/// Checks if a chat message is a local boat toggle command.
pub fn is_boat_command(message: &str) -> bool {
    message.trim().eq_ignore_ascii_case("/boat")
}

pub fn ensure_boat_state_system(
    mut commands: Commands,
    query: Query<Entity, (With<PlayerCharacter>, Without<BoatState>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(BoatState {
            rider_entity: Some(entity),
            ..default()
        });
    }
}

pub fn boat_toggle_system(
    mut commands: Commands,
    mut board_events: MessageReader<BoardBoatEvent>,
    mut disembark_events: MessageReader<DisembarkBoatEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<(Entity, &mut BoatState, &Position, &FacingDirection), With<PlayerCharacter>>,
) {
    let board_set: HashSet<Entity> = board_events.read().map(|event| event.entity).collect();
    let disembark_set: HashSet<Entity> = disembark_events.read().map(|event| event.entity).collect();

    let mut target_entities = board_set.clone();
    target_entities.extend(disembark_set.iter().copied());

    for entity in target_entities {
        let has_board = board_set.contains(&entity);
        let has_disembark = disembark_set.contains(&entity);

        if let Ok((entity, mut boat_state, position, facing)) = query.get_mut(entity) {
            let should_toggle = has_board && has_disembark;
            let should_enable = has_board && !has_disembark;
            let should_disable = has_disembark && !has_board;

            if (should_toggle && !boat_state.active) || should_enable {
                boat_state.active = true;
                boat_state.rider_entity = Some(entity);
                boat_state.heading = facing.actual;
                boat_state.speed = 0.0;
                boat_state.sail_trim = std::f32::consts::FRAC_PI_4;
                boat_state.rudder = 0.0;
                boat_state.water_height_cm = position.z;

                let model_root = spawn_boat_visual(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    position,
                );

                boat_state.model_root_entity = Some(model_root);
                commands.entity(entity).add_child(model_root);
            } else if (should_toggle && boat_state.active) || should_disable {
                boat_state.active = false;
                boat_state.speed = 0.0;
                boat_state.rudder = 0.0;

                if let Some(model_root_entity) = boat_state.model_root_entity.take() {
                    commands.entity(model_root_entity).despawn();
                }
            }
        }
    }
}

fn spawn_boat_visual(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: &Position,
) -> Entity {
    let hull_mesh = meshes.add(Mesh::from(Cuboid::new(1.6, 0.5, 4.0)));
    let mast_mesh = meshes.add(Mesh::from(Cuboid::new(0.08, 3.0, 0.08)));
    let sail_mesh = meshes.add(Mesh::from(bevy::math::primitives::Plane3d::new(
        Vec3::Z,
        Vec2::new(2.0, 2.8),
    )));
    let rudder_mesh = meshes.add(Mesh::from(Cuboid::new(0.05, 0.6, 0.5)));

    let hull_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.28, 0.12),
        perceptual_roughness: 0.85,
        metallic: 0.05,
        ..default()
    });
    let mast_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.38, 0.20),
        perceptual_roughness: 0.8,
        ..default()
    });
    let sail_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.95, 0.92, 0.9),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        perceptual_roughness: 0.6,
        ..default()
    });
    let rudder_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.20, 0.10),
        perceptual_roughness: 0.9,
        ..default()
    });

    let root = commands
        .spawn((
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let hull_entity = commands
        .spawn((
            Mesh3d(hull_mesh),
            MeshMaterial3d(hull_mat),
            Transform::from_xyz(0.0, -0.15, 0.0),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let mast_entity = commands
        .spawn((
            Mesh3d(mast_mesh),
            MeshMaterial3d(mast_mat),
            Transform::from_xyz(0.0, 1.2, -0.3),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let sail_entity = commands
        .spawn((
            SailMesh {
                billow: 0.35,
                side: SailSide::Center,
            },
            Mesh3d(sail_mesh),
            MeshMaterial3d(sail_mat),
            Transform::from_xyz(0.0, 1.2, -0.35)
                .with_rotation(Quat::from_axis_angle(Vec3::X, std::f32::consts::FRAC_PI_2)),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let rudder_entity = commands
        .spawn((
            Mesh3d(rudder_mesh),
            MeshMaterial3d(rudder_mat),
            Transform::from_xyz(0.0, -0.25, 1.8),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let rider_seat_entity = commands
        .spawn((
            Transform::from_xyz(0.0, 0.45, 0.4),
            GlobalTransform::default(),
            Visibility::Hidden,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    commands.entity(root).add_child(hull_entity);
    commands.entity(root).add_child(mast_entity);
    commands.entity(root).add_child(sail_entity);
    commands.entity(root).add_child(rudder_entity);
    commands.entity(root).add_child(rider_seat_entity);
    commands.entity(root).insert(BoatModel {
        root_entity: root,
        hull_entity,
        mast_entity,
        sail_entity,
        rudder_entity,
        rider_seat_entity,
    });

    root
}

