//! Bird spawning and flying behavior system
//! 
//! This system handles:
//! - Spawning birds when a zone is loaded
//! - Bird flying AI (picking targets, moving towards them)
//! - Bird animation (wing flapping, vertical bobbing)
//! - Keeping birds within roam bounds

use bevy::prelude::*;
use bevy::render::mesh::{Mesh, Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use rand::Rng;

use crate::components::{Bird, BirdSettings, BirdMesh, Zone};
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
        
        // Get zone center from query - zone should exist now since ZoneEvent::Loaded
        // is sent AFTER the zone entity is spawned
        let (zone_entity, zone_transform) = zone_query.iter().next()
            .map(|(e, t)| (e, t.translation))
            .unwrap_or((Entity::PLACEHOLDER, Vec3::ZERO));
        
        log::info!(
            "[BIRD] Received ZoneEvent::Loaded for zone {}, spawning birds at zone center {:?}",
            zone_id.get(),
            zone_transform
        );
        
        spawn_birds(
            &mut commands,
            &mut meshes,
            &mut materials,
            &settings,
            zone_transform,
            zone_entity,
        );
    }
    
    if event_count > 0 {
        log::info!("[BIRD] Processed {} ZoneEvent::Loaded event(s) this frame", event_count);
    }
}

/// Spawns a flock of birds around a center point
fn spawn_birds(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &BirdSettings,
    zone_center: Vec3,
    zone_entity: Entity,
) {
    let mut rng = rand::thread_rng();
    
    log::info!(
        "[BIRD] Spawning {} birds at zone center {:?} with roam radius {}",
        settings.birds_per_zone,
        zone_center,
        settings.roam_radius
    );
    
    // Create bird mesh (simple V-shape)
    let bird_mesh = create_bird_mesh(meshes);
    
    // Bird colors for variety
    let bird_colors = [
        Color::srgb(0.4, 0.3, 0.2),  // Brown
        Color::srgb(0.3, 0.3, 0.35), // Gray
        Color::srgb(0.15, 0.15, 0.12), // Dark
        Color::srgb(0.5, 0.4, 0.3),  // Light brown
        Color::srgb(0.35, 0.35, 0.4), // Light gray
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
    
    for i in 0..settings.birds_per_zone {
        // Random position within roam radius
        let angle = rng.gen::<f32>() * std::f32::consts::TAU;
        let distance = rng.gen::<f32>() * settings.roam_radius;
        let altitude = rng.gen_range(settings.min_altitude..settings.max_altitude);
        
        let x = zone_center.x + angle.cos() * distance;
        let z = zone_center.z + angle.sin() * distance;
        let y = zone_center.y + altitude;
        
        let speed = rng.gen_range(settings.min_speed..settings.max_speed);
        let initial_phase = rng.gen::<f32>() * std::f32::consts::TAU;
        
        // Random color material
        let material_idx = rng.gen_range(0..bird_materials.len());
        let material = bird_materials[material_idx].clone();
        
        let target_position = get_new_target(zone_center, settings.roam_radius, settings.min_altitude, settings.max_altitude);
        
        // Initial rotation facing the target
        let direction = target_position - Vec3::new(x, y, z);
        let initial_rotation = if direction.length() > 0.01 {
            let look_direction = direction.normalize();
            Quat::from_rotation_y(look_direction.z.atan2(look_direction.x) - std::f32::consts::FRAC_PI_2)
        } else {
            Quat::IDENTITY
        };
        
        // Spawn bird entity
        let bird_entity = commands.spawn((
            Bird {
                speed,
                target_position,
                roam_center: zone_center,
                roam_radius: settings.roam_radius,
                flap_phase: initial_phase,
                bob_phase: initial_phase * 0.5,
            },
            Transform::from_xyz(x, y, z)
                .with_rotation(initial_rotation)
                .with_scale(Vec3::splat(0.5)), // Scale birds appropriately
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        )).id();
        
        // Spawn bird mesh as child entity
        let mesh_entity = commands.spawn((
            BirdMesh,
            Mesh3d(bird_mesh.clone()),
            MeshMaterial3d(material),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        )).id();
        
        commands.entity(bird_entity).add_child(mesh_entity);
        
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
    
    log::info!("[BIRD] Spawned {} birds total", settings.birds_per_zone);
}

/// Creates a simple V-shaped bird mesh
fn create_bird_mesh(meshes: &mut ResMut<Assets<Mesh>>) -> Handle<Mesh> {
    // Simple triangular bird shape
    // Vertices form a V-shape (bird silhouette from below)
    // Bird body is along Z axis, with nose at +Z
    
    let vertices: Vec<[f32; 3]> = vec![
        // Body center
        [0.0, 0.0, 0.0],
        // Left wing tip
        [-0.3, 0.0, -0.15],
        // Left wing mid
        [-0.15, 0.0, 0.05],
        // Nose
        [0.0, 0.0, 0.2],
        // Right wing mid
        [0.15, 0.0, 0.05],
        // Right wing tip
        [0.3, 0.0, -0.15],
    ];
    
    // Simple upward normals
    let normals: Vec<[f32; 3]> = vec![
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    
    // UV coordinates
    let uvs: Vec<[f32; 2]> = vec![
        [0.5, 0.5],
        [0.0, 0.0],
        [0.25, 0.5],
        [0.5, 1.0],
        [0.75, 0.5],
        [1.0, 0.0],
    ];
    
    // Triangle indices
    let indices: Vec<u32> = vec![
        0, 1, 2,  // Left wing
        0, 2, 3,  // Left body
        0, 3, 4,  // Right body
        0, 4, 5,  // Right wing
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
    let altitude = rng.gen_range(min_alt..max_alt);
    
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
    mut mesh_query: Query<&mut Transform, (With<BirdMesh>, Without<Bird>)>,
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
            
            // Face movement direction (smooth rotation)
            let target_rotation = Quat::from_rotation_y(
                move_dir.z.atan2(move_dir.x) - std::f32::consts::FRAC_PI_2
            );
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
        
        // Apply wing flap to mesh (scale Y based on sine wave)
        let flap_scale = 1.0 + (bird.flap_phase.sin() * 0.3);
        let bob_offset = (bird.bob_phase.sin() * settings.bob_amplitude) * 0.1;
        
        // Apply to child mesh
        if let Ok(children) = children_query.get(bird_entity) {
            for &child in children.iter() {
                if let Ok(mut mesh_transform) = mesh_query.get_mut(child) {
                    mesh_transform.scale.y = flap_scale;
                    mesh_transform.translation.y = bob_offset;
                }
            }
        }
    }
}
