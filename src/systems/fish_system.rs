//! Fish spawning and swimming behavior system
//!
//! This system handles:
//! - Spawning fish when water is created
//! - Fish variety: multiple fish types with distinct colors, sizes,
//!   body shapes, swim speeds, depth ranges and schooling behavior
//! - Fish swimming AI (picking targets, moving, turning)
//! - Fish animation (tail wobble)
//! - Keeping fish within water bounds

use bevy::asset::RenderAssetUsages;
use bevy::math::Vec3;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::prelude::*;
use bevy::render::alpha::AlphaMode;
use bevy::render::render_resource::Face;
use bevy_mesh::{Indices, Mesh, PrimitiveTopology};
use rand::Rng;

use crate::components::{Fish, FishSettings, FishWaterRef, WaterSpawnedEvent};

/// Identifies a distinct fish type with its own look and behavior profile.
///
/// The default spawn distribution lives in [`default_fish_type_distribution`];
/// types not listed there do not spawn unless added to the distribution.
/// All eight types (including [`FishType::Eel`]) are part of the default mix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FishType {
    /// Classic gold/orange fish (original look)
    Gold,
    /// Classic blue fish (original look)
    Blue,
    /// Classic metallic silver fish (original look)
    Silver,
    /// Small, fast, brightly colored (orange body, green fins)
    Tropical,
    /// Dark body with light fins ("striped" approximation)
    Striped,
    /// Slow round-bodied fish (flattened sphere profile)
    Puffer,
    /// Large, fast, dark steel-blue fish
    Tuna,
    /// Long, thin, olive-green fish that swims deep
    Eel,
}

/// Appearance and behavior parameters for one [`FishType`].
#[derive(Debug, Clone, Copy)]
struct FishTypeConfig {
    /// Body color (with slight transparency for the underwater look)
    base_color: Color,
    /// Fin color (tail + dorsal fin child mesh)
    fin_color: Color,
    /// Uniform size multiplier applied on top of the base 0.3 scale
    scale: f32,
    /// Non-uniform body proportions (e.g. slender eel)
    body_aspect: Vec3,
    /// Speed multiplier applied on top of the configured speed range
    speed_multiplier: f32,
    /// Depth bias in meters; positive = deeper, clamped into the valid range
    depth_bias: f32,
    /// Inclusive school size range `(min, max)`; members share a school_id
    school_range: (u32, u32),
    /// Use the round body mesh profile instead of the standard elongated one
    round_body: bool,
    /// Metallic value used by the StandardMaterial
    metallic: f32,
    /// Perceptual roughness used by the StandardMaterial
    roughness: f32,
}

