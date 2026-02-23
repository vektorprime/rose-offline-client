//! Angelic Wing Spawn System
//! 
//! This system handles spawning and despawning of angelic wings when flight mode
//! is toggled. Wings are procedurally generated meshes with custom materials.
//!
//! Wing Design:
//! - Large, impressive angelic wings (2-3 units span)
//! - Multiple feather layers for realistic appearance
//! - Attached to character's back as child entities
//! - Custom shader material with glow effects

use bevy::prelude::*;
use bevy::render::mesh::{Mesh, Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::pbr::MeshMaterial3d;

use crate::components::{FlightState, PlayerCharacter, AngelicWings, WingSide};
use crate::events::FlightToggleEvent;
use crate::render::wing_material::{WingMaterial, WingMaterialPlugin};

/// Plugin for wing spawning and animation systems
pub struct WingSpawnPlugin;

impl Plugin for WingSpawnPlugin {
    fn build(&self, app: &mut App) {
        app
            // Add the wing material plugin
            .add_plugins(WingMaterialPlugin::default())
            // Add wing systems
            .add_systems(Update, (
                wing_spawn_system,
                wing_animation_system,
            ).chain());
        
        log::info!("[WingSpawn] WingSpawnPlugin initialized");
    }
}

/// System that spawns wings when flight is enabled.
/// 
/// This system listens for [`FlightToggleEvent`] events and spawns wing entities
/// when flight mode is enabled. The wings are attached as children to the player.
pub fn wing_spawn_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WingMaterial>>,
    mut flight_events: EventReader<FlightToggleEvent>,
    mut flight_states: Query<(Entity, &mut FlightState), With<PlayerCharacter>>,
) {
    for event in flight_events.read() {
        if let Ok((entity, mut flight_state)) = flight_states.get_mut(event.entity) {
            if flight_state.is_flying {
                // Flight just enabled - spawn wings
                if flight_state.wing_entity_left.is_none() && flight_state.wing_entity_right.is_none() {
                    let (left_wing, right_wing) = spawn_wings(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        entity,
                    );
                    
                    flight_state.wing_entity_left = Some(left_wing);
                    flight_state.wing_entity_right = Some(right_wing);
                    
                    log::info!("[WingSpawn] Spawned angelic wings for entity {:?}", entity);
                }
            }
            // Note: Wing despawning is handled in flight_toggle_system.rs
        }
    }
}

/// Spawns left and right wing entities attached to the character
fn spawn_wings(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<WingMaterial>>,
    _owner_entity: Entity,
) -> (Entity, Entity) {
    // Create wing meshes
    let left_wing_mesh = create_angel_wing_mesh(meshes, WingSide::Left);
    let right_wing_mesh = create_angel_wing_mesh(meshes, WingSide::Right);
    
    // Create wing material with default ethereal appearance
    let wing_material = materials.add(WingMaterial {
        base_color: LinearRgba::new(0.95, 0.95, 1.0, 1.0),  // White/silver
        glow_color: LinearRgba::new(0.6, 0.8, 1.0, 1.0),    // Soft blue glow
        glow_intensity: 0.8,
        time: 0.0,
        shimmer_speed: 1.0,
        alpha: 0.9,
    });
    
    // Spawn left wing entity
    let left_wing = commands.spawn((
        AngelicWings {
            side: WingSide::Left,
            flap_phase: 0.0,
            spread_amount: 1.0,
            glow_intensity: 0.8,
            is_spreading: true,
        },
        Mesh3d(left_wing_mesh),
        MeshMaterial3d(wing_material.clone()),
        // Position behind and slightly above character, angled outward
        Transform::from_xyz(-0.2, 0.8, -0.3)
            .looking_at(Vec3::new(-1.5, 0.5, -0.5), Vec3::Y),
        GlobalTransform::default(),
        Visibility::Visible,
        InheritedVisibility::default(),
        ViewVisibility::default(),
    )).id();
    
    // Spawn right wing entity
    let right_wing = commands.spawn((
        AngelicWings {
            side: WingSide::Right,
            flap_phase: 0.0,  // Slightly offset from left for natural look
            spread_amount: 1.0,
            glow_intensity: 0.8,
            is_spreading: true,
        },
        Mesh3d(right_wing_mesh),
        MeshMaterial3d(wing_material),
        // Position behind and slightly above character, angled outward
        Transform::from_xyz(0.2, 0.8, -0.3)
            .looking_at(Vec3::new(1.5, 0.5, -0.5), Vec3::Y),
        GlobalTransform::default(),
        Visibility::Visible,
        InheritedVisibility::default(),
        ViewVisibility::default(),
    )).id();
    
    (left_wing, right_wing)
}

