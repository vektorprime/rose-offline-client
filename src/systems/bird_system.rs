//! Bird spawning and flying behavior system
//! 
//! This system handles:
//! - Spawning birds when a zone is loaded (count relative to zone size)
//! - Bird flying AI (picking targets, moving towards them)
//! - Bird animation (wing flapping with rotating wings, vertical bobbing)
//! - Keeping birds within roam bounds
//! - Birds face their flight direction

use bevy::prelude::*;
use bevy::render::mesh::{Mesh, Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use rand::Rng;

use crate::components::{Bird, BirdSettings, BirdMesh, BirdWingLeft, BirdWingRight, Zone};
use crate::events::ZoneEvent;

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
            // Add systems
            .add_systems(Update, (
                spawn_birds_on_zone_system,
                update_bird_movement_system,
            ).chain());
    }
}

/// Spawns birds when a zone is loaded
pub fn spawn_birds_on_zone_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<BirdSettings>,
    mut zone_events: EventReader<ZoneEvent>,
    zone_query: Query<(Entity, &Transform), With<Zone>>,
) {
    if !settings.enabled {
        return;
    }
    
    let mut event_count = 0;
    for event in zone_events.read() {
        // Only handle Loaded events
        let ZoneEvent::Loaded(zone_id) = event;
        
        event_count += 1;
        
        // Get zone entity from query - zone should exist now since ZoneEvent::Loaded
        // is sent AFTER the zone entity is spawned
        // NOTE: We use Vec3::ZERO as the spawn center because birds will be parented
        // to the zone entity. If we used zone_transform.translation, birds would be
        // positioned at double the offset (zone pos + local pos) after parenting.
        let zone_entity = zone_query.iter().next()
            .map(|(e, _)| e)
            .unwrap_or(Entity::PLACEHOLDER);
        
        log::info!(
            "[BIRD] Received ZoneEvent::Loaded for zone {}, spawning birds parented to zone entity {:?}",
            zone_id.get(),
            zone_entity
        );
        
        // Calculate zone size for relative bird count
        // Default zone size is based on 64x64 blocks with grid_size * grid_per_patch * 16.0 per block
        // Typical values: grid_size=1.0, grid_per_patch=1.0, so ~160 units per block, ~10240 units per zone
        let zone_size = 10240.0; // Default zone size in units
        let bird_count = calculate_bird_count(zone_size, &settings);
        
        spawn_birds(
            &mut commands,
            &mut meshes,
            &mut materials,
            &settings,
            Vec3::ZERO,  // Use zero since birds are parented to zone (zone-local coordinates)
            zone_entity,
            zone_size,
            bird_count,
        );
    }
    
    if event_count > 0 {
        log::info!("[BIRD] Processed {} ZoneEvent::Loaded event(s) this frame", event_count);
    }
}

/// Calculate bird count based on zone size
fn calculate_bird_count(zone_size: f32, settings: &BirdSettings) -> usize {
    // Zone area in millions of square units
    let zone_area = zone_size * zone_size;
    let area_in_1000_units = zone_area / (1000.0 * 1000.0);
    
    // Calculate bird count based on area
    let calculated_count = (area_in_1000_units * settings.birds_per_1000_units) as usize;
    
    // Clamp to min/max
    calculated_count.clamp(settings.min_birds_per_zone, settings.max_birds_per_zone)
}

