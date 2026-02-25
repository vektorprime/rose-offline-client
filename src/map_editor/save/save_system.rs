//! Save System for Map Editor
//!
//! This module provides Bevy systems and events for saving zone data.

use std::collections::HashSet;
use std::path::PathBuf;

use bevy::prelude::*;

use crate::components::{
    EventObject, WarpObject, ZoneObject,
};
use crate::map_editor::resources::{DeletedZoneObjects, ZoneObjectType};
use crate::map_editor::systems::model_placement_system::EditorPlacedObject;
use crate::resources::CurrentZone;
use crate::zone_loader::ZoneLoaderAsset;

use super::ifo_export::{export_ifo_block, ExportStats};
use super::ifo_types::*;

/// Event to trigger saving a zone
#[derive(Event, Debug, Clone)]
pub struct SaveZoneEvent {
    /// Zone ID to save
    pub zone_id: u16,
    /// Optional custom path (None = save to original path)
    pub path: Option<PathBuf>,
}

impl SaveZoneEvent {
    /// Create a new SaveZoneEvent to save to the original path
    pub fn new(zone_id: u16) -> Self {
        Self {
            zone_id,
            path: None,
        }
    }

    /// Create a SaveZoneEvent with a custom path (Save As)
    pub fn with_path(zone_id: u16, path: PathBuf) -> Self {
        Self {
            zone_id,
            path: Some(path),
        }
    }
}

/// Resource to track save status
#[derive(Resource, Default, Debug, Clone)]
pub struct SaveStatus {
    /// Whether a save operation is in progress
    pub is_saving: bool,
    /// Last save result (if any)
    pub last_result: Option<SaveResult>,
    /// Status message to display
    pub status_message: String,
}

impl SaveStatus {
    /// Create a new SaveStatus
    pub fn new() -> Self {
        Self {
            is_saving: false,
            last_result: None,
            status_message: String::new(),
        }
    }

    /// Set saving in progress
    pub fn set_saving(&mut self, message: &str) {
        self.is_saving = true;
        self.status_message = message.to_string();
    }

    /// Set save complete
    pub fn set_complete(&mut self, result: SaveResult) {
        self.is_saving = false;
        self.last_result = Some(result.clone());
        self.status_message = result.message();
    }

    /// Clear the status
    pub fn clear(&mut self) {
        self.is_saving = false;
        self.status_message.clear();
    }
}

/// Result of a save operation
#[derive(Debug, Clone)]
pub struct SaveResult {
    /// Whether the save was successful
    pub success: bool,
    /// Number of blocks saved
    pub blocks_saved: usize,
    /// Number of objects saved
    pub objects_saved: usize,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl SaveResult {
    /// Create a successful save result
    pub fn success(blocks_saved: usize, objects_saved: usize) -> Self {
        Self {
            success: true,
            blocks_saved,
            objects_saved,
            error: None,
        }
    }

    /// Create a failed save result
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            blocks_saved: 0,
            objects_saved: 0,
            error: Some(error),
        }
    }

    /// Get a human-readable message
    pub fn message(&self) -> String {
        if self.success {
            format!(
                "Saved successfully ({} blocks, {} objects)",
                self.blocks_saved, self.objects_saved
            )
        } else {
            format!("Save failed: {}", self.error.as_deref().unwrap_or("Unknown error"))
        }
    }
}

/// Plugin for the save system
pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveStatus>()
            .add_event::<SaveZoneEvent>()
            .add_systems(Update, save_zone_system);
        
        log::info!("[SavePlugin] Save system initialized");
    }
}

