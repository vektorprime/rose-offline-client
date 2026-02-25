//! Keyboard Shortcuts System for the Map Editor
//!
//! Handles keyboard shortcuts for common editor operations:
//! - Delete: Delete selected entities
//! - Ctrl+D: Duplicate selected entities
//! - Ctrl+Z: Undo
//! - Ctrl+Y: Redo
//! - Escape: Deselect all
//! - E/R: Switch to Rotate/Scale mode (Q for Select, V for Add, X for Delete)
//! - Tab: Toggle free camera on/off
//! - Note: W is reserved for FreeCamera forward movement

use bevy::prelude::*;
use bevy_egui::EguiContexts;
use std::collections::HashSet;

use crate::map_editor::components::{EditorSelectable, SelectedInEditor};
use crate::map_editor::resources::{EditorAction, EditorMode, MapEditorState};
use crate::systems::{FreeCamera, OrbitCamera};

/// System to handle keyboard shortcuts for the map editor
pub fn keyboard_shortcuts_system(
    mut commands: Commands,
    mut map_editor_state: ResMut<MapEditorState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut egui_contexts: EguiContexts,
    selected_entities: Query<Entity, With<SelectedInEditor>>,
    transforms: Query<&Transform>,
    names: Query<&Name>,
    camera_query: Query<Entity, With<Camera3d>>,
    free_camera_query: Query<&FreeCamera>,
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
    
    // Handle mode switching (W/E/R/Q)
    handle_mode_switches(&mut map_editor_state, &keyboard);
    
    // Handle Escape - Deselect all
    if keyboard.just_pressed(KeyCode::Escape) {
        handle_deselect_all(&mut map_editor_state, &mut commands, &selected_entities);
    }
    
    // Handle Tab - Toggle free camera
    if keyboard.just_pressed(KeyCode::Tab) {
        handle_toggle_free_camera(&mut commands, &camera_query, &free_camera_query);
    }
    
    // Handle Delete - Delete selected entities
    if keyboard.just_pressed(KeyCode::Delete) ||
       (keyboard.just_pressed(KeyCode::Backspace) && keyboard.pressed(KeyCode::ControlLeft)) {
        handle_delete_selected(&mut commands, &mut map_editor_state, &selected_entities, &transforms);
    }
    
    // Handle Ctrl+D - Duplicate selected entities
    if keyboard.just_pressed(KeyCode::KeyD) && is_ctrl_pressed(&keyboard) {
        handle_duplicate_selected(
            commands.reborrow(),
            &mut map_editor_state,
            &selected_entities,
            &transforms,
            &names,
        );
    }
    
    // Handle Ctrl+A - Select all
    if keyboard.just_pressed(KeyCode::KeyA) && is_ctrl_pressed(&keyboard) {
        handle_select_all(&mut map_editor_state);
    }
    
    // Handle Ctrl+Shift+A - Deselect all (alternative)
    if keyboard.just_pressed(KeyCode::KeyA) && is_ctrl_pressed(&keyboard) && is_shift_pressed(&keyboard) {
        handle_deselect_all(&mut map_editor_state, &mut commands, &selected_entities);
    }
    
    // Handle F - Focus on selected entity
    if keyboard.just_pressed(KeyCode::KeyF) {
        handle_focus_selected(&map_editor_state);
    }
    
    // Handle G - Toggle snap to grid
    if keyboard.just_pressed(KeyCode::KeyG) && !is_ctrl_pressed(&keyboard) {
        map_editor_state.snap_to_grid = !map_editor_state.snap_to_grid;
        log::info!("[KeyboardShortcuts] Snap to grid: {}", map_editor_state.snap_to_grid);
    }
    
    // Note: Ctrl+S save functionality is handled via the menu bar UI
    // The keyboard input S (without modifiers) is used for FreeCamera movement
    
    // Handle Ctrl+N - New map (log for now)
    if keyboard.just_pressed(KeyCode::KeyN) && is_ctrl_pressed(&keyboard) {
        log::info!("[KeyboardShortcuts] New map requested (not implemented yet)");
    }
    
    // Handle Ctrl+O - Open map (log for now)
    if keyboard.just_pressed(KeyCode::KeyO) && is_ctrl_pressed(&keyboard) {
        log::info!("[KeyboardShortcuts] Open map requested (not implemented yet)");
    }
}

