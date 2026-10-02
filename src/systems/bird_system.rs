//! Bird spawning and flying behavior system
//!
//! This system handles:
//! - Spawning a flock for each loaded zone (count relative to zone size)
//! - Applying Birds settings edits to the living flock immediately
//! - Bird flying AI (picking targets, moving towards them)
//! - Bird animation (wing flapping with rotating wings, vertical bobbing)
//! - Keeping birds within roam bounds
//! - Birds face their flight direction

use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::{asset::RenderAssetUsages, prelude::*};
use bevy_mesh::{Indices, Mesh, PrimitiveTopology};
use rand::Rng;

use crate::components::{Bird, BirdMesh, BirdSettings, BirdWingLeft, BirdWingRight, Zone};
use crate::zone_loader::zone_loaded_from_vfs_system;

/// Zone size in world units (64x64 blocks of 160 units), used for the bird
/// count and roam radius.
const ZONE_SIZE: f32 = 10240.0;

/// Wing child entities of a bird, stored at spawn so the flap animation can
/// address them directly instead of scanning every bird's children each frame.
#[derive(Component, Clone, Copy, Debug)]
pub struct BirdWings {
    pub left: Entity,
    pub right: Entity,
}

/// Plugin for bird systems
pub struct BirdPlugin;

impl Plugin for BirdPlugin {
    fn build(&self, app: &mut App) {
        log::info!("[BIRD] BirdPlugin::build() called - registering bird systems");
        app
            // Register types for reflection
            .register_type::<Bird>()
            .register_type::<BirdSettings>()
            // Add resources
            .init_resource::<BirdSettings>()
            // Add systems. After zone_loaded_from_vfs_system (and the command
            // flush after it): a zone swap is seen in the frame it happens, and
            // birds are never parented to a zone that is being despawned.
            .add_systems(
                Update,
                (
                    spawn_birds_on_zone_system.after(zone_loaded_from_vfs_system),
                    update_bird_movement_system,
                )
                    .chain(),
            );
    }
}

/// Meshes and materials shared by every bird. Built once and kept, so the
/// flocks of later zones and birds added from the settings reuse them.
struct BirdAssets {
    body_mesh: Handle<Mesh>,
    left_wing_mesh: Handle<Mesh>,
    right_wing_mesh: Handle<Mesh>,
    materials: Vec<Handle<StandardMaterial>>,
}

impl BirdAssets {
    fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        // Bird colors for variety - made more vibrant for visibility
        let bird_colors = [
            Color::srgb(0.6, 0.4, 0.2),  // Brighter brown
            Color::srgb(0.5, 0.5, 0.6),  // Lighter gray
            Color::srgb(0.2, 0.2, 0.25), // Dark but visible
            Color::srgb(0.7, 0.5, 0.3),  // Light brown
            Color::srgb(0.6, 0.6, 0.7),  // Light gray
            Color::srgb(0.8, 0.7, 0.5),  // Tan
            Color::srgb(0.4, 0.3, 0.2),  // Dark brown
        ];

        Self {
            body_mesh: create_bird_body_mesh(meshes),
            left_wing_mesh: create_bird_wing_mesh(meshes, false),
            right_wing_mesh: create_bird_wing_mesh(meshes, true),
            materials: bird_colors
                .iter()
                .map(|&color| {
                    materials.add(StandardMaterial {
                        base_color: color,
                        unlit: true, // Birds don't need complex lighting for distance viewing
                        cull_mode: None,
                        ..default()
                    })
                })
                .collect(),
        }
    }
}

/// State of [`spawn_birds_on_zone_system`]: the living flock.
#[derive(Default)]
pub struct BirdFlock {
    /// Zone entity the living birds are parented to; `None` while there are none.
    zone_entity: Option<Entity>,
    /// Settings the living birds reflect (meaningful while `zone_entity` is set).
    settings: BirdSettings,
    /// Built on the first spawn.
    assets: Option<BirdAssets>,
}

