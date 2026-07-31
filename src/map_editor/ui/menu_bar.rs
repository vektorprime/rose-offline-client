//! Menu Bar for the Map Editor
//!
//! Provides the top menu bar with File, Edit, View, Zone, Object, and Help menus.

use bevy::prelude::*;
use bevy_egui::egui;
use std::path::PathBuf;

use crate::map_editor::resources::SelectedModel;
use crate::map_editor::save::{SaveStatus, SaveZoneEvent};
use crate::map_editor::ui::zone_list_panel::ZoneListPanelState;
use crate::map_editor::ui::{AddWaterPlaneEvent, NewZoneEvent};

/// Resource to track help window state
#[derive(Resource, Default)]
pub struct HelpWindowState {
    pub show_shortcuts: bool,
    pub show_about: bool,
}

/// Resource to track "Save Version" dialog state
#[derive(Resource, Default)]
pub struct SaveVersionDialogState {
    pub is_open: bool,
    pub path_input: String,
}

/// Resource to track "New Zone" dialog state
#[derive(Resource)]
pub struct NewZoneDialogState {
    pub is_open: bool,
    pub zone_id_input: String,
    pub output_path_input: String,
    pub initialize_default_block: bool,
}

impl Default for NewZoneDialogState {
    fn default() -> Self {
        Self {
            is_open: false,
            zone_id_input: String::new(),
            output_path_input: String::new(),
            initialize_default_block: true,
        }
    }
}

/// Render the editor menu bar
pub fn editor_menu_bar(
    ctx: &egui::Context,
    save_status: &SaveStatus,
    current_zone_id: Option<u16>,
    next_zone_id_hint: Option<u16>,
    save_events: &mut MessageWriter<SaveZoneEvent>,
    zone_list_state: &mut ZoneListPanelState,
    new_zone_events: &mut MessageWriter<NewZoneEvent>,
    add_water_events: &mut MessageWriter<AddWaterPlaneEvent>,
    help_state: &mut HelpWindowState,
    selected_model: &mut SelectedModel,
    save_version_dialog_state: &mut SaveVersionDialogState,
    new_zone_dialog_state: &mut NewZoneDialogState,
) {
    egui::TopBottomPanel::top("editor_menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            file_menu(
                ui,
                save_status,
                current_zone_id,
                next_zone_id_hint,
                save_events,
                zone_list_state,
                save_version_dialog_state,
                new_zone_dialog_state,
            );
            view_menu(ui, selected_model);
            zone_menu(ui, zone_list_state);
            object_menu(ui, add_water_events);
            help_menu(
                ui,
                &mut help_state.show_shortcuts,
                &mut help_state.show_about,
            );
        });
    });

    // Show help windows
    show_keyboard_shortcuts_window(ctx, &mut help_state.show_shortcuts);
    show_about_window(ctx, &mut help_state.show_about);
    show_save_version_dialog(ctx, save_version_dialog_state, current_zone_id, save_events);
    show_new_zone_dialog(
        ctx,
        new_zone_dialog_state,
        new_zone_events,
        next_zone_id_hint,
    );
}

