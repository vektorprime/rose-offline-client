//! Map Editor UI Module
//!
//! This module contains egui-based UI panels for the map editor.
//!
//! # Panel Layout
//!
//! ```text
//! +--------------------------------------------------+
//! | Menu Bar                                         |
//! +------------+---------------------+---------------+
//! | Hierarchy  |                     | Properties    |
//! | Panel      |    3D Viewport      | Panel         |
//! | (Left)     |                     | (Right)       |
//! |            |                     |               |
//! +------------+---------------------+---------------+
//! | Model Browser Panel                              |
//! +--------------------------------------------------+
//! | Status Bar                                       |
//! +--------------------------------------------------+
//! ```

pub mod menu_bar;
pub mod hierarchy_panel;
pub mod model_browser_panel;
pub mod properties_panel;
pub mod status_bar;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::components::{EventObject, WarpObject, ZoneObject};
use crate::map_editor::components::SelectedInEditor;
use crate::map_editor::resources::{AvailableModels, EditorMode, HierarchyFilter, MapEditorState, SelectedModel};
use crate::map_editor::systems::property_update_system::PropertyChangeEvent;
use crate::map_editor::save::{SaveZoneEvent, SaveStatus};
use crate::resources::CurrentZone;

use menu_bar::editor_menu_bar;
use hierarchy_panel::editor_hierarchy_panel;
use model_browser_panel::editor_model_browser_panel;
use status_bar::editor_status_bar;

// Re-export the standalone properties panel function
pub use properties_panel::{
    editor_properties_panel, EntityDataQuery, PendingPropertyEdits,
};

/// Plugin for the map editor UI systems
pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingPropertyEdits>()
            .init_resource::<SelectedModel>()
            .add_event::<PropertyChangeEvent>()
            .add_systems(
                Update,
                editor_ui_system.run_if(resource_exists::<MapEditorState>),
            )
            .add_systems(
                Update,
                model_browser_panel_system.run_if(resource_exists::<AvailableModels>),
            );
        
        log::info!("[EditorUiPlugin] Editor UI plugin initialized with model browser");
    }
}

/// Main UI system that renders all editor panels
///
/// This system only renders when `MapEditorState::enabled` is true.
pub fn editor_ui_system(
    mut contexts: EguiContexts,
    map_editor_state: Res<MapEditorState>,
    save_status: Res<SaveStatus>,
    current_zone: Option<Res<CurrentZone>>,
    mut save_events: EventWriter<SaveZoneEvent>,
    entity_data: EntityDataQuery,
    mut pending_edits: ResMut<PendingPropertyEdits>,
    name_query: Query<&Name>,
    transform_query: Query<&Transform>,
    mut event_writer: EventWriter<PropertyChangeEvent>,
) {
    // Only render UI when editor is enabled
    if !map_editor_state.enabled {
        return;
    }
    
    let ctx = contexts.ctx_mut();
    
    // Get current zone ID
    let current_zone_id = current_zone.map(|z| z.id.get());
    
    // Menu Bar (top)
    editor_menu_bar(ctx, &map_editor_state, &save_status, current_zone_id, &mut save_events);
    
    // Hierarchy Panel (left side)
    editor_hierarchy_panel(ctx, &map_editor_state);
    
    // Properties Panel (right side) - now with entity data access
    editor_properties_panel(
        ctx,
        &map_editor_state,
        &entity_data,
        &mut pending_edits,
        &name_query,
        &transform_query,
        &mut event_writer,
    );
    
    // Status Bar (bottom)
    editor_status_bar(ctx, &map_editor_state, &save_status);
}

/// System to render the model browser panel
pub fn model_browser_panel_system(
    mut contexts: EguiContexts,
    map_editor_state: Res<MapEditorState>,
    available_models: Res<AvailableModels>,
    mut selected_model: ResMut<SelectedModel>,
) {
    // Only render when editor is enabled
    if !map_editor_state.enabled {
        return;
    }
    
    let ctx = contexts.ctx_mut();
    
    editor_model_browser_panel(
        ctx,
        &map_editor_state,
        &available_models,
        &mut selected_model,
    );
}