impl FishType {
    /// Config lookup for a fish type.
    fn config(self) -> FishTypeConfig {
        match self {
            FishType::Gold => FishTypeConfig {
                base_color: Color::srgba(0.9, 0.6, 0.2, 0.9),
                fin_color: Color::srgba(0.75, 0.45, 0.15, 0.9),
                scale: 1.0,
                body_aspect: Vec3::ONE,
                speed_multiplier: 1.0,
                depth_bias: 0.0,
                school_range: (1, 3),
                round_body: false,
                metallic: 0.1,
                roughness: 0.3,
            },
            FishType::Blue => FishTypeConfig {
                base_color: Color::srgba(0.3, 0.5, 0.8, 0.9),
                fin_color: Color::srgba(0.25, 0.4, 0.65, 0.9),
                scale: 1.0,
                body_aspect: Vec3::ONE,
                speed_multiplier: 0.9,
                depth_bias: -0.2,
                school_range: (3, 6),
                round_body: false,
                metallic: 0.1,
                roughness: 0.3,
            },
            FishType::Silver => FishTypeConfig {
                base_color: Color::srgba(0.7, 0.75, 0.8, 0.9),
                fin_color: Color::srgba(0.6, 0.65, 0.7, 0.9),
                scale: 0.9,
                body_aspect: Vec3::ONE,
                speed_multiplier: 1.1,
                depth_bias: -0.5,
                school_range: (3, 6),
                round_body: false,
                metallic: 0.3,
                roughness: 0.2,
            },
            FishType::Tropical => FishTypeConfig {
                base_color: Color::srgba(1.0, 0.45, 0.25, 0.95),
                fin_color: Color::srgba(0.25, 0.8, 0.45, 0.95),
                scale: 0.7,
                body_aspect: Vec3::ONE,
                speed_multiplier: 1.4,
                depth_bias: 0.0,
                school_range: (4, 8),
                round_body: false,
                metallic: 0.0,
                roughness: 0.4,
            },
            FishType::Striped => FishTypeConfig {
                base_color: Color::srgba(0.16, 0.18, 0.26, 0.95),
                fin_color: Color::srgba(0.85, 0.88, 0.95, 0.95),
                scale: 0.85,
                body_aspect: Vec3::ONE,
                speed_multiplier: 1.0,
                depth_bias: 0.3,
                school_range: (1, 3),
                round_body: false,
                metallic: 0.0,
                roughness: 0.4,
            },
            FishType::Puffer => FishTypeConfig {
                base_color: Color::srgba(0.8, 0.7, 0.35, 0.95),
                fin_color: Color::srgba(0.9, 0.85, 0.6, 0.95),
                scale: 1.6,
                body_aspect: Vec3::ONE,
                speed_multiplier: 0.55,
                depth_bias: 0.5,
                school_range: (1, 2),
                round_body: true,
                metallic: 0.0,
                roughness: 0.5,
            },
            FishType::Tuna => FishTypeConfig {
                base_color: Color::srgba(0.25, 0.3, 0.42, 0.95),
                fin_color: Color::srgba(0.4, 0.45, 0.55, 0.95),
                scale: 1.8,
                body_aspect: Vec3::ONE,
                speed_multiplier: 1.5,
                depth_bias: 1.0,
                school_range: (1, 3),
                round_body: false,
                metallic: 0.4,
                roughness: 0.3,
            },
            FishType::Eel => FishTypeConfig {
                base_color: Color::srgba(0.35, 0.42, 0.28, 0.9),
                fin_color: Color::srgba(0.55, 0.6, 0.4, 0.9),
                scale: 0.9,
                body_aspect: Vec3::new(1.5, 0.4, 0.4),
                speed_multiplier: 0.8,
                depth_bias: 0.7,
                school_range: (1, 2),
                round_body: false,
                metallic: 0.0,
                roughness: 0.4,
            },
        }
    }
}

/// Default weighted type distribution used for every water plane.
///
/// Weights are percentages (sum = 100). The weighted pick happens once per
/// school, so all members of a school share the same type.
fn default_fish_type_distribution() -> Vec<(FishType, f32)> {
    vec![
        (FishType::Gold, 18.0),
        (FishType::Blue, 18.0),
        (FishType::Silver, 14.0),
        (FishType::Tropical, 15.0),
        (FishType::Striped, 15.0),
        (FishType::Tuna, 5.0),
        (FishType::Puffer, 10.0),
        (FishType::Eel, 5.0),
    ]
}

/// System to spawn fish when water is created
pub fn spawn_fish_on_water_system(
    mut events: MessageReader<WaterSpawnedEvent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<FishSettings>,
    zone_query: Query<Entity, With<crate::components::Zone>>,
) {
    let mut event_count = 0;
    for event in events.read() {
        event_count += 1;
        //log::info!("[FISH DEBUG] Received WaterSpawnedEvent #{}: water_entity={:?}, zone_entity={:?}, center={:?}, extents={:?}",
        //    event_count, event.water_entity, event.zone_entity, event.water_center, event.water_half_extents);

        // Check if zone_entity still exists before spawning fish
        let zone_exists = zone_query.get(event.zone_entity).is_ok();
        if !zone_exists {
            log::warn!(
                "[FISH] Zone entity {:?} no longer exists, skipping fish spawn",
                event.zone_entity
            );
            continue;
        }

        spawn_fish_in_water(
            event.water_entity,
            event.zone_entity,
            event.water_center,
            event.water_half_extents,
            &mut commands,
            &mut meshes,
            &mut materials,
            &settings,
        );
    }

    if event_count > 0 {
        //log::info!("[FISH DEBUG] Processed {} WaterSpawnedEvent(s) this frame", event_count);
    }
}