/// Keeps one flock of birds in the loaded zone, in line with `BirdSettings`.
///
/// - A new zone entity (first load, zone change) gets a new flock. The previous
///   zone's birds are its children and were despawned with it. A repeated
///   `ZoneEvent::Loaded` for the zone on screen (respawn, same-zone teleport)
///   keeps its zone entity, so it does not add a second flock.
/// - Disabling birds despawns the flock; enabling spawns one in the current zone.
/// - Other edits apply to the living flock in place (bird count, speed, altitude,
///   roam radius), so dragging a slider never respawns the flock and makes the
///   birds jump. Flap and bob speeds are read every frame by
///   [`update_bird_movement_system`].
pub fn spawn_birds_on_zone_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<BirdSettings>,
    zone_query: Query<Entity, With<Zone>>,
    mut bird_query: Query<(Entity, &mut Bird)>,
    mut flock: Local<BirdFlock>,
) {
    let flock = &mut *flock;

    // There is at most one zone: zone_loaded_from_vfs_system despawns the old
    // zone in the same command flush that spawns the new one.
    let zone_entity = zone_query.iter().next().filter(|_| settings.enabled);

    if zone_entity != flock.zone_entity {
        // Only finds birds when birds were disabled: a replaced zone already
        // took its birds with it.
        let mut despawned = 0;
        for (bird_entity, _) in bird_query.iter() {
            commands.entity(bird_entity).despawn();
            despawned += 1;
        }
        if despawned > 0 {
            log::info!("[BIRD] Despawned {} birds", despawned);
        }
        flock.zone_entity = None;

        if let Some(zone_entity) = zone_entity {
            let bird_count = calculate_bird_count(ZONE_SIZE, &settings);
            log::info!(
                "[BIRD] Spawning {} birds parented to zone entity {:?} (roam radius {})",
                bird_count,
                zone_entity,
                bird_roam_radius(&settings)
            );
            let assets = flock
                .assets
                .get_or_insert_with(|| BirdAssets::new(&mut meshes, &mut materials));
            spawn_birds(&mut commands, assets, &settings, zone_entity, bird_count);
            flock.zone_entity = Some(zone_entity);
            flock.settings = settings.clone();
        }
        return;
    }

    // Same zone: only settings edits matter. set_if_neq in the settings window
    // flags real edits only; the comparison skips anything else.
    let Some(zone_entity) = zone_entity else {
        return;
    };
    if !settings.is_changed() || flock.settings == *settings {
        return;
    }

    // Bird count: add the missing birds or despawn the extra ones.
    let bird_count = calculate_bird_count(ZONE_SIZE, &settings);
    let alive_count = bird_query.iter().count();
    if bird_count > alive_count {
        let assets = flock
            .assets
            .get_or_insert_with(|| BirdAssets::new(&mut meshes, &mut materials));
        spawn_birds(
            &mut commands,
            assets,
            &settings,
            zone_entity,
            bird_count - alive_count,
        );
    } else {
        for (bird_entity, _) in bird_query.iter().take(alive_count - bird_count) {
            commands.entity(bird_entity).despawn();
        }
    }

    // Speed range, altitude band and roam radius: adjust the living birds.
    let (old_min_speed, old_max_speed) = bird_speed_range(&flock.settings);
    let (min_speed, max_speed) = bird_speed_range(&settings);
    let speed_changed = (old_min_speed, old_max_speed) != (min_speed, max_speed);
    let roam_radius = bird_roam_radius(&settings);
    let (min_altitude, max_altitude) = bird_altitude_range(&settings);
    let area_changed = roam_radius != bird_roam_radius(&flock.settings)
        || (min_altitude, max_altitude) != bird_altitude_range(&flock.settings);

    if speed_changed || area_changed {
        let mut rng = rand::thread_rng();
        for (_, mut bird) in bird_query.iter_mut() {
            if speed_changed {
                // Keep each bird's place within the range (random if the old
                // range was a single value).
                let t = if old_max_speed > old_min_speed {
                    ((bird.speed - old_min_speed) / (old_max_speed - old_min_speed)).clamp(0.0, 1.0)
                } else {
                    rng.gen::<f32>()
                };
                bird.speed = min_speed + t * (max_speed - min_speed);
            }

            if area_changed {
                // Move the current target into the new roam area and altitude
                // band: the bird flies there instead of jumping.
                bird.roam_radius = roam_radius;
                let center = bird.roam_center;
                let target = bird.target_position;
                let horizontal = Vec2::new(target.x - center.x, target.z - center.z)
                    .clamp_length_max(roam_radius);
                bird.target_position = Vec3::new(
                    center.x + horizontal.x,
                    center.y + (target.y - center.y).clamp(min_altitude, max_altitude),
                    center.z + horizontal.y,
                );
            }
        }
    }

    flock.settings = settings.clone();
}

