//! Map Editor UI Module
//!
//! This module contains egui-based UI panels for the map editor.
//!
//! # Panel Layout
//!
//! ```text
//! +--------------------------------------------------+
//! | Menu Bar                                         |
//! +------------+---------------------+---------------+
//! | Hierarchy  |                     | Properties    |
//! | Panel      |    3D Viewport      | Panel         |
//! | (Left)     |                     | (Right)       |
//! |            |                     |               |
//! +------------+---------------------+---------------+
//! | Model Browser Panel                              |
//! +--------------------------------------------------+
//! | Status Bar                                       |
//! +--------------------------------------------------+
//! ```

pub mod menu_bar;
pub mod hierarchy_panel;
pub mod model_browser_panel;
pub mod properties_panel;
pub mod status_bar;
pub mod zone_list_panel;

use bevy::{ecs::{schedule::IntoScheduleConfigs, system::SystemParam}, prelude::*};
use bevy_egui::{egui, EguiContexts};
use bevy_rapier3d::prelude::{Collider, CollisionGroups, RigidBody};
use std::io::Write;
use std::path::PathBuf;

use crate::components::{
    EventObject, MapEditorTerrainBlock, MapEditorWaterPlane, WarpObject, Zone, ZoneObject,
    COLLISION_FILTER_INSPECTABLE, COLLISION_GROUP_ZONE_WATER,
};
use crate::map_editor::components::SelectedInEditor;
use crate::map_editor::resources::{AvailableModels, DuplicateSelectedEvent, EditorMode, HierarchyFilter, MapEditorState, SelectedModel};
use crate::map_editor::systems::property_update_system::PropertyChangeEvent;
use crate::map_editor::save::{SaveZoneEvent, SaveStatus};
use crate::map_editor::save::ifo_export::export_ifo_block;
use crate::map_editor::save::ifo_types::IfoBlock;
use crate::resources::{CurrentZone, GameData};
use crate::events::LoadZoneEvent;
use crate::render::WaterMaterial;
use crate::zone_loader::ZoneLoaderAsset;
use crate::VfsResource;
use rose_data::ZoneId;

use menu_bar::editor_menu_bar;
use menu_bar::{HelpWindowState, NewZoneDialogState, SaveVersionDialogState};
use hierarchy_panel::{editor_hierarchy_panel, HierarchyQuery};
use model_browser_panel::editor_model_browser_panel;
use status_bar::editor_status_bar;
use zone_list_panel::{ZoneListPanelState, zone_list_panel_system};

// Re-export the standalone properties panel function
pub use properties_panel::{
    editor_properties_panel, EntityDataQuery, PendingPropertyEdits,
};

/// System parameter combining queries needed by the properties panel
#[derive(SystemParam)]
pub struct PropertiesQueries<'w, 's> {
    pub name_query: Query<'w, 's, &'static Name>,
    pub transform_query: Query<'w, 's, &'static Transform>,
}

/// System parameter grouping menu/UI mutable state to keep system parameter count manageable.
#[derive(SystemParam)]
pub struct EditorUiMenuState<'w, 's> {
    pub zone_list_state: ResMut<'w, ZoneListPanelState>,
    pub new_zone_events: MessageWriter<'w, NewZoneEvent>,
    pub add_water_events: MessageWriter<'w, AddWaterPlaneEvent>,
    pub help_state: ResMut<'w, HelpWindowState>,
    pub selected_model: ResMut<'w, SelectedModel>,
    pub save_version_dialog_state: ResMut<'w, SaveVersionDialogState>,
    pub new_zone_dialog_state: ResMut<'w, NewZoneDialogState>,
    pub _phantom: std::marker::PhantomData<&'s ()>,
}

