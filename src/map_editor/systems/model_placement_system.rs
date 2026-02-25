//! Model Placement System for Map Editor
//!
//! This system handles placing selected models at cursor position in the 3D world.

use bevy::{
    input::ButtonInput,
    prelude::{
        App, AssetServer, Camera, Camera3d, Commands, Entity, GlobalTransform,
        KeyCode, MouseButton, Plugin, Query, Res, ResMut, Transform, Update, Vec3, With,
        Mesh3d, MeshMaterial3d, Visibility, InheritedVisibility, ViewVisibility,
        Name, Mesh, Assets, StandardMaterial, Color, Local, Handle, Quat,
    },
    window::{PrimaryWindow, Window},
    pbr::{NotShadowCaster, NotShadowReceiver, ExtendedMaterial},
    math::primitives::Cuboid,
    ecs::schedule::IntoScheduleConfigs,
    render::{alpha::AlphaMode, view::RenderLayers},
};
use bevy_egui::EguiContexts;
use bevy_rapier3d::prelude::{CollisionGroups, Group, QueryFilter, RigidBody, Collider, AsyncCollider, ComputedColliderShape};
use bevy_rapier3d::plugin::context::systemparams::ReadRapierContext;

use crate::{
    components::{
        ZoneObject, ZoneObjectId, ZoneObjectPart, ColliderParent,
        COLLISION_FILTER_INSPECTABLE, COLLISION_FILTER_COLLIDABLE,
        COLLISION_GROUP_ZONE_OBJECT,
    },
    map_editor::{
        resources::{MapEditorState, SelectedModel, EditorMode, ModelCategory},
        components::EditorSelectable,
    },
    zone_loader::ZoneLoaderAsset,
    resources::CurrentZone,
    render::RoseObjectExtension,
    VfsResource,
};

/// Plugin for the model placement system
pub struct ModelPlacementPlugin;

impl Plugin for ModelPlacementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            model_placement_system
                .after(bevy_egui::EguiPreUpdateSet::InitContexts)
        )
        .add_systems(
            Update,
            model_preview_system
                .after(bevy_egui::EguiPreUpdateSet::InitContexts)
        )
        .add_systems(
            Update,
            add_to_zone_system
                .after(bevy_egui::EguiPreUpdateSet::InitContexts)
        );
    }
}

