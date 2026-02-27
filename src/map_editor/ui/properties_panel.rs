//! Properties Panel for the Map Editor
//! 
//! Displays properties of the selected entity including transform and components.
//! Connects to actual entity data and sends property change events.

use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui;

use crate::components::{
    EventObject, WarpObject, ZoneObject, ZoneObjectPart, ZoneObjectPartCollisionShape,
};
use crate::map_editor::components::SelectedInEditor;
use crate::map_editor::resources::{DuplicateSelectedEvent, EditorMode, MapEditorState};
use crate::map_editor::systems::property_update_system::PropertyChangeEvent;

/// System parameter for accessing entity data in the properties panel
#[derive(SystemParam)]
pub struct EntityDataQuery<'w, 's> {
    transforms: Query<'w, 's, &'static Transform, With<SelectedInEditor>>,
    zone_objects: Query<'w, 's, &'static ZoneObject, With<SelectedInEditor>>,
    event_objects: Query<'w, 's, &'static EventObject, With<SelectedInEditor>>,
    warp_objects: Query<'w, 's, &'static WarpObject, With<SelectedInEditor>>,
    names: Query<'w, 's, &'static Name, With<SelectedInEditor>>,
}

/// Resource to store pending property changes from UI
#[derive(Resource, Default)]
pub struct PendingPropertyEdits {
    /// Position edit buffer
    pub position: Vec3,
    /// Rotation edit buffer (Euler angles in degrees)
    pub rotation: Vec3,
    /// Scale edit buffer
    pub scale: Vec3,
    /// Whether we've initialized from entity data
    pub initialized: bool,
    /// The entity we're editing
    pub editing_entity: Option<Entity>,
}

/// Render the properties panel (right side panel)
pub fn editor_properties_panel_system(
    mut commands: Commands,
    map_editor_state: Res<MapEditorState>,
    entity_data: EntityDataQuery,
    mut pending_edits: Local<PendingPropertyEdits>,
    name_query: Query<&Name>,
    transform_query: Query<&Transform>,
) {
    // Early return if editor is disabled
    if !map_editor_state.enabled {
        return;
    }
    
    // Get egui context from commands
    let ctx = match commands.get_egui_context() {
        Some(ctx) => ctx,
        None => return,
    };
    
    egui::SidePanel::right("properties_panel")
        .default_width(300.0)
        .min_width(200.0)
        .resizable(true)
        .show(&ctx, |ui| {
            ui.heading("Properties");
            ui.separator();
            
            // Check if any entity is selected
            let selection_count = map_editor_state.selection_count();
            
            if selection_count == 0 {
                ui.label(egui::RichText::new("No object selected").italics());
                ui.label("Click on an object in the viewport or hierarchy to select it.");
                pending_edits.initialized = false;
                pending_edits.editing_entity = None;
                return;
            }
            
            // Show selection info
            if selection_count == 1 {
                // Single selection - show full properties
                if let Some(entity) = map_editor_state.first_selected() {
                    single_object_properties(
                        ui,
                        entity,
                        &map_editor_state,
                        &entity_data,
                        &mut pending_edits,
                        &name_query,
                        &transform_query,
                        &mut commands,
                    );
                }
            } else {
                // Multi-selection - show summary
                multi_object_properties(ui, &map_editor_state, &mut commands);
            }
        });
}

/// Trait to get egui context from Commands (helper)
trait GetEguiContext {
    fn get_egui_context(&mut self) -> Option<egui::Context>;
}

impl GetEguiContext for Commands<'_, '_> {
    fn get_egui_context(&mut self) -> Option<egui::Context> {
        // This is a workaround - we'll use a different approach
        None
    }
}

/// Standalone function to render properties panel (called from ui/mod.rs)
pub fn editor_properties_panel(
    ctx: &egui::Context,
    map_editor_state: &MapEditorState,
    entity_data: &EntityDataQuery,
    pending_edits: &mut PendingPropertyEdits,
    name_query: &Query<&Name>,
    transform_query: &Query<&Transform>,
    event_writer: &mut EventWriter<PropertyChangeEvent>,
    duplicate_event_writer: &mut EventWriter<DuplicateSelectedEvent>,
) {
    egui::SidePanel::right("properties_panel")
        .default_width(300.0)
        .min_width(200.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Properties");
            ui.separator();
            
            // Check if any entity is selected
            let selection_count = map_editor_state.selection_count();
            
            if selection_count == 0 {
                ui.label(egui::RichText::new("No object selected").italics());
                ui.label("Click on an object in the viewport or hierarchy to select it.");
                pending_edits.initialized = false;
                pending_edits.editing_entity = None;
                return;
            }
            
            // Show selection info
            if selection_count == 1 {
                // Single selection - show full properties
                if let Some(entity) = map_editor_state.first_selected() {
                    single_object_properties_standalone(
                        ui,
                        entity,
                        map_editor_state,
                        entity_data,
                        pending_edits,
                        name_query,
                        transform_query,
                        event_writer,
                        duplicate_event_writer,
                    );
                }
            } else {
                // Multi-selection - show summary
                multi_object_properties_standalone(ui, map_editor_state, event_writer, duplicate_event_writer);
            }
        });
}

