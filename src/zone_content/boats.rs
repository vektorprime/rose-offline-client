//! Decorative NPC boats for zone 200 (ocean).
//!
//! Spawns a fleet of civilian sailboats that patrol hardcoded open-water
//! routes between the island clusters, with extra boats cruising around the
//! island docks and marinas so the "towns" feel alive. Each boat reuses
//! `spawn_boat_visual` from the player sailing system, so the hull/mast/sails,
//! buoyancy bobbing, wave roll/pitch and sail animation all come for free from
//! the existing systems. Boats are pure decoration: no combat, no interaction,
//! no server involvement. Entities are parented to the zone entity so they
//! despawn automatically when the zone unloads.
//!
//! The movement system keeps boats out of land, dock footprints and other
//! boats: terrain is sampled ahead of every step via the zone heightmap, dock
//! footprints are checked in each dock's local space, and nearby boats (NPC or
//! player) apply a mild repulsion to the steering target.

use bevy::prelude::*;
use rand::Rng;

use crate::{
    components::{BoatState, PlayerCharacter, Position, Zone},
    events::ZoneEvent,
    graphics::GraphicsSettings,
    render::underwater_effect::UnderwaterVolumes,
    resources::CurrentZone,
    sailing::{normalize_angle, shortest_angle_delta},
    systems::spawn_boat_visual,
    zone_content::docks::Dock,
    zone_loader::ZoneLoaderAsset,
};

/// Mirrors `systems::boat_spawn_system::OCEAN_ZONE_ID`. That const is
/// pub(crate) inside a private module and is not re-exported, so it is not
/// reachable from here; the value is confirmed at boat_spawn_system.rs:19.
const OCEAN_ZONE_ID: u16 = 200;

/// Map center in game cm. The zone entity sits at world (5200, 0, -5200), so
/// zone-local meters for a game cm position (x, y, z) are
/// ((x - CENTER)/100, z/100, -(y - CENTER)/100). See zone_content/npcs.rs:29.
const ZONE_CENTER_CM: f32 = 520000.0;

/// Number of decorative boats spawned per zone load.
const NPC_BOAT_COUNT: usize = 18;

/// A waypoint counts as reached within this radius (cm).
const WAYPOINT_REACH_CM: f32 = 300.0;

/// Water surface in zone 200 is at 0 cm (see zone_content/docks.rs:35).
const WATER_HEIGHT_CM: f32 = 0.0;

/// A step is blocked when the sampled terrain is this far above the water
/// surface (cm). Absorbs the procedural terrain noise (± a few cm) while
/// keeping hulls off the beaches.
const LAND_MARGIN_CM: f32 = 20.0;

/// Dock deck dimensions in meters, mirrored from the (private) constants in
/// zone_content/docks.rs:51-52: 8 planks of 6 m laid end to end (48 m) across
/// 8 rows of 1.1 m (8.8 m). The dock's local +X points from the shore out
/// into the water, so the deck spans local x in [0, DOCK_LENGTH_M].
const DOCK_LENGTH_M: f32 = 48.0;
const DOCK_WIDTH_M: f32 = 8.8;

/// Extra clearance around a dock footprint (m) that boats must not enter,
/// covering the mooring posts, rail and hull beam.
const DOCK_MARGIN_M: f32 = 2.0;

/// Horizontal distance (cm) within which boats repel each other.
const BOAT_SEPARATION_RADIUS_CM: f32 = 350.0;

/// Weight of the summed repulsion vector when blended into the steering
/// target (1.0 = repulsion fully dominates at contact range).
const BOAT_SEPARATION_BLEND: f32 = 1.6;

/// Speed reduction at full overlap: speed factor = 1 - weight * this.
const BOAT_SEPARATION_SLOWDOWN: f32 = 0.5;

/// Distance (m) ahead of the boat used to probe the port/starboard sides when
/// the way ahead is blocked.
const BLOCKED_PROBE_M: f32 = 30.0;

/// Consecutive frames with no movement before the boat skips to the next
/// waypoint, so a route that crosses newly generated land cannot trap a boat
/// forever.
const MAX_BLOCKED_FRAMES: u32 = 8;

