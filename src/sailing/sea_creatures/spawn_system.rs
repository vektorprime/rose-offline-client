//! Sea creature spawning system.
//!
//! Spawns Krakens, Sharks, and Whales at designated spawn points when Zone 200
//! (SailingZone) is loaded. Each creature type has its own spawn logic and
//! visual representation.

use bevy::prelude::*;
use rand::Rng;

use crate::components::Zone;
use crate::events::ZoneEvent;

use super::components::{
    Kraken, KrakenSpawnPoint, SeaCreatureSettings, Shark, SharkPackRole, SharkSpawnPoint,
    Whale, WhaleSpawnPoint, WhaleWaypoint,
};

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

/// Kraken spawn point positions (meters, world space).
/// Positioned in deep water far from islands.
const KRAKEN_SPAWN_POINTS: [(f32, f32); 1] = [
    (5200.0, -5200.0), // Center of the zone, deep water
];

/// Shark spawn point positions (meters, world space).
/// Positioned near islands where boats might hide.
const SHARK_SPAWN_POINTS: [(f32, f32, usize); 4] = [
    (2500.0, -3000.0, 4),  // Between islands 1, 7, 13
    (7000.0, -7000.0, 4),  // Near island 16
    (4000.0, -1500.0, 3),  // Between islands 5, 11
    (8500.0, -4000.0, 3),  // Between islands 2, 8, 10
];

/// Whale spawn point positions (meters, world space).
/// Positioned in open ocean channels.
const WHALE_SPAWN_POINTS: [(f32, f32); 2] = [
    (5200.0, -3500.0), // Central ocean channel
    (5200.0, -7000.0), // Southern ocean channel
];

/// System that spawns all sea creatures when the sailing zone is loaded.
pub fn sea_creature_spawn_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<SeaCreatureSettings>,
    mut zone_events: MessageReader<ZoneEvent>,
    zone_query: Query<Entity, With<Zone>>,
    existing_krakens: Query<Entity, With<Kraken>>,
    existing_sharks: Query<Entity, With<Shark>>,
    existing_whales: Query<Entity, With<Whale>>,
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

        let kraken_count = existing_krakens.iter().len();
        let shark_count = existing_sharks.iter().len();
        let whale_count = existing_whales.iter().len();

        log::info!(
            "[SEA_CREATURES] Zone 200 loaded, spawning sea creatures (krakens: {}, sharks: {}, whales: {})",
            kraken_count, shark_count, whale_count
        );

        // Spawn Kraken (max 1)
        if kraken_count < settings.max_krakens {
            spawn_kraken(
                &mut commands,
                &mut meshes,
                &mut materials,
                0,
                &settings,
                zone_entity,
            );
        }

        // Spawn shark packs
        let sharks_to_spawn = settings.max_sharks.saturating_sub(shark_count);
        spawn_shark_packs(
            &mut commands,
            &mut meshes,
            &mut materials,
            sharks_to_spawn,
            &settings,
            zone_entity,
        );

        // Spawn whales
        let whales_to_spawn = settings.max_whales.saturating_sub(whale_count);
        spawn_whales(
            &mut commands,
            &mut meshes,
            &mut materials,
            whales_to_spawn,
            &settings,
            zone_entity,
        );
    }
}

