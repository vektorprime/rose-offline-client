//! Property Update System for the Map Editor
//! 
//! Listens for property changes from the UI and applies them to selected entities.
//! Tracks modifications in MapEditorState for undo/redo support.

use bevy::prelude::*;

use crate::components::{
    EventObject, WarpObject, ZoneObject, ZoneObjectPart, ZoneObjectPartCollisionShape,
};
use crate::map_editor::components::SelectedInEditor;
use crate::map_editor::resources::{EditorAction, MapEditorState};

/// Events for property changes from the UI
#[derive(Event, Debug, Clone)]
pub enum PropertyChangeEvent {
    /// Transform position changed
    PositionChanged {
        entity: Entity,
        old_position: Vec3,
        new_position: Vec3,
    },
    /// Transform rotation changed (in Euler angles, degrees)
    RotationChanged {
        entity: Entity,
        old_rotation: Vec3,
        new_rotation: Vec3,
    },
    /// Transform scale changed
    ScaleChanged {
        entity: Entity,
        old_scale: Vec3,
        new_scale: Vec3,
    },
    /// Full transform changed
    TransformChanged {
        entity: Entity,
        old_transform: Transform,
        new_transform: Transform,
    },
    /// Zone object ID changed
    ZoneObjectIdChanged {
        entity: Entity,
        old_ifo_id: usize,
        new_ifo_id: usize,
        old_zsc_id: usize,
        new_zsc_id: usize,
    },
    /// Event object property changed
    EventObjectChanged {
        entity: Entity,
        property_name: String,
        old_value: String,
        new_value: String,
    },
    /// Warp object property changed
    WarpObjectChanged {
        entity: Entity,
        property_name: String,
        old_value: String,
        new_value: String,
    },
    /// Collision property changed
    CollisionChanged {
        entity: Entity,
        property_name: String,
        old_value: String,
        new_value: String,
    },
}

/// Resource to store pending property changes (for batch processing)
#[derive(Resource, Default)]
pub struct PendingPropertyChanges {
    pub changes: Vec<PropertyChangeEvent>,
}

impl PendingPropertyChanges {
    pub fn new() -> Self {
        Self { changes: Vec::new() }
    }
    
    pub fn push(&mut self, change: PropertyChangeEvent) {
        self.changes.push(change);
    }
    