/// Plugin for the map editor UI systems
pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingPropertyEdits>()
            .init_resource::<SelectedModel>()
            .init_resource::<ZoneListPanelState>()
            .init_resource::<HelpWindowState>()
            .init_resource::<SaveVersionDialogState>()
            .init_resource::<NewZoneDialogState>()
            .add_message::<PropertyChangeEvent>()
            .add_message::<NewZoneEvent>()
            .add_message::<AddWaterPlaneEvent>()
            // Map editor UI systems must run in EguiPrimaryContextPass for bevy_egui 0.39
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                editor_ui_system.run_if(resource_exists::<MapEditorState>),
            )
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                model_browser_panel_system.run_if(resource_exists::<AvailableModels>),
            )
            // Keyboard shortcuts don't render UI, can stay in Update
            .add_systems(
                Update,
                model_browser_panel::model_browser_keyboard_shortcuts.run_if(resource_exists::<SelectedModel>),
            )
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                zone_list_panel_system.run_if(resource_exists::<MapEditorState>),
            )
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                new_zone_system.run_if(resource_exists::<MapEditorState>),
            )
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                add_water_plane_system.run_if(resource_exists::<MapEditorState>),
            );
        
        log::info!("[EditorUiPlugin] Editor UI plugin initialized with model browser, zone list, and new zone handler");
    }
}

/// Message to request creating a new zone
#[derive(Message)]
pub struct NewZoneEvent {
    /// Whether to prompt for unsaved changes
    pub prompt_if_modified: bool,
    /// Zone id to create / bootstrap
    pub zone_id: u16,
    /// Optional output folder to bootstrap map files into
    pub output_path: Option<PathBuf>,
    /// Whether to initialize a default 0_0 block set (HIM/TIL/IFO)
    pub initialize_default_block: bool,
}

impl NewZoneEvent {
    pub fn new() -> Self {
        Self {
            prompt_if_modified: true,
            zone_id: 1,
            output_path: None,
            initialize_default_block: true,
        }
    }
}

/// Message to add a default water plane into the currently loaded zone.
#[derive(Message, Default)]
pub struct AddWaterPlaneEvent;

/// Main UI system that renders all editor panels
///
/// This system only renders when `MapEditorState::enabled` is true.
#[allow(clippy::too_many_arguments)]
pub fn editor_ui_system(
    mut contexts: EguiContexts,
    mut map_editor_state: ResMut<MapEditorState>,
    game_data: Res<GameData>,
    save_status: Res<SaveStatus>,
    current_zone: Option<Res<CurrentZone>>,
    custom_zone_path: Option<Res<crate::map_editor::resources::CustomZonePath>>,
    mut save_events: MessageWriter<SaveZoneEvent>,
    entity_data: EntityDataQuery,
    hierarchy_query: HierarchyQuery,
    mut pending_edits: ResMut<PendingPropertyEdits>,
    queries: PropertiesQueries,
    mut property_change_event: MessageWriter<PropertyChangeEvent>,
    mut duplicate_event: MessageWriter<DuplicateSelectedEvent>,
    mut menu_state: EditorUiMenuState,
    mut commands: Commands,
) {
    // Only render UI when editor is enabled
    if !map_editor_state.enabled {
        return;
    }
    
    let ctx = contexts.ctx_mut().unwrap();
    
    // Get effective zone ID for editor actions.
    // If editing a brand-new custom zone via fallback-loaded source zone,
    // prefer the custom zone id so Save targets/logs reflect user intent.
    let loaded_zone_id = current_zone.map(|z| z.id.get());
    let current_zone_id = custom_zone_path
        .as_ref()
        .and_then(|custom| {
            custom
                .path
                .as_ref()
                .map(|_| custom.zone_id)
                .filter(|id| *id > 0)
        })
        .or(loaded_zone_id);
    let next_zone_id_hint = find_next_available_zone_id(&game_data);
    
    // Menu Bar (top)
    editor_menu_bar(
        &*ctx,
        &map_editor_state,
        &save_status,
        current_zone_id,
        next_zone_id_hint,
        &mut save_events,
        &mut menu_state.zone_list_state,
        &mut menu_state.new_zone_events,
        &mut menu_state.add_water_events,
        &mut menu_state.help_state,
        &mut menu_state.selected_model,
        &mut menu_state.save_version_dialog_state,
        &mut menu_state.new_zone_dialog_state,
    );
    
    // Hierarchy Panel (left side) - now with entity query access
    editor_hierarchy_panel(&*ctx, &map_editor_state, &hierarchy_query, &mut commands);
    
    // Properties Panel (right side) - now with entity data access
    editor_properties_panel(
        &*ctx,
        &map_editor_state,
        &entity_data,
        &mut pending_edits,
        &queries.name_query,
        &queries.transform_query,
        &mut property_change_event,
        &mut duplicate_event,
    );
    
    // Status Bar (bottom)
    editor_status_bar(&*ctx, &mut map_editor_state, &save_status, current_zone_id);
}