/// Show properties for a single selected object
fn single_object_properties(
    ui: &mut egui::Ui,
    entity: Entity,
    map_editor_state: &MapEditorState,
    entity_data: &EntityDataQuery,
    pending_edits: &mut PendingPropertyEdits,
    name_query: &Query<&Name>,
    transform_query: &Query<&Transform>,
    commands: &mut Commands,
) {
    // Initialize pending edits when switching to a new entity
    if pending_edits.editing_entity != Some(entity) {
        if let Ok(transform) = transform_query.get(entity) {
            pending_edits.position = transform.translation;
            let euler = transform.rotation.to_euler(EulerRot::XYZ);
            pending_edits.rotation = Vec3::new(
                euler.0.to_degrees(),
                euler.1.to_degrees(),
                euler.2.to_degrees(),
            );
            pending_edits.scale = transform.scale;
            pending_edits.initialized = true;
            pending_edits.editing_entity = Some(entity);
        }
    }
    
    // Entity header
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Entity:").strong());
            let entity_name = name_query.get(entity).ok()
                .map(|n| n.as_str())
                .unwrap_or("Unnamed");
            ui.label(entity_name);
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Type:").strong());
            let entity_type = get_entity_type_string(entity, entity_data);
            ui.label(entity_type);
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Entity ID:").strong());
            ui.label(format!("{:?}", entity));
        });
    });
    
    ui.separator();
    
    // Transform section
    ui.collapsing("Transform", |ui| {
        transform_editor(ui, entity, pending_edits, commands, map_editor_state);
    });
    
    ui.separator();
    
    // Zone Object section (if applicable)
    if entity_data.zone_objects.get(entity).is_ok() {
        ui.collapsing("Zone Object", |ui| {
            zone_object_editor(ui, entity, entity_data, commands, map_editor_state);
        });
        ui.separator();
    }
    
    // Event Object section (if applicable)
    if entity_data.event_objects.get(entity).is_ok() {
        ui.collapsing("Event Object", |ui| {
            event_object_editor(ui, entity, entity_data, commands, map_editor_state);
        });
        ui.separator();
    }
    
    // Warp Object section (if applicable)
    if entity_data.warp_objects.get(entity).is_ok() {
        ui.collapsing("Warp Object", |ui| {
            warp_object_editor(ui, entity, entity_data, commands, map_editor_state);
        });
        ui.separator();
    }
    
    // Collision section (if applicable)
    if has_collision(entity, entity_data) {
        ui.collapsing("Collision", |ui| {
            collision_editor(ui, entity, entity_data, commands, map_editor_state);
        });
        ui.separator();
    }
    
    // Additional components section
    ui.collapsing("Components", |ui| {
        list_components(ui, entity, entity_data);
    });
}

/// Show properties for a single selected object (standalone version)
fn single_object_properties_standalone(
    ui: &mut egui::Ui,
    entity: Entity,
    map_editor_state: &MapEditorState,
    entity_data: &EntityDataQuery,
    pending_edits: &mut PendingPropertyEdits,
    name_query: &Query<&Name>,
    transform_query: &Query<&Transform>,
    event_writer: &mut EventWriter<PropertyChangeEvent>,
    duplicate_event_writer: &mut EventWriter<DuplicateSelectedEvent>,
) {
    // Initialize pending edits when switching to a new entity
    if pending_edits.editing_entity != Some(entity) {
        if let Ok(transform) = transform_query.get(entity) {
            pending_edits.position = transform.translation;
            let euler = transform.rotation.to_euler(EulerRot::XYZ);
            pending_edits.rotation = Vec3::new(
                euler.0.to_degrees(),
                euler.1.to_degrees(),
                euler.2.to_degrees(),
            );
            pending_edits.scale = transform.scale;
            pending_edits.initialized = true;
            pending_edits.editing_entity = Some(entity);
        }
    }
    
    // Entity header
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Entity:").strong());
            let entity_name = name_query.get(entity).ok()
                .map(|n| n.as_str())
                .unwrap_or("Unnamed");
            ui.label(entity_name);
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Type:").strong());
            let entity_type = get_entity_type_string(entity, entity_data);
            ui.label(entity_type);
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Entity ID:").strong());
            ui.label(format!("{:?}", entity));
        });
    });
    
    ui.separator();
    
    // Quick actions
    ui.horizontal(|ui| {
        if ui.button("Duplicate").on_hover_text("Duplicate this object (Ctrl+D)").clicked() {
            duplicate_event_writer.write(DuplicateSelectedEvent::new());
            log::info!("[Properties] Duplicate clicked for entity {:?}", entity);
        }
        if ui.button("Delete").on_hover_text("Delete this object (Delete key)").clicked() {
            log::info!("[Properties] Delete clicked for entity {:?}", entity);
        }
    });
    
    ui.separator();
    
    // Transform section
    ui.collapsing("Transform", |ui| {
        transform_editor_standalone(ui, entity, pending_edits, event_writer, map_editor_state);
    });
    
    ui.separator();
    
    // Zone Object section (if applicable)
    if entity_data.zone_objects.get(entity).is_ok() {
        ui.collapsing("Zone Object", |ui| {
            zone_object_editor_standalone(ui, entity, entity_data, event_writer, map_editor_state);
        });
        ui.separator();
    }
    
    // Event Object section (if applicable)
    if entity_data.event_objects.get(entity).is_ok() {
        ui.collapsing("Event Object", |ui| {
            event_object_editor_standalone(ui, entity, entity_data, event_writer, map_editor_state);
        });
        ui.separator();
    }
    
    // Warp Object section (if applicable)
    if entity_data.warp_objects.get(entity).is_ok() {
        ui.collapsing("Warp Object", |ui| {
            warp_object_editor_standalone(ui, entity, entity_data, event_writer, map_editor_state);
        });
        ui.separator();
    }
    
    // Collision section (if applicable)
    if has_collision(entity, entity_data) {
        ui.collapsing("Collision", |ui| {
            collision_editor_standalone(ui, entity, entity_data, event_writer, map_editor_state);
        });
        ui.separator();
    }
    
    // Additional components section
    ui.collapsing("Components", |ui| {
        list_components(ui, entity, entity_data);
    });
}

