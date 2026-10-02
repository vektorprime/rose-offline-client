use std::collections::VecDeque;

use rose_file_readers::VfsFile;

use super::*;

#[derive(Default, TypePath)]
pub struct ZoneLoader;

static ZONE_LIST: OnceLock<Arc<ZoneList>> = OnceLock::new();

impl ZoneLoader {
    pub fn init_zone_list(zone_list: Arc<ZoneList>) {
        let _ = ZONE_LIST.set(zone_list);
    }

    fn get_zone_list() -> Arc<ZoneList> {
        ZONE_LIST.get().expect("ZoneList not initialized").clone()
    }
}

/// Path of `vfs_path` relative to the game data directory on the real filesystem.
fn real_filesystem_path_str(vfs_path: &VfsPath) -> String {
    vfs_path.path().to_string_lossy().replace('\\', "/")
}

/// Reads `path_str` from the real filesystem, which takes priority over the VFS.
/// `None` means the VFS copy should be used.
fn read_real_file(base_path: &Path, path_str: &str) -> Option<Vec<u8>> {
    let real_filesystem_path = base_path.join(path_str);
    // One read instead of exists() + read: most files are only in the VFS.
    match std::fs::read(&real_filesystem_path) {
        Ok(data) => {
            log::info!(
                "[VFS PRIORITY] Loaded from real filesystem: {} ({} bytes)",
                path_str,
                crate::vfs_asset_io::format_bytes(data.len())
            );
            Some(data)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            // Warn only for files that exist, as the former exists() check did.
            if real_filesystem_path.exists() {
                log::warn!("[VFS PRIORITY] File exists on real filesystem but failed to read {}: {}, falling back to VFS",
                    path_str, e);
            }
            None
        }
    }
}

fn vfs_file_into_bytes(file: VfsFile) -> Vec<u8> {
    match file {
        VfsFile::Buffer(buffer) => buffer,
        VfsFile::View(view) => view.into(),
    }
}

/// Reads raw file bytes with real filesystem priority, falling back to the VFS.
/// Files are checked at base_path first, then VFS is used as fallback.
fn read_bytes_with_priority(
    vfs: &VirtualFilesystem,
    base_path: &Path,
    vfs_path: &VfsPath,
) -> Result<Vec<u8>, anyhow::Error> {
    let path_str = real_filesystem_path_str(vfs_path);

    // PRIORITY: Check real filesystem first
    if let Some(data) = read_real_file(base_path, &path_str) {
        return Ok(data);
    }

    // FALLBACK: Load from VFS using open_file
    match vfs.open_file(vfs_path) {
        Ok(file) => Ok(vfs_file_into_bytes(file)),
        Err(e) => Err(anyhow::anyhow!(
            "Failed to open VFS file {}: {:?}",
            path_str,
            e
        )),
    }
}

