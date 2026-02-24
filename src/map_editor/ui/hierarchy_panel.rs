//! Hierarchy Panel for the Map Editor
//! 
//! Displays a tree view of all zone objects with filtering and selection support.

use bevy_egui::egui;

use crate::map_editor::resources::{MapEditorState, HierarchyFilter};

/// Render the hierarchy panel (left side panel)
pub fn editor_hierarchy_panel(ctx: &egui::Context, map_editor_state: &MapEditorState) {
    egui::SidePanel::left("hierarchy_panel")
        .default_width(250.0)
        .min_width(150.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Hierarchy");
            ui.separator();
            
            // Search filter
            ui.horizontal(|ui| {
                ui.label("Search:");
                let mut search = map_editor_state.hierarchy_filter.clone();
                ui.text_edit_singleline(&mut search);
                // Note: In a full implementation, we would update the state here
            });
            
            ui.separator();
            
            // Object type filter dropdown
            ui.horizontal(|ui| {
                ui.label("Filter:");
                egui::ComboBox::from_id_salt("hierarchy_filter")
                    .selected_text(map_editor_state.editor_mode.display_name())
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        // Note: These would need to update state in a full implementation
                        ui.selectable_label(false, "All");
                        ui.selectable_label(false, "Deco Objects");
                        ui.selectable_label(false, "Cnst Objects");
                        ui.selectable_label(false, "Event Objects");
                        ui.selectable_label(false, "Warp Objects");
                        ui.selectable_label(false, "Terrain");
                        ui.selectable_label(false, "Water");
                        ui.selectable_label(false, "Effects");
                        ui.selectable_label(false, "Sounds");
                    });
            });
            
            ui.separator();
            
            // Object list (scrollable)
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Placeholder content - in a full implementation this would show
                // actual zone objects from a query
                
                ui.collapsing("Zone 1", |ui| {
                    ui.collapsing("Block 0_0", |ui| {
                        object_list_item(ui, "Deco Object 1", false);
                        object_list_item(ui, "Deco Object 2", false);
                        object_list_item(ui, "Cnst Object 1", false);
                        object_list_item(ui, "Event Object 1", false);
                    });
                    
                    ui.collapsing("Block 0_1", |ui| {
                        object_list_item(ui, "Deco Object 3", false);
                        object_list_item(ui, "Warp Object 1", true);
                    });
                    
                    ui.collapsing("Block 1_0", |ui| {
                        object_list_item(ui, "Terrain", false);
                        object_list_item(ui, "Water", false);
                    });
                });
                
                // Example with selection
                ui.collapsing("Effects", |ui| {
                    object_list_item(ui, "Effect: spawn_01", false);
                    object_list_item(ui, "Effect: ambient_dust", false);
                });
                
                ui.collapsing("Sounds", |ui| {
                    object_list_item(ui, "Sound: bgm_forest", false);
                    object_list_item(ui, "Sound: ambient_birds", false);
                });
            });
        });
}

/// Render a single object list item with selection and context menu support
fn object_list_item(ui: &mut egui::Ui, name: &str, is_selected: bool) {
    let response = ui.selectable_label(is_selected, name);
    
    // Context menu on right-click
    response.context_menu(|ui| {
        if ui.button("Select").clicked() {
            log::info!("[Hierarchy] Context: Select {}", name);
            ui.close_menu();
        }
        
        if ui.button("Rename").clicked() {
            log::info!("[Hierarchy] Context: Rename {}", name);
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("Duplicate").clicked() {
            log::info!("[Hierarchy] Context: Duplicate {}", name);
            ui.close_menu();
        }
        
        if ui.button("Delete").clicked() {
            log::info!("[Hierarchy] Context: Delete {}", name);
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("Focus in Viewport").clicked() {
            log::info!("[Hierarchy] Context: Focus {}", name);
            ui.close_menu();
        }
    });
    
    // Handle click for selection
    if response.clicked() {
        log::info!("[Hierarchy] Selected: {}", name);
    }
}

/// Format a zone object for display in the hierarchy
#[allow(dead_code)]
fn format_zone_object_label(zone_object_type: &str, id: usize, path: Option<&str>) -> String {
    match path {
        Some(p) => format!("{}: {}", zone_object_type, p),
        None => format!("{} {}", zone_object_type, id),
    }
}