/// Show properties for multiple selected objects
fn multi_object_properties(
    ui: &mut egui::Ui,
    map_editor_state: &MapEditorState,
    _commands: &mut Commands,
) {
    let count = map_editor_state.selection_count();
    
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Selected:").strong());
            ui.label(format!("{} objects", count));
        });
    });
    
    ui.separator();
    
    ui.label("Multi-selection editing:");
    
    // Shared transform operations
    ui.collapsing("Transform (Shared)", |ui| {
        ui.label("Position:");
        let mut pos = 0.0f32;
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut pos).prefix("X: ").speed(0.1));
            ui.add(egui::DragValue::new(&mut pos).prefix("Y: ").speed(0.1));
            ui.add(egui::DragValue::new(&mut pos).prefix("Z: ").speed(0.1));
        });
        
        if ui.button("Apply to All").clicked() {
            log::info!("[Properties] Apply position to all clicked");
        }
    });
    
    ui.separator();
    
    // Bulk actions
    ui.label("Actions:");
    if ui.button("Delete All Selected").clicked() {
        log::info!("[Properties] Delete all selected clicked");
    }
    
    if ui.button("Duplicate All Selected").clicked() {
        log::info!("[Properties] Duplicate all selected clicked");
    }
    
    if ui.button("Group Selected").clicked() {
        log::info!("[Properties] Group selected clicked");
    }
}

/// Show properties for multiple selected objects (standalone version)
fn multi_object_properties_standalone(
    ui: &mut egui::Ui,
    map_editor_state: &MapEditorState,
    _event_writer: &mut EventWriter<PropertyChangeEvent>,
    duplicate_event_writer: &mut EventWriter<DuplicateSelectedEvent>,
) {
    let count = map_editor_state.selection_count();
    
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Selected:").strong());
            ui.label(format!("{} objects", count));
        });
    });
    
    ui.separator();
    
    ui.label("Multi-selection editing:");
    
    // Shared transform operations
    ui.collapsing("Transform (Shared)", |ui| {
        ui.label("Position:");
        let mut pos = 0.0f32;
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut pos).prefix("X: ").speed(0.1));
            ui.add(egui::DragValue::new(&mut pos).prefix("Y: ").speed(0.1));
            ui.add(egui::DragValue::new(&mut pos).prefix("Z: ").speed(0.1));
        });
        
        if ui.button("Apply to All").clicked() {
            log::info!("[Properties] Apply position to all clicked");
        }
    });
    
    ui.separator();
    
    // Bulk actions
    ui.label("Actions:");
    if ui.button("Delete All Selected").clicked() {
        log::info!("[Properties] Delete all selected clicked");
    }
    
    if ui.button("Duplicate All Selected").clicked() {
        duplicate_event_writer.write(DuplicateSelectedEvent::new());
        log::info!("[Properties] Duplicate all selected clicked");
    }
    
    if ui.button("Group Selected").clicked() {
        log::info!("[Properties] Group selected clicked");
    }
}

/// Transform editor with position, rotation, and scale
fn transform_editor(
    ui: &mut egui::Ui,
    entity: Entity,
    pending_edits: &mut PendingPropertyEdits,
    _commands: &mut Commands,
    _map_editor_state: &MapEditorState,
) {
    // Position
    ui.label(egui::RichText::new("Position:").strong());
    ui.horizontal(|ui| {
        let old_pos = pending_edits.position;
        ui.add(egui::DragValue::new(&mut pending_edits.position.x).prefix("X: ").speed(0.1));
        ui.add(egui::DragValue::new(&mut pending_edits.position.y).prefix("Y: ").speed(0.1));
        ui.add(egui::DragValue::new(&mut pending_edits.position.z).prefix("Z: ").speed(0.1));
        
        // Check if position changed
        if old_pos != pending_edits.position {
            log::info!("[Properties] Position changed: {:?}", pending_edits.position);
            // In a full implementation, we would send a PropertyChangeEvent here
        }
    });
    
    ui.add_space(4.0);
    
    // Rotation (as Euler angles)
    ui.label(egui::RichText::new("Rotation (degrees):").strong());
    ui.horizontal(|ui| {
        let old_rot = pending_edits.rotation;
        ui.add(egui::DragValue::new(&mut pending_edits.rotation.x).prefix("X: ").speed(1.0));
        ui.add(egui::DragValue::new(&mut pending_edits.rotation.y).prefix("Y: ").speed(1.0));
        ui.add(egui::DragValue::new(&mut pending_edits.rotation.z).prefix("Z: ").speed(1.0));
        
        if old_rot != pending_edits.rotation {
            log::info!("[Properties] Rotation changed: {:?}", pending_edits.rotation);
        }
    });
    
    ui.add_space(4.0);
    
    // Scale
    ui.label(egui::RichText::new("Scale:").strong());
    let mut uniform_scale = true;
    ui.horizontal(|ui| {
        if ui.checkbox(&mut uniform_scale, "Uniform").clicked() {
            log::info!("[Properties] Uniform scale toggled");
        }
    });
    ui.horizontal(|ui| {
        let old_scale = pending_edits.scale;
        ui.add(egui::DragValue::new(&mut pending_edits.scale.x).prefix("X: ").speed(0.01));
        ui.add(egui::DragValue::new(&mut pending_edits.scale.y).prefix("Y: ").speed(0.01));
        ui.add(egui::DragValue::new(&mut pending_edits.scale.z).prefix("Z: ").speed(0.01));
        
        if old_scale != pending_edits.scale {
            log::info!("[Properties] Scale changed: {:?}", pending_edits.scale);
        }
    });
    
    ui.add_space(4.0);
    
    // Quick actions
    ui.horizontal(|ui| {
        if ui.button("Reset Transform").clicked() {
            pending_edits.position = Vec3::ZERO;
            pending_edits.rotation = Vec3::ZERO;
            pending_edits.scale = Vec3::ONE;
            log::info!("[Properties] Reset transform clicked for entity {:?}", entity);
        }
        if ui.button("Center on Origin").clicked() {
            pending_edits.position = Vec3::ZERO;
            log::info!("[Properties] Center on origin clicked for entity {:?}", entity);
        }
    });
}