pub(super) async fn load_zone(
    zone_id: ZoneId,
    vfs: Arc<VirtualFilesystem>,
    base_path: PathBuf,
    use_new_terrain: bool,
    terrain_noise: Arc<crate::terrain::GlobalTerrainNoise>,
) -> Result<ZoneLoaderAsset, anyhow::Error> {
    let zone_list = ZoneLoader::get_zone_list();
    let zone_list_entry = zone_list
        .get_zone(zone_id)
        .ok_or(ZoneLoadError::InvalidZoneId)?;
    let zon_file_path_buf = zone_list_entry.zon_file_path.path().to_path_buf();
    let zon_file_path = VfsPath::from(zon_file_path_buf.clone());
    let zsc_cnst_path = VfsPath::from(zone_list_entry.zsc_cnst_path.path().to_path_buf());
    let zsc_deco_path = VfsPath::from(zone_list_entry.zsc_deco_path.path().to_path_buf());

    // PRIORITY: Real filesystem takes priority over VFS
    let zon: ZonFile = match read_bytes_with_priority(&vfs, &base_path, &zon_file_path) {
        Ok(data) => RoseFile::read(RoseFileReader::from(&data), &Default::default())
            .map_err(|e| anyhow::anyhow!("Failed to parse ZON file: {:?}", e))?,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to load ZON file: {:?}", e));
        }
    };

    // PRIORITY: Real filesystem takes priority over VFS
    let zsc_cnst: ZscFile = match read_bytes_with_priority(&vfs, &base_path, &zsc_cnst_path) {
        Ok(data) => RoseFile::read(RoseFileReader::from(&data), &Default::default())
            .map_err(|e| anyhow::anyhow!("Failed to parse ZSC constant file: {:?}", e))?,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to load ZSC constant file: {:?}", e));
        }
    };

    // PRIORITY: Real filesystem takes priority over VFS
    let zsc_deco: ZscFile = match read_bytes_with_priority(&vfs, &base_path, &zsc_deco_path) {
        Ok(data) => RoseFile::read(RoseFileReader::from(&data), &Default::default())
            .map_err(|e| anyhow::anyhow!("Failed to parse ZSC deco file: {:?}", e))?,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to load ZSC deco file: {:?}", e));
        }
    };

    let zone_path: Arc<Path> =
        Arc::from(zon_file_path_buf.parent().unwrap_or_else(|| Path::new("")));
    let base_path: Arc<Path> = Arc::from(base_path);

    // The tile texture count matches spawn_zone's list, which stops at the "end" entry.
    let tile_texture_count = zon
        .tile_textures
        .iter()
        .take_while(|path| path.as_str() != "end")
        .count();
    let zon = Arc::new(zon);

    // Each block position is loaded (files + terrain geometry) by its own task on this
    // pool. Only a few run at a time, so other users of the pool (e.g. render pipeline
    // compilation) are never queued behind all 4096 positions. Tasks are awaited in
    // spawn order (an await yields, it does not block a pool thread), so the block list
    // is the same as loading them one after another.
    let pool = AsyncComputeTaskPool::get();
    let max_blocks_in_flight = pool.thread_num().max(1) * 2;
    let mut blocks_in_flight: VecDeque<Task<Result<Box<ZoneLoaderBlock>, anyhow::Error>>> =
        VecDeque::with_capacity(max_blocks_in_flight);
    let mut zone_blocks = Vec::new();

    // Most positions have no HIM file, so their task would only fail to open it. Skip
    // the ones no VFS device can open (same blocks in the same order); a device that
    // cannot list the zone directory makes every position load as before.
    let block_files = vfs.list_dir(&*zone_path);

    for block_y in 0..64 {
        for block_x in 0..64 {
            if !block_files.may_exist(&block_him_path(&zone_path, block_x, block_y)) {
                continue;
            }

            if blocks_in_flight.len() >= max_blocks_in_flight {
                if let Some(task) = blocks_in_flight.pop_front() {
                    if let Ok(block) = task.await {
                        zone_blocks.push(block);
                    }
                }
            }

            let vfs = vfs.clone();
            let base_path = base_path.clone();
            let zone_path = zone_path.clone();
            let zon = zon.clone();
            let terrain_noise = terrain_noise.clone();
            blocks_in_flight.push_back(pool.spawn(async move {
                load_block(
                    &vfs,
                    &base_path,
                    &zone_path,
                    block_x,
                    block_y,
                    use_new_terrain,
                    &zon,
                    tile_texture_count,
                    &terrain_noise,
                )
            }));
        }
    }
    while let Some(task) = blocks_in_flight.pop_front() {
        if let Ok(block) = task.await {
            zone_blocks.push(block);
        }
    }

    // Every block task has completed, and a task drops its future (with its clone of
    // `zon`) before its result can be awaited, so this is the only reference left.
    let zon = Arc::try_unwrap(zon).map_err(|_| {
        anyhow::anyhow!(
            "ZON data of zone {} still shared after loading its blocks",
            zone_id.get()
        )
    })?;

    let mut npcs = Vec::new();
    let mut blocks = Vec::new();
    blocks.resize_with(64 * 64, || None);
    for block in zone_blocks {
        let index = block.block_x + block.block_y * 64;

        if let Some(ifo) = &block.ifo {
            let objects_offset = Vec3::new(
                (64.0 / 2.0) * (zon.grid_size * zon.grid_per_patch * 16.0)
                    + (zon.grid_size * zon.grid_per_patch * 16.0) / 2.0,
                (64.0 / 2.0) * (zon.grid_size * zon.grid_per_patch * 16.0)
                    + (zon.grid_size * zon.grid_per_patch * 16.0) / 2.0,
                0.0,
            );

            for npc in ifo.npcs.iter() {
                let Some(npc_id) = NpcId::new(npc.object.object_id as u16) else {
                    continue;
                };

                npcs.push(ZoneNpc {
                    npc_id,
                    position: Vec3::new(
                        npc.object.position.x,
                        npc.object.position.y,
                        npc.object.position.z,
                    ) + objects_offset,
                });
            }
        }

        blocks[index] = Some(block);
    }

    Ok(ZoneLoaderAsset {
        zone_path: zone_path.to_path_buf(),
        zone_id,
        zon,
        zsc_cnst,
        zsc_deco,
        blocks,
        npcs,
    })
}