/// Roam radius around the zone center.
fn bird_roam_radius(settings: &BirdSettings) -> f32 {
    ZONE_SIZE * settings.roam_radius_multiplier * 0.5
}

/// (min, max) flight altitude, ordered even if the settings are not.
fn bird_altitude_range(settings: &BirdSettings) -> (f32, f32) {
    (
        settings.min_altitude.min(settings.max_altitude),
        settings.max_altitude.max(settings.min_altitude),
    )
}

/// (min, max) flight speed, ordered even if the settings are not.
fn bird_speed_range(settings: &BirdSettings) -> (f32, f32) {
    (
        settings.min_speed.min(settings.max_speed),
        settings.max_speed.max(settings.min_speed),
    )
}

/// Calculate bird count based on zone size
fn calculate_bird_count(zone_size: f32, settings: &BirdSettings) -> usize {
    // Zone area in millions of square units
    let zone_area = zone_size * zone_size;
    let area_in_1000_units = zone_area / (1000.0 * 1000.0);

    // Calculate bird count based on area
    let calculated_count = (area_in_1000_units * settings.birds_per_1000_units) as usize;

    // Clamp to min/max (ordered: `Ord::clamp` panics when min > max)
    let min_count = settings.min_birds_per_zone;
    let max_count = settings.max_birds_per_zone.max(min_count);
    calculated_count.clamp(min_count, max_count)
}

