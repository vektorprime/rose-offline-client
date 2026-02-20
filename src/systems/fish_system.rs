//! Fish spawning and swimming behavior system
//! 
//! This system handles:
//! - Spawning fish when water is created
//! - Fish swimming AI (picking targets, moving, turning)
//! - Fish animation (tail wobble)
//! - Keeping fish within water bounds

use bevy::prelude::*;
use bevy::math::Vec3;
use bevy::render::mesh::{Mesh, Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::render::alpha::AlphaMode;
use rand::Rng;

use crate::components::{Fish, FishSettings, FishWaterRef, WaterSpawnedEvent};

/// System to spawn fish when water is created
pub fn spawn_fish_on_water_system(
    mut events: EventReader<WaterSpawnedEvent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<FishSettings>,
) {
    let mut event_count = 0;
    for event in events.read() {
        event_count += 1;
        log::info!("[FISH DEBUG] Received WaterSpawnedEvent #{}: water_entity={:?}, zone_entity={:?}, center={:?}, extents={:?}",
            event_count, event.water_entity, event.zone_entity, event.water_center, event.water_half_extents);
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
        log::info!("[FISH DEBUG] Processed {} WaterSpawnedEvent(s) this frame", event_count);
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
    
    log::info!(
        "[FISH] Spawning {} fish in water at {:?} with extents {:?}",
        settings.fish_count_per_water,
        water_center,
        water_half_extents
    );
    
    // Create fish mesh (simple elongated shape)
    let fish_mesh = create_fish_mesh(meshes);
    
    // Create fish material with slight transparency for underwater effect
    // Use a gold/orange color for visibility
    let fish_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.9, 0.6, 0.2, 0.9), // Gold/orange with slight transparency
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.3,
        metallic: 0.1,
        cull_mode: None, // Double-sided for better visibility
        ..default()
    });
    
    // Create a second material for variation (blue-ish fish)
    let fish_material_blue = materials.add(StandardMaterial {
        base_color: Color::srgba(0.3, 0.5, 0.8, 0.9), // Blue with slight transparency
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.3,
        metallic: 0.1,
        cull_mode: None,
        ..default()
    });
    
    // Create a third material for variation (silver fish)
    let fish_material_silver = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.75, 0.8, 0.9), // Silver with slight transparency
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.2,
        metallic: 0.3,
        cull_mode: None,
        ..default()
    });
    
    let materials_vec = vec![fish_material, fish_material_blue, fish_material_silver];
    
    for i in 0..settings.fish_count_per_water {
        // Random position within water bounds
        let x_offset = rng.gen_range(-water_half_extents.x..water_half_extents.x) * settings.boundary_margin;
        let z_offset = rng.gen_range(-water_half_extents.y..water_half_extents.y) * settings.boundary_margin;
        let depth = rng.gen_range(settings.min_depth..settings.max_depth);
        
        let position = Vec3::new(
            water_center.x + x_offset,
            water_center.y - depth, // Below water surface
            water_center.z + z_offset,
        );
        
        // Random speed
        let speed = rng.gen_range(settings.min_speed..settings.max_speed);
        
        // Random initial target
        let target = pick_new_target(water_center, water_half_extents, settings.boundary_margin, depth);
        
        // Random rotation
        let rotation = Quat::from_rotation_y(rng.gen_range(0.0..std::f32::consts::TAU));
        
        // Pick a random material for variety
        let material_idx = rng.gen_range(0..materials_vec.len());
        let material = materials_vec[material_idx].clone();
        
        // Spawn fish entity
        let fish_entity = commands.spawn((
            Fish {
                speed,
                turn_speed: rng.gen_range(2.0..4.0),
                target_position: target,
                depth,
                school_id: (i % 3) as u32, // Simple schooling groups
                water_center,
                water_half_extents,
                wobble_time: rng.gen_range(0.0..std::f32::consts::TAU), // Random phase offset
            },
            FishWaterRef { water_entity },
            Transform::from_translation(position)
                .with_rotation(rotation)
                .with_scale(Vec3::splat(0.3)), // Scale down the fish
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        )).id();
        
        // Spawn fish mesh as child entity
        let mesh_entity = commands.spawn((
            Mesh3d(fish_mesh.clone()),
            MeshMaterial3d(material),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        )).id();
        
        commands.entity(fish_entity).add_child(mesh_entity);
        
        // Parent fish to zone entity so it inherits zone transform
        commands.entity(zone_entity).add_child(fish_entity);
        
        log::info!(
            "[FISH DEBUG] Spawned fish {} at position {:?} (water_center={:?}, depth={}), parented to zone {:?}",
            i, position, water_center, depth, zone_entity
        );
    }
    
    log::info!(
        "[FISH] Spawned {} fish total",
        settings.fish_count_per_water
    );
}

