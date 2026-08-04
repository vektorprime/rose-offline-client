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

/// Reads raw file bytes with real filesystem priority, falling back to the VFS.
/// Files are checked at base_path first, then VFS is used as fallback.
/// The global byte cache is consulted first (and populated on miss) so zone
/// revisits do not re-decompress the same files.
fn read_bytes_with_priority(
    vfs: &VirtualFilesystem,
    base_path: &Path,
    vfs_path: &VfsPath,
    zone_id: ZoneId,
) -> Result<Vec<u8>, anyhow::Error> {
    use rose_file_readers::VfsFile;

    let path_str = vfs_path.path().to_string_lossy().replace('\\', "/");

    // CHECK CACHE FIRST - avoids re-decompressing files already loaded this session
    if let Some(cached_data) = crate::vfs_asset_io::get_cached_bytes(&path_str) {
        return Ok((*cached_data).clone());
    }

    // PRIORITY: Check real filesystem first
    let real_filesystem_path = base_path.join(&path_str);
    if real_filesystem_path.exists() {
        match std::fs::read(&real_filesystem_path) {
            Ok(data) => {
                log::debug!(
                    "[VFS PRIORITY] Loaded from real filesystem: {} ({} bytes)",
                    path_str,
                    crate::vfs_asset_io::format_bytes(data.len())
                );
                return Ok(crate::vfs_asset_io::store_cached_bytes(
                    &path_str,
                    data,
                    Some(zone_id.get()),
                )
                .to_vec());
            }
            Err(e) => {
                log::warn!("[VFS PRIORITY] File exists on real filesystem but failed to read {}: {}, falling back to VFS",
                    path_str, e);
            }
        }
    }

    // FALLBACK: Load from VFS using open_file
    match vfs.open_file(vfs_path) {
        Ok(file) => {
            let data = match file {
                VfsFile::Buffer(buffer) => buffer,
                VfsFile::View(view) => view.into(),
            };
            Ok(crate::vfs_asset_io::store_cached_bytes(
                &path_str,
                data,
                Some(zone_id.get()),
            )
            .to_vec())
        }
        Err(e) => Err(anyhow::anyhow!(
            "Failed to open VFS file {}: {:?}",
            path_str,
            e
        )),
    }
}

pub(super) async fn load_zone(
    zone_id: ZoneId,
    vfs: &VirtualFilesystem,
    base_path: &Path,
    use_new_terrain: bool,
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
    let zon: ZonFile = match read_bytes_with_priority(vfs, base_path, &zon_file_path, zone_id) {
        Ok(data) => RoseFile::read(RoseFileReader::from(&data), &Default::default())
            .map_err(|e| anyhow::anyhow!("Failed to parse ZON file: {:?}", e))?,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to load ZON file: {:?}", e));
        }
    };

    // PRIORITY: Real filesystem takes priority over VFS
    let zsc_cnst: ZscFile = match read_bytes_with_priority(vfs, base_path, &zsc_cnst_path, zone_id)
    {
        Ok(data) => RoseFile::read(RoseFileReader::from(&data), &Default::default())
            .map_err(|e| anyhow::anyhow!("Failed to parse ZSC constant file: {:?}", e))?,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to load ZSC constant file: {:?}", e));
        }
    };

    // PRIORITY: Real filesystem takes priority over VFS
    let zsc_deco: ZscFile = match read_bytes_with_priority(vfs, base_path, &zsc_deco_path, zone_id)
    {
        Ok(data) => RoseFile::read(RoseFileReader::from(&data), &Default::default())
            .map_err(|e| anyhow::anyhow!("Failed to parse ZSC deco file: {:?}", e))?,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to load ZSC deco file: {:?}", e));
        }
    };

    let zone_path = zon_file_path_buf.parent().unwrap_or_else(|| Path::new(""));

    let mut zone_blocks = Vec::new();

    for block_y in 0..64 {
        for block_x in 0..64 {
            if let Ok(block) = load_block_files(
                vfs,
                base_path,
                zone_path,
                block_x,
                block_y,
                zone_id,
                use_new_terrain,
            )
            .await
            {
                zone_blocks.push(block);
            }
        }
    }

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
        zone_path: zone_path.into(),
        zone_id,
        zon,
        zsc_cnst,
        zsc_deco,
        blocks,
        npcs,
    })
}