/// Open-water patrol routes in game-space centimeters (z filled at spawn).
/// Cleared of the current map's land: main island (520000, 520000) r300 m,
/// east island (~560000, 559000) r150 m, west island (~465000, 544000)
/// r130 m, the spawn point (520000, 520000), and the ring islands from
/// tools/generate_ocean_islands.py. All waypoints are >= 300 m from the
/// player spawn. Newer ring islands may still cut across a route; the
/// movement system steers around any terrain that pokes above the water.
const BOAT_ROUTES: &[&[Vec3]] = &[
    // North strait loop: between the main island and the northern map edge.
    &[
        Vec3::new(500000.0, 585000.0, 0.0),
        Vec3::new(535000.0, 595000.0, 0.0),
        Vec3::new(575000.0, 585000.0, 0.0),
        Vec3::new(585000.0, 535000.0, 0.0),
        Vec3::new(450000.0, 570000.0, 0.0),
    ],
    // South strait loop: between the main island and the southern map edge.
    &[
        Vec3::new(490000.0, 465000.0, 0.0),
        Vec3::new(540000.0, 460000.0, 0.0),
        Vec3::new(585000.0, 475000.0, 0.0),
        Vec3::new(455000.0, 490000.0, 0.0),
        Vec3::new(445000.0, 535000.0, 0.0),
    ],
    // West transit loop: west of the main island, between the west island and
    // the map edge.
    &[
        Vec3::new(483000.0, 558000.0, 0.0),
        Vec3::new(480000.0, 486000.0, 0.0),
        Vec3::new(470000.0, 465000.0, 0.0),
    ],
    // Marina bay loop: cruising just north of the main island's dock/marina
    // (ferryman at ~(520000, 527000)), a short sail from the town.
    &[
        Vec3::new(505000.0, 560000.0, 0.0),
        Vec3::new(535000.0, 562000.0, 0.0),
        Vec3::new(540000.0, 552000.0, 0.0),
        Vec3::new(510000.0, 554000.0, 0.0),
    ],
    // Marina approach: smaller loop just off the north shore, where the docks
    // are visible from the island.
    &[
        Vec3::new(508000.0, 556000.0, 0.0),
        Vec3::new(532000.0, 560000.0, 0.0),
        Vec3::new(538000.0, 552000.0, 0.0),
        Vec3::new(511000.0, 552000.0, 0.0),
    ],
    // East island docks: loops around the east island cluster.
    &[
        Vec3::new(545000.0, 578000.0, 0.0),
        Vec3::new(570000.0, 582000.0, 0.0),
        Vec3::new(580000.0, 572000.0, 0.0),
        Vec3::new(548000.0, 572000.0, 0.0),
    ],
    // West island docks: loops around the west island cluster.
    &[
        Vec3::new(448000.0, 562000.0, 0.0),
        Vec3::new(478000.0, 560000.0, 0.0),
        Vec3::new(482000.0, 550000.0, 0.0),
        Vec3::new(448000.0, 550000.0, 0.0),
    ],
];

/// Marker and movement state for a decorative NPC boat.
#[derive(Component)]
pub struct NpcBoat {
    /// Patrol route waypoints in game-space centimeters.
    route: Vec<Vec3>,
    /// Index into `route` the boat is currently sailing toward.
    next_waypoint: usize,
    /// Constant cruise speed in meters/second.
    speed: f32,
    /// Maximum heading turn rate in radians/second.
    turn_rate: f32,
    /// Consecutive frames the boat could not move (land/dock/boat blocking).
    blocked_frames: u32,
}

/// Nearest water surface height in game cm. Mirrors
/// `systems::boat_spawn_system::nearest_water_surface_height_cm`
/// (private module, not re-exported, so not reachable from here).
fn nearest_water_surface_height_cm(
    position_cm: Vec3,
    underwater_volumes: &UnderwaterVolumes,
) -> Option<f32> {
    underwater_volumes
        .volumes
        .iter()
        .map(|volume| {
            let world_x = position_cm.x / 100.0;
            let world_z = -position_cm.y / 100.0;
            let dx = ((world_x - volume.center.x).abs() - volume.half_extents.x).max(0.0);
            let dz = ((world_z - volume.center.z).abs() - volume.half_extents.y).max(0.0);
            let distance_m = (dx * dx + dz * dz).sqrt();
            (distance_m, volume.surface_y * 100.0)
        })
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, surface_cm)| surface_cm)
}

/// Converts a game-cm position to the zone-local x/z meters that the dock
/// parent transforms use (same conversion as the boat render transform
/// below). Docks and boats share the zone entity as parent, so their
/// transforms can be compared directly in this space.
fn boat_zone_local_xy(position_cm: Vec3) -> Vec2 {
    Vec2::new(
        (position_cm.x - ZONE_CENTER_CM) / 100.0,
        -(position_cm.y - ZONE_CENTER_CM) / 100.0,
    )
}

