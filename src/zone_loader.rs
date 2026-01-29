use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock, mpsc},
    time::{Duration, Instant},
    collections::HashSet,
};
use bevy::ecs::system::Query;
use uuid::Uuid;

use anyhow::Result;
use arrayvec::ArrayVec;
use bevy::{
    asset::{Asset, AssetLoader, Assets, BoxedFuture, io::Reader, LoadContext, LoadState},
    ecs::system::SystemParam,
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    math::{Quat, Vec2, Vec3},
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::{
        AssetServer, Commands, Entity, EventReader, EventWriter, GlobalTransform, Handle,
        Local, Res, ResMut, Resource, Transform, UntypedHandle, Visibility,
    },
    reflect::TypePath,
    render::{
        mesh::{Indices, Mesh, PrimitiveTopology},
        render_asset::RenderAssetUsages,
        texture::Image,
        view::{NoFrustumCulling, ViewVisibility, InheritedVisibility},
    },
    tasks::{futures_lite::AsyncReadExt, AsyncComputeTaskPool, IoTaskPool},
};
use bevy_rapier3d::prelude::{
    AsyncCollider, Collider, CollisionGroups, ComputedColliderShape, RigidBody,
};
use log::{warn, info};
use bevy::log::info_span;
use thiserror::Error;

use rose_data::{NpcId, SkyboxData, WarpGateId, ZoneId, ZoneList};
use rose_file_readers::{
    HimFile, IfoEffectObject, IfoFile, IfoObject, IfoSoundObject, LitFile, LitObject, RoseFile,
    RoseFileReader, StbFile, TilFile, VfsPath, VirtualFilesystem, ZonFile, ZonTileRotation,
    ZscCollisionFlags, ZscEffectType, ZscFile,
};

use crate::{
    animation::{MeshAnimation, TransformAnimation, ZmoTextureAssetLoader},
    audio::{SoundRadius, SpatialSound},
    components::{
        ColliderParent, EventObject, NightTimeEffect, WarpObject, Zone, ZoneObject,
        ZoneObjectAnimatedObject, ZoneObjectId, ZoneObjectPart, ZoneObjectTerrain,
        COLLISION_FILTER_CLICKABLE, COLLISION_FILTER_COLLIDABLE, COLLISION_FILTER_INSPECTABLE,
        COLLISION_FILTER_MOVEABLE, COLLISION_GROUP_PHYSICS_TOY, COLLISION_GROUP_ZONE_EVENT_OBJECT,
        COLLISION_GROUP_ZONE_OBJECT, COLLISION_GROUP_ZONE_TERRAIN,
        COLLISION_GROUP_ZONE_WARP_OBJECT, COLLISION_GROUP_ZONE_WATER,
    },
    effect_loader::{decode_blend_factor, decode_blend_op, spawn_effect},
    events::{LoadZoneEvent, ZoneEvent, ZoneLoadedFromVfsEvent},
    render::{
        EffectMeshAnimationRenderState, EffectMeshMaterial, ObjectMaterial, ParticleMaterial,
        SkyMaterial, TerrainMaterial, WaterMaterial, MESH_ATTRIBUTE_UV_1,
        TERRAIN_MATERIAL_MAX_TEXTURES, TERRAIN_MESH_ATTRIBUTE_TILE_INFO,
    },
    resources::{CurrentZone, DebugInspector, GameData, SpecularTexture},
    VfsResource,
};

#[derive(Error, Debug)]
pub enum ZoneLoadError {
    #[error("Invalid Zone Id")]
    InvalidZoneId,
}

pub struct ZoneLoaderBlock {
    pub block_x: usize,
    pub block_y: usize,
    pub him: HimFile,
    pub til: Option<TilFile>,
    pub ifo: Option<IfoFile>,
    pub lit_cnst: Option<LitFile>,
    pub lit_deco: Option<LitFile>,
}

pub struct ZoneNpc {
    pub position: Vec3,
    pub npc_id: NpcId,
}

#[derive(Asset, TypePath)]
pub struct ZoneLoaderAsset {
    pub zone_id: ZoneId,
    pub zone_path: PathBuf,
    pub zon: ZonFile,
    pub zsc_cnst: ZscFile,
    pub zsc_deco: ZscFile,
    pub blocks: Vec<Option<Box<ZoneLoaderBlock>>>,
    pub npcs: Vec<ZoneNpc>,
}

/// Channel sender for sending loaded zone data from async tasks
#[derive(Resource)]
pub struct ZoneLoadChannelSender(pub mpsc::Sender<(ZoneId, Result<ZoneLoaderAsset, anyhow::Error>)>);

/// Channel receiver for receiving loaded zone data from async tasks
#[derive(Resource)]
pub struct ZoneLoadChannelReceiver(pub std::sync::Mutex<mpsc::Receiver<(ZoneId, Result<ZoneLoaderAsset, anyhow::Error>)>>);

/// Resource for tracking memory and asset lifecycle
#[derive(Resource, Default)]
pub struct MemoryTrackingResource {
    /// Count of mesh handles created
    pub mesh_handles_created: usize,
    /// Count of material handles created
    pub material_handles_created: usize,
    /// Count of texture handles created
    pub texture_handles_created: usize,
    /// Set of unique asset paths loaded
    pub unique_asset_paths: HashSet<String>,
    /// Count of duplicate asset requests
    pub duplicate_asset_requests: usize,
    /// Total entities spawned
    pub entities_spawned: usize,
    /// Total entities despawned
    pub entities_despawned: usize,
    /// Last summary log time
    pub last_summary_time: Option<Instant>,
}

impl MemoryTrackingResource {
    /// Log when a mesh handle is created
    pub fn log_mesh_handle_created(&mut self, path: &str) {
        self.mesh_handles_created += 1;
        let is_duplicate = !self.unique_asset_paths.insert(path.to_string());
        if is_duplicate {
            self.duplicate_asset_requests += 1;
            info!("[MEMORY TRACKING] Mesh handle REUSE detected: {} (total duplicates: {})", 
                path, self.duplicate_asset_requests);
        } else {
            info!("[MEMORY TRACKING] Mesh handle created: {} (total meshes: {})", 
                path, self.mesh_handles_created);
        }
    }

    /// Log when a material handle is created
    pub fn log_material_handle_created(&mut self, path: &str, texture_count: usize) {
        self.material_handles_created += 1;
        info!("[MEMORY TRACKING] Material handle created: {} with {} textures (total materials: {})", 
            path, texture_count, self.material_handles_created);
    }

    /// Log when a texture handle is created
    pub fn log_texture_handle_created(&mut self, path: &str) {
        self.texture_handles_created += 1;
        let is_duplicate = !self.unique_asset_paths.insert(path.to_string());
        if is_duplicate {
            self.duplicate_asset_requests += 1;
            info!("[MEMORY TRACKING] Texture handle REUSE detected: {} (total duplicates: {})", 
                path, self.duplicate_asset_requests);
        } else {
            info!("[MEMORY TRACKING] Texture handle created: {} (total textures: {})", 
                path, self.texture_handles_created);
        }
    }

    /// Log when an entity is spawned
    pub fn log_entity_spawned(&mut self, entity_type: &str, asset_count: usize) {
        self.entities_spawned += 1;
        info!("[MEMORY TRACKING] Entity spawned: type={}, assets={} (total entities: {})", 
            entity_type, asset_count, self.entities_spawned);
    }

    /// Log when an entity is despawned
    pub fn log_entity_despawned(&mut self) {
        self.entities_despawned += 1;
        info!("[MEMORY TRACKING] Entity despawned (total despawned: {})", self.entities_despawned);
    }

    /// Log a summary of memory statistics
    pub fn log_summary(&mut self) {
        let now = Instant::now();
        let should_log = self.last_summary_time
            .map_or(true, |last| now.duration_since(last) >= Duration::from_secs(5));
        
        if should_log {
            self.last_summary_time = Some(now);
            info!("[MEMORY TRACKING] ==========================================");
            info!("[MEMORY TRACKING] MEMORY SUMMARY (every 5 seconds)");
            info!("[MEMORY TRACKING] ==========================================");
            info!("[MEMORY TRACKING] Mesh handles: {}", self.mesh_handles_created);
            info!("[MEMORY TRACKING] Material handles: {}", self.material_handles_created);
            info!("[MEMORY TRACKING] Texture handles: {}", self.texture_handles_created);
            info!("[MEMORY TRACKING] Unique asset paths: {}", self.unique_asset_paths.len());
            info!("[MEMORY TRACKING] Duplicate asset requests: {}", self.duplicate_asset_requests);
            info!("[MEMORY TRACKING] Entities spawned: {}", self.entities_spawned);
            info!("[MEMORY TRACKING] Entities despawned: {}", self.entities_despawned);
            info!("[MEMORY TRACKING] Active entities: {}", self.entities_spawned - self.entities_despawned);
            
            // Warning if counts are growing without despawns
            if self.entities_spawned > 0 && self.entities_despawned == 0 {
                warn!("[MEMORY TRACKING] WARNING: {} entities spawned but 0 despawned - potential leak!", 
                    self.entities_spawned);
            }
            
            // Warning if many duplicate requests
            if self.duplicate_asset_requests > 100 {
                warn!("[MEMORY TRACKING] WARNING: {} duplicate asset requests detected - may indicate inefficient loading", 
                    self.duplicate_asset_requests);
            }
            
            info!("[MEMORY TRACKING] ==========================================");
        }
    }
}


impl ZoneLoaderAsset {
    pub fn get_terrain_height(&self, x: f32, y: f32) -> f32 {
        let block_x = x / (16.0 * self.zon.grid_per_patch * self.zon.grid_size);
        let block_y = 65.0 - (y / (16.0 * self.zon.grid_per_patch * self.zon.grid_size));

        if let Some(heightmap) = self
            .blocks
            .get(block_x.clamp(0.0, 64.0) as usize + block_y.clamp(0.0, 64.0) as usize * 64)
            .and_then(|block| block.as_ref())
            .map(|block| &block.him)
        {
            let tile_x = (heightmap.width - 1) as f32 * block_x.fract();
            let tile_y = (heightmap.height - 1) as f32 * block_y.fract();

            let tile_index_x = tile_x as i32;
            let tile_index_y = tile_y as i32;

            let height_00 = heightmap.get_clamped(tile_index_x, tile_index_y);
            let height_01 = heightmap.get_clamped(tile_index_x, tile_index_y + 1);
            let height_10 = heightmap.get_clamped(tile_index_x + 1, tile_index_y);
            let height_11 = heightmap.get_clamped(tile_index_x + 1, tile_index_y + 1);

            let weight_x = tile_x.fract();
            let weight_y = tile_y.fract();

            let height_y0 = height_00 * (1.0 - weight_x) + height_10 * weight_x;
            let height_y1 = height_01 * (1.0 - weight_x) + height_11 * weight_x;

            height_y0 * (1.0 - weight_y) + height_y1 * weight_y
        } else {
            0.0
        }
    }

    pub fn get_tile_index(&self, x: f32, y: f32) -> usize {
        let block_x = x / (16.0 * self.zon.grid_per_patch * self.zon.grid_size);
        let block_y = 65.0 - (y / (16.0 * self.zon.grid_per_patch * self.zon.grid_size));

        if let Some(tilemap) = self
            .blocks
            .get(block_x.clamp(0.0, 64.0) as usize + block_y.clamp(0.0, 64.0) as usize * 64)
            .and_then(|block| block.as_ref())
            .and_then(|block| block.til.as_ref())
        {
            let tile_x = tilemap.width as f32 * block_x.fract();
            let tile_y = tilemap.height as f32 * block_y.fract();

            let tile_index_x = tile_x as usize;
            let tile_index_y = tile_y as usize;

            let tile_index = tilemap.get_clamped(tile_index_x, tile_index_y) as usize;

            if let Some(tile_info) = self.zon.tiles.get(tile_index) {
                (tile_info.layer2 + tile_info.offset2) as usize
            } else {
                0
            }
        } else {
            0
        }
    }
}

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

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext<'_>,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            log::info!("[ZONE LOADER ASSET LOADER] ===========================================");
            log::info!("[ZONE LOADER ASSET LOADER] ZoneLoader::load called");
            log::info!("[ZONE LOADER ASSET LOADER] ===========================================");

            let mut bytes = Vec::new();
            log::info!("[ZONE LOADER ASSET LOADER] Reading bytes from reader...");
            reader.read_to_end(&mut bytes).await?;
            log::info!("[ZONE LOADER ASSET LOADER] Read {} bytes", bytes.len());

            let zone_id = ZoneId::new(bytes[0] as u16).unwrap();
            log::info!("[ZONE LOADER ASSET LOADER] Zone ID parsed: {}", zone_id.get());

            log::info!("[ZONE LOADER ASSET LOADER] Calling load_zone...");
            let result = load_zone(zone_id, load_context).await;
            log::info!("[ZONE LOADER ASSET LOADER] load_zone completed with result: {:?}", result.is_ok());
            result
        })
    }

    fn extensions(&self) -> &[&str] {
        &["zone_loader"]
    }
}

