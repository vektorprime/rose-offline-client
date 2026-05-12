use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::alpha::AlphaMode;
use bevy_mesh::{Indices, PrimitiveTopology};
use rose_game_common::components::MoveMode;
use std::collections::HashSet;

use crate::components::{
    BoatModel, BoatState, CharacterModel, Command, Dead, FacingDirection, PlayerCharacter,
    Position, SailMesh, SailSide,
};
use crate::events::{BoardBoatEvent, ChatboxEvent, DisembarkBoatEvent};
use crate::graphics::{GraphicsSettings, SailQuality};
use crate::render::underwater_effect::{UnderwaterVolumes, WaterVolume};
use crate::resources::CurrentZone;
use crate::zone_loader::ZoneLoaderAsset;

const OCEAN_ZONE_ID: u16 = 200;
const BOARD_NEAR_WATER_DISTANCE_M: f32 = 10.0;
const SHORE_HEIGHT_MARGIN_CM: f32 = 50.0;
const SHORE_SEARCH_STEP_CM: f32 = 200.0;
const SHORE_SEARCH_MAX_CM: f32 = 2000.0;

fn distance_to_volume_horizontal_m(position_cm: Vec3, volume: &WaterVolume) -> f32 {
    let world_x = position_cm.x / 100.0;
    let world_z = -position_cm.y / 100.0;

    let dx = (world_x - volume.center.x).abs() - volume.half_extents.x;
    let dz = (world_z - volume.center.z).abs() - volume.half_extents.y;
    let dx = dx.max(0.0);
    let dz = dz.max(0.0);
    (dx * dx + dz * dz).sqrt()
}

fn nearest_water_surface_height_cm(
    position_cm: Vec3,
    underwater_volumes: &UnderwaterVolumes,
) -> Option<f32> {
    underwater_volumes
        .volumes
        .iter()
        .map(|volume| {
            (
                distance_to_volume_horizontal_m(position_cm, volume),
                volume.surface_y * 100.0,
            )
        })
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, surface_cm)| surface_cm)
}

fn is_near_water_plane(
    position_cm: Vec3,
    underwater_volumes: &UnderwaterVolumes,
    max_distance_m: f32,
) -> bool {
    underwater_volumes
        .volumes
        .iter()
        .any(|volume| distance_to_volume_horizontal_m(position_cm, volume) <= max_distance_m)
}

fn find_nearest_shore_position(
    current_position_cm: Vec3,
    zone_data: &ZoneLoaderAsset,
    water_height_cm: f32,
) -> Option<Vec3> {
    let directions = [
        Vec2::new(1.0, 0.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, -1.0),
        Vec2::new(1.0, 1.0).normalize(),
        Vec2::new(-1.0, 1.0).normalize(),
        Vec2::new(1.0, -1.0).normalize(),
        Vec2::new(-1.0, -1.0).normalize(),
    ];

    let mut best: Option<(f32, Vec3)> = None;
    let mut distance_cm = SHORE_SEARCH_STEP_CM;
    while distance_cm <= SHORE_SEARCH_MAX_CM {
        for dir in directions.iter() {
            let sample_x = current_position_cm.x + dir.x * distance_cm;
            let sample_y = current_position_cm.y + dir.y * distance_cm;
            let terrain_height_cm = zone_data.get_terrain_height(sample_x, sample_y);

            if terrain_height_cm > water_height_cm + SHORE_HEIGHT_MARGIN_CM {
                let candidate = Vec3::new(sample_x, sample_y, terrain_height_cm + 10.0);
                match best {
                    Some((best_distance, _)) if distance_cm >= best_distance => {}
                    _ => {
                        best = Some((distance_cm, candidate));
                    }
                }
            }
        }

        distance_cm += SHORE_SEARCH_STEP_CM;
    }

    best.map(|(_, position)| position)
}

