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
pub mod zone_list_panel;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::components::{EventObject, WarpObject, ZoneObject};
use crate::map_editor::components::SelectedInEditor;
use crate::map_editor::resources::{AvailableModels, EditorMode, HierarchyFilter, MapEditorState, SelectedModel};
use crate::map_editor::systems::property_update_system::PropertyChangeEvent;
use crate::map_editor::save::{SaveZoneEvent, SaveStatus};
use crate::resources::{CurrentZone, GameData};
use crate::events::LoadZoneEvent;

use menu_bar::editor_menu_bar;
use menu_bar::HelpWindowState;
use hierarchy_panel::{editor_hierarchy_panel, HierarchyQuery};
use model_browser_panel::editor_model_browser_panel;
use status_bar::editor_status_bar;
use zone_list_panel::{ZoneListPanelState, zone_list_panel_system};

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
            .init_resource::<ZoneListPanelState>()
            .init_resource::<HelpWindowState>()
            .add_event::<PropertyChangeEvent>()
            .add_event::<NewZoneEvent>()
            .add_systems(
                Update,
                editor_ui_system.run_if(resource_exists::<MapEditorState>),
            )
            .add_systems(
                Update,
                model_browser_panel_system.run_if(resource_exists::<AvailableModels>),
            )
            .add_systems(
                Update,
                model_browser_panel::model_browser_keyboard_shortcuts.run_if(resource_exists::<SelectedModel>),
            )
            .add_systems(
                Update,
                zone_list_panel_system.run_if(resource_exists::<MapEditorState>),
            )
            .add_systems(
                Update,
                new_zone_system.run_if(resource_exists::<MapEditorState>),
            );
        
        log::info!("[EditorUiPlugin] Editor UI plugin initialized with model browser, zone list, and new zone handler");
    }
}

/// Event to request creating a new zone
#[derive(Event)]
pub struct NewZoneEvent {
    /// Whether to prompt for unsaved changes
    pub prompt_if_modified: bool,
}

impl NewZoneEvent {
    pub fn new() -> Self {
        Self {
            prompt_if_modified: true,
        }
    }
}

/// Main UI system that renders all editor panels
///
/// This system only renders when `MapEditorState::enabled` is true.
#[allow(clippy::too_many_arguments)]
pub fn editor_ui_system(
    mut contexts: EguiContexts,
    mut map_editor_state: ResMut<MapEditorState>,
    save_status: Res<SaveStatus>,
    current_zone: Option<Res<CurrentZone>>,
    mut save_events: EventWriter<SaveZoneEvent>,
    entity_data: EntityDataQuery,
    hierarchy_query: HierarchyQuery,
    mut pending_edits: ResMut<PendingPropertyEdits>,
    name_query: Query<&Name>,
    transform_query: Query<&Transform>,
    mut event_writer: EventWriter<PropertyChangeEvent>,
    mut zone_list_state: ResMut<ZoneListPanelState>,
    mut new_zone_events: EventWriter<NewZoneEvent>,
    mut help_state: ResMut<HelpWindowState>,
    mut commands: Commands,
    mut selected_model: ResMut<SelectedModel>,
) {
    // Only render UI when editor is enabled
    if !map_editor_state.enabled {
        return;
    }
    
    let ctx = contexts.ctx_mut();
    
    // Get current zone ID
    let current_zone_id = current_zone.map(|z| z.id.get());
    
    // Menu Bar (top)
    editor_menu_bar(
        ctx,
        &map_editor_state,
        &save_status,
        current_zone_id,
        &mut save_events,
        &mut zone_list_state,
        &mut new_zone_events,
        &mut help_state,
        &mut selected_model,
    );
    
    // Hierarchy Panel (left side) - now with entity query access
    editor_hierarchy_panel(ctx, &map_editor_state, &hierarchy_query, &mut commands);
    
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
    editor_status_bar(ctx, &mut map_editor_state, &save_status, current_zone_id);
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

/// System to handle NewZoneEvent - clears all zone objects and resets editor state
pub fn new_zone_system(
    mut events: EventReader<NewZoneEvent>,
    mut commands: Commands,
    query: Query<Entity, With<ZoneObject>>,
    mut map_editor_state: ResMut<MapEditorState>,
) {
    for event in events.read() {
        // Check if we should prompt for unsaved changes
        if event.prompt_if_modified && map_editor_state.is_modified {
            // For now, just log a warning. In a full implementation,
            // we would show a dialog asking the user to save.
            log::warn!("[NewZone] Zone has unsaved changes, but proceeding with new zone (dialog not implemented)");
        }
        
        // Despawn all zone objects
        let mut despawned_count = 0;
        for entity in query.iter() {
            commands.entity(entity).despawn();
            despawned_count += 1;
        }
        
        // Clear selection and reset modification state
        map_editor_state.clear_selection();
        map_editor_state.is_modified = false;
        map_editor_state.clear_history();
        
        log::info!("[NewZone] Cleared {} zone objects, editor state reset", despawned_count);
    }
}