/// True when `position_cm` is not inside any dock footprint. Each dock's
/// parent transform is yaw-rotated so its local +X points from the shore into
/// the water; the deck occupies local x in [0, 48] m and |local z| <= 4.4 m
/// (zone_content/docks.rs:51-52, 240-248).
fn is_clear_of_docks(position_cm: Vec3, docks: &[(Vec2, Quat)]) -> bool {
    let local = boat_zone_local_xy(position_cm);
    for (dock_xy, dock_rotation) in docks {
        let dock_local = dock_rotation.inverse() * Vec3::new(local.x, 0.0, local.y);
        if dock_local.x >= -DOCK_MARGIN_M
            && dock_local.x <= DOCK_LENGTH_M + DOCK_MARGIN_M
            && dock_local.z.abs() <= DOCK_WIDTH_M / 2.0 + DOCK_MARGIN_M
        {
            return false;
        }
    }
    true
}

/// True when `position_cm` is safe to sail through: terrain below the water
/// surface plus margin, and outside every dock footprint. When the zone data
/// is not loaded yet only the dock check applies (movement keeps working).
fn is_position_navigable(zone_data: Option<&ZoneLoaderAsset>, position_cm: Vec3, docks: &[(Vec2, Quat)]) -> bool {
    let over_water = match zone_data {
        Some(zone_data) => {
            zone_data.get_terrain_height(position_cm.x, position_cm.y)
                <= WATER_HEIGHT_CM + LAND_MARGIN_CM
        }
        None => true,
    };
    over_water && is_clear_of_docks(position_cm, docks)
}

/// Accumulates one boat's repulsion contribution into `repel`. Boats closer
/// than `BOAT_SEPARATION_RADIUS_CM` push apart with a linear weight that peaks
/// at 1.0 at contact.
fn accumulate_separation(repel: &mut Vec2, separation_weight: &mut f32, from_xy: Vec2, my_xy: Vec2) {
    let delta = my_xy - from_xy;
    let dist = delta.length();
    if dist > 0.001 && dist < BOAT_SEPARATION_RADIUS_CM {
        let weight = 1.0 - dist / BOAT_SEPARATION_RADIUS_CM;
        *repel += delta / dist * weight;
        *separation_weight = separation_weight.max(weight);
    }
}

/// Spawns one decorative boat with its visual, parented to the zone entity.
fn spawn_npc_boat(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    graphics_settings: &GraphicsSettings,
    underwater_volumes: &UnderwaterVolumes,
    zone_entity: Entity,
    route: &[Vec3],
    rng: &mut impl Rng,
) {
    let speed = rng.gen_range(4.0..6.0);
    let turn_rate = rng.gen_range(0.8..1.2);

    // Spawn just off the first waypoint so boats sharing a route diverge.
    let mut position = route[0];
    position.x += rng.gen_range(-2500.0..2500.0);
    position.y += rng.gen_range(-2500.0..2500.0);
    position.z = nearest_water_surface_height_cm(position, underwater_volumes).unwrap_or(0.0);
    let water_height_cm = position.z;

    // Heading toward the next waypoint: forward = (sin h, cos h) in Position
    // space, so h = atan2(dx, dy) (see remote_boat_system.rs:72).
    let dir = Vec2::new(route[1].x - position.x, route[1].y - position.y);
    let heading = dir.x.atan2(dir.y);

    let model_root = spawn_boat_visual(
        commands,
        meshes,
        materials,
        &Position::new(position),
        graphics_settings.sailing.sail_deformation_quality,
    );

    // Zone-local transform: game cm -> meters relative to the zone entity.
    let transform = Transform::from_xyz(
        (position.x - ZONE_CENTER_CM) / 100.0,
        position.z / 100.0,
        -(position.y - ZONE_CENTER_CM) / 100.0,
    );

    let boat_entity = commands
        .spawn((
            NpcBoat {
                route: route.to_vec(),
                next_waypoint: 1,
                speed,
                turn_rate,
                blocked_frames: 0,
            },
            BoatState {
                active: true,
                rider_entity: None,
                heading,
                speed,
                max_speed: 10.0,
                sail_trim: std::f32::consts::FRAC_PI_4,
                rudder: 0.0,
                hull_health: 100.0,
                hull_max_health: 100.0,
                model_root_entity: Some(model_root),
                water_height_cm,
                wave_roll: 0.0,
                wave_pitch: 0.0,
            },
            Position::new(position),
            transform,
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            Name::new("Npc Boat"),
        ))
        .id();

    commands.entity(boat_entity).add_child(model_root);
    commands.entity(zone_entity).add_child(boat_entity);
}

