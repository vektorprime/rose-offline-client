//! Undo/Redo System for the Map Editor
//! 
//! Provides undo/redo functionality for editor actions including:
//! - Transform changes
//! - Entity deletion
//! - Entity duplication
//! - Component modifications

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::map_editor::components::{EditorSelectable, SelectedInEditor};
use crate::map_editor::resources::{EditorAction, MapEditorState};

/// Maximum number of undo steps to keep
const MAX_UNDO_STEPS: usize = 50;

/// System to handle undo/redo keyboard shortcuts
pub fn undo_redo_system(
    mut map_editor_state: ResMut<MapEditorState>,
    mut transforms: Query<&mut Transform>,
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut egui_contexts: EguiContexts,
) {
    // Don't process if editor is disabled
    if !map_editor_state.enabled {
        return;
    }
    
    // Check if egui wants keyboard input
    let ctx = egui_contexts.ctx_mut();
    if ctx.wants_keyboard_input() {
        return;
    }
    
    let ctrl_pressed = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    let shift_pressed = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    
    // Handle Ctrl+Z (undo) - but not Ctrl+Shift+Z
    if keyboard.just_pressed(KeyCode::KeyZ) && ctrl_pressed && !shift_pressed {
        if let Some(action) = map_editor_state.pop_undo() {
            apply_undo(&mut commands, &mut transforms, action, &mut map_editor_state);
            log::info!("[UndoRedo] Undo applied, {} steps remaining", map_editor_state.undo_stack.len());
        } else {
            log::info!("[UndoRedo] Nothing to undo");
        }
    }
    
    // Handle Ctrl+Y (redo)
    if keyboard.just_pressed(KeyCode::KeyY) && ctrl_pressed && !shift_pressed {
        if let Some(action) = map_editor_state.pop_redo() {
            apply_redo(&mut commands, &mut transforms, action, &mut map_editor_state);
            log::info!("[UndoRedo] Redo applied, {} steps remaining", map_editor_state.redo_stack.len());
        } else {
            log::info!("[UndoRedo] Nothing to redo");
        }
    }
    
    // Handle Ctrl+Shift+Z (redo - alternative)
    if keyboard.just_pressed(KeyCode::KeyZ) && ctrl_pressed && shift_pressed {
        if let Some(action) = map_editor_state.pop_redo() {
            apply_redo(&mut commands, &mut transforms, action, &mut map_editor_state);
            log::info!("[UndoRedo] Redo applied (Ctrl+Shift+Z), {} steps remaining", map_editor_state.redo_stack.len());
        }
    }
}

/// Apply an undo action
fn apply_undo(
    commands: &mut Commands,
    transforms: &mut Query<&mut Transform>,
    action: EditorAction,
    map_editor_state: &mut MapEditorState,
) {
    match action {
        EditorAction::TransformEntity {
            entity,
            old_transform,
            new_transform,
        } => {
            if let Ok(mut transform) = transforms.get_mut(entity) {
                *transform = old_transform;
                
                // Push to redo stack (without clearing it)
                map_editor_state.push_redo(EditorAction::TransformEntity {
                    entity,
                    old_transform,
                    new_transform,
                });
                
                log::info!("[UndoRedo] Undid transform for entity {:?}", entity);
            }
        }
        
        EditorAction::TransformEntities { entities } => {
            let mut redo_entities = Vec::new();
            for (entity, old_transform, new_transform) in entities {
                if let Ok(mut transform) = transforms.get_mut(entity) {
                    *transform = old_transform;
                    redo_entities.push((entity, old_transform, new_transform));
                }
            }
            if !redo_entities.is_empty() {
                let count = redo_entities.len();
                map_editor_state.push_redo(EditorAction::TransformEntities {
                    entities: redo_entities,
                });
                log::info!("[UndoRedo] Undid transform for {} entities", count);
            }
        }
        
        EditorAction::AddEntity { entity } => {
            // Undo add = delete the entity
            commands.entity(entity).despawn_recursive();
            map_editor_state.deselect_entity(entity);
            map_editor_state.push_redo(EditorAction::AddEntity { entity });
            log::info!("[UndoRedo] Undid entity addition (despawned {:?})", entity);
        }
        
        EditorAction::AddEntities { entities } => {
            for entity in &entities {
                commands.entity(*entity).despawn_recursive();
                map_editor_state.deselect_entity(*entity);
            }
            map_editor_state.push_redo(EditorAction::AddEntities { entities: entities.clone() });
            log::info!("[UndoRedo] Undid addition of {} entities", entities.len());
        }
        
        EditorAction::DeleteEntity {
            entity: _,
            transform,
            entity_type,
            serialized_data,
        } => {
            // Undo delete = recreate entity
            // Note: Full recreation requires deserialization of stored data
            // For now, we create a placeholder with the original transform
            let new_entity = commands.spawn((
                Transform::from_translation(transform.translation)
                    .with_rotation(transform.rotation)
                    .with_scale(transform.scale),
                GlobalTransform::default(),
                Name::new(format!("Restored_{}", entity_type)),
                EditorSelectable,
            )).id();
            
            // Select the restored entity
            commands.entity(new_entity).insert(SelectedInEditor);
            map_editor_state.select_entity(new_entity);
            
            log::info!(
                "[UndoRedo] Undid entity deletion (created placeholder for type {})",
                entity_type
            );
            
            // Store the redo action with the new entity
            map_editor_state.push_redo(EditorAction::DeleteEntity {
                entity: new_entity,
                transform,
                entity_type,
                serialized_data,
            });
        }
        
        EditorAction::DeleteEntities { entities } => {
            let mut redo_entities = Vec::new();
            for (old_entity, transform, entity_type, serialized_data) in entities {
                // Recreate each entity as a placeholder
                let new_entity = commands.spawn((
                    Transform::from_translation(transform.translation)
                        .with_rotation(transform.rotation)
                        .with_scale(transform.scale),
                    GlobalTransform::default(),
                    Name::new(format!("Restored_{}", entity_type)),
                    EditorSelectable,
                )).id();
                
                commands.entity(new_entity).insert(SelectedInEditor);
                map_editor_state.select_entity(new_entity);
                
                redo_entities.push((new_entity, transform, entity_type, serialized_data));
            }
            
            if !redo_entities.is_empty() {
                let count = redo_entities.len();
                map_editor_state.push_redo(EditorAction::DeleteEntities { entities: redo_entities });
                log::info!("[UndoRedo] Undid deletion of {} entities", count);
            }
        }
        
        EditorAction::ModifyComponent {
            entity,
            component_type,
            old_value,
            new_value,
        } => {
            // Component modification undo would require component-specific handling
            // This is a placeholder that swaps old and new values
            log::info!(
                "[UndoRedo] Undid component {} modification for {:?}: {} <- {}",
                component_type,
                entity,
                old_value,
                new_value
            );
            
            // Push to redo with swapped values
            map_editor_state.push_redo(EditorAction::ModifyComponent {
                entity,
                component_type,
                old_value: new_value,
                new_value: old_value,
            });
        }
    }
}