    pub fn clear(&mut self) {
        self.changes.clear();
    }
    
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// System to process property change events and apply them to entities
pub fn property_update_system(
    mut events: EventReader<PropertyChangeEvent>,
    mut map_editor_state: ResMut<MapEditorState>,
    mut transforms: Query<&mut Transform>,
    mut zone_objects: Query<&mut ZoneObject>,
    mut event_objects: Query<&mut EventObject>,
    mut warp_objects: Query<&mut WarpObject>,
    mut commands: Commands,
) {
    for event in events.read() {
        match event {
            PropertyChangeEvent::PositionChanged {
                entity,
                old_position,
                new_position,
            } => {
                if let Ok(mut transform) = transforms.get_mut(*entity) {
                    let old_transform = *transform;
                    transform.translation = *new_position;
                    
                    map_editor_state.push_action(EditorAction::TransformEntity {
                        entity: *entity,
                        old_transform,
                        new_transform: *transform,
                    });
                    
                    log::info!(
                        "[PropertyUpdate] Position changed for entity {:?}: {:?} -> {:?}",
                        entity,
                        old_position,
                        new_position
                    );
                }
            }
            
            PropertyChangeEvent::RotationChanged {
                entity,
                old_rotation,
                new_rotation,
            } => {
                if let Ok(mut transform) = transforms.get_mut(*entity) {
                    let old_transform = *transform;
                    
                    // Convert Euler angles (degrees) to quaternion
                    let euler_rad = Vec3::new(
                        new_rotation.x.to_radians(),
                        new_rotation.y.to_radians(),
                        new_rotation.z.to_radians(),
                    );
                    transform.rotation = Quat::from_euler(EulerRot::XYZ, euler_rad.x, euler_rad.y, euler_rad.z);
                    
                    map_editor_state.push_action(EditorAction::TransformEntity {
                        entity: *entity,
                        old_transform,
                        new_transform: *transform,
                    });
                    
                    log::info!(
                        "[PropertyUpdate] Rotation changed for entity {:?}: {:?} -> {:?}",
                        entity,
                        old_rotation,
                        new_rotation
                    );
                }
            }
            
            PropertyChangeEvent::ScaleChanged {
                entity,
                old_scale,
                new_scale,
            } => {
                if let Ok(mut transform) = transforms.get_mut(*entity) {
                    let old_transform = *transform;
                    transform.scale = *new_scale;
                    
                    map_editor_state.push_action(EditorAction::TransformEntity {
                        entity: *entity,
                        old_transform,
                        new_transform: *transform,
                    });
                    
                    log::info!(
                        "[PropertyUpdate] Scale changed for entity {:?}: {:?} -> {:?}",
                        entity,
                        old_scale,
                        new_scale
                    );
                }
            }
            
            PropertyChangeEvent::TransformChanged {
                entity,
                old_transform,
                new_transform,
            } => {
                if let Ok(mut transform) = transforms.get_mut(*entity) {
                    *transform = *new_transform;
                    
                    map_editor_state.push_action(EditorAction::TransformEntity {
                        entity: *entity,
                        old_transform: *old_transform,
                        new_transform: *new_transform,
                    });
                    
                    log::info!(
                        "[PropertyUpdate] Full transform changed for entity {:?}",
                        entity
                    );
                }
            }
            
            PropertyChangeEvent::ZoneObjectIdChanged {
                entity,
                old_ifo_id,
                new_ifo_id,
                old_zsc_id,
                new_zsc_id,
            } => {
                if let Ok(mut zone_object) = zone_objects.get_mut(*entity) {
                    // Update the zone object IDs based on the variant
                    match zone_object.as_mut() {
                        ZoneObject::DecoObject(id) |
                        ZoneObject::CnstObject(id) |
                        ZoneObject::WarpObject(id) |
                        ZoneObject::EventObject(id) => {
                            let old_ifo = id.ifo_object_id;
                            let old_zsc = id.zsc_object_id;
                            
                            id.ifo_object_id = *new_ifo_id;
                            id.zsc_object_id = *new_zsc_id;
                            
                            map_editor_state.push_action(EditorAction::ModifyComponent {
                                entity: *entity,
                                component_type: "ZoneObjectId".to_string(),
                                old_value: format!("ifo:{}, zsc:{}", old_ifo, old_zsc),
                                new_value: format!("ifo:{}, zsc:{}", new_ifo_id, new_zsc_id),
                            });
                        }
                        ZoneObject::DecoObjectPart(part) |
                        ZoneObject::CnstObjectPart(part) |
                        ZoneObject::WarpObjectPart(part) |
                        ZoneObject::EventObjectPart(part) => {
                            let old_ifo = part.ifo_object_id;
                            let old_zsc = part.zsc_object_id;
                            
                            part.ifo_object_id = *new_ifo_id;
                            part.zsc_object_id = *new_zsc_id;
                            
                            map_editor_state.push_action(EditorAction::ModifyComponent {
                                entity: *entity,
                                component_type: "ZoneObjectPart".to_string(),
                                old_value: format!("ifo:{}, zsc:{}", old_ifo, old_zsc),
                                new_value: format!("ifo:{}, zsc:{}", new_ifo_id, new_zsc_id),
                            });
                        }
                        _ => {}
                    }
                    
                    log::info!(
                        "[PropertyUpdate] ZoneObject ID changed for entity {:?}: ifo {} -> {}, zsc {} -> {}",
                        entity, old_ifo_id, new_ifo_id, old_zsc_id, new_zsc_id
                    );
                }
            }
            
            PropertyChangeEvent::EventObjectChanged {
                entity,
                property_name,
                old_value,
                new_value,
            } => {
                if let Ok(mut event_object) = event_objects.get_mut(*entity) {
                    match property_name.as_str() {
                        "quest_trigger_name" => {
                            event_object.quest_trigger_name = new_value.clone();
                        }
                        "script_function_name" => {
                            event_object.script_function_name = new_value.clone();
                        }
                        _ => {}
                    }
                    
                    map_editor_state.push_action(EditorAction::ModifyComponent {
                        entity: *entity,
                        component_type: "EventObject".to_string(),
                        old_value: old_value.clone(),
                        new_value: new_value.clone(),
                    });
                    
                    log::info!(
                        "[PropertyUpdate] EventObject {} changed for entity {:?}: {} -> {}",
                        property_name,
                        entity,
                        old_value,
                        new_value
                    );
                }
            }
            
            PropertyChangeEvent::WarpObjectChanged {
                entity,
                property_name,
                old_value,
                new_value,
            } => {
                if let Ok(mut warp_object) = warp_objects.get_mut(*entity) {
                    // WarpObject has warp_id which is a WarpGateId
                    // For now, just log the change - actual update would need WarpGateId parsing
                    map_editor_state.push_action(EditorAction::ModifyComponent {
                        entity: *entity,
                        component_type: "WarpObject".to_string(),
                        old_value: old_value.clone(),
                        new_value: new_value.clone(),
                    });
                    
                    log::info!(
                        "[PropertyUpdate] WarpObject {} changed for entity {:?}: {} -> {}",
                        property_name,
                        entity,
                        old_value,
                        new_value
                    );
                }
            }
            
            PropertyChangeEvent::CollisionChanged {
                entity,
                property_name,
                old_value,
                new_value,
            } => {
                // Collision changes require updating the ZoneObjectPart collision fields
                if let Ok(mut zone_object) = zone_objects.get_mut(*entity) {
                    let collision_update = |part: &mut ZoneObjectPart| {
                        match property_name.as_str() {
                            "collision_shape" => {
                                part.collision_shape = match new_value.as_str() {
                                    "None" => ZoneObjectPartCollisionShape::None,
                                    "Sphere" => ZoneObjectPartCollisionShape::Sphere,
                                    "AABB" => ZoneObjectPartCollisionShape::AxisAlignedBoundingBox,
                                    "OBB" => ZoneObjectPartCollisionShape::ObjectOrientedBoundingBox,
                                    "Polygon" => ZoneObjectPartCollisionShape::Polygon,
                                    _ => ZoneObjectPartCollisionShape::default(),
                                };
                            }
                            "not_moveable" => {
                                part.collision_not_moveable = new_value == "true";
                            }
                            "not_pickable" => {
                                part.collision_not_pickable = new_value == "true";
                            }
                            "height_only" => {
                                part.collision_height_only = new_value == "true";
                            }
                            "no_camera" => {
                                part.collision_no_camera = new_value == "true";
                            }
                            _ => {}
                        }
                    };
                    
                    match zone_object.as_mut() {
                        ZoneObject::DecoObjectPart(part) |
                        ZoneObject::CnstObjectPart(part) |
                        ZoneObject::WarpObjectPart(part) |
                        ZoneObject::EventObjectPart(part) => {
                            collision_update(part);
                        }
                        _ => {}
                    }
                    
                    map_editor_state.push_action(EditorAction::ModifyComponent {
                        entity: *entity,
                        component_type: "Collision".to_string(),
                        old_value: old_value.clone(),
                        new_value: new_value.clone(),
                    });
                    
                    log::info!(
                        "[PropertyUpdate] Collision {} changed for entity {:?}: {} -> {}",
                        property_name,
                        entity,
                        old_value,
                        new_value
                    );
                }
            }
        }
    }
}

/// System to apply undo operations
pub fn apply_undo_system(
    mut map_editor_state: ResMut<MapEditorState>,
    mut transforms: Query<&mut Transform>,
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Check for Ctrl+Z (undo)
    if keyboard.just_pressed(KeyCode::KeyZ) && (keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight)) {
        if !keyboard.pressed(KeyCode::ShiftLeft) && !keyboard.pressed(KeyCode::ShiftRight) {
            if let Some(action) = map_editor_state.pop_undo() {
                apply_undo_action(action, &mut transforms, &mut commands, &mut map_editor_state);
                log::info!("[PropertyUpdate] Undo applied");
            }
        }
    }
    