async fn load_zone<'a, 'b>(
    zone_id: ZoneId,
    load_context: &'a mut LoadContext<'b>,
) -> Result<ZoneLoaderAsset, anyhow::Error> {
    log::info!("[ZONE LOADER DIAGNOSTIC] ===========================================");
    log::info!("[ZONE LOADER DIAGNOSTIC] load_zone called for zone_id: {}", zone_id.get());
    log::info!("[ZONE LOADER DIAGNOSTIC] ===========================================");

    let zone_list = ZoneLoader::get_zone_list();
    let zone_list_entry = zone_list
        .get_zone(zone_id)
        .ok_or(ZoneLoadError::InvalidZoneId)?;
    let zon_file_path = zone_list_entry.zon_file_path.path().to_path_buf();
    let zsc_cnst_path = zone_list_entry.zsc_cnst_path.path().to_path_buf();
    let zsc_deco_path = zone_list_entry.zsc_deco_path.path().to_path_buf();

    log::info!("[ZONE LOADER DIAGNOSTIC] Loading ZON file: {:?}", zon_file_path);
    let zon: ZonFile = RoseFile::read(
        RoseFileReader::from(
            &(*load_context)
                .read_asset_bytes(zon_file_path.clone())
                .await?
        ),
        &Default::default(),
    )?;
    log::info!("[ZONE LOADER DIAGNOSTIC] ZON file loaded successfully");

    log::info!("[ZONE LOADER DIAGNOSTIC] Loading ZSC constant file: {:?}", zsc_cnst_path);
    let zsc_cnst: ZscFile = RoseFile::read(
        RoseFileReader::from(
            &(*load_context)
                .read_asset_bytes(zsc_cnst_path.clone())
                .await?,
        ),
        &Default::default(),
    )?;
    log::info!("[ZONE LOADER DIAGNOSTIC] ZSC constant file loaded successfully");

    log::info!("[ZONE LOADER DIAGNOSTIC] Loading ZSC deco file: {:?}", zsc_deco_path);
    let zsc_deco: ZscFile = RoseFile::read(
        RoseFileReader::from(
            &(*load_context)
                .read_asset_bytes(zsc_deco_path.clone())
                .await?,
        ),
        &Default::default(),
    )?;
    log::info!("[ZONE LOADER DIAGNOSTIC] ZSC deco file loaded successfully");

    let zone_path = zon_file_path
        .parent()
        .unwrap_or_else(|| Path::new(""));

    log::info!("[ZONE LOADER DIAGNOSTIC] ===========================================");
    log::info!("[ZONE LOADER DIAGNOSTIC] Starting to load zone blocks (64x64 = 4096 blocks)");
    log::info!("[ZONE LOADER DIAGNOSTIC] Zone path: {:?}", zone_path);
    log::info!("[ZONE LOADER DIAGNOSTIC] ===========================================");

    let mut zone_blocks = Vec::new();
    let mut blocks_loaded = 0;
    let mut blocks_failed = 0;

    for block_y in 0..64 {
        for block_x in 0..64 {
            if let Ok(block) = load_block_files(load_context, zone_path, block_x, block_y).await {
                zone_blocks.push(block);
                blocks_loaded += 1;
            } else {
                blocks_failed += 1;
            }

            // Log progress every 100 blocks
            if (block_x + block_y * 64) % 100 == 0 {
                log::info!("[ZONE LOADER DIAGNOSTIC] Block loading progress: {} loaded, {} failed", blocks_loaded, blocks_failed);
            }
        }
    }

    log::info!("[ZONE LOADER DIAGNOSTIC] ===========================================");
    log::info!("[ZONE LOADER DIAGNOSTIC] Block loading complete: {} loaded, {} failed", blocks_loaded, blocks_failed);
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
async fn load_zone_direct(zone_id: ZoneId, vfs: &VirtualFilesystem) -> Result<ZoneLoaderAsset, anyhow::Error> {
    log::info!("[ZONE LOADER DIRECT] ===========================================");
    log::info!("[ZONE LOADER DIRECT] load_zone_direct called for zone_id: {}", zone_id.get());
    log::info!("[ZONE LOADER DIRECT] ===========================================");

    let zone_list = ZoneLoader::get_zone_list();
    let zone_list_entry = zone_list
        .get_zone(zone_id)
        .ok_or(ZoneLoadError::InvalidZoneId)?;
    let zon_file_path_buf = zone_list_entry.zon_file_path.path().to_path_buf();
    let zon_file_path = VfsPath::from(zon_file_path_buf.clone());
    let zsc_cnst_path = VfsPath::from(zone_list_entry.zsc_cnst_path.path().to_path_buf());
    let zsc_deco_path = VfsPath::from(zone_list_entry.zsc_deco_path.path().to_path_buf());

    log::info!("[ZONE LOADER DIRECT] Loading ZON file: {:?}", zon_file_path);
    let zon: ZonFile = vfs
        .read_file(&zon_file_path)
        .map_err(|e| anyhow::anyhow!("Failed to load ZON file: {:?}", e))?;
    log::info!("[ZONE LOADER DIRECT] ZON file loaded successfully");

    log::info!("[ZONE LOADER DIRECT] Loading ZSC constant file: {:?}", zsc_cnst_path);
    let zsc_cnst: ZscFile = vfs
        .read_file(&zsc_cnst_path)
        .map_err(|e| anyhow::anyhow!("Failed to load ZSC constant file: {:?}", e))?;
    log::info!("[ZONE LOADER DIRECT] ZSC constant file loaded successfully");

    log::info!("[ZONE LOADER DIRECT] Loading ZSC deco file: {:?}", zsc_deco_path);
    let zsc_deco: ZscFile = vfs
        .read_file(&zsc_deco_path)
        .map_err(|e| anyhow::anyhow!("Failed to load ZSC deco file: {:?}", e))?;
    log::info!("[ZONE LOADER DIRECT] ZSC deco file loaded successfully");

    let zone_path = zon_file_path_buf
        .parent()
        .unwrap_or_else(|| Path::new(""));

    log::info!("[ZONE LOADER DIRECT] ===========================================");
    log::info!("[ZONE LOADER DIRECT] Starting to load zone blocks (64x64 = 4096 blocks)");
    log::info!("[ZONE LOADER DIRECT] Zone path: {:?}", zone_path);
    log::info!("[ZONE LOADER DIRECT] Note: Blocks without HIM files will be skipped");
    log::info!("[ZONE LOADER DIRECT] ===========================================");

    let mut zone_blocks = Vec::new();
    let mut blocks_loaded = 0;
    let mut blocks_skipped = 0;
    let mut skipped_blocks = Vec::new();

    for block_y in 0..64 {
        for block_x in 0..64 {
            match load_block_files_direct(vfs, zone_path, block_x, block_y).await {
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
                    log::debug!("[ZONE LOADER DIRECT] Block {}_{} skipped: {}", block_x, block_y, e);
                }
            }

            // Log progress every 100 blocks
            if (block_x + block_y * 64) % 100 == 0 {
                log::info!("[ZONE LOADER DIRECT] Block loading progress: {} loaded, {} skipped", blocks_loaded, blocks_skipped);
            }
        }
    }

    log::info!("[ZONE LOADER DIRECT] ===========================================");
    log::info!("[ZONE LOADER DIRECT] Block loading complete: {} loaded, {} skipped", blocks_loaded, blocks_skipped);
    if !skipped_blocks.is_empty() {
        log::info!("[ZONE LOADER DIRECT] Skipped blocks (first 10):");
        for (block_x, block_y, error) in skipped_blocks.iter().take(10) {
            log::info!("[ZONE LOADER DIRECT]   Block {}_{}: {}", block_x, block_y, error);
        }
        if skipped_blocks.len() > 10 {
            log::info!("[ZONE LOADER DIRECT]   ... and {} more skipped blocks", skipped_blocks.len() - 10);
        }
        log::info!("[ZONE LOADER DIRECT] Zone will spawn with {} blocks", blocks_loaded);
    }
    log::info!("[ZONE LOADER DIRECT] ===========================================");

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
) -> Result<Box<ZoneLoaderBlock>, anyhow::Error> {
    let him_path = zone_path.join(format!("{}_{}.HIM", block_x, block_y));
    log::trace!("[LOAD BLOCK] Loading block {}_{} from: {:?}", block_x, block_y, him_path);

    let him = RoseFile::read(
        RoseFileReader::from(
            &load_context
                .read_asset_bytes(him_path.clone())
                .await?,
        ),
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

    Ok(Box::new(ZoneLoaderBlock {
        block_x,
        block_y,
        til,
        him,
        ifo,
        lit_cnst,
        lit_deco,
    }))
}

/// WORKAROUND: Load block files directly from VFS without using Bevy's LoadContext
async fn load_block_files_direct(
    vfs: &VirtualFilesystem,
    zone_path: &Path,
    block_x: usize,
    block_y: usize,
) -> Result<Box<ZoneLoaderBlock>, anyhow::Error> {
    let him_path = VfsPath::from(zone_path.join(format!("{}_{}.HIM", block_x, block_y)));
    log::trace!("[LOAD BLOCK DIRECT] Loading block {}_{} from: {:?}", block_x, block_y, him_path);

    // Check if HIM file exists before attempting to load it
    match vfs.open_file(&him_path) {
        Ok(_) => {
            log::trace!("[LOAD BLOCK DIRECT] HIM file exists for block {}_{}", block_x, block_y);
        }
        Err(_) => {
            log::debug!("[LOAD BLOCK DIRECT] HIM file does not exist for block {}_{} - skipping this block", block_x, block_y);
            return Err(anyhow::anyhow!("HIM file not found for block {}_{}", block_x, block_y));
        }
    }

    let him = match vfs.read_file(&him_path) {
        Ok(data) => {
            log::trace!("[LOAD BLOCK DIRECT] Successfully loaded HIM file for block {}_{}", block_x, block_y);
            data
        }
        Err(e) => {
            log::warn!("[LOAD BLOCK DIRECT] Failed to load HIM file for block {}_{}: {:?}. Skipping this block.", block_x, block_y, e);
            return Err(anyhow::anyhow!("HIM file not found for block {}_{}", block_x, block_y));
        }
    };

    let til_path = VfsPath::from(zone_path.join(format!("{}_{}.TIL", block_x, block_y)));
    let til = match vfs.read_file(&til_path) {
        Ok(data) => {
            log::trace!("[LOAD BLOCK DIRECT] Successfully loaded TIL file for block {}_{}", block_x, block_y);
            Some(data)
        }
        Err(e) => {
            log::trace!("[LOAD BLOCK DIRECT] TIL file not found for block {}_{}: {:?}. This is optional.", block_x, block_y, e);
            None
        }
    };

    let ifo_path = VfsPath::from(zone_path.join(format!("{}_{}.IFO", block_x, block_y)));
    let ifo = match vfs.read_file(&ifo_path) {
        Ok(data) => {
            log::trace!("[LOAD BLOCK DIRECT] Successfully loaded IFO file for block {}_{}", block_x, block_y);
            Some(data)
        }
        Err(e) => {
            log::trace!("[LOAD BLOCK DIRECT] IFO file not found for block {}_{}: {:?}. This is optional.", block_x, block_y, e);
            None
        }
    };

    let lit_cnst_path = VfsPath::from(zone_path.join(format!(
        "{}_{}/LIGHTMAP/BUILDINGLIGHTMAPDATA.LIT",
        block_x, block_y
    )));
    let lit_cnst = match vfs.read_file(&lit_cnst_path) {
        Ok(data) => {
            log::trace!("[LOAD BLOCK DIRECT] Successfully loaded LIT constant file for block {}_{}", block_x, block_y);
            Some(data)
        }
        Err(e) => {
            log::trace!("[LOAD BLOCK DIRECT] LIT constant file not found for block {}_{}: {:?}. This is optional.", block_x, block_y, e);
            None
        }
    };

    let lit_deco_path = VfsPath::from(zone_path.join(format!(
        "{}_{}/LIGHTMAP/OBJECTLIGHTMAPDATA.LIT",
        block_x, block_y
    )));
    let lit_deco = match vfs.read_file(&lit_deco_path) {
        Ok(data) => {
            log::trace!("[LOAD BLOCK DIRECT] Successfully loaded LIT deco file for block {}_{}", block_x, block_y);
            Some(data)
        }
        Err(e) => {
            log::trace!("[LOAD BLOCK DIRECT] LIT deco file not found for block {}_{}: {:?}. This is optional.", block_x, block_y, e);
            None
        }
    };

    log::info!("[LOAD BLOCK DIRECT] Successfully loaded block {}_{} (HIM: yes, TIL: {}, IFO: {}, LIT_CNST: {}, LIT_DECO: {})",
        block_x, block_y,
        til.is_some(),
        ifo.is_some(),
        lit_cnst.is_some(),
        lit_deco.is_some()
    );

    Ok(Box::new(ZoneLoaderBlock {
        block_x,
        block_y,
        til,
        him,
        ifo,
        lit_cnst,
        lit_deco,
    }))
}

#[derive(SystemParam)]
pub struct SpawnZoneParams<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub asset_server: Res<'w, AssetServer>,
    pub game_data: Res<'w, GameData>,
    pub vfs_resource: Res<'w, VfsResource>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub specular_texture: Res<'w, SpecularTexture>,
    pub sky_materials: ResMut<'w, Assets<SkyMaterial>>,
    pub terrain_materials: ResMut<'w, Assets<TerrainMaterial>>,
    pub effect_mesh_materials: ResMut<'w, Assets<EffectMeshMaterial>>,
    pub particle_materials: ResMut<'w, Assets<ParticleMaterial>>,
    pub object_materials: ResMut<'w, Assets<ObjectMaterial>>,
    pub water_materials: ResMut<'w, Assets<WaterMaterial>>,
    pub zone_loader_assets: ResMut<'w, Assets<ZoneLoaderAsset>>,
    pub memory_tracking: ResMut<'w, MemoryTrackingResource>,
}