/// Transform editor with position, rotation, and scale (standalone version with events)
fn transform_editor_standalone(
    ui: &mut egui::Ui,
    entity: Entity,
    pending_edits: &mut PendingPropertyEdits,
    event_writer: &mut EventWriter<PropertyChangeEvent>,
    _map_editor_state: &MapEditorState,
) {
    // Position
    ui.label(egui::RichText::new("Position:").strong());
    ui.horizontal(|ui| {
        let old_pos = pending_edits.position;
        let x_changed = ui.add(egui::DragValue::new(&mut pending_edits.position.x).prefix("X: ").speed(0.1)).changed();
        let y_changed = ui.add(egui::DragValue::new(&mut pending_edits.position.y).prefix("Y: ").speed(0.1)).changed();
        let z_changed = ui.add(egui::DragValue::new(&mut pending_edits.position.z).prefix("Z: ").speed(0.1)).changed();
        
        // Check if position changed
        if x_changed || y_changed || z_changed {
            event_writer.write(PropertyChangeEvent::PositionChanged {
                entity,
                old_position: old_pos,
                new_position: pending_edits.position,
            });
        }
    });
    
    ui.add_space(4.0);
    
    // Rotation (as Euler angles)
    ui.label(egui::RichText::new("Rotation (degrees):").strong());
    ui.horizontal(|ui| {
        let old_rot = pending_edits.rotation;
        let x_changed = ui.add(egui::DragValue::new(&mut pending_edits.rotation.x).prefix("X: ").speed(1.0)).changed();
        let y_changed = ui.add(egui::DragValue::new(&mut pending_edits.rotation.y).prefix("Y: ").speed(1.0)).changed();
        let z_changed = ui.add(egui::DragValue::new(&mut pending_edits.rotation.z).prefix("Z: ").speed(1.0)).changed();
        
        if x_changed || y_changed || z_changed {
            event_writer.write(PropertyChangeEvent::RotationChanged {
                entity,
                old_rotation: old_rot,
                new_rotation: pending_edits.rotation,
            });
        }
    });
    
    ui.add_space(4.0);
    
    // Scale
    ui.label(egui::RichText::new("Scale:").strong());
    let mut uniform_scale = true;
    ui.horizontal(|ui| {
        if ui.checkbox(&mut uniform_scale, "Uniform").clicked() {
            log::info!("[Properties] Uniform scale toggled");
        }
    });
    ui.horizontal(|ui| {
        let old_scale = pending_edits.scale;
        let x_changed = ui.add(egui::DragValue::new(&mut pending_edits.scale.x).prefix("X: ").speed(0.01)).changed();
        let y_changed = ui.add(egui::DragValue::new(&mut pending_edits.scale.y).prefix("Y: ").speed(0.01)).changed();
        let z_changed = ui.add(egui::DragValue::new(&mut pending_edits.scale.z).prefix("Z: ").speed(0.01)).changed();
        
        if x_changed || y_changed || z_changed {
            event_writer.write(PropertyChangeEvent::ScaleChanged {
                entity,
                old_scale,
                new_scale: pending_edits.scale,
            });
        }
    });
    
    ui.add_space(4.0);
    
    // Quick actions
    ui.horizontal(|ui| {
        if ui.button("Reset Transform").clicked() {
            let old_pos = pending_edits.position;
            let old_rot = pending_edits.rotation;
            let old_scale = pending_edits.scale;
            
            pending_edits.position = Vec3::ZERO;
            pending_edits.rotation = Vec3::ZERO;
            pending_edits.scale = Vec3::ONE;
            
            event_writer.write(PropertyChangeEvent::TransformChanged {
                entity,
                old_transform: Transform::from_translation(old_pos)
                    .with_rotation(Quat::from_euler(EulerRot::XYZ, 
                        old_rot.x.to_radians(), 
                        old_rot.y.to_radians(), 
                        old_rot.z.to_radians()))
                    .with_scale(old_scale),
                new_transform: Transform::IDENTITY,
            });
            log::info!("[Properties] Reset transform clicked for entity {:?}", entity);
        }
        if ui.button("Center on Origin").clicked() {
            let old_pos = pending_edits.position;
            pending_edits.position = Vec3::ZERO;
            
            event_writer.write(PropertyChangeEvent::PositionChanged {
                entity,
                old_position: old_pos,
                new_position: Vec3::ZERO,
            });
            log::info!("[Properties] Center on origin clicked for entity {:?}", entity);
        }
    });
}

/// Zone object type-specific editor
fn zone_object_editor(
    ui: &mut egui::Ui,
    entity: Entity,
    entity_data: &EntityDataQuery,
    _commands: &mut Commands,
    _map_editor_state: &MapEditorState,
) {
    if let Ok(zone_object) = entity_data.zone_objects.get(entity) {
        zone_object_editor_inner(ui, entity, zone_object);
    }
}

/// Zone object type-specific editor (standalone version)
fn zone_object_editor_standalone(
    ui: &mut egui::Ui,
    entity: Entity,
    entity_data: &EntityDataQuery,
    event_writer: &mut EventWriter<PropertyChangeEvent>,
    _map_editor_state: &MapEditorState,
) {
    if let Ok(zone_object) = entity_data.zone_objects.get(entity) {
        zone_object_editor_inner_with_events(ui, entity, zone_object, event_writer);
    }
}

