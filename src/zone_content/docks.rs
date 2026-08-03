//! Procedural harbor-style wooden docks for zone 200 (ocean).
//!
//! Zone 200's IFO contains no dock structures, so the docks are spawned
//! client-side: a wide multi-row plank deck (48 m long x 8.8 m wide) on
//! crosswise support beams and sturdy posts extending from each island's
//! shore into the water, with mooring posts, a small rail, a shore ramp and
//! a few crates/barrels at the landward end. Purely visual: no collision,
//! no interaction, no server involvement. Entities are parented to the zone
//! entity so they despawn automatically when the zone unloads, and their
//! transforms are zone-local (the zone entity is positioned at world
//! (5200, 0, -5200), which is the origin of the map's signed coordinate
//! space).

use bevy::prelude::*;

use crate::{
    components::Zone,
    events::ZoneEvent,
    resources::CurrentZone,
    systems::find_nearest_shore_position,
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

/// Water surface in zone 200 is at 0 cm (see boats.rs / SailingZoneConfig).
const WATER_HEIGHT_CM: f32 = 0.0;

/// Deck planking dimensions (meters). Planks are long along the dock's local
/// +X axis (the parent is yaw-rotated so +X points from shore into the water).
const PLANK_LENGTH_M: f32 = 6.0;
const PLANK_THICKNESS_M: f32 = 0.15;
const PLANK_WIDTH_M: f32 = 1.1;
/// Number of plank rows side by side across the deck (deck width).
const PLANK_ROWS: usize = 8;
/// Number of planks laid end to end per row (deck length).
const PLANKS_PER_ROW: usize = 8;
/// Half-plank stagger for alternating rows (brick pattern); staggered rows
/// get a short cap plank at the outer end to square off the deck.
const PLANK_STAGGER_M: f32 = PLANK_LENGTH_M / 2.0;
const CAP_LENGTH_M: f32 = PLANK_STAGGER_M;

const DECK_LENGTH_M: f32 = PLANKS_PER_ROW as f32 * PLANK_LENGTH_M; // 48 m
const DECK_WIDTH_M: f32 = PLANK_ROWS as f32 * PLANK_WIDTH_M; // 8.8 m

/// Crosswise support beams under the plank joints, spanning the full width.
const BEAM_LENGTH_M: f32 = DECK_WIDTH_M + 0.2;
const BEAM_CROSS_M: f32 = 0.16;

/// Support posts under the deck, sunk well below the water surface.
const POST_HEIGHT_M: f32 = 3.2;
const POST_CROSS_M: f32 = 0.3;
/// Spacing between the three post columns across the deck width.
const POST_Z_SPACING_M: f32 = 3.3;

/// Mooring posts standing on the deck at the outer end.
const MOORING_HEIGHT_M: f32 = 1.2;
const MOORING_CROSS_M: f32 = 0.3;
const MOORING_Z_OFFSET_M: f32 = DECK_WIDTH_M / 2.0 - 0.8;

/// Rail around the outer end: posts and horizontal rails.
const RAIL_HEIGHT_M: f32 = 1.0;
const RAIL_CROSS_M: f32 = 0.12;
/// Rail height above the deck surface.
const RAIL_TOP_M: f32 = 0.9;
/// Distance of the inner rail posts from the outer deck end.
const RAIL_INNER_BACK_M: f32 = 5.4;
/// Distance of the outer rail posts from the outer deck end.
const RAIL_OUTER_BACK_M: f32 = 0.1;
const RAIL_SIDE_LENGTH_M: f32 = RAIL_INNER_BACK_M - RAIL_OUTER_BACK_M;
const RAIL_END_LENGTH_M: f32 = DECK_WIDTH_M - 2.0 * RAIL_CROSS_M;

/// Sloped ramp at the shore end, pitched down toward the beach.
const RAMP_LENGTH_M: f32 = 3.0;
const RAMP_THICKNESS_M: f32 = 0.12;
const RAMP_WIDTH_M: f32 = 5.0;
const RAMP_PITCH_RAD: f32 = 15.0_f32.to_radians();

/// Cargo props near the shore end.
const CRATE_SIZE_M: f32 = 0.8;
const BARREL_RADIUS_M: f32 = 0.28;
const BARREL_HEIGHT_M: f32 = 0.75;

/// Deck height above the water level (meters) when the shore is low.
const DECK_MIN_HEIGHT_M: f32 = 0.35;
/// Extra deck clearance above the shore terrain at the landward end.
const DECK_SHORE_CLEARANCE_M: f32 = 0.10;

/// Marker component identifying dock entities spawned by this system.
#[derive(Component)]
pub struct Dock;

/// Shared meshes and materials for every dock part, created once per zone
/// load and reused by all docks.
struct DockAssets {
    plank: Handle<Mesh>,
    cap: Handle<Mesh>,
    beam: Handle<Mesh>,
    post: Handle<Mesh>,
    mooring: Handle<Mesh>,
    rail_post: Handle<Mesh>,
    side_rail: Handle<Mesh>,
    end_rail: Handle<Mesh>,
    ramp: Handle<Mesh>,
    crate_box: Handle<Mesh>,
    barrel: Handle<Mesh>,
    deck_mat: Handle<StandardMaterial>,
    post_mat: Handle<StandardMaterial>,
}

/// One dock definition: the island it belongs to (for the outward direction)
/// and probe positions in game-space centimeters (x, y plane).
///
/// The probes must lie within ~20 m of terrain higher than the water level
/// (+50 cm), because `find_nearest_shore_position` only searches 20 m. The
/// probe positions below were verified against the actual HIM height data:
/// each probe is in open water with the shore contour within 2-20 m.
struct DockSpawn {
    name: &'static str,
    island_center_cm: Vec2,
    probes: &'static [Vec2],
}

const DOCKS: &[DockSpawn] = &[
    // Main island (map origin, spawn point), three docks on different shores.
    DockSpawn {
        name: "Main North Dock",
        island_center_cm: Vec2::new(520000.0, 520000.0),
        probes: &[Vec2::new(516000.0, 539800.0)],
    },
    DockSpawn {
        name: "Main West Dock",
        island_center_cm: Vec2::new(520000.0, 520000.0),
        probes: &[Vec2::new(494600.0, 525000.0)],
    },
    DockSpawn {
        name: "Main East Dock",
        island_center_cm: Vec2::new(520000.0, 520000.0),
        probes: &[Vec2::new(522400.0, 524000.0)],
    },
    // East island, one dock on each of the north and south shores. The island
    // center (563000, 484700) was measured from the HIM terrain; the older
    // estimate (560000, 559000) from npcs.rs is not on land.
    DockSpawn {
        name: "East Island North Dock",
        island_center_cm: Vec2::new(563000.0, 484700.0),
        probes: &[Vec2::new(563000.0, 497000.0)],
    },
    DockSpawn {
        name: "East Island South Dock",
        island_center_cm: Vec2::new(563000.0, 484700.0),
        probes: &[Vec2::new(563000.0, 471800.0)],
    },
    // West island, one dock on its east shore.
    DockSpawn {
        name: "West Island Dock",
        island_center_cm: Vec2::new(463300.0, 491400.0),
        probes: &[Vec2::new(473600.0, 491400.0)],
    },
    // Ring islands from tools/generate_ocean_islands.py. As of writing the
    // generator has not been run, so no land exists there and these probes
    // resolve to None (no dock spawned); if the islands are generated later
    // the shore search succeeds and the docks appear automatically.
    DockSpawn {
        name: "Northeast Ring Dock",
        island_center_cm: Vec2::new(640000.0, 640000.0),
        probes: &[Vec2::new(627837.0, 627837.0)],
    },
    DockSpawn {
        name: "Southeast Ring Dock",
        island_center_cm: Vec2::new(640000.0, 400000.0),
        probes: &[Vec2::new(629535.0, 410465.0)],
    },
];

/// Spawns a single mesh part as a child-style entity (same component set as
/// the boat visual parts, see systems/boat_spawn_system.rs:534).
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

/// Spawns one mesh part and parents it to the dock entity.
fn spawn_dock_child(
    commands: &mut Commands,
    dock: Entity,
    mesh: &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
    transform: Transform,
) {
    let part = spawn_visual_part(commands, mesh.clone(), material.clone(), transform);
    commands.entity(dock).add_child(part);
}

/// Builds one dock at the given shore position (game cm) and parents it to
/// the zone entity. The deck runs from the shore outward, away from the
/// island center: 8 rows of staggered planks, crosswise support beams under
/// the joints, posts in three columns under the beams, mooring posts and a
/// rail at the outer end, plus a ramp and cargo props near the shore. The
/// deck height follows the shore terrain so the landward end does not sink
/// into the beach.
fn spawn_dock_at(
    commands: &mut Commands,
    zone_entity: Entity,
    spawn: &DockSpawn,
    shore_cm: Vec3,
    assets: &DockAssets,
) {
    // Deck level: just above the shore terrain at the landward end, never
    // below the minimum clearance over the water.
    let deck_y = (shore_cm.z / 100.0 + DECK_SHORE_CLEARANCE_M).max(DECK_MIN_HEIGHT_M);

    // Direction from the island center through the shore point (outward).
    // Yaw about Y maps the dock's local +X (plank long axis) onto it:
    // from_rotation_y(yaw) * +X = (cos yaw, 0, -sin yaw) in zone-local space,
    // and a game cm direction (dx, dy) maps to zone-local (dx, -dy), so
    // yaw = atan2(dy, dx) (see systems/boat_buoyancy_system.rs:25-32).
    let dir = (shore_cm.truncate() - spawn.island_center_cm).normalize_or_zero();
    let yaw = dir.y.atan2(dir.x);

    let transform = Transform::from_xyz(
        (shore_cm.x - ZONE_CENTER_CM) / 100.0,
        0.0,
        -(shore_cm.y - ZONE_CENTER_CM) / 100.0,
    )
    .with_rotation(Quat::from_rotation_y(yaw));

    let dock_entity = commands
        .spawn((
            Dock,
            Name::new(spawn.name),
            transform,
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Deck planks: PLANK_ROWS rows across the width, PLANKS_PER_ROW per row
    // along local +X starting at the shore. Alternate rows are staggered by
    // half a plank so joints do not line up; each staggered row gets a short
    // cap plank at the outer end so the deck edge stays square.
    for row in 0..PLANK_ROWS {
        let z = (row as f32 + 0.5) * PLANK_WIDTH_M - DECK_WIDTH_M / 2.0;
        let stagger = if row % 2 == 0 { 0.0 } else { PLANK_STAGGER_M };
        for p in 0..PLANKS_PER_ROW {
            let x = PLANK_LENGTH_M / 2.0 + p as f32 * PLANK_LENGTH_M + stagger;
            spawn_dock_child(
                commands,
                dock_entity,
                &assets.plank,
                &assets.deck_mat,
                Transform::from_xyz(x, deck_y, z),
            );
        }
        if row % 2 == 1 {
            spawn_dock_child(
                commands,
                dock_entity,
                &assets.cap,
                &assets.deck_mat,
                Transform::from_xyz(DECK_LENGTH_M - CAP_LENGTH_M / 2.0, deck_y, z),
            );
        }
    }

    // Crosswise support beams under every plank joint.
    let beam_y = deck_y - PLANK_THICKNESS_M / 2.0 - BEAM_CROSS_M / 2.0;
    for i in 0..=PLANKS_PER_ROW {
        spawn_dock_child(
            commands,
            dock_entity,
            &assets.beam,
            &assets.post_mat,
            Transform::from_xyz(i as f32 * PLANK_LENGTH_M, beam_y, 0.0),
        );
    }

    // Support posts in three columns under the beams, every other joint,
    // sunk so their bottoms sit ~3 m below the water surface.
    let post_y = deck_y - PLANK_THICKNESS_M / 2.0 - POST_HEIGHT_M / 2.0;
    for i in 0..=PLANKS_PER_ROW / 2 {
        let x = i as f32 * PLANK_LENGTH_M * 2.0;
        for z in [-POST_Z_SPACING_M, 0.0, POST_Z_SPACING_M] {
            spawn_dock_child(
                commands,
                dock_entity,
                &assets.post,
                &assets.post_mat,
                Transform::from_xyz(x, post_y, z),
            );
        }
    }

    // Mooring posts at the outer end of the deck.
    let outer_x = DECK_LENGTH_M + MOORING_CROSS_M / 2.0 + 0.05;
    for z in [-MOORING_Z_OFFSET_M, MOORING_Z_OFFSET_M] {
        spawn_dock_child(
            commands,
            dock_entity,
            &assets.mooring,
            &assets.post_mat,
            Transform::from_xyz(outer_x, deck_y + MOORING_HEIGHT_M / 2.0, z),
        );
    }

    // Rail around the outer end: posts at the outer corners and a pair 5.4 m
    // inward, with horizontal rails between them and across the outer edge.
    let rail_z = DECK_WIDTH_M / 2.0 - RAIL_CROSS_M / 2.0;
    let rail_post_y = deck_y + RAIL_HEIGHT_M / 2.0;
    let rail_y = deck_y + RAIL_TOP_M;
    for x in [
        DECK_LENGTH_M - RAIL_INNER_BACK_M,
        DECK_LENGTH_M - RAIL_OUTER_BACK_M,
    ] {
        for z in [-rail_z, rail_z] {
            spawn_dock_child(
                commands,
                dock_entity,
                &assets.rail_post,
                &assets.post_mat,
                Transform::from_xyz(x, rail_post_y, z),
            );
        }
    }
    let side_rail_x = DECK_LENGTH_M - (RAIL_INNER_BACK_M + RAIL_OUTER_BACK_M) / 2.0;
    for z in [-rail_z, rail_z] {
        spawn_dock_child(
            commands,
            dock_entity,
            &assets.side_rail,
            &assets.post_mat,
            Transform::from_xyz(side_rail_x, rail_y, z),
        );
    }
    spawn_dock_child(
        commands,
        dock_entity,
        &assets.end_rail,
        &assets.post_mat,
        Transform::from_xyz(DECK_LENGTH_M - RAIL_OUTER_BACK_M, rail_y, 0.0),
    );

    // Ramp at the shore end, pitched down toward the beach (positive Z
    // rotation raises the deck-side end, so the shore end drops).
    let ramp_y =
        deck_y - RAMP_THICKNESS_M / 2.0 - (RAMP_LENGTH_M / 2.0) * RAMP_PITCH_RAD.sin();
    spawn_dock_child(
        commands,
        dock_entity,
        &assets.ramp,
        &assets.deck_mat,
        Transform::from_translation(Vec3::new(-RAMP_LENGTH_M / 2.0, ramp_y, 0.0))
            .with_rotation(Quat::from_rotation_z(RAMP_PITCH_RAD)),
    );

    // A few crates and barrels near the shore end.
    for (x, z) in [(2.0, 2.9), (2.6, 3.4)] {
        spawn_dock_child(
            commands,
            dock_entity,
            &assets.crate_box,
            &assets.deck_mat,
            Transform::from_xyz(x, deck_y + CRATE_SIZE_M / 2.0, z),
        );
    }
    spawn_dock_child(
        commands,
        dock_entity,
        &assets.crate_box,
        &assets.deck_mat,
        Transform::from_xyz(2.0, deck_y + CRATE_SIZE_M * 1.5, 2.9),
    );
    for (x, z) in [(2.4, -2.7), (3.1, -3.3)] {
        spawn_dock_child(
            commands,
            dock_entity,
            &assets.barrel,
            &assets.post_mat,
            Transform::from_xyz(x, deck_y + BARREL_HEIGHT_M / 2.0, z),
        );
    }

    commands.entity(zone_entity).add_child(dock_entity);
}

/// Tries each probe around the island until one resolves to a shore point,
/// then spawns the dock there. Returns true if a dock was placed.
fn try_spawn_dock(
    commands: &mut Commands,
    zone_entity: Entity,
    zone_data: Option<&ZoneLoaderAsset>,
    spawn: &DockSpawn,
    assets: &DockAssets,
) -> bool {
    let Some(zone_data) = zone_data else {
        return false;
    };

    for probe in spawn.probes {
        if let Some(shore_cm) = find_nearest_shore_position(
            Vec3::new(probe.x, probe.y, 0.0),
            zone_data,
            WATER_HEIGHT_CM,
        ) {
            spawn_dock_at(commands, zone_entity, spawn, shore_cm, assets);
            return true;
        }
    }
    false
}

/// Spawns the docks once when zone 200 loads. Falls back to spawning if the
/// ZoneEvent::Loaded event was missed but the zone entity already exists, and
/// resets the guard when the zone has been unloaded so re-entry respawns
/// them. Same pattern as spawn_dock_npcs_system in src/zone_content/npcs.rs.
pub fn spawn_docks_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut zone_events: MessageReader<ZoneEvent>,
    zone_query: Query<(Entity, &Zone)>,
    mut spawned: Local<bool>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
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
            let zone_data = current_zone
                .as_ref()
                .and_then(|zone| zone_loader_assets.get(&zone.handle));

            // Shared meshes and materials for every dock.
            let assets = DockAssets {
                plank: meshes.add(Mesh::from(Cuboid::new(
                    PLANK_LENGTH_M,
                    PLANK_THICKNESS_M,
                    PLANK_WIDTH_M,
                ))),
                cap: meshes.add(Mesh::from(Cuboid::new(
                    CAP_LENGTH_M,
                    PLANK_THICKNESS_M,
                    PLANK_WIDTH_M,
                ))),
                beam: meshes.add(Mesh::from(Cuboid::new(
                    BEAM_LENGTH_M,
                    BEAM_CROSS_M,
                    BEAM_CROSS_M,
                ))),
                post: meshes.add(Mesh::from(Cuboid::new(
                    POST_CROSS_M,
                    POST_HEIGHT_M,
                    POST_CROSS_M,
                ))),
                mooring: meshes.add(Mesh::from(Cuboid::new(
                    MOORING_CROSS_M,
                    MOORING_HEIGHT_M,
                    MOORING_CROSS_M,
                ))),
                rail_post: meshes.add(Mesh::from(Cuboid::new(
                    RAIL_CROSS_M,
                    RAIL_HEIGHT_M,
                    RAIL_CROSS_M,
                ))),
                side_rail: meshes.add(Mesh::from(Cuboid::new(
                    RAIL_SIDE_LENGTH_M,
                    RAIL_CROSS_M,
                    RAIL_CROSS_M,
                ))),
                end_rail: meshes.add(Mesh::from(Cuboid::new(
                    RAIL_CROSS_M,
                    RAIL_CROSS_M,
                    RAIL_END_LENGTH_M,
                ))),
                ramp: meshes.add(Mesh::from(Cuboid::new(
                    RAMP_LENGTH_M,
                    RAMP_THICKNESS_M,
                    RAMP_WIDTH_M,
                ))),
                crate_box: meshes.add(Mesh::from(Cuboid::new(
                    CRATE_SIZE_M,
                    CRATE_SIZE_M,
                    CRATE_SIZE_M,
                ))),
                barrel: meshes.add(Mesh::from(Cylinder::new(
                    BARREL_RADIUS_M,
                    BARREL_HEIGHT_M,
                ))),
                deck_mat: materials.add(StandardMaterial {
                    base_color: Color::srgb(0.52, 0.37, 0.21),
                    perceptual_roughness: 0.8,
                    ..default()
                }),
                post_mat: materials.add(StandardMaterial {
                    base_color: Color::srgb(0.30, 0.22, 0.14),
                    perceptual_roughness: 0.85,
                    ..default()
                }),
            };

            for spawn in DOCKS.iter() {
                try_spawn_dock(&mut commands, zone_entity, zone_data, spawn, &assets);
            }

            *spawned = true;
        }
        (_, false, None) => {}
    }
}
