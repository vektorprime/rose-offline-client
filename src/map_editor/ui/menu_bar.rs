//! Menu Bar for the Map Editor
//! 
//! Provides the top menu bar with File, Edit, View, and Object menus.

use bevy::prelude::*;
use bevy_egui::egui;

use crate::map_editor::resources::{MapEditorState, EditorMode};
use crate::map_editor::save::{SaveZoneEvent, SaveStatus};

/// Render the editor menu bar
pub fn editor_menu_bar(
    ctx: &egui::Context, 
    map_editor_state: &MapEditorState,
    save_status: &SaveStatus,
    current_zone_id: Option<u16>,
    mut save_events: &mut EventWriter<SaveZoneEvent>,
) {
    egui::TopBottomPanel::top("editor_menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            file_menu(ui, map_editor_state, save_status, current_zone_id, &mut save_events);
            edit_menu(ui, map_editor_state);
            view_menu(ui, map_editor_state);
            object_menu(ui);
        });
    });
}

/// File menu with New, Open, Save, Save As, Exit options
fn file_menu(
    ui: &mut egui::Ui, 
    map_editor_state: &MapEditorState,
    save_status: &SaveStatus,
    current_zone_id: Option<u16>,
    save_events: &mut EventWriter<SaveZoneEvent>,
) {
    ui.menu_button("File", |ui| {
        if ui.button("New Zone").clicked() {
            log::info!("[MapEditor] File > New Zone clicked");
            ui.close_menu();
        }
        
        if ui.button("Open Zone...").clicked() {
            log::info!("[MapEditor] File > Open Zone clicked");
            ui.close_menu();
        }
        
        ui.separator();
        
        // Save button with Ctrl+S shortcut
        let save_button = ui.add_enabled(
            current_zone_id.is_some() && !save_status.is_saving,
            egui::Button::new("Save").shortcut_text("Ctrl+S"),
        );
        
        if save_button.clicked() {
            if let Some(zone_id) = current_zone_id {
                log::info!("[MapEditor] File > Save clicked for zone {}", zone_id);
                save_events.write(SaveZoneEvent::new(zone_id));
            }
            ui.close_menu();
        }
        
        // Save As button
        let save_as_button = ui.add_enabled(
            current_zone_id.is_some() && !save_status.is_saving,
            egui::Button::new("Save As...").shortcut_text("Ctrl+Shift+S"),
        );
        
        if save_as_button.clicked() {
            if let Some(zone_id) = current_zone_id {
                log::info!("[MapEditor] File > Save As clicked for zone {}", zone_id);
                // For now, just trigger a regular save
                // TODO: Implement file dialog for Save As
                save_events.write(SaveZoneEvent::new(zone_id));
            }
            ui.close_menu();
        }
        
        // Show save status
        if save_status.is_saving {
            ui.label(egui::RichText::new("Saving...").color(egui::Color32::YELLOW));
        } else if let Some(ref result) = save_status.last_result {
            if result.success {
                ui.label(egui::RichText::new("✓ Saved").color(egui::Color32::GREEN));
            } else {
                ui.label(egui::RichText::new("✗ Save failed").color(egui::Color32::RED));
            }
        }
        
        ui.separator();
        
        if ui.button("Exit Editor").clicked() {
            log::info!("[MapEditor] File > Exit Editor clicked");
            ui.close_menu();
        }
    });
}

/// Edit menu with Undo, Redo, Cut, Copy, Paste, Delete, Duplicate options
fn edit_menu(ui: &mut egui::Ui, _map_editor_state: &MapEditorState) {
    ui.menu_button("Edit", |ui| {
        // Undo with shortcut
        let undo_button = ui.add_enabled(
            _map_editor_state.can_undo(),
            egui::Button::new("Undo").shortcut_text("Ctrl+Z"),
        );
        if undo_button.clicked() {
            log::info!("[MapEditor] Edit > Undo clicked");
            ui.close_menu();
        }
        
        // Redo with shortcut
        let redo_button = ui.add_enabled(
            _map_editor_state.can_redo(),
            egui::Button::new("Redo").shortcut_text("Ctrl+Y"),
        );
        if redo_button.clicked() {
            log::info!("[MapEditor] Edit > Redo clicked");
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("Cut").clicked() {
            log::info!("[MapEditor] Edit > Cut clicked");
            ui.close_menu();
        }
        
        if ui.button("Copy").clicked() {
            log::info!("[MapEditor] Edit > Copy clicked");
            ui.close_menu();
        }
        
        if ui.button("Paste").clicked() {
            log::info!("[MapEditor] Edit > Paste clicked");
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("Delete").clicked() {
            log::info!("[MapEditor] Edit > Delete clicked");
            ui.close_menu();
        }
        
        if ui.button("Duplicate").clicked() {
            log::info!("[MapEditor] Edit > Duplicate clicked");
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("Select All").clicked() {
            log::info!("[MapEditor] Edit > Select All clicked");
            ui.close_menu();
        }
        
        if ui.button("Deselect All").clicked() {
            log::info!("[MapEditor] Edit > Deselect All clicked");
            ui.close_menu();
        }
    });
}

/// View menu with grid and camera options
fn view_menu(ui: &mut egui::Ui, map_editor_state: &MapEditorState) {
    ui.menu_button("View", |ui| {
        // Toggle Grid
        let grid_text = if map_editor_state.show_grid {
            "✓ Toggle Grid"
        } else {
            "  Toggle Grid"
        };
        if ui.button(grid_text).clicked() {
            log::info!("[MapEditor] View > Toggle Grid clicked");
            ui.close_menu();
        }
        
        // Snap to Grid
        let snap_text = if map_editor_state.snap_to_grid {
            "✓ Snap to Grid"
        } else {
            "  Snap to Grid"
        };
        if ui.button(snap_text).clicked() {
            log::info!("[MapEditor] View > Snap to Grid clicked");
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("Reset Camera").clicked() {
            log::info!("[MapEditor] View > Reset Camera clicked");
            ui.close_menu();
        }
        
        if ui.button("Frame Selection").clicked() {
            log::info!("[MapEditor] View > Frame Selection clicked");
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("Toggle Colliders").clicked() {
            log::info!("[MapEditor] View > Toggle Colliders clicked");
            ui.close_menu();
        }
        
        if ui.button("Toggle Gizmos").clicked() {
            log::info!("[MapEditor] View > Toggle Gizmos clicked");
            ui.close_menu();
        }
    });
}

/// Object menu with Add Object, Delete Selected options
fn object_menu(ui: &mut egui::Ui) {
    ui.menu_button("Object", |ui| {
        if ui.button("Add Object...").clicked() {
            log::info!("[MapEditor] Object > Add Object clicked");
            ui.close_menu();
        }
        
        if ui.button("Add Effect...").clicked() {
            log::info!("[MapEditor] Object > Add Effect clicked");
            ui.close_menu();
        }
        
        if ui.button("Add Sound...").clicked() {
            log::info!("[MapEditor] Object > Add Sound clicked");
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("Delete Selected").clicked() {
            log::info!("[MapEditor] Object > Delete Selected clicked");
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("Group Selected").clicked() {
            log::info!("[MapEditor] Object > Group Selected clicked");
            ui.close_menu();
        }
        
        if ui.button("Ungroup Selected").clicked() {
            log::info!("[MapEditor] Object > Ungroup Selected clicked");
            ui.close_menu();
        }
    });
}