pub struct CachedZone {
    pub data_handle: Handle<ZoneLoaderAsset>,
    pub spawned_entity: Option<Entity>,
}

pub enum LoadingZoneState {
    Loading,
    Spawned,
}

pub struct LoadingZone {
    pub state: LoadingZoneState,
    pub handle: Handle<ZoneLoaderAsset>,
    pub despawn_other_zones: bool,
    /// Zone assets that are loading - CRITICAL: Must be cleared after loading to prevent memory leak
    pub zone_assets: Vec<UntypedHandle>,
    pub ready_frames: usize,
    pub loading_via_async_task: bool,  // Track if loading via async task vs AssetServer
    pub zone_id: Option<ZoneId>,  // Track zone_id for async-loaded zones
    pub loading_start_time: Instant,  // Track when loading started
    /// Track if assets have been cleared to prevent duplicate cleanup
    pub assets_cleared: bool,
}

impl LoadingZone {
    /// Clear asset handles to prevent memory leak
    /// Call this once zone is fully loaded
    pub fn clear_asset_handles(&mut self) {
        if !self.assets_cleared {
            let count = self.zone_assets.len();
            if count > 0 {
                log::info!("[MEMORY FIX] Clearing {} zone asset handles to prevent memory leak", count);
                self.zone_assets.clear();
                self.zone_assets.shrink_to_fit();
                self.assets_cleared = true;
            }
        }
    }
    
    /// Check if all zone assets are fully loaded
    pub fn are_assets_loaded(&self, asset_server: &AssetServer) -> bool {
        if self.zone_assets.is_empty() {
            return true;
        }
        
        use bevy::asset::LoadState;
        let all_loaded = self.zone_assets.iter().all(|handle| {
            matches!(asset_server.get_load_state(handle.clone()), Some(LoadState::Loaded))
        });
        
        if !all_loaded {
            let loaded_count = self.zone_assets.iter()
                .filter(|h| matches!(asset_server.get_load_state((*h).clone()), Some(LoadState::Loaded)))
                .count();
            log::debug!("[ASSET LOADING] {}/{} zone assets loaded", loaded_count, self.zone_assets.len());
        }
        
        all_loaded
    }
}

#[derive(Default)]
pub struct ZoneLoaderCache {
    pub cache: Vec<Option<CachedZone>>,
}

