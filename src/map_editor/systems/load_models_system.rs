//! Model Loading System for Map Editor
//!
//! This system loads model information from ZSC files and populates the AvailableModels resource.

use bevy::prelude::*;
use rose_file_readers::{ZscFile, VfsPath, VirtualFilesystem, VfsFile, RoseFile, RoseFileReader};
use crate::resources::GameData;
use crate::VfsResource;
use crate::resources::CurrentZone;
use crate::zone_loader::ZoneLoaderAsset;
use crate::events::ZoneEvent;
use super::super::resources::{AvailableModels, ModelInfo, ModelCategory};

/// System to load available models from ZSC files on startup
///
/// This reads the ZSC files (event_object, special_object) from GameData and
/// (deco, cnst) from the current zone when available.
pub fn load_available_models_system(
    mut commands: Commands,
    game_data: Res<GameData>,
    vfs_resource: Res<VfsResource>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    available_models: Option<ResMut<AvailableModels>>,
) {
    // Skip if models are already loaded
    if available_models.is_some() {
        return;
    }
    
    log::info!("[LOAD MODELS] Loading available models from ZSC files...");
    
    let mut models = AvailableModels::default();
    
    // Load event object models from GameData
    load_models_from_zsc(
        &game_data.zsc_event_object,
        ModelCategory::Event,
        &mut models.event_models,
        "Event",
    );
    
    // Load special object models from GameData
    load_models_from_zsc(
        &game_data.zsc_special_object,
        ModelCategory::Special,
        &mut models.special_models,
        "Special",
    );
    
    // Try to load deco and cnst from current zone if available
    if let Some(current_zone) = current_zone {
        if let Some(zone_asset) = zone_loader_assets.get(&current_zone.handle) {
            load_models_from_zsc(
                &zone_asset.zsc_deco,
                ModelCategory::Deco,
                &mut models.deco_models,
                "Deco",
            );
            
            load_models_from_zsc(
                &zone_asset.zsc_cnst,
                ModelCategory::Cnst,
                &mut models.cnst_models,
                "Cnst",
            );
            
            log::info!(
                "[LOAD MODELS] Loaded zone-specific models: {} deco, {} cnst",
                models.deco_models.len(),
                models.cnst_models.len()
            );
        } else {
            log::debug!("[LOAD MODELS] Zone asset not yet loaded, loading defaults from VFS");
            load_default_deco_cnst_from_vfs(&vfs_resource.vfs, &vfs_resource.base_path, &mut models);
        }
    } else {
        // No zone loaded yet, try to load from VFS defaults
        log::debug!("[LOAD MODELS] No zone loaded, loading defaults from VFS");
        load_default_deco_cnst_from_vfs(&vfs_resource.vfs, &vfs_resource.base_path, &mut models);
    }
    
    log::info!(
        "[LOAD MODELS] Loaded {} deco, {} cnst, {} event, {} special models ({} total)",
        models.deco_models.len(),
        models.cnst_models.len(),
        models.event_models.len(),
        models.special_models.len(),
        models.total_count()
    );
    
    commands.insert_resource(models);
}

