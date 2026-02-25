//! Menu Bar for the Map Editor
//!
//! Provides the top menu bar with File, Edit, View, Zone, Object, and Help menus.

use bevy::prelude::*;
use bevy_egui::egui;

use crate::map_editor::resources::{MapEditorState, EditorMode, SelectedModel};
use crate::map_editor::save::{SaveZoneEvent, SaveStatus};
use crate::map_editor::ui::NewZoneEvent;
use crate::map_editor::ui::zone_list_panel::ZoneListPanelState;

/// Resource to track help window state
#[derive(Resource, Default)]
pub struct HelpWindowState {
    pub show_shortcuts: bool,
    pub show_about: bool,
}

/// Render the editor menu bar
pub fn editor_menu_bar(
    ctx: &egui::Context,
    map_editor_state: &MapEditorState,
    save_status: &SaveStatus,
    current_zone_id: Option<u16>,
    save_events: &mut EventWriter<SaveZoneEvent>,
    zone_list_state: &mut ZoneListPanelState,
    new_zone_events: &mut EventWriter<NewZoneEvent>,
    help_state: &mut HelpWindowState,
    selected_model: &mut SelectedModel,
) {
    egui::TopBottomPanel::top("editor_menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            file_menu(ui, map_editor_state, save_status, current_zone_id, save_events, new_zone_events, zone_list_state);
            edit_menu(ui, map_editor_state);
            view_menu(ui, map_editor_state, selected_model);
            zone_menu(ui, zone_list_state);
            object_menu(ui);
            help_menu(ui, &mut help_state.show_shortcuts, &mut help_state.show_about);
        });
    });
    
    // Show help windows
    show_keyboard_shortcuts_window(ctx, &mut help_state.show_shortcuts);
    show_about_window(ctx, &mut help_state.show_about);
}

