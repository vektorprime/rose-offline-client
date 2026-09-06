use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::material::AlphaMode;
use bevy_mesh::{Indices, PrimitiveTopology};
use rose_game_common::components::MoveMode;
use rose_game_common::messages::client::ClientMessage;
use std::collections::HashSet;

use crate::components::{
    BoatModel, BoatState, CharacterModel, Command, Dead, FacingDirection, PlayerCharacter,
    Position, SailMesh, SailSide,
};
use crate::events::{BoardBoatEvent, ChatboxEvent, DisembarkBoatEvent};
use crate::graphics::{GraphicsSettings, SailQuality};
use crate::render::underwater_effect::{UnderwaterVolumes, WaterVolume};
use crate::resources::{CurrentZone, GameConnection};
use crate::zone_loader::ZoneLoaderAsset;

pub(crate) const OCEAN_ZONE_ID: u16 = 200;
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

pub(crate) fn nearest_water_surface_height_cm(
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

pub(crate) fn find_nearest_shore_position(
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

pub(crate) fn set_character_model_visibility(
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
    game_connection: Option<Res<GameConnection>>,
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

                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::DisembarkBoat {
                                x: shore_position.x,
                                y: shore_position.y,
                                z: shore_position.z,
                            })
                            .ok();
                    }
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

                // Dead check — prevent boarding while dead
                if dead.is_some() {
                    chatbox_events
                        .write(ChatboxEvent::System("Cannot board while dead.".to_string()));
                    continue;
                }

                // Combat check — prevent boarding while in combat
                if command.map_or(false, |command| {
                    matches!(command, Command::Attack(_) | Command::CastSkill(_))
                }) {
                    chatbox_events.write(ChatboxEvent::System(
                        "Cannot board while in combat.".to_string(),
                    ));
                    continue;
                }

                // Zone restriction — sailing is only available in the ocean zone
                if !matches!(zone_id, Some(id) if id == OCEAN_ZONE_ID) {
                    chatbox_events.write(ChatboxEvent::System(
                        "Cannot board here: sailing is only available in the ocean zone."
                            .to_string(),
                    ));
                    continue;
                }

                // Driving check — prevent boarding while driving a vehicle
                if move_mode.map_or(false, |move_mode| {
                    matches!(move_mode, MoveMode::Drive | MoveMode::Sail)
                }) {
                    chatbox_events.write(ChatboxEvent::System(
                        "Cannot board while driving.".to_string(),
                    ));
                    continue;
                }

                // Water proximity check — must be near a water plane
                if !is_near_water_plane(
                    position.position,
                    &underwater_volumes,
                    BOARD_NEAR_WATER_DISTANCE_M,
                ) {
                    chatbox_events.write(ChatboxEvent::System(
                        "Cannot board: move closer to water.".to_string(),
                    ));
                    continue;
                }

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

                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::BoardBoat {
                            x: position.position.x,
                            y: position.position.y,
                            z: position.position.z,
                        })
                        .ok();
                }
            }
        }
    }
}

fn create_triangular_sail_mesh(
    width: f32,
    height: f32,
    subdivisions: u32,
) -> (Mesh, Vec<[f32; 3]>) {
    if subdivisions == 0 {
        let positions = vec![
            [-0.5 * width, 0.0, 0.0],
            [0.5 * width, 0.0, 0.0],
            [-0.5 * width, height, 0.0],
        ];
        let normals = vec![[0.0, 0.0, 1.0]; 3];
        let uvs = vec![[0.0, 1.0], [1.0, 1.0], [0.0, 0.0]];
        let indices = vec![0u32, 2, 1];

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
            let row_width = width * (1.0 - v);

            positions.push([-0.5 * width + u * row_width, v * height, 0.0]);
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
            let (mesh, base_positions) = create_triangular_sail_mesh(width, height, 0);
            (mesh, base_positions, 0)
        }
        SailQuality::Medium => {
            let (mesh, base_positions) = create_triangular_sail_mesh(width, height, 4);
            (mesh, base_positions, 4)
        }
        SailQuality::High => {
            let (mesh, base_positions) = create_triangular_sail_mesh(width, height, 8);
            (mesh, base_positions, 8)
        }
    }
}