fn set_character_model_visibility(
    commands: &mut Commands,
    character_model: Option<&CharacterModel>,
    hidden: bool,
) {
    let Some(character_model) = character_model else {
        return;
    };

    for (_, (_, model_entities)) in character_model.model_parts.iter() {
        for &model_entity in model_entities.iter() {
            commands.entity(model_entity).insert(if hidden {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            });
        }
    }
}

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
    mut chatbox_events: MessageWriter<ChatboxEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    underwater_volumes: Res<UnderwaterVolumes>,
    graphics_settings: Res<GraphicsSettings>,
    mut query: Query<
        (
            Entity,
            &mut BoatState,
            &mut Position,
            &FacingDirection,
            Option<&CharacterModel>,
            Option<&Command>,
            Option<&Dead>,
            Option<&MoveMode>,
        ),
        With<PlayerCharacter>,
    >,
) {
    let zone_id = current_zone.as_ref().map(|zone| zone.id.get());
    let zone_data = current_zone
        .as_ref()
        .and_then(|zone| zone_loader_assets.get(&zone.handle));

    let board_set: HashSet<Entity> = board_events.read().map(|event| event.entity).collect();
    let disembark_set: HashSet<Entity> =
        disembark_events.read().map(|event| event.entity).collect();

    let mut target_entities = board_set.clone();
    target_entities.extend(disembark_set.iter().copied());

    for entity in target_entities {
        let has_board = board_set.contains(&entity);
        let has_disembark = disembark_set.contains(&entity);

        if let Ok((
            entity,
            mut boat_state,
            mut position,
            facing,
            character_model,
            command,
            dead,
            move_mode,
        )) = query.get_mut(entity)
        {
            if has_disembark {
                if !boat_state.active {
                    continue;
                }

                let water_height_cm = if boat_state.water_height_cm.abs() > f32::EPSILON {
                    boat_state.water_height_cm
                } else {
                    position.z
                };

                let Some(zone_data) = zone_data else {
                    chatbox_events.write(ChatboxEvent::System(
                        "Cannot disembark: zone data is not available.".to_string(),
                    ));
                    continue;
                };

                if let Some(shore_position) =
                    find_nearest_shore_position(position.position, zone_data, water_height_cm)
                {
                    position.position = shore_position;
                    boat_state.active = false;
                    boat_state.speed = 0.0;
                    boat_state.rudder = 0.0;

                    if let Some(model_root_entity) = boat_state.model_root_entity.take() {
                        commands.entity(model_root_entity).despawn();
                    }

                    set_character_model_visibility(&mut commands, character_model, false);
                } else {
                    chatbox_events.write(ChatboxEvent::System(
                        "Cannot disembark here. Sail closer to a dock or shore.".to_string(),
                    ));
                }

                continue;
            }

            if has_board {
                if boat_state.active {
                    chatbox_events.write(ChatboxEvent::System(
                        "Already sailing. Press E near shore to disembark.".to_string(),
                    ));
                    continue;
                }

                // REMOVED: Dead check - players can now use /boat while dead
                // if dead.is_some() {
                //     chatbox_events.write(ChatboxEvent::System(
                //         "Cannot board while dead.".to_string(),
                //     ));
                //     continue;
                // }

                // REMOVED: Combat check - players can now use /boat while in combat
                // if command.map_or(false, |command| {
                //     matches!(command, Command::Attack(_) | Command::CastSkill(_))
                // }) {
                //     chatbox_events.write(ChatboxEvent::System(
                //         "Cannot board while in combat.".to_string(),
                //     ));
                //     continue;
                // }

                // REMOVED: Zone restriction - players can now use /boat in any zone
                // if !matches!(zone_id, Some(id) if id == OCEAN_ZONE_ID) {
                //     chatbox_events.write(ChatboxEvent::System(
                //         "Cannot board here: sailing is only available in the ocean zone.".to_string(),
                //     ));
                //     continue;
                // }

                // REMOVED: Driving check - players can now use /boat while driving
                // if move_mode.map_or(false, |move_mode| matches!(move_mode, MoveMode::Drive)) {
                //     chatbox_events.write(ChatboxEvent::System(
                //         "Cannot board while driving.".to_string(),
                //     ));
                //     continue;
                // }

                // REMOVED: Water proximity check - players can now use /boat anywhere
                // if !is_near_water_plane(
                //     position.position,
                //     &underwater_volumes,
                //     BOARD_NEAR_WATER_DISTANCE_M,
                // ) {
                //     chatbox_events.write(ChatboxEvent::System(
                //         "Cannot board: move closer to water.".to_string(),
                //     ));
                //     continue;
                // }

                boat_state.active = true;
                boat_state.rider_entity = Some(entity);
                boat_state.heading = facing.actual;
                boat_state.speed = 0.0;
                boat_state.sail_trim = std::f32::consts::FRAC_PI_4;
                boat_state.rudder = 0.0;
                boat_state.water_height_cm =
                    nearest_water_surface_height_cm(position.position, &underwater_volumes)
                        .unwrap_or(position.z);
                position.z = boat_state.water_height_cm;

                let model_root = spawn_boat_visual(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &position,
                    graphics_settings.sailing.sail_deformation_quality,
                );

                boat_state.model_root_entity = Some(model_root);
                commands.entity(entity).add_child(model_root);

                set_character_model_visibility(&mut commands, character_model, true);
            }
        }
    }
}

