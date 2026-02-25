//! Hierarchy Panel for the Map Editor
//! 
//! Displays a tree view of all zone objects with filtering and selection support.
//! Queries actual ZoneObject entities from the ECS world.

use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui;

use crate::components::{EventObject, WarpObject, ZoneObject};
use crate::map_editor::components::{EditorSelectable, SelectedInEditor};
use crate::map_editor::resources::{MapEditorState, HierarchyFilter};

/// System parameter for querying hierarchy objects
#[derive(SystemParam)]
pub struct HierarchyQuery<'w, 's> {
    /// All zone objects with their transforms and names
    zone_objects: Query<'w, 's, (Entity, &'static ZoneObject, Option<&'static Name>, Option<&'static Transform>), With<EditorSelectable>>,
    /// Entities that are currently selected
    selected: Query<'w, 's, Entity, With<SelectedInEditor>>,
    /// Event objects
    event_objects: Query<'w, 's, (Entity, &'static EventObject, Option<&'static Name>), With<EditorSelectable>>,
    /// Warp objects  
    warp_objects: Query<'w, 's, (Entity, &'static WarpObject, Option<&'static Name>), With<EditorSelectable>>,
}

/// Categories for organizing hierarchy objects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectCategory {
    Deco,
    Cnst,
    Event,
    Warp,
    Terrain,
    Water,
    Effect,
    Sound,
    Animated,
}

impl ObjectCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            ObjectCategory::Deco => "Decoration",
            ObjectCategory::Cnst => "Construction",
            ObjectCategory::Event => "Event Objects",
            ObjectCategory::Warp => "Warp Objects",
            ObjectCategory::Terrain => "Terrain",
            ObjectCategory::Water => "Water",
            ObjectCategory::Effect => "Effects",
            ObjectCategory::Sound => "Sounds",
            ObjectCategory::Animated => "Animated",
        }
    }
}

/// Get the category of a zone object
fn get_zone_object_category(zone_object: &ZoneObject) -> ObjectCategory {
    match zone_object {
        ZoneObject::DecoObject(_) | ZoneObject::DecoObjectPart(_) => ObjectCategory::Deco,
        ZoneObject::CnstObject(_) | ZoneObject::CnstObjectPart(_) => ObjectCategory::Cnst,
        ZoneObject::EventObject(_) | ZoneObject::EventObjectPart(_) => ObjectCategory::Event,
        ZoneObject::WarpObject(_) | ZoneObject::WarpObjectPart(_) => ObjectCategory::Warp,
        ZoneObject::Terrain(_) => ObjectCategory::Terrain,
        ZoneObject::Water => ObjectCategory::Water,
        ZoneObject::EffectObject { .. } => ObjectCategory::Effect,
        ZoneObject::SoundObject { .. } => ObjectCategory::Sound,
        ZoneObject::AnimatedObject(_) => ObjectCategory::Animated,
    }
}

/// Get a display name for a zone object
fn get_zone_object_name(zone_object: &ZoneObject, name: Option<&Name>, entity: Entity) -> String {
    if let Some(name) = name {
        return name.as_str().to_string();
    }
    
    match zone_object {
        ZoneObject::DecoObject(id) => format!("Deco #{}", id.ifo_object_id),
        ZoneObject::DecoObjectPart(part) => format!("DecoPart #{}:{}", part.zsc_object_id, part.zsc_part_id),
        ZoneObject::CnstObject(id) => format!("Cnst #{}", id.ifo_object_id),
        ZoneObject::CnstObjectPart(part) => format!("CnstPart #{}:{}", part.zsc_object_id, part.zsc_part_id),
        ZoneObject::EventObject(id) => format!("Event #{}", id.ifo_object_id),
        ZoneObject::EventObjectPart(part) => format!("EventPart #{}:{}", part.zsc_object_id, part.zsc_part_id),
        ZoneObject::WarpObject(id) => format!("Warp #{}", id.ifo_object_id),
        ZoneObject::WarpObjectPart(part) => format!("WarpPart #{}:{}", part.zsc_object_id, part.zsc_part_id),
        ZoneObject::Terrain(terrain) => format!("Terrain ({}, {})", terrain.block_x, terrain.block_y),
        ZoneObject::Water => "Water".to_string(),
        ZoneObject::EffectObject { ifo_object_id, effect_path } => {
            let path_short = effect_path.rsplit('/').next().unwrap_or(effect_path);
            format!("Effect: {} (#{})", path_short, ifo_object_id)
        }
        ZoneObject::SoundObject { ifo_object_id, sound_path } => {
            let path_short = sound_path.rsplit('/').next().unwrap_or(sound_path);
            format!("Sound: {} (#{})", path_short, ifo_object_id)
        }
        ZoneObject::AnimatedObject(obj) => {
            let path_short = obj.mesh_path.rsplit('/').next().unwrap_or(&obj.mesh_path);
            format!("Animated: {}", path_short)
        }
    }
}