/// Spawns the decorative boats once when zone 200 loads. Falls back to
/// spawning if the ZoneEvent::Loaded event was missed but the zone entity
/// already exists, and resets the guard when the zone has been unloaded so
/// re-entry respawns them. Same pattern as spawn_dock_npcs_system in
/// src/zone_content/npcs.rs.
pub fn spawn_random_boats_system(
    mut commands: Commands,
    mut zone_events: MessageReader<ZoneEvent>,
    zone_query: Query<(Entity, &Zone)>,
    mut spawned: Local<bool>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    graphics_settings: Res<GraphicsSettings>,
    underwater_volumes: Res<UnderwaterVolumes>,
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
        // Spawn on the Loaded event, or on the fallback path if the event was missed.
        (_, false, Some(zone_entity)) => {
            let mut rng = rand::thread_rng();
            for _ in 0..NPC_BOAT_COUNT {
                let route = BOAT_ROUTES[rng.gen_range(0..BOAT_ROUTES.len())];
                spawn_npc_boat(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &graphics_settings,
                    &underwater_volumes,
                    zone_entity,
                    route,
                    &mut rng,
                );
            }
            *spawned = true;
        }
        (_, false, None) => {}
    }
}

/// Moves every NPC boat along its route at constant speed, turning toward the
/// current waypoint at a limited rate, and syncs the render transform. The
/// heading/speed are written to BoatState so the shared wake and sail systems
/// keep working; boat_buoyancy_system owns the Y position and hull rotation.
///
/// Collision behavior:
/// - Land: the terrain height is sampled at the would-be new position; a step
///   into terrain above the water surface is refused, the boat slides along
///   the axis that stays in water, and the heading turns toward whichever
///   side (90 deg port or starboard, probed 30 m ahead) is clear.
/// - Docks: positions inside any dock footprint (deck + 2 m margin) count as
///   blocked and get the same slide/turn treatment.
/// - Other boats: NPC boats and the player's boat within 350 cm push the
///   steering target away and scale the speed down.
/// - Stuck boats: after `MAX_BLOCKED_FRAMES` frames without any movement the
///   boat skips to the next route waypoint so it can never freeze forever.
pub fn npc_boat_movement_system(
    time: Res<Time>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    mut query: Query<
        (Entity, &mut BoatState, &mut Position, &mut Transform, &mut NpcBoat),
        Without<Dock>,
    >,
    player_boat: Query<(&Position, &BoatState), (With<PlayerCharacter>, Without<NpcBoat>)>,
    docks: Query<&Transform, With<Dock>>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    let zone_data = current_zone
        .as_ref()
        .and_then(|zone| zone_loader_assets.get(&zone.handle));

    // Dock parent transforms are static, so capture them once for the frame.
    let dock_spaces: Vec<(Vec2, Quat)> = docks
        .iter()
        .map(|transform| (transform.translation.xz(), transform.rotation))
        .collect();

    // Snapshot every boat's position for pairwise separation; self is
    // excluded by entity id during the loop.
    let boat_positions: Vec<(Entity, Vec2)> = query
        .iter()
        .map(|(entity, _, position, _, _)| (entity, position.position.xy()))
        .collect();

    for (entity, mut boat, mut position, mut transform, mut npc) in query.iter_mut() {
        if !boat.active || npc.route.is_empty() {
            continue;
        }

        let waypoint = npc.route[npc.next_waypoint % npc.route.len()];
        let mut dir = Vec2::new(
            waypoint.x - position.position.x,
            waypoint.y - position.position.y,
        );

        if dir.length_squared() < WAYPOINT_REACH_CM * WAYPOINT_REACH_CM {
            npc.next_waypoint = (npc.next_waypoint + 1) % npc.route.len();
            let waypoint = npc.route[npc.next_waypoint];
            dir = Vec2::new(
                waypoint.x - position.position.x,
                waypoint.y - position.position.y,
            );
            if dir.length_squared() < f32::EPSILON {
                continue;
            }
        }

        // Separation: sum repulsion from nearby NPC boats and the player's
        // boat, then blend it into the waypoint steering target.
        let my_xy = position.position.xy();
        let mut repel = Vec2::ZERO;
        let mut separation_weight = 0.0;
        for (other_entity, other_xy) in &boat_positions {
            if *other_entity != entity {
                accumulate_separation(&mut repel, &mut separation_weight, *other_xy, my_xy);
            }
        }
        for (player_position, player_state) in player_boat.iter() {
            if player_state.active {
                accumulate_separation(
                    &mut repel,
                    &mut separation_weight,
                    player_position.position.xy(),
                    my_xy,
                );
            }
        }

        // Turn toward the blended target, limited by the per-boat turn rate.
        let steer = dir.normalize_or_zero() + repel * BOAT_SEPARATION_BLEND;
        let target_heading = steer.x.atan2(steer.y);
        let speed_factor = 1.0 - separation_weight * BOAT_SEPARATION_SLOWDOWN;
        let turn = shortest_angle_delta(boat.heading, target_heading)
            .clamp(-npc.turn_rate * dt, npc.turn_rate * dt);
        boat.heading = normalize_angle(boat.heading + turn);

        // Sail forward at constant speed; forward = (sin h, cos h) in cm space.
        let forward = Vec2::new(boat.heading.sin(), boat.heading.cos());
        let step_cm = forward * (npc.speed * speed_factor * dt * 100.0);
        let candidate = Vec2::new(
            position.position.x + step_cm.x,
            position.position.y + step_cm.y,
        );

        if is_position_navigable(zone_data, Vec3::new(candidate.x, candidate.y, 0.0), &dock_spaces)
        {
            position.position.x = candidate.x;
            position.position.y = candidate.y;
            npc.blocked_frames = 0;
        } else {
            // Blocked by land or a dock. Try sliding along each axis; a slide
            // counts as progress, a fully stuck frame increments the counter.
            let slide_x = Vec2::new(candidate.x, position.position.y);
            let slide_y = Vec2::new(position.position.x, candidate.y);
            let mut moved = false;
            if is_position_navigable(zone_data, Vec3::new(slide_x.x, slide_x.y, 0.0), &dock_spaces)
            {
                position.position.x = slide_x.x;
                moved = true;
            } else if is_position_navigable(
                zone_data,
                Vec3::new(slide_y.x, slide_y.y, 0.0),
                &dock_spaces,
            ) {
                position.position.y = slide_y.y;
                moved = true;
            }

            if moved {
                npc.blocked_frames = 0;
            } else {
                npc.blocked_frames += 1;
            }

            // Probe 30 m ahead at heading +/- 90 deg and turn toward the side
            // that stays in open water (preferring the side closer to the
            // waypoint so the detour makes progress). Both sides blocked:
            // hold course; the blocked-frames skip below gets us out.
            let side_a_heading = normalize_angle(boat.heading + std::f32::consts::FRAC_PI_2);
            let side_b_heading = normalize_angle(boat.heading - std::f32::consts::FRAC_PI_2);
            let probe_cm = BLOCKED_PROBE_M * 100.0;
            let a_point = Vec3::new(
                position.position.x + side_a_heading.sin() * probe_cm,
                position.position.y + side_a_heading.cos() * probe_cm,
                0.0,
            );
            let b_point = Vec3::new(
                position.position.x + side_b_heading.sin() * probe_cm,
                position.position.y + side_b_heading.cos() * probe_cm,
                0.0,
            );
            let a_clear = is_position_navigable(zone_data, a_point, &dock_spaces);
            let b_clear = is_position_navigable(zone_data, b_point, &dock_spaces);
            let escape_heading = match (a_clear, b_clear) {
                (true, false) => Some(side_a_heading),
                (false, true) => Some(side_b_heading),
                (true, true) => {
                    let a_dist = a_point.xy().distance_squared(waypoint.xy());
                    let b_dist = b_point.xy().distance_squared(waypoint.xy());
                    if a_dist <= b_dist {
                        Some(side_a_heading)
                    } else {
                        Some(side_b_heading)
                    }
                }
                (false, false) => None,
            };
            if let Some(escape_heading) = escape_heading {
                let escape_turn = shortest_angle_delta(boat.heading, escape_heading)
                    .clamp(-npc.turn_rate * dt, npc.turn_rate * dt);
                boat.heading = normalize_angle(boat.heading + escape_turn);
            }

            // A route now crossing generated land must not trap a boat
            // forever: skip to the next waypoint after too many stuck frames.
            if npc.blocked_frames > MAX_BLOCKED_FRAMES {
                npc.next_waypoint = (npc.next_waypoint + 1) % npc.route.len();
                npc.blocked_frames = 0;
            }
        }

        position.position.z = boat.water_height_cm;

        // Sync the render transform (zone-local). Y and rotation are owned by
        // boat_buoyancy_system, which must run after this system.
        transform.translation.x = (position.position.x - ZONE_CENTER_CM) / 100.0;
        transform.translation.z = -(position.position.y - ZONE_CENTER_CM) / 100.0;

        boat.speed = npc.speed * speed_factor;
    }
}