/// Handle Tab - Toggle free camera on/off
fn handle_toggle_free_camera(
    commands: &mut Commands,
    camera_query: &Query<Entity, With<Camera3d>>,
    free_camera_query: &Query<&FreeCamera>,
) {
    for camera_entity in camera_query.iter() {
        if free_camera_query.get(camera_entity).is_ok() {
            // FreeCamera exists, remove it and add OrbitCamera
            commands.entity(camera_entity)
                .remove::<FreeCamera>()
                .insert(OrbitCamera::new(camera_entity, Vec3::ZERO, 10.0));
            log::info!("[KeyboardShortcuts] Switched to OrbitCamera");
        } else {
            // No FreeCamera, add it and remove OrbitCamera
            commands.entity(camera_entity)
                .remove::<OrbitCamera>()
                .insert(FreeCamera::new(Vec3::new(5120.0, 50.0, -5120.0), -45.0, -20.0));
            log::info!("[KeyboardShortcuts] Switched to FreeCamera");
        }
    }
}

/// Check if Ctrl is pressed (either left or right)
fn is_ctrl_pressed(keyboard: &ButtonInput<KeyCode>) -> bool {
    keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight)
}

/// Check if Shift is pressed (either left or right)
fn is_shift_pressed(keyboard: &ButtonInput<KeyCode>) -> bool {
    keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight)
}

/// Check if Alt is pressed (either left or right)
#[allow(dead_code)]
fn is_alt_pressed(keyboard: &ButtonInput<KeyCode>) -> bool {
    keyboard.pressed(KeyCode::AltLeft) || keyboard.pressed(KeyCode::AltRight)
}

/// Handle mode switching with E/R/Q/V/X keys
/// Note: W is NOT used for Translate mode to avoid conflict with FreeCamera WASD movement
fn handle_mode_switches(map_editor_state: &mut MapEditorState, keyboard: &ButtonInput<KeyCode>) {
    // E for Rotate mode
    if keyboard.just_pressed(KeyCode::KeyE) {
        map_editor_state.editor_mode = EditorMode::Rotate;
        log::info!("[KeyboardShortcuts] Switched to Rotate mode");
    }
    
    // R for Scale mode
    if keyboard.just_pressed(KeyCode::KeyR) {
        map_editor_state.editor_mode = EditorMode::Scale;
        log::info!("[KeyboardShortcuts] Switched to Scale mode");
    }
    
    // Q for Select mode
    if keyboard.just_pressed(KeyCode::KeyQ) {
        map_editor_state.editor_mode = EditorMode::Select;
        log::info!("[KeyboardShortcuts] Switched to Select mode");
    }
    
    // V for Add mode
    if keyboard.just_pressed(KeyCode::KeyV) {
        map_editor_state.editor_mode = EditorMode::Add;
        log::info!("[KeyboardShortcuts] Switched to Add mode");
    }
    
    // X for Delete mode
    if keyboard.just_pressed(KeyCode::KeyX) {
        map_editor_state.editor_mode = EditorMode::Delete;
        log::info!("[KeyboardShortcuts] Switched to Delete mode");
    }
}

/// Handle Escape - Deselect all entities
fn handle_deselect_all(
    map_editor_state: &mut MapEditorState,
    commands: &mut Commands,
    selected_entities: &Query<Entity, With<SelectedInEditor>>,
) {
    let count = map_editor_state.selection_count();
    
    if count > 0 {
        // Remove SelectedInEditor component from all selected entities
        for entity in selected_entities.iter() {
            commands.entity(entity).remove::<SelectedInEditor>();
        }
        
        // Clear the selection set
        map_editor_state.clear_selection();
        
        log::info!("[KeyboardShortcuts] Deselected {} entities", count);
    }
}

/// Handle Delete - Delete selected entities
fn handle_delete_selected(
    commands: &mut Commands,
    map_editor_state: &mut MapEditorState,
    selected_entities: &Query<Entity, With<SelectedInEditor>>,
    transforms: &Query<&Transform>,
) {
    let entities: Vec<Entity> = selected_entities.iter().collect();
    
    if entities.is_empty() {
        return;
    }
    
    // Collect entities with their transforms for undo
    let mut deleted_entities = Vec::new();
    
    for entity in &entities {
        let transform = transforms.get(*entity).ok().copied().unwrap_or_default();
        
        // For a full implementation, we would serialize the entity's components here
        let serialized_data = String::new(); // Placeholder
        
        deleted_entities.push((*entity, transform, "Unknown".to_string(), serialized_data));
    }
    
    // Record the action for undo
    if deleted_entities.len() == 1 {
        let (entity, transform, entity_type, serialized_data) = deleted_entities.into_iter().next().unwrap();
        map_editor_state.push_action(EditorAction::DeleteEntity {
            entity,
            transform,
            entity_type,
            serialized_data,
        });
    } else {
        map_editor_state.push_action(EditorAction::DeleteEntities {
            entities: deleted_entities,
        });
    }
    
    // Despawn all selected entities
    for entity in &entities {
        commands.entity(*entity).despawn_recursive();
    }
    
    // Clear selection
    map_editor_state.clear_selection();
    
    log::info!("[KeyboardShortcuts] Deleted {} entities", entities.len());
}