/// Render the hierarchy panel (left side panel)
pub fn editor_hierarchy_panel(
    ctx: &egui::Context, 
    map_editor_state: &MapEditorState,
    hierarchy_query: &HierarchyQuery,
    mut commands: &mut Commands,
) {
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
                // Note: In a full implementation, we would update a state resource here
                // For now, we just show the filter value
                let mut search = map_editor_state.hierarchy_filter.clone();
                ui.text_edit_singleline(&mut search);
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
            
            // Collect objects by category
            let mut categories: std::collections::HashMap<ObjectCategory, Vec<(Entity, String, bool)>> = 
                std::collections::HashMap::new();
            
            // Get selected entities set for quick lookup
            let selected_set: std::collections::HashSet<Entity> = map_editor_state.selected_entities.iter().copied().collect();
            
            // Query zone objects
            for (entity, zone_object, name, _transform) in hierarchy_query.zone_objects.iter() {
                let category = get_zone_object_category(zone_object);
                let display_name = get_zone_object_name(zone_object, name, entity);
                let is_selected = selected_set.contains(&entity);
                
                categories.entry(category)
                    .or_insert_with(Vec::new)
                    .push((entity, display_name, is_selected));
            }
            
            // Query event objects (separate component)
            for (entity, _event_object, name) in hierarchy_query.event_objects.iter() {
                let display_name = name
                    .map(|n| n.as_str().to_string())
                    .unwrap_or_else(|| format!("Event {:?}", entity));
                let is_selected = selected_set.contains(&entity);
                
                categories.entry(ObjectCategory::Event)
                    .or_insert_with(Vec::new)
                    .push((entity, display_name, is_selected));
            }
            
            // Query warp objects (separate component)
            for (entity, _warp_object, name) in hierarchy_query.warp_objects.iter() {
                let display_name = name
                    .map(|n| n.as_str().to_string())
                    .unwrap_or_else(|| format!("Warp {:?}", entity));
                let is_selected = selected_set.contains(&entity);
                
                categories.entry(ObjectCategory::Warp)
                    .or_insert_with(Vec::new)
                    .push((entity, display_name, is_selected));
            }
            
            // Object list (scrollable)
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Define order of categories
                let category_order = [
                    ObjectCategory::Deco,
                    ObjectCategory::Cnst,
                    ObjectCategory::Event,
                    ObjectCategory::Warp,
                    ObjectCategory::Terrain,
                    ObjectCategory::Water,
                    ObjectCategory::Effect,
                    ObjectCategory::Sound,
                    ObjectCategory::Animated,
                ];
                
                // Track if any entity was clicked
                let mut clicked_entity: Option<Entity> = None;
                let mut clicked_entity_is_selected = false;
                
                for category in category_order {
                    if let Some(objects) = categories.get(&category) {
                        if objects.is_empty() {
                            continue;
                        }
                        
                        ui.collapsing(format!("{} ({})", category.display_name(), objects.len()), |ui| {
                            for (entity, display_name, is_selected) in objects {
                                let response = ui.selectable_label(*is_selected, display_name);
                                
                                // Context menu on right-click
                                response.context_menu(|ui| {
                                    if ui.button("Select").clicked() {
                                        clicked_entity = Some(*entity);
                                        clicked_entity_is_selected = false;
                                        ui.close_menu();
                                    }
                                    
                                    if ui.button("Focus in Viewport").clicked() {
                                        log::info!("[Hierarchy] Focus on entity: {:?}", entity);
                                        ui.close_menu();
                                    }
                                    
                                    ui.separator();
                                    
                                    if ui.button("Delete").clicked() {
                                        log::info!("[Hierarchy] Delete entity: {:?}", entity);
                                        ui.close_menu();
                                    }
                                });
                                
                                // Handle click for selection
                                if response.clicked() {
                                    clicked_entity = Some(*entity);
                                    clicked_entity_is_selected = *is_selected;
                                }
                            }
                        });
                    }
                }
                
                // Handle selection change after the UI loop (to avoid borrow issues)
                if let Some(entity) = clicked_entity {
                    // Clear previous selection and select new entity
                    for selected_entity in hierarchy_query.selected.iter() {
                        commands.entity(selected_entity).remove::<SelectedInEditor>();
                    }
                    
                    // Note: We need to update MapEditorState as well
                    // This would require ResMut<MapEditorState> which we can't easily pass here
                    // For now, just add/remove the component
                    commands.entity(entity).insert(SelectedInEditor);
                    
                    log::info!("[Hierarchy] Selected entity: {:?}", entity);
                }
            });
            
            // Footer with object count
            let total_count: usize = categories.values().map(|v| v.len()).sum();
            ui.separator();
            ui.label(format!("Total objects: {}", total_count));
        });
}

/// Render a single object list item with selection and context menu support
#[allow(dead_code)]
fn object_list_item(ui: &mut egui::Ui, name: &str, is_selected: bool) -> bool {
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
    
    // Return true if clicked
    response.clicked()
}

/// Format a zone object for display in the hierarchy
#[allow(dead_code)]
fn format_zone_object_label(zone_object_type: &str, id: usize, path: Option<&str>) -> String {
    match path {
        Some(p) => format!("{}: {}", zone_object_type, p),
        None => format!("{} {}", zone_object_type, id),
    }
}