/// System that handles model placement when in Add mode
///
/// This system:
/// - Shows a preview of the selected model at cursor position
/// - Places the model on left click when in Add mode
/// - Uses raycast to find placement position on terrain/objects
#[allow(clippy::too_many_arguments)]
pub fn model_placement_system(
    mut commands: Commands,
    map_editor_state: Res<MapEditorState>,
    selected_model: Res<SelectedModel>,
    mut egui_ctx: EguiContexts,
    mouse_input: Res<ButtonInput<MouseButton>>,
    rapier_context: ReadRapierContext,
    query_window: Query<&Window, With<PrimaryWindow>>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    asset_server: Res<AssetServer>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: ResMut<Assets<ZoneLoaderAsset>>,
    object_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>>,
) {
    // Only run when map editor is enabled and in Add mode
    if !map_editor_state.enabled {
        return;
    }
    
    if map_editor_state.editor_mode != EditorMode::Add {
        // Only log occasionally to avoid spam
        return;
    }
    
    // Check if a model is selected
    let Some(ref model_info) = selected_model.model else {
        return;
    };
    
    // Skip if egui wants pointer input (mouse is over UI)
    if egui_ctx.ctx_mut().wants_pointer_input() {
        return;
    }
    
    // Check if we have zone data available for spawning meshes
    let Some(current_zone) = current_zone else {
        log::warn!("[MODEL PLACEMENT] No current zone loaded - cannot place model with meshes");
        return;
    };
    
    let Some(zone_data) = zone_loader_assets.get(&current_zone.handle) else {
        log::warn!("[MODEL PLACEMENT] Zone data not loaded yet - cannot place model");
        return;
    };

    // Get rapier context
    let Ok(rapier_context) = rapier_context.single() else {
        return;
    };

    // Get the primary window
    let Ok(window) = query_window.get_single() else {
        return;
    };

    // Get cursor position
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Get camera and cast ray
    for (camera, camera_transform) in query_camera.iter() {
        let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
            continue;
        };

        // Cast ray to find placement position
        let hit_result = rapier_context.cast_ray(
            ray.origin,
            *ray.direction,
            10000000.0,
            true, // Get closest hit for placement
            QueryFilter::new().groups(CollisionGroups::new(
                COLLISION_FILTER_INSPECTABLE | COLLISION_FILTER_COLLIDABLE,
                Group::all(),
            )),
        );

        // Get placement position (either hit point or default)
        let placement_position = if let Some((_entity, distance)) = hit_result {
            ray.origin + *ray.direction * distance
        } else {
            // No hit - use a default position or skip
            // Could also project to a plane at y=0
            let t = -ray.origin.y / ray.direction.y;
            if t > 0.0 {
                ray.origin + *ray.direction * t
            } else {
                continue; // Can't place without a valid position
            }
        };

        // Handle left click for placement
        if mouse_input.just_pressed(MouseButton::Left) {
            log::info!(
                "[MODEL PLACEMENT] Left click detected! Placing model '{}' (ID: {}) at WORLD position {:?}",
                model_info.name,
                model_info.id,
                placement_position
            );
            
            // CRITICAL: The placement_position is in WORLD coordinates from the raycast.
            // The entity is NOT parented to the zone entity, so it needs WORLD coordinates
            // for its Transform to render at the correct position.
            //
            // The save_system will convert world coordinates to local coordinates when
            // calculating block positions for saving.
            //
            // Zone center is at world position (5200, 0, -5200)
            // For saving: local = world - zone_center
            //   block_x = ((local_x + 5200.0) / 160.0).floor()
            //   block_y = ((local_z + 5200.0) / 160.0).floor()
            
            // Place the model with full mesh spawning using WORLD coordinates
            // (entity is not parented to zone, so transform is in world space)
            place_model_at_position(
                &mut commands,
                &asset_server,
                object_materials.into_inner(),
                zone_data,
                model_info,
                placement_position,  // Use WORLD coordinates for rendering
            );
            
            log::info!(
                "[MODEL PLACEMENT] Successfully placed model '{}' (ID: {}) at WORLD position {:?}",
                model_info.name,
                model_info.id,
                placement_position
            );
        }

        // Only process the first camera
        break;
    }
}