fn create_subdivided_sail_mesh(
    width: f32,
    height: f32,
    subdivisions: u32,
) -> (Mesh, Vec<[f32; 3]>) {
    if subdivisions == 0 {
        let positions = vec![
            [-0.5 * width, 0.0, 0.0],
            [0.5 * width, 0.0, 0.0],
            [-0.5 * width, height, 0.0],
            [0.5 * width, height, 0.0],
        ];
        let normals = vec![[0.0, 0.0, 1.0]; 4];
        let uvs = vec![[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]];
        let indices = vec![0u32, 2, 1, 1, 2, 3];

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_indices(Indices::U32(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        return (mesh, positions);
    }

    let cols = subdivisions + 1;
    let rows = subdivisions + 1;
    let mut positions = Vec::with_capacity((cols * rows) as usize);
    let mut normals = Vec::with_capacity((cols * rows) as usize);
    let mut uvs = Vec::with_capacity((cols * rows) as usize);
    let mut indices = Vec::new();

    for row in 0..rows {
        for col in 0..cols {
            let u = col as f32 / subdivisions as f32;
            let v = row as f32 / subdivisions as f32;

            positions.push([(u - 0.5) * width, v * height, 0.0]);
            normals.push([0.0, 0.0, 1.0]);
            uvs.push([u, 1.0 - v]);
        }
    }

    for row in 0..(rows - 1) {
        for col in 0..(cols - 1) {
            let tl = row * cols + col;
            let tr = tl + 1;
            let bl = tl + cols;
            let br = bl + 1;
            indices.extend_from_slice(&[tl, bl, tr]);
            indices.extend_from_slice(&[tr, bl, br]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_indices(Indices::U32(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    (mesh, positions)
}

fn create_sail_mesh_for_quality(
    width: f32,
    height: f32,
    quality: SailQuality,
) -> (Mesh, Vec<[f32; 3]>, u32) {
    match quality {
        SailQuality::Low => {
            let (mesh, base_positions) = create_subdivided_sail_mesh(width, height, 0);
            (mesh, base_positions, 0)
        }
        SailQuality::Medium => {
            let (mesh, base_positions) = create_subdivided_sail_mesh(width, height, 4);
            (mesh, base_positions, 4)
        }
        SailQuality::High => {
            let (mesh, base_positions) = create_subdivided_sail_mesh(width, height, 8);
            (mesh, base_positions, 8)
        }
    }
}

fn spawn_boat_visual(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: &Position,
    sail_quality: SailQuality,
) -> Entity {
    let hull_core_mesh = meshes.add(Mesh::from(Cuboid::new(0.9, 0.45, 3.0)));
    let hull_side_mesh = meshes.add(Mesh::from(Cuboid::new(0.08, 0.42, 2.6)));
    let bow_stem_mesh = meshes.add(Mesh::from(Cuboid::new(0.22, 0.55, 0.95)));
    let deck_mesh = meshes.add(Mesh::from(Cuboid::new(1.0, 0.08, 2.5)));
    let cabin_mesh = meshes.add(Mesh::from(Cuboid::new(0.55, 0.28, 0.75)));
    let mast_mesh = meshes.add(Mesh::from(Cuboid::new(0.08, 2.9, 0.08)));
    let fore_mast_mesh = meshes.add(Mesh::from(Cuboid::new(0.06, 1.9, 0.06)));
    let boom_mesh = meshes.add(Mesh::from(Cuboid::new(0.06, 0.06, 1.35)));
    let bowsprit_mesh = meshes.add(Mesh::from(Cuboid::new(0.05, 0.05, 0.9)));
    let (main_sail_mesh_data, main_sail_base_positions, main_sail_subdivisions) =
        create_sail_mesh_for_quality(2.25, 2.9, sail_quality);
    let main_sail_mesh = meshes.add(main_sail_mesh_data);
    let (jib_sail_mesh_data, jib_sail_base_positions, jib_sail_subdivisions) =
        create_sail_mesh_for_quality(1.2, 1.8, sail_quality);
    let jib_sail_mesh = meshes.add(jib_sail_mesh_data);
    let rudder_mesh = meshes.add(Mesh::from(Cuboid::new(0.06, 0.75, 0.45)));

    let hull_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.23, 0.11),
        perceptual_roughness: 0.86,
        metallic: 0.05,
        ..default()
    });
    let hull_trim_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.47, 0.30),
        perceptual_roughness: 0.72,
        metallic: 0.03,
        ..default()
    });
    let mast_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.58, 0.40, 0.21),
        perceptual_roughness: 0.78,
        ..default()
    });
    let sail_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.94, 0.93, 0.88, 0.9),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        perceptual_roughness: 0.55,
        ..default()
    });
    let sail_trim_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.78, 0.24, 0.18, 0.92),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        perceptual_roughness: 0.5,
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
            Mesh3d(hull_core_mesh.clone()),
            MeshMaterial3d(hull_mat.clone()),
            Transform::from_xyz(0.0, -0.12, 0.0),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Port hull side
    let hull_side_port = commands
        .spawn((
            Mesh3d(hull_side_mesh.clone()),
            MeshMaterial3d(hull_mat.clone()),
            Transform::from_xyz(-0.48, 0.0, -0.12)
                .with_rotation(Quat::from_rotation_z(0.22) * Quat::from_rotation_y(0.05)),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Starboard hull side
    let hull_side_starboard = commands
        .spawn((
            Mesh3d(hull_side_mesh),
            MeshMaterial3d(hull_mat),
            Transform::from_xyz(0.48, 0.0, -0.12)
                .with_rotation(Quat::from_rotation_z(-0.22) * Quat::from_rotation_y(-0.05)),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Bow stem pieces to taper the front and avoid a raft silhouette
    let bow_center = commands
        .spawn((
            Mesh3d(bow_stem_mesh.clone()),
            MeshMaterial3d(hull_trim_mat.clone()),
            Transform::from_xyz(0.0, 0.05, -1.6).with_rotation(
                Quat::from_rotation_x(-0.12) * Quat::from_rotation_y(std::f32::consts::PI),
            ),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let bow_port = commands
        .spawn((
            Mesh3d(bow_stem_mesh.clone()),
            MeshMaterial3d(hull_trim_mat.clone()),
            Transform::from_xyz(-0.22, 0.02, -1.52)
                .with_rotation(Quat::from_rotation_y(2.35) * Quat::from_rotation_x(-0.08)),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let bow_starboard = commands
        .spawn((
            Mesh3d(bow_stem_mesh),
            MeshMaterial3d(hull_trim_mat.clone()),
            Transform::from_xyz(0.22, 0.02, -1.52)
                .with_rotation(Quat::from_rotation_y(-2.35) * Quat::from_rotation_x(-0.08)),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let deck_entity = commands
        .spawn((
            Mesh3d(deck_mesh),
            MeshMaterial3d(hull_trim_mat.clone()),
            Transform::from_xyz(0.0, 0.13, -0.1),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let cabin_entity = commands
        .spawn((
            Mesh3d(cabin_mesh),
            MeshMaterial3d(hull_trim_mat.clone()),
            Transform::from_xyz(0.0, 0.30, 0.45),
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
            Transform::from_xyz(0.0, 1.35, -0.25),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let fore_mast_entity = commands
        .spawn((
            Mesh3d(fore_mast_mesh),
            MeshMaterial3d(hull_trim_mat.clone()),
            Transform::from_xyz(0.0, 1.0, -1.1),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let boom_entity = commands
        .spawn((
            Mesh3d(boom_mesh),
            MeshMaterial3d(hull_trim_mat.clone()),
            Transform::from_xyz(0.0, 0.95, 0.22).with_rotation(Quat::from_rotation_x(-0.35)),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let bowsprit_entity = commands
        .spawn((
            Mesh3d(bowsprit_mesh),
            MeshMaterial3d(hull_trim_mat.clone()),
            Transform::from_xyz(0.0, 0.45, -1.9).with_rotation(Quat::from_rotation_x(-0.18)),
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
                base_positions: main_sail_base_positions,
                width: 2.25,
                height: 2.9,
                subdivisions: main_sail_subdivisions,
            },
            Mesh3d(main_sail_mesh),
            MeshMaterial3d(sail_mat.clone()),
            Transform::from_xyz(0.0, 1.28, -0.30)
                .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let jib_sail_entity = commands
        .spawn((
            SailMesh {
                billow: 0.25,
                side: SailSide::Center,
                base_positions: jib_sail_base_positions,
                width: 1.2,
                height: 1.8,
                subdivisions: jib_sail_subdivisions,
            },
            Mesh3d(jib_sail_mesh),
            MeshMaterial3d(sail_trim_mat),
            Transform::from_xyz(0.0, 1.35, -1.25).with_rotation(
                Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)
                    * Quat::from_axis_angle(Vec3::Y, 0.25),
            ),
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
            Transform::from_xyz(0.0, -0.22, 1.62),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let rider_seat_entity = commands
        .spawn((
            Transform::from_xyz(0.0, 0.52, 0.35),
            GlobalTransform::default(),
            Visibility::Hidden,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    commands.entity(root).add_child(hull_entity);
    commands.entity(root).add_child(hull_side_port);
    commands.entity(root).add_child(hull_side_starboard);
    commands.entity(root).add_child(bow_center);
    commands.entity(root).add_child(bow_port);
    commands.entity(root).add_child(bow_starboard);
    commands.entity(root).add_child(deck_entity);
    commands.entity(root).add_child(cabin_entity);
    commands.entity(root).add_child(mast_entity);
    commands.entity(root).add_child(fore_mast_entity);
    commands.entity(root).add_child(boom_entity);
    commands.entity(root).add_child(bowsprit_entity);
    commands.entity(root).add_child(sail_entity);
    commands.entity(root).add_child(jib_sail_entity);
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