/// Loads one block's files and builds its terrain mesh, bounds and collider here, on
/// the load task, instead of inside spawn_zone on the main thread. Same functions and
/// inputs as the spawn-time path (which stays as a fallback).
#[allow(clippy::too_many_arguments)]
fn load_block(
    vfs: &VirtualFilesystem,
    base_path: &Path,
    zone_path: &Path,
    block_x: usize,
    block_y: usize,
    use_new_terrain: bool,
    zon: &ZonFile,
    tile_texture_count: usize,
    terrain_noise: &crate::terrain::GlobalTerrainNoise,
) -> Result<Box<ZoneLoaderBlock>, anyhow::Error> {
    let mut block = load_block_files(vfs, base_path, zone_path, block_x, block_y, use_new_terrain)?;
    block.terrain_geometry = if use_new_terrain && block.new_terrain_mesh.is_some() {
        super::spawning::build_new_terrain_geometry(&block)
    } else {
        super::spawning::build_terrain_geometry(zon, tile_texture_count, &block, terrain_noise)
    };
    Ok(block)
}

/// VFS path of a block's HIM file. A block exists when the VFS can open this file.
fn block_him_path(zone_path: &Path, block_x: usize, block_y: usize) -> VfsPath<'static> {
    let him_path_buf = zone_path.join(format!("{}_{}.HIM", block_x, block_y));
    let him_path_str = him_path_buf.to_string_lossy().replace('\\', "/");
    VfsPath::from(PathBuf::from(&him_path_str))
}