/// Spawn fish in a water area
#[allow(clippy::too_many_arguments)]
fn spawn_fish_in_water(
    water_entity: Entity,
    zone_entity: Entity,
    water_center: Vec3,
    water_half_extents: Vec2,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &Res<FishSettings>,
) {
    let mut rng = rand::thread_rng();

    // Guard against degenerate (zero-size) water planes: an empty gen_range
    // would panic and disable the whole spawn system
    let ext_x = water_half_extents.x.max(0.001);
    let ext_z = water_half_extents.y.max(0.001);

    // Fish count scales with the water plane area (in 1000 m^2 steps) so a
    // tiny pond gets a few fish instead of the same crowd as a huge lake.
    // A non-positive density disables fish entirely.
    if settings.fish_per_1000_sqm <= 0.0 {
        return;
    }
    let water_area_sqm = (2.0 * ext_x) * (2.0 * ext_z);
    let fish_count = ((water_area_sqm / 1000.0) * settings.fish_per_1000_sqm) as usize;
    // Normalize the clamp range so misconfigured settings cannot panic
    let min_fish = settings.min_fish_per_water.min(settings.max_fish_per_water);
    let max_fish = settings.min_fish_per_water.max(settings.max_fish_per_water);
    let fish_count = fish_count.clamp(min_fish, max_fish);

    log::info!(
        "[FISH] Spawning {} fish in water at {:?} with extents {:?} (area {:.1} m^2)",
        fish_count,
        water_center,
        water_half_extents,
        water_area_sqm
    );

    // Build one mesh per body profile (shared by all fish in this water plane):
    // - standard elongated body
    // - round body (puffer-style)
    // - fins (tail + dorsal), shared by both body profiles
    let body_mesh_standard = create_fish_body_mesh(meshes, false);
    let body_mesh_round = create_fish_body_mesh(meshes, true);
    let fin_mesh = create_fish_fin_mesh(meshes);

    // Weighted type distribution for this water plane
    let distribution = default_fish_type_distribution();
    if distribution.is_empty() {
        log::warn!("[FISH] Fish type distribution is empty, skipping spawn");
        return;
    }
    let total_weight: f32 = distribution.iter().map(|(_, weight)| weight).sum();

    // Build one body + one fin material per fish type (built once, reused by
    // every fish of that type in this water plane)
    let mut type_materials: Vec<(FishType, Handle<StandardMaterial>, Handle<StandardMaterial>)> =
        Vec::with_capacity(distribution.len());
    for (fish_type, _) in &distribution {
        let config = fish_type.config();
        let body_material = materials.add(StandardMaterial {
            base_color: config.base_color,
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: config.roughness,
            metallic: config.metallic,
            cull_mode: None,
            ..default()
        });
        let fin_material = materials.add(StandardMaterial {
            base_color: config.fin_color,
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: config.roughness,
            metallic: config.metallic,
            cull_mode: None,
            ..default()
        });
        type_materials.push((*fish_type, body_material, fin_material));
    }

    // Clamp min/max to prevent crash if settings are invalid
    let min_depth = settings.min_depth.min(settings.max_depth);
    let max_depth = settings.max_depth.max(settings.min_depth);
    let min_speed = settings.min_speed.min(settings.max_speed);
    let max_speed = settings.max_speed.max(settings.min_speed);

    // Fish are spawned in schools: each school picks a type, a depth and a
    // position, then all members spawn near each other with the same school_id
    let mut school_id: u32 = 1;
    let mut i = 0usize;
    while i < fish_count {
        // Weighted random type pick for this school
        let mut picked_index = 0usize;
        if total_weight > 0.0 {
            let mut roll = rng.gen_range(0.0..total_weight);
            for (index, (_, weight)) in distribution.iter().enumerate() {
                if roll < *weight {
                    picked_index = index;
                    break;
                }
                roll -= *weight;
            }
        }
        let fish_type = distribution[picked_index].0;
        let config = fish_type.config();

        // School size for this type, capped by the remaining fish count
        let school_min = config.school_range.0.max(1);
        let school_max = config.school_range.1.max(school_min);
        let remaining = fish_count - i;
        let school_size = rng.gen_range(school_min..=school_max).min(remaining as u32) as usize;

        // School depth with per-type bias, clamped into the valid range
        let school_depth =
            (rng.gen_range(min_depth..max_depth) + config.depth_bias).clamp(min_depth, max_depth);

        // School position: uniform random point anywhere in the water area
        // (no fixed grid, so schools never line up in rows); uses the guarded
        // extents so zero-size planes cannot produce an empty gen_range
        let school_center = Vec3::new(
            rng.gen_range(-ext_x..ext_x) * settings.boundary_margin,
            water_center.y - school_depth,
            rng.gen_range(-ext_z..ext_z) * settings.boundary_margin,
        );

        // Member spread grows with school size so larger schools fan out
        // instead of piling up into a tight line; capped so schools never
        // exceed the water plane they live in
        let member_spread = (0.8 * (school_size as f32).sqrt())
            .min(water_half_extents.x.max(0.1))
            .min(water_half_extents.y.max(0.1));

        let base_scale = 0.3 * config.scale;
        let body_mesh = if config.round_body {
            &body_mesh_round
        } else {
            &body_mesh_standard
        };
        let fin_offset = if config.round_body { 0.25 } else { 0.0 };
        let (body_material, fin_material) = {
            let (_, body_mat, fin_mat) = &type_materials[picked_index];
            (body_mat.clone(), fin_mat.clone())
        };

        for _member in 0..school_size {
            // Uniform random position inside the school's spread circle
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let radius = member_spread * rng.gen_range(0.0f32..1.0).sqrt();
            let member_offset_x = angle.cos() * radius;
            let member_offset_z = angle.sin() * radius;

            // Slight per-member depth variation keeps the school from being a flat plane
            let member_depth =
                (school_depth + rng.gen_range(-0.3..0.3)).clamp(min_depth, max_depth);

            let position = Vec3::new(
                water_center.x + school_center.x + member_offset_x,
                water_center.y - member_depth, // Below water surface
                water_center.z + school_center.z + member_offset_z,
            );

            // Random speed scaled by the type multiplier
            let speed = rng.gen_range(min_speed..max_speed) * config.speed_multiplier;

            // Each member wanders on its own: individual target so the school
            // disperses naturally instead of converging on a single point
            let target = pick_new_target(
                water_center,
                water_half_extents,
                settings.boundary_margin,
                member_depth,
            );

            // Random rotation
            let rotation = Quat::from_rotation_y(rng.gen_range(0.0..std::f32::consts::TAU));

            // Spawn fish entity
            let fish_entity = commands
                .spawn((
                    Fish {
                        speed,
                        turn_speed: rng.gen_range(2.0..4.0),
                        target_position: target,
                        depth: member_depth,
                        school_id,
                        water_center,
                        water_half_extents,
                        wobble_time: rng.gen_range(0.0..std::f32::consts::TAU), // Random phase offset
                    },
                    FishWaterRef { water_entity },
                    Transform::from_translation(position)
                        .with_rotation(rotation)
                        .with_scale(Vec3::splat(base_scale) * config.body_aspect),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    // Fish are underwater so they never appear in water
                    // reflections; layer 1 excludes them from the reflection
                    // camera (layer 0) while the main camera still renders them.
                    bevy::camera::visibility::RenderLayers::layer(1),
                ))
                .id();

            // Spawn body mesh as child entity
            let body_entity = commands
                .spawn((
                    Mesh3d(body_mesh.clone()),
                    MeshMaterial3d(body_material.clone()),
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    bevy::camera::visibility::RenderLayers::layer(1),
                ))
                .id();

            // Spawn fin mesh as a second child entity (tail + dorsal fin)
            let fin_entity = commands
                .spawn((
                    Mesh3d(fin_mesh.clone()),
                    MeshMaterial3d(fin_material.clone()),
                    Transform::from_translation(Vec3::new(fin_offset, 0.0, 0.0)),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    bevy::camera::visibility::RenderLayers::layer(1),
                ))
                .id();

            commands
                .entity(fish_entity)
                .add_child(body_entity)
                .add_child(fin_entity);

            // Parent fish to zone entity so it inherits zone transform
            // Only add as child if zone_entity is valid (not PLACEHOLDER)
            if zone_entity != Entity::PLACEHOLDER {
                commands.entity(zone_entity).add_child(fish_entity);
            }

            // log::info!(
            //     "[FISH DEBUG] Spawned fish {} at position {:?} (water_center={:?}, depth={}), parented to zone {:?}",
            //     i, position, water_center, depth, zone_entity
            // );
        }

        school_id += 1;
        i += school_size;
    }

    log::info!("[FISH] Spawned {} fish total", fish_count);
}