/// Apply a redo action
fn apply_redo(
    commands: &mut Commands,
    transforms: &mut Query<&mut Transform>,
    action: EditorAction,
    map_editor_state: &mut MapEditorState,
) {
    match action {
        EditorAction::TransformEntity {
            entity,
            old_transform,
            new_transform,
        } => {
            if let Ok(mut transform) = transforms.get_mut(entity) {
                *transform = new_transform;
                
                // Push back to undo stack
                // Note: We directly manipulate the undo stack to avoid clearing redo
                if map_editor_state.undo_stack.len() >= MAX_UNDO_STEPS {
                    map_editor_state.undo_stack.remove(0);
                }
                map_editor_state.undo_stack.push(EditorAction::TransformEntity {
                    entity,
                    old_transform,
                    new_transform,
                });
                
                log::info!("[UndoRedo] Redid transform for entity {:?}", entity);
            }
        }
        
        EditorAction::TransformEntities { entities } => {
            let mut undo_entities = Vec::new();
            for (entity, old_transform, new_transform) in entities {
                if let Ok(mut transform) = transforms.get_mut(entity) {
                    *transform = new_transform;
                    undo_entities.push((entity, old_transform, new_transform));
                }
            }
            if !undo_entities.is_empty() {
                let count = undo_entities.len();
                if map_editor_state.undo_stack.len() >= MAX_UNDO_STEPS {
                    map_editor_state.undo_stack.remove(0);
                }
                map_editor_state.undo_stack.push(EditorAction::TransformEntities {
                    entities: undo_entities,
                });
                log::info!("[UndoRedo] Redid transform for {} entities", count);
            }
        }
        
        EditorAction::AddEntity { entity } => {
            // Redo add = entity should be respawned
            // Note: This requires storing enough data to recreate the entity
            log::info!("[UndoRedo] Redo AddEntity for {:?} (entity recreation needed)", entity);
        }
        
        EditorAction::AddEntities { entities } => {
            log::info!("[UndoRedo] Redo AddEntities for {} entities (entity recreation needed)", entities.len());
        }
        
        EditorAction::DeleteEntity { entity, .. } => {
            // Redo delete = despawn the entity
            commands.entity(entity).despawn_recursive();
            map_editor_state.deselect_entity(entity);
            log::info!("[UndoRedo] Redid entity deletion (despawned {:?})", entity);
        }
        
        EditorAction::DeleteEntities { entities } => {
            for (entity, ..) in &entities {
                commands.entity(*entity).despawn_recursive();
                map_editor_state.deselect_entity(*entity);
            }
            log::info!("[UndoRedo] Redid deletion of {} entities", entities.len());
        }
        
        EditorAction::ModifyComponent {
            entity,
            component_type,
            old_value,
            new_value,
        } => {
            log::info!(
                "[UndoRedo] Redid component {} modification for {:?}: {} -> {}",
                component_type,
                entity,
                old_value,
                new_value
            );
            
            // Push back to undo
            if map_editor_state.undo_stack.len() >= MAX_UNDO_STEPS {
                map_editor_state.undo_stack.remove(0);
            }
            map_editor_state.undo_stack.push(EditorAction::ModifyComponent {
                entity,
                component_type,
                old_value,
                new_value,
            });
        }
    }
}

/// Plugin for the undo/redo system
pub struct UndoRedoPlugin;

impl Plugin for UndoRedoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, undo_redo_system);
    }
}