/// System to render the model browser panel
pub fn model_browser_panel_system(
    mut contexts: EguiContexts,
    map_editor_state: Res<MapEditorState>,
    available_models: Res<AvailableModels>,
    mut selected_model: ResMut<SelectedModel>,
) {
    // Only render when editor is enabled
    if !map_editor_state.enabled {
        return;
    }
    
    let ctx = contexts.ctx_mut().unwrap();
    
    editor_model_browser_panel(
        &*ctx,
        &map_editor_state,
        &available_models,
        &mut selected_model,
    );
}

/// System to handle NewZoneEvent - clears all zone objects and resets editor state
pub fn new_zone_system(
    mut events: MessageReader<NewZoneEvent>,
    mut commands: Commands,
    query: Query<Entity, With<ZoneObject>>,
    mut map_editor_state: ResMut<MapEditorState>,
    mut custom_zone_path: ResMut<crate::map_editor::resources::CustomZonePath>,
    game_data: Res<GameData>,
    mut load_zone_events: MessageWriter<LoadZoneEvent>,
    vfs_resource: Res<VfsResource>,
) {
    for event in events.read() {
        // Check if we should prompt for unsaved changes
        if event.prompt_if_modified && map_editor_state.is_modified {
            // For now, just log a warning. In a full implementation,
            // we would show a dialog asking the user to save.
            log::warn!("[NewZone] Zone has unsaved changes, but proceeding with new zone (dialog not implemented)");
        }
        
        // Despawn all zone objects
        let mut despawned_count = 0;
        for entity in query.iter() {
            commands.entity(entity).despawn();
            despawned_count += 1;
        }

        let requested_zone_id = if event.zone_id > 0 {
            event.zone_id
        } else {
            find_next_available_zone_id(&game_data).unwrap_or(1)
        };

        let target_path = if let Some(path) = &event.output_path {
            path.clone()
        } else {
            vfs_resource
                .base_path
                .join(format!("3DDATA/MAPS/CUSTOM/ZONE_{:03}", requested_zone_id))
        };

        // Bootstrap default files if requested
        if event.initialize_default_block {
            if let Err(err) = bootstrap_default_zone_blocks(&target_path) {
                log::error!("[NewZone] Failed to bootstrap default block files at {:?}: {}", target_path, err);
            } else {
                log::info!("[NewZone] Bootstrapped flat default zone files at {:?}", target_path);
            }
        }
        
        // Clear selection and reset modification state
        map_editor_state.clear_selection();
        map_editor_state.is_modified = false;
        map_editor_state.clear_history();

        // Set custom zone path for saving
        custom_zone_path.path = Some(target_path.clone());
        custom_zone_path.zone_id = requested_zone_id;
        log::info!("[NewZone] Set custom zone path: {:?} for zone id {}", target_path, requested_zone_id);

        if let Some(zone_id) = ZoneId::new(requested_zone_id) {
            if game_data.zone_list.get_zone(zone_id).is_some() {
                // Zone exists in zone list - clear custom path and load the zone
                custom_zone_path.clear();
                load_zone_events.write(LoadZoneEvent::new(zone_id));
            } else {
                // Zone not in zone list - load zone 1 as a fallback so the user can see terrain
                // The custom zone files are saved to target_path for later use
                log::info!(
                    "[NewZone] Zone id {} not in zone list (likely awaiting STB registration). Files bootstrapped at {:?}. Loading zone 1 as fallback for editing.",
                    requested_zone_id,
                    target_path
                );
                if let Some(fallback_zone_id) = ZoneId::new(1) {
                    if game_data.zone_list.get_zone(fallback_zone_id).is_some() {
                        load_zone_events.write(LoadZoneEvent::new(fallback_zone_id));
                    } else {
                        log::error!("[NewZone] Neither zone {} nor fallback zone 1 found in zone list. Cannot load any zone for editing.", requested_zone_id);
                    }
                }
            }
        }
        
        log::info!("[NewZone] Cleared {} zone objects, editor state reset", despawned_count);
    }
}