/// Handle Ctrl+D - Duplicate selected entities
fn handle_duplicate_selected(
    mut commands: Commands,
    map_editor_state: &mut MapEditorState,
    selected_entities: &Query<Entity, With<SelectedInEditor>>,
    transforms: &Query<&Transform>,
    names: &Query<&Name>,
) {
    let entities: Vec<Entity> = selected_entities.iter().collect();
    
    if entities.is_empty() {
        return;
    }
    
    // Offset for duplicated entities
    let duplicate_offset = Vec3::new(1.0, 0.0, 1.0);
    
    // Remove selection from current entities
    for entity in &entities {
        commands.entity(*entity).remove::<SelectedInEditor>();
    }
    
    // Create duplicated entities
    let mut new_entities = Vec::new();
    let mut new_selection = HashSet::new();
    
    for entity in &entities {
        // Get the original transform
        if let Ok(transform) = transforms.get(*entity) {
            let new_transform = Transform::from_translation(transform.translation + duplicate_offset)
                .with_rotation(transform.rotation)
                .with_scale(transform.scale);
            
            // Get the original name
            let name = names.get(*entity).ok()
                .map(|n| format!("{}_copy", n.as_str()))
                .unwrap_or_else(|| "DuplicatedEntity".to_string());
            
            // Create the duplicated entity
            let new_entity = commands.spawn((
                new_transform,
                GlobalTransform::default(),
                Name::new(name),
                EditorSelectable,
                SelectedInEditor,
            )).id();
            
            new_entities.push(new_entity);
            new_selection.insert(new_entity);
        }
    }
    
    // Clear old selection and set new selection
    map_editor_state.clear_selection();
    for entity in &new_selection {
        map_editor_state.select_entity(*entity);
    }
    
    // Record the action for undo
    if !new_entities.is_empty() {
        map_editor_state.push_action(EditorAction::AddEntities {
            entities: new_entities.clone(),
        });
    }
    
    log::info!("[KeyboardShortcuts] Duplicated {} entities", new_entities.len());
}

/// Handle Ctrl+A - Select all entities
fn handle_select_all(map_editor_state: &mut MapEditorState) {
    // For a full implementation, we would:
    // 1. Query all selectable entities (with EditorSelectable component)
    // 2. Add SelectedInEditor component to all
    // 3. Add all to map_editor_state.selected_entities
    
    log::info!("[KeyboardShortcuts] Select all (not fully implemented)");
}

/// Handle F - Focus on selected entity
fn handle_focus_selected(map_editor_state: &MapEditorState) {
    if map_editor_state.selection_count() == 0 {
        return;
    }
    
    // For a full implementation, we would:
    // 1. Get the first selected entity's transform
    // 2. Move the editor camera to focus on it
    
    if let Some(_entity) = map_editor_state.first_selected() {
        log::info!("[KeyboardShortcuts] Focus on selected entity (not fully implemented)");
    }
}

/// System to display keyboard shortcuts help overlay
#[allow(dead_code)]
pub fn keyboard_shortcuts_help_system(
    map_editor_state: Res<MapEditorState>,
    mut egui_contexts: EguiContexts,
) {
    if !map_editor_state.enabled {
        return;
    }
    
    let ctx = egui_contexts.ctx_mut();
    
    // Show help when H is pressed (would need keyboard input)
    // For now, this is a placeholder for a help overlay
    
    egui::Window::new("Keyboard Shortcuts")
        .collapsible(true)
        .default_open(false)
        .show(ctx, |ui| {
            ui.label("Selection:");
            ui.label("  Click - Select object");
            ui.label("  Ctrl+Click - Add to selection");
            ui.label("  Ctrl+A - Select all");
            ui.label("  Escape - Deselect all");
            ui.label("");
            ui.label("Transform Modes:");
            ui.label("  Q - Select mode");
            ui.label("  W - Translate mode");
            ui.label("  E - Rotate mode");
            ui.label("  R - Scale mode");
            ui.label("");
            ui.label("Camera:");
            ui.label("  Tab - Toggle free/orbit camera");
            ui.label("");
            ui.label("Actions:");
            ui.label("  Delete - Delete selected");
            ui.label("  Ctrl+D - Duplicate selected");
            ui.label("  Ctrl+Z - Undo");
            ui.label("  Ctrl+Y - Redo");
            ui.label("  G - Toggle snap to grid");
            ui.label("  F - Focus on selected");
            ui.label("");
            ui.label("File:");
            ui.label("  Ctrl+S - Save");
            ui.label("  Ctrl+O - Open");
            ui.label("  Ctrl+N - New");
        });
}

/// Plugin for keyboard shortcuts system
pub struct KeyboardShortcutsPlugin;

impl Plugin for KeyboardShortcutsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, keyboard_shortcuts_system);
    }
}