/// Spawns `bird_count` birds roaming around the zone center, parented to the
/// zone entity (so they use zone-local coordinates and go away with the zone).
fn spawn_birds(
    commands: &mut Commands,
    assets: &BirdAssets,
    settings: &BirdSettings,
    zone_entity: Entity,
    bird_count: usize,
) {
    let mut rng = rand::thread_rng();

    // Zone-local origin: birds are children of the zone entity
    let roam_center = Vec3::ZERO;
    let roam_radius = bird_roam_radius(settings);
    // Inclusive ranges below: min == max is a valid setting, and an empty
    // exclusive range panics in gen_range
    let (min_altitude, max_altitude) = bird_altitude_range(settings);
    let (min_speed, max_speed) = bird_speed_range(settings);

    for _ in 0..bird_count {
        // Random position within roam radius
        let angle = rng.gen::<f32>() * std::f32::consts::TAU;
        let distance = rng.gen::<f32>() * roam_radius;
        let altitude = rng.gen_range(min_altitude..=max_altitude);
        let position =
            roam_center + Vec3::new(angle.cos() * distance, altitude, angle.sin() * distance);

        let speed = rng.gen_range(min_speed..=max_speed);
        let initial_phase = rng.gen::<f32>() * std::f32::consts::TAU;

        // Random color material
        let material = assets.materials[rng.gen_range(0..assets.materials.len())].clone();

        let target_position = get_new_target(roam_center, roam_radius, min_altitude, max_altitude);

        // Initial rotation facing the target
        let direction = target_position - position;
        let initial_rotation = if direction.length() > 0.01 {
            let look_direction = direction.normalize();
            // Bird body faces +Z (forward), so we need to rotate to face movement direction
            Quat::from_rotation_y((-look_direction.x).atan2(look_direction.z))
        } else {
            Quat::IDENTITY
        };

        // Spawn bird entity (parent container)
        let bird_entity = commands
            .spawn((
                Bird {
                    speed,
                    target_position,
                    roam_center,
                    roam_radius,
                    flap_phase: initial_phase,
                    bob_phase: initial_phase * 0.5,
                },
                Transform::from_translation(position)
                    .with_rotation(initial_rotation)
                    .with_scale(Vec3::splat(1.5)), // Scale for visibility
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .id();

        // Spawn bird body mesh as child. NotShadowCaster: small alpha birds must
        // never pay the 2048 shadow-map pass.
        let body_entity = commands
            .spawn((
                BirdMesh,
                Mesh3d(assets.body_mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::default(),
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
                bevy::light::NotShadowCaster,
            ))
            .id();
        commands.entity(bird_entity).add_child(body_entity);

        // Spawn left wing as child (rotates around body center)
        let left_wing_entity = commands
            .spawn((
                BirdWingLeft,
                Mesh3d(assets.left_wing_mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_rotation(Quat::from_rotation_z(0.3)), // Slightly spread
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
                bevy::light::NotShadowCaster,
            ))
            .id();
        commands.entity(bird_entity).add_child(left_wing_entity);

        // Spawn right wing as child (rotates around body center)
        let right_wing_entity = commands
            .spawn((
                BirdWingRight,
                Mesh3d(assets.right_wing_mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_rotation(Quat::from_rotation_z(-0.3)), // Slightly spread
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
                bevy::light::NotShadowCaster,
            ))
            .id();
        commands.entity(bird_entity).add_child(right_wing_entity);
        commands.entity(bird_entity).insert(BirdWings {
            left: left_wing_entity,
            right: right_wing_entity,
        });

        // Parent bird to zone entity so it inherits zone transform
        commands.entity(zone_entity).add_child(bird_entity);
    }
}

/// Creates the bird body mesh (torso, head, tail)
fn create_bird_body_mesh(meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    // Bird body oriented along Z axis:
    // - Nose/beak at +Z
    // - Tail at -Z
    // - Wings attach along X axis
    // - Back is +Y

    let vertices: Vec<[f32; 3]> = vec![
        // Beak tip (pointed nose)
        [0.0, 0.05, 0.25],
        // Head front (widens from beak)
        [-0.04, 0.06, 0.18],
        [0.04, 0.06, 0.18],
        [0.0, 0.1, 0.18],   // Top of head
        [0.0, -0.02, 0.18], // Bottom of beak junction
        // Head back / neck
        [-0.05, 0.07, 0.1],
        [0.05, 0.07, 0.1],
        [0.0, 0.11, 0.1],  // Top
        [0.0, -0.02, 0.1], // Bottom
        // Body front (widest part of chest)
        [-0.08, 0.06, 0.0],
        [0.08, 0.06, 0.0],
        [0.0, 0.1, 0.0],   // Top of back
        [0.0, -0.03, 0.0], // Belly
        // Body back (where tail starts)
        [-0.06, 0.05, -0.12],
        [0.06, 0.05, -0.12],
        [0.0, 0.08, -0.12],  // Top
        [0.0, -0.02, -0.12], // Bottom
        // Tail tip (fan shape)
        [-0.08, 0.03, -0.25],
        [0.0, 0.05, -0.28], // Center tail feather (longest)
        [0.08, 0.03, -0.25],
        [0.0, -0.01, -0.22], // Bottom tail
    ];

    // Triangle indices for the body
    let indices: Vec<u32> = vec![
        // Beak to head front
        0, 4, 1, // Bottom left
        0, 2, 4, // Bottom right
        0, 1, 3, // Top left
        0, 3, 2, // Top right
        // Head front to head back
        1, 4, 8, // Bottom left
        4, 2, 8, // Bottom right (fixed)
        1, 5, 3, // Left top
        3, 5, 6, // Top
        3, 6, 2, // Right top
        1, 8, 5, // Left side
        2, 6, 8, // Right side
        // Head back to body front
        5, 8, 12, // Left bottom
        8, 6, 12, // Right bottom (fixed)
        5, 9, 7, // Left top
        7, 9, 10, // Top
        7, 10, 6, // Right top
        6, 10, 11, // Right side
        5, 12, 9, // Left side
        // Body front to body back
        9, 12, 16, // Left bottom
        12, 11, 16, // Bottom right (fixed)
        9, 13, 10, // Left top
        10, 13, 14, // Top
        10, 14, 11, // Right top
        11, 14, 15, // Right side
        9, 16, 13, // Left side
        // Body back to tail
        13, 16, 19, // Left bottom
        16, 15, 19, // Bottom right
        13, 17, 14, // Left top
        14, 17, 18, // Top center
        14, 18, 15, // Right top
        15, 18, 19, // Right side
        13, 19, 17, // Left side to tail tip
        // Tail fan (fill in the tail shape)
        16, 19, 17, // Left tail
        17, 18, 16, // Top tail
        18, 15, 16, // Right tail
    ];

    // Calculate normals (simple approximation - pointing outward)
    let normals: Vec<[f32; 3]> = vec![
        [0.0, 0.3, 1.0],   // 0: Beak tip
        [-0.5, 0.3, 0.8],  // 1: Head front left
        [0.5, 0.3, 0.8],   // 2: Head front right
        [0.0, 0.8, 0.6],   // 3: Top of head front
        [0.0, -0.5, 0.9],  // 4: Bottom of beak
        [-0.6, 0.4, 0.6],  // 5: Head back left
        [0.6, 0.4, 0.6],   // 6: Head back right
        [0.0, 0.9, 0.4],   // 7: Top of head back
        [0.0, -0.3, 0.9],  // 8: Throat
        [-0.7, 0.3, 0.5],  // 9: Body front left
        [0.7, 0.3, 0.5],   // 10: Body front right
        [0.0, 0.9, 0.3],   // 11: Top of back
        [0.0, -0.8, 0.5],  // 12: Belly
        [-0.6, 0.3, -0.6], // 13: Body back left
        [0.6, 0.3, -0.6],  // 14: Body back right
        [0.0, 0.8, -0.5],  // 15: Top of rump
        [0.0, -0.5, -0.8], // 16: Bottom of rump
        [-0.5, 0.2, -0.8], // 17: Tail left
        [0.0, 0.5, -0.9],  // 18: Tail center
        [0.5, 0.2, -0.8],  // 19: Tail right
    ];

    // UV coordinates
    let uvs: Vec<[f32; 2]> = vec![
        [0.5, 0.95], // 0: Beak tip
        [0.3, 0.85], // 1
        [0.7, 0.85], // 2
        [0.5, 0.9],  // 3
        [0.5, 0.8],  // 4
        [0.25, 0.7], // 5
        [0.75, 0.7], // 6
        [0.5, 0.75], // 7
        [0.5, 0.65], // 8
        [0.2, 0.5],  // 9
        [0.8, 0.5],  // 10
        [0.5, 0.55], // 11
        [0.5, 0.45], // 12
        [0.25, 0.3], // 13
        [0.75, 0.3], // 14
        [0.5, 0.35], // 15
        [0.5, 0.25], // 16
        [0.2, 0.1],  // 17
        [0.5, 0.05], // 18
        [0.8, 0.1],  // 19
    ];

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    mesh.insert_indices(Indices::U32(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    meshes.add(mesh)
}

/// Creates a bird wing mesh (left by default, right when mirrored).
/// Left wing extends in -X direction from body center, the right wing is the
/// exact mirror (negated X positions/normals, flipped UV X and triangle winding).
fn create_bird_wing_mesh(meshes: &mut Assets<Mesh>, right_side: bool) -> Handle<Mesh> {
    // Wing pivots at body center (0,0,0) for flapping animation
    let vertices: Vec<[f32; 3]> = vec![
        // Wing root (attaches to body)
        [-0.05, 0.05, 0.05],  // Front top
        [-0.05, 0.02, 0.05],  // Front bottom
        [-0.05, 0.05, -0.05], // Back top
        [-0.05, 0.02, -0.05], // Back bottom
        // Wing mid
        [-0.2, 0.06, 0.03],  // Front top
        [-0.2, 0.0, 0.03],   // Front bottom
        [-0.2, 0.05, -0.05], // Back top
        [-0.2, 0.0, -0.05],  // Back bottom
        // Wing tip (pointed)
        [-0.35, 0.04, -0.02],  // Tip top
        [-0.35, -0.02, -0.02], // Tip bottom
    ];

    let indices: Vec<u32> = vec![
        // Top surface
        0, 2, 4, 4, 2, 6, 4, 6, 8, // Bottom surface
        1, 5, 3, 3, 5, 7, 5, 9, 7, // Front edge
        0, 4, 1, 1, 4, 5, // Back edge
        2, 3, 6, 6, 3, 7, // Tip
        6, 7, 8, 8, 7, 9,
    ];

    let normals: Vec<[f32; 3]> = vec![
        [0.2, 0.9, 0.1],     // 0
        [0.2, -0.9, 0.1],    // 1
        [0.2, 0.9, -0.1],    // 2
        [0.2, -0.9, -0.1],   // 3
        [0.1, 0.95, 0.05],   // 4
        [0.1, -0.95, 0.05],  // 5
        [0.1, 0.95, -0.05],  // 6
        [0.1, -0.95, -0.05], // 7
        [0.0, 0.95, 0.0],    // 8
        [0.0, -0.95, 0.0],   // 9
    ];

    let uvs: Vec<[f32; 2]> = vec![
        [0.9, 0.6],
        [0.9, 0.4],
        [0.7, 0.6],
        [0.7, 0.4],
        [0.5, 0.65],
        [0.5, 0.35],
        [0.3, 0.6],
        [0.3, 0.4],
        [0.1, 0.5],
        [0.1, 0.5],
    ];

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    if right_side {
        let vertices: Vec<[f32; 3]> = vertices
            .iter()
            .map(|v| [-v[0], v[1], v[2]])
            .collect();
        let normals: Vec<[f32; 3]> = normals
            .iter()
            .map(|n| [-n[0], n[1], n[2]])
            .collect();
        let uvs: Vec<[f32; 2]> = uvs.iter().map(|u| [1.0 - u[0], u[1]]).collect();
        let indices: Vec<u32> = indices
            .chunks(3)
            .flat_map(|t| [t[2], t[1], t[0]])
            .collect();

        mesh.insert_indices(Indices::U32(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    } else {
        mesh.insert_indices(Indices::U32(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    }

    meshes.add(mesh)
}

/// Gets a new random target position within roam bounds
fn get_new_target(center: Vec3, radius: f32, min_alt: f32, max_alt: f32) -> Vec3 {
    let mut rng = rand::thread_rng();
    let angle = rng.gen::<f32>() * std::f32::consts::TAU;
    let distance = rng.gen::<f32>() * radius;
    // Ordered, inclusive range: min == max is a valid setting, and gen_range
    // panics on an empty (exclusive) range
    let altitude = rng.gen_range(min_alt.min(max_alt)..=max_alt.max(min_alt));

    Vec3::new(
        center.x + angle.cos() * distance,
        center.y + altitude,
        center.z + angle.sin() * distance,
    )
}

/// Updates bird movement and animation
pub fn update_bird_movement_system(
    time: Res<Time>,
    settings: Res<BirdSettings>,
    mut bird_query: Query<(&mut Bird, &mut Transform, Option<&BirdWings>)>,
    mut wing_query: Query<
        &mut Transform,
        (Or<(With<BirdWingLeft>, With<BirdWingRight>)>, Without<Bird>),
    >,
) {
    if !settings.enabled {
        return;
    }

    let dt = time.delta_secs();

    for (mut bird, mut transform, wings) in bird_query.iter_mut() {
        // Move towards target
        let current_pos = transform.translation;
        let direction = bird.target_position - current_pos;
        let distance = direction.length();

        if distance < 2.0 {
            // Reached target, get new one
            bird.target_position = get_new_target(
                bird.roam_center,
                bird.roam_radius,
                settings.min_altitude,
                settings.max_altitude,
            );
        } else {
            // Move towards target
            let move_dir = direction.normalize();
            let move_amount = (bird.speed * dt).min(distance);
            transform.translation += move_dir * move_amount;

            // Face movement direction
            // Bird body faces +Z (forward), so we calculate rotation to align +Z with movement direction
            let target_rotation = Quat::from_rotation_y((-move_dir.x).atan2(move_dir.z));
            transform.rotation = transform.rotation.slerp(target_rotation, 2.0 * dt);
        }

        // Update wing flap animation
        bird.flap_phase += settings.flap_speed * dt;
        if bird.flap_phase > std::f32::consts::TAU {
            bird.flap_phase -= std::f32::consts::TAU;
        }

        // Update bob phase (kept on the component; the bob offset itself was
        // never applied to the transform, so it is not computed)
        bird.bob_phase += settings.bob_speed * dt;
        if bird.bob_phase > std::f32::consts::TAU {
            bird.bob_phase -= std::f32::consts::TAU;
        }

        // Calculate wing flap angle (sinusoidal motion)
        // Wings flap up and down: positive angle = up, negative = down
        let flap_angle = (bird.flap_phase).sin() * 0.6; // ±34 degrees flap

        // Apply to the wing children recorded at spawn
        if let Some(wings) = wings {
            if let Ok(mut wing_transform) = wing_query.get_mut(wings.left) {
                // Left wing rotates around Z axis (positive = up)
                wing_transform.rotation = Quat::from_rotation_z(0.3 + flap_angle);
            }
            if let Ok(mut wing_transform) = wing_query.get_mut(wings.right) {
                // Right wing rotates around Z axis (negative = up, so we negate)
                wing_transform.rotation = Quat::from_rotation_z(-0.3 - flap_angle);
            }
        }
    }
}
