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

impl AssetLoader for ZoneLoader {
    type Asset = ZoneLoaderAsset;
    type Settings = ();
    type Error = anyhow::Error;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>> + Send {
        async move {
            log::info!("[ZONE LOADER ASSET LOADER] ===========================================");
            log::info!("[ZONE LOADER ASSET LOADER] ZoneLoader::load called");
            log::info!("[ZONE LOADER ASSET LOADER] ===========================================");

            let mut bytes = Vec::new();
            log::info!("[ZONE LOADER ASSET LOADER] Reading bytes from reader...");
            reader.read_to_end(&mut bytes).await?;
            log::info!("[ZONE LOADER ASSET LOADER] Read {} bytes", bytes.len());

            let zone_id = ZoneId::new(bytes[0] as u16).unwrap();
            log::info!(
                "[ZONE LOADER ASSET LOADER] Zone ID parsed: {}",
                zone_id.get()
            );

            log::info!("[ZONE LOADER ASSET LOADER] Calling load_zone...");
            let result = load_zone(zone_id, load_context, false).await;
            log::info!(
                "[ZONE LOADER ASSET LOADER] load_zone completed with result: {:?}",
                result.is_ok()
            );
            result
        }
    }

    fn extensions(&self) -> &[&str] {
        &["zone_loader"]
    }
}