async fn load_block_files(
    vfs: &VirtualFilesystem,
    base_path: &Path,
    zone_path: &Path,
    block_x: usize,
    block_y: usize,
    zone_id: ZoneId,
    use_new_terrain: bool,
) -> Result<Box<ZoneLoaderBlock>, anyhow::Error> {
    let him_path_buf = zone_path.join(format!("{}_{}.HIM", block_x, block_y));
    let him_path_str = him_path_buf.to_string_lossy().replace('\\', "/");
    let him_path = VfsPath::from(PathBuf::from(&him_path_str));

    // Load and parse HIM file (the single read also serves as the existence probe)
    let him: HimFile = match read_bytes_with_priority(vfs, base_path, &him_path, zone_id) {
        Ok(data) => {
            RoseFile::read(RoseFileReader::from(&data), &Default::default()).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse HIM file for block {}_{}: {:?}",
                    block_x,
                    block_y,
                    e
                )
            })?
        }
        Err(e) => {
            log::debug!(
                "[LOAD BLOCK DIRECT] HIM file not found for block {}_{}: {:?}. Skipping this block.",
                block_x,
                block_y,
                e
            );
            return Err(anyhow::anyhow!(
                "HIM file not found for block {}_{}",
                block_x,
                block_y
            ));
        }
    };

    // Load and parse TIL file (optional)
    let til_path_str = zone_path
        .join(format!("{}_{}.TIL", block_x, block_y))
        .to_string_lossy()
        .replace('\\', "/");
    let til_path = VfsPath::from(PathBuf::from(&til_path_str));
    let til: Option<TilFile> =
        match read_bytes_with_priority(vfs, base_path, &til_path, zone_id) {
            Ok(data) => RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok(),
            Err(_) => None,
        };

    // Load and parse IFO file (optional)
    let ifo_path = VfsPath::from(zone_path.join(format!("{}_{}.IFO", block_x, block_y)));
    let ifo: Option<IfoFile> = match read_bytes_with_priority(vfs, base_path, &ifo_path, zone_id) {
        Ok(data) => {
            log::debug!(
                "[IFO LOADER] Loading IFO file for block {}_{} ({} bytes)",
                block_x,
                block_y,
                data.len()
            );
            let result: Result<IfoFile, _> =
                RoseFile::read(RoseFileReader::from(&data), &Default::default());
            match result {
                Ok(ifo_file) => {
                    log::debug!("[IFO LOADER] Block {}_{} loaded: deco={}, cnst={}, event={}, warp={}, sound={}, effect={}, animated={}, npc={}, monster={}, water_planes={}",
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
    let lit_cnst: Option<LitFile> =
        match read_bytes_with_priority(vfs, base_path, &lit_cnst_path, zone_id) {
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
    let lit_deco: Option<LitFile> =
        match read_bytes_with_priority(vfs, base_path, &lit_deco_path, zone_id) {
            Ok(data) => RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok(),
            Err(_) => None,
        };

    let new_terrain_mesh = if use_new_terrain {
        let mesh_path =
            VfsPath::from(zone_path.join(format!("block_{}_{}.mesh.bin", block_x, block_y)));
        match read_bytes_with_priority(vfs, base_path, &mesh_path, zone_id) {
            Ok(data) => {
                log::debug!(
                    "[LOAD BLOCK FILES DIRECT] Found new terrain mesh for block {}_{}: {} bytes",
                    block_x,
                    block_y,
                    data.len()
                );
                Some(data)
            }
            Err(_) => {
                log::debug!(
                    "[LOAD BLOCK FILES DIRECT] New terrain mesh NOT FOUND for block {}_{}",
                    block_x,
                    block_y
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
    }))
}