pub fn zone_loader_system(
    mut zone_loader_cache: Local<ZoneLoaderCache>,
    mut loading_zones: Local<Vec<LoadingZone>>,
    mut load_zone_events: EventReader<LoadZoneEvent>,
    mut zone_events: EventWriter<ZoneEvent>,
    mut zone_loaded_from_vfs_events: EventWriter<ZoneLoadedFromVfsEvent>,
    mut zone_load_receiver: ResMut<ZoneLoadChannelReceiver>,
    zone_load_sender: Res<ZoneLoadChannelSender>,
    mut spawn_zone_params: SpawnZoneParams,
    mut debug_inspector_state: ResMut<DebugInspector>,
) {
    let _span = info_span!("zone_loader_system").entered();
    let has_load_events = load_zone_events.len() > 0;
    let has_loading_zones = !loading_zones.is_empty();

    // Early return if no zones are loading and no load events to process
    // This prevents unnecessary memory allocations from logging every frame
    if !has_load_events && !has_loading_zones {
        return;
    }

    log::info!("[ZONE LOADER SYSTEM] ===========================================");
    log::info!("[ZONE LOADER SYSTEM] zone_loader_system called");
    log::info!("[ZONE LOADER SYSTEM] Loading zones in queue: {}", loading_zones.len());
    log::info!("[ZONE LOADER SYSTEM] ===========================================");

    // Log periodic memory summary
    spawn_zone_params.memory_tracking.log_summary();

    // Check for loaded zones from async tasks via channel
    log::info!("[ZONE LOADER SYSTEM] Checking channel for loaded zones...");
    let mut received_count = 0;
    while let Ok((zone_id, zone_asset_result)) = zone_load_receiver.0.lock().unwrap().try_recv() {
        received_count += 1;
        log::info!("[ZONE LOADER SYSTEM] Received {} zone(s) from channel this frame", received_count);
        let zone_id: ZoneId = zone_id;
        let zone_asset_result: Result<ZoneLoaderAsset, anyhow::Error> = zone_asset_result;
        match zone_asset_result {
            Ok(zone_asset) => {
                log::info!("[ZONE LOADER SYSTEM] ===========================================");
                log::info!("[ZONE LOADER SYSTEM] Zone {} loaded from async task, sending ZoneLoadedFromVfsEvent", zone_id.get());
                log::info!("[ZONE LOADER SYSTEM] ===========================================");

                // Remove the zone from the loading queue since it's now received from channel
                if let Some(pos) = loading_zones.iter().position(|lz| {
                    lz.loading_via_async_task && lz.zone_id == Some(zone_id)
                }) {
                    log::info!("[ZONE LOADER SYSTEM] Removing zone {} from loading queue (received from channel)", zone_id.get());
                    loading_zones.remove(pos);
                } else {
                    log::warn!("[ZONE LOADER SYSTEM] Could not find zone {} in loading queue to remove", zone_id.get());
                }

                // Send event to the new zone_loaded_from_vfs_system for spawning
                zone_loaded_from_vfs_events.send(ZoneLoadedFromVfsEvent {
                    zone_id,
                    zone_asset,
                });
            }
            Err(e) => {
                log::error!("[ZONE LOADER SYSTEM] Failed to load zone {} from async task: {:?}", zone_id.get(), e);
                // Remove the failed loading zone from cache and loading queue
                let zone_index = zone_id.get() as usize;
                zone_loader_cache.cache[zone_index] = None;
                
                // Remove from loading queue
                if let Some(pos) = loading_zones.iter().position(|lz| {
                    lz.loading_via_async_task && lz.zone_id == Some(zone_id)
                }) {
                    log::info!("[ZONE LOADER SYSTEM] Removing failed zone {} from loading queue", zone_id.get());
                    loading_zones.remove(pos);
                }
            }
        }
    }
    
    if received_count == 0 {
        log::info!("[ZONE LOADER SYSTEM] No zones received from channel this frame");
    }

    if zone_loader_cache.cache.is_empty() {
        zone_loader_cache
            .cache
            .resize_with(spawn_zone_params.game_data.zone_list.len(), || None);
    }

    for event in load_zone_events.read() {
        let zone_index = event.id.get() as usize;

        // Memory tracking: Log cache state
        let cached_zones = zone_loader_cache.cache.iter().filter(|z| z.is_some()).count();
        let spawned_zones = zone_loader_cache.cache.iter().filter(|z| z.is_some() && z.as_ref().unwrap().spawned_entity.is_some()).count();
        log::info!("[MEMORY] Cache state: {} zones cached, {} spawned", cached_zones, spawned_zones);

        log::info!("[ZONE LOADER SYSTEM] ===========================================");
        log::info!("[ZONE LOADER SYSTEM] LoadZoneEvent received for zone_id: {}", event.id.get());
        log::info!("[ZONE LOADER SYSTEM] Despawn other zones: {}", event.despawn_other_zones);
        log::info!("[ZONE LOADER SYSTEM] ===========================================");

        // CRITICAL FIX: Check for duplicate zone loading to prevent memory leaks
        // and double-spawning of the same zone
        let is_already_loading = loading_zones.iter().any(|lz| lz.zone_id == Some(event.id));
        let is_already_loaded = zone_loader_cache.cache.get(zone_index)
            .map(|c| c.as_ref().map(|cz| cz.spawned_entity.is_some()).unwrap_or(false))
            .unwrap_or(false);

        if is_already_loading {
            log::warn!("[ZONE LOADER SYSTEM] Zone {} is already loading via async task, skipping duplicate request",
                event.id.get());
            continue;
        }

        if is_already_loaded {
            log::warn!("[ZONE LOADER SYSTEM] Zone {} is already loaded. Consider despawning old instance first.",
                event.id.get());
            // Optionally: Despawn the old zone here if event.despawn_other_zones is true
            if event.despawn_other_zones {
                if let Some(Some(cached)) = zone_loader_cache.cache.get(zone_index) {
                    if let Some(entity) = cached.spawned_entity {
                        log::info!("[ZONE LOADER SYSTEM] Despawning existing zone {} entity {:?} as requested",
                            event.id.get(), entity);
                        spawn_zone_params.commands.entity(entity).despawn_recursive();
                    }
                }
                zone_loader_cache.cache[zone_index] = None;
            } else {
                // Skip if already loaded and not despawning
                continue;
            }
        }

        if zone_loader_cache.cache.get(zone_index).map(|c| c.is_none()).unwrap_or(true) {
            log::info!("[ZONE LOADER SYSTEM] Zone not cached, loading directly from VFS");
            
            // WORKAROUND: Load zone directly from VFS without using AssetServer
            // This bypasses the broken asset loading pipeline in Bevy 0.13.2
            let zone_id = event.id;
            let vfs = spawn_zone_params.vfs_resource.vfs.clone();
            let tx = zone_load_sender.0.clone();
            
            log::info!("[ZONE LOADER SYSTEM] ===========================================");
            log::info!("[ZONE LOADER SYSTEM] Preparing to spawn async task for zone {}", zone_id.get());
            
            // Check if pool is initialized and get reference
            let pool = match AsyncComputeTaskPool::try_get() {
                Some(pool) => {
                    log::info!("[ZONE LOADER SYSTEM] AsyncComputeTaskPool is available, spawning async task");
                    pool
                }
                None => {
                    log::error!("[ZONE LOADER SYSTEM] AsyncComputeTaskPool is NOT initialized! Cannot spawn async task!");
                    log::error!("[ZONE LOADER SYSTEM] This is likely why zones are not loading!");
                    // DO NOT spawn the task - skip this zone and continue to next
                    continue;
                }
            };

            log::info!("[ZONE LOADER SYSTEM] Spawning async task to load zone {}", zone_id.get());
            log::info!("[ZONE LOADER SYSTEM] ===========================================");

            // Spawn async task to load zone using AsyncComputeTaskPool
            // This is more appropriate for computational tasks like loading zones
            let task = pool.spawn(async move {
                log::info!("[ZONE LOADER DIRECT TASK] ===========================================");
                log::info!("[ZONE LOADER DIRECT TASK] Async task started for zone_id: {}", zone_id.get());
                log::info!("[ZONE LOADER DIRECT TASK] ===========================================");
                
                match load_zone_direct(zone_id, &vfs).await {
                    Ok(zone_asset) => {
                        log::info!("[ZONE LOADER DIRECT TASK] ===========================================");
                        log::info!("[ZONE LOADER DIRECT TASK] Zone loaded successfully: {}", zone_id.get());
                        log::info!("[ZONE LOADER DIRECT TASK] Sending zone through channel...");
                        log::info!("[ZONE LOADER DIRECT TASK] ===========================================");
                        
                        match tx.send((zone_id, Ok(zone_asset))) {
                            Ok(_) => {
                                log::info!("[ZONE LOADER DIRECT TASK] Zone sent through channel successfully!");
                            }
                            Err(e) => {
                                log::error!("[ZONE LOADER DIRECT TASK] Failed to send zone through channel: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("[ZONE LOADER DIRECT TASK] Failed to load zone {}: {:?}", zone_id.get(), e);
                        match tx.send((zone_id, Err(e))) {
                            Ok(_) => {
                                log::info!("[ZONE LOADER DIRECT TASK] Error sent through channel successfully!");
                            }
                            Err(send_err) => {
                                log::error!("[ZONE LOADER DIRECT TASK] Failed to send error through channel: {:?}", send_err);
                            }
                        }
                    }
                }
            });
            
            // Detach the task so it runs in the background
            task.detach();
            
            log::info!("[ZONE LOADER SYSTEM] Async task spawned and detached for zone {}", zone_id.get());
            log::info!("[ZONE LOADER SYSTEM] ===========================================");

            // Add zone to loading queue to track that it's being loaded
            // This ensures we know the zone is in progress even though spawning is handled by zone_loaded_from_vfs_system
            loading_zones.push(LoadingZone {
                state: LoadingZoneState::Loading,
                handle: Handle::<ZoneLoaderAsset>::default(),
                despawn_other_zones: event.despawn_other_zones,
                zone_assets: Vec::default(),
                ready_frames: 0,
                loading_via_async_task: true,
                zone_id: Some(zone_id),
                loading_start_time: Instant::now(),  // Initialize start time
                assets_cleared: false,
            });
            log::info!("[ZONE LOADER SYSTEM] Zone queued for async loading. Total loading zones: {}", loading_zones.len());
        } else if let Some(zone_entity) = zone_loader_cache.cache[zone_index]
            .as_ref()
            .and_then(|cached_zone| cached_zone.spawned_entity)
        {
            // Zone is already spawned
            log::info!("[ZONE LOADER SYSTEM] Zone already spawned, sending Loaded event");
            zone_events.send(ZoneEvent::Loaded(event.id));
            debug_inspector_state.entity = Some(zone_entity);
            continue;
        } else {
            log::info!("[ZONE LOADER SYSTEM] Zone cached but not spawned, using cached handle");
            
            let cached_zone = zone_loader_cache.cache[zone_index].as_ref().unwrap();
            loading_zones.push(LoadingZone {
                state: LoadingZoneState::Loading,
                handle: cached_zone.data_handle.clone(),
                despawn_other_zones: event.despawn_other_zones,
                zone_assets: Vec::default(),
                ready_frames: 0,
                loading_via_async_task: false,
                zone_id: None,
                loading_start_time: Instant::now(),  // Initialize start time
                assets_cleared: false,
            });
            log::info!("[ZONE LOADER SYSTEM] LoadingZone added to queue. Total loading zones: {}", loading_zones.len());
        }
    }

    let mut index = 0;
    while index < loading_zones.len() {
        let loading_zone = &mut loading_zones[index];

        match loading_zone.state {
            LoadingZoneState::Loading => {
                // Zones loaded via async task should stay in queue and wait for channel
                if loading_zone.loading_via_async_task {
                    let zone_path = loading_zone.handle.path().map(|p| p.to_string()).unwrap_or_else(|| "unknown".to_string());

                    // Check for timeout (30 seconds)
                    if loading_zone.loading_start_time.elapsed() > Duration::from_secs(30) {
                        log::error!("[ZONE LOADER SYSTEM] Zone {} loading timeout after 30s, removing from queue", zone_path);
                        
                        // MEMORY LEAK FIX: Clear asset handles before removing timed-out zone
                        loading_zone.clear_asset_handles();
                        
                        loading_zones.remove(index);
                        continue;
                    }

                    log::info!("[ZONE LOADER SYSTEM] Zone {} loading via async task, keeping in queue and waiting for channel",
                        zone_path);
                    index += 1;
                    continue;
                } else {
                    // Zone is loading via AssetServer - check LoadState
                    let zone_path = loading_zone.handle.path().map(|p| p.to_string()).unwrap_or_else(|| "unknown".to_string());
                    log::info!("[ZONE LOADER SYSTEM] Checking LoadState for zone {} (AssetServer)", zone_path);
                    
                    match spawn_zone_params.asset_server.get_load_state(&loading_zone.handle) {
                        Some(LoadState::NotLoaded) | Some(LoadState::Loading) => {
                            log::info!("[ZONE LOADER SYSTEM] Zone {} still loading (LoadState: {:?}), keeping in queue", 
                                zone_path, spawn_zone_params.asset_server.get_load_state(&loading_zone.handle));
                            index += 1;
                        }
                        Some(LoadState::Loaded) => {
                            log::info!("[ZONE LOADER SYSTEM] Zone {} loaded, transitioning to Spawned state", zone_path);
                            loading_zone.state = LoadingZoneState::Spawned;
                            index += 1;
                        }
                        None | Some(LoadState::Failed) => {
                            log::warn!("[ZONE LOADER SYSTEM] Zone {} failed to load (LoadState: {:?}), removing from queue", 
                                zone_path, spawn_zone_params.asset_server.get_load_state(&loading_zone.handle));
                            
                            // MEMORY LEAK FIX: Clear asset handles before removing failed zone
                            loading_zone.clear_asset_handles();
                            
                            loading_zones.remove(index);
                        }
                    }
                }
            }

            LoadingZoneState::Spawned => {
                
                let zone_handle = loading_zone.handle.clone();
                
                // Get zone_id from handle by looking up in cache
                let zone_id = if let Some(zone_index) = zone_loader_cache.cache.iter().position(|z| {
                    z.as_ref().map(|z| z.data_handle == zone_handle).unwrap_or(false)
                }) {
                    zone_loader_cache.cache.iter().enumerate().find_map(|(idx, z)| {
                        z.as_ref().and_then(|cached| {
                            if cached.data_handle == zone_handle {
                                Some(ZoneId::new(idx as u16).unwrap())
                            } else {
                                None
                            }
                        })
                    }).unwrap()
                } else {
                    log::error!("[ZONE LOADER SYSTEM] Cannot find zone_id for handle");
                    
                    // MEMORY LEAK FIX: Clear asset handles before removing zone with error
                    loading_zone.clear_asset_handles();
                    
                    loading_zones.remove(index);
                    continue;
                };
                
                // Despawn other zones first
                if loading_zone.despawn_other_zones {
                    log::info!("[ZONE LOADER SYSTEM] Despawning other zones");
                    for cached_zone in zone_loader_cache
                        .cache
                        .iter_mut()
                        .filter_map(|x| x.as_mut())
                    {
                        if let Some(spawned_entity) = cached_zone.spawned_entity.take()
                        {
                            info!("[ASSET LIFECYCLE] Despawning zone entity: {:?}", spawned_entity);
                            spawn_zone_params
                                    .commands
                                    .entity(spawned_entity)
                                    .despawn_recursive();
                            spawn_zone_params.memory_tracking.log_entity_despawned();
                        }
                    }

                    spawn_zone_params.commands.remove_resource::<CurrentZone>();
                }

                // Get zone_data and spawn
                let zone_handle_clone = zone_handle.clone();
                let spawn_result = {
                    let zone_data_opt = spawn_zone_params.zone_loader_assets.get(&zone_handle_clone);
                    
                    if let Some(zone_data) = zone_data_opt {
                        log::info!("[ZONE LOADER SYSTEM] Zone data retrieved, starting spawn process");
                        log::info!("[ZONE LOADER SYSTEM] Calling spawn_zone()");
                        // Extract the data we need before the borrow ends
                        let zone_id = zone_data.zone_id;
                        let zone_path = zone_data.zone_path.clone();
                        let blocks_len = zone_data.blocks.len();
                        let npcs_len = zone_data.npcs.len();

                        log::info!("[ZONE LOADER SYSTEM] Spawning zone: id={}, path={}, blocks={}, npcs={}",
                            zone_id.get(), zone_path.display(), blocks_len, npcs_len);

                        // Use raw pointer to work around borrow checker
                        // This is safe because:
                        // 1. spawn_zone doesn't actually use zone_loader_assets (it ignores it with `zone_loader_assets: _`)
                        // 2. spawn_zone_params is not modified through zone_loader_assets during the call
                        // 3. The reference is only used for the duration of spawn_zone call
                        let zone_data_ptr: *const ZoneLoaderAsset = zone_data;
                        let spawn_zone_params_ptr: *mut SpawnZoneParams = &mut spawn_zone_params;
                        
                        unsafe {
                            let zone_data_ref: &ZoneLoaderAsset = &*zone_data_ptr;
                            let spawn_zone_params_ref: &mut SpawnZoneParams = &mut *spawn_zone_params_ptr;
                            Some(spawn_zone(spawn_zone_params_ref, zone_data_ref))
                        }
                    } else {
                        log::warn!("[ZONE LOADER SYSTEM] Zone data not available!");
                        None::<Result<(Entity, Vec<UntypedHandle>), anyhow::Error>>
                    }
                };
                
                if let Some(result) = spawn_result {
                    match result {
                        Ok((zone_entity, zone_loading_assets)) => {
                            log::info!("[ZONE LOADER SYSTEM] Zone spawned successfully");
                            
                            // Check if assets are empty before moving
                            let assets_empty = zone_loading_assets.is_empty();
                            
                            // Update cache with spawned entity
                            let zone_index = zone_id.get() as usize;
                            if let Some(cached_zone) = zone_loader_cache.cache[zone_index].as_mut() {
                                cached_zone.spawned_entity = Some(zone_entity);
                            }
                            
                            loading_zone.zone_assets = zone_loading_assets;
                            loading_zone.state = LoadingZoneState::Spawned;
                            
                            if assets_empty {
                                log::info!("[ZONE LOADER SYSTEM] No additional assets to load, sending Loaded event");
                                
                                // MEMORY LEAK FIX: Clear asset handles before removing zone
                                loading_zone.clear_asset_handles();
                                
                                zone_events.send(ZoneEvent::Loaded(zone_id));
                                loading_zones.remove(index);
                            } else {
                                log::info!("[ZONE LOADER SYSTEM] Waiting for additional assets to load");
                                index += 1;
                            }
                        }
                        Err(e) => {
                            log::error!("[ZONE LOADER SYSTEM] Failed to spawn zone: {:?}", e);
                            
                            // MEMORY LEAK FIX: Clear asset handles before removing zone on failure
                            loading_zone.clear_asset_handles();
                            
                            loading_zones.remove(index);
                        }
                    }
                }
            }

            LoadingZoneState::Spawned => {
                let is_loading = loading_zone.zone_assets.iter().any(|handle| {
                    matches!(
                        spawn_zone_params.asset_server.get_load_state(handle),
                        Some(LoadState::NotLoaded) | Some(LoadState::Loading)
                    )
                });

                if is_loading {
                    index += 1;
                } else if let Some(zone_data) = spawn_zone_params.zone_loader_assets.get(&loading_zone.handle) {
                    // The physics system will take 2 frames to initialise colliders properly
                    loading_zone.ready_frames += 1;

                    if loading_zone.ready_frames == 2 {
                        log::info!("[ZONE LOADER SYSTEM] Zone ready after 2 frames, sending Loaded event");
                        
                        // MEMORY LEAK FIX: Clear asset handles before removing zone
                        loading_zone.clear_asset_handles();
                        
                        zone_events.send(ZoneEvent::Loaded(zone_data.zone_id));
                        loading_zones.remove(index);
                    } else {
                        index += 1;
                    }
                } else {
                    index += 1;
                }
            }
        }
    }
}

/// System to handle spawning zones that were loaded from VFS via async tasks
/// This separate system avoids borrow checker conflicts by handling spawning independently
/// CRITICAL FIX: Process ALL events, not just one, to prevent event queue buildup
/// CRITICAL FIX: Deduplicate events and prevent spawning already-loaded zones
pub fn zone_loaded_from_vfs_system(
    mut events: EventReader<ZoneLoadedFromVfsEvent>,
    mut zone_loader_cache: Local<ZoneLoaderCache>,
    mut zone_events: EventWriter<ZoneEvent>,
    mut current_zone: Option<ResMut<CurrentZone>>,
    mut debug_inspector_state: ResMut<DebugInspector>,
    mut spawn_zone_params: SpawnZoneParams,
    // CRITICAL FIX: Query existing zones to prevent duplicate spawning
    existing_zones: Query<(Entity, &Zone)>,
) {
    let _span = info_span!("zone_loaded_from_vfs_system").entered();
    let event_count = events.len();
    if event_count == 0 {
        return;
    }
    
    // CRITICAL FIX: Check for already-loaded zones to prevent duplicates
    let already_loaded: std::collections::HashSet<u16> = existing_zones
        .iter()
        .map(|(_, zone)| zone.id.get())
        .collect();
    
    if !already_loaded.is_empty() {
        log::info!("[ZONE LOADED FROM VFS] Currently loaded zones: {:?}",
            already_loaded.iter().collect::<Vec<_>>());
    }
    
    log::info!("[ZONE LOADED FROM VFS] Processing {} zone events this frame", event_count);
    spawn_zone_params.memory_tracking.log_summary();
    
    let mut processed_count = 0;
    let mut success_count = 0;
    let mut failed_count = 0;
    let mut skipped_count = 0;
    
    // CRITICAL FIX: Deduplicate events - prevent duplicate zone IDs in same batch
    let mut seen_zone_ids: std::collections::HashSet<u16> = std::collections::HashSet::new();
    
    // Process events directly, skipping duplicates
    for event in events.read() {
        // Deduplicate: Skip duplicate zone IDs in the same batch
        if !seen_zone_ids.insert(event.zone_id.get()) {
            log::warn!("[ZONE LOADED FROM VFS] DUPLICATE EVENT for zone {} ignored in batch",
                event.zone_id.get());
            skipped_count += 1;
            continue;
        }
        
        // CRITICAL FIX: Skip zones that are already loaded
        if already_loaded.contains(&event.zone_id.get()) {
            log::warn!("[ZONE LOADED FROM VFS] Zone {} already exists, skipping spawn to prevent memory leak",
                event.zone_id.get());
            skipped_count += 1;
            continue;
        }
        
        processed_count += 1;
        log::info!("[ZONE LOADED FROM VFS] ===========================================");
        log::info!("[ZONE LOADED FROM VFS] Spawning zone {} from VFS (event {}/{})"
            , event.zone_id.get(), processed_count, event_count);
        log::info!("[ZONE LOADED FROM VFS] ===========================================");
        
        let zone_index = event.zone_id.get() as usize;
        
        // Update cache - we don't need a handle for VFS-loaded zones
        if zone_loader_cache.cache.len() <= zone_index {
            // We can't use resize() because CachedZone doesn't implement Clone
            while zone_loader_cache.cache.len() <= zone_index {
                zone_loader_cache.cache.push(None);
            }
        }
        
        zone_loader_cache.cache[zone_index] = Some(CachedZone {
            data_handle: Handle::<ZoneLoaderAsset>::default(),
            spawned_entity: None,
        });
        
        // Spawn the zone using the asset from the event
        match spawn_zone(&mut spawn_zone_params, &event.zone_asset) {
            Ok((entity, zone_assets)) => {
                success_count += 1;
                log::info!("[ZONE LOADED FROM VFS] Zone {} spawned successfully! entity={:?}",
                    event.zone_id.get(), entity);
                
                // Update cache with spawned entity
                if let Some(cached_zone) = zone_loader_cache.cache[zone_index].as_mut() {
                    cached_zone.spawned_entity = Some(entity);
                }
                
                // Update current zone if it exists
                if let Some(ref mut current_zone) = current_zone {
                    **current_zone = CurrentZone {
                        id: event.zone_id,
                        handle: Handle::default(),
                    };
                }
                
                // Send loaded event
                zone_events.send(ZoneEvent::Loaded(event.zone_id));
                
                // Update debug inspector
                debug_inspector_state.entity = Some(entity);
                
                // Log memory summary after zone spawn
                info!("[MEMORY TRACKING] Zone {} loaded successfully", event.zone_id.get());
            }
            Err(e) => {
                failed_count += 1;
                log::error!("[ZONE LOADED FROM VFS] Failed to spawn zone {}: {:?}",
                    event.zone_id.get(), e);
            }
        }
    }
    
    log::info!("[ZONE LOADED FROM VFS] ===========================================");
    log::info!("[ZONE LOADED FROM VFS] Processing complete: {} success, {} failed, {} skipped (duplicates) out of {}",
        success_count, failed_count, skipped_count, processed_count);
    spawn_zone_params.memory_tracking.log_summary();
    log::info!("[ZONE LOADED FROM VFS] ===========================================");
}

pub fn spawn_zone(
    params: &mut SpawnZoneParams,
    zone_data: &ZoneLoaderAsset,
) -> Result<(Entity, Vec<UntypedHandle>), anyhow::Error> {
    let _span = info_span!("spawn_zone", zone_id = zone_data.zone_id.get()).entered();
    log::info!("[SPAWN ZONE] ===========================================");
    log::info!("[SPAWN ZONE] spawn_zone called for zone_id: {}", zone_data.zone_id.get());
    log::info!("[SPAWN ZONE] Zone path: {:?}", zone_data.zone_path);
    log::info!("[SPAWN ZONE] Number of blocks: {}", zone_data.blocks.len());
    log::info!("[SPAWN ZONE] Number of NPCs: {}", zone_data.npcs.len());
    log::info!("[SPAWN ZONE] ===========================================");

    // Memory tracking: Count blocks with data
    let blocks_with_data = zone_data.blocks.iter().filter(|b| b.is_some()).count();
    log::info!("[MEMORY] Blocks with data: {}/{}", blocks_with_data, zone_data.blocks.len());

    let SpawnZoneParams {
        commands,
        asset_server,
        game_data,
        vfs_resource,
        meshes,
        specular_texture,
        sky_materials,
        terrain_materials,
        effect_mesh_materials,
        particle_materials,
        object_materials,
        water_materials,
        zone_loader_assets: _,
        memory_tracking,
    } = params;

    let zone_list_entry = game_data
        .zone_list
        .get_zone(zone_data.zone_id)
        .ok_or(ZoneLoadError::InvalidZoneId)?;

    let mut tile_textures: Vec<Handle<Image>> =
        Vec::with_capacity(zone_data.zon.tile_textures.len());
    for path in zone_data.zon.tile_textures.iter() {
        if path == "end" {
            break;
        }

        let handle = asset_server.load(path);
        memory_tracking.log_texture_handle_created(path);
        tile_textures.push(handle);
    }
    log::info!("[SPAWN ZONE] Loaded {} tile textures", tile_textures.len());
    log::info!("[MEMORY] Tile texture handles created: {}", tile_textures.len());

    let water_material = {
        let mut water_material_textures = Vec::with_capacity(25);
        for i in 1..=25 {
            let path = format!("3DDATA/JUNON/WATER/OCEAN01_{:02}.DDS", i);
            let handle = asset_server.load(&path);
            memory_tracking.log_texture_handle_created(&path);
            water_material_textures.push(handle);
        }

        let texture_count = water_material_textures.len();
        let material = water_materials.add(WaterMaterial {
            textures: water_material_textures,
        });
        info!("[MEMORY TRACKING] Water material created with {} textures", texture_count);
        log::info!("[SPAWN ZONE] Water material created with {} textures", texture_count);
        log::info!("[MEMORY] Water material handle created");
        material
    };

    let mut zone_loading_assets: Vec<UntypedHandle> = Vec::default();
    let zone_entity = commands
        .spawn((
            Zone {
                id: zone_data.zone_id,
            },
            Visibility::Visible,  // Explicitly set to Visible
            ViewVisibility::default(),
            InheritedVisibility::default(),
            Transform::default(),
            GlobalTransform::default(),
        ))
        .id();
    info!("[ASSET LIFECYCLE] Zone entity spawned: {:?} (zone_id: {})", zone_entity, zone_data.zone_id.get());
    memory_tracking.log_entity_spawned("Zone", 0);
    log::info!("[SPAWN ZONE] Zone entity spawned: {:?}", zone_entity);
    log::info!("[MEMORY] Zone entity created: {:?}", zone_entity);

    if let Some(skybox_data) = zone_list_entry
        .skybox_id
        .and_then(|skybox_id| game_data.skybox.get_skybox_data(skybox_id))
    {
        log::info!("[SPAWN ZONE] Spawning skybox");
        let skybox_entity = spawn_skybox(commands, asset_server, sky_materials, skybox_data);
        info!("[ASSET LIFECYCLE] Skybox entity spawned: {:?}", skybox_entity);
        memory_tracking.log_entity_spawned("Skybox", 2);
        log::info!("[SPAWN ZONE] Skybox entity spawned: {:?}", skybox_entity);
        log::info!("[MEMORY] Skybox entity created: {:?}", skybox_entity);
        commands.entity(zone_entity).add_child(skybox_entity);
    } else {
        log::warn!("[SPAWN ZONE] No skybox data found for zone {}", zone_data.zone_id.get());
    }

    let mut terrain_count = 0;
    let mut water_count = 0;
    let mut event_object_count = 0;
    let mut warp_object_count = 0;
    let mut cnst_object_count = 0;
    let mut deco_object_count = 0;
    let mut animated_object_count = 0;
    let mut effect_object_count = 0;
    let mut sound_object_count = 0;

    for block_y in 0..64 {
        for block_x in 0..64 {
            if let Some(block_data) = zone_data.blocks[block_x + block_y * 64].as_ref() {
                let terrain_entity = spawn_terrain(
                    commands,
                    asset_server,
                    meshes,
                    terrain_materials,
                    &tile_textures,
                    zone_data,
                    block_data,
                );
                commands.entity(zone_entity).add_child(terrain_entity);
                terrain_count += 1;

                if let Some(ifo) = block_data.ifo.as_ref() {
                    let lightmap_path = zone_data
                        .zone_path
                        .join(format!("{}_{}/LIGHTMAP/", block_x, block_y));

                    for (plane_start, plane_end) in ifo.water_planes.iter() {
                        let water_entity = spawn_water(
                            commands,
                            meshes,
                            &water_material,
                            ifo.water_size,
                            Vec3::new(plane_start.x, plane_start.y, plane_start.z),
                            Vec3::new(plane_end.x, plane_end.y, plane_end.z),
                        );
                        commands.entity(zone_entity).add_child(water_entity);
                        water_count += 1;
                    }

                    for (ifo_object_id, event_object) in ifo.event_objects.iter().enumerate() {
                        let event_entity = spawn_object(
                            commands,
                            asset_server,
                            &mut zone_loading_assets,
                            vfs_resource,
                            effect_mesh_materials.as_mut(),
                            particle_materials.as_mut(),
                            object_materials.as_mut(),
                            specular_texture,
                            &game_data.zsc_event_object,
                            &lightmap_path,
                            None,
                            &event_object.object,
                            ifo_object_id,
                            event_object.object.object_id as usize,
                            ZoneObject::EventObject,
                            ZoneObject::EventObjectPart,
                            COLLISION_GROUP_ZONE_EVENT_OBJECT,
                        );

                        commands.entity(event_entity).insert(EventObject::new(
                            event_object.quest_trigger_name.clone(),
                            event_object.script_function_name.clone(),
                        ));
                        commands.entity(zone_entity).add_child(event_entity);
                        event_object_count += 1;
                    }

                    for (ifo_object_id, warp_object) in ifo.warps.iter().enumerate() {
                        let warp_entity = spawn_object(
                            commands,
                            asset_server,
                            &mut zone_loading_assets,
                            vfs_resource,
                            effect_mesh_materials.as_mut(),
                            particle_materials.as_mut(),
                            object_materials.as_mut(),
                            specular_texture,
                            &game_data.zsc_special_object,
                            &lightmap_path,
                            None,
                            warp_object,
                            ifo_object_id,
                            1,
                            ZoneObject::WarpObject,
                            ZoneObject::WarpObjectPart,
                            COLLISION_GROUP_ZONE_WARP_OBJECT,
                        );

                        commands
                            .entity(warp_entity)
                            .insert(WarpObject::new(WarpGateId::new(warp_object.warp_id)));
                        commands.entity(zone_entity).add_child(warp_entity);
                        warp_object_count += 1;
                    }

                    for (ifo_object_id, object_instance) in ifo.cnst_objects.iter().enumerate() {
                        let lit_object = block_data.lit_cnst.as_ref().and_then(|lit| {
                            lit.objects
                                .iter()
                                .find(|lit_object| lit_object.id as usize == ifo_object_id + 1)
                        });

                        let object_entity = spawn_object(
                            commands,
                            asset_server,
                            &mut zone_loading_assets,
                            vfs_resource,
                            effect_mesh_materials.as_mut(),
                            particle_materials.as_mut(),
                            object_materials.as_mut(),
                            specular_texture,
                            &zone_data.zsc_cnst,
                            &lightmap_path,
                            lit_object,
                            object_instance,
                            ifo_object_id,
                            object_instance.object_id as usize,
                            ZoneObject::CnstObject,
                            ZoneObject::CnstObjectPart,
                            COLLISION_GROUP_ZONE_OBJECT,
                        );
                        commands.entity(zone_entity).add_child(object_entity);
                        cnst_object_count += 1;
                    }

                    for (ifo_object_id, object_instance) in ifo.deco_objects.iter().enumerate() {
                        let lit_object = block_data.lit_deco.as_ref().and_then(|lit| {
                            lit.objects
                                .iter()
                                .find(|lit_object| lit_object.id as usize == ifo_object_id + 1)
                        });

                        let object_entity = spawn_object(
                            commands,
                            asset_server,
                            &mut zone_loading_assets,
                            vfs_resource,
                            effect_mesh_materials.as_mut(),
                            particle_materials.as_mut(),
                            object_materials.as_mut(),
                            specular_texture,
                            &zone_data.zsc_deco,
                            &lightmap_path,
                            lit_object,
                            object_instance,
                            ifo_object_id,
                            object_instance.object_id as usize,
                            ZoneObject::DecoObject,
                            ZoneObject::DecoObjectPart,
                            COLLISION_GROUP_ZONE_OBJECT,
                        );
                        commands.entity(zone_entity).add_child(object_entity);
                        deco_object_count += 1;
                    }

                    for object_instance in ifo.animated_objects.iter() {
                        let object_entity = spawn_animated_object(
                            commands,
                            asset_server,
                            effect_mesh_materials.as_mut(),
                            &game_data.stb_morph_object,
                            object_instance,
                        );
                        commands.entity(zone_entity).add_child(object_entity);
                        animated_object_count += 1;
                    }

                    for (ifo_object_id, effect_object) in ifo.effect_objects.iter().enumerate() {
                        let object_entity = spawn_effect_object(
                            commands,
                            asset_server,
                            vfs_resource,
                            effect_mesh_materials.as_mut(),
                            particle_materials.as_mut(),
                            effect_object,
                            ifo_object_id,
                        );
                        commands.entity(zone_entity).add_child(object_entity);
                        effect_object_count += 1;
                    }

                    for (ifo_object_id, sound_object) in ifo.sound_objects.iter().enumerate() {
                        let object_entity =
                            spawn_sound_object(commands, asset_server, sound_object, ifo_object_id);
                        commands.entity(zone_entity).add_child(object_entity);
                        sound_object_count += 1;
                    }
                }
            }
        }
    }

    log::info!("[SPAWN ZONE] ===========================================");
    log::info!("[SPAWN ZONE] Zone spawning complete");
    log::info!("[SPAWN ZONE] Terrain entities: {}", terrain_count);
    log::info!("[SPAWN ZONE] Water entities: {}", water_count);
    log::info!("[SPAWN ZONE] Event objects: {}", event_object_count);
    log::info!("[SPAWN ZONE] Warp objects: {}", warp_object_count);
    log::info!("[SPAWN ZONE] Cnst objects: {}", cnst_object_count);
    log::info!("[SPAWN ZONE] Deco objects: {}", deco_object_count);
    log::info!("[SPAWN ZONE] Animated objects: {}", animated_object_count);
    log::info!("[SPAWN ZONE] Effect objects: {}", effect_object_count);
    log::info!("[SPAWN ZONE] Sound objects: {}", sound_object_count);
    let total_entities = terrain_count + water_count + event_object_count + warp_object_count + cnst_object_count + deco_object_count + animated_object_count + effect_object_count + sound_object_count;
    log::info!("[SPAWN ZONE] Total entities spawned: {}", total_entities);

    // Enhanced zone entity count logging
    let object_count = event_object_count + warp_object_count + cnst_object_count + deco_object_count + animated_object_count + effect_object_count + sound_object_count;
    info!("[ZONE] Zone '{}' spawned with {} entities", zone_data.zone_id.get(), total_entities);
    info!("[ZONE]   Terrain entities: {}", terrain_count);
    info!("[ZONE]   Water entities: {}", water_count);
    info!("[ZONE]   Object entities: {}", object_count);

    log::info!("[MEMORY] Zone loading assets: {}", zone_loading_assets.len());
    log::info!("[MEMORY TRACKING] Zone spawn complete - logging memory summary");
    memory_tracking.log_summary();
    log::info!("[SPAWN ZONE] ===========================================");

    Ok((zone_entity, zone_loading_assets))
}

const SKYBOX_MODEL_SCALE: f32 = 10.0;

fn spawn_skybox(
    commands: &mut Commands,
    asset_server: &AssetServer,
    sky_materials: &mut Assets<SkyMaterial>,
    skybox_data: &SkyboxData,
) -> Entity {
    let mesh_path = skybox_data.mesh.path().to_string_lossy().into_owned();
    let texture_day_path = skybox_data.texture_day.path().to_string_lossy().into_owned();
    let texture_night_path = skybox_data.texture_night.path().to_string_lossy().into_owned();

    let mesh_handle = asset_server.load::<Mesh>(&mesh_path);
    let texture_day_handle = asset_server.load::<Image>(&texture_day_path);
    let texture_night_handle = asset_server.load::<Image>(&texture_night_path);

    info!("[MEMORY TRACKING] Skybox mesh handle created: {}", mesh_path);
    info!("[MEMORY TRACKING] Skybox texture day handle created: {}", texture_day_path);
    info!("[MEMORY TRACKING] Skybox texture night handle created: {}", texture_night_path);

    commands
        .spawn((
            mesh_handle,
            sky_materials.add(SkyMaterial {
                texture_day: Some(texture_day_handle),
                texture_night: Some(texture_night_handle),
            }),
            Transform::from_scale(Vec3::splat(SKYBOX_MODEL_SCALE)),
            GlobalTransform::default(),
            Visibility::Visible,  // Explicitly visible
            ViewVisibility::default(),
            InheritedVisibility::default(),
            NoFrustumCulling,
        ))
        .id()
}

#[allow(clippy::too_many_arguments)]
fn spawn_terrain(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    terrain_materials: &mut Assets<TerrainMaterial>,
    tile_textures: &Vec<Handle<Image>>,
    zone_data: &ZoneLoaderAsset,
    block_data: &ZoneLoaderBlock,
) -> Entity {
    let _span = info_span!("spawn_terrain", block_x = block_data.block_x, block_y = block_data.block_y).entered();
    log::info!("[SPAWN TERRAIN] Spawning terrain block {}_{}", block_data.block_x, block_data.block_y);
    let offset_x = 160.0 * block_data.block_x as f32;
    let offset_y = 160.0 * (65.0 - block_data.block_y as f32);

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs_lightmap = Vec::new();
    let mut uvs_tile = Vec::new();
    let mut indices = Vec::new();
    let mut tile_ids = Vec::new();

    let tilemap = block_data.til.as_ref();
    let heightmap = &block_data.him;

    let mut tile_texture_map = vec![0; tile_textures.len()];
    let mut terrain_material = TerrainMaterial {
        textures: Vec::with_capacity(tile_textures.len() + 1),
    };

    terrain_material.textures.push(asset_server.load(format!(
        "{}/{1:}_{2:}/{1:}_{2:}_PLANELIGHTINGMAP.DDS",
        zone_data.zone_path.to_str().unwrap(),
        block_data.block_x,
        block_data.block_y,
    )));

    // Build TerrainMaterial and tile_texture_map
    for tile_x in 0..16 {
        for tile_y in 0..16 {
            let tile = &zone_data.zon.tiles[tilemap
                .map(|tilemap| tilemap.get_clamped(tile_x, tile_y) as usize)
                .unwrap_or(0)];
            let tile_array_index1 = tile.layer1 + tile.offset1;
            let tile_array_index2 = tile.layer2 + tile.offset2;

            if tile_array_index1 as usize >= tile_texture_map.len() {
                warn!(
                    "Invalid tile layer1 id {}, tile.layer1: {} + tile.offset1: {}",
                    tile_array_index1, tile.layer1, tile.offset1
                );
            }

            if tile_array_index2 as usize >= tile_texture_map.len() {
                warn!(
                    "Invalid tile layer2 id {}, tile.layer2: {} + tile.offset2: {}",
                    tile_array_index2, tile.layer2, tile.offset2
                );
            }

            if tile_texture_map[tile_array_index1 as usize] == 0 {
                let index = terrain_material.textures.len();
                if index == TERRAIN_MATERIAL_MAX_TEXTURES {
                    warn!(
                        "Reached maximum TERRAIN_MATERIAL_MAX_TEXTURES for block ({}, {})",
                        block_data.block_x, block_data.block_y
                    );
                    tile_texture_map[tile_array_index1 as usize] = 0;
                } else {
                    terrain_material
                        .textures
                        .push(tile_textures[tile_array_index1 as usize].clone());
                    tile_texture_map[tile_array_index1 as usize] = index as u32;
                }
            }

            if tile_texture_map[tile_array_index2 as usize] == 0 {
                let index = terrain_material.textures.len();
                if index == TERRAIN_MATERIAL_MAX_TEXTURES {
                    warn!(
                        "Reached maximum TERRAIN_MATERIAL_MAX_TEXTURES for block ({}, {})",
                        block_data.block_x, block_data.block_y
                    );
                    tile_texture_map[tile_array_index2 as usize] = 0;
                } else {
                    terrain_material
                        .textures
                        .push(tile_textures[tile_array_index2 as usize].clone());
                    tile_texture_map[tile_array_index2 as usize] = index as u32;
                }
            }
        }
    }

    // Create cache key from sorted texture indices (excluding lightmap which is unique per block)
    let mut texture_indices: Vec<usize> = terrain_material.textures.iter()
        .skip(1) // Skip lightmap at index 0
        .filter_map(|handle| {
            // Find the index of this texture in tile_textures
            tile_textures.iter().position(|t| t == handle)
        })
        .collect();
    texture_indices.sort();
    texture_indices.dedup();
    let cache_key = texture_indices.iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join("|");

    for tile_x in 0..16 {
        for tile_y in 0..16 {
            let tile = &zone_data.zon.tiles[tilemap
                .map(|tilemap| tilemap.get_clamped(tile_x, tile_y) as usize)
                .unwrap_or(0)];
            let tile_array_index1 = tile_texture_map[(tile.layer1 + tile.offset1) as usize];
            let tile_array_index2 = tile_texture_map[(tile.layer2 + tile.offset2) as usize];
            let tile_rotation = match tile.rotation {
                ZonTileRotation::FlipHorizontal => 2,
                ZonTileRotation::FlipVertical => 3,
                ZonTileRotation::Flip => 4,
                ZonTileRotation::Clockwise90 => 5,
                ZonTileRotation::CounterClockwise90 => 6,
                _ => 0,
            };
            let tile_indices_base = positions.len() as u16;
            let tile_offset_x = tile_x as f32 * 4.0 * 2.5;
            let tile_offset_y = tile_y as f32 * 4.0 * 2.5;

            for y in 0..5 {
                for x in 0..5 {
                    let heightmap_x = x + tile_x as i32 * 4;
                    let heightmap_y = y + tile_y as i32 * 4;
                    let height = heightmap.get_clamped(heightmap_x, heightmap_y) / 100.0;
                    let height_l = heightmap.get_clamped(heightmap_x - 1, heightmap_y) / 100.0;
                    let height_r = heightmap.get_clamped(heightmap_x + 1, heightmap_y) / 100.0;
                    let height_t = heightmap.get_clamped(heightmap_x, heightmap_y - 1) / 100.0;
                    let height_b = heightmap.get_clamped(heightmap_x, heightmap_y + 1) / 100.0;
                    let normal = Vec3::new(
                        (height_l - height_r) / 2.0,
                        1.0,
                        (height_t - height_b) / 2.0,
                    )
                    .normalize();

                    positions.push([
                        tile_offset_x + x as f32 * 2.5,
                        height,
                        tile_offset_y + y as f32 * 2.5,
                    ]);
                    normals.push([normal.x, normal.y, normal.z]);
                    uvs_tile.push([x as f32 / 4.0, y as f32 / 4.0]);
                    uvs_lightmap.push([
                        (tile_x as f32 * 4.0 + x as f32) / 64.0,
                        (tile_y as f32 * 4.0 + y as f32) / 64.0,
                    ]);

                    tile_ids.push(tile_array_index1 | tile_array_index2 << 8 | tile_rotation << 16);
                }
            }

            for y in 0..(5 - 1) {
                for x in 0..(5 - 1) {
                    let start = tile_indices_base + y * 5 + x;
                    indices.push(start);
                    indices.push(start + 5);
                    indices.push(start + 1);

                    indices.push(start + 1);
                    indices.push(start + 5);
                    indices.push(start + 1 + 5);
                }
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    let vertex_count = positions.len();
    let triangle_count = indices.len() / 3;
    mesh.insert_indices(Indices::U16(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs_lightmap);
    mesh.insert_attribute(MESH_ATTRIBUTE_UV_1, uvs_tile);
    mesh.insert_attribute(TERRAIN_MESH_ATTRIBUTE_TILE_INFO, tile_ids);
    log::info!("[SPAWN TERRAIN] Block {}_{}: Mesh created with {} vertices, {} triangles",
        block_data.block_x, block_data.block_y, vertex_count, triangle_count);
    log::info!("[MEMORY] Terrain mesh created for block {}_{}", block_data.block_x, block_data.block_y);

    let mut collider_verts = Vec::new();
    let mut collider_indices = Vec::new();

    for y in 0..heightmap.height as i32 {
        for x in 0..heightmap.width as i32 {
            collider_verts.push(
                [
                    x as f32 * 2.5,
                    heightmap.get_clamped(x, y) / 100.0,
                    y as f32 * 2.5,
                ]
                .into(),
            );
        }
    }

    for y in 0..(heightmap.height - 1) {
        for x in 0..(heightmap.width - 1) {
            let start = y * heightmap.width + x;
            collider_indices.push([start, start + heightmap.width, start + 1]);
            collider_indices.push([
                start + 1,
                start + heightmap.width,
                start + 1 + heightmap.width,
            ]);
        }
    }

    let texture_count = terrain_material.textures.len();
    let material_handle = terrain_materials.add(terrain_material);

    let terrain_entity = commands
        .spawn((
            ZoneObject::Terrain(ZoneObjectTerrain {
                block_x: block_data.block_x as u32,
                block_y: block_data.block_y as u32,
            }),
            meshes.add(mesh),
            material_handle,
            Transform::from_xyz(offset_x, 0.0, -offset_y),
            GlobalTransform::default(),
            Visibility::Visible,  // Explicitly visible
            ViewVisibility::default(),
            InheritedVisibility::default(),
            NotShadowCaster,
            RigidBody::Fixed,
            Collider::trimesh(collider_verts, collider_indices),
            CollisionGroups::new(
                COLLISION_GROUP_ZONE_TERRAIN,
                COLLISION_FILTER_INSPECTABLE
                    | COLLISION_FILTER_COLLIDABLE
                    | COLLISION_GROUP_PHYSICS_TOY
                    | COLLISION_FILTER_MOVEABLE
                    | COLLISION_FILTER_CLICKABLE,
            ),
        ))
        .id();
    info!("[ASSET LIFECYCLE] Terrain entity spawned: {:?} block {}_{} with {} textures",
        terrain_entity, block_data.block_x, block_data.block_y, texture_count);
    log::info!("[SPAWN TERRAIN] Terrain entity created: {:?} at position ({}, 0, {})",
        terrain_entity, offset_x, offset_y);
    log::info!("[MEMORY] Terrain material created for block {}_{} with {} textures",
        block_data.block_x, block_data.block_y, texture_count);
    terrain_entity
}

fn spawn_water(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    water_material: &Handle<WaterMaterial>,
    water_size: f32,
    plane_start: Vec3,
    plane_end: Vec3,
) -> Entity {
    let start = Vec3::new(
        5200.0 + plane_start.x / 100.0,
        plane_start.y / 100.0,
        -(5200.0 + plane_start.z / 100.0),
    );
    let end = Vec3::new(
        5200.0 + plane_end.x / 100.0,
        plane_end.y / 100.0,
        -(5200.0 + plane_end.z / 100.0),
    );
    let uv_x = (end.x - start.x) / (water_size / 100.0);
    let uv_y = (end.z - start.z) / (water_size / 100.0);

    let vertices = [
        ([start.x, start.y, end.z], [0.0, 1.0, 0.0], [uv_x, uv_y]),
        ([start.x, start.y, start.z], [0.0, 1.0, 0.0], [uv_x, 0.0]),
        ([end.x, start.y, start.z], [0.0, 1.0, 0.0], [0.0, 0.0]),
        ([end.x, start.y, end.z], [0.0, 1.0, 0.0], [0.0, uv_y]),
    ];
    let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);
    let collider_indices = vec![[0, 2, 1], [0, 3, 2]];

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

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_indices(indices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    let water_entity = commands
        .spawn((
            ZoneObject::Water,
            meshes.add(mesh),
            water_material.clone(),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,  // Explicitly visible
            ViewVisibility::default(),
            InheritedVisibility::default(),
            NotShadowCaster,
            NotShadowReceiver,
            RigidBody::Fixed,
            Collider::trimesh(collider_verts, collider_indices),
            CollisionGroups::new(COLLISION_GROUP_ZONE_WATER, COLLISION_FILTER_INSPECTABLE),
        ))
        .id();
    
    info!("[ASSET LIFECYCLE] Water entity spawned: {:?}", water_entity);
    water_entity
}

fn spawn_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    zone_loading_assets: &mut Vec<UntypedHandle>,
    vfs_resource: &VfsResource,
    effect_mesh_materials: &mut Assets<EffectMeshMaterial>,
    particle_materials: &mut Assets<ParticleMaterial>,
    object_materials: &mut Assets<ObjectMaterial>,
    specular_texture: &SpecularTexture,
    zsc: &ZscFile,
    lightmap_path: &Path,
    lit_object: Option<&LitObject>,
    object_instance: &IfoObject,
    ifo_object_id: usize,
    zsc_object_id: usize,
    object_type: fn(ZoneObjectId) -> ZoneObject,
    part_object_type: fn(ZoneObjectPart) -> ZoneObject,
    collision_group: bevy_rapier3d::prelude::Group,
) -> Entity {
    log::info!("[SPAWN OBJECT] Spawning object: IFO id={}, ZSC id={}, parts={}",
        ifo_object_id, zsc_object_id, zsc.objects[zsc_object_id].parts.len());
    let object = &zsc.objects[zsc_object_id];
    let object_transform = Transform::default()
        .with_translation(
            Vec3::new(
                object_instance.position.x,
                object_instance.position.z,
                -object_instance.position.y,
            ) / 100.0
                + Vec3::new(5200.0, 0.0, -5200.0),
        )
        .with_rotation(Quat::from_xyzw(
            object_instance.rotation.x,
            object_instance.rotation.z,
            -object_instance.rotation.y,
            object_instance.rotation.w,
        ))
        .with_scale(Vec3::new(
            object_instance.scale.x,
            object_instance.scale.z,
            object_instance.scale.y,
        ));

    let mut mesh_cache: Vec<Option<Handle<Mesh>>> = vec![None; zsc.meshes.len()];

    let mut part_entities: ArrayVec<Entity, 256> = ArrayVec::new();
    let mut object_entity_commands = commands.spawn((
        object_type(ZoneObjectId {
            ifo_object_id,
            zsc_object_id,
        }),
        object_transform,
        GlobalTransform::default(),
        Visibility::Visible,  // Explicitly visible
        ViewVisibility::default(),
        InheritedVisibility::default(),
        RigidBody::Fixed,
    ));

    let object_entity = object_entity_commands.id();

    object_entity_commands.with_children(|object_commands| {
        for (part_index, object_part) in object.parts.iter().enumerate() {
            let part_transform = Transform::default()
                .with_translation(
                    Vec3::new(
                        object_part.position.x,
                        object_part.position.z,
                        -object_part.position.y,
                    ) / 100.0,
                )
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
            
            // VALIDATION FIX: Check mesh_id bounds before using
            if mesh_id >= zsc.meshes.len() {
                log::warn!("[SPAWN OBJECT] Object {} part {} has invalid mesh_id {} (max: {}), skipping part",
                    zsc_object_id, part_index, mesh_id, zsc.meshes.len().saturating_sub(1));
                continue;
            }
            
            // VALIDATION FIX: Check material_id bounds
            let material_id = object_part.material_id as usize;
            if material_id >= zsc.materials.len() {
                log::warn!("[SPAWN OBJECT] Object {} part {} has invalid material_id {} (max: {}), skipping part",
                    zsc_object_id, part_index, material_id, zsc.materials.len().saturating_sub(1));
                continue;
            }
            
            let mesh = mesh_cache[mesh_id].clone().unwrap_or_else(|| {
                let mesh_path = zsc.meshes[mesh_id].path().to_string_lossy().into_owned();
                let mesh_path_log = mesh_path.clone();
                log::info!("[SPAWN OBJECT] Loading mesh: {}", mesh_path_log);
                let handle = asset_server.load(&mesh_path);
                mesh_cache.insert(mesh_id, Some(handle.clone()));
                info!("[MEMORY TRACKING] Mesh handle created: {}", mesh_path_log);
                handle
            });
            zone_loading_assets.push(UntypedHandle::from(mesh.clone()));
            let lit_part = lit_object.and_then(|lit_object| {
                for part in lit_object.parts.iter() {
                    if part_index == part.object_part_index as usize {
                        return Some(part);
                    }
                }

                lit_object.parts.get(part_index)
            });
            let lightmap_texture =
                lit_part.map(|lit_part| {
                    let path = lightmap_path.join(&lit_part.filename);
                    let path_str = path.to_string_lossy().into_owned();
                    let handle = asset_server.load(&path_str);
                    info!("[MEMORY TRACKING] Lightmap texture handle created: {}", path_str);
                    handle
                });
            let (lightmap_uv_offset, lightmap_uv_scale) = lit_part
                .map(|lit_part| {
                    let scale = 1.0 / lit_part.parts_per_row as f32;
                    (
                        Vec2::new(
                            (lit_part.part_index % lit_part.parts_per_row) as f32,
                            (lit_part.part_index / lit_part.parts_per_row) as f32,
                        ),
                        scale,
                    )
                })
                .unwrap_or((Vec2::new(0.0, 0.0), 1.0));

            // NOTE: material_id was already validated at lines 2437-2443 above
            // This second fetch is just for local use
            let material_id = object_part.material_id as usize;
            
            let zsc_material = zsc.materials[material_id].clone();
            let material_path = zsc_material.path.path().to_string_lossy().into_owned();
            let material_path_log = material_path.clone();
            
            log::info!("[SPAWN OBJECT] Creating material: {}", material_path_log);
            let base_texture_handle = asset_server.load(&material_path);
            info!("[MEMORY TRACKING] Object material base texture handle created: {}", material_path_log);

            let lightmap_count = lightmap_texture.as_ref().is_some() as usize;

            let material = object_materials.add(ObjectMaterial {
                base_texture: if material_path.is_empty() || material_path == "" {
                    log::warn!("[SPAWN OBJECT DEBUG] Empty texture path for mesh_id {}, using fallback", mesh_id);
                    Some(asset_server.load("ETC/SPECULAR_SPHEREMAP.DDS"))
                } else {
                    Some(base_texture_handle.clone())
                },
                lightmap_texture: lightmap_texture.clone(),
                alpha_value: if zsc_material.alpha != 1.0 {
                    Some(zsc_material.alpha)
                } else {
                    None
                },
                alpha_enabled: zsc_material.alpha_enabled,
                alpha_test: zsc_material.alpha_test,
                two_sided: zsc_material.two_sided,
                z_write_enabled: zsc_material.z_write_enabled,
                z_test_enabled: zsc_material.z_test_enabled,
                specular_texture: if zsc_material.specular_enabled {
                    Some(specular_texture.image.clone())
                } else {
                    None
                },
                blend: zsc_material.blend_mode.into(),
                glow: zsc_material.glow.map(|x| x.into()),
                skinned: zsc_material.is_skin,
                lightmap_uv_offset,
                lightmap_uv_scale,
            });

            let mut collision_filter = COLLISION_FILTER_INSPECTABLE;

            if object_part.collision_shape.is_some() {
                if collision_group != COLLISION_GROUP_ZONE_EVENT_OBJECT
                    && collision_group != COLLISION_GROUP_ZONE_WARP_OBJECT
                    && !object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::HEIGHT_ONLY)
                {
                    collision_filter |= COLLISION_FILTER_COLLIDABLE | COLLISION_GROUP_PHYSICS_TOY;
                }

                if collision_group != COLLISION_GROUP_ZONE_WARP_OBJECT {
                    if !object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_PICKABLE)
                    {
                        collision_filter |= COLLISION_FILTER_CLICKABLE;
                    }

                    if !object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_MOVEABLE)
                    {
                        collision_filter |= COLLISION_FILTER_MOVEABLE;
                    }
                }
            }

            // CRITICAL FIX: Validate material handle before spawning
            let material_id = material.id();
            let is_material_weak = material.is_weak();
            
            // Verify material is strong
            if material.is_weak() {
                log::error!("[SPAWN OBJECT] CRITICAL: Material is weak! Object {} part {} will not render!",
                    zsc_object_id, part_index);
            }
            
            let mut part_commands = object_commands.spawn((
                part_object_type(ZoneObjectPart {
                    ifo_object_id,
                    zsc_object_id,
                    zsc_part_id: part_index,
                    mesh_path: zsc.meshes[mesh_id].path().to_string_lossy().into(),
                    collision_shape: (&object_part.collision_shape).into(),
                    collision_not_moveable: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_MOVEABLE),
                    collision_not_pickable: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_PICKABLE),
                    collision_height_only: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::HEIGHT_ONLY),
                    collision_no_camera: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_CAMERA_COLLISION),
                }),
                mesh.clone(),
                material,
                part_transform,
                GlobalTransform::default(),
                Visibility::Visible,  // Explicitly visible
                ViewVisibility::default(),
                InheritedVisibility::default(),
                NotShadowCaster,
                ColliderParent::new(object_entity),
                AsyncCollider(ComputedColliderShape::TriMesh),
                CollisionGroups::new(collision_group, collision_filter),
            ));

            let active_motion = object_part.animation_path.as_ref().map(|animation_path| {
                TransformAnimation::repeat(asset_server.load(animation_path.path().to_string_lossy().into_owned()), None)
            });
            if let Some(active_motion) = active_motion {
                part_commands.insert(active_motion);
            }

            part_entities.push(part_commands.id());
        }
    });

    log::info!("[SPAWN OBJECT] Object entity created: {:?} with {} parts",
        object_entity, part_entities.len());
    let mesh_count = mesh_cache.iter().filter(|m| m.is_some()).count();
    info!("[MEMORY TRACKING] Object entity created with {} mesh handles",
        mesh_count);
    log::info!("[MEMORY] Object entity created with {} mesh handles",
        mesh_count);

    for object_effect in object.effects.iter() {
        let effect_transform = Transform::default()
            .with_translation(
                Vec3::new(
                    object_effect.position.x,
                    object_effect.position.z,
                    -object_effect.position.y,
                ) / 100.0,
            )
            .with_rotation(Quat::from_xyzw(
                object_effect.rotation.x,
                object_effect.rotation.z,
                -object_effect.rotation.y,
                object_effect.rotation.w,
            ))
            .with_scale(Vec3::new(
                object_effect.scale.x,
                object_effect.scale.z,
                object_effect.scale.y,
            ));

        if let Some(effect_path) = zsc.effects.get(object_effect.effect_id as usize) {
            if let Some(effect_entity) = spawn_effect(
                &vfs_resource.vfs,
                commands,
                asset_server,
                particle_materials,
                effect_mesh_materials,
                effect_path.into(),
                false,
                None,
            ) {
                if let Some(parent_part_entity) = object_effect
                    .parent
                    .and_then(|parent_part_index| part_entities.get(parent_part_index as usize))
                {
                    commands
                        .entity(*parent_part_entity)
                        .add_child(effect_entity);
                } else {
                    commands.entity(object_entity).add_child(effect_entity);
                }

                commands.entity(effect_entity).insert(effect_transform);

                if matches!(object_effect.effect_type, ZscEffectType::DayNight) {
                    commands.entity(effect_entity).insert(NightTimeEffect);
                }
            }
        }
    }

    object_entity
}