/// System to handle save zone events
pub fn save_zone_system(
    mut events: EventReader<SaveZoneEvent>,
    mut save_status: ResMut<SaveStatus>,
    mut map_editor_state: ResMut<crate::map_editor::resources::MapEditorState>,
    mut deleted_zone_objects: ResMut<DeletedZoneObjects>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    vfs_resource: Res<crate::resources::VfsResource>,
    zone_objects_query: Query<(
        Entity,
        &Transform,
        &ZoneObject,
        Option<&EventObject>,
        Option<&WarpObject>,
        Option<&EditorPlacedObject>,
    )>,
) {
    // Process all save events
    for event in events.read() {
        log::info!("[SaveSystem] ====== SAVE ZONE SYSTEM TRIGGERED ======");
        log::info!("[SaveSystem] Processing SaveZoneEvent for zone {}", event.zone_id);
        
        save_status.set_saving("Saving zone...");

        // Get the zone data
        let zone_data = if let Some(ref current_zone) = current_zone {
            log::info!("[SaveSystem] CurrentZone resource exists, zone_id: {}", current_zone.id.get());
            zone_loader_assets.get(&current_zone.handle)
        } else {
            log::error!("[SaveSystem] CurrentZone resource does NOT exist!");
            None
        };

        let Some(zone_data) = zone_data else {
            let error = "No zone currently loaded or zone data not available";
            log::error!("[SaveSystem] {}", error);
            save_status.set_complete(SaveResult::failure(error.to_string()));
            continue;
        };

        // Determine output path:
        // - The zone_path is a VFS path like "3DDATA/MAPS/JUNON/JDT01"
        // - We need to join it with the base_path to get the real filesystem path
        // - If a custom path is provided, use that instead
        let output_path = if let Some(ref custom_path) = event.path {
            custom_path.clone()
        } else {
            // Join base_path with zone_path to get the real filesystem path
            vfs_resource.base_path.join(&zone_data.zone_path)
        };

        log::info!("[SaveSystem] VFS base_path: {:?}", vfs_resource.base_path);
        log::info!("[SaveSystem] Zone path from zone_data: {:?}", zone_data.zone_path);
        log::info!("[SaveSystem] Output path for save: {:?}", output_path);
        log::info!("[SaveSystem] Zone ID: {}", zone_data.zone_id.get());

        // Count zone objects for logging
        let zone_object_count = zone_objects_query.iter().count();
        log::info!("[SaveSystem] Found {} zone objects in query", zone_object_count);

        // STEP 1: Pre-populate with existing IFO data to preserve objects that weren't modified
        // This is crucial - without this, any objects not spawned as entities would be lost
        let mut export_data = ZoneExportData::from_existing_blocks(
            event.zone_id,
            &zone_data.blocks,
            output_path.to_string_lossy().to_string(),
        );
        let existing_object_count = export_data.total_objects();
        log::info!("[SaveSystem] Pre-populated export_data with {} existing objects from IFO files", existing_object_count);

        // STEP 1.5: Process tracked deletions - remove deleted objects from export_data
        // This must happen BEFORE processing spawned objects so deleted objects don't get re-added
        let mut deleted_count = 0usize;
        let mut deletion_modified_blocks: HashSet<(u32, u32)> = HashSet::new();
        
        log::info!("[SaveSystem] ====== PROCESSING DELETIONS ======");
        log::info!("[SaveSystem] Tracking {} deleted objects", deleted_zone_objects.len());
        
        for (block_x, block_y, ifo_object_id, object_type) in deleted_zone_objects.objects.iter() {
            // Get the block data
            let index = (block_x + block_y * 64) as usize;
            if let Some(block_ref) = export_data.blocks[index].as_mut() {
                // Remove the object from the appropriate list based on type
                let removed = match object_type {
                    ZoneObjectType::Deco => {
                        if *ifo_object_id < block_ref.block.deco_objects.len() {
                            // Mark for removal by setting to an invalid object (we'll filter later)
                            // For now, we use swap_remove to maintain valid indices
                            // Note: This changes indices, but since we're saving all at once, it's okay
                            block_ref.block.deco_objects.swap_remove(*ifo_object_id);
                            true
                        } else {
                            log::warn!("[SaveSystem] Deco deletion index {} out of bounds (len={})", 
                                ifo_object_id, block_ref.block.deco_objects.len());
                            false
                        }
                    }
                    ZoneObjectType::Cnst => {
                        if *ifo_object_id < block_ref.block.cnst_objects.len() {
                            block_ref.block.cnst_objects.swap_remove(*ifo_object_id);
                            true
                        } else {
                            log::warn!("[SaveSystem] Cnst deletion index {} out of bounds (len={})", 
                                ifo_object_id, block_ref.block.cnst_objects.len());
                            false
                        }
                    }
                    ZoneObjectType::Event => {
                        if *ifo_object_id < block_ref.block.event_objects.len() {
                            block_ref.block.event_objects.swap_remove(*ifo_object_id);
                            true
                        } else {
                            log::warn!("[SaveSystem] Event deletion index {} out of bounds (len={})", 
                                ifo_object_id, block_ref.block.event_objects.len());
                            false
                        }
                    }
                    ZoneObjectType::Warp => {
                        if *ifo_object_id < block_ref.block.warp_objects.len() {
                            block_ref.block.warp_objects.swap_remove(*ifo_object_id);
                            true
                        } else {
                            log::warn!("[SaveSystem] Warp deletion index {} out of bounds (len={})", 
                                ifo_object_id, block_ref.block.warp_objects.len());
                            false
                        }
                    }
                    ZoneObjectType::Sound => {
                        if *ifo_object_id < block_ref.block.sound_objects.len() {
                            block_ref.block.sound_objects.swap_remove(*ifo_object_id);
                            true
                        } else {
                            log::warn!("[SaveSystem] Sound deletion index {} out of bounds (len={})", 
                                ifo_object_id, block_ref.block.sound_objects.len());
                            false
                        }
                    }
                    ZoneObjectType::Effect => {
                        if *ifo_object_id < block_ref.block.effect_objects.len() {
                            block_ref.block.effect_objects.swap_remove(*ifo_object_id);
                            true
                        } else {
                            log::warn!("[SaveSystem] Effect deletion index {} out of bounds (len={})", 
                                ifo_object_id, block_ref.block.effect_objects.len());
                            false
                        }
                    }
                    ZoneObjectType::Animated => {
                        if *ifo_object_id < block_ref.block.animated_objects.len() {
                            block_ref.block.animated_objects.swap_remove(*ifo_object_id);
                            true
                        } else {
                            log::warn!("[SaveSystem] Animated deletion index {} out of bounds (len={})", 
                                ifo_object_id, block_ref.block.animated_objects.len());
                            false
                        }
                    }
                };
                
                if removed {
                    deleted_count += 1;
                    deletion_modified_blocks.insert((*block_x, *block_y));
                    block_ref.modified = true;
                    log::debug!("[SaveSystem] Removed {:?} with ifo_object_id={} from block ({}, {})", 
                        object_type, ifo_object_id, block_x, block_y);
                }
            } else {
                log::warn!("[SaveSystem] Block ({}, {}) not found in export_data for deletion", block_x, block_y);
            }
        }
        
        log::info!("[SaveSystem] ====== FINISHED PROCESSING DELETIONS ======");
        log::info!("[SaveSystem] Removed {} objects from export_data", deleted_count);
        log::info!("[SaveSystem] Blocks modified by deletions: {:?}", deletion_modified_blocks);
        
        // Clear the tracked deletions after processing
        deleted_zone_objects.clear();

        // STEP 2: Track which blocks have been modified by the editor
        let mut modified_blocks: HashSet<(u32, u32)> = HashSet::new();

        // STEP 3: Process all spawned zone objects - update existing or add new
        log::info!("[SaveSystem] ====== PROCESSING SPAWNED ZONE OBJECTS ======");
        let mut updated_objects_count = 0usize;
        let mut added_objects_count = 0usize;
        
        for (_entity, transform, zone_object, event_object, warp_object, editor_placed) in zone_objects_query.iter() {
            // Determine block coordinates from position
            // Zone is 64x64 blocks, each block is 160 units
            let (translation, rotation, scale) = (
                transform.translation,
                transform.rotation,
                transform.scale,
            );
            
            // Calculate block coordinates from WORLD coordinates
            // Zone center is at world position (5200, 0, -5200)
            // Objects are in WORLD coordinates (not parented to zone entity)
            let block_x = (translation.x / 160.0).floor() as u32;
            let block_y = ((translation.z + 10400.0) / 160.0).floor() as u32;
            
            // Clamp to valid range
            let block_x = block_x.clamp(0, 63);
            let block_y = block_y.clamp(0, 63);

            // Convert WORLD coordinates to LOCAL coordinates for IFO file
            // Zone center is at world position (5200, 0, -5200)
            // local = world - zone_center
            let zone_center = Vec3::new(5200.0, 0.0, -5200.0);
            let local_translation = translation - zone_center;

            // Create IfoObject from local coordinates
            let ifo_object = IfoObject::from_transform(
                0, // Will be set based on zone object type
                local_translation,
                rotation,
                scale,
            );

            // Get the ifo_object_id to check if this is an existing object
            let (ifo_object_id, zsc_object_id) = match zone_object {
                ZoneObject::DecoObject(id) => (Some(id.ifo_object_id), id.zsc_object_id),
                ZoneObject::DecoObjectPart(part) => (Some(part.ifo_object_id), part.zsc_object_id),
                ZoneObject::CnstObject(id) => (Some(id.ifo_object_id), id.zsc_object_id),
                ZoneObject::CnstObjectPart(part) => (Some(part.ifo_object_id), part.zsc_object_id),
                ZoneObject::EventObject(id) => (Some(id.ifo_object_id), id.zsc_object_id),
                ZoneObject::EventObjectPart(part) => (Some(part.ifo_object_id), part.zsc_object_id),
                ZoneObject::WarpObject(id) => (Some(id.ifo_object_id), id.zsc_object_id),
                ZoneObject::WarpObjectPart(part) => (Some(part.ifo_object_id), part.zsc_object_id),
                ZoneObject::SoundObject { ifo_object_id, .. } => (Some(*ifo_object_id), 0),
                ZoneObject::EffectObject { ifo_object_id, .. } => (Some(*ifo_object_id), 0),
                ZoneObject::AnimatedObject(_) => (None, 0), // Animated objects don't have ifo_object_id
                ZoneObject::Water => (None, 0),
                ZoneObject::Terrain(_) => (None, 0),
            };

            // Try to find and update existing object, or add as new
            // IMPORTANT: Objects with EditorPlacedObject component are ALWAYS new objects
            // They have ifo_object_id=0 which would incorrectly match existing objects at index 0
            let mut is_new_object = editor_placed.is_some();
            
            // Only try to update existing objects if this is NOT an editor-placed object
            if !is_new_object {
                if let Some(ifo_id) = ifo_object_id {
                    // Try to update existing object by index
                    let block = export_data.get_block(block_x, block_y);
                    if let Some(block_data) = block {
                        let existing_count = block_data.block.total_objects();
                        
                        // Check if this ifo_id could be valid for this block
                        if ifo_id < existing_count {
                            // Try to find and update in the appropriate list
                            // For simplicity, we'll mark the block as modified and add as new
                            // A more sophisticated approach would find and update the exact object
                            log::debug!("[SaveSystem] Object with ifo_object_id={} may exist in block ({}, {})",
                                ifo_id, block_x, block_y);
                            
                            // For now, we'll check if we can find a matching object and update it
                            // This is a simplified approach - we check by object_id match
                            let found = update_existing_object(
                                &mut export_data,
                                block_x,
                                block_y,
                                ifo_id,
                                zsc_object_id,
                                zone_object,
                                &ifo_object,
                                event_object,
                                warp_object,
                            );
                            
                            if found {
                                is_new_object = false;
                                updated_objects_count += 1;
                                log::debug!("[SaveSystem] Updated existing object with ifo_object_id={}", ifo_id);
                            }
                        }
                    }
                }
            }
            
            if is_new_object {
                // Add as new object
                added_objects_count += 1;
                modified_blocks.insert((block_x, block_y));
                
                // Get or create the block (marked as modified)
                let block = export_data.get_or_create_modified_block(block_x, block_y);

                // Add to appropriate object list based on type
                match zone_object {
                    ZoneObject::DecoObject(id) => {
                        let mut obj = ifo_object.clone();
                        obj.object_id = id.zsc_object_id as u32;
                        block.block.deco_objects.push(obj);
                    }
                    ZoneObject::DecoObjectPart(part) => {
                        let mut obj = ifo_object.clone();
                        obj.object_id = part.zsc_object_id as u32;
                        block.block.deco_objects.push(obj);
                    }
                    ZoneObject::CnstObject(id) => {
                        let mut obj = ifo_object.clone();
                        obj.object_id = id.zsc_object_id as u32;
                        block.block.cnst_objects.push(obj);
                    }
                    ZoneObject::CnstObjectPart(part) => {
                        let mut obj = ifo_object.clone();
                        obj.object_id = part.zsc_object_id as u32;
                        block.block.cnst_objects.push(obj);
                    }
                    ZoneObject::EventObject(id) => {
                        if let Some(event_obj) = event_object {
                            let mut ifo_event = IfoEventObject::new(id.zsc_object_id as u32);
                            ifo_event.object = ifo_object.clone();
                            ifo_event.quest_trigger_name = event_obj.quest_trigger_name.clone();
                            ifo_event.script_function_name = event_obj.script_function_name.clone();
                            block.block.event_objects.push(ifo_event);
                        }
                    }
                    ZoneObject::EventObjectPart(part) => {
                        if let Some(event_obj) = event_object {
                            let mut ifo_event = IfoEventObject::new(part.zsc_object_id as u32);
                            ifo_event.object = ifo_object.clone();
                            ifo_event.quest_trigger_name = event_obj.quest_trigger_name.clone();
                            ifo_event.script_function_name = event_obj.script_function_name.clone();
                            block.block.event_objects.push(ifo_event);
                        }
                    }
                    ZoneObject::WarpObject(id) => {
                        if let Some(warp_obj) = warp_object {
                            let mut ifo_warp = IfoWarpObject::new(id.zsc_object_id as u32, warp_obj.warp_id.get());
                            ifo_warp.object = ifo_object.clone();
                            block.block.warp_objects.push(ifo_warp);
                        }
                    }
                    ZoneObject::WarpObjectPart(part) => {
                        if let Some(warp_obj) = warp_object {
                            let mut ifo_warp = IfoWarpObject::new(part.zsc_object_id as u32, warp_obj.warp_id.get());
                            ifo_warp.object = ifo_object.clone();
                            block.block.warp_objects.push(ifo_warp);
                        }
                    }
                    ZoneObject::SoundObject { sound_path, .. } => {
                        let mut ifo_sound = IfoSoundObject::new(0);
                        ifo_sound.object = ifo_object.clone();
                        ifo_sound.sound_path = sound_path.clone();
                        block.block.sound_objects.push(ifo_sound);
                    }
                    ZoneObject::EffectObject { effect_path, .. } => {
                        let mut ifo_effect = IfoEffectObject::new(0);
                        ifo_effect.object = ifo_object.clone();
                        ifo_effect.effect_path = effect_path.clone();
                        block.block.effect_objects.push(ifo_effect);
                    }
                    ZoneObject::AnimatedObject(_) => {
                        block.block.animated_objects.push(ifo_object.clone());
                    }
                    ZoneObject::Water => {
                        // Water is handled separately via water planes
                    }
                    ZoneObject::Terrain(_) => {
                        // Terrain is not saved in IFO files
                    }
                }
            }
        }
        
        log::info!("[SaveSystem] ====== FINISHED PROCESSING SPAWNED ZONE OBJECTS ======");
        log::info!("[SaveSystem] Updated {} existing objects, added {} new objects", updated_objects_count, added_objects_count);
        log::info!("[SaveSystem] Total objects in export_data: {} (was {} before processing)",
            export_data.total_objects(), existing_object_count);
        log::info!("[SaveSystem] Modified blocks: {:?}", modified_blocks);

        // Create backup of original files before overwriting
        if let Err(e) = create_backup(&output_path) {
            log::warn!("[SaveSystem] Failed to create backup: {}", e);
            // Continue anyway - backup failure shouldn't prevent save
        }

        // Export only modified IFO files
        let mut stats = ExportStats::default();
        let mut errors = Vec::new();
        let mut skipped_blocks = 0usize;

        for block_data in export_data.blocks.iter().filter_map(|b| b.as_ref()) {
            // Skip empty blocks
            if block_data.block.total_objects() == 0 {
                continue;
            }

            // Skip unmodified blocks - only write files that have been changed
            if !block_data.modified {
                skipped_blocks += 1;
                log::debug!("[SaveSystem] Skipping unmodified block {}_{} ({} objects)",
                    block_data.block_x, block_data.block_y, block_data.block.total_objects());
                continue;
            }

            let file_name = block_data.file_name();
            let file_path = output_path.join(&file_name);

            log::info!("[SaveSystem] Writing modified block: {:?}", file_path);

            match export_ifo_block(&block_data.block, &file_path) {
                Ok(size) => {
                    stats.blocks_exported += 1;
                    stats.bytes_written += size;
                    stats.total_objects += block_data.block.total_objects();
                    log::info!("[SaveSystem] Exported {} ({} bytes, {} objects)",
                        file_name, size, block_data.block.total_objects());
                }
                Err(e) => {
                    stats.blocks_failed += 1;
                    errors.push(format!("{}: {}", file_name, e));
                    log::error!("[SaveSystem] Failed to export {}: {}", file_name, e);
                }
            }
        }

        if skipped_blocks > 0 {
            log::info!("[SaveSystem] Skipped {} unmodified blocks", skipped_blocks);
        }

        // Update save status
        if stats.blocks_failed == 0 && stats.blocks_exported > 0 {
            let result = SaveResult::success(stats.blocks_exported, stats.total_objects);
            log::info!("[SaveSystem] {}", result.message());
            save_status.set_complete(result);
            
            // Mark zone as unmodified
            map_editor_state.is_modified = false;
        } else if stats.blocks_exported == 0 {
            let result = SaveResult::failure("No blocks were exported (no objects found or all blocks empty)".to_string());
            log::error!("[SaveSystem] {}", result.message());
            save_status.set_complete(result);
        } else {
            let result = SaveResult::failure(format!(
                "Partial save: {} blocks failed ({})",
                stats.blocks_failed,
                errors.join(", ")
            ));
            log::warn!("[SaveSystem] {}", result.message());
            save_status.set_complete(result);
        }
    }
}