/// Spawn a single Kraken at a spawn point.
#[allow(clippy::too_many_arguments)]
fn spawn_kraken(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    creature_id: u32,
    _settings: &SeaCreatureSettings,
    zone_entity: Entity,
) {
    let mut rng = rand::thread_rng();

    let (spawn_x_m, spawn_z_m) = KRAKEN_SPAWN_POINTS[creature_id as usize % KRAKEN_SPAWN_POINTS.len()];

    // Add random offset to avoid exact overlap
    let offset_x = rng.gen_range(-500.0..500.0);
    let offset_z = rng.gen_range(-500.0..500.0);

    let spawn_position = Vec3::new(
        (spawn_x_m + offset_x) * 100.0,
        -(spawn_z_m + offset_z) * 100.0,
        -500.0, // Start deep underwater
    );

    let kraken = Kraken::new(creature_id, spawn_position);

    let position = crate::components::Position::new(spawn_position);

    let client_entity = crate::components::ClientEntity::new(
        rose_game_common::messages::ClientEntityId::new(creature_id as u32 + 50000),
        crate::components::ClientEntityType::Monster,
    );

    let client_entity_name = crate::components::ClientEntityName::new(format!("Kraken #{}", creature_id + 1));

    // Spawn the Kraken entity
    let kraken_entity = commands
        .spawn((
            kraken,
            position,
            client_entity,
            client_entity_name,
            Transform::from_translation(spawn_position / 100.0)
                .with_scale(Vec3::splat(8.0)),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Spawn Kraken visual body
    let body_entity = spawn_kraken_visual(commands, meshes, materials);
    commands.entity(kraken_entity).add_child(body_entity);

    // Spawn tentacles
    for i in 0..8 {
        let angle_offset = (i as f32 / 8.0) * std::f32::consts::TAU;
        let tentacle_length = 3.0 + rng.gen_range(0.0..2.0);

        let tentacle = super::components::KrakenTentacle::new(i, angle_offset, tentacle_length);

        let tentacle_entity = commands
            .spawn((
                tentacle,
                Transform::from_xyz(0.0, -1.0, 0.0)
                    .with_rotation(Quat::from_axis_angle(Vec3::Y, angle_offset))
                    .with_scale(Vec3::new(0.3, tentacle_length, 0.3)),
                GlobalTransform::default(),
                Visibility::Inherited,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .id();

        commands.entity(body_entity).add_child(tentacle_entity);
    }

    // Parent to zone entity
    if zone_entity != Entity::PLACEHOLDER {
        commands.entity(zone_entity).add_child(kraken_entity);
    }

    log::info!(
        "[SEA_CREATURES] Spawned Kraken #{} at position {:?}",
        creature_id,
        spawn_position
    );
}

/// Spawn shark packs at spawn points.
#[allow(clippy::too_many_arguments)]
fn spawn_shark_packs(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    max_sharks: usize,
    settings: &SeaCreatureSettings,
    zone_entity: Entity,
) {
    let mut rng = rand::thread_rng();
    let mut total_spawned = 0;
    let mut creature_id: u32 = 0;

    for &(spawn_x_m, spawn_z_m, pack_size) in &SHARK_SPAWN_POINTS {
        if total_spawned >= max_sharks {
            break;
        }

        let pack_id = creature_id;
        let roles = [
            SharkPackRole::Alpha,
            SharkPackRole::Flanker,
            SharkPackRole::Harasser,
            SharkPackRole::Ambusher,
        ];

        let spawn_count = pack_size.min(max_sharks - total_spawned);

        for i in 0..spawn_count {
            // Spread sharks around the spawn point
            let angle = (i as f32 / spawn_count as f32) * std::f32::consts::TAU;
            let spread = rng.gen_range(50.0..150.0);
            let offset_x = spawn_x_m + angle.cos() * spread;
            let offset_z = spawn_z_m + angle.sin() * spread;

            let spawn_position = Vec3::new(
                offset_x * 100.0,
                -offset_z * 100.0,
                -200.0, // Start underwater
            );

            let role = roles[i % roles.len()];
            let shark = Shark::new(creature_id, spawn_position, pack_id, role);

            let position = crate::components::Position::new(spawn_position);

            let client_entity = crate::components::ClientEntity::new(
                rose_game_common::messages::ClientEntityId::new(creature_id as u32 + 60000),
                crate::components::ClientEntityType::Monster,
            );

            let client_entity_name = crate::components::ClientEntityName::new(format!("Shark #{}", creature_id + 1));

            let shark_entity = commands
                .spawn((
                    shark,
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

            // Spawn shark visual
            let visual_entity = spawn_shark_visual(commands, meshes, materials);
            commands.entity(shark_entity).add_child(visual_entity);

            if zone_entity != Entity::PLACEHOLDER {
                commands.entity(zone_entity).add_child(shark_entity);
            }

            creature_id += 1;
            total_spawned += 1;
        }
    }

    log::info!("[SEA_CREATURES] Spawned {} sharks in packs", total_spawned);
}

/// Spawn whales at spawn points.
#[allow(clippy::too_many_arguments)]
fn spawn_whales(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    max_whales: usize,
    settings: &SeaCreatureSettings,
    zone_entity: Entity,
) {
    let mut rng = rand::thread_rng();
    let mut creature_id: u32 = 0;

    for &(spawn_x_m, spawn_z_m) in &WHALE_SPAWN_POINTS {
        if creature_id as usize >= max_whales {
            break;
        }

        let spawn_position = Vec3::new(
            spawn_x_m * 100.0,
            -spawn_z_m * 100.0,
            0.0, // Start at surface
        );

        // Generate migration waypoints around the zone
        let waypoints = generate_whale_migration_waypoints(spawn_x_m, spawn_z_m, &mut rng);

        let whale = Whale::new(creature_id, spawn_position, waypoints);

        let position = crate::components::Position::new(spawn_position);

        let client_entity = crate::components::ClientEntity::new(
            rose_game_common::messages::ClientEntityId::new(creature_id as u32 + 70000),
            crate::components::ClientEntityType::Monster,
        );

        let names = ["Blue Whale", "Humpback Whale", "Sperm Whale"];
        let name = names[creature_id as usize % names.len()];
        let client_entity_name = crate::components::ClientEntityName::new(name.to_string());

        let whale_entity = commands
            .spawn((
                whale,
                position,
                client_entity,
                client_entity_name,
                Transform::from_translation(spawn_position / 100.0)
                    .with_scale(Vec3::splat(15.0)),
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .id();

        // Spawn whale visual
        let visual_entity = spawn_whale_visual(commands, meshes, materials);
        commands.entity(whale_entity).add_child(visual_entity);

        if zone_entity != Entity::PLACEHOLDER {
            commands.entity(zone_entity).add_child(whale_entity);
        }

        creature_id += 1;
    }

    log::info!("[SEA_CREATURES] Spawned {} whales", creature_id);
}

/// Generate migration waypoints for a whale.
fn generate_whale_migration_waypoints(
    spawn_x_m: f32,
    spawn_z_m: f32,
    rng: &mut impl Rng,
) -> Vec<WhaleWaypoint> {
    let mut waypoints = Vec::new();

    // Create a large circular migration route
    let radius = 2000.0 + rng.gen_range(0.0..1500.0);
    let num_waypoints = 6 + rng.gen_range(0..4);

    for i in 0..num_waypoints {
        let angle = (i as f32 / num_waypoints as f32) * std::f32::consts::TAU;
        let wx = spawn_x_m + angle.cos() * radius;
        let wz = spawn_z_m + angle.sin() * radius;

        let depth = rng.gen_range(0.0..0.5);
        let pause = 3.0 + rng.gen_range(0.0..5.0);

        let pos = Vec3::new(wx * 100.0, -wz * 100.0, 0.0);
        waypoints.push(WhaleWaypoint::new(pos, depth, pause));
    }

    waypoints
}

/// Spawn the visual representation of a Kraken.
fn spawn_kraken_visual(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> Entity {
    // Deep purple/dark body material
    let body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.15, 0.40),
        perceptual_roughness: 0.5,
        metallic: 0.1,
        emissive: Color::srgb(0.05, 0.02, 0.08).to_linear(),
        emissive_intensity: 0.5,
        ..Default::default()
    });

    // Eye material (glowing)
    let eye_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.1, 0.1),
        emissive: Color::srgb(0.8, 0.0, 0.0).to_linear(),
        emissive_intensity: 2.0,
        ..Default::default()
    });

    // Body - large sphere
    let body_mesh = meshes.add(Mesh::from(Sphere::new(1.0)));
    let body = commands
        .spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(body_mat),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Eyes
    let eye_mesh = meshes.add(Mesh::from(Sphere::new(0.15)));
    for x_offset in [-0.3, 0.3] {
        let eye = commands
            .spawn((
                Mesh3d(eye_mesh.clone()),
                MeshMaterial3d(eye_mat.clone()),
                Transform::from_xyz(x_offset, 0.3, 0.8),
                GlobalTransform::default(),
                Visibility::Inherited,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .id();
        commands.entity(body).add_child(eye);
    }

    body
}

/// Spawn the visual representation of a shark.
fn spawn_shark_visual(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> Entity {
    // Grey shark body material
    let body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.48, 0.50),
        perceptual_roughness: 0.4,
        metallic: 0.05,
        ..Default::default()
    });

    // White underbelly material
    let belly_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.85, 0.82),
        perceptual_roughness: 0.5,
        ..Default::default()
    });

    // Fin material (darker)
    let fin_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.32, 0.35),
        perceptual_roughness: 0.4,
        ..Default::default()
    });

    // Body - elongated shape
    let body_mesh = meshes.add(Mesh::from(Cuboid::new(0.4, 0.2, 1.5)));
    let body = commands
        .spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(body_mat),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Belly
    let belly_mesh = meshes.add(Mesh::from(Cuboid::new(0.35, 0.05, 1.4)));
    let belly = commands
        .spawn((
            Mesh3d(belly_mesh),
            MeshMaterial3d(belly_mat),
            Transform::from_xyz(0.0, -0.2, 0.0),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();
    commands.entity(body).add_child(belly);

    // Dorsal fin
    let fin_mesh = meshes.add(Mesh::from(Cuboid::new(0.05, 0.4, 0.5)));
    let fin = commands
        .spawn((
            Mesh3d(fin_mesh),
            MeshMaterial3d(fin_mat),
            Transform::from_xyz(0.0, 0.3, -0.2),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();
    commands.entity(body).add_child(fin);

    // Tail
    let tail_mesh = meshes.add(Mesh::from(Cuboid::new(0.6, 0.05, 0.1)));
    let tail = commands
        .spawn((
            Mesh3d(tail_mesh),
            MeshMaterial3d(fin_mat.clone()),
            Transform::from_xyz(0.0, 0.0, 1.0),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();
    commands.entity(body).add_child(tail);

    body
}

/// Spawn the visual representation of a whale.
fn spawn_whale_visual(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> Entity {
    // Blue-grey whale body material
    let body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.35, 0.45),
        perceptual_roughness: 0.35,
        metallic: 0.05,
        ..Default::default()
    });

    // White underbelly material
    let belly_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.90, 0.90, 0.88),
        perceptual_roughness: 0.4,
        ..Default::default()
    });

    // Body - massive elongated shape
    let body_mesh = meshes.add(Mesh::from(Cuboid::new(1.5, 1.0, 5.0)));
    let body = commands
        .spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(body_mat),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Belly
    let belly_mesh = meshes.add(Mesh::from(Cuboid::new(1.4, 0.2, 4.8)));
    let belly = commands
        .spawn((
            Mesh3d(belly_mesh),
            MeshMaterial3d(belly_mat),
            Transform::from_xyz(0.0, -0.6, 0.0),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();
    commands.entity(body).add_child(belly);

    // Tail fluke
    let tail_mesh = meshes.add(Mesh::from(Cuboid::new(2.5, 0.1, 0.5)));
    let tail = commands
        .spawn((
            Mesh3d(tail_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, 0.3, 3.0),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();
    commands.entity(body).add_child(tail);

    // Blowhole
    let blowhole_mesh = meshes.add(Mesh::from(Cylinder::new(0.2, 0.1)));
    let blowhole = commands
        .spawn((
            Mesh3d(blowhole_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, 0.6, -2.0),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();
    commands.entity(body).add_child(blowhole);

    body
}