async fn load_zone<'a, 'b>(
    zone_id: ZoneId,
    load_context: &'a mut LoadContext<'b>,
    use_new_terrain: bool,
) -> Result<ZoneLoaderAsset, anyhow::Error> {
    log::info!("[ZONE LOADER DIAGNOSTIC] ===========================================");
    log::info!(
        "[ZONE LOADER DIAGNOSTIC] load_zone called for zone_id: {}",
        zone_id.get()
    );
    log::info!("[ZONE LOADER DIAGNOSTIC] ===========================================");

    let zone_list = ZoneLoader::get_zone_list();
    let zone_list_entry = zone_list
        .get_zone(zone_id)
        .ok_or(ZoneLoadError::InvalidZoneId)?;
    let zon_file_path = zone_list_entry.zon_file_path.path().to_path_buf();
    let zsc_cnst_path = zone_list_entry.zsc_cnst_path.path().to_path_buf();
    let zsc_deco_path = zone_list_entry.zsc_deco_path.path().to_path_buf();

    log::info!(
        "[ZONE LOADER DIAGNOSTIC] Loading ZON file: {:?}",
        zon_file_path
    );
    let zon: ZonFile = RoseFile::read(
        RoseFileReader::from(
            &(*load_context)
                .read_asset_bytes(zon_file_path.clone())
                .await?,
        ),
        &Default::default(),
    )?;
    log::info!("[ZONE LOADER DIAGNOSTIC] ZON file loaded successfully");

    log::info!(
        "[ZONE LOADER DIAGNOSTIC] Loading ZSC constant file: {:?}",
        zsc_cnst_path
    );
    let zsc_cnst: ZscFile = RoseFile::read(
        RoseFileReader::from(
            &(*load_context)
                .read_asset_bytes(zsc_cnst_path.clone())
                .await?,
        ),
        &Default::default(),
    )?;
    log::info!("[ZONE LOADER DIAGNOSTIC] ZSC constant file loaded successfully");

    log::info!(
        "[ZONE LOADER DIAGNOSTIC] Loading ZSC deco file: {:?}",
        zsc_deco_path
    );
    let zsc_deco: ZscFile = RoseFile::read(
        RoseFileReader::from(
            &(*load_context)
                .read_asset_bytes(zsc_deco_path.clone())
                .await?,
        ),
        &Default::default(),
    )?;
    log::info!("[ZONE LOADER DIAGNOSTIC] ZSC deco file loaded successfully");

    let zone_path = zon_file_path.parent().unwrap_or_else(|| Path::new(""));

    log::info!("[ZONE LOADER DIAGNOSTIC] ===========================================");
    log::info!("[ZONE LOADER DIAGNOSTIC] Starting to load zone blocks (64x64 = 4096 blocks)");
    log::info!("[ZONE LOADER DIAGNOSTIC] Zone path: {:?}", zone_path);
    log::info!("[ZONE LOADER DIAGNOSTIC] ===========================================");

    let mut zone_blocks = Vec::new();
    let mut blocks_loaded = 0;
    let mut blocks_failed = 0;

    for block_y in 0..64 {
        for block_x in 0..64 {
            if let Ok(block) =
                load_block_files(load_context, zone_path, block_x, block_y, use_new_terrain).await
            {
                zone_blocks.push(block);
                blocks_loaded += 1;
            } else {
                blocks_failed += 1;
            }

            // Log progress every 100 blocks
            if (block_x + block_y * 64) % 100 == 0 {
                log::info!(
                    "[ZONE LOADER DIAGNOSTIC] Block loading progress: {} loaded, {} failed",
                    blocks_loaded,
                    blocks_failed
                );
            }
        }
    }

    log::info!("[ZONE LOADER DIAGNOSTIC] ===========================================");
    log::info!(
        "[ZONE LOADER DIAGNOSTIC] Block loading complete: {} loaded, {} failed",
        blocks_loaded,
        blocks_failed
    );
    log::info!("[ZONE LOADER DIAGNOSTIC] ===========================================");

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

/// WORKAROUND: Load zone directly from VFS without using Bevy's AssetServer
/// This bypasses the broken asset loading pipeline in Bevy 0.13.2
///
/// IMPORTANT: Real filesystem takes priority over VFS to support map editor modifications.
/// Files are checked at base_path first, then VFS is used as fallback.
/// Helper function to read raw bytes with real filesystem priority for use in load_zone_direct
/// Returns the raw file data either from real filesystem or VFS
fn read_bytes_with_priority_sync(
    vfs: &VirtualFilesystem,
    base_path: &Path,
    vfs_path: &VfsPath,
) -> Result<Vec<u8>, anyhow::Error> {
    use rose_file_readers::VfsFile;

    let path_str = vfs_path.path().to_string_lossy().replace('\\', "/");

    // PRIORITY: Check real filesystem first
    let real_filesystem_path = base_path.join(&path_str);
    if real_filesystem_path.exists() {
        match std::fs::read(&real_filesystem_path) {
            Ok(data) => {
                log::info!(
                    "[VFS PRIORITY] Loaded from real filesystem: {} ({} bytes)",
                    path_str,
                    memory_monitor::format_bytes(data.len() as u64)
                );
                return Ok(data);
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
            Ok(data)
        }
        Err(e) => Err(anyhow::anyhow!(
            "Failed to open VFS file {}: {:?}",
            path_str,
            e
        )),
    }
}

pub(super) async fn load_zone_direct(
    zone_id: ZoneId,
    vfs: &VirtualFilesystem,
    base_path: &Path,
    use_new_terrain: bool,
) -> Result<ZoneLoaderAsset, anyhow::Error> {
    //log::info!("[ZONE LOADER DIRECT] ===========================================");
    //log::info!("[ZONE LOADER DIRECT] load_zone_direct called for zone_id: {}", zone_id.get());
    //log::info!("[ZONE LOADER DIRECT] ===========================================");

    let zone_list = ZoneLoader::get_zone_list();
    let zone_list_entry = zone_list
        .get_zone(zone_id)
        .ok_or(ZoneLoadError::InvalidZoneId)?;
    let zon_file_path_buf = zone_list_entry.zon_file_path.path().to_path_buf();
    let zon_file_path = VfsPath::from(zon_file_path_buf.clone());
    let zsc_cnst_path = VfsPath::from(zone_list_entry.zsc_cnst_path.path().to_path_buf());
    let zsc_deco_path = VfsPath::from(zone_list_entry.zsc_deco_path.path().to_path_buf());

    //log::info!("[ZONE LOADER DIRECT] Loading ZON file: {:?}", zon_file_path);
    // PRIORITY: Real filesystem takes priority over VFS
    let zon: ZonFile = match read_bytes_with_priority_sync(vfs, base_path, &zon_file_path) {
        Ok(data) => RoseFile::read(RoseFileReader::from(&data), &Default::default())
            .map_err(|e| anyhow::anyhow!("Failed to parse ZON file: {:?}", e))?,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to load ZON file: {:?}", e));
        }
    };
    //log::info!("[ZONE LOADER DIRECT] ZON file loaded successfully");

    //log::info!("[ZONE LOADER DIRECT] Loading ZSC constant file: {:?}", zsc_cnst_path);
    // PRIORITY: Real filesystem takes priority over VFS
    let zsc_cnst: ZscFile = match read_bytes_with_priority_sync(vfs, base_path, &zsc_cnst_path) {
        Ok(data) => RoseFile::read(RoseFileReader::from(&data), &Default::default())
            .map_err(|e| anyhow::anyhow!("Failed to parse ZSC constant file: {:?}", e))?,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to load ZSC constant file: {:?}", e));
        }
    };
    //log::info!("[ZONE LOADER DIRECT] ZSC constant file loaded successfully");

    //log::info!("[ZONE LOADER DIRECT] Loading ZSC deco file: {:?}", zsc_deco_path);
    // PRIORITY: Real filesystem takes priority over VFS
    let zsc_deco: ZscFile = match read_bytes_with_priority_sync(vfs, base_path, &zsc_deco_path) {
        Ok(data) => RoseFile::read(RoseFileReader::from(&data), &Default::default())
            .map_err(|e| anyhow::anyhow!("Failed to parse ZSC deco file: {:?}", e))?,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to load ZSC deco file: {:?}", e));
        }
    };
    //log::info!("[ZONE LOADER DIRECT] ZSC deco file loaded successfully");

    let zone_path = zon_file_path_buf.parent().unwrap_or_else(|| Path::new(""));

    //log::info!("[ZONE LOADER DIRECT] ===========================================");
    //log::info!("[ZONE LOADER DIRECT] Starting to load zone blocks (64x64 = 4096 blocks)");
    //log::info!("[ZONE LOADER DIRECT] Zone path: {:?}", zone_path);
    //log::info!("[ZONE LOADER DIRECT] Note: Blocks without HIM files will be skipped");
    //log::info!("[ZONE LOADER DIRECT] ===========================================");

    let mut zone_blocks = Vec::new();
    let mut blocks_loaded = 0;
    let mut blocks_skipped = 0;
    let mut skipped_blocks = Vec::new();

    for block_y in 0..64 {
        for block_x in 0..64 {
            match load_block_files_direct(
                vfs,
                base_path,
                zone_path,
                block_x,
                block_y,
                use_new_terrain,
            )
            .await
            {
                Ok(block) => {
                    zone_blocks.push(block);
                    blocks_loaded += 1;
                }
                Err(e) => {
                    blocks_skipped += 1;
                    // Only track first 50 skipped blocks to avoid excessive memory usage
                    if skipped_blocks.len() < 50 {
                        skipped_blocks.push((block_x, block_y, e.to_string()));
                    }
                    //log::trace!("[ZONE LOADER DIRECT] Block {}_{} skipped: {}", block_x, block_y, e);
                }
            }

            // Log progress every 100 blocks
            if (block_x + block_y * 64) % 100 == 0 {
                //log::info!("[ZONE LOADER DIRECT] Block loading progress: {} loaded, {} skipped", blocks_loaded, blocks_skipped);
            }
        }
    }

    //log::info!("[ZONE LOADER DIRECT] ===========================================");
    //log::info!("[ZONE LOADER DIRECT] Block loading complete: {} loaded, {} skipped", blocks_loaded, blocks_skipped);
    if !skipped_blocks.is_empty() {
        //log::info!("[ZONE LOADER DIRECT] Skipped blocks (first 10):");
        for (block_x, block_y, error) in skipped_blocks.iter().take(10) {
            //log::info!("[ZONE LOADER DIRECT]   Block {}_{}: {}", block_x, block_y, error);
        }
        if skipped_blocks.len() > 10 {
            //log::info!("[ZONE LOADER DIRECT]   ... and {} more skipped blocks", skipped_blocks.len() - 10);
        }
        //log::info!("[ZONE LOADER DIRECT] Zone will spawn with {} blocks", blocks_loaded);
    }
    //log::info!("[ZONE LOADER DIRECT] ===========================================");

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