/// Try to find and update an existing object in the export data
/// Returns true if the object was found and updated, false otherwise
///
/// The ifo_object_id is the index within the specific object type list:
/// - For DecoObject: index in deco_objects
/// - For CnstObject: index in cnst_objects
/// - For EventObject: index in event_objects
/// - For WarpObject: index in warp_objects
/// - For SoundObject: index in sound_objects
/// - For EffectObject: index in effect_objects
fn update_existing_object(
    export_data: &mut ZoneExportData,
    block_x: u32,
    block_y: u32,
    ifo_object_id: usize,
    _zsc_object_id: usize,
    zone_object: &ZoneObject,
    new_ifo_object: &IfoObject,
    event_object: Option<&EventObject>,
    warp_object: Option<&WarpObject>,
) -> bool {
    // Get mutable access to the block
    let index = (block_x + block_y * 64) as usize;
    let Some(block_ref) = export_data.blocks[index].as_mut() else {
        return false;
    };
    
    // Mark block as modified since we're updating an object
    block_ref.modified = true;
    
    // Use ifo_object_id as direct index into the appropriate list
    // This is the unique identifier for objects within their type-specific list
    match zone_object {
        ZoneObject::DecoObject(_) | ZoneObject::DecoObjectPart(_) => {
            // Use ifo_object_id as direct index into deco_objects
            if ifo_object_id < block_ref.block.deco_objects.len() {
                let obj = &mut block_ref.block.deco_objects[ifo_object_id];
                obj.position = new_ifo_object.position;
                obj.rotation = new_ifo_object.rotation;
                obj.scale = new_ifo_object.scale;
                log::debug!("[SaveSystem] Updated deco_object[{}] in block ({}, {})",
                    ifo_object_id, block_x, block_y);
                return true;
            }
        }
        ZoneObject::CnstObject(_) | ZoneObject::CnstObjectPart(_) => {
            // Use ifo_object_id as direct index into cnst_objects
            if ifo_object_id < block_ref.block.cnst_objects.len() {
                let obj = &mut block_ref.block.cnst_objects[ifo_object_id];
                obj.position = new_ifo_object.position;
                obj.rotation = new_ifo_object.rotation;
                obj.scale = new_ifo_object.scale;
                log::debug!("[SaveSystem] Updated cnst_object[{}] in block ({}, {})",
                    ifo_object_id, block_x, block_y);
                return true;
            }
        }
        ZoneObject::EventObject(_) | ZoneObject::EventObjectPart(_) => {
            // Use ifo_object_id as direct index into event_objects
            if ifo_object_id < block_ref.block.event_objects.len() {
                let evt_obj = &mut block_ref.block.event_objects[ifo_object_id];
                evt_obj.object.position = new_ifo_object.position;
                evt_obj.object.rotation = new_ifo_object.rotation;
                evt_obj.object.scale = new_ifo_object.scale;
                // Update event properties if available
                if let Some(event) = event_object {
                    evt_obj.quest_trigger_name = event.quest_trigger_name.clone();
                    evt_obj.script_function_name = event.script_function_name.clone();
                }
                log::debug!("[SaveSystem] Updated event_object[{}] in block ({}, {})",
                    ifo_object_id, block_x, block_y);
                return true;
            }
        }
        ZoneObject::WarpObject(_) | ZoneObject::WarpObjectPart(_) => {
            // Use ifo_object_id as direct index into warp_objects
            if ifo_object_id < block_ref.block.warp_objects.len() {
                let warp_obj = &mut block_ref.block.warp_objects[ifo_object_id];
                warp_obj.object.position = new_ifo_object.position;
                warp_obj.object.rotation = new_ifo_object.rotation;
                warp_obj.object.scale = new_ifo_object.scale;
                // Update warp_id if available
                if let Some(warp) = warp_object {
                    warp_obj.object.warp_id = warp.warp_id.get();
                }
                log::debug!("[SaveSystem] Updated warp_object[{}] in block ({}, {})",
                    ifo_object_id, block_x, block_y);
                return true;
            }
        }
        ZoneObject::SoundObject { .. } => {
            // Use ifo_object_id as direct index into sound_objects
            if ifo_object_id < block_ref.block.sound_objects.len() {
                let sound_obj = &mut block_ref.block.sound_objects[ifo_object_id];
                sound_obj.object.position = new_ifo_object.position;
                sound_obj.object.rotation = new_ifo_object.rotation;
                sound_obj.object.scale = new_ifo_object.scale;
                log::debug!("[SaveSystem] Updated sound_object[{}] in block ({}, {})",
                    ifo_object_id, block_x, block_y);
                return true;
            }
        }
        ZoneObject::EffectObject { .. } => {
            // Use ifo_object_id as direct index into effect_objects
            if ifo_object_id < block_ref.block.effect_objects.len() {
                let effect_obj = &mut block_ref.block.effect_objects[ifo_object_id];
                effect_obj.object.position = new_ifo_object.position;
                effect_obj.object.rotation = new_ifo_object.rotation;
                effect_obj.object.scale = new_ifo_object.scale;
                log::debug!("[SaveSystem] Updated effect_object[{}] in block ({}, {})",
                    ifo_object_id, block_x, block_y);
                return true;
            }
        }
        ZoneObject::AnimatedObject(_) => {
            // Animated objects use ifo_object_id as direct index
            if ifo_object_id < block_ref.block.animated_objects.len() {
                let obj = &mut block_ref.block.animated_objects[ifo_object_id];
                obj.position = new_ifo_object.position;
                obj.rotation = new_ifo_object.rotation;
                obj.scale = new_ifo_object.scale;
                log::debug!("[SaveSystem] Updated animated_object[{}] in block ({}, {})",
                    ifo_object_id, block_x, block_y);
                return true;
            }
        }
        ZoneObject::Water | ZoneObject::Terrain(_) => {
            // Water and terrain are not updated this way
            return false;
        }
    }
    
    // Object index out of bounds for this type list
    log::warn!("[SaveSystem] ifo_object_id {} out of bounds for {:?} in block ({}, {})",
        ifo_object_id, zone_object, block_x, block_y);
    false
}

/// Create a backup of the original IFO files
fn create_backup(zone_path: &PathBuf) -> std::io::Result<()> {
    // Check if the zone path exists on the real filesystem
    if !zone_path.exists() {
        log::warn!("[SaveSystem] Zone path does not exist on filesystem: {:?}", zone_path);
        return Ok(()); // Skip backup if path doesn't exist
    }

    let backup_dir = zone_path.join("backup");
    
    // Create backup directory if it doesn't exist
    if !backup_dir.exists() {
        std::fs::create_dir_all(&backup_dir)?;
    }

    // Get current timestamp for backup folder name
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let timestamped_backup_dir = backup_dir.join(timestamp.to_string());
    std::fs::create_dir_all(&timestamped_backup_dir)?;

    // Copy all IFO files to backup
    let mut copied_count = 0;
    for entry in std::fs::read_dir(zone_path)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("ifo")) {
            let file_name = path.file_name().unwrap();
            let backup_path = timestamped_backup_dir.join(file_name);
            std::fs::copy(&path, &backup_path)?;
            copied_count += 1;
        }
    }

    if copied_count > 0 {
        log::info!("[SaveSystem] Created backup of {} IFO files in {:?}", copied_count, timestamped_backup_dir);
    }

    Ok(())
}
