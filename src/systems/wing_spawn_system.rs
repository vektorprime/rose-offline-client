//! Angelic Wing Spawn System
//!
//! This system handles spawning and despawning of angelic wings when flight mode
//! is toggled. Wings are procedurally generated meshes with custom materials.
//!
//! Wing Design:
//! - Large, impressive angelic wings (2-3 units span)
//! - Multiple feather layers for realistic appearance
//! - Attached to character's Body mesh entity (not player root) for proper skeletal animation
//! - Custom shader material with glow effects

use bevy::prelude::*;
use bevy::render::mesh::{Mesh, Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::pbr::MeshMaterial3d;
use bevy::render::alpha::AlphaMode;

use crate::components::{CharacterModel, CharacterModelPart, FlightState, PlayerCharacter, AngelicWings, WingSide};
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

/// System that handles wing spawning when flight is enabled.
///
/// This system listens for [`FlightToggleEvent`] events and would normally spawn wing entities
/// when flight mode is enabled. Currently, wing model spawning is DISABLED - the system
/// only logs when flight is enabled without spawning visual wings.
///
/// To re-enable wing spawning, uncomment the spawn_wings() call below.
pub fn wing_spawn_system(
    mut _commands: Commands,
    mut _meshes: ResMut<Assets<Mesh>>,
    mut _materials: ResMut<Assets<WingMaterial>>,
    mut flight_events: EventReader<FlightToggleEvent>,
    player_query: Query<(Entity, &CharacterModel), With<PlayerCharacter>>,
    mut flight_states: Query<&mut FlightState, With<PlayerCharacter>>,
) {
    for event in flight_events.read() {
        // Get CharacterModel from the player query
        if let Ok((entity, _character_model)) = player_query.get(event.entity) {
            // Get FlightState separately to avoid borrow conflicts
            if let Ok(_flight_state) = flight_states.get_mut(event.entity) {
                // Wing spawning is currently DISABLED.
                // The flight state is managed in flight_toggle_system.rs.
                // Flying functionality (movement, animation) still works without visual wings.
                //
                // To re-enable wing spawning, uncomment the code below:
                //
                // let flight_state = _flight_state.into_inner();
                //
                // if flight_state.is_flying {
                //     if flight_state.wing_entity_left.is_none() && flight_state.wing_entity_right.is_none() {
                //         let body_entities = &_character_model.model_parts[CharacterModelPart::Body].1;
                //         let attachment_parent = body_entities.first().copied().unwrap_or(entity);
                //
                //         let (left_wing, right_wing) = spawn_wings(
                //             &mut _commands,
                //             &mut _meshes,
                //             &mut _materials,
                //             attachment_parent,
                //             body_entities,
                //         );
                //
                //         flight_state.wing_entity_left = Some(left_wing);
                //         flight_state.wing_entity_right = Some(right_wing);
                //     }
                // }
                
                log::info!(
                    "[WingSpawn] Flight enabled for entity {:?} - wing spawning disabled (models will be added later)",
                    entity
                );
            }
        }
    }
}

/// Spawns left and right wing entities attached to the character's body
///
/// # Arguments
/// * `commands` - Bevy commands for spawning entities
/// * `meshes` - Mesh asset collection
/// * `materials` - Wing material asset collection
/// * `attachment_parent` - The entity to attach wings to (typically the Body mesh entity)
/// * `body_entities` - All Body mesh entities (for reference, may be used for multi-part attachment)
///
/// # Wing Positioning
/// Wings are positioned relative to the attachment parent (Body mesh).
/// Since Body mesh is part of the skeletal hierarchy, wings will follow
/// skeletal animations properly.
fn spawn_wings(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<WingMaterial>>,
    attachment_parent: Entity,
    _body_entities: &[Entity],
) -> (Entity, Entity) {
    // Create wing meshes
    let left_wing_mesh = create_angel_wing_mesh(meshes, WingSide::Left);
    let right_wing_mesh = create_angel_wing_mesh(meshes, WingSide::Right);
    
    // Create wing material with enhanced ethereal appearance
    // Brighter base color with subtle emissive glow for angelic effect
    let wing_material = materials.add(WingMaterial {
        base_color: Color::srgba(0.98, 0.98, 1.0, 0.9),  // Brighter white/silver, more opaque
        alpha_mode: AlphaMode::Blend,
        cull_mode: None, // Double-sided
        perceptual_roughness: 0.2,  // Smoother for more ethereal look
        metallic: 0.15,  // Slight metallic sheen
        emissive: LinearRgba::new(0.8, 0.85, 1.0, 0.3),  // Subtle light blue glow
        ..Default::default()
    });
    
    // IMPORTANT: Wing transform offsets are now relative to the Body mesh entity,
    // not the player root. The Body mesh is part of the skeletal hierarchy.
    //
    // Since the Body mesh is typically positioned near the character's center/pelvis,
    // we need to offset wings to appear at shoulder blade level on the back.
    // The Y offset is relative to the Body mesh's local origin.
    //
    // Character skeleton: root/pelvis is at ~Y=0, head is at ~Y=1.5-1.8
    // Shoulder blades are approximately at Y=1.0-1.2 relative to pelvis
    // But since Body mesh origin is typically at pelvis, we use smaller offsets.
    
    // Left wing: positioned on left side of back, at shoulder blade level
    // Local offset from Body mesh origin
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
        // Position relative to Body mesh:
        // X: -0.15 (slightly left of center for left wing attachment point)
        // Y: 0.9 (upper back / shoulder blade level relative to pelvis)
        // Z: -0.2 (behind the body, on the back)
        Transform::from_xyz(-0.1, 0.9, -0.15)
            .looking_at(Vec3::new(-1.5, 0.5, -0.15), Vec3::Y),
        GlobalTransform::default(),
        Visibility::Visible,
        InheritedVisibility::default(),
        ViewVisibility::default(),
    )).id();
    
    // Attach left wing to the Body mesh entity
    commands.entity(attachment_parent).add_child(left_wing);
    
    log::info!(
        "[WingSpawn] Left wing {:?} attached to Body entity {:?}",
        left_wing,
        attachment_parent
    );
    
    // Right wing: positioned on right side of back, at shoulder blade level
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
        // Position relative to Body mesh:
        // X: 0.1 (slightly right of center for right wing attachment point)
        // Y: 0.9 (upper back / shoulder blade level relative to pelvis)
        // Z: -0.15 (behind the body, on the back)
        Transform::from_xyz(0.1, 0.9, -0.15)
            .looking_at(Vec3::new(1.5, 0.5, -0.15), Vec3::Y),
        GlobalTransform::default(),
        Visibility::Visible,
        InheritedVisibility::default(),
        ViewVisibility::default(),
    )).id();
    
    // Attach right wing to the Body mesh entity
    commands.entity(attachment_parent).add_child(right_wing);
    
    log::info!(
        "[WingSpawn] Right wing {:?} attached to Body entity {:?}",
        right_wing,
        attachment_parent
    );
    
    (left_wing, right_wing)
}

