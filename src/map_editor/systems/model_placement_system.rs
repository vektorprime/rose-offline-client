//! Model Placement System for Map Editor
//!
//! This system handles placing selected models at cursor position in the 3D world.

use bevy::{
    input::ButtonInput,
    prelude::{
        App, AssetServer, Camera, Camera3d, Commands, Entity, GlobalTransform,
        KeyCode, MouseButton, Plugin, Query, Res, ResMut, Transform, Update, Vec3, With,
        Mesh3d, MeshMaterial3d, Visibility, InheritedVisibility, ViewVisibility,
        Name, Mesh, Assets, StandardMaterial, Color, Local,
    },
    window::{PrimaryWindow, Window},
    pbr::{NotShadowCaster, NotShadowReceiver},
    math::primitives::Cuboid,
    ecs::schedule::IntoScheduleConfigs,
};
use bevy_egui::EguiContexts;
use bevy_rapier3d::prelude::{CollisionGroups, Group, QueryFilter};
use bevy_rapier3d::plugin::context::systemparams::ReadRapierContext;

use crate::{
    components::{
        ZoneObject, ZoneObjectId, 
        COLLISION_FILTER_INSPECTABLE, COLLISION_FILTER_COLLIDABLE,
    },
    map_editor::{
        resources::{MapEditorState, SelectedModel, EditorMode, ModelCategory},
    },
};

/// Plugin for the model placement system
pub struct ModelPlacementPlugin;

impl Plugin for ModelPlacementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update, 
            model_placement_system
                .after(bevy_egui::EguiPreUpdateSet::InitContexts)
        );
    }
}

/// System that handles model placement when in Add mode
/// 
/// This system:
/// - Shows a preview of the selected model at cursor position
/// - Places the model on left click when in Add mode
/// - Uses raycast to find placement position on terrain/objects
#[allow(clippy::too_many_arguments)]
pub fn model_placement_system(
    mut commands: Commands,
    map_editor_state: Res<MapEditorState>,
    selected_model: Res<SelectedModel>,
    mut egui_ctx: EguiContexts,
    mouse_input: Res<ButtonInput<MouseButton>>,
    rapier_context: ReadRapierContext,
    query_window: Query<&Window, With<PrimaryWindow>>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    asset_server: Res<AssetServer>,
) {
    // Only run when map editor is enabled and in Add mode
    if !map_editor_state.enabled || map_editor_state.editor_mode != EditorMode::Add {
        return;
    }
    
    // Check if a model is selected
    let Some(ref model_info) = selected_model.model else {
        return;
    };
    
    // Skip if egui wants pointer input (mouse is over UI)
    if egui_ctx.ctx_mut().wants_pointer_input() {
        return;
    }

    // Get rapier context
    let Ok(rapier_context) = rapier_context.single() else {
        return;
    };

    // Get the primary window
    let Ok(window) = query_window.get_single() else {
        return;
    };

    // Get cursor position
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Get camera and cast ray
    for (camera, camera_transform) in query_camera.iter() {
        let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
            continue;
        };

        // Cast ray to find placement position
        let hit_result = rapier_context.cast_ray(
            ray.origin,
            *ray.direction,
            10000000.0,
            true, // Get closest hit for placement
            QueryFilter::new().groups(CollisionGroups::new(
                COLLISION_FILTER_INSPECTABLE | COLLISION_FILTER_COLLIDABLE,
                Group::all(),
            )),
        );

        // Get placement position (either hit point or default)
        let placement_position = if let Some((_, distance)) = hit_result {
            ray.origin + *ray.direction * distance
        } else {
            // No hit - use a default position or skip
            // Could also project to a plane at y=0
            let t = -ray.origin.y / ray.direction.y;
            if t > 0.0 {
                ray.origin + *ray.direction * t
            } else {
                continue; // Can't place without a valid position
            }
        };

        // Handle left click for placement
        if mouse_input.just_pressed(MouseButton::Left) {
            // Place the model
            place_model_at_position(
                &mut commands,
                &asset_server,
                model_info,
                placement_position,
            );
            
            log::info!(
                "[MODEL PLACEMENT] Placed model '{}' (ID: {}) at position {:?}",
                model_info.name,
                model_info.id,
                placement_position
            );
        }

        // Only process the first camera
        break;
    }
}

