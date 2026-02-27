//! Duplicate System for Map Editor
//!
//! Handles duplicating selected zone objects with all their components,
//! including meshes, materials, collision, and child parts.

use bevy::{
    prelude::*,
    pbr::{NotShadowCaster, ExtendedMaterial},
    render::{alpha::AlphaMode, view::RenderLayers},
};
use bevy_rapier3d::prelude::{CollisionGroups, Group, RigidBody, Collider, AsyncCollider, ComputedColliderShape};

use crate::components::{
    ZoneObject, ZoneObjectId, ZoneObjectPart, ColliderParent,
    COLLISION_FILTER_INSPECTABLE, COLLISION_FILTER_COLLIDABLE,
    COLLISION_GROUP_ZONE_OBJECT,
};
use crate::map_editor::{
    components::{EditorSelectable, SelectedInEditor},
    resources::{DuplicateSelectedEvent, EditorAction, EditorMode, MapEditorState, ModelCategory},
    systems::model_placement_system::EditorPlacedObject,
};
use crate::render::RoseObjectExtension;
use crate::resources::CurrentZone;
use crate::zone_loader::ZoneLoaderAsset;

/// Plugin for the duplicate system
pub struct DuplicateSystemPlugin;

impl Plugin for DuplicateSystemPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DuplicateSelectedEvent>()
            .add_systems(Update, handle_duplicate_event);
    }
}

/// System to handle duplicate events
///
/// This system listens for DuplicateSelectedEvent and duplicates all currently
/// selected entities with their full component data including meshes and collision.
#[allow(clippy::too_many_arguments)]
pub fn handle_duplicate_event(
    mut commands: Commands,
    mut events: EventReader<DuplicateSelectedEvent>,
    mut map_editor_state: ResMut<MapEditorState>,
    selected_entities: Query<Entity, With<SelectedInEditor>>,
    transforms: Query<&GlobalTransform>,
    zone_objects: Query<&ZoneObject>,
    names: Query<&Name>,
    children_query: Query<&Children>,
    part_query: Query<(
        &Transform,
        &ZoneObjectPart,
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>>,
    )>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut object_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    asset_server: Res<AssetServer>,
) {
    for event in events.read() {
        let entities: Vec<Entity> = selected_entities.iter().collect();
        
        if entities.is_empty() {
            log::debug!("[DuplicateSystem] No entities selected for duplication");
            continue;
        }
        
        log::info!("[DuplicateSystem] Duplicating {} entities with offset {:?}", entities.len(), event.offset);
        
        // Remove selection from current entities
        for entity in &entities {
            commands.entity(*entity).remove::<SelectedInEditor>();
        }
        
        // Track new entities for selection and undo
        let mut new_entities = Vec::new();
        
        // Get zone data for mesh loading if available
        let zone_data = current_zone.as_ref().and_then(|z| zone_loader_assets.get(&z.handle));
        
        for entity in &entities {
            // Get the original transform
            let Ok(global_transform) = transforms.get(*entity) else {
                log::warn!("[DuplicateSystem] Could not get transform for entity {:?}", entity);
                continue;
            };
            
            let original_translation = global_transform.translation();
            let new_translation = original_translation + event.offset;
            let new_transform = Transform::from_translation(new_translation)
                .with_rotation(global_transform.rotation())
                .with_scale(global_transform.scale());
            
            // Get the original name
            let new_name = names.get(*entity).ok()
                .map(|n| format!("{}_copy", n.as_str()))
                .unwrap_or_else(|| "DuplicatedEntity".to_string());
            
            // Get the ZoneObject component
            let zone_object = zone_objects.get(*entity).ok();
            
            // Create the duplicated parent entity
            let mut entity_commands = commands.spawn((
                new_transform,
                GlobalTransform::default(),
                Name::new(new_name.clone()),
                EditorSelectable,
                SelectedInEditor,
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ));
            
            // Copy ZoneObject component if present
            if let Some(zone_obj) = zone_object {
                let duplicated_zone_obj = duplicate_zone_object(zone_obj);
                entity_commands.insert(duplicated_zone_obj);
                
                // Add EditorPlacedObject marker with info from zone object
                if let Some((model_id, category)) = get_model_info_from_zone_object(zone_obj) {
                    entity_commands.insert(EditorPlacedObject {
                        model_id,
                        category,
                        placed_at: std::time::Instant::now(),
                    });
                }
                
                // Add RigidBody for physics
                entity_commands.insert(RigidBody::Fixed);
            }
            
            let new_entity = entity_commands.id();
            new_entities.push(new_entity);
            
            // Duplicate child parts (meshes, collision, etc.)
            if let Ok(children) = children_query.get(*entity) {
                duplicate_child_parts(
                    &mut commands,
                    new_entity,
                    children,
                    &children_query,
                    &part_query,
                    &mut mesh_assets,
                    &mut object_materials,
                    zone_data,
                    &asset_server,
                );
            }
            
            log::info!(
                "[DuplicateSystem] Created duplicate entity {:?} '{}' at position {:?}",
                new_entity,
                new_name.clone(),
                new_translation
            );
        }
        
        // Clear old selection and set new selection
        map_editor_state.clear_selection();
        for entity in &new_entities {
            map_editor_state.select_entity(*entity);
        }
        
        // Record the action for undo
        if !new_entities.is_empty() {
            map_editor_state.push_action(EditorAction::AddEntities {
                entities: new_entities.clone(),
            });
        }
        
        log::info!("[DuplicateSystem] Duplicated {} entities", new_entities.len());
    }
}

