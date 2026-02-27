//! Map Editor Module
//!
//! This module provides a live map editor for the Rose Online client.
//! The editor allows real-time editing of zone objects, terrain, and entity
//! properties through an egui-based interface.
//!
//! # Usage
//!
//! Enable the map editor by passing the `--map-editor` flag when launching
//! the application:
//!
//! ```sh
//! rose-offline-client --map-editor --zone 1
//! ```
//!
//! # Architecture
//!
//! The map editor is organized into the following submodules:
//!
//! - `resources`: State management resources (MapEditorState, etc.)
//! - `components`: Marker components for editor entities
//! - `systems`: Selection, highlighting, grid, transform gizmo, property update, and keyboard shortcuts
//! - `ui`: egui panels (menu bar, hierarchy, properties, status bar)
//! - `save`: Zone save/export functionality
//!
//! # Features
//!
//! ## Phase 2.4 - Property Editing
//! - Transform gizmo manipulation (Translate/Rotate/Scale)
//! - Property panel with component-specific editors
//! - Undo/Redo support via Ctrl+Z/Ctrl+Y
//! - Keyboard shortcuts (W/E/R for modes, Delete, Ctrl+D for duplicate)
//!
//! ## Phase 2.5 - Model Management
//! - Model browser panel with category tabs (Deco, Cnst, Event)
//! - Search/filter models by name
//! - Click-to-place model placement
//! - Model preview at cursor position
//!
//! ## Phase 2.6 - Save Functionality
//! - Save zone to IFO format
//! - Save As for exporting to custom locations
//! - Automatic backup of original files
//! - Save status feedback in UI

pub mod components;
pub mod resources;
pub mod systems;
pub mod ui;
pub mod save;

// Re-export commonly used types for convenience
pub use components::{
    EditorGizmo,
    EditorGrid,
    EditorHandle,
    EditorModified,
    EditorOnly,
    EditorPreview,
    EditorSelectable,
    GizmoType,
    HandleType,
    SelectedInEditor,
};

pub use resources::{
    AvailableModels,
    DeletedZoneObjects,
    EditorGridSettings,
    EditorMode,
    HierarchyFilter,
    MapEditorState,
    ModelCategory,
    ModelInfo,
    SelectedModel,
    SelectionMode,
    TransformSpace,
    ZoneObjectType,
};

pub use save::{
    SaveZoneEvent,
    SaveStatus,
    SavePlugin,
};

use bevy::prelude::*;
use systems::duplicate_system::DuplicateSystemPlugin;
use systems::grid_system::EditorGridPlugin;
use systems::keyboard_shortcuts_system::KeyboardShortcutsPlugin;
use systems::load_models_system;
use systems::model_placement_system::ModelPlacementPlugin;
use systems::property_update_system::PropertyUpdatePlugin;
use systems::selection_highlight_system::SelectionHighlightPlugin;
use systems::selection_system::EditorSelectionPlugin;
use systems::transform_gizmo_system::TransformGizmoPlugin;
use systems::undo_system::UndoRedoPlugin;
use ui::EditorUiPlugin;
use ui::zone_list_panel::ZoneListPanelState;
use crate::systems::{FreeCamera, OrbitCamera};
use crate::animation::CameraAnimation;

/// Plugin for the map editor system
///
/// This plugin registers all resources, components, and systems needed
/// for the map editor functionality.
pub struct MapEditorPlugin;

impl Plugin for MapEditorPlugin {
    fn build(&self, app: &mut App) {
        // Register resources
        app.init_resource::<MapEditorState>()
            .init_resource::<EditorGridSettings>()
            .init_resource::<SelectedModel>()
            .init_resource::<DeletedZoneObjects>();
        
        // Add subsystem plugins
        app.add_plugins(EditorSelectionPlugin)
            .add_plugins(SelectionHighlightPlugin)
            .add_plugins(EditorGridPlugin)
            .add_plugins(EditorUiPlugin)
            // Phase 2.4: Property editing plugins
            .add_plugins(TransformGizmoPlugin)
            .add_plugins(PropertyUpdatePlugin)
            .add_plugins(KeyboardShortcutsPlugin)
            .add_plugins(UndoRedoPlugin)
            // Phase 2.5: Model management plugins
            .add_plugins(ModelPlacementPlugin)
            .add_plugins(DuplicateSystemPlugin)
            // Phase 2.6: Save functionality
            .add_plugins(save::SavePlugin);
        
        // Phase 2.5: Load available models on startup (after GameData is loaded)
        app.add_systems(Update, load_models_system::load_available_models_system);
        
        // Update models when a zone is loaded (fixes empty CNST/DECO tabs)
        app.add_systems(Update, load_models_system::update_models_on_zone_load_system);
        
        // Log plugin initialization
        log::info!("[MapEditorPlugin] Map editor plugin initialized with property editing, model management, and save support");
    }
}

/// System to initialize the map editor when entering MapEditor state
///
/// This system:
/// - Sets the editor state to enabled
/// - Configures the camera for free-cam editing mode
pub fn map_editor_enter_system(
    mut commands: Commands,
    mut map_editor_state: ResMut<MapEditorState>,
    mut zone_list_state: ResMut<ZoneListPanelState>,
    query_cameras: Query<Entity, With<Camera3d>>,
) {
    log::info!("[MapEditor] Entering map editor mode");
    map_editor_state.enabled = true;
    
    // Open zone list panel by default
    zone_list_state.is_open = true;
    
    // Set up camera for map editing - position at zone center
    let camera_position = Vec3::new(5120.0, 50.0, -5120.0);
    let camera_yaw: f32 = -45.0;
    let camera_pitch: f32 = -20.0;
    
    for entity in query_cameras.iter() {
        commands.entity(entity)
            .remove::<OrbitCamera>()  // Remove OrbitCamera to prevent conflict with FreeCamera
            .remove::<CameraAnimation>()
            .insert(FreeCamera::new(camera_position, camera_yaw, camera_pitch));
        
        log::info!("[MapEditor] Camera configured for editing at position {:?}", camera_position);
    }
}

/// System to clean up when exiting MapEditor state
pub fn map_editor_exit_system(
    mut map_editor_state: ResMut<MapEditorState>,
) {
    log::info!("[MapEditor] Exiting map editor mode");
    map_editor_state.enabled = false;
    map_editor_state.clear_selection();
}