pub fn add_water_plane_system(
    mut events: MessageReader<AddWaterPlaneEvent>,
    mut commands: Commands,
    current_zone: Option<Res<CurrentZone>>,
    zone_entities: Query<(Entity, &Zone)>,
    selected_terrain: Query<&MapEditorTerrainBlock, With<SelectedInEditor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
) {
    for _ in events.read() {
        let Some(current_zone) = current_zone.as_ref() else {
            log::warn!("[MapEditor] Add Water Plane requested but no CurrentZone is loaded");
            continue;
        };

        let zone_entity = zone_entities
            .iter()
            .find(|(_, zone)| zone.id == current_zone.id)
            .map(|(entity, _)| entity);

        // Get block coordinates and terrain height from selected terrain block.
        // Water is placed 20 metres (2000 cm) above the terrain surface so that
        // there is always 20 m of visible water depth below the water plane.
        let (block_x, block_y, terrain_avg_height_cm) = selected_terrain
            .iter()
            .next()
            .map(|terrain| {
                // Average height from the HIM samples plus any pending offset.
                let avg = if terrain.him_heights_cm.is_empty() {
                    0.0f32
                } else {
                    terrain.him_heights_cm.iter().copied().sum::<f32>()
                        / terrain.him_heights_cm.len() as f32
                };
                (terrain.block_x, terrain.block_y, avg + terrain.height_offset_cm)
            })
            .unwrap_or((0, 0, 0.0));

        // Place water 20 metres above terrain (20 m = 2000 cm).
        let water_height_cm = terrain_avg_height_cm + 2000.0;

        let water_entity = spawn_editor_water_plane(
            &mut commands,
            &mut meshes,
            &mut water_materials,
            block_x,
            block_y,
            water_height_cm,
        );

        if let Some(zone_entity) = zone_entity {
            commands.entity(zone_entity).add_child(water_entity);
        }

        log::info!(
            "[MapEditor] Added water plane for block ({}, {}) at height {:.1} cm ({:.1} m, terrain avg {:.1} cm) as entity {:?}",
            block_x,
            block_y,
            water_height_cm,
            water_height_cm / 100.0,
            terrain_avg_height_cm,
            water_entity
        );
    }
}