/// Inner zone object editor logic
fn zone_object_editor_inner(ui: &mut egui::Ui, entity: Entity, zone_object: &ZoneObject) {
    ui.label(egui::RichText::new("Object Info:").strong());
    
    // Get IDs based on zone object type
    let (ifo_id, zsc_id, part_id, mesh_path) = match zone_object {
        ZoneObject::DecoObject(id) => (id.ifo_object_id, id.zsc_object_id, None, None),
        ZoneObject::CnstObject(id) => (id.ifo_object_id, id.zsc_object_id, None, None),
        ZoneObject::WarpObject(id) => (id.ifo_object_id, id.zsc_object_id, None, None),
        ZoneObject::EventObject(id) => (id.ifo_object_id, id.zsc_object_id, None, None),
        ZoneObject::DecoObjectPart(part) => (part.ifo_object_id, part.zsc_object_id, Some(part.zsc_part_id), Some(&part.mesh_path)),
        ZoneObject::CnstObjectPart(part) => (part.ifo_object_id, part.zsc_object_id, Some(part.zsc_part_id), Some(&part.mesh_path)),
        ZoneObject::WarpObjectPart(part) => (part.ifo_object_id, part.zsc_object_id, Some(part.zsc_part_id), Some(&part.mesh_path)),
        ZoneObject::EventObjectPart(part) => (part.ifo_object_id, part.zsc_object_id, Some(part.zsc_part_id), Some(&part.mesh_path)),
        ZoneObject::AnimatedObject(obj) => (0, 0, None, Some(&obj.mesh_path)),
        ZoneObject::Terrain(terrain) => (terrain.block_x as usize, terrain.block_y as usize, None, None),
        ZoneObject::EffectObject { ifo_object_id, effect_path } => (*ifo_object_id, 0, None, Some(effect_path)),
        ZoneObject::SoundObject { ifo_object_id, sound_path } => (*ifo_object_id, 0, None, Some(sound_path)),
        ZoneObject::Water => (0, 0, None, None),
    };
    
    ui.horizontal(|ui| {
        ui.label("IFO Object ID:");
        let mut ifo_id_mut = ifo_id;
        if ui.add(egui::DragValue::new(&mut ifo_id_mut).speed(1.0)).changed() {
            log::info!("[Properties] IFO Object ID changed: {} -> {}", ifo_id, ifo_id_mut);
        }
    });
    
    ui.horizontal(|ui| {
        ui.label("ZSC Object ID:");
        let mut zsc_id_mut = zsc_id;
        if ui.add(egui::DragValue::new(&mut zsc_id_mut).speed(1.0)).changed() {
            log::info!("[Properties] ZSC Object ID changed: {} -> {}", zsc_id, zsc_id_mut);
        }
    });
    
    if let Some(pid) = part_id {
        ui.horizontal(|ui| {
            ui.label("Part ID:");
            ui.label(format!("{}", pid));
        });
    }
    
    ui.add_space(4.0);
    
    if let Some(path) = mesh_path {
        ui.label(egui::RichText::new("Model:").strong());
        ui.label(path);
    }
    
    ui.add_space(4.0);
    
    // Editable properties
    ui.label(egui::RichText::new("Properties:").strong());
    
    ui.horizontal(|ui| {
        ui.label("Name:");
        let mut name = format!("Object_{}", ifo_id);
        ui.text_edit_singleline(&mut name);
    });
    
    ui.horizontal(|ui| {
        ui.label("Tag:");
        let mut tag = String::new();
        ui.text_edit_singleline(&mut tag);
    });
}

/// Inner zone object editor logic with events
fn zone_object_editor_inner_with_events(
    ui: &mut egui::Ui,
    entity: Entity,
    zone_object: &ZoneObject,
    event_writer: &mut EventWriter<PropertyChangeEvent>,
) {
    ui.label(egui::RichText::new("Object Info:").strong());
    
    // Get IDs based on zone object type
    let (ifo_id, zsc_id, part_id, mesh_path) = match zone_object {
        ZoneObject::DecoObject(id) => (id.ifo_object_id, id.zsc_object_id, None, None),
        ZoneObject::CnstObject(id) => (id.ifo_object_id, id.zsc_object_id, None, None),
        ZoneObject::WarpObject(id) => (id.ifo_object_id, id.zsc_object_id, None, None),
        ZoneObject::EventObject(id) => (id.ifo_object_id, id.zsc_object_id, None, None),
        ZoneObject::DecoObjectPart(part) => (part.ifo_object_id, part.zsc_object_id, Some(part.zsc_part_id), Some(&part.mesh_path)),
        ZoneObject::CnstObjectPart(part) => (part.ifo_object_id, part.zsc_object_id, Some(part.zsc_part_id), Some(&part.mesh_path)),
        ZoneObject::WarpObjectPart(part) => (part.ifo_object_id, part.zsc_object_id, Some(part.zsc_part_id), Some(&part.mesh_path)),
        ZoneObject::EventObjectPart(part) => (part.ifo_object_id, part.zsc_object_id, Some(part.zsc_part_id), Some(&part.mesh_path)),
        ZoneObject::AnimatedObject(obj) => (0, 0, None, Some(&obj.mesh_path)),
        ZoneObject::Terrain(terrain) => (terrain.block_x as usize, terrain.block_y as usize, None, None),
        ZoneObject::EffectObject { ifo_object_id, effect_path } => (*ifo_object_id, 0, None, Some(effect_path)),
        ZoneObject::SoundObject { ifo_object_id, sound_path } => (*ifo_object_id, 0, None, Some(sound_path)),
        ZoneObject::Water => (0, 0, None, None),
    };
    
    ui.horizontal(|ui| {
        ui.label("IFO Object ID:");
        let mut ifo_id_mut = ifo_id;
        if ui.add(egui::DragValue::new(&mut ifo_id_mut).speed(1.0)).changed() {
            event_writer.write(PropertyChangeEvent::ZoneObjectIdChanged {
                entity,
                old_ifo_id: ifo_id,
                new_ifo_id: ifo_id_mut,
                old_zsc_id: zsc_id,
                new_zsc_id: zsc_id,
            });
        }
    });
    
    ui.horizontal(|ui| {
        ui.label("ZSC Object ID:");
        let mut zsc_id_mut = zsc_id;
        if ui.add(egui::DragValue::new(&mut zsc_id_mut).speed(1.0)).changed() {
            event_writer.write(PropertyChangeEvent::ZoneObjectIdChanged {
                entity,
                old_ifo_id: ifo_id,
                new_ifo_id: ifo_id,
                old_zsc_id: zsc_id,
                new_zsc_id: zsc_id_mut,
            });
        }
    });
    
    if let Some(pid) = part_id {
        ui.horizontal(|ui| {
            ui.label("Part ID:");
            ui.label(format!("{}", pid));
        });
    }
    
    ui.add_space(4.0);
    
    if let Some(path) = mesh_path {
        ui.label(egui::RichText::new("Model:").strong());
        ui.label(path);
    }
    
    ui.add_space(4.0);
    
    // Editable properties
    ui.label(egui::RichText::new("Properties:").strong());
    
    ui.horizontal(|ui| {
        ui.label("Name:");
        let mut name = format!("Object_{}", ifo_id);
        ui.text_edit_singleline(&mut name);
    });
    
    ui.horizontal(|ui| {
        ui.label("Tag:");
        let mut tag = String::new();
        ui.text_edit_singleline(&mut tag);
    });
}

