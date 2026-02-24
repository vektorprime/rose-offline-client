//! Status Bar for the Map Editor
//! 
//! Displays zone info, modification status, selection count, and camera position.

use bevy_egui::egui;

use crate::map_editor::resources::MapEditorState;
use crate::map_editor::save::SaveStatus;

/// Render the status bar (bottom panel)
pub fn editor_status_bar(ctx: &egui::Context, map_editor_state: &MapEditorState, save_status: &SaveStatus) {
    egui::TopBottomPanel::bottom("status_bar")
        .exact_height(24.0)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                // Zone name
                ui.label(egui::RichText::new("Zone: 1 (Forest)").strong());
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
                
                // Editor mode
                ui.label(format!("Mode: {}", map_editor_state.editor_mode.display_name()));
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
