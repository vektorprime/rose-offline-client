//! Status Bar for the Map Editor
//!
//! Displays zone info, modification status, selection count, and camera position.

use bevy_egui::egui;

use crate::map_editor::resources::{EditorMode, MapEditorState};
use crate::map_editor::save::SaveStatus;

/// Get zone name from zone ID
fn get_zone_name(zone_id: u16) -> &'static str {
    // Common zone names from the game data
    match zone_id {
        1 => "Forest",
        2 => "Canyon",
        3 => "Desert",
        4 => "Snow",
        5 => "Swamp",
        6 => "Beach",
        7 => "Underground",
        8 => "City",
        9 => "Dungeon",
        10 => "Tower",
        11 => "Temple",
        12 => "Cave",
        20 => "Junon",
        21 => "Luna",
        22 => "Eldeon",
        23 => "Skaaj",
        24 => "Orlo",
        25 => "Krawfy",
        26 => "Xita",
        27 => "Aurora",
        28 => "Bamboo",
        29 => "Crystal",
        30 => "Frozen",
        31 => "Gorge",
        32 => "Mountain",
        33 => "Ruins",
        34 => "Sanctuary",
        35 => "Shrine",
        36 => "Valley",
        37 => "Volcano",
        38 => "Waterfall",
        39 => "Wind",
        _ => "Unknown",
    }
}

/// Render the status bar (bottom panel)
pub fn editor_status_bar(
    ctx: &egui::Context,
    map_editor_state: &mut MapEditorState,
    save_status: &SaveStatus,
    current_zone_id: Option<u16>,
) {
    egui::TopBottomPanel::bottom("status_bar")
        .exact_height(24.0)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                // Zone name - use actual zone ID if available
                let zone_text = if let Some(id) = current_zone_id {
                    format!("Zone: {} ({})", id, get_zone_name(id))
                } else {
                    "Zone: None".to_string()
                };
                ui.label(egui::RichText::new(&zone_text).strong());
                ui.separator();
                
                // Modification status or save status
                if save_status.is_saving {
                    ui.label(egui::RichText::new("⏳ Saving...").color(egui::Color32::YELLOW));
                } else if let Some(ref result) = save_status.last_result {
                    if result.success {
                        ui.label(egui::RichText::new("✓ Saved").color(egui::Color32::GREEN));
                    } else {
                        ui.label(egui::RichText::new("✗ Save Failed").color(egui::Color32::RED));
                    }
                    ui.separator();
                    
                    // Show modification status after save result
                    if map_editor_state.is_modified {
                        ui.label(egui::RichText::new("● Modified").color(egui::Color32::YELLOW));
                    }
                } else if map_editor_state.is_modified {
                    ui.label(egui::RichText::new("● Modified").color(egui::Color32::YELLOW));
                } else {
                    ui.label(egui::RichText::new("Saved").color(egui::Color32::GREEN));
                }
                ui.separator();
                
                // Selection count
                let selection_count = map_editor_state.selection_count();
                if selection_count > 0 {
                    ui.label(format!("Selected: {}", selection_count));
                } else {
                    ui.label("No selection");
                }
                ui.separator();
                
                // Editor mode - clickable dropdown that opens upward
                let current_mode = map_editor_state.editor_mode;
                let mode_text = format!("Mode: {} ▲", current_mode.display_name());
                
                let button_response = ui.button(&mode_text);
                let popup_id = ui.make_persistent_id("mode_dropdown_popup");
                
                // Toggle popup when button is clicked
                if button_response.clicked() {
                    ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                }
                
                // Open popup above the button (since status bar is at bottom)
                let above = egui::AboveOrBelow::Above;
                let close_behavior = egui::popup::PopupCloseBehavior::CloseOnClick;
                egui::popup::popup_above_or_below_widget(
                    ui,
                    popup_id,
                    &button_response,
                    above,
                    close_behavior,
                    |ui| {
                        ui.set_min_width(100.0);
                        
                        let modes = [
                            EditorMode::Select,
                            EditorMode::Translate,
                            EditorMode::Rotate,
                            EditorMode::Scale,
                            EditorMode::Add,
                            EditorMode::Delete,
                        ];
                        
                        for mode in modes {
                            let is_selected = mode == current_mode;
                            let label = if is_selected {
                                format!("✓ {}", mode.display_name())
                            } else {
                                format!("  {}", mode.display_name())
                            };
                            
                            if ui.button(&label).clicked() {
                                map_editor_state.editor_mode = mode;
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                        }
                    },
                );
                
                ui.separator();
                
                // Grid status
                if map_editor_state.show_grid {
                    ui.label(format!("Grid: {:.1}", map_editor_state.grid_size));
                } else {
                    ui.label("Grid: Off");
                }
                
                // Spacer to push camera position to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Camera position (placeholder - would be populated from camera query)
                    ui.label("Camera: (0.0, 50.0, -100.0)");
                    ui.separator();
                    
                    // Object count (placeholder)
                    ui.label("Objects: 1,234");
                    ui.separator();
                    
                    // FPS indicator (placeholder)
                    ui.label("FPS: 60");
                });
            });
        });
}