/// Event object editor
fn event_object_editor(
    ui: &mut egui::Ui,
    entity: Entity,
    entity_data: &EntityDataQuery,
    _commands: &mut Commands,
    _map_editor_state: &MapEditorState,
) {
    if let Ok(event_object) = entity_data.event_objects.get(entity) {
        event_object_editor_inner(ui, entity, event_object);
    }
}

/// Event object editor (standalone version with events)
fn event_object_editor_standalone(
    ui: &mut egui::Ui,
    entity: Entity,
    entity_data: &EntityDataQuery,
    event_writer: &mut EventWriter<PropertyChangeEvent>,
    _map_editor_state: &MapEditorState,
) {
    if let Ok(event_object) = entity_data.event_objects.get(entity) {
        event_object_editor_inner_with_events(ui, entity, event_object, event_writer);
    }
}

/// Inner event object editor logic
fn event_object_editor_inner(ui: &mut egui::Ui, entity: Entity, event_object: &EventObject) {
    ui.label(egui::RichText::new("Event Object Properties:").strong());
    
    ui.horizontal(|ui| {
        ui.label("Quest Trigger:");
        let mut quest_trigger = event_object.quest_trigger_name.clone();
        if ui.text_edit_singleline(&mut quest_trigger).changed() {
            log::info!("[Properties] Quest trigger changed: {}", quest_trigger);
        }
    });
    
    ui.horizontal(|ui| {
        ui.label("Script Function:");
        let mut script_func = event_object.script_function_name.clone();
        if ui.text_edit_singleline(&mut script_func).changed() {
            log::info!("[Properties] Script function changed: {}", script_func);
        }
    });
}

/// Inner event object editor logic with events
fn event_object_editor_inner_with_events(
    ui: &mut egui::Ui,
    entity: Entity,
    event_object: &EventObject,
    event_writer: &mut EventWriter<PropertyChangeEvent>,
) {
    ui.label(egui::RichText::new("Event Object Properties:").strong());
    
    ui.horizontal(|ui| {
        ui.label("Quest Trigger:");
        let mut quest_trigger = event_object.quest_trigger_name.clone();
        let old_value = quest_trigger.clone();
        if ui.text_edit_singleline(&mut quest_trigger).changed() {
            event_writer.write(PropertyChangeEvent::EventObjectChanged {
                entity,
                property_name: "quest_trigger_name".to_string(),
                old_value,
                new_value: quest_trigger,
            });
        }
    });
    
    ui.horizontal(|ui| {
        ui.label("Script Function:");
        let mut script_func = event_object.script_function_name.clone();
        let old_value = script_func.clone();
        if ui.text_edit_singleline(&mut script_func).changed() {
            event_writer.write(PropertyChangeEvent::EventObjectChanged {
                entity,
                property_name: "script_function_name".to_string(),
                old_value,
                new_value: script_func,
            });
        }
    });
}

/// Warp object editor
fn warp_object_editor(
    ui: &mut egui::Ui,
    entity: Entity,
    entity_data: &EntityDataQuery,
    _commands: &mut Commands,
    _map_editor_state: &MapEditorState,
) {
    if let Ok(warp_object) = entity_data.warp_objects.get(entity) {
        warp_object_editor_inner(ui, entity, warp_object);
    }
}

/// Warp object editor (standalone version with events)
fn warp_object_editor_standalone(
    ui: &mut egui::Ui,
    entity: Entity,
    entity_data: &EntityDataQuery,
    event_writer: &mut EventWriter<PropertyChangeEvent>,
    _map_editor_state: &MapEditorState,
) {
    if let Ok(warp_object) = entity_data.warp_objects.get(entity) {
        warp_object_editor_inner_with_events(ui, entity, warp_object, event_writer);
    }
}

/// Inner warp object editor logic
fn warp_object_editor_inner(ui: &mut egui::Ui, entity: Entity, warp_object: &WarpObject) {
    ui.label(egui::RichText::new("Warp Object Properties:").strong());
    
    ui.horizontal(|ui| {
        ui.label("Warp ID:");
        // Display warp ID (WarpGateId is likely an enum or struct)
        ui.label(format!("{:?}", warp_object.warp_id));
    });
    
    ui.add_space(4.0);
    
    // Target zone/position would be editable here
    ui.label(egui::RichText::new("Target:").strong());
    ui.label("(Warp target information)");
}

/// Inner warp object editor logic with events
fn warp_object_editor_inner_with_events(
    ui: &mut egui::Ui,
    entity: Entity,
    warp_object: &WarpObject,
    event_writer: &mut EventWriter<PropertyChangeEvent>,
) {
    ui.label(egui::RichText::new("Warp Object Properties:").strong());
    
    ui.horizontal(|ui| {
        ui.label("Warp ID:");
        // Display warp ID (WarpGateId is likely an enum or struct)
        let warp_id_str = format!("{:?}", warp_object.warp_id);
        ui.label(&warp_id_str);
    });
    
    ui.add_space(4.0);
    
    // Target zone/position would be editable here
    ui.label(egui::RichText::new("Target:").strong());
    
    let mut target_zone = String::from("Unknown");
    ui.horizontal(|ui| {
        ui.label("Target Zone:");
        let old_value = target_zone.clone();
        if ui.text_edit_singleline(&mut target_zone).changed() {
            event_writer.write(PropertyChangeEvent::WarpObjectChanged {
                entity,
                property_name: "target_zone".to_string(),
                old_value,
                new_value: target_zone,
            });
        }
    });
}