/// File menu with New, Open, Save, Save As, Exit options
fn file_menu(
    ui: &mut egui::Ui,
    save_status: &SaveStatus,
    current_zone_id: Option<u16>,
    next_zone_id_hint: Option<u16>,
    save_events: &mut MessageWriter<SaveZoneEvent>,
    zone_list_state: &mut ZoneListPanelState,
    save_version_dialog_state: &mut SaveVersionDialogState,
    new_zone_dialog_state: &mut NewZoneDialogState,
) {
    ui.menu_button("File", |ui| {
        if ui.button("New Zone").clicked() {
            log::info!("[MapEditor] File > New Zone clicked");
            if let Some(next_zone_id) = next_zone_id_hint {
                new_zone_dialog_state.zone_id_input = next_zone_id.to_string();
                if new_zone_dialog_state.output_path_input.trim().is_empty() {
                    new_zone_dialog_state.output_path_input =
                        format!("3DDATA/MAPS/CUSTOM/ZONE_{:03}", next_zone_id);
                }
            }
            new_zone_dialog_state.is_open = true;
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
                log::info!(
                    "[MapEditor] File > Save Version clicked for zone {}",
                    zone_id
                );
                if save_version_dialog_state.path_input.is_empty() {
                    save_version_dialog_state.path_input = format!("zone_{}_export", zone_id);
                }
                save_version_dialog_state.is_open = true;
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

fn show_save_version_dialog(
    ctx: &egui::Context,
    dialog_state: &mut SaveVersionDialogState,
    current_zone_id: Option<u16>,
    save_events: &mut MessageWriter<SaveZoneEvent>,
) {
    if !dialog_state.is_open {
        return;
    }

    let mut is_open = dialog_state.is_open;
    egui::Window::new("Save Version")
        .open(&mut is_open)
        .resizable(false)
        .default_width(520.0)
        .show(ctx, |ui| {
            ui.label("Output folder path (absolute or relative to current working directory):");
            ui.text_edit_singleline(&mut dialog_state.path_input);
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    if let Some(zone_id) = current_zone_id {
                        let path = PathBuf::from(dialog_state.path_input.trim());
                        if !dialog_state.path_input.trim().is_empty() {
                            save_events.write(SaveZoneEvent::with_path(zone_id, path));
                        } else {
                            save_events.write(SaveZoneEvent::new(zone_id));
                        }
                        dialog_state.is_open = false;
                    }
                }

                if ui.button("Cancel").clicked() {
                    dialog_state.is_open = false;
                }
            });
        });

    dialog_state.is_open = is_open && dialog_state.is_open;
}

fn show_new_zone_dialog(
    ctx: &egui::Context,
    dialog_state: &mut NewZoneDialogState,
    new_zone_events: &mut MessageWriter<NewZoneEvent>,
    next_zone_id_hint: Option<u16>,
) {
    if !dialog_state.is_open {
        return;
    }

    if dialog_state.zone_id_input.trim().is_empty() {
        if let Some(next_zone_id) = next_zone_id_hint {
            dialog_state.zone_id_input = next_zone_id.to_string();
        }
    }

    if dialog_state.output_path_input.trim().is_empty() {
        if let Some(next_zone_id) = next_zone_id_hint {
            dialog_state.output_path_input = format!("3DDATA/MAPS/CUSTOM/ZONE_{:03}", next_zone_id);
        }
    }

    let mut is_open = dialog_state.is_open;
    egui::Window::new("New Zone")
        .open(&mut is_open)
        .resizable(false)
        .default_width(560.0)
        .show(ctx, |ui| {
            ui.label("Zone ID (auto-filled with next available):");
            ui.text_edit_singleline(&mut dialog_state.zone_id_input);

            ui.label("Optional output folder for zone bootstrap files (HIM/TIL/IFO). Leave empty to use current zone folder.");
            ui.text_edit_singleline(&mut dialog_state.output_path_input);
            ui.checkbox(
                &mut dialog_state.initialize_default_block,
                "Initialize flat 64x64 terrain + empty IFO blocks",
            );

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Create").clicked() {
                    let parsed_zone_id = dialog_state
                        .zone_id_input
                        .trim()
                        .parse::<u16>()
                        .ok()
                        .filter(|id| *id > 0)
                        .or(next_zone_id_hint)
                        .unwrap_or(1);

                    let output_path = if dialog_state.output_path_input.trim().is_empty() {
                        None
                    } else {
                        Some(PathBuf::from(dialog_state.output_path_input.trim()))
                    };

                    new_zone_events.write(NewZoneEvent {
                        prompt_if_modified: true,
                        zone_id: parsed_zone_id,
                        output_path,
                        initialize_default_block: dialog_state.initialize_default_block,
                    });
                    dialog_state.is_open = false;
                }

                if ui.button("Cancel").clicked() {
                    dialog_state.is_open = false;
                }
            });
        });

    dialog_state.is_open = is_open && dialog_state.is_open;
}

/// View menu with grid and camera options
fn view_menu(ui: &mut egui::Ui, selected_model: &mut SelectedModel) {
    ui.menu_button("View", |ui| {
        // Model Browser toggle
        let browser_text = if selected_model.browser_visible {
            "✓ Model Browser"
        } else {
            "  Model Browser"
        };
        if ui
            .add(egui::Button::new(browser_text).shortcut_text("Ctrl+M"))
            .clicked()
        {
            selected_model.toggle_browser();
            log::info!(
                "[MapEditor] View > Model Browser clicked (visible: {})",
                selected_model.browser_visible
            );
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
    });
}

/// Object menu with Add Object, Delete Selected options
fn object_menu(ui: &mut egui::Ui, add_water_events: &mut MessageWriter<AddWaterPlaneEvent>) {
    ui.menu_button("Object", |ui| {
        if ui.button("Add Water Plane").clicked() {
            log::info!("[MapEditor] Object > Add Water Plane clicked");
            add_water_events.write(AddWaterPlaneEvent);
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