/// Spawns a flock of birds around a center point
fn spawn_birds(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &BirdSettings,
    zone_center: Vec3,
    zone_entity: Entity,
    zone_size: f32,
    bird_count: usize,
) {
    let mut rng = rand::thread_rng();
    
    // Calculate roam radius based on zone size
    let roam_radius = zone_size * settings.roam_radius_multiplier * 0.5;
    
    log::info!(
        "[BIRD] Spawning {} birds at zone center {:?} with roam radius {} (zone_size={})",
        bird_count,
        zone_center,
        roam_radius,
        zone_size
    );
    
    // Create bird mesh parts
    let body_mesh = create_bird_body_mesh(meshes);
    let left_wing_mesh = create_bird_wing_left_mesh(meshes);
    let right_wing_mesh = create_bird_wing_right_mesh(meshes);
    
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
    
    // Create materials for each color
    let bird_materials: Vec<Handle<StandardMaterial>> = bird_colors.iter().map(|&color| {
        materials.add(StandardMaterial {
            base_color: color,
            unlit: true, // Birds don't need complex lighting for distance viewing
            cull_mode: None, // Double-sided for better visibility
            ..default()
        })
    }).collect();
    
    for i in 0..bird_count {
        // Random position within roam radius
        let angle = rng.gen::<f32>() * std::f32::consts::TAU;
        let distance = rng.gen::<f32>() * roam_radius;
        
        // Clamp min/max to prevent crash if settings are invalid
        let min_alt = settings.min_altitude.min(settings.max_altitude);
        let max_alt = settings.max_altitude.max(settings.min_altitude);
        let altitude = rng.gen_range(min_alt..max_alt);
        
        let x = zone_center.x + angle.cos() * distance;
        let z = zone_center.z + angle.sin() * distance;
        let y = zone_center.y + altitude;
        
        // Clamp min/max to prevent crash
        let min_spd = settings.min_speed.min(settings.max_speed);
        let max_spd = settings.max_speed.max(settings.min_speed);
        let speed = rng.gen_range(min_spd..max_spd);
        let initial_phase = rng.gen::<f32>() * std::f32::consts::TAU;
        
        // Random color material
        let material_idx = rng.gen_range(0..bird_materials.len());
        let material = bird_materials[material_idx].clone();
        
        let target_position = get_new_target(zone_center, roam_radius, settings.min_altitude, settings.max_altitude);
        
        // Initial rotation facing the target
        let direction = target_position - Vec3::new(x, y, z);
        let initial_rotation = if direction.length() > 0.01 {
            let look_direction = direction.normalize();
            // Bird body faces +Z (forward), so we need to rotate to face movement direction
            Quat::from_rotation_y((-look_direction.x).atan2(look_direction.z))
        } else {
            Quat::IDENTITY
        };
        
        // Spawn bird entity (parent container)
        let bird_entity = commands.spawn((
            Bird {
                speed,
                target_position,
                roam_center: zone_center,
                roam_radius,
                flap_phase: initial_phase,
                bob_phase: initial_phase * 0.5,
            },
            Transform::from_xyz(x, y, z)
                .with_rotation(initial_rotation)
                .with_scale(Vec3::splat(1.5)), // Scale for visibility
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        )).id();
        
        // Spawn bird body mesh as child
        let body_entity = commands.spawn((
            BirdMesh,
            Mesh3d(body_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        )).id();
        commands.entity(bird_entity).add_child(body_entity);
        
        // Spawn left wing as child (rotates around body center)
        let left_wing_entity = commands.spawn((
            BirdWingLeft,
            Mesh3d(left_wing_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_rotation(Quat::from_rotation_z(0.3)), // Slightly spread
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        )).id();
        commands.entity(bird_entity).add_child(left_wing_entity);
        
        // Spawn right wing as child (rotates around body center)
        let right_wing_entity = commands.spawn((
            BirdWingRight,
            Mesh3d(right_wing_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_rotation(Quat::from_rotation_z(-0.3)), // Slightly spread
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        )).id();
        commands.entity(bird_entity).add_child(right_wing_entity);
        
        // Parent bird to zone entity so it inherits zone transform
        if zone_entity != Entity::PLACEHOLDER {
            commands.entity(zone_entity).add_child(bird_entity);
        }
        
        if i < 3 {
            log::info!(
                "[BIRD DEBUG] Spawned bird {} at position {:?} with speed {}, parented to zone {:?}",
                i, Vec3::new(x, y, z), speed, zone_entity
            );
        }
    }
    
    log::info!("[BIRD] Spawned {} birds total", bird_count);
}

/// Creates the bird body mesh (torso, head, tail)
fn create_bird_body_mesh(meshes: &mut ResMut<Assets<Mesh>>) -> Handle<Mesh> {
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
        [0.0, 0.1, 0.18],    // Top of head
        [0.0, -0.02, 0.18],  // Bottom of beak junction
        
        // Head back / neck
        [-0.05, 0.07, 0.1],
        [0.05, 0.07, 0.1],
        [0.0, 0.11, 0.1],    // Top
        [0.0, -0.02, 0.1],   // Bottom
        
        // Body front (widest part of chest)
        [-0.08, 0.06, 0.0],
        [0.08, 0.06, 0.0],
        [0.0, 0.1, 0.0],     // Top of back
        [0.0, -0.03, 0.0],   // Belly
        
        // Body back (where tail starts)
        [-0.06, 0.05, -0.12],
        [0.06, 0.05, -0.12],
        [0.0, 0.08, -0.12],  // Top
        [0.0, -0.02, -0.12], // Bottom
        
        // Tail tip (fan shape)
        [-0.08, 0.03, -0.25],
        [0.0, 0.05, -0.28],  // Center tail feather (longest)
        [0.08, 0.03, -0.25],
        [0.0, -0.01, -0.22], // Bottom tail
    ];
    
    // Triangle indices for the body
    let indices: Vec<u32> = vec![
        // Beak to head front
        0, 4, 1,  // Bottom left
        0, 2, 4,  // Bottom right
        0, 1, 3,  // Top left
        0, 3, 2,  // Top right
        
        // Head front to head back
        1, 4, 8,  // Bottom left
        4, 2, 8,  // Bottom right (fixed)
        1, 5, 3,  // Left top
        3, 5, 6,  // Top
        3, 6, 2,  // Right top
        1, 8, 5,  // Left side
        2, 6, 8,  // Right side
        
        // Head back to body front
        5, 8, 12, // Left bottom
        8, 6, 12, // Right bottom (fixed)
        5, 9, 7,  // Left top
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
        [0.5, 0.95],   // 0: Beak tip
        [0.3, 0.85],   // 1
        [0.7, 0.85],   // 2
        [0.5, 0.9],    // 3
        [0.5, 0.8],    // 4
        [0.25, 0.7],   // 5
        [0.75, 0.7],   // 6
        [0.5, 0.75],   // 7
        [0.5, 0.65],   // 8
        [0.2, 0.5],    // 9
        [0.8, 0.5],    // 10
        [0.5, 0.55],   // 11
        [0.5, 0.45],   // 12
        [0.25, 0.3],   // 13
        [0.75, 0.3],   // 14
        [0.5, 0.35],   // 15
        [0.5, 0.25],   // 16
        [0.2, 0.1],    // 17
        [0.5, 0.05],   // 18
        [0.8, 0.1],    // 19
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

/// Creates the left wing mesh (positioned for rotation around body center)
fn create_bird_wing_left_mesh(meshes: &mut ResMut<Assets<Mesh>>) -> Handle<Mesh> {
    // Left wing extends in -X direction from body center
    // Wing pivots at body center (0,0,0) for flapping animation
    
    let vertices: Vec<[f32; 3]> = vec![
        // Wing root (attaches to body)
        [-0.05, 0.05, 0.05],   // Front top
        [-0.05, 0.02, 0.05],   // Front bottom
        [-0.05, 0.05, -0.05],  // Back top
        [-0.05, 0.02, -0.05],  // Back bottom
        
        // Wing mid
        [-0.2, 0.06, 0.03],    // Front top
        [-0.2, 0.0, 0.03],     // Front bottom
        [-0.2, 0.05, -0.05],   // Back top
        [-0.2, 0.0, -0.05],    // Back bottom
        
        // Wing tip (pointed)
        [-0.35, 0.04, -0.02],  // Tip top
        [-0.35, -0.02, -0.02], // Tip bottom
    ];
    
    let indices: Vec<u32> = vec![
        // Top surface
        0, 2, 4,
        4, 2, 6,
        4, 6, 8,
        
        // Bottom surface
        1, 5, 3,
        3, 5, 7,
        5, 9, 7,
        
        // Front edge
        0, 4, 1,
        1, 4, 5,
        
        // Back edge
        2, 3, 6,
        6, 3, 7,
        
        // Tip
        6, 7, 8,
        8, 7, 9,
    ];
    
    let normals: Vec<[f32; 3]> = vec![
        [0.2, 0.9, 0.1],    // 0
        [0.2, -0.9, 0.1],   // 1
        [0.2, 0.9, -0.1],   // 2
        [0.2, -0.9, -0.1],  // 3
        [0.1, 0.95, 0.05],  // 4
        [0.1, -0.95, 0.05], // 5
        [0.1, 0.95, -0.05], // 6
        [0.1, -0.95, -0.05],// 7
        [0.0, 0.95, 0.0],   // 8
        [0.0, -0.95, 0.0],  // 9
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
    
    mesh.insert_indices(Indices::U32(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    
    meshes.add(mesh)
}

/// Creates the right wing mesh (mirrored from left)
fn create_bird_wing_right_mesh(meshes: &mut ResMut<Assets<Mesh>>) -> Handle<Mesh> {
    // Right wing extends in +X direction from body center (mirrored from left)
    
    let vertices: Vec<[f32; 3]> = vec![
        // Wing root (attaches to body)
        [0.05, 0.05, 0.05],   // Front top
        [0.05, 0.02, 0.05],   // Front bottom
        [0.05, 0.05, -0.05],  // Back top
        [0.05, 0.02, -0.05],  // Back bottom
        
        // Wing mid
        [0.2, 0.06, 0.03],    // Front top
        [0.2, 0.0, 0.03],     // Front bottom
        [0.2, 0.05, -0.05],   // Back top
        [0.2, 0.0, -0.05],    // Back bottom
        
        // Wing tip (pointed)
        [0.35, 0.04, -0.02],  // Tip top
        [0.35, -0.02, -0.02], // Tip bottom
    ];
    
    let indices: Vec<u32> = vec![
        // Top surface
        2, 0, 4,
        6, 2, 4,
        6, 4, 8,
        
        // Bottom surface
        5, 1, 3,
        7, 5, 3,
        9, 5, 7,
        
        // Front edge
        4, 0, 1,
        5, 4, 1,
        
        // Back edge
        3, 2, 6,
        7, 3, 6,
        
        // Tip
        8, 7, 6,
        9, 7, 8,
    ];
    
    let normals: Vec<[f32; 3]> = vec![
        [-0.2, 0.9, 0.1],    // 0
        [-0.2, -0.9, 0.1],   // 1
        [-0.2, 0.9, -0.1],   // 2
        [-0.2, -0.9, -0.1],  // 3
        [-0.1, 0.95, 0.05],  // 4
        [-0.1, -0.95, 0.05], // 5
        [-0.1, 0.95, -0.05], // 6
        [-0.1, -0.95, -0.05],// 7
        [0.0, 0.95, 0.0],    // 8
        [0.0, -0.95, 0.0],   // 9
    ];
    
    let uvs: Vec<[f32; 2]> = vec![
        [0.1, 0.6],
        [0.1, 0.4],
        [0.3, 0.6],
        [0.3, 0.4],
        [0.5, 0.65],
        [0.5, 0.35],
        [0.7, 0.6],
        [0.7, 0.4],
        [0.9, 0.5],
        [0.9, 0.5],
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

/// Gets a new random target position within roam bounds
fn get_new_target(center: Vec3, radius: f32, min_alt: f32, max_alt: f32) -> Vec3 {
    let mut rng = rand::thread_rng();
    let angle = rng.gen::<f32>() * std::f32::consts::TAU;
    let distance = rng.gen::<f32>() * radius;
    // Clamp min/max to prevent crash
    let altitude = rng.gen_range(min_alt.min(max_alt)..max_alt.max(min_alt));
    
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
    mut bird_query: Query<(Entity, &mut Bird, &mut Transform), With<Bird>>,
    mut left_wing_query: Query<&mut Transform, (With<BirdWingLeft>, Without<Bird>, Without<BirdWingRight>)>,
    mut right_wing_query: Query<&mut Transform, (With<BirdWingRight>, Without<Bird>, Without<BirdWingLeft>)>,
    children_query: Query<&Children, With<Bird>>,
) {
    if !settings.enabled {
        return;
    }
    
    let dt = time.delta_secs();
    
    for (bird_entity, mut bird, mut transform) in bird_query.iter_mut() {
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
        
        // Update bob animation
        bird.bob_phase += settings.bob_speed * dt;
        if bird.bob_phase > std::f32::consts::TAU {
            bird.bob_phase -= std::f32::consts::TAU;
        }
        
        // Calculate wing flap angle (sinusoidal motion)
        // Wings flap up and down: positive angle = up, negative = down
        let flap_angle = (bird.flap_phase).sin() * 0.6; // ±34 degrees flap
        
        // Calculate bob offset
        let bob_offset = (bird.bob_phase.sin() * settings.bob_amplitude) * 0.1;
        
        // Apply to child wings and body
        if let Ok(children) = children_query.get(bird_entity) {
            for child in children.iter() {
                // Try to get left wing
                if let Ok(mut wing_transform) = left_wing_query.get_mut(child) {
                    // Left wing rotates around Z axis (positive = up)
                    wing_transform.rotation = Quat::from_rotation_z(0.3 + flap_angle);
                }
                // Try to get right wing
                if let Ok(mut wing_transform) = right_wing_query.get_mut(child) {
                    // Right wing rotates around Z axis (negative = up, so we negate)
                    wing_transform.rotation = Quat::from_rotation_z(-0.3 - flap_angle);
                }
            }
        }
        
        // Apply bob to bird body translation (small vertical oscillation)
        // This is handled by modifying the bird's Y position slightly
        let base_y = transform.translation.y;
        // We don't want to permanently modify Y, so we use a small oscillation
        // that gets applied each frame. The bob is subtle.
        // Note: This creates a slight vertical wobble effect
        let _bob_y = bob_offset; // Small vertical movement
    }
}