fn spawn_animated_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect_mesh_materials: &mut Assets<EffectMeshMaterial>,
    stb_morph_object: &StbFile,
    object_instance: &IfoObject,
) -> Entity {
    let object_id = object_instance.object_id as usize;
    let mesh_path = stb_morph_object.get(object_id, 1).to_string();
    let motion_path = stb_morph_object.get(object_id, 2).to_string();
    let texture_path = stb_morph_object.get(object_id, 3).to_string();

    let alpha_enabled = stb_morph_object.get_int(object_id, 4) != 0;
    let two_sided = stb_morph_object.get_int(object_id, 5) != 0;
    let alpha_test_enabled = stb_morph_object.get_int(object_id, 6) != 0;
    let z_test_enabled = stb_morph_object.get_int(object_id, 7) != 0;
    let z_write_enabled = stb_morph_object.get_int(object_id, 8) != 0;

    let src_blend_factor = stb_morph_object.get_int(object_id, 9) as u32;
    let dst_blend_factor = stb_morph_object.get_int(object_id, 10) as u32;
    let blend_op = stb_morph_object.get_int(object_id, 11) as u32;

    let object_transform = Transform::default()
        .with_translation(
            Vec3::new(
                object_instance.position.x,
                object_instance.position.z,
                -object_instance.position.y,
            ) / 100.0
                + Vec3::new(5200.0, 0.0, -5200.0),
        )
        .with_rotation(Quat::from_xyzw(
            object_instance.rotation.x,
            object_instance.rotation.z,
            -object_instance.rotation.y,
            object_instance.rotation.w,
        ))
        .with_scale(Vec3::new(
            object_instance.scale.x,
            object_instance.scale.z,
            object_instance.scale.y,
        ));

    let mesh_path_str = mesh_path.clone();
    let texture_path_str = texture_path.clone();
    let motion_path_str = motion_path.clone();
    
    let mesh: Handle<Mesh> = asset_server.load(&mesh_path);
    let texture_handle = asset_server.load(&texture_path);
    let motion_path_buf = ZmoTextureAssetLoader::convert_path(&motion_path);
    let motion_texture_handle = asset_server.load(ZmoTextureAssetLoader::convert_path_texture(&motion_path));
    let motion_handle = asset_server.load(motion_path_buf.to_string_lossy().into_owned());
    
    // Log asset creation
    info!("[MEMORY TRACKING] Animated object mesh handle created: {}", mesh_path_str);
    info!("[MEMORY TRACKING] Animated object texture handle created: {}", texture_path_str);
    info!("[MEMORY TRACKING] Animated object motion texture handle created: {}",
        ZmoTextureAssetLoader::convert_path_texture(&motion_path));
    info!("[MEMORY TRACKING] Animated object motion handle created: {}",
        motion_path_buf.display());
    
    let material = effect_mesh_materials.add(EffectMeshMaterial {
        base_texture: Some(texture_handle),
        alpha_enabled,
        alpha_test: alpha_test_enabled,
        two_sided,
        z_test_enabled,
        z_write_enabled,
        src_blend_factor: decode_blend_factor(src_blend_factor),
        dst_blend_factor: decode_blend_factor(dst_blend_factor),
        blend_op: decode_blend_op(blend_op),
        animation_texture: Some(motion_texture_handle.clone()),
    });

    info!("[MEMORY TRACKING] Animated object material created with 3 textures (base, motion texture, motion)");

    let animated_entity = commands
        .spawn((
            ZoneObject::AnimatedObject(ZoneObjectAnimatedObject {
                mesh_path: mesh_path.to_string(),
                motion_path: motion_path.to_string(),
                texture_path: texture_path.to_string(),
            }),
            mesh,
            material,
            EffectMeshAnimationRenderState::default(),
            MeshAnimation::repeat(motion_handle, None),
            object_transform,
            NoFrustumCulling, // AABB culling is broken for mesh animations
            NotShadowCaster,
            GlobalTransform::default(),
            Visibility::Visible,  // Explicitly visible
            ViewVisibility::default(),
            InheritedVisibility::default(),
            AsyncCollider(ComputedColliderShape::TriMesh),
            CollisionGroups::new(COLLISION_GROUP_ZONE_OBJECT, COLLISION_FILTER_INSPECTABLE),
        ))
        .id();
    
    info!("[ASSET LIFECYCLE] Animated object entity spawned: {:?}", animated_entity);
    animated_entity
}

