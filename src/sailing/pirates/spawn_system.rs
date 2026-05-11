//! Pirate ship spawning system.
//!
//! Spawns pirate ships at designated spawn points when Zone 200 (SailingZone) is loaded.
//! Generates patrol waypoints around nearby islands for each ship.

use bevy::prelude::*;
use rand::Rng;

use crate::components::Zone;
use crate::events::ZoneEvent;

use super::components::{PirateShip, PirateShipSettings, PirateShipWaypoint, PirateSpawnPoint};

/// Zone ID for the sailing zone.
const SAILING_ZONE_ID: u16 = 200;

/// Island centers from zone manifest (meters, world space).
const ISLAND_CENTERS: [(f32, f32); 17] = [
    (5200.0, -5200.0),  // Island 0 - large central island
    (2000.0, -2000.0),  // Island 1
    (8000.0, -2000.0),  // Island 2
    (2000.0, -8000.0),  // Island 3
    (8000.0, -8000.0),  // Island 4
    (5200.0, -2000.0),  // Island 5
    (5200.0, -8000.0),  // Island 6
    (3000.0, -5200.0),  // Island 7
    (7400.0, -5200.0),  // Island 8
    (1000.0, -5200.0),  // Island 9
    (9400.0, -5200.0),  // Island 10
    (5200.0, -1000.0),  // Island 11
    (5200.0, -9400.0),  // Island 12
    (3500.0, -3500.0),  // Island 13
    (6900.0, -3500.0),  // Island 14
    (3500.0, -6900.0),  // Island 15
    (6900.0, -6900.0),  // Island 16
];

/// Island radii in meters.
const ISLAND_RADII: [f32; 17] = [
    300.0, 150.0, 120.0, 180.0, 100.0, 200.0, 160.0, 140.0, 130.0,
    100.0, 110.0, 90.0, 80.0, 50.0, 40.0, 45.0, 35.0,
];

/// Pirate spawn point positions (meters, world space).
/// These are positioned in open water between islands.
const PIRATE_SPAWN_POINTS: [(f32, f32); 3] = [
    (3600.0, -4000.0),   // Between islands 1, 7, 13
    (6600.0, -4000.0),   // Between islands 5, 8, 14
    (4200.0, -7200.0),   // Between islands 3, 6, 15
];

/// System that spawns pirate ships when the sailing zone is loaded.
pub fn pirate_ship_spawn_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<PirateShipSettings>,
    mut zone_events: MessageReader<ZoneEvent>,
    zone_query: Query<Entity, With<Zone>>,
    existing_ships: Query<Entity, With<PirateShip>>,
) {
    if !settings.enabled {
        return;
    }

    for event in zone_events.read() {
        let ZoneEvent::Loaded(zone_id) = event;
        if zone_id.get() != SAILING_ZONE_ID {
            continue;
        }

        let zone_entity = zone_query.iter().next().copied().unwrap_or(Entity::PLACEHOLDER);
        let existing_count = existing_ships.iter().len();

        log::info!(
            "[PIRATE] Zone 200 loaded, spawning pirate ships (existing: {})",
            existing_count
        );

        let ships_to_spawn = (settings.max_ships - existing_count).min(settings.max_ships);

        for i in 0..ships_to_spawn {
            spawn_pirate_ship(
                &mut commands,
                &mut meshes,
                &mut materials,
                i as u32,
                &settings,
                zone_entity,
            );
        }

        log::info!("[PIRATE] Spawned {} pirate ships", ships_to_spawn);
    }
}

