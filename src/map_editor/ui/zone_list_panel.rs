//! Zone List Panel for the Map Editor
//! 
//! Provides a panel for switching between zones in the map editor.
//! Based on the zone viewer's ui_debug_zone_list_system.rs

use bevy::prelude::{EventWriter, Res, ResMut, Resource};
use bevy_egui::{egui, EguiContexts};
use regex::Regex;

use rose_data::ZoneId;

use crate::{
    events::LoadZoneEvent,
    resources::{GameData, CurrentZone},
};

/// State for the zone list panel
#[derive(Resource)]
pub struct ZoneListPanelState {
    /// Filter text for searching zones
    pub filter_name: String,
    /// Cached list of filtered zone IDs
    pub filtered_zones: Vec<ZoneId>,
    /// Whether the filter needs to be re-applied
    pub filter_dirty: bool,
    /// Whether the panel is open
    pub is_open: bool,
    /// Whether to despawn other zones when loading a new one
    pub despawn_other_zones: bool,
    /// Whether the initial zone list has been loaded
    pub initialized: bool,
}

impl Default for ZoneListPanelState {
    fn default() -> Self {
        Self {
            filter_name: String::new(),
            filtered_zones: Vec::new(),
            filter_dirty: true, // Important: start true so initial list is populated
            is_open: false,
            despawn_other_zones: true,
            initialized: false,
        }
    }
}

impl ZoneListPanelState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Render the zone list panel
#[allow(clippy::too_many_arguments)]
pub fn editor_zone_list_panel(
    ctx: &egui::Context,
    state: &mut ZoneListPanelState,
    game_data: &GameData,
    current_zone: Option<&CurrentZone>,
    load_zone_events: &mut EventWriter<LoadZoneEvent>,
) {
    if !state.is_open {
        return;
    }
    
    // Update filtered zones if needed (before the window to avoid borrow issues)
    if state.filter_dirty {
        update_filtered_zones(state, game_data);
        state.filter_dirty = false;
    }
    
    let mut is_open = state.is_open;
    let current_zone_id = current_zone.map(|c| c.id);
    
    egui::Window::new("Open Zone")
        .open(&mut is_open)
        .resizable(true)
        .default_width(400.0)
        .default_height(500.0)
        .show(ctx, |ui| {
            // Filter input
            ui.horizontal(|ui| {
                ui.label("Search:");
                if ui.text_edit_singleline(&mut state.filter_name).changed() {
                    state.filter_dirty = true;
                }
                if ui.button("Clear").clicked() {
                    state.filter_name.clear();
                    state.filter_dirty = true;
                }
            });
            
            // Despawn option
            ui.horizontal(|ui| {
                ui.label("Despawn other zones:");
                ui.checkbox(&mut state.despawn_other_zones, "Enable");
            });
            
            ui.separator();
            
            // Show current zone info
            if let Some(current) = current_zone_id {
                let zone_name = game_data
                    .zone_list
                    .get_zone(current)
                    .map(|z| z.name)
                    .unwrap_or("Unknown");
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Current Zone: {} ({})", 
                        zone_name,
                        current.get()
                    )).color(egui::Color32::LIGHT_BLUE));
                });
                ui.separator();
            }
            
            // Zone list table
            let filtered_zones = state.filtered_zones.clone();
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(egui_extras::Column::initial(50.0).at_least(50.0)) // ID
                .column(egui_extras::Column::remainder().at_least(150.0))  // Name
                .column(egui_extras::Column::initial(80.0).at_least(80.0)) // Action
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("ID");
                    });
                    header.col(|ui| {
                        ui.heading("Zone Name");
                    });
                    header.col(|ui| {
                        ui.heading("Action");
                    });
                })
                .body(|body| {
                    body.rows(24.0, filtered_zones.len(), |mut row| {
                        if let Some(&zone_id) = filtered_zones.get(row.index()) {
                            if let Some(zone_data) = game_data.zone_list.get_zone(zone_id) {
                                row.col(|ui| {
                                    ui.label(format!("{}", zone_data.id.get()));
                                });
                                
                                row.col(|ui| {
                                    ui.label(zone_data.name);
                                });
                                
                                row.col(|ui| {
                                    // Highlight current zone
                                    let is_current = current_zone_id
                                        .map(|c| c == zone_id)
                                        .unwrap_or(false);
                                    
                                    if is_current {
                                        // Show a styled "Current" label for the current zone
                                        ui.add_enabled(
                                            false,
                                            egui::Button::new(
                                                egui::RichText::new("Current").color(egui::Color32::GRAY)
                                            )
                                        );
                                    } else {
                                        // Show a clickable "Load" button for other zones
                                        if ui.button("Load").clicked() {
                                            log::info!(
                                                "[MapEditor] Loading zone {} ({}) with despawn_other_zones={}",
                                                zone_data.id.get(),
                                                zone_data.name,
                                                state.despawn_other_zones
                                            );
                                            load_zone_events.write(LoadZoneEvent {
                                                id: zone_data.id,
                                                despawn_other_zones: state.despawn_other_zones,
                                            });
                                            state.is_open = false;
                                        }
                                    }
                                });
                            }
                        }
                    });
                });
            
            ui.separator();
            
            // Footer with count
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Showing {} of {} zones",
                    state.filtered_zones.len(),
                    game_data.zone_list.len()
                ));
            });
        });
    
    state.is_open = is_open;
}

/// Update the filtered zones list based on the current filter
fn update_filtered_zones(state: &mut ZoneListPanelState, game_data: &GameData) {
    let filter_re = if !state.filter_name.is_empty() {
        Regex::new(&format!("(?i){}", regex::escape(&state.filter_name))).ok()
    } else {
        None
    };
    
    state.filtered_zones = game_data
        .zone_list
        .iter()
        .filter_map(|zone_data| {
            if let Some(ref re) = filter_re {
                if re.is_match(zone_data.name) || re.is_match(&zone_data.id.get().to_string()) {
                    Some(zone_data.id)
                } else {
                    None
                }
            } else {
                Some(zone_data.id)
            }
        })
        .collect();
    
    // Sort by zone ID
    state.filtered_zones.sort_by_key(|id| id.get());
}

/// System to render the zone list panel in the map editor
#[allow(clippy::too_many_arguments)]
pub fn zone_list_panel_system(
    mut egui_context: EguiContexts,
    mut state: ResMut<ZoneListPanelState>,
    game_data: Res<GameData>,
    current_zone: Option<Res<CurrentZone>>,
    mut load_zone_events: EventWriter<LoadZoneEvent>,
    map_editor_state: Res<crate::map_editor::resources::MapEditorState>,
) {
    // Only show when map editor is enabled
    if !map_editor_state.enabled {
        return;
    }
    
    editor_zone_list_panel(
        egui_context.ctx_mut(),
        &mut state,
        &game_data,
        current_zone.as_deref(),
        &mut load_zone_events,
    );
}