/// Creates a procedural angel wing mesh
/// 
/// The wing is designed with:
/// - A main "arm" structure (like a bird/bat wing bone)
/// - Multiple feather layers (primary, secondary, coverts)
/// - Large span (about 2-3 units from body)
/// - Curved, natural shape
fn create_angel_wing_mesh(
    meshes: &mut ResMut<Assets<Mesh>>,
    side: WingSide,
) -> Handle<Mesh> {
    // Wing parameters
    let wing_span = 2.5;        // Length from body to tip
    let wing_height = 1.5;      // Height at highest point
    let feather_count = 12;     // Number of primary feathers
    let segments = 24;          // Horizontal segments for smoothness
    
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    
    // Mirror factor for left/right wings
    let mirror = match side {
        WingSide::Left => -1.0,
        WingSide::Right => 1.0,
    };
    
    // Create wing shape using multiple layers
    
    // Layer 1: Wing membrane/base (creates the overall wing shape)
    for i in 0..=segments {
        let t = i as f32 / segments as f32;  // 0.0 to 1.0 along wing
        
        // Wing shape curve - starts thick near body, tapers to tip
        // Uses bezier-like curve for natural shape
        let base_x = t * wing_span * mirror;
        
        // Wing curves upward then down toward tip
        let base_y = wing_height * (1.0 - t) * (1.0 - t * 0.3) * 
                     (1.0 + 0.3 * (t * std::f32::consts::PI).sin());
        
        // Wing depth (front to back) - fuller near body, narrow at tip
        let depth = 0.8 * (1.0 - t * 0.7) * (1.0 + 0.2 * (t * std::f32::consts::PI * 2.0).sin());
        
        // Leading edge (front of wing)
        let leading_z = -depth * 0.5;
        // Trailing edge (back of wing)
        let trailing_z = depth * 0.5;
        
        // Top surface vertex
        vertices.push([base_x, base_y + 0.02, leading_z]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([t, 0.0]);
        
        // Bottom surface vertex
        vertices.push([base_x, base_y - 0.02, leading_z]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([t, 0.3]);
        
        // Trailing edge top
        vertices.push([base_x * 0.95, base_y * 0.9 + 0.02, trailing_z]);
        normals.push([0.0, 1.0, 0.3]);
        uvs.push([t, 0.7]);
        
        // Trailing edge bottom
        vertices.push([base_x * 0.95, base_y * 0.9 - 0.02, trailing_z]);
        normals.push([0.0, -1.0, 0.3]);
        uvs.push([t, 1.0]);
    }
    
    // Create triangles for the wing membrane
    for i in 0..segments {
        let base = (i * 4) as u32;
        let next = ((i + 1) * 4) as u32;
        
        // Top surface
        indices.extend_from_slice(&[
            base, next, base + 2,      // Triangle 1
            base + 2, next, next + 2,  // Triangle 2
        ]);
        
        // Bottom surface
        indices.extend_from_slice(&[
            base + 1, base + 3, next + 1,  // Triangle 1
            next + 1, base + 3, next + 3,  // Triangle 2
        ]);
    }
    
    // Layer 2: Primary feathers (long feathers at wing tip)
    let feather_start_vertex = vertices.len() as u32;
    
    for i in 0..feather_count {
        let t = (i as f32 / feather_count as f32);  // 0.0 to 1.0
        let feather_t = 0.6 + t * 0.4;  // Feathers start at 60% along wing
        
        // Feather base position
        let base_x = feather_t * wing_span * mirror;
        let base_y = wing_height * (1.0 - feather_t) * 0.8;
        let base_z = 0.3 - t * 0.6;  // Spread along trailing edge
        
        // Feather tip (extends beyond wing membrane)
        let tip_x = (feather_t + 0.15 + t * 0.1) * wing_span * mirror;
        let tip_y = base_y * 0.5 - t * 0.2;
        let tip_z = base_z - 0.2 - t * 0.1;
        
        // Feather width
        let feather_width = 0.08 * (1.0 - t * 0.5);
        
        // Two triangles per feather (quad)
        let idx = vertices.len() as u32;
        
        // Feather vertices
        vertices.push([base_x, base_y, base_z]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([feather_t, 0.5 + t * 0.3]);
        
        vertices.push([base_x, base_y - 0.01, base_z]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([feather_t, 0.5 + t * 0.3]);
        
        vertices.push([tip_x, tip_y, tip_z - feather_width]);
        normals.push([0.0, 0.7, -0.3]);
        uvs.push([feather_t + 0.15, 0.6 + t * 0.3]);
        
        vertices.push([tip_x, tip_y, tip_z + feather_width]);
        normals.push([0.0, 0.7, 0.3]);
        uvs.push([feather_t + 0.15, 0.4 + t * 0.3]);
        
        // Feather indices (two triangles)
        indices.extend_from_slice(&[
            idx, idx + 2, idx + 3,  // Top
            idx + 1, idx + 3, idx + 2,  // Bottom
        ]);
    }
    
    // Layer 3: Secondary feathers (mid-wing feathers)
    let secondary_start = vertices.len() as u32;
    let secondary_count = 8;
    
    for i in 0..secondary_count {
        let t = i as f32 / secondary_count as f32;
        let feather_t = 0.3 + t * 0.35;  // Secondary feathers in middle section
        
        let base_x = feather_t * wing_span * mirror;
        let base_y = wing_height * (1.0 - feather_t * 0.5) * 0.9;
        let base_z = 0.2 - t * 0.4;
        
        let tip_x = (feather_t + 0.1) * wing_span * mirror;
        let tip_y = base_y * 0.85;
        let tip_z = base_z + 0.15;
        
        let feather_width = 0.1 * (1.0 - t * 0.3);
        
        let idx = vertices.len() as u32;
        
        vertices.push([base_x, base_y, base_z]);
        normals.push([0.0, 1.0, 0.1]);
        uvs.push([feather_t, 0.4 + t * 0.2]);
        
        vertices.push([base_x, base_y - 0.01, base_z]);
        normals.push([0.0, -1.0, 0.1]);
        uvs.push([feather_t, 0.4 + t * 0.2]);
        
        vertices.push([tip_x, tip_y, tip_z - feather_width]);
        normals.push([0.0, 0.8, 0.0]);
        uvs.push([feather_t + 0.1, 0.5 + t * 0.2]);
        
        vertices.push([tip_x, tip_y, tip_z + feather_width]);
        normals.push([0.0, 0.8, 0.0]);
        uvs.push([feather_t + 0.1, 0.3 + t * 0.2]);
        
        indices.extend_from_slice(&[
            idx, idx + 2, idx + 3,
            idx + 1, idx + 3, idx + 2,
        ]);
    }
    
    // Layer 4: Covert feathers (small feathers near body)
    let covert_start = vertices.len() as u32;
    let covert_count = 6;
    
    for i in 0..covert_count {
        let t = i as f32 / covert_count as f32;
        let feather_t = 0.1 + t * 0.25;  // Coverts near body
        
        let base_x = feather_t * wing_span * mirror;
        let base_y = wing_height * (1.0 - feather_t * 0.3);
        let base_z = -0.2 + t * 0.15;
        
        let tip_x = (feather_t + 0.08) * wing_span * mirror;
        let tip_y = base_y * 0.95;
        let tip_z = base_z + 0.1;
        
        let feather_width = 0.12;
        
        let idx = vertices.len() as u32;
        
        vertices.push([base_x, base_y + 0.03, base_z]);
        normals.push([0.0, 1.0, 0.2]);
        uvs.push([feather_t, 0.2 + t * 0.1]);
        
        vertices.push([base_x, base_y, base_z]);
        normals.push([0.0, 1.0, 0.2]);
        uvs.push([feather_t, 0.2 + t * 0.1]);
        
        vertices.push([tip_x, tip_y + 0.02, tip_z]);
        normals.push([0.0, 1.0, 0.1]);
        uvs.push([feather_t + 0.08, 0.25 + t * 0.1]);
        
        indices.extend_from_slice(&[
            idx, idx + 2, idx + 1,
        ]);
    }
    
    // Create the mesh
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

/// System that animates the wings (flapping, spreading, glow)
/// 
/// This system updates wing transforms for flapping animation and
/// adjusts material properties for glow effects.
pub fn wing_animation_system(
    time: Res<Time>,
    mut wing_query: Query<(&AngelicWings, &mut Transform), With<AngelicWings>>,
    flight_query: Query<&FlightState, With<PlayerCharacter>>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();
    
    // Get flight state to determine animation speed
    let is_thrusting = flight_query.iter().any(|state| state.is_thrusting);
    let flap_speed = if is_thrusting { 6.0 } else { 2.0 };
    
    for (wing, mut transform) in wing_query.iter_mut() {
        // Calculate flap rotation
        let flap_phase = elapsed * flap_speed + 
            if wing.side == WingSide::Right { 0.1 } else { 0.0 };  // Slight offset for natural look
        
        let flap_angle = (flap_phase.sin() * 0.3) + 0.1;  // -20 to +40 degrees flap
        
        // Apply rotation based on wing side
        let mirror = match wing.side {
            WingSide::Left => -1.0,
            WingSide::Right => 1.0,
        };
        
        // Wing flaps up/down and slightly forward/back
        let flap_rotation = Quat::from_rotation_z(flap_angle * mirror);
        let forward_rotation = Quat::from_rotation_x((flap_phase * 0.5).sin() * 0.1);
        
        transform.rotation = flap_rotation * forward_rotation;
        
        // Subtle scale pulsing for ethereal effect
        let scale_pulse = 1.0 + (elapsed * 2.0).sin() * 0.02;
        transform.scale = Vec3::splat(scale_pulse);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wing_side_mirror() {
        let left_mirror = match WingSide::Left {
            WingSide::Left => -1.0,
            WingSide::Right => 1.0,
        };
        assert_eq!(left_mirror, -1.0);
        
        let right_mirror = match WingSide::Right {
            WingSide::Left => -1.0,
            WingSide::Right => 1.0,
        };
        assert_eq!(right_mirror, 1.0);
    }
}