/// File menu with New, Open, Save, Save As, Exit options
fn file_menu(
    ui: &mut egui::Ui,
    map_editor_state: &MapEditorState,
    save_status: &SaveStatus,
    current_zone_id: Option<u16>,
    save_events: &mut EventWriter<SaveZoneEvent>,
    new_zone_events: &mut EventWriter<NewZoneEvent>,
    zone_list_state: &mut ZoneListPanelState,
) {
    ui.menu_button("File", |ui| {
        if ui.button("New Zone").clicked() {
            log::info!("[MapEditor] File > New Zone clicked");
            new_zone_events.write(NewZoneEvent::new());
            ui.close_menu();
        }
        
        if ui.button("Open Zone...").clicked() {
            log::info!("[MapEditor] File > Open Zone clicked");
            zone_list_state.is_open = true;
            ui.close_menu();
        }
        
        ui.separator();
        
        // Save button
        let save_button = ui.add_enabled(
            current_zone_id.is_some() && !save_status.is_saving,
            egui::Button::new("Save"),
        );
        
        if save_button.clicked() {
            if let Some(zone_id) = current_zone_id {
                log::info!("[MapEditor] File > Save clicked for zone {}", zone_id);
                log::info!("[MapEditor] Writing SaveZoneEvent to event writer");
                save_events.write(SaveZoneEvent::new(zone_id));
                log::info!("[MapEditor] SaveZoneEvent written successfully");
            } else {
                log::warn!("[MapEditor] Save clicked but no current_zone_id available!");
            }
            ui.close_menu();
        }
        
        // Save As button (creates timestamped backup)
        let save_as_button = ui.add_enabled(
            current_zone_id.is_some() && !save_status.is_saving,
            egui::Button::new("Save Version..."),
        );
        
        if save_as_button.clicked() {
            if let Some(zone_id) = current_zone_id {
                log::info!("[MapEditor] File > Save Version clicked for zone {}", zone_id);
                // This will create a timestamped backup via the save system
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
fn view_menu(ui: &mut egui::Ui, map_editor_state: &MapEditorState, selected_model: &mut SelectedModel) {
    ui.menu_button("View", |ui| {
        // Model Browser toggle
        let browser_text = if selected_model.browser_visible {
            "✓ Model Browser"
        } else {
            "  Model Browser"
        };
        if ui.add(egui::Button::new(browser_text).shortcut_text("Ctrl+M")).clicked() {
            selected_model.toggle_browser();
            log::info!("[MapEditor] View > Model Browser clicked (visible: {})", selected_model.browser_visible);
            ui.close_menu();
        }
        
        ui.separator();
        
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

/// Zone menu with zone switching options
fn zone_menu(ui: &mut egui::Ui, zone_list_state: &mut ZoneListPanelState) {
    ui.menu_button("Zone", |ui| {
        if ui.button("Open Zone...").clicked() {
            log::info!("[MapEditor] Zone > Open Zone clicked");
            zone_list_state.is_open = true;
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("Zone Info").clicked() {
            log::info!("[MapEditor] Zone > Zone Info clicked");
            ui.close_menu();
        }
        
        if ui.button("Validate Zone").clicked() {
            log::info!("[MapEditor] Zone > Validate Zone clicked");
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

/// Help menu with keyboard shortcuts and about information
pub fn help_menu(ui: &mut egui::Ui, show_shortcuts: &mut bool, show_about: &mut bool) {
    ui.menu_button("Help", |ui| {
        if ui.button("Keyboard Shortcuts").clicked() {
            *show_shortcuts = true;
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("About Map Editor").clicked() {
            *show_about = true;
            ui.close_menu();
        }
    });
}

/// Show keyboard shortcuts help window
pub fn show_keyboard_shortcuts_window(ctx: &egui::Context, is_open: &mut bool) {
    if !*is_open {
        return;
    }
    
    egui::Window::new("Keyboard Shortcuts")
        .open(is_open)
        .collapsible(true)
        .default_width(350.0)
        .show(ctx, |ui| {
            ui.heading("Selection");
            ui.separator();
            ui.label("Click - Select object");
            ui.label("Ctrl+Click - Add to selection");
            ui.label("Ctrl+A - Select all");
            ui.label("Escape - Deselect all");
            
            ui.add_space(8.0);
            ui.heading("Transform Modes");
            ui.separator();
            ui.label("Q - Select mode");
            ui.label("E - Rotate mode");
            ui.label("R - Scale mode");
            ui.label("V - Add mode");
            ui.label("X - Delete mode");
            
            ui.add_space(8.0);
            ui.heading("Actions");
            ui.separator();
            ui.label("Delete - Delete selected objects");
            ui.label("Ctrl+D - Duplicate selected objects");
            ui.label("Ctrl+Z - Undo last action");
            ui.label("Ctrl+Y - Redo last undone action");
            ui.label("Ctrl+Shift+Z - Redo (alternative)");
            ui.label("G - Toggle snap to grid");
            ui.label("F - Focus on selected object");
            
            ui.add_space(8.0);
            ui.heading("Camera");
            ui.separator();
            ui.label("Tab - Toggle free/orbit camera");
            ui.label("WASD - Move camera (free camera mode)");
            ui.label("Mouse - Look around (free camera mode)");
            ui.label("Scroll - Zoom in/out");
            
            ui.add_space(8.0);
            ui.heading("Panels");
            ui.separator();
            ui.label("Ctrl+M - Toggle Model Browser");
            
            ui.add_space(8.0);
            ui.heading("File Operations");
            ui.separator();
            ui.label("Use File menu for Save/Save Version");
        });
}

/// Show about window
pub fn show_about_window(ctx: &egui::Context, is_open: &mut bool) {
    if !*is_open {
        return;
    }
    
    egui::Window::new("About Map Editor")
        .open(is_open)
        .collapsible(true)
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.heading("Rose Online Map Editor");
            ui.label("Version 2.6");
            ui.add_space(8.0);
            ui.label("A live map editor for the Rose Online client.");
            ui.add_space(8.0);
            ui.label("Features:");
            ui.label("• Real-time object manipulation");
            ui.label("• Transform gizmos (translate, rotate, scale)");
            ui.label("• Model browser with search");
            ui.label("• Undo/Redo support");
            ui.label("• Zone save/export functionality");
            ui.add_space(8.0);
            ui.label("Use the Help menu for keyboard shortcuts.");
        });
}
