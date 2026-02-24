//! Save System for Map Editor
//! 
//! This module provides Bevy systems and events for saving zone data.

use std::path::PathBuf;

use bevy::prelude::*;

use crate::components::{
    EventObject, WarpObject, ZoneObject,
};
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
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    zone_objects_query: Query<(
        Entity,
        &GlobalTransform,
        &ZoneObject,
        Option<&EventObject>,
        Option<&WarpObject>,
    )>,
) {
    for event in events.read() {
        log::info!("[SaveSystem] Processing SaveZoneEvent for zone {}", event.zone_id);
        
        save_status.set_saving("Saving zone...");

        // Get the zone data
        let zone_data = if let Some(current_zone) = &current_zone {
            zone_loader_assets.get(&current_zone.handle)
        } else {
            None
        };

        let Some(zone_data) = zone_data else {
            let error = "No zone currently loaded or zone data not available";
            log::error!("[SaveSystem] {}", error);
            save_status.set_complete(SaveResult::failure(error.to_string()));
            continue;
        };

        // Determine output path
        let output_path = event.path.clone().unwrap_or_else(|| {
            zone_data.zone_path.clone()
        });

        log::info!("[SaveSystem] Output path: {:?}", output_path);

        // Collect zone objects and group by block
        let mut export_data = ZoneExportData::new(event.zone_id, output_path.to_string_lossy().to_string());

        // Process all zone objects
        for (_entity, global_transform, zone_object, event_object, warp_object) in zone_objects_query.iter() {
            let transform = global_transform.compute_transform();
            
            // Determine block coordinates from position
            // Zone is 64x64 blocks, each block is 160 units
            let (translation, rotation, scale) = (
                transform.translation,
                transform.rotation,
                transform.scale,
            );
            
            // Calculate block coordinates
            // Zone center is at (5200, 0, -5200), blocks are 160 units each
            let block_x = ((translation.x + 5200.0) / 160.0).floor() as u32;
            let block_y = ((-translation.z + 5200.0) / 160.0).floor() as u32;
            
            // Clamp to valid range
            let block_x = block_x.clamp(0, 63);
            let block_y = block_y.clamp(0, 63);

            // Get or create the block
            let block = export_data.get_or_create_block(block_x, block_y);

            // Create IfoObject from transform
            let ifo_object = IfoObject::from_transform(
                0, // Will be set based on zone object type
                translation,
                rotation,
                scale,
            );

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

        // Create backup of original files before overwriting
        if let Err(e) = create_backup(&output_path) {
            log::warn!("[SaveSystem] Failed to create backup: {}", e);
            // Continue anyway - backup failure shouldn't prevent save
        }

        // Export all IFO files
        let mut stats = ExportStats::default();
        let mut errors = Vec::new();

        for block_data in export_data.blocks.iter().filter_map(|b| b.as_ref()) {
            if block_data.block.total_objects() == 0 {
                continue; // Skip empty blocks
            }

            let file_name = block_data.file_name();
            let file_path = output_path.join(&file_name);

            match export_ifo_block(&block_data.block, &file_path) {
                Ok(size) => {
                    stats.blocks_exported += 1;
                    stats.bytes_written += size;
                    stats.total_objects += block_data.block.total_objects();
                    log::info!("[SaveSystem] Exported {} ({} bytes)", file_name, size);
                }
                Err(e) => {
                    stats.blocks_failed += 1;
                    errors.push(format!("{}: {}", file_name, e));
                    log::error!("[SaveSystem] Failed to export {}: {}", file_name, e);
                }
            }
        }

        // Update save status
        if stats.blocks_failed == 0 && stats.blocks_exported > 0 {
            let result = SaveResult::success(stats.blocks_exported, stats.total_objects);
            log::info!("[SaveSystem] {}", result.message());
            save_status.set_complete(result);
            
            // Mark zone as unmodified
            map_editor_state.is_modified = false;
        } else if stats.blocks_exported == 0 {
            let result = SaveResult::failure("No blocks were exported".to_string());
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

/// Create a backup of the original IFO files
fn create_backup(zone_path: &PathBuf) -> std::io::Result<()> {
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