/// Create a fish body mesh (elongated standard shape or round puffer shape)
///
/// Fish body is elongated along X axis, with nose at +X and tail at -X.
/// The tail fin and dorsal fin are separate meshes (see [`create_fish_fin_mesh`]).
fn create_fish_body_mesh(meshes: &mut ResMut<Assets<Mesh>>, round: bool) -> Handle<Mesh> {
    // Standard body vertices (nose, three 4-vertex rings, tail base point)
    let standard_vertices: Vec<[f32; 3]> = vec![
        // Nose (point)
        [1.0, 0.0, 0.0],
        // Body front (wider)
        [0.5, 0.0, 0.3],
        [0.5, 0.2, 0.0],
        [0.5, 0.0, -0.3],
        [0.5, -0.15, 0.0],
        // Body middle (widest)
        [0.0, 0.0, 0.4],
        [0.0, 0.25, 0.0],
        [0.0, 0.0, -0.4],
        [0.0, -0.2, 0.0],
        // Body back (narrower)
        [-0.5, 0.0, 0.25],
        [-0.5, 0.15, 0.0],
        [-0.5, 0.0, -0.25],
        [-0.5, -0.1, 0.0],
        // Tail base
        [-0.8, 0.0, 0.0],
    ];

    // Round body vertices (flattened ball: nose, three rings, tail point)
    let round_vertices: Vec<[f32; 3]> = vec![
        // Nose (point)
        [0.85, 0.0, 0.0],
        // Ring A (front of the ball)
        [0.3, 0.0, 0.4],
        [0.3, 0.28, 0.0],
        [0.3, 0.0, -0.4],
        [0.3, -0.22, 0.0],
        // Ring B (equator, widest)
        [-0.15, 0.0, 0.5],
        [-0.15, 0.35, 0.0],
        [-0.15, 0.0, -0.5],
        [-0.15, -0.28, 0.0],
        // Ring C (narrowing)
        [-0.55, 0.0, 0.35],
        [-0.55, 0.24, 0.0],
        [-0.55, 0.0, -0.35],
        [-0.55, -0.18, 0.0],
        // Tail point
        [-0.9, 0.0, 0.0],
    ];

    // Ring-connection triangles: nose to ring A, ring to ring, ring to tail.
    // The same topology works for both vertex sets.
    let indices: Vec<u32> = vec![
        // Nose to first ring
        0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 1,
        // Ring 1 to ring 2
        1, 5, 2, 2, 5, 6, 2, 6, 3, 3, 6, 7, 3, 7, 4, 4, 7, 8, 4, 8, 1, 1, 8, 5,
        // Ring 2 to ring 3
        5, 9, 6, 6, 9, 10, 6, 10, 7, 7, 10, 11, 7, 11, 8, 8, 11, 12, 8, 12, 5, 5, 12, 9,
        // Ring 3 to tail point
        9, 13, 10, 10, 13, 11, 11, 13, 12, 12, 13, 9,
    ];

    let vertices = if round {
        round_vertices
    } else {
        standard_vertices
    };

    // Calculate normals (simple approximation)
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(vertices.len());
    for _ in &vertices {
        normals.push([0.0, 1.0, 0.0]); // Simple upward normals
    }

    // UV coordinates (simple mapping)
    let uvs: Vec<[f32; 2]> = vertices
        .iter()
        .map(|v| {
            let u = (v[0] + 1.2) / 2.2; // Map -1.2..1.0 to 0..1
            let v = (v[2] + 0.4) / 0.8; // Map -0.4..0.4 to 0..1
            [u.clamp(0.0, 1.0), v.clamp(0.0, 1.0)]
        })
        .collect();

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

/// Create a fish fin mesh (tail fin + dorsal fin)
///
/// Tail fin spans from the tail base point (at -0.8) backwards, dorsal fin
/// sits on top of the body middle. This mesh is shared by all fish types;
/// the round-body fish offsets it slightly backwards via its Transform.
fn create_fish_fin_mesh(meshes: &mut ResMut<Assets<Mesh>>) -> Handle<Mesh> {
    let vertices: Vec<[f32; 3]> = vec![
        // Tail fin base (matches body tail base point)
        [-0.8, 0.0, 0.0],
        // Tail fin top
        [-1.2, 0.3, 0.0],
        // Tail fin bottom
        [-1.2, -0.2, 0.0],
        // Dorsal fin front base (on top of body middle)
        [0.0, 0.25, 0.0],
        // Dorsal fin tip
        [0.0, 0.4, 0.0],
        // Dorsal fin back
        [-0.3, 0.35, 0.0],
        // Dorsal fin rear base
        [-0.5, 0.15, 0.0],
    ];

    let indices: Vec<u32> = vec![
        // Tail fin
        0, 1, 2,
        // Dorsal fin
        3, 4, 5, 3, 5, 6,
    ];

    // Calculate normals (simple approximation)
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(vertices.len());
    for _ in &vertices {
        normals.push([0.0, 1.0, 0.0]); // Simple upward normals
    }

    // UV coordinates (simple mapping)
    let uvs: Vec<[f32; 2]> = vertices
        .iter()
        .map(|v| {
            let u = (v[0] + 1.2) / 2.2; // Map -1.2..1.0 to 0..1
            let v = (v[2] + 0.4) / 0.8; // Map -0.4..0.4 to 0..1
            [u.clamp(0.0, 1.0), v.clamp(0.0, 1.0)]
        })
        .collect();

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

/// Pick a new random target position within water bounds
fn pick_new_target(
    water_center: Vec3,
    water_half_extents: Vec2,
    boundary_margin: f32,
    depth: f32,
) -> Vec3 {
    let mut rng = rand::thread_rng();

    // Guard against degenerate (zero-size) water planes: an empty range
    // would panic inside gen_range and disable the whole movement system
    let ext_x = water_half_extents.x.max(0.001);
    let ext_z = water_half_extents.y.max(0.001);

    Vec3::new(
        water_center.x
            + rng.gen_range(-ext_x..ext_x) * boundary_margin,
        water_center.y - depth, // Stay at same depth
        water_center.z
            + rng.gen_range(-ext_z..ext_z) * boundary_margin,
    )
}

/// Reusable per-frame buffers for the fish separation pass, so the system
/// does not allocate fresh Vecs every frame.
#[derive(Default)]
pub struct FishSeparationBuffers {
    positions: Vec<Vec3>,
    order: Vec<usize>,
    pushes: Vec<Vec3>,
}

/// System to update fish movement and swimming behavior
///
/// Fish farther than [`FishSettings::simulation_distance`] from the camera
/// are skipped entirely, so distant water planes cost nothing per frame.
pub fn update_fish_movement_system(
    time: Res<Time>,
    settings: Res<FishSettings>,
    mut query: Query<(&mut Transform, &GlobalTransform, &mut Fish)>,
    camera_query: Query<
        &GlobalTransform,
        (
            With<Camera3d>,
            Without<crate::render::WaterReflectionCamera>,
        ),
    >,
    mut buffers: Local<FishSeparationBuffers>,
) {
    let mut rng = rand::thread_rng();
    let delta = time.delta_secs();

    // Dereference the Local once so field borrows split (borrows through the
    // DerefMut of Local are not split by the borrow checker)
    let buffers = &mut *buffers;

    // Camera position for distance culling (no culling if there is no camera)
    let camera_pos = camera_query.iter().next().map(|gt| gt.translation());
    let cull_enabled = settings.simulation_distance > 0.0 && camera_pos.is_some();
    let cull_dist_sq = settings.simulation_distance * settings.simulation_distance;

    // Snapshot current world positions (one frame old is fine) so we can push
    // overlapping fish apart without borrowing the query twice. World space is
    // used so fish from different (possibly overlapping) water planes also
    // separate, and so the camera distance check has comparable coordinates.
    // Only fish that will actually be simulated (inside the cull radius) take
    // part in the separation pass, so distant schools cost nothing here.
    buffers.positions.clear();
    for (_, global_transform, _) in query.iter() {
        let pos = global_transform.translation();
        if cull_enabled && pos.distance_squared(camera_pos.unwrap()) > cull_dist_sq {
            continue;
        }
        buffers.positions.push(pos);
    }

    // Separation: push overlapping fish closer than the separation radius
    // apart so spawn piles and schools spread out instead of converging.
    // Fish from different water planes are included on purpose, so stacked
    // fish where planes overlap also get pushed apart. Indices are sorted by
    // X so the inner scan early-outs as soon as the X delta exceeds the
    // separation radius, keeping this cheap even with thousands of fish
    // across many water planes.
    const SEPARATION_RADIUS: f32 = 0.9;
    buffers.order.clear();
    buffers.order.extend(0..buffers.positions.len());
    buffers
        .order
        .sort_by(|&a, &b| buffers.positions[a].x.total_cmp(&buffers.positions[b].x));

    buffers.pushes.clear();
    buffers.pushes.resize(buffers.positions.len(), Vec3::ZERO);
    for k in 0..buffers.order.len() {
        let i = buffers.order[k];
        let pos_i = buffers.positions[i];
        for l in (k + 1)..buffers.order.len() {
            let j = buffers.order[l];
            let pos_j = buffers.positions[j];
            if pos_j.x - pos_i.x > SEPARATION_RADIUS {
                break;
            }
            let dx = pos_i.x - pos_j.x;
            let dy = pos_i.y - pos_j.y;
            let dz = pos_i.z - pos_j.z;
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if dist_sq >= SEPARATION_RADIUS * SEPARATION_RADIUS || dist_sq <= 1e-8 {
                continue;
            }
            let dist = dist_sq.sqrt();
            let strength = (SEPARATION_RADIUS - dist) / SEPARATION_RADIUS;
            let push = Vec3::new(dx, dy, dz) / dist * strength;
            buffers.pushes[i] += push;
            buffers.pushes[j] -= push;
        }
    }

    let mut near_idx = 0usize;
    for (mut transform, global_transform, mut fish) in query.iter_mut() {
        // Skip fish too far from the camera to be noticed (world positions are
        // at most one frame stale, which is fine for a culling decision)
        if cull_enabled {
            let fish_pos = global_transform.translation();
            if fish_pos.distance_squared(camera_pos.unwrap()) > cull_dist_sq {
                continue;
            }
        }

        // Index into the separation buffers: collected fish are exactly the
        // non-culled ones, in query iteration order, so this stays in sync
        // with `buffers.pushes`.
        let push = buffers.pushes[near_idx];
        near_idx += 1;

        // Update wobble time for swimming animation - each fish has unique wobble speed
        fish.wobble_time += delta * fish.speed * (2.5 + rng.gen_range(0.0..1.0));

        // Calculate direction to target
        let direction = fish.target_position - transform.translation;
        let distance = direction.length();

        // Check if we reached the target (or the direction is corrupted)
        if !distance.is_finite() || distance < settings.target_reach_distance {
            // Pick a new random target with some randomness in depth
            let new_depth = fish.depth + rng.gen_range(-0.3..0.3);
            fish.depth = new_depth.clamp(settings.min_depth, settings.max_depth);
            fish.target_position = pick_new_target(
                fish.water_center,
                fish.water_half_extents,
                settings.boundary_margin,
                fish.depth,
            );
            continue;
        }

        // Normalize direction
        let direction_normalized = direction / distance;

        // Calculate target rotation (face direction of movement)
        // Fish mesh has nose at +X, so we rotate to make +X face the target direction
        // atan2(z, x) gives the angle from +X axis to the direction
        // Add slight random variation to prevent perfect alignment
        let rotation_noise = rng.gen_range(-0.05..0.05);
        let target_rotation = Quat::from_rotation_y(
            direction_normalized.z.atan2(direction_normalized.x) + rotation_noise,
        );

        // Smoothly rotate towards target with slight speed variation
        let turn_speed_variation = fish.turn_speed * rng.gen_range(0.9..1.1);
        transform.rotation = transform
            .rotation
            .slerp(target_rotation, turn_speed_variation * delta);

        // Move forward in facing direction with slight speed variation
        // Fish mesh faces +X, so use right() instead of forward()
        let speed_variation = fish.speed * rng.gen_range(0.95..1.05);
        let forward = transform.right();
        transform.translation += forward * speed_variation * delta;

        // Add swimming wobble (side-to-side motion) with unique amplitude per fish.
        // Scaled by delta: a per-second lateral speed, so fish glide forward
        // instead of weaving sideways as hard as they swim.
        let wobble_amplitude = 0.015 + (fish.wobble_time.sin() * 0.005).abs(); // Varies between 0.01 and 0.02
        let wobble = (fish.wobble_time.sin() * wobble_amplitude * fish.speed);
        let wobble_z = (fish.wobble_time.cos() * wobble_amplitude * 0.5 * fish.speed); // Secondary wobble
        transform.translation.x += transform.left().x * wobble * delta;
        transform.translation.z += transform.left().z * wobble * delta;
        // Add slight vertical wobble for more natural movement
        transform.translation.y += wobble_z * 0.3 * delta;

        // Push apart from neighbors (frame-rate independent). Pushes are
        // world-space directions applied to the local translation, which is
        // valid because zone parents are pure translations (no rotation/scale).
        transform.translation += push.clamp_length_max(1.0) * 2.0 * delta;

        // Keep fish within water bounds (clamp position)
        let min_x = fish.water_center.x - fish.water_half_extents.x * settings.boundary_margin;
        let max_x = fish.water_center.x + fish.water_half_extents.x * settings.boundary_margin;
        let min_z = fish.water_center.z - fish.water_half_extents.y * settings.boundary_margin;
        let max_z = fish.water_center.z + fish.water_half_extents.y * settings.boundary_margin;

        transform.translation.x = transform.translation.x.clamp(min_x, max_x);
        transform.translation.z = transform.translation.z.clamp(min_z, max_z);
        // Maintain depth with slight variation
        let target_y = fish.water_center.y - fish.depth;
        transform.translation.y = transform
            .translation
            .y
            .clamp(target_y - 0.1, target_y + 0.1);

        // If fish hit boundary, pick new target away from boundary
        if transform.translation.x <= min_x + 0.5
            || transform.translation.x >= max_x - 0.5
            || transform.translation.z <= min_z + 0.5
            || transform.translation.z >= max_z - 0.5
        {
            fish.target_position = pick_new_target(
                fish.water_center,
                fish.water_half_extents,
                settings.boundary_margin,
                fish.depth,
            );
        }
    }
}

/// Plugin for fish systems
pub struct FishPlugin;

impl Plugin for FishPlugin {
    fn build(&self, app: &mut App) {
        //log::info!("[FISH DEBUG] FishPlugin::build() called - registering fish systems");
        app
            // Register types for reflection
            .register_type::<Fish>()
            .register_type::<FishSettings>()
            // Add resources
            .init_resource::<FishSettings>()
            // Add messages
            .add_message::<WaterSpawnedEvent>()
            // Add systems
            .add_systems(
                Update,
                (spawn_fish_on_water_system, update_fish_movement_system).chain(),
            );
    }
}