/// Load models from a ZSC file into a vector of ModelInfo
fn load_models_from_zsc(
    zsc: &ZscFile,
    category: ModelCategory,
    models: &mut Vec<ModelInfo>,
    category_name: &str,
) {
    for (object_id, object) in zsc.objects.iter().enumerate() {
        // Get the first part's mesh path as the primary mesh
        let primary_mesh_path = object.parts.first()
            .map(|part| {
                let mesh_id = part.mesh_id as usize;
                zsc.meshes.get(mesh_id)
                    .map(|m| m.path().to_string_lossy().into_owned())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        
        // Create a display name from the mesh path
        let name = create_model_name(&primary_mesh_path, object_id, category_name);
        
        let model_info = ModelInfo::new(
            object_id as u32,
            name,
            primary_mesh_path,
            category,
            object.parts.len(),
        );
        
        models.push(model_info);
    }
}

/// Load default deco and cnst models from VFS (fallback when no zone is loaded)
/// IMPORTANT: Real filesystem takes priority over VFS to support map editor modifications.
fn load_default_deco_cnst_from_vfs(
    vfs: &VirtualFilesystem,
    base_path: &std::path::Path,
    models: &mut AvailableModels,
) {
    // Try common paths for ZSC files
    let deco_paths = [
        "3DDATA/ZSC_DECO.TXT",
        "3DDATA/ZONES/JUNON/ZSC_DECO.TXT",
    ];
    
    let cnst_paths = [
        "3DDATA/ZSC_CNST.TXT",
        "3DDATA/ZONES/JUNON/ZSC_CNST.TXT",
    ];
    
    for path in deco_paths {
        if try_load_zsc_with_priority(vfs, base_path, path, ModelCategory::Deco, &mut models.deco_models, "Deco") {
            break;
        }
    }
    
    for path in cnst_paths {
        if try_load_zsc_with_priority(vfs, base_path, path, ModelCategory::Cnst, &mut models.cnst_models, "Cnst") {
            break;
        }
    }
}

/// Try to load a ZSC file with real filesystem priority
/// Checks real filesystem first, then falls back to VFS
fn try_load_zsc_with_priority(
    vfs: &VirtualFilesystem,
    base_path: &std::path::Path,
    path: &str,
    category: ModelCategory,
    models: &mut Vec<ModelInfo>,
    category_name: &str,
) -> bool {
    let vfs_path = VfsPath::from(std::path::PathBuf::from(path));
    
    // PRIORITY: Check real filesystem first
    let real_filesystem_path = base_path.join(path);
    if real_filesystem_path.exists() {
        match std::fs::read(&real_filesystem_path) {
            Ok(data) => {
                match RoseFile::read(RoseFileReader::from(&data), &Default::default()) {
                    Ok(zsc) => {
                        load_models_from_zsc(&zsc, category, models, category_name);
                        log::info!(
                            "[LOAD MODELS] Loaded {} {} models from real filesystem (priority): {}",
                            models.len(),
                            category_name,
                            path
                        );
                        return true;
                    }
                    Err(e) => {
                        log::warn!(
                            "[LOAD MODELS] Failed to parse ZSC from real filesystem {}: {:?}, trying VFS",
                            path, e
                        );
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "[LOAD MODELS] Failed to read ZSC from real filesystem {}: {:?}, trying VFS",
                    path, e
                );
            }
        }
    }
    
    // FALLBACK: Load from VFS
    match vfs.open_file(&vfs_path) {
        Ok(file) => {
            let data: Vec<u8> = match file {
                VfsFile::Buffer(buffer) => buffer,
                VfsFile::View(view) => view.into(),
            };
            match RoseFile::read(RoseFileReader::from(&data), &Default::default()) {
                Ok(zsc) => {
                    load_models_from_zsc(&zsc, category, models, category_name);
                    log::info!(
                        "[LOAD MODELS] Loaded {} {} models from VFS: {}",
                        models.len(),
                        category_name,
                        path
                    );
                    true
                }
                Err(e) => {
                    log::debug!("[LOAD MODELS] Could not parse ZSC {}: {:?}", path, e);
                    false
                }
            }
        }
        Err(e) => {
            log::debug!("[LOAD MODELS] Could not load {}: {:?}", path, e);
            false
        }
    }
}

/// Try to load a ZSC file from VFS (legacy - use try_load_zsc_with_priority instead)
#[allow(dead_code)]
fn try_load_zsc_from_vfs(
    vfs: &VirtualFilesystem,
    path: &str,
    category: ModelCategory,
    models: &mut Vec<ModelInfo>,
    category_name: &str,
) -> bool {
    let vfs_path = VfsPath::from(std::path::PathBuf::from(path));
    
    match vfs.read_file(&vfs_path) {
        Ok(zsc) => {
            load_models_from_zsc(&zsc, category, models, category_name);
            log::info!(
                "[LOAD MODELS] Loaded {} {} models from VFS: {}",
                models.len(),
                category_name,
                path
            );
            true
        }
        Err(e) => {
            log::debug!("[LOAD MODELS] Could not load {}: {:?}", path, e);
            false
        }
    }
}

/// Create a display name for a model from its mesh path
fn create_model_name(mesh_path: &str, object_id: usize, category_name: &str) -> String {
    if mesh_path.is_empty() {
        return format!("{}_{}", category_name, object_id);
    }
    
    // Extract the file name without extension
    let path = mesh_path.replace('\\', "/");
    let file_name = if let Some(pos) = path.rfind('/') {
        &path[pos + 1..]
    } else {
        &path
    };
    
    // Remove the .ZMS extension if present
    let name = if file_name.to_uppercase().ends_with(".ZMS") {
        &file_name[..file_name.len() - 4]
    } else {
        file_name
    };
    
    name.to_string()
}

/// System to update AvailableModels when a zone is loaded
///
/// This listens for ZoneEvent::Loaded events and updates the deco/cnst models
/// from the newly loaded zone's ZSC files.
pub fn update_models_on_zone_load_system(
    mut events: EventReader<ZoneEvent>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    mut available_models: Option<ResMut<AvailableModels>>,
) {
    // Check if any zone was loaded
    let mut zone_loaded = false;
    for event in events.read() {
        if let ZoneEvent::Loaded(_) = event {
            zone_loaded = true;
            break;
        }
    }
    
    if !zone_loaded {
        return;
    }
    
    // Get the current zone's asset
    let Some(current_zone) = current_zone else {
        log::debug!("[UPDATE MODELS] Zone loaded but no CurrentZone resource");
        return;
    };
    
    let Some(zone_asset) = zone_loader_assets.get(&current_zone.handle) else {
        log::debug!("[UPDATE MODELS] Zone loaded but asset not yet available");
        return;
    };
    
    // Update or create AvailableModels
    if let Some(mut models) = available_models {
        // Clear existing deco/cnst models and reload from zone
        models.deco_models.clear();
        models.cnst_models.clear();
        
        load_models_from_zsc(
            &zone_asset.zsc_deco,
            ModelCategory::Deco,
            &mut models.deco_models,
            "Deco",
        );
        
        load_models_from_zsc(
            &zone_asset.zsc_cnst,
            ModelCategory::Cnst,
            &mut models.cnst_models,
            "Cnst",
        );
        
        log::info!(
            "[UPDATE MODELS] Updated models from loaded zone: {} deco, {} cnst",
            models.deco_models.len(),
            models.cnst_models.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_model_name() {
        assert_eq!(
            create_model_name("3DDATA/OBJECT/BUILDING/TREE01.ZMS", 0, "Deco"),
            "TREE01"
        );
        assert_eq!(
            create_model_name("3DDATA\\OBJECT\\BUILDING\\HOUSE01.ZMS", 1, "Cnst"),
            "HOUSE01"
        );
        assert_eq!(
            create_model_name("", 2, "Event"),
            "Event_2"
        );
    }
}