/// Duplicate a ZoneObject component with a fresh IFO object ID
fn duplicate_zone_object(zone_obj: &ZoneObject) -> ZoneObject {
    match zone_obj {
        ZoneObject::DecoObject(id) => ZoneObject::DecoObject(ZoneObjectId {
            ifo_object_id: 0, // Will be assigned when saved
            zsc_object_id: id.zsc_object_id,
        }),
        ZoneObject::CnstObject(id) => ZoneObject::CnstObject(ZoneObjectId {
            ifo_object_id: 0,
            zsc_object_id: id.zsc_object_id,
        }),
        ZoneObject::EventObject(id) => ZoneObject::EventObject(ZoneObjectId {
            ifo_object_id: 0,
            zsc_object_id: id.zsc_object_id,
        }),
        ZoneObject::WarpObject(id) => ZoneObject::WarpObject(ZoneObjectId {
            ifo_object_id: 0,
            zsc_object_id: id.zsc_object_id,
        }),
        ZoneObject::DecoObjectPart(part) => ZoneObject::DecoObjectPart(ZoneObjectPart {
            ifo_object_id: 0,
            zsc_object_id: part.zsc_object_id,
            zsc_part_id: part.zsc_part_id,
            mesh_path: part.mesh_path.clone(),
            collision_shape: part.collision_shape.clone(),
            collision_not_moveable: part.collision_not_moveable,
            collision_not_pickable: part.collision_not_pickable,
            collision_height_only: part.collision_height_only,
            collision_no_camera: part.collision_no_camera,
        }),
        ZoneObject::CnstObjectPart(part) => ZoneObject::CnstObjectPart(ZoneObjectPart {
            ifo_object_id: 0,
            zsc_object_id: part.zsc_object_id,
            zsc_part_id: part.zsc_part_id,
            mesh_path: part.mesh_path.clone(),
            collision_shape: part.collision_shape.clone(),
            collision_not_moveable: part.collision_not_moveable,
            collision_not_pickable: part.collision_not_pickable,
            collision_height_only: part.collision_height_only,
            collision_no_camera: part.collision_no_camera,
        }),
        ZoneObject::EventObjectPart(part) => ZoneObject::EventObjectPart(ZoneObjectPart {
            ifo_object_id: 0,
            zsc_object_id: part.zsc_object_id,
            zsc_part_id: part.zsc_part_id,
            mesh_path: part.mesh_path.clone(),
            collision_shape: part.collision_shape.clone(),
            collision_not_moveable: part.collision_not_moveable,
            collision_not_pickable: part.collision_not_pickable,
            collision_height_only: part.collision_height_only,
            collision_no_camera: part.collision_no_camera,
        }),
        ZoneObject::WarpObjectPart(part) => ZoneObject::WarpObjectPart(ZoneObjectPart {
            ifo_object_id: 0,
            zsc_object_id: part.zsc_object_id,
            zsc_part_id: part.zsc_part_id,
            mesh_path: part.mesh_path.clone(),
            collision_shape: part.collision_shape.clone(),
            collision_not_moveable: part.collision_not_moveable,
            collision_not_pickable: part.collision_not_pickable,
            collision_height_only: part.collision_height_only,
            collision_no_camera: part.collision_no_camera,
        }),
        ZoneObject::AnimatedObject(obj) => ZoneObject::AnimatedObject(obj.clone()),
        ZoneObject::Terrain(terrain) => ZoneObject::Terrain(terrain.clone()),
        ZoneObject::EffectObject { ifo_object_id: _, effect_path } => ZoneObject::EffectObject {
            ifo_object_id: 0,
            effect_path: effect_path.clone(),
        },
        ZoneObject::SoundObject { ifo_object_id: _, sound_path } => ZoneObject::SoundObject {
            ifo_object_id: 0,
            sound_path: sound_path.clone(),
        },
        ZoneObject::Water => ZoneObject::Water,
    }
}