fn spawn_editor_water_plane(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    water_materials: &mut Assets<WaterMaterial>,
    block_x: u32,
    block_y: u32,
    height_cm: f32,
) -> Entity {
    let block_size_m = 160.0f32;
    let local_start_x = block_x as f32 * block_size_m - 5200.0;
    let local_start_z = block_y as f32 * block_size_m - 5200.0;
    let local_end_x = local_start_x + block_size_m;
    let local_end_z = local_start_z + block_size_m;

    let start_ifo_cm = Vec3::new(local_start_x * 100.0, height_cm, -local_start_z * 100.0);
    let end_ifo_cm = Vec3::new(local_end_x * 100.0, height_cm, -local_end_z * 100.0);
    let water_size = 16000.0;

    let start = Vec3::new(
        start_ifo_cm.x / 100.0,
        start_ifo_cm.y / 100.0,
        -start_ifo_cm.z / 100.0,
    );
    let end = Vec3::new(
        end_ifo_cm.x / 100.0,
        end_ifo_cm.y / 100.0,
        -end_ifo_cm.z / 100.0,
    );

    let uv_x = (end.x - start.x) / (water_size / 100.0);
    let uv_y = (end.z - start.z) / (water_size / 100.0);

    let vertices = [
        ([start.x, start.y, end.z], [0.0, 1.0, 0.0], [uv_x, uv_y]),
        ([start.x, start.y, start.z], [0.0, 1.0, 0.0], [uv_x, 0.0]),
        ([end.x, start.y, start.z], [0.0, 1.0, 0.0], [0.0, 0.0]),
        ([end.x, start.y, end.z], [0.0, 1.0, 0.0], [0.0, uv_y]),
    ];

    let mut collider_verts = Vec::new();
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    for (position, normal, uv) in &vertices {
        collider_verts.push((*position).into());
        positions.push(*position);
        normals.push(*normal);
        uvs.push(*uv);
    }

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 2, 1, 0, 3, 2]));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    commands
        .spawn((
            crate::map_editor::components::EditorSelectable,
            ZoneObject::Water,
            MapEditorWaterPlane::new(block_x, block_y, start_ifo_cm, end_ifo_cm, water_size),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(water_materials.add(WaterMaterial::default())),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
            bevy::camera::visibility::InheritedVisibility::default(),
            bevy::camera::visibility::ViewVisibility::default(),
            bevy::camera::primitives::Aabb::from_min_max(
                Vec3::splat(-100000.0),
                Vec3::splat(100000.0),
            ),
            bevy::camera::visibility::RenderLayers::layer(0),
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
        ))
        .insert((
            RigidBody::Fixed,
            Collider::trimesh(collider_verts, vec![[0, 2, 1], [0, 3, 2]])
                .expect("Failed to create editor water collider"),
            CollisionGroups::new(COLLISION_GROUP_ZONE_WATER, COLLISION_FILTER_INSPECTABLE),
        ))
        .id()
}

fn find_next_available_zone_id(game_data: &GameData) -> Option<u16> {
    let mut max_zone_id = 0u16;
    for zone in game_data.zone_list.iter() {
        max_zone_id = max_zone_id.max(zone.id.get());
    }

    max_zone_id.checked_add(1)
}

fn bootstrap_default_zone_blocks(zone_path: &PathBuf) -> Result<(), anyhow::Error> {
    if !zone_path.exists() {
        std::fs::create_dir_all(zone_path)?;
    }

    // Bootstrap a complete flat zone scaffold so zone 1 fallback can still render
    // a full editable terrain surface for the new custom zone path.
    for block_y in 0..64u32 {
        for block_x in 0..64u32 {
            write_default_him(&zone_path.join(format!("{}_{}.HIM", block_x, block_y)), 65, 65)?;
            write_default_til(&zone_path.join(format!("{}_{}.TIL", block_x, block_y)), 16, 16)?;

            let mut ifo_block = IfoBlock::new(block_x, block_y);
            ifo_block.original_block_order = vec![];
            export_ifo_block(&ifo_block, &zone_path.join(format!("{}_{}.IFO", block_x, block_y)))?;
        }
    }

    Ok(())
}

fn write_default_him(path: &PathBuf, width: u32, height: u32) -> Result<(), anyhow::Error> {
    let mut data = Vec::new();
    data.extend_from_slice(&width.to_le_bytes());
    data.extend_from_slice(&height.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());

    let count = (width * height) as usize;
    for _ in 0..count {
        data.extend_from_slice(&0.0f32.to_le_bytes());
    }

    let mut file = std::fs::File::create(path)?;
    file.write_all(&data)?;
    Ok(())
}

fn write_default_til(path: &PathBuf, width: u32, height: u32) -> Result<(), anyhow::Error> {
    let mut data = Vec::new();
    data.extend_from_slice(&width.to_le_bytes());
    data.extend_from_slice(&height.to_le_bytes());

    let count = (width * height) as usize;
    // Use tile index 0 for all tiles - this references the first tile definition in the ZON file
    // which typically has valid texture references (layer1=0, layer2=0, offset1=0, offset2=0)
    // The 3 bytes before the tile index are reserved/unused in the TIL format
    for _ in 0..count {
        data.extend_from_slice(&[0u8; 3]);
        data.extend_from_slice(&0u32.to_le_bytes()); // Use tile index 0 (first tile, 0-indexed)
    }

    let mut file = std::fs::File::create(path)?;
    file.write_all(&data)?;
    Ok(())
}