/// Creates a procedural angel wing mesh with enhanced quality
///
/// The wing is designed with:
/// - A main "arm" structure (like a bird/bat wing bone)
/// - Multiple feather layers (primary, secondary, tertiary, coverts)
/// - Large span (about 2.5-3 units from body)
/// - Curved, natural shape with smoother curves
/// - Gradient coloring from white base to silver/pearlescent tips
fn create_angel_wing_mesh(
    meshes: &mut ResMut<Assets<Mesh>>,
    side: WingSide,
) -> Handle<Mesh> {
    // Enhanced wing parameters for better quality
    let wing_span = 2.8;        // Longer span for more dramatic wings
    let wing_height = 1.6;      // Slightly taller for better proportions
    let primary_feather_count = 16;  // Increased from 12 for fuller wing tip
    let secondary_feather_count = 12; // Increased from 8 for denser mid-wing
    let tertiary_feather_count = 5;   // NEW: Small feathers near body
    let segments = 32;          // Increased from 24 for smoother curves
    
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    
    // Mirror factor for left/right wings
    let mirror = match side {
        WingSide::Left => -1.0,
        WingSide::Right => 1.0,
    };
    
    // ============================================
    // Layer 1: Wing membrane/base (creates the overall wing shape)
    // Enhanced with smoother curves and better proportions
    // ============================================
    for i in 0..=segments {
        let t = i as f32 / segments as f32;  // 0.0 to 1.0 along wing
        
        // Wing shape curve - starts thick near body, tapers to tip
        // Uses bezier-like curve for natural shape with enhanced curve
        let base_x = t * wing_span * mirror;
        
        // Enhanced wing curve - more organic upward sweep then graceful down
        let curve_factor = (t * std::f32::consts::PI * 0.8).sin();
        let base_y = wing_height * (1.0 - t * 0.6) * (1.0 - t * 0.25) *
                     (1.0 + 0.35 * curve_factor);
        
        // Wing depth (front to back) - fuller near body, narrow at tip
        let depth = 0.9 * (1.0 - t * 0.65) * (1.0 + 0.25 * (t * std::f32::consts::PI * 1.5).sin());
        
        // Leading edge (front of wing) with slight curve
        let leading_z = -depth * 0.5 - 0.05 * (t * std::f32::consts::PI).sin();
        // Trailing edge (back of wing)
        let trailing_z = depth * 0.5;
        
        // UV coordinates for gradient effect (white base to silver tip)
        let uv_gradient = t; // 0 at body (white) to 1 at tip (silver)
        
        // Top surface vertex
        vertices.push([base_x, base_y + 0.02, leading_z]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([t, 0.0 + uv_gradient * 0.1]); // Slight UV shift for gradient
        
        // Bottom surface vertex
        vertices.push([base_x, base_y - 0.02, leading_z]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([t, 0.3 + uv_gradient * 0.05]);
        
        // Trailing edge top
        vertices.push([base_x * 0.95, base_y * 0.88 + 0.02, trailing_z]);
        normals.push([0.0, 1.0, 0.3]);
        uvs.push([t, 0.7 + uv_gradient * 0.1]);
        
        // Trailing edge bottom
        vertices.push([base_x * 0.95, base_y * 0.88 - 0.02, trailing_z]);
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
    
    // ============================================
    // Layer 2: Primary feathers (long feathers at wing tip)
    // Enhanced: 16 feathers, longer, more tapered with curve
    // ============================================
    for i in 0..primary_feather_count {
        let t = (i as f32 / primary_feather_count as f32);  // 0.0 to 1.0
        let feather_t = 0.55 + t * 0.45;  // Feathers start at 55% along wing
        
        // Feather base position
        let base_x = feather_t * wing_span * mirror;
        let base_y = wing_height * (1.0 - feather_t) * 0.85;
        let base_z = 0.35 - t * 0.7;  // Spread along trailing edge
        
        // Enhanced feather tip - longer and more dramatic
        // Add slight curve for organic look
        let curve_offset = 0.08 * (t * std::f32::consts::PI).sin();
        let tip_extension = 0.18 + t * 0.12; // Longer feathers toward tip
        let tip_x = (feather_t + tip_extension) * wing_span * mirror;
        let tip_y = base_y * 0.45 - t * 0.25 + curve_offset;
        let tip_z = base_z - 0.25 - t * 0.12;
        
        // Feather width - more tapered
        let feather_width = 0.07 * (1.0 - t * 0.6);
        
        let idx = vertices.len() as u32;
        
        // Feather vertices with enhanced UVs for gradient
        vertices.push([base_x, base_y, base_z]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([feather_t, 0.5 + t * 0.25]); // Gradient toward tip
        
        vertices.push([base_x, base_y - 0.01, base_z]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([feather_t, 0.5 + t * 0.25]);
        
        // Tip vertices with curve
        vertices.push([tip_x, tip_y, tip_z - feather_width]);
        normals.push([0.0, 0.7, -0.3]);
        uvs.push([feather_t + tip_extension, 0.6 + t * 0.25]);
        
        vertices.push([tip_x, tip_y, tip_z + feather_width]);
        normals.push([0.0, 0.7, 0.3]);
        uvs.push([feather_t + tip_extension, 0.4 + t * 0.25]);
        
        // Feather indices (two triangles)
        indices.extend_from_slice(&[
            idx, idx + 2, idx + 3,  // Top
            idx + 1, idx + 3, idx + 2,  // Bottom
        ]);
    }
    
    // ============================================
    // Layer 3: Secondary feathers (mid-wing feathers)
    // Enhanced: 12 feathers with better curve
    // ============================================
    for i in 0..secondary_feather_count {
        let t = i as f32 / secondary_feather_count as f32;
        let feather_t = 0.25 + t * 0.35;  // Secondary feathers in middle section
        
        let base_x = feather_t * wing_span * mirror;
        let base_y = wing_height * (1.0 - feather_t * 0.45) * 0.92;
        let base_z = 0.25 - t * 0.45;
        
        // Add slight curve to secondary feathers
        let curve_offset = 0.05 * (t * std::f32::consts::PI).sin();
        let tip_x = (feather_t + 0.12) * wing_span * mirror;
        let tip_y = base_y * 0.82 + curve_offset;
        let tip_z = base_z + 0.18;
        
        let feather_width = 0.095 * (1.0 - t * 0.35);
        
        let idx = vertices.len() as u32;
        
        vertices.push([base_x, base_y, base_z]);
        normals.push([0.0, 1.0, 0.1]);
        uvs.push([feather_t, 0.35 + t * 0.15]);
        
        vertices.push([base_x, base_y - 0.01, base_z]);
        normals.push([0.0, -1.0, 0.1]);
        uvs.push([feather_t, 0.35 + t * 0.15]);
        
        vertices.push([tip_x, tip_y, tip_z - feather_width]);
        normals.push([0.0, 0.8, 0.0]);
        uvs.push([feather_t + 0.12, 0.45 + t * 0.15]);
        
        vertices.push([tip_x, tip_y, tip_z + feather_width]);
        normals.push([0.0, 0.8, 0.0]);
        uvs.push([feather_t + 0.12, 0.25 + t * 0.15]);
        
        indices.extend_from_slice(&[
            idx, idx + 2, idx + 3,
            idx + 1, idx + 3, idx + 2,
        ]);
    }
    
    // ============================================
    // Layer 4: Tertiary feathers (NEW - small feathers near body)
    // These add fullness near the wing joint
    // ============================================
    for i in 0..tertiary_feather_count {
        let t = i as f32 / tertiary_feather_count as f32;
        let feather_t = 0.08 + t * 0.18;  // Tertiary feathers very close to body
        
        let base_x = feather_t * wing_span * mirror;
        let base_y = wing_height * (1.0 - feather_t * 0.2) * 0.95;
        let base_z = -0.15 + t * 0.2;
        
        // Short, rounded feathers
        let tip_x = (feather_t + 0.06) * wing_span * mirror;
        let tip_y = base_y * 0.92;
        let tip_z = base_z + 0.08;
        
        let feather_width = 0.1 * (1.0 - t * 0.2);
        
        let idx = vertices.len() as u32;
        
        vertices.push([base_x, base_y + 0.025, base_z]);
        normals.push([0.0, 1.0, 0.15]);
        uvs.push([feather_t, 0.15 + t * 0.08]);
        
        vertices.push([base_x, base_y, base_z]);
        normals.push([0.0, 1.0, 0.15]);
        uvs.push([feather_t, 0.15 + t * 0.08]);
        
        vertices.push([tip_x, tip_y + 0.02, tip_z - feather_width]);
        normals.push([0.0, 1.0, 0.1]);
        uvs.push([feather_t + 0.06, 0.2 + t * 0.08]);
        
        vertices.push([tip_x, tip_y + 0.02, tip_z + feather_width]);
        normals.push([0.0, 1.0, 0.1]);
        uvs.push([feather_t + 0.06, 0.1 + t * 0.08]);
        
        // Two triangles for fuller tertiary feathers
        indices.extend_from_slice(&[
            idx, idx + 2, idx + 3,
            idx + 1, idx + 3, idx + 2,
        ]);
    }
    
    // ============================================
    // Layer 5: Covert feathers (small overlapping feathers near body)
    // Enhanced with better coverage
    // ============================================
    let covert_count = 8; // Increased from 6
    
    for i in 0..covert_count {
        let t = i as f32 / covert_count as f32;
        let feather_t = 0.05 + t * 0.28;  // Coverts near body
        
        let base_x = feather_t * wing_span * mirror;
        let base_y = wing_height * (1.0 - feather_t * 0.25);
        let base_z = -0.25 + t * 0.18;
        
        let tip_x = (feather_t + 0.07) * wing_span * mirror;
        let tip_y = base_y * 0.96;
        let tip_z = base_z + 0.12;
        
        let feather_width = 0.11;
        
        let idx = vertices.len() as u32;
        
        vertices.push([base_x, base_y + 0.035, base_z]);
        normals.push([0.0, 1.0, 0.2]);
        uvs.push([feather_t, 0.18 + t * 0.08]);
        
        vertices.push([base_x, base_y, base_z]);
        normals.push([0.0, 1.0, 0.2]);
        uvs.push([feather_t, 0.18 + t * 0.08]);
        
        vertices.push([tip_x, tip_y + 0.025, tip_z]);
        normals.push([0.0, 1.0, 0.1]);
        uvs.push([feather_t + 0.07, 0.22 + t * 0.08]);
        
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