fn create_flat_shaded_mesh(vertices: &[[f32; 3]], faces: &[[usize; 3]]) -> Mesh {
    let mut positions = Vec::with_capacity(faces.len() * 3);
    let mut normals = Vec::with_capacity(faces.len() * 3);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(faces.len() * 3);
    let mut indices = Vec::with_capacity(faces.len() * 3);

    for face in faces {
        let base_index = positions.len() as u32;
        let a = Vec3::from_array(vertices[face[0]]);
        let b = Vec3::from_array(vertices[face[1]]);
        let c = Vec3::from_array(vertices[face[2]]);
        let normal = (b - a).cross(c - a).normalize_or_zero();

        positions.push(a.to_array());
        positions.push(b.to_array());
        positions.push(c.to_array());
        normals.extend_from_slice(&[normal.to_array(); 3]);
        uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]]);
        indices.extend_from_slice(&[base_index, base_index + 1, base_index + 2]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_indices(Indices::U32(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh
}

/// Scale applied to the boat visual root so the boat reads as a full-sized
/// sailboat next to the player character. All part offsets scale with it.
pub const BOAT_VISUAL_SCALE: f32 = 1.6;

fn create_hull_mesh() -> Mesh {
    // Stations from stern (+Z) to bow (-Z):
    // (z, half_beam, sheer_y, chine_half, chine_y, keel_y)
    // The sheer is low amidships and rises toward bow and stern; the keel
    // rocks up toward the ends and the beam tapers to a flared bow and a
    // flat transom.
    let stations = [
        (2.05, 0.58, 0.42, 0.42, -0.18, -0.48), // transom (stern)
        (1.45, 0.78, 0.30, 0.56, -0.24, -0.58),
        (0.85, 0.98, 0.24, 0.72, -0.28, -0.64),
        (0.25, 1.08, 0.22, 0.80, -0.30, -0.66), // max beam amidships
        (-0.35, 1.06, 0.24, 0.78, -0.29, -0.65),
        (-0.95, 0.92, 0.28, 0.68, -0.26, -0.60),
        (-1.55, 0.66, 0.36, 0.48, -0.22, -0.52),
        (-2.10, 0.30, 0.50, 0.20, -0.16, -0.40), // raked bow
    ];

    let mut vertices = Vec::with_capacity(stations.len() * 5);
    for (z, top_half, sheer_y, chine_half, chine_y, keel_y) in stations {
        vertices.push([-top_half, sheer_y, z]);
        vertices.push([top_half, sheer_y, z]);
        vertices.push([-chine_half, chine_y, z]);
        vertices.push([chine_half, chine_y, z]);
        vertices.push([0.0, keel_y, z]);
    }

    let mut faces: Vec<[usize; 3]> = Vec::new();
    for station in 0..(stations.len() - 1) {
        let current = station * 5;
        let next = (station + 1) * 5;

        let tl0 = current;
        let tr0 = current + 1;
        let cl0 = current + 2;
        let cr0 = current + 3;
        let k0 = current + 4;

        let tl1 = next;
        let tr1 = next + 1;
        let cl1 = next + 2;
        let cr1 = next + 3;
        let k1 = next + 4;

        faces.extend_from_slice(&[
            [tl0, cl0, tl1],
            [tl1, cl0, cl1],
            [tr0, tr1, cr0],
            [cr0, tr1, cr1],
            [cl0, k0, cl1],
            [cl1, k0, k1],
            [k0, cr0, k1],
            [k1, cr0, cr1],
        ]);
    }

    // Flat transom cap at the stern (faces outward, +Z).
    faces.extend_from_slice(&[[0, 4, 2], [0, 1, 4], [1, 3, 4]]);

    // Pointed wedge cap at the bow (faces outward, -Z).
    let last = (stations.len() - 1) * 5;
    faces.extend_from_slice(&[
        [last, last + 2, last + 4],
        [last, last + 4, last + 1],
        [last + 1, last + 4, last + 3],
    ]);

    create_flat_shaded_mesh(&vertices, &faces)
}

fn spawn_visual_part(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
) -> Entity {
    commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            transform,
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id()
}

pub(crate) fn spawn_boat_visual(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    _position: &Position,
    sail_quality: SailQuality,
) -> Entity {
    let hull_mesh = meshes.add(create_hull_mesh());
    let deck_mesh = meshes.add(Mesh::from(Cuboid::new(1.90, 0.08, 2.90)));
    let cockpit_mesh = meshes.add(Mesh::from(Cuboid::new(0.74, 0.07, 0.70)));
    let cabin_mesh = meshes.add(Mesh::from(Cuboid::new(0.64, 0.32, 0.68)));
    let cabin_roof_mesh = meshes.add(Mesh::from(Cuboid::new(0.78, 0.08, 0.82)));
    let windshield_mesh = meshes.add(Mesh::from(Cuboid::new(0.56, 0.18, 0.035)));
    let mast_mesh = meshes.add(Mesh::from(Cuboid::new(0.07, 3.25, 0.07)));
    let boom_mesh = meshes.add(Mesh::from(Cuboid::new(0.055, 0.055, 1.78)));
    let bowsprit_mesh = meshes.add(Mesh::from(Cuboid::new(0.055, 0.055, 1.05)));
    let stay_front_mesh = meshes.add(Mesh::from(Cuboid::new(0.024, 0.024, 3.0)));
    let stay_back_mesh = meshes.add(Mesh::from(Cuboid::new(0.024, 0.024, 3.35)));
    let keel_mesh = meshes.add(Mesh::from(Cuboid::new(0.16, 0.72, 1.62)));
    let rail_mesh = meshes.add(Mesh::from(Cuboid::new(0.045, 0.08, 3.10)));
    let batten_mesh = meshes.add(Mesh::from(Cuboid::new(0.028, 0.028, 0.80)));
    let (main_sail_mesh_data, main_sail_base_positions, main_sail_subdivisions) =
        create_sail_mesh_for_quality(1.75, 2.65, sail_quality);
    let main_sail_mesh = meshes.add(main_sail_mesh_data);
    let (jib_sail_mesh_data, jib_sail_base_positions, jib_sail_subdivisions) =
        create_sail_mesh_for_quality(1.15, 1.85, sail_quality);
    let jib_sail_mesh = meshes.add(jib_sail_mesh_data);
    let rudder_mesh = meshes.add(Mesh::from(Cuboid::new(0.08, 0.82, 0.42)));

    let hull_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.13, 0.16, 0.18),
        perceptual_roughness: 0.82,
        metallic: 0.05,
        ..default()
    });
    let hull_trim_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.58, 0.37, 0.18),
        perceptual_roughness: 0.74,
        metallic: 0.03,
        ..default()
    });
    let deck_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.74, 0.61, 0.42),
        perceptual_roughness: 0.86,
        metallic: 0.0,
        ..default()
    });
    let cockpit_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.055, 0.065, 0.07),
        perceptual_roughness: 0.9,
        ..default()
    });
    let mast_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.46, 0.28, 0.12),
        perceptual_roughness: 0.78,
        ..default()
    });
    let sail_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.90, 0.88, 0.78),
        alpha_mode: AlphaMode::Opaque,
        cull_mode: None,
        perceptual_roughness: 0.92,
        ..default()
    });
    let sail_trim_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.83, 0.80, 0.68),
        alpha_mode: AlphaMode::Opaque,
        cull_mode: None,
        perceptual_roughness: 0.94,
        ..default()
    });
    let sail_detail_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.58, 0.48),
        perceptual_roughness: 0.92,
        ..default()
    });
    let glass_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.18, 0.22),
        perceptual_roughness: 0.35,
        reflectance: 0.35,
        ..default()
    });
    let rudder_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.13, 0.07),
        perceptual_roughness: 0.9,
        ..default()
    });

    let root = commands
        .spawn((
            Transform::from_scale(Vec3::splat(BOAT_VISUAL_SCALE)),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let hull_entity = spawn_visual_part(
        commands,
        hull_mesh,
        hull_mat.clone(),
        Transform::from_xyz(0.0, -0.02, 0.0),
    );
    let deck_entity = spawn_visual_part(
        commands,
        deck_mesh,
        deck_mat.clone(),
        Transform::from_xyz(0.0, 0.16, 0.04),
    );
    let cockpit_entity = spawn_visual_part(
        commands,
        cockpit_mesh,
        cockpit_mat,
        Transform::from_xyz(0.0, 0.22, 0.76),
    );
    let cabin_entity = spawn_visual_part(
        commands,
        cabin_mesh,
        deck_mat.clone(),
        Transform::from_xyz(0.0, 0.37, 0.12),
    );
    let cabin_roof_entity = spawn_visual_part(
        commands,
        cabin_roof_mesh,
        hull_trim_mat.clone(),
        Transform::from_xyz(0.0, 0.56, 0.12),
    );
    let windshield_entity = spawn_visual_part(
        commands,
        windshield_mesh,
        glass_mat,
        Transform::from_xyz(0.0, 0.47, -0.24),
    );
    let mast_entity = spawn_visual_part(
        commands,
        mast_mesh,
        mast_mat.clone(),
        Transform::from_xyz(0.0, 1.58, -0.28),
    );
    let boom_entity = spawn_visual_part(
        commands,
        boom_mesh,
        mast_mat.clone(),
        Transform::from_xyz(0.0, 0.96, 0.60).with_rotation(Quat::from_rotation_x(-0.05)),
    );
    let bowsprit_entity = spawn_visual_part(
        commands,
        bowsprit_mesh,
        mast_mat.clone(),
        Transform::from_xyz(0.0, 0.42, -2.10).with_rotation(Quat::from_rotation_x(-0.16)),
    );
    let fore_stay_entity = spawn_visual_part(
        commands,
        stay_front_mesh,
        mast_mat.clone(),
        Transform::from_xyz(0.0, 1.84, -1.18).with_rotation(Quat::from_rotation_x(2.22)),
    );
    let back_stay_entity = spawn_visual_part(
        commands,
        stay_back_mesh,
        mast_mat.clone(),
        Transform::from_xyz(0.0, 1.82, 0.84).with_rotation(Quat::from_rotation_x(0.86)),
    );
    let keel_entity = spawn_visual_part(
        commands,
        keel_mesh,
        hull_mat,
        Transform::from_xyz(0.0, -0.50, 0.18),
    );
    let rail_port_entity = spawn_visual_part(
        commands,
        rail_mesh.clone(),
        hull_trim_mat.clone(),
        Transform::from_xyz(-0.92, 0.28, 0.16),
    );
    let rail_starboard_entity = spawn_visual_part(
        commands,
        rail_mesh,
        hull_trim_mat.clone(),
        Transform::from_xyz(0.92, 0.28, 0.16),
    );

    let sail_entity = commands
        .spawn((
            SailMesh {
                billow: 0.35,
                side: SailSide::Center,
                base_positions: main_sail_base_positions,
                width: 1.75,
                height: 2.65,
                subdivisions: main_sail_subdivisions,
            },
            Mesh3d(main_sail_mesh),
            MeshMaterial3d(sail_mat.clone()),
            Transform::from_xyz(0.0, 0.78, 0.60)
                .with_rotation(Quat::from_axis_angle(Vec3::Y, -std::f32::consts::FRAC_PI_2)),
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
                width: 1.15,
                height: 1.85,
                subdivisions: jib_sail_subdivisions,
            },
            Mesh3d(jib_sail_mesh),
            MeshMaterial3d(sail_trim_mat),
            Transform::from_xyz(0.0, 0.78, -1.78)
                .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    let batten_low_entity = spawn_visual_part(
        commands,
        batten_mesh.clone(),
        sail_detail_mat.clone(),
        Transform::from_xyz(0.0, 1.30, 0.88).with_rotation(Quat::from_rotation_x(1.32)),
    );
    let batten_mid_entity = spawn_visual_part(
        commands,
        batten_mesh.clone(),
        sail_detail_mat.clone(),
        Transform::from_xyz(0.0, 1.75, 0.66).with_rotation(Quat::from_rotation_x(1.32)),
    );
    let batten_high_entity = spawn_visual_part(
        commands,
        batten_mesh,
        sail_detail_mat,
        Transform::from_xyz(0.0, 2.18, 0.42).with_rotation(Quat::from_rotation_x(1.32)),
    );

    let rudder_entity = spawn_visual_part(
        commands,
        rudder_mesh,
        rudder_mat,
        Transform::from_xyz(0.0, -0.20, 2.05),
    );

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
    commands.entity(root).add_child(deck_entity);
    commands.entity(root).add_child(cockpit_entity);
    commands.entity(root).add_child(cabin_entity);
    commands.entity(root).add_child(cabin_roof_entity);
    commands.entity(root).add_child(windshield_entity);
    commands.entity(root).add_child(mast_entity);
    commands.entity(root).add_child(boom_entity);
    commands.entity(root).add_child(bowsprit_entity);
    commands.entity(root).add_child(fore_stay_entity);
    commands.entity(root).add_child(back_stay_entity);
    commands.entity(root).add_child(keel_entity);
    commands.entity(root).add_child(rail_port_entity);
    commands.entity(root).add_child(rail_starboard_entity);
    commands.entity(root).add_child(sail_entity);
    commands.entity(root).add_child(jib_sail_entity);
    commands.entity(root).add_child(batten_low_entity);
    commands.entity(root).add_child(batten_mid_entity);
    commands.entity(root).add_child(batten_high_entity);
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