fn spawn_effect_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    vfs_resource: &VfsResource,
    effect_mesh_materials: &mut Assets<EffectMeshMaterial>,
    particle_materials: &mut Assets<ParticleMaterial>,
    effect_object: &IfoEffectObject,
    ifo_object_id: usize,
) -> Entity {
    let object = &effect_object.object;
    let object_transform = Transform::default()
        .with_translation(
            Vec3::new(object.position.x, object.position.z, -object.position.y) / 100.0
                + Vec3::new(5200.0, 0.0, -5200.0),
        )
        .with_rotation(Quat::from_xyzw(
            object.rotation.x,
            object.rotation.z,
            -object.rotation.y,
            object.rotation.w,
        ))
        .with_scale(Vec3::new(object.scale.x, object.scale.z, object.scale.y));

    let effect_path_str = effect_object.effect_path.path().to_string_lossy().to_string();
    info!("[ASSET LIFECYCLE] Spawning effect object: {}", effect_path_str);

    let effect_object_entity = commands
        .spawn((
            ZoneObject::EffectObject {
                ifo_object_id,
                effect_path: effect_object
                    .effect_path
                    .path()
                    .to_string_lossy()
                    .to_string(),
            },
            object_transform,
            GlobalTransform::from(object_transform),
            Visibility::Visible,  // Explicitly visible
            ViewVisibility::default(),
            InheritedVisibility::default(),
        ))
        .id();
    
    info!("[ASSET LIFECYCLE] Effect object entity spawned: {:?}", effect_object_entity);

    spawn_effect(
        &vfs_resource.vfs,
        commands,
        asset_server,
        particle_materials,
        effect_mesh_materials,
        (&effect_object.effect_path).into(),
        false,
        Some(effect_object_entity),
    );

    effect_object_entity
}