/// Load block files using Bevy's LoadContext (for AssetLoader implementation)
async fn load_block_files<'a>(
    load_context: &mut LoadContext<'a>,
    zone_path: &Path,
    block_x: usize,
    block_y: usize,
    use_new_terrain: bool,
) -> Result<Box<ZoneLoaderBlock>, anyhow::Error> {
    let him_path = zone_path.join(format!("{}_{}.HIM", block_x, block_y));
    log::trace!(
        "[LOAD BLOCK] Loading block {}_{} from: {:?}",
        block_x,
        block_y,
        him_path
    );

    let him = RoseFile::read(
        RoseFileReader::from(&load_context.read_asset_bytes(him_path.clone()).await?),
        &Default::default(),
    )?;

    let til = if let Ok(data) = load_context
        .read_asset_bytes(zone_path.join(format!("{}_{}.TIL", block_x, block_y)))
        .await
    {
        RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok()
    } else {
        None
    };

    let ifo = if let Ok(data) = load_context
        .read_asset_bytes(zone_path.join(format!("{}_{}.IFO", block_x, block_y)))
        .await
    {
        RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok()
    } else {
        None
    };

    let lit_cnst = if let Ok(data) = load_context
        .read_asset_bytes(zone_path.join(format!(
            "{}_{}/LIGHTMAP/BUILDINGLIGHTMAPDATA.LIT",
            block_x, block_y
        )))
        .await
    {
        RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok()
    } else {
        None
    };

    let lit_deco = if let Ok(data) = load_context
        .read_asset_bytes(zone_path.join(format!(
            "{}_{}/LIGHTMAP/OBJECTLIGHTMAPDATA.LIT",
            block_x, block_y
        )))
        .await
    {
        RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok()
    } else {
        None
    };

    let new_terrain_mesh = if use_new_terrain {
        load_context
            .read_asset_bytes(zone_path.join(format!("block_{}_{}.mesh.bin", block_x, block_y)))
            .await
            .ok()
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

/// WORKAROUND: Load block files directly from VFS without using Bevy's LoadContext
///
/// IMPORTANT: Real filesystem takes priority over VFS to support map editor modifications.
/// Files are checked at base_path first, then VFS is used as fallback.
async fn load_block_files_direct(
    vfs: &VirtualFilesystem,
    base_path: &Path,
    zone_path: &Path,
    block_x: usize,
    block_y: usize,
    use_new_terrain: bool,
) -> Result<Box<ZoneLoaderBlock>, anyhow::Error> {
    /// Helper function to read raw bytes with real filesystem priority
    /// Returns the raw file data either from real filesystem or VFS
    fn read_bytes_with_priority(
        vfs: &VirtualFilesystem,
        base_path: &Path,
        vfs_path: &VfsPath,
    ) -> Result<Vec<u8>, anyhow::Error> {
        use rose_file_readers::VfsFile;

        let path_str = vfs_path.path().to_string_lossy().replace('\\', "/");

        // PRIORITY: Check real filesystem first
        let real_filesystem_path = base_path.join(&path_str);
        if real_filesystem_path.exists() {
            match std::fs::read(&real_filesystem_path) {
                Ok(data) => {
                    log::info!(
                        "[VFS PRIORITY] Loaded from real filesystem: {} ({} bytes)",
                        path_str,
                        memory_monitor::format_bytes(data.len() as u64)
                    );
                    return Ok(data);
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
                Ok(data)
            }
            Err(e) => Err(anyhow::anyhow!(
                "Failed to open VFS file {}: {:?}",
                path_str,
                e
            )),
        }
    }

    let him_path_buf = zone_path.join(format!("{}_{}.HIM", block_x, block_y));
    let him_path_str = him_path_buf.to_string_lossy().replace('\\', "/");
    let him_path = VfsPath::from(PathBuf::from(&him_path_str));

    // Check if HIM file exists before attempting to load it
    match vfs.open_file(&him_path) {
        Ok(_) => {}
        Err(_) => {
            return Err(anyhow::anyhow!(
                "HIM file not found for block {}_{}",
                block_x,
                block_y
            ));
        }
    }

    // Load and parse HIM file
    let him: HimFile = match read_bytes_with_priority(vfs, base_path, &him_path) {
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
            log::warn!("[LOAD BLOCK DIRECT] Failed to load HIM file for block {}_{}: {:?}. Skipping this block.", block_x, block_y, e);
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
    }))
}