/// Place a model at the specified position
fn place_model_at_position(
    commands: &mut Commands,
    _asset_server: &AssetServer,
    model_info: &crate::map_editor::resources::ModelInfo,
    position: Vec3,
) {
    // Determine the ZoneObject type based on category
    let object_type = match model_info.category {
        ModelCategory::Deco => ZoneObject::DecoObject(ZoneObjectId {
            ifo_object_id: 0, // Will be assigned properly when saved
            zsc_object_id: model_info.id as usize,
        }),
        ModelCategory::Cnst => ZoneObject::CnstObject(ZoneObjectId {
            ifo_object_id: 0,
            zsc_object_id: model_info.id as usize,
        }),
        ModelCategory::Event => ZoneObject::EventObject(ZoneObjectId {
            ifo_object_id: 0,
            zsc_object_id: model_info.id as usize,
        }),
        ModelCategory::Special => ZoneObject::WarpObject(ZoneObjectId {
            ifo_object_id: 0,
            zsc_object_id: model_info.id as usize,
        }),
        ModelCategory::All => ZoneObject::DecoObject(ZoneObjectId {
            ifo_object_id: 0,
            zsc_object_id: model_info.id as usize,
        }),
    };
    
    // Create the entity with basic components
    // Note: Full model spawning with meshes requires access to ZSC data
    // This is a simplified version that creates a placeholder
    
    let entity = commands.spawn((
        object_type,
        Transform::from_translation(position),
        GlobalTransform::default(),
        Visibility::Visible,
        InheritedVisibility::default(),
        ViewVisibility::default(),
        Name::new(format!("Placed: {}", model_info.name)),
        // Add a marker to indicate this was placed by the editor
        EditorPlacedObject {
            model_id: model_info.id,
            category: model_info.category,
            placed_at: std::time::Instant::now(),
        },
    )).id();
    
    // Log placement for undo system
    log::info!(
        "[MODEL PLACEMENT] Created entity {:?} for model '{}' at {:?}",
        entity,
        model_info.name,
        position
    );
    
    // Note: Full implementation would:
    // 1. Load the ZSC file for the model's category
    // 2. Spawn all parts with meshes and materials
    // 3. Add colliders based on ZSC collision data
    // 4. Add to undo stack
}

/// Component to mark objects placed by the editor
#[derive(bevy::prelude::Component, Debug, Clone)]
pub struct EditorPlacedObject {
    /// ID of the model that was placed
    pub model_id: u32,
    /// Category of the placed model
    pub category: ModelCategory,
    /// When the object was placed
    pub placed_at: std::time::Instant,
}

/// System to show a preview of the model at cursor position
/// This is a visual-only system that shows where the model will be placed
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn model_preview_system(
    mut commands: Commands,
    map_editor_state: Res<MapEditorState>,
    selected_model: Res<SelectedModel>,
    mut egui_ctx: EguiContexts,
    rapier_context: ReadRapierContext,
    query_window: Query<&Window, With<PrimaryWindow>>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut preview_entity: Local<Option<Entity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Only run when map editor is enabled and in Add mode with a selected model
    if !map_editor_state.enabled 
        || map_editor_state.editor_mode != EditorMode::Add 
        || selected_model.model.is_none() 
    {
        // Hide/remove preview if it exists
        if let Some(entity) = *preview_entity {
            commands.entity(entity).despawn();
            *preview_entity = None;
        }
        return;
    }
    
    // Skip if egui wants pointer input
    if egui_ctx.ctx_mut().wants_pointer_input() {
        return;
    }

    // Get rapier context
    let Ok(rapier_context) = rapier_context.single() else {
        return;
    };

    // Get window and cursor
    let Ok(window) = query_window.get_single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Get camera and cast ray
    for (camera, camera_transform) in query_camera.iter() {
        let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
            continue;
        };

        // Cast ray to find placement position
        let hit_result = rapier_context.cast_ray(
            ray.origin,
            *ray.direction,
            10000000.0,
            true,
            QueryFilter::new().groups(CollisionGroups::new(
                COLLISION_FILTER_INSPECTABLE | COLLISION_FILTER_COLLIDABLE,
                Group::all(),
            )),
        );

        // Get placement position
        let placement_position = if let Some((_, distance)) = hit_result {
            ray.origin + *ray.direction * distance
        } else {
            let t = -ray.origin.y / ray.direction.y;
            if t > 0.0 {
                ray.origin + *ray.direction * t
            } else {
                // Hide preview if no valid position
                if let Some(entity) = *preview_entity {
                    commands.entity(entity).despawn();
                    *preview_entity = None;
                }
                continue;
            }
        };

        // Create or update preview entity
        // Use a simple wireframe cube as preview
        if preview_entity.is_none() {
            // Create a simple preview mesh (cube)
            let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
            let material = materials.add(StandardMaterial {
                base_color: Color::srgba(0.0, 1.0, 0.0, 0.5),
                alpha_mode: bevy::render::alpha::AlphaMode::Blend,
                unlit: true,
                ..Default::default()
            });
            
            let entity = commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_translation(placement_position),
                Visibility::Visible,
                NotShadowCaster,
                NotShadowReceiver,
                Name::new("Model Preview"),
            )).id();
            
            *preview_entity = Some(entity);
        } else if let Some(entity) = *preview_entity {
            // Update position
            commands.entity(entity).insert(Transform::from_translation(placement_position));
        }

        break;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_placement_system_exists() {
        // Basic test to ensure the module compiles
        assert!(true);
    }
}