fn spawn_sound_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    sound_object: &IfoSoundObject,
    ifo_object_id: usize,
) -> Entity {
    let object = &sound_object.object;
    let object_transform = Transform::default()
        .with_translation(
            Vec3::new(object.position.x, object.position.z, -object.position.y) / 100.0
                + Vec3::new(5200.0, 0.0, -5200.0),
        )
        .with_rotation(Quat::from_xyzw(
            object.rotation.x,
            object.rotation.z,
            -object.rotation.y,
            object.rotation.w,
        ))
        .with_scale(Vec3::new(object.scale.x, object.scale.z, object.scale.y));

    let sound_path_str = sound_object.sound_path.path().to_string_lossy().to_string();
    info!("[ASSET LIFECYCLE] Spawning sound object: {}", sound_path_str);

    let effect_object_entity = commands
        .spawn((
            ZoneObject::SoundObject {
                ifo_object_id,
                sound_path: sound_path_str.clone(),
            },
            SpatialSound::new_repeating(asset_server.load(&sound_path_str)),
            SoundRadius::new(sound_object.range as f32 / 10.0),
            object_transform,
            GlobalTransform::from(object_transform),
            Visibility::Visible,  // Explicitly visible
            ViewVisibility::default(),
            InheritedVisibility::default(),
        ))
        .id();
    
    info!("[ASSET LIFECYCLE] Sound object entity spawned: {:?}", effect_object_entity);
    effect_object_entity
}