fn load_block_files(
    vfs: &VirtualFilesystem,
    base_path: &Path,
    zone_path: &Path,
    block_x: usize,
    block_y: usize,
    use_new_terrain: bool,
) -> Result<Box<ZoneLoaderBlock>, anyhow::Error> {
    let him_path = block_him_path(zone_path, block_x, block_y);

    // The block exists if the VFS has its HIM file. That open file is also the HIM
    // data unless a real filesystem copy takes priority, so it is not opened twice.
    let him_vfs_file = match vfs.open_file(&him_path) {
        Ok(file) => file,
        Err(_) => {
            return Err(anyhow::anyhow!(
                "HIM file not found for block {}_{}",
                block_x,
                block_y
            ));
        }
    };

    // Load and parse HIM file
    let him_data = match read_real_file(base_path, &real_filesystem_path_str(&him_path)) {
        Some(data) => data,
        None => vfs_file_into_bytes(him_vfs_file),
    };
    let him: HimFile = RoseFile::read(RoseFileReader::from(&him_data), &Default::default())
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse HIM file for block {}_{}: {:?}",
                block_x,
                block_y,
                e
            )
        })?;

    // Load and parse TIL file (optional)
    let til_path_str = zone_path
        .join(format!("{}_{}.TIL", block_x, block_y))
        .to_string_lossy()
        .replace('\\', "/");
    let til_path = VfsPath::from(PathBuf::from(&til_path_str));
    let til: Option<TilFile> = match read_bytes_with_priority(vfs, base_path, &til_path) {
        Ok(data) => RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok(),
        Err(_) => None,
    };

    // Load and parse IFO file (optional)
    let ifo_path = VfsPath::from(zone_path.join(format!("{}_{}.IFO", block_x, block_y)));
    let ifo: Option<IfoFile> = match read_bytes_with_priority(vfs, base_path, &ifo_path) {
        Ok(data) => {
            log::info!(
                "[IFO LOADER] Loading IFO file for block {}_{} ({} bytes)",
                block_x,
                block_y,
                data.len()
            );
            let result: Result<IfoFile, _> =
                RoseFile::read(RoseFileReader::from(&data), &Default::default());
            match result {
                Ok(ifo_file) => {
                    log::info!("[IFO LOADER] Block {}_{} loaded: deco={}, cnst={}, event={}, warp={}, sound={}, effect={}, animated={}, npc={}, monster={}, water_planes={}",
                        block_x, block_y,
                        ifo_file.deco_objects.len(),
                        ifo_file.cnst_objects.len(),
                        ifo_file.event_objects.len(),
                        ifo_file.warps.len(),
                        ifo_file.sound_objects.len(),
                        ifo_file.effect_objects.len(),
                        ifo_file.animated_objects.len(),
                        ifo_file.npcs.len(),
                        ifo_file.monster_spawns.len(),
                        ifo_file.water_planes.len()
                    );
                    // Log first few deco objects for debugging
                    for (i, obj) in ifo_file.deco_objects.iter().take(3).enumerate() {
                        log::info!("[IFO LOADER]   Deco[{}]: object_id={}, name='{}', pos=({:.2}, {:.2}, {:.2})",
                            i, obj.object_id, obj.object_name, obj.position.x, obj.position.y, obj.position.z);
                    }
                    Some(ifo_file)
                }
                Err(e) => {
                    log::error!(
                        "[IFO LOADER] Failed to parse IFO file for block {}_{}: {:?}",
                        block_x,
                        block_y,
                        e
                    );
                    None
                }
            }
        }
        Err(_) => {
            log::debug!(
                "[IFO LOADER] No IFO file found for block {}_{}",
                block_x,
                block_y
            );
            None
        }
    };

    // Load and parse LIT constant file (optional)
    let lit_cnst_path_str = zone_path
        .join(format!(
            "{}_{}/LIGHTMAP/BUILDINGLIGHTMAPDATA.LIT",
            block_x, block_y
        ))
        .to_string_lossy()
        .replace('\\', "/");
    let lit_cnst_path = VfsPath::from(PathBuf::from(&lit_cnst_path_str));
    let lit_cnst: Option<LitFile> = match read_bytes_with_priority(vfs, base_path, &lit_cnst_path) {
        Ok(data) => RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok(),
        Err(_) => None,
    };

    // Load and parse LIT deco file (optional)
    let lit_deco_path_str = zone_path
        .join(format!(
            "{}_{}/LIGHTMAP/OBJECTLIGHTMAPDATA.LIT",
            block_x, block_y
        ))
        .to_string_lossy()
        .replace('\\', "/");
    let lit_deco_path = VfsPath::from(PathBuf::from(&lit_deco_path_str));
    let lit_deco: Option<LitFile> = match read_bytes_with_priority(vfs, base_path, &lit_deco_path) {
        Ok(data) => RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok(),
        Err(_) => None,
    };

    let new_terrain_mesh = if use_new_terrain {
        let mesh_path =
            VfsPath::from(zone_path.join(format!("block_{}_{}.mesh.bin", block_x, block_y)));
        match read_bytes_with_priority(vfs, base_path, &mesh_path) {
            Ok(data) => {
                log::info!(
                    "[LOAD BLOCK FILES DIRECT] Found new terrain mesh for block {}_{}: {} bytes",
                    block_x,
                    block_y,
                    data.len()
                );
                Some(data)
            }
            Err(e) => {
                log::warn!(
                    "[LOAD BLOCK FILES DIRECT] New terrain mesh NOT FOUND for block {}_{}: {:?}",
                    block_x,
                    block_y,
                    e
                );
                None
            }
        }
    } else {
        None
    };

    Ok(Box::new(ZoneLoaderBlock {
        block_x,
        block_y,
        til,
        him,
        ifo,
        lit_cnst,
        lit_deco,
        new_terrain_mesh,
        terrain_geometry: None,
    }))
}