/// Place a model at the specified position with full visual mesh spawning
#[allow(clippy::too_many_arguments)]
fn place_model_at_position(
    commands: &mut Commands,
    asset_server: &AssetServer,
    object_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
    zone_data: &ZoneLoaderAsset,
    model_info: &crate::map_editor::resources::ModelInfo,
    position: Vec3,
) {
    use rose_file_readers::ZscCollisionFlags;
    
    let zsc_object_id = model_info.id as usize;
    
    // Get the appropriate ZSC file based on category
    let zsc = match model_info.category {
        ModelCategory::Deco => &zone_data.zsc_deco,
        ModelCategory::Cnst => &zone_data.zsc_cnst,
        ModelCategory::Event | ModelCategory::Special | ModelCategory::All => {
            log::warn!(
                "[MODEL PLACEMENT] Category {:?} not fully supported yet, using deco ZSC",
                model_info.category
            );
            &zone_data.zsc_deco
        }
    };
    
    // Check if the object ID is valid
    if zsc_object_id >= zsc.objects.len() {
        log::error!(
            "[MODEL PLACEMENT] Invalid zsc_object_id {} (max: {})",
            zsc_object_id,
            zsc.objects.len().saturating_sub(1)
        );
        return;
    }
    
    let object = &zsc.objects[zsc_object_id];
    
    // Determine the ZoneObject type based on category
    let object_type = match model_info.category {
        ModelCategory::Deco => ZoneObject::DecoObject(ZoneObjectId {
            ifo_object_id: 0, // Will be assigned properly when saved
            zsc_object_id,
        }),
        ModelCategory::Cnst => ZoneObject::CnstObject(ZoneObjectId {
            ifo_object_id: 0,
            zsc_object_id,
        }),
        ModelCategory::Event => ZoneObject::EventObject(ZoneObjectId {
            ifo_object_id: 0,
            zsc_object_id,
        }),
        ModelCategory::Special => ZoneObject::WarpObject(ZoneObjectId {
            ifo_object_id: 0,
            zsc_object_id,
        }),
        ModelCategory::All => ZoneObject::DecoObject(ZoneObjectId {
            ifo_object_id: 0,
            zsc_object_id,
        }),
    };
    
    // Create the parent entity with transform
    let object_transform = Transform::from_translation(position);
    
    let mut object_entity_commands = commands.spawn((
        object_type,
        object_transform,
        GlobalTransform::default(),
        Visibility::Visible,
        InheritedVisibility::default(),
        ViewVisibility::default(),
        Name::new(format!("Placed: {}", model_info.name)),
        EditorSelectable,
        EditorPlacedObject {
            model_id: model_info.id,
            category: model_info.category,
            placed_at: std::time::Instant::now(),
        },
        RigidBody::Fixed,
    ));
    
    let object_entity = object_entity_commands.id();
    
    // Spawn each part of the object with mesh and material
    let mut mesh_cache: Vec<Option<Handle<bevy::render::mesh::Mesh>>> = vec![None; zsc.meshes.len()];
    
    for (part_index, object_part) in object.parts.iter().enumerate() {
        let part_transform = Transform::default()
            .with_translation(Vec3::new(
                object_part.position.x,
                object_part.position.z,
                -object_part.position.y,
            ) / 100.0)
            .with_rotation(Quat::from_xyzw(
                object_part.rotation.x,
                object_part.rotation.z,
                -object_part.rotation.y,
                object_part.rotation.w,
            ))
            .with_scale(Vec3::new(
                object_part.scale.x,
                object_part.scale.z,
                object_part.scale.y,
            ));

        let mesh_id = object_part.mesh_id as usize;

        // Validate mesh_id bounds
        if mesh_id >= zsc.meshes.len() {
            log::warn!(
                "[MODEL PLACEMENT] Part {} has invalid mesh_id {} (max: {}), skipping",
                part_index, mesh_id, zsc.meshes.len().saturating_sub(1)
            );
            continue;
        }

        // Validate material_id bounds
        let material_id = object_part.material_id as usize;
        if material_id >= zsc.materials.len() {
            log::warn!(
                "[MODEL PLACEMENT] Part {} has invalid material_id {} (max: {}), skipping",
                part_index, material_id, zsc.materials.len().saturating_sub(1)
            );
            continue;
        }

        // Load or get cached mesh
        let mesh = mesh_cache[mesh_id].clone().unwrap_or_else(|| {
            let mesh_path = zsc.meshes[mesh_id].path().to_string_lossy().into_owned();
            let handle = asset_server.load(&mesh_path);
            mesh_cache[mesh_id] = Some(handle.clone());
            handle
        });

        // Get material info
        let zsc_material = &zsc.materials[material_id];
        let material_path = zsc_material.path.path().to_string_lossy().into_owned();

        // Load base texture
        let base_texture_handle = if material_path.is_empty() || material_path == "NULL" {
            asset_server.load("ETC/SPECULAR_SPHEREMAP.DDS")
        } else {
            asset_server.load(&material_path)
        };

        // Create material with proper settings
        let material = object_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color_texture: Some(base_texture_handle),
                unlit: false,  // Enable PBR lighting
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
                lightmap_params: Vec3::new(0.0, 0.0, 1.0).extend(0.0), // No lightmap for placed objects
                lightmap_texture: None,
                specular_texture: None, // No specular for placed objects
            },
        });

        // Determine collision settings
        let mut collision_filter = COLLISION_FILTER_INSPECTABLE;
        if object_part.collision_shape.is_some() {
            if !object_part.collision_flags.contains(ZscCollisionFlags::HEIGHT_ONLY) {
                collision_filter |= COLLISION_FILTER_COLLIDABLE;
            }
            if !object_part.collision_flags.contains(ZscCollisionFlags::NOT_PICKABLE) {
                collision_filter |= Group::from_bits_retain(1 << 4); // COLLISION_FILTER_CLICKABLE
            }
        }

        // Determine if transparent for shadow casting
        let is_transparent = zsc_material.alpha_enabled && zsc_material.alpha_test.is_none();

        let mut part_cmd = commands.spawn((
            EditorSelectable,
            ZoneObjectPart {
                ifo_object_id: 0,
                zsc_object_id,
                zsc_part_id: part_index,
                mesh_path: zsc.meshes[mesh_id].path().to_string_lossy().into(),
                collision_shape: (&object_part.collision_shape).into(),
                collision_not_moveable: object_part.collision_flags.contains(ZscCollisionFlags::NOT_MOVEABLE),
                collision_not_pickable: object_part.collision_flags.contains(ZscCollisionFlags::NOT_PICKABLE),
                collision_height_only: object_part.collision_flags.contains(ZscCollisionFlags::HEIGHT_ONLY),
                collision_no_camera: object_part.collision_flags.contains(ZscCollisionFlags::NOT_CAMERA_COLLISION),
            },
            Mesh3d(mesh),
            MeshMaterial3d(material),
            part_transform,
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));

        // Insert individually so the compiler identifies the exact failing component
        part_cmd.insert(bevy::render::view::NoFrustumCulling);
        part_cmd.insert(bevy::render::primitives::Aabb::from_min_max(
            Vec3::splat(-100000.0),
            Vec3::splat(100000.0),
        ));
        part_cmd.insert(RenderLayers::layer(0));
        part_cmd.insert(ColliderParent::new(object_entity));
        part_cmd.insert(AsyncCollider(ComputedColliderShape::TriMesh(
            bevy_rapier3d::prelude::TriMeshFlags::FIX_INTERNAL_EDGES,
        )));
        part_cmd.insert(CollisionGroups::new(
            COLLISION_GROUP_ZONE_OBJECT,
            collision_filter,
        ));

        let part_entity = part_cmd.id();
        
        // Disable shadow casting for transparent materials
        if is_transparent {
            commands.entity(part_entity).insert(NotShadowCaster);
        }

        // Add the part as a child of the object entity
        commands.entity(object_entity).add_child(part_entity);
    }
    
    log::info!(
        "[MODEL PLACEMENT] Created entity {:?} for model '{}' with {} parts at {:?}",
        object_entity,
        model_info.name,
        object.parts.len(),
        position
    );
}

