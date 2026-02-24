//! Editor Selection System
//! 
//! This module provides raycast-based entity picking for the map editor.
//! It handles click-based selection with multi-select support via Ctrl modifier.

use bevy::{
    input::ButtonInput,
    prelude::{
        App, Camera, Camera3d, Commands, Entity, GlobalTransform, IntoScheduleConfigs, KeyCode, 
        MouseButton, Plugin, Query, Res, ResMut, Update, With, Added, Or, Without,
    },
    window::{PrimaryWindow, Window},
};
use bevy_egui::EguiContexts;
use bevy_rapier3d::prelude::{CollisionGroups, Group, QueryFilter};
use bevy_rapier3d::plugin::context::systemparams::ReadRapierContext;

use crate::{
    components::{COLLISION_FILTER_INSPECTABLE, ColliderParent},
    map_editor::{
        components::{EditorSelectable, SelectedInEditor},
        resources::MapEditorState,
    },
};

/// Plugin for the editor selection system
pub struct EditorSelectionPlugin;

impl Plugin for EditorSelectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update, 
            editor_picking_system.after(bevy_egui::EguiPreUpdateSet::InitContexts)
        );
    }
}

/// System that handles entity picking via raycast from mouse position
/// 
/// This system:
/// - Casts a ray from the camera through the mouse position
/// - Uses Rapier3D raycast to detect hits
/// - Filters to only select entities with colliders in the INSPECTABLE group
/// - Supports multi-select with Ctrl+click
/// - Updates the MapEditorState with selection changes
#[allow(clippy::too_many_arguments)]
pub fn editor_picking_system(
    mut commands: Commands,
    mut map_editor_state: ResMut<MapEditorState>,
    mut egui_ctx: EguiContexts,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    rapier_context: ReadRapierContext,
    query_window: Query<&Window, With<PrimaryWindow>>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    query_collider_parent: Query<&ColliderParent>,
    query_selectable: Query<&EditorSelectable>,
    query_selected: Query<Entity, With<SelectedInEditor>>,
) {
    // Only run when map editor is enabled
    if !map_editor_state.enabled {
        return;
    }

    // Get rapier context
    let Ok(rapier_context) = rapier_context.single() else {
        return;
    };

    // Skip if egui wants pointer input (mouse is over UI)
    if egui_ctx.ctx_mut().wants_pointer_input() {
        return;
    }

    // Get the primary window
    let Ok(window) = query_window.get_single() else {
        return;
    };

    // Get cursor position
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Handle left click for selection
    if mouse_input.just_pressed(MouseButton::Left) {
        // Get camera and cast ray
        for (camera, camera_transform) in query_camera.iter() {
            let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
                continue;
            };

            // Cast ray to find entity
            let hit_result = rapier_context.cast_ray(
                ray.origin,
                *ray.direction,
                10000000.0,
                false,
                QueryFilter::new().groups(CollisionGroups::new(
                    COLLISION_FILTER_INSPECTABLE,
                    Group::all(),
                )),
            );

            if let Some((hit_entity, _distance)) = hit_result {
                // The ray hit a collider entity. We need to find the parent entity
                // which is the actual game object (not the collider child)
                let target_entity = if let Ok(collider_parent) = query_collider_parent.get(hit_entity) {
                    collider_parent.entity
                } else {
                    // If no ColliderParent, the collider is on the main entity itself
                    hit_entity
                };

                // Check if the entity is selectable in the editor
                let is_selectable = query_selectable.get(target_entity).is_ok();

                // For now, allow selection of all entities with colliders
                // The EditorSelectable component can be used to filter if needed
                let _ = is_selectable; // Acknowledge the variable

                // Handle multi-select with Ctrl
                let ctrl_pressed = keyboard.pressed(KeyCode::ControlLeft) 
                    || keyboard.pressed(KeyCode::ControlRight);

                if ctrl_pressed {
                    // Toggle selection
                    if map_editor_state.selected_entities.contains(&target_entity) {
                        map_editor_state.deselect_entity(target_entity);
                        // Remove SelectedInEditor marker
                        commands.entity(target_entity).remove::<SelectedInEditor>();
                    } else {
                        map_editor_state.select_entity(target_entity);
                        // Add SelectedInEditor marker
                        commands.entity(target_entity).insert(SelectedInEditor);
                    }
                } else {
                    // Single selection - clear previous and select new
                    // First, remove SelectedInEditor from all currently selected entities
                    for entity in query_selected.iter() {
                        commands.entity(entity).remove::<SelectedInEditor>();
                    }
                    
                    // Clear selection state
                    map_editor_state.clear_selection();
                    
                    // Select the new entity
                    map_editor_state.select_entity(target_entity);
                    commands.entity(target_entity).insert(SelectedInEditor);
                }

                log::debug!(
                    "[MapEditor] Selected entity: {:?}, total selected: {}",
                    target_entity,
                    map_editor_state.selection_count()
                );
            } else {
                // Clicked empty space - clear selection (unless Ctrl is held)
                if !keyboard.pressed(KeyCode::ControlLeft) 
                    && !keyboard.pressed(KeyCode::ControlRight) 
                {
                    // Remove SelectedInEditor from all currently selected entities
                    for entity in query_selected.iter() {
                        commands.entity(entity).remove::<SelectedInEditor>();
                    }
                    
                    map_editor_state.clear_selection();
                    log::debug!("[MapEditor] Selection cleared");
                }
            }

            // Only process the first camera
            break;
        }
    }

    // Handle Escape key to deselect all
    if keyboard.just_pressed(KeyCode::Escape) {
        for entity in query_selected.iter() {
            commands.entity(entity).remove::<SelectedInEditor>();
        }
        map_editor_state.clear_selection();
        log::debug!("[MapEditor] Selection cleared via Escape");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_system_exists() {
        // Basic test to ensure the module compiles
        assert!(true);
    }
}
