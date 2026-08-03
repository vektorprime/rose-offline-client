//! Client-side sea monster (shark) spawner, AI and basic boat combat for the
//! ocean zone (zone 200). Sharks cruise the open water, hunt the player's
//! boat within detection range and bite its hull; when the hull reaches zero
//! the boat sinks and the player is disembarked on the nearest shore via the
//! existing DisembarkBoatEvent path. Monsters are purely client-side.

use bevy::asset::RenderAssetUsages;
use bevy::math::{Vec3, Vec3Swizzles};
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::prelude::*;
use bevy_mesh::{Indices, Mesh, PrimitiveTopology};
use rand::Rng;

use crate::components::{
    BoatState, ClientEntity, ClientEntityId, ClientEntityName, ClientEntityType, ModelHeight,
    MonsterSeparation, PlayerCharacter, Position, Zone,
};
use crate::events::{ChatboxEvent, DisembarkBoatEvent, ZoneEvent};
use crate::resources::CurrentZone;
use crate::sailing::{normalize_angle, shortest_angle_delta};
use crate::systems::OCEAN_ZONE_ID;
use crate::zone_loader::ZoneLoaderAsset;

/// Map center in game cm. The zone entity transform (5200, 0, -5200) places
/// this point at the world origin, so zone-local meters for a game cm
/// position (x, y, z) are ((x - CENTER)/100, z/100, -(y - CENTER)/100).
const ZONE_CENTER_CM: f32 = 520000.0;
/// Player spawn point in game cm (on the main island).
const PLAYER_SPAWN_CM: Vec3 = Vec3::new(520000.0, 520000.0, 0.0);

/// Open-water rectangle in game cm (file coords 3600..6800 m on each axis).
const WATER_MIN_CM: f32 = 360000.0;
const WATER_MAX_CM: f32 = 680000.0;

/// Number of sharks spawned when zone 200 loads.
const MONSTER_COUNT: usize = 10;
/// Base for synthetic client entity ids used by sea monsters (docks use
/// 0x4000_0000; keep sea monsters in a separate range).
const MONSTER_ENTITY_ID_BASE: usize = 0x5000_0000;

/// Spawns must stay at least 800 m away from the player spawn point.
const SPAWN_MIN_DISTANCE_CM: f32 = 80000.0;
/// Waypoints stay in open water, at least 450 m from the central island.
const WAYPOINT_MIN_DISTANCE_CM: f32 = 45000.0;
const WAYPOINT_REACH_CM: f32 = 5000.0;

/// Horizontal detection range in cm (400 m).
const DETECT_RANGE_CM: f32 = 40000.0;
/// A hunting shark loses interest at 1.5x detection range.
const LOSE_RANGE_MULTIPLIER: f32 = 1.5;
/// Bite range in cm (1.8 m).
const BITE_RANGE_CM: f32 = 180.0;
/// Hull damage per bite (4 bites sink a full 100 HP hull).
const BITE_DAMAGE: f32 = 25.0;
/// Seconds between bites.
const ATTACK_COOLDOWN_SECS: f32 = 2.0;
/// Max turn rate in rad/s.
const TURN_RATE_RAD_PER_SEC: f32 = 2.5;
/// Terrain above this height (cm) pushes a shark back to open water.
const GROUND_CLEARANCE_CM: f32 = -30.0;