/// Component to mark objects placed by the editor
#[derive(bevy::prelude::Component, Debug, Clone)]
pub struct EditorPlacedObject {
    /// ID of the model that was placed
    pub model_id: u32,
    /// Category of the placed model
    pub category: ModelCategory,
    /// When the object was placed
    pub placed_at: std::time::Instant,
}

/// System to show a preview of the model at cursor position
/// This is a visual-only system that shows where the model will be placed
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn model_preview_system(
    mut commands: Commands,
    map_editor_state: Res<MapEditorState>,
    selected_model: Res<SelectedModel>,
    mut egui_ctx: EguiContexts,
    rapier_context: ReadRapierContext,
    query_window: Query<&Window, With<PrimaryWindow>>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut preview_entity: Local<Option<Entity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Only run when map editor is enabled and in Add mode with a selected model
    if !map_editor_state.enabled 
        || map_editor_state.editor_mode != EditorMode::Add 
        || selected_model.model.is_none() 
    {
        // Hide/remove preview if it exists
        if let Some(entity) = *preview_entity {
            commands.entity(entity).despawn();
            *preview_entity = None;
        }
        return;
    }
    
    // Skip if egui wants pointer input
    if egui_ctx.ctx_mut().wants_pointer_input() {
        return;
    }

    // Get rapier context
    let Ok(rapier_context) = rapier_context.single() else {
        return;
    };

    // Get window and cursor
    let Ok(window) = query_window.get_single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Get camera and cast ray
    for (camera, camera_transform) in query_camera.iter() {
        let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
            continue;
        };

        // Cast ray to find placement position
        let hit_result = rapier_context.cast_ray(
            ray.origin,
            *ray.direction,
            10000000.0,
            true,
            QueryFilter::new().groups(CollisionGroups::new(
                COLLISION_FILTER_INSPECTABLE | COLLISION_FILTER_COLLIDABLE,
                Group::all(),
            )),
        );

        // Get placement position
        let placement_position = if let Some((_, distance)) = hit_result {
            ray.origin + *ray.direction * distance
        } else {
            let t = -ray.origin.y / ray.direction.y;
            if t > 0.0 {
                ray.origin + *ray.direction * t
            } else {
                // Hide preview if no valid position
                if let Some(entity) = *preview_entity {
                    commands.entity(entity).despawn();
                    *preview_entity = None;
                }
                continue;
            }
        };

        // Create or update preview entity
        // Use a simple wireframe cube as preview
        if preview_entity.is_none() {
            // Create a simple preview mesh (cube)
            let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
            let material = materials.add(StandardMaterial {
                base_color: Color::srgba(0.0, 1.0, 0.0, 0.5),
                alpha_mode: bevy::render::alpha::AlphaMode::Blend,
                unlit: true,
                ..Default::default()
            });
            
            let entity = commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_translation(placement_position),
                Visibility::Visible,
                NotShadowCaster,
                NotShadowReceiver,
                Name::new("Model Preview"),
            )).id();
            
            *preview_entity = Some(entity);
        } else if let Some(entity) = *preview_entity {
            // Update position
            commands.entity(entity).insert(Transform::from_translation(placement_position));
        }

        break;
    }
}