/// Get model info from ZoneObject for EditorPlacedObject
fn get_model_info_from_zone_object(zone_obj: &ZoneObject) -> Option<(u32, ModelCategory)> {
    match zone_obj {
        ZoneObject::DecoObject(id) => Some((id.zsc_object_id as u32, ModelCategory::Deco)),
        ZoneObject::CnstObject(id) => Some((id.zsc_object_id as u32, ModelCategory::Cnst)),
        ZoneObject::EventObject(id) => Some((id.zsc_object_id as u32, ModelCategory::Event)),
        ZoneObject::WarpObject(id) => Some((id.zsc_object_id as u32, ModelCategory::Special)),
        ZoneObject::DecoObjectPart(part) => Some((part.zsc_object_id as u32, ModelCategory::Deco)),
        ZoneObject::CnstObjectPart(part) => Some((part.zsc_object_id as u32, ModelCategory::Cnst)),
        ZoneObject::EventObjectPart(part) => Some((part.zsc_object_id as u32, ModelCategory::Event)),
        ZoneObject::WarpObjectPart(part) => Some((part.zsc_object_id as u32, ModelCategory::Special)),
        _ => None,
    }
}

/// Duplicate child parts (meshes, collision) from original entity to duplicate
#[allow(clippy::too_many_arguments)]
fn duplicate_child_parts(
    commands: &mut Commands,
    parent_entity: Entity,
    children: &Children,
    children_query: &Query<&Children>,
    part_query: &Query<(
        &Transform,
        &ZoneObjectPart,
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>>,
    )>,
    mesh_assets: &mut Assets<Mesh>,
    object_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
    zone_data: Option<&ZoneLoaderAsset>,
    asset_server: &AssetServer,
) {
    for child in children.iter() {
        // Try to get part data
        if let Ok((transform, part, mesh, material)) = part_query.get(child) {
            // Create duplicate part
            let mut part_commands = commands.spawn((
                EditorSelectable,
                ZoneObjectPart {
                    ifo_object_id: 0,
                    zsc_object_id: part.zsc_object_id,
                    zsc_part_id: part.zsc_part_id,
                    mesh_path: part.mesh_path.clone(),
                    collision_shape: part.collision_shape.clone(),
                    collision_not_moveable: part.collision_not_moveable,
                    collision_not_pickable: part.collision_not_pickable,
                    collision_height_only: part.collision_height_only,
                    collision_no_camera: part.collision_no_camera,
                },
                *transform,
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ));
            
            // Copy or reload mesh
            if let Some(mesh_handle) = mesh {
                part_commands.insert(mesh_handle.clone());
            } else if let Some(zd) = zone_data {
                // Try to load mesh from zone data
                let mesh_path = &part.mesh_path;
                if !mesh_path.is_empty() {
                    let mesh_handle: Handle<Mesh> = asset_server.load(mesh_path);
                    part_commands.insert(Mesh3d(mesh_handle));
                }
            }
            
            // Copy or reload material
            if let Some(material_handle) = material {
                part_commands.insert(material_handle.clone());
            } else if let Some(zd) = zone_data {
                // Try to load material from zone data
                load_material_for_part(
                    &mut part_commands,
                    part.zsc_object_id,
                    part.zsc_part_id,
                    zd,
                    asset_server,
                    object_materials,
                );
            }
            
            // Add rendering components
            part_commands.insert(bevy::render::view::NoFrustumCulling);
            part_commands.insert(bevy::render::primitives::Aabb::from_min_max(
                Vec3::splat(-100000.0),
                Vec3::splat(100000.0),
            ));
            part_commands.insert(RenderLayers::layer(0));
            
            // Add collision components
            part_commands.insert(ColliderParent::new(parent_entity));
            part_commands.insert(AsyncCollider(ComputedColliderShape::TriMesh(
                bevy_rapier3d::prelude::TriMeshFlags::FIX_INTERNAL_EDGES,
            )));
            
            // Determine collision filter
            let mut collision_filter = COLLISION_FILTER_INSPECTABLE;
            if part.collision_shape != crate::components::ZoneObjectPartCollisionShape::None {
                if !part.collision_height_only {
                    collision_filter |= COLLISION_FILTER_COLLIDABLE;
                }
                if !part.collision_not_pickable {
                    collision_filter |= Group::from_bits_retain(1 << 4); // COLLISION_FILTER_CLICKABLE
                }
            }
            
            part_commands.insert(CollisionGroups::new(
                COLLISION_GROUP_ZONE_OBJECT,
                collision_filter,
            ));
            
            let part_entity = part_commands.id();
            
            // Add as child of parent
            commands.entity(parent_entity).add_child(part_entity);
            
            // Recursively handle nested children
            if let Ok(nested_children) = children_query.get(child) {
                duplicate_child_parts(
                    commands,
                    parent_entity,
                    nested_children,
                    children_query,
                    part_query,
                    mesh_assets,
                    object_materials,
                    zone_data,
                    asset_server,
                );
            }
        }
    }
}