/// Runtime state of one sea monster.
#[derive(Component)]
pub struct SeaMonster {
    pub state: SeaMonsterState,
    /// The player boat entity currently hunted.
    pub target: Option<Entity>,
    pub attack_cooldown: Timer,
    /// Cruise waypoint in game cm.
    pub waypoint: Vec3,
    /// Current yaw (heading) in radians; world direction is (sin, cos).
    pub heading: f32,
    pub swim_speed: f32,
    pub chase_speed: f32,
    pub bite_damage: f32,
    pub bite_range_cm: f32,
    pub detect_range_cm: f32,
    /// Swimming depth below the water surface (negative z in game cm).
    pub depth_cm: f32,
    /// Phase offset for the depth bobbing animation.
    pub bob_phase: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeaMonsterState {
    Cruising,
    Hunting,
}

/// Zone-local transform (meters) for a game cm position.
fn zone_local_transform(position_cm: Vec3) -> Transform {
    Transform::from_xyz(
        (position_cm.x - ZONE_CENTER_CM) / 100.0,
        position_cm.z / 100.0,
        -(position_cm.y - ZONE_CENTER_CM) / 100.0,
    )
}

/// Random open-water position in game cm, at least `min_distance_cm` from the
/// player spawn point.
fn random_open_water_position(rng: &mut impl Rng, min_distance_cm: f32) -> Vec3 {
    for _ in 0..64 {
        let x = rng.gen_range(WATER_MIN_CM..WATER_MAX_CM);
        let y = rng.gen_range(WATER_MIN_CM..WATER_MAX_CM);
        let distance = Vec2::new(x - PLAYER_SPAWN_CM.x, y - PLAYER_SPAWN_CM.y).length();
        if distance >= min_distance_cm {
            return Vec3::new(x, y, 0.0);
        }
    }
    Vec3::new(
        (WATER_MIN_CM + WATER_MAX_CM) * 0.5,
        (WATER_MIN_CM + WATER_MAX_CM) * 0.5,
        0.0,
    )
}

/// Builds the shark mesh: an elongated tapered body with a dorsal fin and a
/// crescent tail fin, nose pointing along +X (same convention as the fish).
fn create_shark_mesh() -> Mesh {
    let vertices: Vec<[f32; 3]> = vec![
        // Nose tip.
        [1.55, 0.0, 0.0],
        // Station 0 (x = 1.0): tl, tr, cl, cr, k.
        [1.0, 0.16, -0.13],
        [1.0, 0.16, 0.13],
        [1.0, -0.05, -0.19],
        [1.0, -0.05, 0.19],
        [1.0, -0.17, 0.0],
        // Station 1 (x = 0.0): widest.
        [0.0, 0.24, -0.17],
        [0.0, 0.24, 0.17],
        [0.0, -0.08, -0.25],
        [0.0, -0.08, 0.25],
        [0.0, -0.25, 0.0],
        // Station 2 (x = -0.9).
        [-0.9, 0.17, -0.12],
        [-0.9, 0.17, 0.12],
        [-0.9, -0.06, -0.17],
        [-0.9, -0.06, 0.17],
        [-0.9, -0.16, 0.0],
        // Station 3 (x = -1.25): tail base.
        [-1.25, 0.06, -0.04],
        [-1.25, 0.06, 0.04],
        [-1.25, -0.02, -0.05],
        [-1.25, -0.02, 0.05],
        [-1.25, -0.07, 0.0],
        // Dorsal fin base front/back and apex.
        [0.25, 0.30, 0.0],
        [-0.55, 0.22, 0.0],
        [0.0, 0.62, 0.0],
        // Tail fin tips.
        [-1.8, 0.32, 0.0],
        [-1.8, -0.24, 0.0],
    ];

    let mut faces: Vec<[usize; 3]> = Vec::new();

    // Nose cap (point to station 0 ring).
    faces.extend_from_slice(&[
        [0, 1, 3],
        [0, 3, 5],
        [0, 5, 2],
        [0, 2, 4],
        [0, 4, 1],
    ]);

    // Body segments between consecutive stations (boat hull face pattern).
    for station in 0..3 {
        let current = 1 + station * 5;
        let next = current + 5;
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

    // Tail cap at station 3 (tl, tr, cl, cr, k).
    faces.extend_from_slice(&[[16, 20, 18], [16, 17, 20], [17, 19, 20]]);

    // Tail fin (double sided).
    faces.extend_from_slice(&[[20, 24, 25], [20, 25, 24]]);

    // Dorsal fin (double sided).
    faces.extend_from_slice(&[[21, 23, 22], [22, 23, 21]]);

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

/// Spawns all sharks for one zone 200 load. Entities are parented to the zone
/// entity so they despawn with it and inherit its world transform.
#[allow(clippy::too_many_arguments)]
fn spawn_sharks(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    zone_entity: Entity,
) {
    let shark_mesh = meshes.add(create_shark_mesh());

    let skins = [
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.48, 0.52),
            perceptual_roughness: 0.6,
            cull_mode: None,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.36, 0.41, 0.47),
            perceptual_roughness: 0.6,
            cull_mode: None,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.43, 0.48, 0.56),
            perceptual_roughness: 0.6,
            cull_mode: None,
            ..default()
        }),
    ];

    let mut rng = rand::thread_rng();

    for index in 0..MONSTER_COUNT {
        let position_cm = random_open_water_position(&mut rng, SPAWN_MIN_DISTANCE_CM);
        let transform = zone_local_transform(position_cm);
        let depth_cm = rng.gen_range(-120.0..-50.0);

        let monster_entity = commands
            .spawn((
                SeaMonster {
                    state: SeaMonsterState::Cruising,
                    target: None,
                    attack_cooldown: Timer::from_seconds(ATTACK_COOLDOWN_SECS, TimerMode::Once),
                    waypoint: random_open_water_position(&mut rng, WAYPOINT_MIN_DISTANCE_CM),
                    heading: rng.gen_range(0.0..std::f32::consts::TAU),
                    swim_speed: rng.gen_range(3.0..4.0),
                    chase_speed: rng.gen_range(6.0..9.0),
                    bite_damage: BITE_DAMAGE,
                    bite_range_cm: BITE_RANGE_CM,
                    detect_range_cm: DETECT_RANGE_CM,
                    depth_cm,
                    bob_phase: rng.gen_range(0.0..std::f32::consts::TAU),
                },
                Position::new(position_cm),
                ClientEntity::new(
                    ClientEntityId(MONSTER_ENTITY_ID_BASE + index),
                    ClientEntityType::Monster,
                ),
                ClientEntityName::new("Sea Shark".to_string()),
                ModelHeight::new(1.4),
                MonsterSeparation {
                    separation_radius: 2.0,
                    ..default()
                },
                transform,
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .insert((Mesh3d(shark_mesh.clone()), MeshMaterial3d(skins[index % 3].clone())))
            .id();

        commands.entity(zone_entity).add_child(monster_entity);
    }

    log::info!("[SEA MONSTER] Spawned {} sharks for zone {}", MONSTER_COUNT, OCEAN_ZONE_ID);
}

/// Spawns the sea monsters once when zone 200 loads. Falls back to spawning
/// if the ZoneEvent::Loaded event was missed but the zone entity already
/// exists, and resets the guard when the zone has been unloaded so re-entry
/// respawns them.
pub fn spawn_sea_monsters_system(
    mut commands: Commands,
    mut zone_events: MessageReader<ZoneEvent>,
    zone_query: Query<(Entity, &Zone)>,
    mut spawned: Local<bool>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut zone_loaded = false;
    for event in zone_events.read() {
        if matches!(event, ZoneEvent::Loaded(zone_id) if zone_id.get() == OCEAN_ZONE_ID) {
            zone_loaded = true;
        }
    }

    let zone_entity = zone_query
        .iter()
        .find(|(_, zone)| zone.id.get() == OCEAN_ZONE_ID)
        .map(|(entity, _)| entity);

    match (zone_loaded, *spawned, zone_entity) {
        // Zone was unloaded: reset the guard so a future load respawns.
        (false, true, None) => *spawned = false,
        // Already spawned: do nothing.
        (_, true, _) => {}
        // Spawn on the Loaded event, or on the fallback path if it was missed.
        (_, false, Some(zone_entity)) => {
            spawn_sharks(&mut commands, &mut meshes, &mut materials, zone_entity);
            *spawned = true;
        }
        (_, false, None) => {}
    }
}

/// Drives shark AI (cruise/chase/bite), sinks the player boat when the hull
/// reaches zero and triggers the standard disembark path.
#[allow(clippy::too_many_arguments)]
pub fn sea_monster_ai_system(
    time: Res<Time>,
    mut chatbox_events: MessageWriter<ChatboxEvent>,
    mut disembark_events: MessageWriter<DisembarkBoatEvent>,
    mut hull_low_warned: Local<bool>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    mut player_query: Query<
        (Entity, &mut BoatState, &Position),
        (With<PlayerCharacter>, Without<SeaMonster>),
    >,
    mut monsters: Query<
        (&mut SeaMonster, &mut Position, &mut Transform),
        (With<SeaMonster>, Without<PlayerCharacter>),
    >,
) {
    let dt = time.delta_secs();
    let delta = time.delta();

    // Snapshot the sailing player. If the hull reached zero the boat sinks:
    // reset the hull and reuse the existing disembark event so the player is
    // carried to the nearest shore.
    let mut player_boat: Option<(Entity, Vec3)> = None;
    for (entity, mut boat_state, position) in player_query.iter_mut() {
        if boat_state.active && boat_state.hull_health <= 0.0 {
            boat_state.hull_health = boat_state.hull_max_health;
            log::info!(
                "[SEA MONSTER] Player boat hull reached zero, sinking ({} HP reset)",
                boat_state.hull_max_health
            );
            chatbox_events.write(ChatboxEvent::System(
                "Your boat has been torn apart by sharks! You wash up on the nearest shore."
                    .to_string(),
            ));
            disembark_events.write(DisembarkBoatEvent { entity });
            *hull_low_warned = false;
            continue;
        }

        if boat_state.active && boat_state.rider_entity.is_some() {
            player_boat = Some((entity, position.position));
            if !*hull_low_warned && boat_state.hull_health < boat_state.hull_max_health * 0.5 {
                *hull_low_warned = true;
                chatbox_events.write(ChatboxEvent::System(
                    "Your boat hull is badly damaged by sharks!".to_string(),
                ));
            }
        } else {
            *hull_low_warned = false;
        }
    }

    let zone_data = current_zone
        .as_ref()
        .and_then(|zone| zone_loader_assets.get(&zone.handle));
    let mut rng = rand::thread_rng();

    for (mut monster, mut position, mut transform) in monsters.iter_mut() {
        monster.attack_cooldown.tick(delta);

        let horizontal = position.position.xy();
        let player_target = player_boat.map(|(_, boat_position)| boat_position);
        let player_distance = player_target.map(|target| target.xy().distance(horizontal));

        let mut speed_mps = monster.swim_speed;
        let mut steer_dir = Vec2::ZERO;
        let mut moving = false;

        match monster.state {
            SeaMonsterState::Cruising => {
                let can_hunt = player_distance
                    .map(|distance| distance < monster.detect_range_cm)
                    .unwrap_or(false);
                if can_hunt {
                    monster.state = SeaMonsterState::Hunting;
                    monster.target = player_boat.map(|(entity, _)| entity);
                } else {
                    let to_waypoint = monster.waypoint.xy() - horizontal;
                    if to_waypoint.length() < WAYPOINT_REACH_CM {
                        monster.waypoint =
                            random_open_water_position(&mut rng, WAYPOINT_MIN_DISTANCE_CM);
                    }
                    let to_waypoint = monster.waypoint.xy() - horizontal;
                    if to_waypoint.length() > 1.0 {
                        steer_dir = to_waypoint.normalize();
                        moving = true;
                    }
                }
            }
            SeaMonsterState::Hunting => {
                let target_valid = player_boat
                    .as_ref()
                    .map(|(entity, _)| Some(*entity) == monster.target)
                    .unwrap_or(false);
                let lost_target = player_distance
                    .map(|distance| distance > monster.detect_range_cm * LOSE_RANGE_MULTIPLIER)
                    .unwrap_or(true);
                if !target_valid || lost_target {
                    monster.state = SeaMonsterState::Cruising;
                    monster.target = None;
                    monster.waypoint =
                        random_open_water_position(&mut rng, WAYPOINT_MIN_DISTANCE_CM);
                } else if let Some(target) = player_target {
                    let to_player = target.xy() - horizontal;
                    let distance = to_player.length();
                    if distance < monster.bite_range_cm && monster.attack_cooldown.is_finished() {
                        // Bite the hull.
                        if let Some((player_entity, _)) = player_boat {
                            if let Ok((_, mut boat_state, _)) =
                                player_query.get_mut(player_entity)
                            {
                                boat_state.hull_health =
                                    (boat_state.hull_health - monster.bite_damage).max(0.0);
                                monster.attack_cooldown.reset();
                                log::info!(
                                    "[SEA MONSTER] Shark bites boat hull: {:.0} / {:.0}",
                                    boat_state.hull_health,
                                    boat_state.hull_max_health
                                );
                            }
                        }
                    } else if distance > monster.bite_range_cm {
                        steer_dir = to_player / distance.max(0.001);
                        moving = true;
                        speed_mps = monster.chase_speed;
                    }
                }
            }
        }

        if moving {
            let target_yaw = (-steer_dir.y).atan2(steer_dir.x);
            let yaw_delta = shortest_angle_delta(monster.heading, target_yaw)
                .clamp(-TURN_RATE_RAD_PER_SEC * dt, TURN_RATE_RAD_PER_SEC * dt);
            monster.heading = normalize_angle(monster.heading + yaw_delta);
            let forward = Vec2::new(monster.heading.sin(), monster.heading.cos());
            position.position.x += forward.x * speed_mps * 100.0 * dt;
            position.position.y += forward.y * speed_mps * 100.0 * dt;
        }

        // Gentle depth bobbing just below the surface.
        monster.bob_phase += dt * 1.4;
        position.position.z = monster.depth_cm + monster.bob_phase.sin() * 12.0;

        // Keep the shark inside the open-water rectangle.
        position.position.x = position.position.x.clamp(WATER_MIN_CM, WATER_MAX_CM);
        position.position.y = position.position.y.clamp(WATER_MIN_CM, WATER_MAX_CM);

        // Stay clear of islands: if the terrain pokes up to our depth, steer
        // back toward open water and pick a fresh waypoint.
        if let Some(zone_data) = zone_data {
            if zone_data.get_terrain_height(position.position.x, position.position.y)
                > GROUND_CLEARANCE_CM
            {
                let away =
                    (position.position.xy() - PLAYER_SPAWN_CM.xy()).normalize_or_zero();
                position.position.x += away.x * 2000.0;
                position.position.y += away.y * 2000.0;
                monster.waypoint =
                    random_open_water_position(&mut rng, WAYPOINT_MIN_DISTANCE_CM);
            }
        }

        // Sync the zone-local visual transform from the authoritative position.
        let mut new_transform = zone_local_transform(position.position);
        new_transform.rotation = Quat::from_rotation_y(monster.heading);
        *transform = new_transform;
    }
}