/// System that handles "Add to Zone" button clicks from the model browser
/// This places the selected model at a default position in front of the camera
#[allow(clippy::too_many_arguments)]
pub fn add_to_zone_system(
    mut commands: Commands,
    map_editor_state: Res<MapEditorState>,
    mut selected_model: ResMut<SelectedModel>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    asset_server: Res<AssetServer>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: ResMut<Assets<ZoneLoaderAsset>>,
    object_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>>,
) {
    // Only run when map editor is enabled
    if !map_editor_state.enabled {
        return;
    }
    
    // Check if there's a pending placement request
    if !selected_model.take_pending_placement() {
        return;
    }
    
    // Check if a model is selected
    let Some(ref model_info) = selected_model.model else {
        log::warn!("[ADD TO ZONE] No model selected for placement");
        return;
    };
    
    // Check if we have zone data available for spawning meshes
    let Some(current_zone) = current_zone else {
        log::warn!("[ADD TO ZONE] No current zone loaded - cannot place model with meshes");
        return;
    };
    
    let Some(zone_data) = zone_loader_assets.get(&current_zone.handle) else {
        log::warn!("[ADD TO ZONE] Zone data not loaded yet - cannot place model");
        return;
    };
    
    // Get camera position to place model in front of it
    // Default to zone center in WORLD coordinates
    let zone_center_world = Vec3::new(5200.0, 0.0, -5200.0);
    let mut world_position = zone_center_world;
    
    if let Ok((_camera, camera_transform)) = query_camera.get_single() {
        let camera_pos = camera_transform.translation();
        let camera_forward = camera_transform.forward();
        
        // Place 10 units in front of the camera, at ground level
        world_position = camera_pos + camera_forward * 10.0;
        world_position.y = 0.0; // Snap to ground level
    }
    
    // CRITICAL: The entity is NOT parented to the zone entity, so it needs WORLD coordinates
    // for its Transform to render at the correct position.
    // The save_system will convert world coordinates to local coordinates when saving.
    
    log::info!(
        "[ADD TO ZONE] Placing model '{}' (ID: {}) at WORLD position {:?}",
        model_info.name,
        model_info.id,
        world_position
    );
    
    // Place the model with full mesh spawning using WORLD coordinates
    place_model_at_position(
        &mut commands,
        &asset_server,
        object_materials.into_inner(),
        zone_data,
        model_info,
        world_position,  // Use WORLD coordinates for rendering
    );
    
    // Note: The pending_placement flag is already cleared by take_pending_placement()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_placement_system_exists() {
        // Basic test to ensure the module compiles
        assert!(true);
    }
}