/// Create a simple fish mesh (elongated body with tail)
fn create_fish_mesh(meshes: &mut ResMut<Assets<Mesh>>) -> Handle<Mesh> {
    // Create a simple fish shape using vertices
    // Fish body is elongated along X axis, with nose at +X and tail at -X
    
    let vertices: Vec<[f32; 3]> = vec![
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
        
        // Tail fin (top and bottom)
        [-1.2, 0.3, 0.0],
        [-1.2, -0.2, 0.0],
        
        // Dorsal fin (top)
        [0.0, 0.4, 0.0],
        [-0.3, 0.35, 0.0],
    ];
    
    // Define triangles using indices
    let indices: Vec<u32> = vec![
        // Nose to body front
        0, 1, 2,
        0, 2, 3,
        0, 3, 4,
        0, 4, 1,
        
        // Body front to body middle
        1, 5, 2,
        2, 5, 6,
        2, 6, 3,
        3, 6, 7,
        3, 7, 4,
        4, 7, 8,
        4, 8, 1,
        1, 8, 5,
        
        // Body middle to body back
        5, 9, 6,
        6, 9, 10,
        6, 10, 7,
        7, 10, 11,
        7, 11, 8,
        8, 11, 12,
        8, 12, 5,
        5, 12, 9,
        
        // Body back to tail
        9, 13, 10,
        10, 13, 11,
        11, 13, 12,
        12, 13, 9,
        
        // Tail fin
        13, 14, 15,
        
        // Dorsal fin
        6, 16, 17,
        6, 17, 10,
    ];
    
    // Calculate normals (simple approximation)
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(vertices.len());
    for _ in &vertices {
        normals.push([0.0, 1.0, 0.0]); // Simple upward normals
    }
    
    // UV coordinates (simple mapping)
    let uvs: Vec<[f32; 2]> = vertices.iter().map(|v| {
        let u = (v[0] + 1.2) / 2.2; // Map -1.2..1.0 to 0..1
        let v = (v[2] + 0.4) / 0.8; // Map -0.4..0.4 to 0..1
        [u.clamp(0.0, 1.0), v.clamp(0.0, 1.0)]
    }).collect();
    
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
    
    Vec3::new(
        water_center.x + rng.gen_range(-water_half_extents.x..water_half_extents.x) * boundary_margin,
        water_center.y - depth, // Stay at same depth
        water_center.z + rng.gen_range(-water_half_extents.y..water_half_extents.y) * boundary_margin,
    )
}

/// System to update fish movement and swimming behavior
pub fn update_fish_movement_system(
    time: Res<Time>,
    settings: Res<FishSettings>,
    mut query: Query<(&mut Transform, &mut Fish)>,
) {
    for (mut transform, mut fish) in query.iter_mut() {
        let delta = time.delta_secs();
        
        // Update wobble time for swimming animation
        fish.wobble_time += delta * fish.speed * 3.0;
        
        // Calculate direction to target
        let direction = fish.target_position - transform.translation;
        let distance = direction.length();
        
        // Check if we reached the target
        if distance < settings.target_reach_distance {
            // Pick a new random target
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
        let target_rotation = Quat::from_rotation_y(
            direction_normalized.z.atan2(direction_normalized.x) - std::f32::consts::FRAC_PI_2
        );
        
        // Smoothly rotate towards target
        transform.rotation = transform.rotation.slerp(
            target_rotation,
            fish.turn_speed * delta
        );
        
        // Move forward in facing direction
        let forward = transform.forward();
        transform.translation += forward * fish.speed * delta;
        
        // Add swimming wobble (side-to-side motion)
        let wobble = (fish.wobble_time.sin() * 0.02 * fish.speed);
        transform.translation.x += transform.left().x * wobble;
        transform.translation.z += transform.left().z * wobble;
        
        // Keep fish within water bounds (clamp position)
        let min_x = fish.water_center.x - fish.water_half_extents.x * settings.boundary_margin;
        let max_x = fish.water_center.x + fish.water_half_extents.x * settings.boundary_margin;
        let min_z = fish.water_center.z - fish.water_half_extents.y * settings.boundary_margin;
        let max_z = fish.water_center.z + fish.water_half_extents.y * settings.boundary_margin;
        
        transform.translation.x = transform.translation.x.clamp(min_x, max_x);
        transform.translation.z = transform.translation.z.clamp(min_z, max_z);
        transform.translation.y = fish.water_center.y - fish.depth; // Maintain depth
        
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
        log::info!("[FISH DEBUG] FishPlugin::build() called - registering fish systems");
        app
            // Register types for reflection
            .register_type::<Fish>()
            .register_type::<FishSettings>()
            // Add resources
            .init_resource::<FishSettings>()
            // Add events
            .add_event::<WaterSpawnedEvent>()
            // Add systems
            .add_systems(Update, (
                spawn_fish_on_water_system,
                update_fish_movement_system,
            ).chain());
    }
}