/// Load material for a part from zone data
#[allow(clippy::too_many_arguments)]
fn load_material_for_part(
    part_commands: &mut EntityCommands,
    zsc_object_id: usize,
    zsc_part_id: usize,
    zone_data: &ZoneLoaderAsset,
    asset_server: &AssetServer,
    object_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
) {
    // Try deco ZSC first, then cnst
    let zsc = if zsc_object_id < zone_data.zsc_deco.objects.len() {
        &zone_data.zsc_deco
    } else {
        &zone_data.zsc_cnst
    };
    
    if zsc_object_id >= zsc.objects.len() {
        return;
    }
    
    let object = &zsc.objects[zsc_object_id];
    if zsc_part_id >= object.parts.len() {
        return;
    }
    
    let object_part = &object.parts[zsc_part_id];
    let material_id = object_part.material_id as usize;
    
    if material_id >= zsc.materials.len() {
        return;
    }
    
    let zsc_material = &zsc.materials[material_id];
    let material_path = zsc_material.path.path().to_string_lossy().into_owned();
    
    // Load base texture
    let base_texture_handle = if material_path.is_empty() || material_path == "NULL" {
        asset_server.load("ETC/SPECULAR_SPHEREMAP.DDS")
    } else {
        asset_server.load(&material_path)
    };
    
    // Create material
    let material = object_materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color_texture: Some(base_texture_handle),
            unlit: false,
            double_sided: zsc_material.two_sided,
            perceptual_roughness: 0.8,
            metallic: 0.0,
            alpha_mode: if zsc_material.alpha_enabled {
                if let Some(threshold) = zsc_material.alpha_test {
                    AlphaMode::Mask(threshold)
                } else {
                    AlphaMode::Blend
                }
            } else {
                AlphaMode::Opaque
            },
            ..Default::default()
        },
        extension: RoseObjectExtension {
            lightmap_params: Vec3::new(0.0, 0.0, 1.0).extend(0.0),
            lightmap_texture: None,
            specular_texture: None,
        },
    });
    
    part_commands.insert(MeshMaterial3d(material));
    
    // Disable shadow casting for transparent materials
    if zsc_material.alpha_enabled && zsc_material.alpha_test.is_none() {
        part_commands.insert(NotShadowCaster);
    }
}