/// Spawn a single pirate ship at a spawn point.
#[allow(clippy::too_many_arguments)]
fn spawn_pirate_ship(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    ship_id: u32,
    settings: &PirateShipSettings,
    zone_entity: Entity,
) {
    let mut rng = rand::thread_rng();

    // Pick a spawn point (cycle through available points)
    let spawn_idx = ship_id as usize % PIRATE_SPAWN_POINTS.len();
    let (spawn_x_m, spawn_z_m) = PIRATE_SPAWN_POINTS[spawn_idx];

    // Convert to game-space centimeters
    // Position uses: x = world_x * 100, y = -world_z * 100, z = height
    let spawn_position = Vec3::new(
        spawn_x_m * 100.0,
        -spawn_z_m * 100.0,
        0.0, // Water surface height
    );

    // Generate patrol waypoints around nearby islands
    let waypoints = generate_patrol_waypoints(spawn_x_m, spawn_z_m, &mut rng);

    // Create the pirate ship component
    let pirate_ship = PirateShip::new(ship_id, spawn_position, waypoints);

    // Create the boat state for sailing physics
    let boat_state = BoatState {
        active: true,
        heading: rng.gen_range(0.0..std::f32::consts::TAU),
        speed: 0.0,
        max_speed: 8.0,
        sail_trim: std::f32::consts::FRAC_PI_4,
        hull_health: pirate_ship.health,
        hull_max_health: pirate_ship.max_health,
        ..Default::default()
    };

    // Create position component
    let position = Position::new(spawn_position);

    // Create client entity for game integration
    let client_entity = ClientEntity::new(
        rose_game_common::messages::ClientEntityId::new(ship_id as u32 + 10000),
        crate::components::ClientEntityType::Monster,
    );

    // Create name tag
    let names = ["Black Pearl", "Dead Man's Chest", "Flying Dutchman",
                 "Jolly Roger", "Ghost Ship", "Kraken's Maw"];
    let name = names[ship_id as usize % names.len()];
    let client_entity_name = ClientEntityName::new(format!("{} {}", name, ship_id + 1));

    // Spawn the ship entity
    let ship_entity = commands
        .spawn((
            pirate_ship,
            boat_state,
            position,
            client_entity,
            client_entity_name,
            Transform::from_translation(spawn_position / 100.0)
                .with_scale(Vec3::splat(1.5)),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Spawn visual hull as child
    let hull_entity = spawn_ship_visual(commands, meshes, materials);
    commands.entity(ship_entity).add_child(hull_entity);

    // Parent to zone entity
    if zone_entity != Entity::PLACEHOLDER {
        commands.entity(zone_entity).add_child(ship_entity);
    }

    log::info!(
        "[PIRATE] Spawned pirate ship #{} ({}) at position {:?}",
        ship_id,
        name,
        spawn_position
    );
}

/// Generate patrol waypoints around nearby islands.
fn generate_patrol_waypoints(
    spawn_x_m: f32,
    spawn_z_m: f32,
    rng: &mut impl Rng,
) -> Vec<PirateShipWaypoint> {
    let mut waypoints = Vec::new();

    // Find 3-4 nearby islands to patrol around
    let mut distances: Vec<(f32, usize)> = ISLAND_CENTERS
        .iter()
        .enumerate()
        .map(|(idx, &(cx, cz))| {
            let dx = spawn_x_m - cx;
            let dz = spawn_z_m - cz;
            (dx * dx + dz * dz).sqrt(), idx
        })
        .collect();
    distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // Pick 3 nearest islands (excluding the one we're too close to)
    let patrol_islands: Vec<usize> = distances
        .iter()
        .take(4)
        .filter(|&&(dist, _)| dist > 200.0) // Skip if too close
        .map(|&(_, idx)| idx)
        .collect();

    for &island_idx in &patrol_islands {
        let (cx, cz) = ISLAND_CENTERS[island_idx];
        let radius = ISLAND_RADII[island_idx];

        // Generate 2 waypoints around each island at a safe distance
        for _ in 0..2 {
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let patrol_distance = radius + 150.0 + rng.gen_range(0.0..100.0);
            let wx = cx + angle.cos() * patrol_distance;
            let wz = cz + angle.sin() * patrol_distance;

            // Convert to game-space centimeters
            let pos = Vec3::new(wx * 100.0, -wz * 100.0, 0.0);

            waypoints.push(PirateShipWaypoint::new(pos, 2.0 + rng.gen_range(0.0..3.0)));
        }
    }

    // Add the spawn point as first waypoint
    let spawn_wp = Vec3::new(spawn_x_m * 100.0, -spawn_z_m * 100.0, 0.0);
    waypoints.insert(0, PirateShipWaypoint::new(spawn_wp, 0.0));

    waypoints
}

/// Spawn the visual representation of a pirate ship.
fn spawn_ship_visual(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> Entity {
    // Dark hull material
    let hull_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.10, 0.08),
        perceptual_roughness: 0.85,
        metallic: 0.05,
        ..Default::default()
    });

    // Red/black sail material
    let sail_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.60, 0.10, 0.10),
        perceptual_roughness: 0.6,
        ..Default::default()
    });

    // Mast material
    let mast_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.18, 0.12),
        perceptual_roughness: 0.8,
        ..Default::default()
    });

    // Cannon material
    let cannon_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.3),
        perceptual_roughness: 0.4,
        metallic: 0.7,
        ..Default::default()
    });

    // Hull
    let hull_mesh = meshes.add(Mesh::from(Cuboid::new(1.0, 0.5, 3.5)));
    let hull = commands
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

    // Deck
    let deck_mesh = meshes.add(Mesh::from(Cuboid::new(1.1, 0.1, 3.0)));
    let deck = commands
        .spawn((
            Mesh3d(deck_mesh),
            MeshMaterial3d(mast_mat.clone()),
            Transform::from_xyz(0.0, 0.15, 0.0),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Main mast
    let mast_mesh = meshes.add(Mesh::from(Cuboid::new(0.1, 4.0, 0.1)));
    let mast = commands
        .spawn((
            Mesh3d(mast_mesh),
            MeshMaterial3d(mast_mat.clone()),
            Transform::from_xyz(0.0, 1.5, 0.0),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Sail
    let sail_mesh = meshes.add(Mesh::from(Cuboid::new(2.5, 2.5, 0.05)));
    let sail = commands
        .spawn((
            Mesh3d(sail_mesh),
            MeshMaterial3d(sail_mat),
            Transform::from_xyz(0.0, 2.0, 0.3),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Cannons (port side)
    let cannon_mesh = meshes.add(Mesh::from(Cuboid::new(0.15, 0.15, 0.6)));
    for z_offset in [-1.0, 0.0, 1.0] {
        let cannon = commands
            .spawn((
                Mesh3d(cannon_mesh.clone()),
                MeshMaterial3d(cannon_mat.clone()),
                Transform::from_xyz(-1.1, 0.05, z_offset),
                GlobalTransform::default(),
                Visibility::Inherited,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .id();
        commands.entity(hull).add_child(cannon);
    }

    // Cannons (starboard side)
    for z_offset in [-1.0, 0.0, 1.0] {
        let cannon = commands
            .spawn((
                Mesh3d(cannon_mesh.clone()),
                MeshMaterial3d(cannon_mat.clone()),
                Transform::from_xyz(1.1, 0.05, z_offset),
                GlobalTransform::default(),
                Visibility::Inherited,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .id();
        commands.entity(hull).add_child(cannon);
    }

    // Parent everything to hull
    commands.entity(hull).add_child(deck);
    commands.entity(hull).add_child(mast);
    commands.entity(hull).add_child(sail);

    hull
}