/// Check if entity has collision data
fn has_collision(entity: Entity, entity_data: &EntityDataQuery) -> bool {
    if let Ok(zone_object) = entity_data.zone_objects.get(entity) {
        match zone_object {
            ZoneObject::DecoObjectPart(part) => part.collision_shape != ZoneObjectPartCollisionShape::None,
            ZoneObject::CnstObjectPart(part) => part.collision_shape != ZoneObjectPartCollisionShape::None,
            ZoneObject::WarpObjectPart(part) => part.collision_shape != ZoneObjectPartCollisionShape::None,
            ZoneObject::EventObjectPart(part) => part.collision_shape != ZoneObjectPartCollisionShape::None,
            _ => false,
        }
    } else {
        false
    }
}

/// Collision editor
fn collision_editor(
    ui: &mut egui::Ui,
    entity: Entity,
    entity_data: &EntityDataQuery,
    _commands: &mut Commands,
    _map_editor_state: &MapEditorState,
) {
    if let Ok(zone_object) = entity_data.zone_objects.get(entity) {
        collision_editor_inner(ui, entity, zone_object);
    }
}

/// Collision editor (standalone version with events)
fn collision_editor_standalone(
    ui: &mut egui::Ui,
    entity: Entity,
    entity_data: &EntityDataQuery,
    event_writer: &mut EventWriter<PropertyChangeEvent>,
    _map_editor_state: &MapEditorState,
) {
    if let Ok(zone_object) = entity_data.zone_objects.get(entity) {
        collision_editor_inner_with_events(ui, entity, zone_object, event_writer);
    }
}

/// Get collision part from zone object
fn get_collision_part(zone_object: &ZoneObject) -> Option<&ZoneObjectPart> {
    match zone_object {
        ZoneObject::DecoObjectPart(part) => Some(part),
        ZoneObject::CnstObjectPart(part) => Some(part),
        ZoneObject::WarpObjectPart(part) => Some(part),
        ZoneObject::EventObjectPart(part) => Some(part),
        _ => None,
    }
}

/// Get mutable collision part from zone object (helper for future use)
#[allow(dead_code)]
fn get_collision_part_mut(zone_object: &mut ZoneObject) -> Option<&mut ZoneObjectPart> {
    match zone_object {
        ZoneObject::DecoObjectPart(part) => Some(part),
        ZoneObject::CnstObjectPart(part) => Some(part),
        ZoneObject::WarpObjectPart(part) => Some(part),
        ZoneObject::EventObjectPart(part) => Some(part),
        _ => None,
    }
}

/// Inner collision editor logic
fn collision_editor_inner(ui: &mut egui::Ui, entity: Entity, zone_object: &ZoneObject) {
    if let Some(part) = get_collision_part(zone_object) {
        ui.label(egui::RichText::new("Collision Shape:").strong());
        
        let shape_text = match part.collision_shape {
            ZoneObjectPartCollisionShape::None => "None",
            ZoneObjectPartCollisionShape::Sphere => "Sphere",
            ZoneObjectPartCollisionShape::AxisAlignedBoundingBox => "Box (AABB)",
            ZoneObjectPartCollisionShape::ObjectOrientedBoundingBox => "Box (OBB)",
            ZoneObjectPartCollisionShape::Polygon => "Polygon",
        };
        
        let mut selected_shape = shape_text.to_string();
        egui::ComboBox::from_id_salt(format!("collision_shape_{:?}", entity))
            .selected_text(&selected_shape)
            .show_ui(ui, |ui| {
                ui.selectable_label(part.collision_shape == ZoneObjectPartCollisionShape::None, "None");
                ui.selectable_label(part.collision_shape == ZoneObjectPartCollisionShape::Sphere, "Sphere");
                ui.selectable_label(part.collision_shape == ZoneObjectPartCollisionShape::AxisAlignedBoundingBox, "Box (AABB)");
                ui.selectable_label(part.collision_shape == ZoneObjectPartCollisionShape::ObjectOrientedBoundingBox, "Box (OBB)");
                ui.selectable_label(part.collision_shape == ZoneObjectPartCollisionShape::Polygon, "Polygon");
            });
        
        ui.add_space(4.0);
        
        ui.label(egui::RichText::new("Collision Flags:").strong());
        
        let mut not_moveable = part.collision_not_moveable;
        let mut not_pickable = part.collision_not_pickable;
        let mut height_only = part.collision_height_only;
        let mut no_camera = part.collision_no_camera;
        
        if ui.checkbox(&mut not_moveable, "Not Moveable").changed() {
            log::info!("[Properties] Not moveable changed: {}", not_moveable);
        }
        if ui.checkbox(&mut not_pickable, "Not Pickable").changed() {
            log::info!("[Properties] Not pickable changed: {}", not_pickable);
        }
        if ui.checkbox(&mut height_only, "Height Only").changed() {
            log::info!("[Properties] Height only changed: {}", height_only);
        }
        if ui.checkbox(&mut no_camera, "No Camera Collision").changed() {
            log::info!("[Properties] No camera changed: {}", no_camera);
        }
    }
}