    // Check for Ctrl+Y or Ctrl+Shift+Z (redo)
    if keyboard.just_pressed(KeyCode::KeyY) && (keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight)) {
        if let Some(action) = map_editor_state.pop_redo() {
            apply_redo_action(action, &mut transforms, &mut commands, &mut map_editor_state);
            log::info!("[PropertyUpdate] Redo applied");
        }
    }
    
    // Also handle Ctrl+Shift+Z for redo
    if keyboard.just_pressed(KeyCode::KeyZ) && 
       (keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight)) &&
       (keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight)) {
        if let Some(action) = map_editor_state.pop_redo() {
            apply_redo_action(action, &mut transforms, &mut commands, &mut map_editor_state);
            log::info!("[PropertyUpdate] Redo applied (Ctrl+Shift+Z)");
        }
    }
}

/// Apply an undo action
fn apply_undo_action(
    action: EditorAction,
    transforms: &mut Query<&mut Transform>,
    commands: &mut Commands,
    map_editor_state: &mut MapEditorState,
) {
    match action {
        EditorAction::TransformEntity {
            entity,
            old_transform,
            new_transform,
        } => {
            if let Ok(mut transform) = transforms.get_mut(entity) {
                *transform = old_transform;
                
                // Push to redo stack
                map_editor_state.push_redo(EditorAction::TransformEntity {
                    entity,
                    old_transform,
                    new_transform,
                });
            }
        }
        
        EditorAction::TransformEntities { entities } => {
            let mut redo_entities = Vec::new();
            for (entity, old_transform, new_transform) in entities {
                if let Ok(mut transform) = transforms.get_mut(entity) {
                    *transform = old_transform;
                    redo_entities.push((entity, old_transform, new_transform));
                }
            }
            map_editor_state.push_redo(EditorAction::TransformEntities {
                entities: redo_entities,
            });
        }
        
        EditorAction::AddEntity { entity } => {
            // Undo add = delete
            commands.entity(entity).despawn();
            map_editor_state.push_redo(EditorAction::AddEntity { entity });
        }
        
        EditorAction::AddEntities { entities } => {
            for entity in &entities {
                commands.entity(*entity).despawn();
            }
            map_editor_state.push_redo(EditorAction::AddEntities { entities });
        }
        
        EditorAction::DeleteEntity {
            entity,
            transform,
            entity_type,
            serialized_data,
        } => {
            // Undo delete = recreate entity (simplified - would need proper deserialization)
            log::info!(
                "[PropertyUpdate] Would recreate entity {:?} of type {}",
                entity,
                entity_type
            );
            // This would require more complex entity recreation logic
            map_editor_state.push_redo(EditorAction::DeleteEntity {
                entity,
                transform,
                entity_type,
                serialized_data,
            });
        }
        
        EditorAction::DeleteEntities { entities } => {
            for (entity, transform, entity_type, serialized_data) in entities {
                log::info!(
                    "[PropertyUpdate] Would recreate entity {:?} of type {}",
                    entity,
                    entity_type
                );
            }
        }
        
        EditorAction::ModifyComponent {
            entity,
            component_type,
            old_value,
            new_value,
        } => {
            // Component modification undo would need component-specific handling
            log::info!(
                "[PropertyUpdate] Would undo component {} modification for {:?}: {} <- {}",
                component_type,
                entity,
                old_value,
                new_value
            );
            map_editor_state.push_redo(EditorAction::ModifyComponent {
                entity,
                component_type,
                old_value,
                new_value,
            });
        }
    }
}

