//! Model Browser Panel for Map Editor
//! 
//! This panel displays available models organized by category (Deco, Cnst, Event)
//! and allows users to select models for placement.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::map_editor::resources::{
    AvailableModels, MapEditorState, ModelCategory, SelectedModel, EditorMode,
};

/// Model browser panel - displays at the bottom of the screen
pub fn editor_model_browser_panel(
    ctx: &egui::Context,
    map_editor_state: &MapEditorState,
    available_models: &AvailableModels,
    selected_model: &mut SelectedModel,
) {
    // Skip if editor is disabled or browser is hidden
    if !map_editor_state.enabled || !selected_model.browser_visible {
        return;
    }
    
    // Panel at the bottom of the screen
    egui::TopBottomPanel::bottom("model_browser_panel")
        .min_height(150.0)
        .default_height(200.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Model Browser");
                
                // Category tabs
                ui.separator();
                
                let categories = [
                    (ModelCategory::Deco, "Deco"),
                    (ModelCategory::Cnst, "Cnst"),
                    (ModelCategory::Event, "Event"),
                    (ModelCategory::Special, "Special"),
                ];
                
                for (category, label) in categories {
                    let is_selected = selected_model.selected_category == category;
                    let text = if is_selected {
                        format!("▶ {}", label)
                    } else {
                        label.to_string()
                    };
                    
                    let count = available_models.get_models(category).len();
                    let button_text = format!("{} ({})", text, count);
                    
                    if ui.selectable_label(is_selected, button_text).clicked() {
                        selected_model.selected_category = category;
                        // Clear filter when switching categories
                        selected_model.search_filter.clear();
                    }
                }
                
                // Search box
                ui.separator();
                ui.label("Search:");
                if ui.text_edit_singleline(&mut selected_model.search_filter).changed() {
                    // Filter is applied dynamically when rendering the list
                }
                if ui.button("Clear").clicked() {
                    selected_model.search_filter.clear();
                }
            });
            
            // Model list with scrolling
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    let category = selected_model.selected_category;
                    let models = available_models.get_models(category);
                    
                    if models.is_empty() {
                        ui.label(format!("No {} models loaded", category.display_name()));
                        return;
                    }
                    
                    // Filter models by search text
                    let search_lower = selected_model.search_filter.to_lowercase();
                    let filtered_models: Vec<_> = models
                        .iter()
                        .filter(|model| {
                            if search_lower.is_empty() {
                                true
                            } else {
                                model.name.to_lowercase().contains(&search_lower)
                                    || model.id.to_string().contains(&search_lower)
                                    || model.mesh_path.to_lowercase().contains(&search_lower)
                            }
                        })
                        .collect();
                    
                    if filtered_models.is_empty() {
                        ui.label(format!(
                            "No models match filter '{}'",
                            selected_model.search_filter
                        ));
                        return;
                    }
                    
                    // Display models in a grid
                    let columns = 4;
                    let mut row_ui = ui.columns(columns, |cols| {
                        for (i, model) in filtered_models.iter().enumerate() {
                            let col_index = i % columns;
                            let col = &mut cols[col_index];
                            
                            // Check if this model is currently selected
                            let is_selected = selected_model.model.as_ref()
                                .map(|m| m.id == model.id && m.category == model.category)
                                .unwrap_or(false);
                            
                            // Model button with name
                            let response = col.selectable_label(is_selected, &model.name);
                            
                            if response.clicked() {
                                selected_model.select((*model).clone());
                                log::info!(
                                    "[MODEL BROWSER] Selected model: {} (ID: {}, Category: {:?})",
                                    model.name, model.id, model.category
                                );
                            }
                            
                            // Show tooltip on hover
                            response.on_hover_ui(|ui| {
                                ui.label(format!("ID: {}", model.id));
                                ui.label(format!("Mesh: {}", model.mesh_path));
                                ui.label(format!("Parts: {}", model.part_count));
                                ui.separator();
                                ui.label("Click to select for placement");
                            });
                        }
                    });
                    
                    // Silence unused variable warning
                    let _ = row_ui;
                });
            
            // Status bar with selected model info and Add to Zone button
            ui.horizontal(|ui| {
                let total = available_models.total_count();
                let category_count = available_models.get_models(selected_model.selected_category).len();
                let search_lower = selected_model.search_filter.to_lowercase();
                let filtered_count = available_models.get_models(selected_model.selected_category)
                    .iter()
                    .filter(|model| {
                        if search_lower.is_empty() {
                            true
                        } else {
                            model.name.to_lowercase().contains(&search_lower)
                                || model.id.to_string().contains(&search_lower)
                        }
                    })
                    .count();
                
                ui.label(format!(
                    "Total: {} models | {} category: {} models",
                    total,
                    selected_model.selected_category.display_name(),
                    category_count
                ));
                
                if !selected_model.search_filter.is_empty() {
                    ui.label(format!("| Filtered: {}", filtered_count));
                }
                
                if let Some(ref model) = selected_model.model {
                    ui.separator();
                    ui.colored_label(egui::Color32::LIGHT_GREEN, format!("Selected: {}", model.name));
                    ui.label(format!("(ID: {})", model.id));
                    
                    // Add to Zone button
                    ui.separator();
                    let add_button = ui.button("➕ Add to Zone");
                    if add_button.clicked() {
                        // Set the pending placement flag
                        selected_model.pending_placement = true;
                        log::info!(
                            "[MODEL BROWSER] Add to Zone clicked for: {} (ID: {})",
                            model.name,
                            model.id
                        );
                    }
                    
                    // Show current editor mode
                    ui.label(format!("| Mode: {}", map_editor_state.editor_mode.display_name()));
                }
            });
        });
}

/// System to render the model browser panel
pub fn model_browser_panel_system(
    mut contexts: EguiContexts,
    map_editor_state: Res<MapEditorState>,
    available_models: Res<AvailableModels>,
    mut selected_model: ResMut<SelectedModel>,
) {
    let ctx = contexts.ctx_mut();
    
    editor_model_browser_panel(
        ctx,
        &map_editor_state,
        &available_models,
        &mut selected_model,
    );
}

/// Toggle model browser visibility
pub fn toggle_model_browser(selected_model: &mut SelectedModel) {
    selected_model.browser_visible = !selected_model.browser_visible;
}

/// Keyboard shortcut handler for model browser
pub fn model_browser_keyboard_shortcuts(
    mut selected_model: ResMut<SelectedModel>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Toggle browser with Ctrl+M
    if keyboard.pressed(KeyCode::ControlLeft) && keyboard.just_pressed(KeyCode::KeyM) {
        toggle_model_browser(&mut selected_model);
    }
}