/// Inner collision editor logic with events
fn collision_editor_inner_with_events(
    ui: &mut egui::Ui,
    entity: Entity,
    zone_object: &ZoneObject,
    event_writer: &mut EventWriter<PropertyChangeEvent>,
) {
    if let Some(part) = get_collision_part(zone_object) {
        ui.label(egui::RichText::new("Collision Shape:").strong());
        
        let shape_text = match part.collision_shape {
            ZoneObjectPartCollisionShape::None => "None",
            ZoneObjectPartCollisionShape::Sphere => "Sphere",
            ZoneObjectPartCollisionShape::AxisAlignedBoundingBox => "Box (AABB)",
            ZoneObjectPartCollisionShape::ObjectOrientedBoundingBox => "Box (OBB)",
            ZoneObjectPartCollisionShape::Polygon => "Polygon",
        };
        
        egui::ComboBox::from_id_salt(format!("collision_shape_{:?}", entity))
            .selected_text(shape_text)
            .show_ui(ui, |ui| {
                if ui.selectable_label(part.collision_shape == ZoneObjectPartCollisionShape::None, "None").clicked() {
                    event_writer.write(PropertyChangeEvent::CollisionChanged {
                        entity,
                        property_name: "collision_shape".to_string(),
                        old_value: shape_text.to_string(),
                        new_value: "None".to_string(),
                    });
                }
                if ui.selectable_label(part.collision_shape == ZoneObjectPartCollisionShape::Sphere, "Sphere").clicked() {
                    event_writer.write(PropertyChangeEvent::CollisionChanged {
                        entity,
                        property_name: "collision_shape".to_string(),
                        old_value: shape_text.to_string(),
                        new_value: "Sphere".to_string(),
                    });
                }
                if ui.selectable_label(part.collision_shape == ZoneObjectPartCollisionShape::AxisAlignedBoundingBox, "Box (AABB)").clicked() {
                    event_writer.write(PropertyChangeEvent::CollisionChanged {
                        entity,
                        property_name: "collision_shape".to_string(),
                        old_value: shape_text.to_string(),
                        new_value: "AABB".to_string(),
                    });
                }
                if ui.selectable_label(part.collision_shape == ZoneObjectPartCollisionShape::ObjectOrientedBoundingBox, "Box (OBB)").clicked() {
                    event_writer.write(PropertyChangeEvent::CollisionChanged {
                        entity,
                        property_name: "collision_shape".to_string(),
                        old_value: shape_text.to_string(),
                        new_value: "OBB".to_string(),
                    });
                }
                if ui.selectable_label(part.collision_shape == ZoneObjectPartCollisionShape::Polygon, "Polygon").clicked() {
                    event_writer.write(PropertyChangeEvent::CollisionChanged {
                        entity,
                        property_name: "collision_shape".to_string(),
                        old_value: shape_text.to_string(),
                        new_value: "Polygon".to_string(),
                    });
                }
            });
        
        ui.add_space(4.0);
        
        ui.label(egui::RichText::new("Collision Flags:").strong());
        
        let mut not_moveable = part.collision_not_moveable;
        let mut not_pickable = part.collision_not_pickable;
        let mut height_only = part.collision_height_only;
        let mut no_camera = part.collision_no_camera;
        
        if ui.checkbox(&mut not_moveable, "Not Moveable").changed() {
            event_writer.write(PropertyChangeEvent::CollisionChanged {
                entity,
                property_name: "not_moveable".to_string(),
                old_value: (!not_moveable).to_string(),
                new_value: not_moveable.to_string(),
            });
        }
        if ui.checkbox(&mut not_pickable, "Not Pickable").changed() {
            event_writer.write(PropertyChangeEvent::CollisionChanged {
                entity,
                property_name: "not_pickable".to_string(),
                old_value: (!not_pickable).to_string(),
                new_value: not_pickable.to_string(),
            });
        }
        if ui.checkbox(&mut height_only, "Height Only").changed() {
            event_writer.write(PropertyChangeEvent::CollisionChanged {
                entity,
                property_name: "height_only".to_string(),
                old_value: (!height_only).to_string(),
                new_value: height_only.to_string(),
            });
        }
        if ui.checkbox(&mut no_camera, "No Camera Collision").changed() {
            event_writer.write(PropertyChangeEvent::CollisionChanged {
                entity,
                property_name: "no_camera".to_string(),
                old_value: (!no_camera).to_string(),
                new_value: no_camera.to_string(),
            });
        }
    }
}

/// Get entity type as a string
fn get_entity_type_string(entity: Entity, entity_data: &EntityDataQuery) -> &'static str {
    if entity_data.zone_objects.get(entity).is_ok() {
        if let Ok(zone_object) = entity_data.zone_objects.get(entity) {
            return match zone_object {
                ZoneObject::AnimatedObject(_) => "Animated Object",
                ZoneObject::WarpObject(_) => "Warp Object",
                ZoneObject::WarpObjectPart(_) => "Warp Object Part",
                ZoneObject::EventObject(_) => "Event Object",
                ZoneObject::EventObjectPart(_) => "Event Object Part",
                ZoneObject::CnstObject(_) => "Construction Object",
                ZoneObject::CnstObjectPart(_) => "Construction Part",
                ZoneObject::DecoObject(_) => "Decoration Object",
                ZoneObject::DecoObjectPart(_) => "Decoration Part",
                ZoneObject::Terrain(_) => "Terrain",
                ZoneObject::EffectObject { .. } => "Effect Object",
                ZoneObject::SoundObject { .. } => "Sound Object",
                ZoneObject::Water => "Water",
            };
        }
    }
    if entity_data.event_objects.get(entity).is_ok() {
        return "Event Object";
    }
    if entity_data.warp_objects.get(entity).is_ok() {
        return "Warp Object";
    }
    "Unknown"
}

/// List all components on an entity
fn list_components(ui: &mut egui::Ui, entity: Entity, entity_data: &EntityDataQuery) {
    let mut component_count = 0;
    
    if entity_data.transforms.get(entity).is_ok() {
        ui.label("• Transform");
        component_count += 1;
    }
    
    if entity_data.zone_objects.get(entity).is_ok() {
        ui.label("• ZoneObject");
        component_count += 1;
    }
    
    if entity_data.event_objects.get(entity).is_ok() {
        ui.label("• EventObject");
        component_count += 1;
    }
    
    if entity_data.warp_objects.get(entity).is_ok() {
        ui.label("• WarpObject");
        component_count += 1;
    }
    
    if entity_data.names.get(entity).is_ok() {
        ui.label("• Name");
        component_count += 1;
    }
    
    if has_collision(entity, entity_data) {
        ui.label("• Collision");
        component_count += 1;
    }
    
    if component_count == 0 {
        ui.label("No recognized components");
    } else {
        ui.label(format!("Total: {} components", component_count));
    }
}