/// Apply a redo action
fn apply_redo_action(
    action: EditorAction,
    transforms: &mut Query<&mut Transform>,
    commands: &mut Commands,
    map_editor_state: &mut MapEditorState,
) {
    match action {
        EditorAction::TransformEntity {
            entity,
            old_transform,
            new_transform,
        } => {
            if let Ok(mut transform) = transforms.get_mut(entity) {
                *transform = new_transform;
                
                // Push back to undo stack
                map_editor_state.push_action(EditorAction::TransformEntity {
                    entity,
                    old_transform,
                    new_transform,
                });
            }
        }
        
        EditorAction::TransformEntities { entities } => {
            let mut undo_entities = Vec::new();
            for (entity, old_transform, new_transform) in entities {
                if let Ok(mut transform) = transforms.get_mut(entity) {
                    *transform = new_transform;
                    undo_entities.push((entity, old_transform, new_transform));
                }
            }
            // Don't push to undo stack here to avoid infinite loop
            // The push_action would clear redo stack
        }
        
        EditorAction::AddEntity { entity } => {
            // Redo add = the entity should already exist
            log::info!("[PropertyUpdate] Redo AddEntity for {:?}", entity);
        }
        
        EditorAction::AddEntities { entities } => {
            log::info!("[PropertyUpdate] Redo AddEntities for {} entities", entities.len());
        }
        
        EditorAction::DeleteEntity { entity, .. } => {
            commands.entity(entity).despawn();
        }
        
        EditorAction::DeleteEntities { entities } => {
            for (entity, ..) in entities {
                commands.entity(entity).despawn();
            }
        }
        
        EditorAction::ModifyComponent {
            entity,
            component_type,
            old_value,
            new_value,
        } => {
            log::info!(
                "[PropertyUpdate] Would redo component {} modification for {:?}: {} -> {}",
                component_type,
                entity,
                old_value,
                new_value
            );
        }
    }
}

/// Plugin for property update systems
pub struct PropertyUpdatePlugin;

impl Plugin for PropertyUpdatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingPropertyChanges>()
            .add_event::<PropertyChangeEvent>()
            .add_systems(Update, (
                property_update_system,
                apply_undo_system,
            ).chain());
    }
}
