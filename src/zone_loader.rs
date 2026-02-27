use std::{
    collections::HashSet,
    future::Future,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock, mpsc},
    time::{Duration, Instant},
};

/// Memory monitoring for zone transitions
/// Tracks resident and virtual memory using Windows API
#[cfg(target_os = "windows")]
pub mod memory_monitor {
    use std::mem;
    use std::time::{Instant, Duration};
    
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct MemoryStatusEx {
        pub dwLength: u32,
        pub dwMemoryLoad: u32,
        pub ullTotalPhys: u64,
        pub ullAvailPhys: u64,
        pub ullTotalPageFile: u64,
        pub ullAvailPageFile: u64,
        pub ullTotalVirtual: u64,
        pub ullAvailVirtual: u64,
        pub ullAvailExtendedVirtual: u64,
    }
    
    impl MemoryStatusEx {
        pub fn new() -> Self {
            let mut status = unsafe { mem::zeroed::<MemoryStatusEx>() };
            status.dwLength = mem::size_of::<MemoryStatusEx>() as u32;
            status
        }
    }
    
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct ProcessMemoryCountersEx {
        pub cb: u32,
        pub PageFaultCount: u32,
        pub PeakWorkingSetSize: usize,
        pub WorkingSetSize: usize,
        pub QuotaPeakPagedPoolUsage: usize,
        pub QuotaPagedPoolUsage: usize,
        pub QuotaPeakNonPagedPoolUsage: usize,
        pub QuotaNonPagedPoolUsage: usize,
        pub PagefileUsage: usize,
        pub PeakPagefileUsage: usize,
        pub PrivateUsage: usize,
    }
    
    impl ProcessMemoryCountersEx {
        pub fn new() -> Self {
            let mut counters = unsafe { mem::zeroed::<ProcessMemoryCountersEx>() };
            counters.cb = mem::size_of::<ProcessMemoryCountersEx>() as u32;
            counters
        }
    }
    
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalMemoryStatusEx(lpBuffer: *mut MemoryStatusEx) -> i32;
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn GetProcessMemoryInfo(
            Process: *mut std::ffi::c_void,
            ppsmemCounters: *mut ProcessMemoryCountersEx,
            cb: u32,
        ) -> i32;
    }
    
    /// Formats bytes into human-readable string
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        if bytes == 0 {
            return "0 B".to_string();
        }
        let exp = (bytes as f64).log(1024.0).min(UNITS.len() as f64 - 1.0) as usize;
        let value = bytes as f64 / 1024_f64.powi(exp as i32);
        if exp == 0 {
            format!("{} {}", bytes, UNITS[exp])
        } else {
            format!("{:.2} {}", value, UNITS[exp])
        }
    }
    
    /// System memory information
    #[derive(Debug, Clone)]
    pub struct SystemMemoryInfo {
        pub memory_load_percent: u32,
        pub total_physical: u64,
        pub available_physical: u64,
        pub total_virtual: u64,
        pub available_virtual: u64,
    }
    
    impl SystemMemoryInfo {
        pub fn used_physical(&self) -> u64 {
            self.total_physical.saturating_sub(self.available_physical)
        }
        
        pub fn used_virtual(&self) -> u64 {
            self.total_virtual.saturating_sub(self.available_virtual)
        }
    }
    
    /// Process memory information
    #[derive(Debug, Clone)]
    pub struct ProcessMemoryInfo {
        pub working_set_size: usize,      // Resident memory (RAM)
        pub peak_working_set_size: usize, // Peak resident memory
        pub pagefile_usage: usize,        // Virtual memory committed
        pub peak_pagefile_usage: usize,   // Peak virtual memory committed
        pub private_usage: usize,         // Private bytes
    }
    
    /// Get current system memory status
    pub fn get_system_memory() -> Option<SystemMemoryInfo> {
        let mut status = MemoryStatusEx::new();
        unsafe {
            if GlobalMemoryStatusEx(&mut status) != 0 {
                Some(SystemMemoryInfo {
                    memory_load_percent: status.dwMemoryLoad,
                    total_physical: status.ullTotalPhys,
                    available_physical: status.ullAvailPhys,
                    total_virtual: status.ullTotalVirtual,
                    available_virtual: status.ullAvailVirtual,
                })
            } else {
                None
            }
        }
    }
    
    /// Get current process memory usage
    pub fn get_process_memory() -> Option<ProcessMemoryInfo> {
        let mut counters = ProcessMemoryCountersEx::new();
        unsafe {
            let process = GetCurrentProcess();
            if GetProcessMemoryInfo(process, &mut counters, counters.cb) != 0 {
                Some(ProcessMemoryInfo {
                    working_set_size: counters.WorkingSetSize,
                    peak_working_set_size: counters.PeakWorkingSetSize,
                    pagefile_usage: counters.PagefileUsage,
                    peak_pagefile_usage: counters.PeakPagefileUsage,
                    private_usage: counters.PrivateUsage,
                })
            } else {
                None
            }
        }
    }
    
    /// Log current memory status with context
    pub fn log_memory_status(context: &str) {
        log::info!("[MEMORY MONITOR] ==========================================");
        log::info!("[MEMORY MONITOR] Memory Status: {}", context);
        log::info!("[MEMORY MONITOR] ==========================================");
        
        if let Some(sys) = get_system_memory() {
            log::info!("[MEMORY MONITOR] System Memory:");
            log::info!("[MEMORY MONITOR]   Load: {}%", sys.memory_load_percent);
            log::info!("[MEMORY MONITOR]   Physical: {} / {} (used)",
                format_bytes(sys.used_physical()), format_bytes(sys.total_physical));
            log::info!("[MEMORY MONITOR]   Virtual:  {} / {} (used)",
                format_bytes(sys.used_virtual()), format_bytes(sys.total_virtual));
        }
        
        if let Some(proc) = get_process_memory() {
            log::info!("[MEMORY MONITOR] Process Memory:");
            log::info!("[MEMORY MONITOR]   Resident (RAM):      {} (peak: {})",
                format_bytes(proc.working_set_size as u64),
                format_bytes(proc.peak_working_set_size as u64));
            log::info!("[MEMORY MONITOR]   Virtual (committed): {} (peak: {})",
                format_bytes(proc.pagefile_usage as u64),
                format_bytes(proc.peak_pagefile_usage as u64));
            log::info!("[MEMORY MONITOR]   Private bytes:       {}",
                format_bytes(proc.private_usage as u64));
        }
        
        log::info!("[MEMORY MONITOR] ==========================================");
    }
    
    /// Memory snapshot for comparison
    #[derive(Debug, Clone)]
    pub struct MemorySnapshot {
        pub timestamp: Instant,
        pub process: ProcessMemoryInfo,
        pub system: SystemMemoryInfo,
        pub context: String,
    }
    
    impl MemorySnapshot {
        pub fn capture(context: &str) -> Option<Self> {
            let process = get_process_memory()?;
            let system = get_system_memory()?;
            Some(Self {
                timestamp: Instant::now(),
                process,
                system,
                context: context.to_string(),
            })
        }
        
        /// Compare with another snapshot and log differences
        pub fn compare_and_log(&self, other: &MemorySnapshot) {
            let duration = other.timestamp.duration_since(self.timestamp);
            
            let resident_delta = other.process.working_set_size as i64 - self.process.working_set_size as i64;
            let virtual_delta = other.process.pagefile_usage as i64 - self.process.pagefile_usage as i64;
            let private_delta = other.process.private_usage as i64 - self.process.private_usage as i64;
            
            log::info!("[MEMORY MONITOR] ==========================================");
            log::info!("[MEMORY MONITOR] Memory Delta: {} → {} (over {:?})",
                self.context, other.context, duration);
            log::info!("[MEMORY MONITOR] ==========================================");
            log::info!("[MEMORY MONITOR] Resident Memory: {:+} bytes ({:+.2} MB)",
                resident_delta, resident_delta as f64 / (1024.0 * 1024.0));
            log::info!("[MEMORY MONITOR] Virtual Memory:  {:+} bytes ({:+.2} MB)",
                virtual_delta, virtual_delta as f64 / (1024.0 * 1024.0));
            log::info!("[MEMORY MONITOR] Private Bytes:   {:+} bytes ({:+.2} MB)",
                private_delta, private_delta as f64 / (1024.0 * 1024.0));
            log::info!("[MEMORY MONITOR] ==========================================");
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod memory_monitor {
    use std::time::{Instant, Duration};
    
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        if bytes == 0 {
            return "0 B".to_string();
        }
        let exp = (bytes as f64).log(1024.0).min(UNITS.len() as f64 - 1.0) as usize;
        let value = bytes as f64 / 1024_f64.powi(exp as i32);
        if exp == 0 {
            format!("{} {}", bytes, UNITS[exp])
        } else {
            format!("{:.2} {}", value, UNITS[exp])
        }
    }
    
    pub fn log_memory_status(context: &str) {
        log::info!("[MEMORY MONITOR] Memory monitoring not available on this platform: {}", context);
    }
    
    #[derive(Debug, Clone)]
    pub struct MemorySnapshot {
        pub timestamp: Instant,
        pub context: String,
    }
    
    impl MemorySnapshot {
        pub fn capture(context: &str) -> Option<Self> {
            Some(Self {
                timestamp: Instant::now(),
                context: context.to_string(),
            })
        }
        
        pub fn compare_and_log(&self, other: &MemorySnapshot) {
            let duration = other.timestamp.duration_since(self.timestamp);
            log::info!("[MEMORY MONITOR] Time delta: {:?} ({} → {})",
                duration, self.context, other.context);
        }
    }
}

use memory_monitor::{MemorySnapshot, log_memory_status};
use bevy::prelude::{Query, Children, Without};
use uuid::Uuid;

use anyhow::Result;
use arrayvec::ArrayVec;
use    bevy::{
        asset::{Asset, AssetLoader, Assets, io::Reader, LoadContext, LoadState},
        ecs::system::SystemParam,
        math::{Quat, Vec2, Vec3},
        pbr::{ExtendedMaterial, NotShadowCaster, NotShadowReceiver, StandardMaterial},
        prelude::{
            AssetServer, Color, Commands, Entity, EventReader, EventWriter, GlobalTransform, Handle,
            Image, Local, Res, ResMut, Resource, Transform, UntypedHandle, Visibility, With,
            Mesh3d, MeshMaterial3d,
        },
        reflect::TypePath,
        render::{
            alpha::AlphaMode,
            mesh::{Indices, Mesh, PrimitiveTopology},
            primitives::Aabb,
            render_asset::RenderAssetUsages,
            view::{NoFrustumCulling, ViewVisibility, InheritedVisibility, RenderLayers},
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
        ColliderParent, EventObject, NightTimeEffect, WarpObject, WindSway, Zone, ZoneObject,
        ZoneObjectAnimatedObject, ZoneObjectId, ZoneObjectPart, ZoneObjectTerrain,
        COLLISION_FILTER_CLICKABLE, COLLISION_FILTER_COLLIDABLE, COLLISION_FILTER_INSPECTABLE,
        COLLISION_FILTER_MOVEABLE, COLLISION_GROUP_PHYSICS_TOY, COLLISION_GROUP_ZONE_EVENT_OBJECT,
        COLLISION_GROUP_ZONE_OBJECT, COLLISION_GROUP_ZONE_TERRAIN,
        COLLISION_GROUP_ZONE_WARP_OBJECT, COLLISION_GROUP_ZONE_WATER,
        WaterSpawnedEvent,
    },
    effect_loader::{decode_blend_factor, decode_blend_op, spawn_effect},
    events::{LoadZoneEvent, ZoneEvent, ZoneLoadedFromVfsEvent},
    map_editor::components::EditorSelectable,
    render::{
        MESH_ATTRIBUTE_UV_1, ParticleMaterial, RoseEffectExtension, RoseObjectExtension, TerrainMaterial,
        WaterMaterial,
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
            //info!("[MEMORY TRACKING] Mesh handle REUSE detected: {} (total duplicates: {})", 
                //path, self.duplicate_asset_requests);
        } else {
            //info!("[MEMORY TRACKING] Mesh handle created: {} (total meshes: {})", 
                //path, self.mesh_handles_created);
        }
    }

    /// Log when a material handle is created
    pub fn log_material_handle_created(&mut self, path: &str, texture_count: usize) {
        self.material_handles_created += 1;
        //info!("[MEMORY TRACKING] Material handle created: {} with {} textures (total materials: {})", 
            //path, texture_count, self.material_handles_created);
    }

    /// Log when a texture handle is created
    pub fn log_texture_handle_created(&mut self, path: &str) {
        self.texture_handles_created += 1;
        let is_duplicate = !self.unique_asset_paths.insert(path.to_string());
        if is_duplicate {
            self.duplicate_asset_requests += 1;
            //info!("[MEMORY TRACKING] Texture handle REUSE detected: {} (total duplicates: {})", 
               // path, self.duplicate_asset_requests);
        } else {
            //info!("[MEMORY TRACKING] Texture handle created: {} (total textures: {})", 
               // path, self.texture_handles_created);
        }
    }

    /// Log when an entity is spawned
    pub fn log_entity_spawned(&mut self, entity_type: &str, asset_count: usize) {
        self.entities_spawned += 1;
        //info!("[MEMORY TRACKING] Entity spawned: type={}, assets={} (total entities: {})", 
            //entity_type, asset_count, self.entities_spawned);
    }

    /// Log when an entity is despawned
    pub fn log_entity_despawned(&mut self) {
        self.entities_despawned += 1;
        //info!("[MEMORY TRACKING] Entity despawned (total despawned: {})", self.entities_despawned);
    }

    /// Log a summary of memory statistics
    pub fn log_summary(&mut self) {
        let now = Instant::now();
        let should_log = self.last_summary_time
            .map_or(true, |last| now.duration_since(last) >= Duration::from_secs(5));
        
        if should_log {
            self.last_summary_time = Some(now);
            //info!("[MEMORY TRACKING] ==========================================");
            //info!("[MEMORY TRACKING] MEMORY SUMMARY (every 5 seconds)");
            //info!("[MEMORY TRACKING] ==========================================");
            //info!("[MEMORY TRACKING] Mesh handles: {}", self.mesh_handles_created);
            //info!("[MEMORY TRACKING] Material handles: {}", self.material_handles_created);
            //info!("[MEMORY TRACKING] Texture handles: {}", self.texture_handles_created);
            //info!("[MEMORY TRACKING] Unique asset paths: {}", self.unique_asset_paths.len());
            //info!("[MEMORY TRACKING] Duplicate asset requests: {}", self.duplicate_asset_requests);
            //info!("[MEMORY TRACKING] Entities spawned: {}", self.entities_spawned);
            //info!("[MEMORY TRACKING] Entities despawned: {}", self.entities_despawned);
            //info!("[MEMORY TRACKING] Active entities: {}", self.entities_spawned - self.entities_despawned);
            
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
            
            //info!("[MEMORY TRACKING] ==========================================");
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
            log::info!("[ZONE LOADER ASSET LOADER] Zone ID parsed: {}", zone_id.get());

            log::info!("[ZONE LOADER ASSET LOADER] Calling load_zone...");
            let result = load_zone(zone_id, load_context).await;
            log::info!("[ZONE LOADER ASSET LOADER] load_zone completed with result: {:?}", result.is_ok());
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
                log::info!("[VFS PRIORITY] Loaded from real filesystem: {} ({} bytes)",
                    path_str, memory_monitor::format_bytes(data.len() as u64));
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
        Err(e) => Err(anyhow::anyhow!("Failed to open VFS file {}: {:?}", path_str, e)),
    }
}

async fn load_zone_direct(zone_id: ZoneId, vfs: &VirtualFilesystem, base_path: &Path) -> Result<ZoneLoaderAsset, anyhow::Error> {
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
        Ok(data) => {
            RoseFile::read(RoseFileReader::from(&data), &Default::default())
                .map_err(|e| anyhow::anyhow!("Failed to parse ZON file: {:?}", e))?
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to load ZON file: {:?}", e));
        }
    };
    //log::info!("[ZONE LOADER DIRECT] ZON file loaded successfully");

    //log::info!("[ZONE LOADER DIRECT] Loading ZSC constant file: {:?}", zsc_cnst_path);
    // PRIORITY: Real filesystem takes priority over VFS
    let zsc_cnst: ZscFile = match read_bytes_with_priority_sync(vfs, base_path, &zsc_cnst_path) {
        Ok(data) => {
            RoseFile::read(RoseFileReader::from(&data), &Default::default())
                .map_err(|e| anyhow::anyhow!("Failed to parse ZSC constant file: {:?}", e))?
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to load ZSC constant file: {:?}", e));
        }
    };
    //log::info!("[ZONE LOADER DIRECT] ZSC constant file loaded successfully");

    //log::info!("[ZONE LOADER DIRECT] Loading ZSC deco file: {:?}", zsc_deco_path);
    // PRIORITY: Real filesystem takes priority over VFS
    let zsc_deco: ZscFile = match read_bytes_with_priority_sync(vfs, base_path, &zsc_deco_path) {
        Ok(data) => {
            RoseFile::read(RoseFileReader::from(&data), &Default::default())
                .map_err(|e| anyhow::anyhow!("Failed to parse ZSC deco file: {:?}", e))?
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to load ZSC deco file: {:?}", e));
        }
    };
    //log::info!("[ZONE LOADER DIRECT] ZSC deco file loaded successfully");

    let zone_path = zon_file_path_buf
        .parent()
        .unwrap_or_else(|| Path::new(""));

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
            match load_block_files_direct(vfs, base_path, zone_path, block_x, block_y).await {
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
///
/// IMPORTANT: Real filesystem takes priority over VFS to support map editor modifications.
/// Files are checked at base_path first, then VFS is used as fallback.
async fn load_block_files_direct(
    vfs: &VirtualFilesystem,
    base_path: &Path,
    zone_path: &Path,
    block_x: usize,
    block_y: usize,
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
                    log::info!("[VFS PRIORITY] Loaded from real filesystem: {} ({} bytes)", 
                        path_str, memory_monitor::format_bytes(data.len() as u64));
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
            Err(e) => Err(anyhow::anyhow!("Failed to open VFS file {}: {:?}", path_str, e)),
        }
    }
    
    let him_path_buf = zone_path.join(format!("{}_{}.HIM", block_x, block_y));
    let him_path_str = him_path_buf.to_string_lossy().replace('\\', "/");
    let him_path = VfsPath::from(PathBuf::from(&him_path_str));

    // Check if HIM file exists before attempting to load it
    match vfs.open_file(&him_path) {
        Ok(_) => {}
        Err(_) => {
            return Err(anyhow::anyhow!("HIM file not found for block {}_{}", block_x, block_y));
        }
    }

    // Load and parse HIM file
    let him: HimFile = match read_bytes_with_priority(vfs, base_path, &him_path) {
        Ok(data) => {
            RoseFile::read(RoseFileReader::from(&data), &Default::default())
                .map_err(|e| anyhow::anyhow!("Failed to parse HIM file for block {}_{}: {:?}", block_x, block_y, e))?
        }
        Err(e) => {
            log::warn!("[LOAD BLOCK DIRECT] Failed to load HIM file for block {}_{}: {:?}. Skipping this block.", block_x, block_y, e);
            return Err(anyhow::anyhow!("HIM file not found for block {}_{}", block_x, block_y));
        }
    };

    // Load and parse TIL file (optional)
    let til_path_str = zone_path.join(format!("{}_{}.TIL", block_x, block_y)).to_string_lossy().replace('\\', "/");
    let til_path = VfsPath::from(PathBuf::from(&til_path_str));
    let til: Option<TilFile> = match read_bytes_with_priority(vfs, base_path, &til_path) {
        Ok(data) => {
            RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok()
        }
        Err(_) => None
    };

    // Load and parse IFO file (optional)
    let ifo_path = VfsPath::from(zone_path.join(format!("{}_{}.IFO", block_x, block_y)));
    let ifo: Option<IfoFile> = match read_bytes_with_priority(vfs, base_path, &ifo_path) {
        Ok(data) => {
            log::info!("[IFO LOADER] Loading IFO file for block {}_{} ({} bytes)", block_x, block_y, data.len());
            let result: Result<IfoFile, _> = RoseFile::read(RoseFileReader::from(&data), &Default::default());
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
                    log::error!("[IFO LOADER] Failed to parse IFO file for block {}_{}: {:?}", block_x, block_y, e);
                    None
                }
            }
        }
        Err(_) => {
            log::debug!("[IFO LOADER] No IFO file found for block {}_{}", block_x, block_y);
            None
        }
    };

    // Load and parse LIT constant file (optional)
    let lit_cnst_path_str = zone_path.join(format!(
        "{}_{}/LIGHTMAP/BUILDINGLIGHTMAPDATA.LIT",
        block_x, block_y
    )).to_string_lossy().replace('\\', "/");
    let lit_cnst_path = VfsPath::from(PathBuf::from(&lit_cnst_path_str));
    let lit_cnst: Option<LitFile> = match read_bytes_with_priority(vfs, base_path, &lit_cnst_path) {
        Ok(data) => {
            RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok()
        }
        Err(_) => None
    };

    // Load and parse LIT deco file (optional)
    let lit_deco_path_str = zone_path.join(format!(
        "{}_{}/LIGHTMAP/OBJECTLIGHTMAPDATA.LIT",
        block_x, block_y
    )).to_string_lossy().replace('\\', "/");
    let lit_deco_path = VfsPath::from(PathBuf::from(&lit_deco_path_str));
    let lit_deco: Option<LitFile> = match read_bytes_with_priority(vfs, base_path, &lit_deco_path) {
        Ok(data) => {
            RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok()
        }
        Err(_) => None
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

#[derive(SystemParam)]
pub struct SpawnZoneParams<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub asset_server: Res<'w, AssetServer>,
    pub game_data: Res<'w, GameData>,
    pub vfs_resource: Res<'w, VfsResource>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub specular_texture: Res<'w, SpecularTexture>,
    pub standard_materials: ResMut<'w, Assets<bevy::pbr::StandardMaterial>>,
    pub terrain_materials: ResMut<'w, Assets<TerrainMaterial>>,
    pub water_materials: ResMut<'w, Assets<WaterMaterial>>,
    pub effect_mesh_materials: ResMut<'w, Assets<ExtendedMaterial<StandardMaterial, RoseEffectExtension>>>,
    pub object_materials: ResMut<'w, Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>>,
    pub particle_materials: ResMut<'w, Assets<ParticleMaterial>>,
    pub storage_buffers: ResMut<'w, Assets<bevy::render::storage::ShaderStorageBuffer>>,
    pub zone_loader_assets: ResMut<'w, Assets<ZoneLoaderAsset>>,
    pub memory_tracking: ResMut<'w, MemoryTrackingResource>,
    pub water_spawned_events: EventWriter<'w, WaterSpawnedEvent>,
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
    /// Memory snapshot at the start of zone loading for comparison
    pub memory_snapshot_start: Option<MemorySnapshot>,
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
            matches!(asset_server.get_load_state(handle.id()), Some(LoadState::Loaded))
        });
        
        if !all_loaded {
            let loaded_count = self.zone_assets.iter()
                .filter(|h| matches!(asset_server.get_load_state(h.id()), Some(LoadState::Loaded)))
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

    // log::info!("[ZONE LOADER SYSTEM] ===========================================");
    // log::info!("[ZONE LOADER SYSTEM] zone_loader_system called");
    // log::info!("[ZONE LOADER SYSTEM] Loading zones in queue: {}", loading_zones.len());
    // log::info!("[ZONE LOADER SYSTEM] ===========================================");

    // Log periodic memory summary
    spawn_zone_params.memory_tracking.log_summary();

    // Check for loaded zones from async tasks via channel
    // log::info!("[ZONE LOADER SYSTEM] Checking channel for loaded zones...");
    let mut received_count = 0;
    while let Ok((zone_id, zone_asset_result)) = zone_load_receiver.0.lock().unwrap().try_recv() {
        received_count += 1;
        // log::info!("[ZONE LOADER SYSTEM] Received {} zone(s) from channel this frame", received_count);
        let zone_id: ZoneId = zone_id;
        let zone_asset_result: Result<ZoneLoaderAsset, anyhow::Error> = zone_asset_result;
        match zone_asset_result {
            Ok(zone_asset) => {
                // log::info!("[ZONE LOADER SYSTEM] ===========================================");
                // log::info!("[ZONE LOADER SYSTEM] Zone {} loaded from async task, sending ZoneLoadedFromVfsEvent", zone_id.get());
                // log::info!("[ZONE LOADER SYSTEM] ===========================================");

                // Remove the zone from the loading queue since it's now received from channel
                if let Some(pos) = loading_zones.iter().position(|lz| {
                    lz.loading_via_async_task && lz.zone_id == Some(zone_id)
                }) {
                    // log::info!("[ZONE LOADER SYSTEM] Removing zone {} from loading queue (received from channel)", zone_id.get());
                    loading_zones.remove(pos);
                } else {
                    log::warn!("[ZONE LOADER SYSTEM] Could not find zone {} in loading queue to remove", zone_id.get());
                }

                // CRITICAL FIX: Add the zone asset to the Assets collection HERE where we have ownership
                // This allows collision_player_system to access terrain height data
                let zone_handle = spawn_zone_params.zone_loader_assets.add(zone_asset);
                log::info!("[ZONE LOADER SYSTEM] Zone {} added to Assets collection with handle: {:?}", zone_id.get(), zone_handle);

                // Send event with the handle (not the Arc) to zone_loaded_from_vfs_system for spawning
                zone_loaded_from_vfs_events.write(ZoneLoadedFromVfsEvent::new(zone_id, zone_handle));
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
                    // log::info!("[ZONE LOADER SYSTEM] Removing failed zone {} from loading queue", zone_id.get());
                    loading_zones.remove(pos);
                }
            }
        }
    }
    
    if received_count == 0 {
        // log::info!("[ZONE LOADER SYSTEM] No zones received from channel this frame");
    }

    if zone_loader_cache.cache.is_empty() {
        zone_loader_cache
            .cache
            .resize_with(spawn_zone_params.game_data.zone_list.len(), || None);
    }

    for event in load_zone_events.read() {
        // DIAGNOSTIC: Track LoadZoneEvent received
        log::info!("[ZONE LOADER SYSTEM DIAGNOSTIC] LoadZoneEvent received: zone_id={}, despawn_other_zones={}",
            event.id.get(), event.despawn_other_zones);
        
        let zone_index = event.id.get() as usize;

        // Memory tracking: Log cache state
        let cached_zones = zone_loader_cache.cache.iter().filter(|z| z.is_some()).count();
        let spawned_zones = zone_loader_cache.cache.iter().filter(|z| z.is_some() && z.as_ref().unwrap().spawned_entity.is_some()).count();
        log::info!("[MEMORY] Cache state: {} zones cached, {} spawned", cached_zones, spawned_zones);

        // log::info!("[ZONE LOADER SYSTEM] ===========================================");
        // log::info!("[ZONE LOADER SYSTEM] LoadZoneEvent received for zone_id: {}", event.id.get());
        // log::info!("[ZONE LOADER SYSTEM] Despawn other zones: {}", event.despawn_other_zones);
        // log::info!("[ZONE LOADER SYSTEM] ===========================================");

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
                        // log::info!("[ZONE LOADER SYSTEM] Despawning existing zone {} entity {:?} as requested",
                            //event.id.get(), entity);
                        spawn_zone_params.commands.entity(entity).despawn();
                    }
                }
                zone_loader_cache.cache[zone_index] = None;
            } else {
                // Skip if already loaded and not despawning
                continue;
            }
        }

        if zone_loader_cache.cache.get(zone_index).map(|c| c.is_none()).unwrap_or(true) {
            // log::info!("[ZONE LOADER SYSTEM] Zone not cached, loading directly from VFS");
            
            // WORKAROUND: Load zone directly from VFS without using AssetServer
            // This bypasses the broken asset loading pipeline in Bevy 0.13.2
            let zone_id = event.id;
            let vfs = spawn_zone_params.vfs_resource.vfs.clone();
            let base_path = spawn_zone_params.vfs_resource.base_path.clone();
            let tx = zone_load_sender.0.clone();
            
            // log::info!("[ZONE LOADER SYSTEM] ===========================================");
            // log::info!("[ZONE LOADER SYSTEM] Preparing to spawn async task for zone {}", zone_id.get());
            
            // Check if pool is initialized and get reference
            let pool = match AsyncComputeTaskPool::try_get() {
                Some(pool) => {
                    // log::info!("[ZONE LOADER SYSTEM] AsyncComputeTaskPool is available, spawning async task");
                    pool
                }
                None => {
                    log::error!("[ZONE LOADER SYSTEM] AsyncComputeTaskPool is NOT initialized! Cannot spawn async task!");
                    log::error!("[ZONE LOADER SYSTEM] This is likely why zones are not loading!");
                    // DO NOT spawn the task - skip this zone and continue to next
                    continue;
                }
            };

            // log::info!("[ZONE LOADER SYSTEM] Spawning async task to load zone {}", zone_id.get());
            // log::info!("[ZONE LOADER SYSTEM] ===========================================");

            // Spawn async task to load zone using AsyncComputeTaskPool
            // This is more appropriate for computational tasks like loading zones
            let task = pool.spawn(async move {
                // log::info!("[ZONE LOADER DIRECT TASK] ===========================================");
                // log::info!("[ZONE LOADER DIRECT TASK] Async task started for zone_id: {}", zone_id.get());
                // log::info!("[ZONE LOADER DIRECT TASK] ===========================================");
                
                match load_zone_direct(zone_id, &vfs, &base_path).await {
                    Ok(zone_asset) => {
                        // log::info!("[ZONE LOADER DIRECT TASK] ===========================================");
                        // log::info!("[ZONE LOADER DIRECT TASK] Zone loaded successfully: {}", zone_id.get());
                        // log::info!("[ZONE LOADER DIRECT TASK] Sending zone through channel...");
                        // log::info!("[ZONE LOADER DIRECT TASK] ===========================================");
                        
                        match tx.send((zone_id, Ok(zone_asset))) {
                            Ok(_) => {
                                // log::info!("[ZONE LOADER DIRECT TASK] Zone sent through channel successfully!");
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
                                // log::info!("[ZONE LOADER DIRECT TASK] Error sent through channel successfully!");
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
            
            // log::info!("[ZONE LOADER SYSTEM] Async task spawned and detached for zone {}", zone_id.get());
            // log::info!("[ZONE LOADER SYSTEM] ===========================================");

            // Add zone to loading queue to track that it's being loaded
            // This ensures we know the zone is in progress even though spawning is handled by zone_loaded_from_vfs_system
            // MEMORY MONITOR: Capture baseline memory before starting zone load
            let memory_snapshot = MemorySnapshot::capture(&format!("Zone {} loading start", zone_id.get()));
            
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
                memory_snapshot_start: memory_snapshot,
            });
            // log::info!("[ZONE LOADER SYSTEM] Zone queued for async loading. Total loading zones: {}", loading_zones.len());
        } else if let Some(zone_entity) = zone_loader_cache.cache[zone_index]
            .as_ref()
            .and_then(|cached_zone| cached_zone.spawned_entity)
        {
            // Zone is already spawned
            // log::info!("[ZONE LOADER SYSTEM] Zone already spawned, sending Loaded event");
            zone_events.write(ZoneEvent::Loaded(event.id));
            debug_inspector_state.entity = Some(zone_entity);
            continue;
        } else {
            // log::info!("[ZONE LOADER SYSTEM] Zone cached but not spawned, using cached handle");
            
            let cached_zone = zone_loader_cache.cache[zone_index].as_ref().unwrap();
            // MEMORY MONITOR: Capture baseline memory before starting zone load (cached zone path)
            let memory_snapshot = MemorySnapshot::capture(&format!("Zone {} loading start (cached)", event.id.get()));
            
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
                memory_snapshot_start: memory_snapshot,
            });
            // log::info!("[ZONE LOADER SYSTEM] LoadingZone added to queue. Total loading zones: {}", loading_zones.len());
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

                    // log::info!("[ZONE LOADER SYSTEM] Zone {} loading via async task, keeping in queue and waiting for channel",
                        //zone_path);
                    index += 1;
                    continue;
                } else {
                    // Zone is loading via AssetServer - check LoadState
                    let zone_path = loading_zone.handle.path().map(|p| p.to_string()).unwrap_or_else(|| "unknown".to_string());
                    // log::info!("[ZONE LOADER SYSTEM] Checking LoadState for zone {} (AssetServer)", zone_path);
                    
                    match spawn_zone_params.asset_server.get_load_state(&loading_zone.handle) {
                        Some(LoadState::NotLoaded) | Some(LoadState::Loading) => {
                            // log::info!("[ZONE LOADER SYSTEM] Zone {} still loading (LoadState: {:?}), keeping in queue", 
                                //zone_path, spawn_zone_params.asset_server.get_load_state(&loading_zone.handle));
                            index += 1;
                        }
                        Some(LoadState::Loaded) => {
                            // log::info!("[ZONE LOADER SYSTEM] Zone {} loaded, transitioning to Spawned state", zone_path);
                            loading_zone.state = LoadingZoneState::Spawned;
                            index += 1;
                        }
                        None | Some(LoadState::Failed(_)) => {
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
                // DIAGNOSTIC: Zone transitioning to Spawned state
                log::info!("[ZONE LOADER SYSTEM DIAGNOSTIC] LoadingZone transitioning to Spawned state for zone");
                
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
                    // log::info!("[ZONE LOADER SYSTEM] Despawning other zones");
                    for cached_zone in zone_loader_cache
                        .cache
                        .iter_mut()
                        .filter_map(|x| x.as_mut())
                    {
                        if let Some(spawned_entity) = cached_zone.spawned_entity.take()
                        {
                           // info!("[ASSET LIFECYCLE] Despawning zone entity: {:?}", spawned_entity);
                            log::warn!("[ZONE LOADER SYSTEM DIAGNOSTIC] ✗ Despawning existing zone entity: entity={:?}", spawned_entity);
                            spawn_zone_params
                                    .commands
                                    .entity(spawned_entity)
                                    .despawn();
                            spawn_zone_params.memory_tracking.log_entity_despawned();
                        }
                    }

                    spawn_zone_params.commands.remove_resource::<CurrentZone>();
                }

                // DIAGNOSTIC: About to spawn zone from zone_loader_system
                log::info!("[ZONE LOADER SYSTEM DIAGNOSTIC] About to call spawn_zone for zone_id={}",
                    zone_id.get());
                
                // Get zone_data and spawn
                let zone_handle_clone = zone_handle.clone();
                let spawn_result = {
                    let zone_data_opt = spawn_zone_params.zone_loader_assets.get(&zone_handle_clone);
                    
                    if let Some(zone_data) = zone_data_opt {
                        // log::info!("[ZONE LOADER SYSTEM] Zone data retrieved, starting spawn process");
                        // log::info!("[ZONE LOADER SYSTEM] Calling spawn_zone()");
                        // Extract the data we need before the borrow ends
                        let zone_id = zone_data.zone_id;
                        let zone_path = zone_data.zone_path.clone();
                        let blocks_len = zone_data.blocks.len();
                        let npcs_len = zone_data.npcs.len();

                        // log::info!("[ZONE LOADER SYSTEM] Spawning zone: id={}, path={}, blocks={}, npcs={}",
                            //zone_id.get(), zone_path.display(), blocks_len, npcs_len);

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
                            // log::info!("[ZONE LOADER SYSTEM] Zone spawned successfully");
                            
                            // DIAGNOSTIC: Zone entity successfully spawned from zone_loader_system
                            log::info!("[ZONE LOADER SYSTEM DIAGNOSTIC] ✓ Zone entity created in zone_loader_system: entity={:?}, zone_id={}",
                                zone_entity, zone_id.get());
                            
                            // Check if assets are empty before moving
                            let assets_empty = zone_loading_assets.is_empty();
                            
                            // Update cache with spawned entity
                            let zone_index = zone_id.get() as usize;
                            if let Some(cached_zone) = zone_loader_cache.cache[zone_index].as_mut() {
                                cached_zone.spawned_entity = Some(zone_entity);
                            }
                            
                            loading_zone.zone_assets = zone_loading_assets;
                            loading_zone.state = LoadingZoneState::Spawned;

                            // CRITICAL FIX: Set CurrentZone resource (was missing in Bevy 0.13 implementation)
                            // This matches Bevy 0.11 behavior (lines 482-485)
                            spawn_zone_params.commands.insert_resource(CurrentZone {
                                id: zone_id,
                                handle: zone_handle_clone,
                            });

                            if assets_empty {
                                // log::info!("[ZONE LOADER SYSTEM] No additional assets to load, sending Loaded event");

                                // MEMORY LEAK FIX: Clear asset handles before removing zone
                                loading_zone.clear_asset_handles();

                                zone_events.write(ZoneEvent::Loaded(zone_id));
                                loading_zones.remove(index);
                            } else {
                                // log::info!("[ZONE LOADER SYSTEM] Waiting for additional assets to load");
                                index += 1;
                            }
                        }
                        Err(e) => {
                            log::error!("[ZONE LOADER SYSTEM] Failed to spawn zone: {:?}", e);
                            
                            // DIAGNOSTIC: Zone entity spawn failed in zone_loader_system
                            log::error!("[ZONE LOADER SYSTEM DIAGNOSTIC] ✗ spawn_zone FAILED in zone_loader_system for zone_id={}: error={:?}",
                                zone_id.get(), e);
                            
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
                        // log::info!("[ZONE LOADER SYSTEM] Zone ready after 2 frames, sending Loaded event");
                        
                        // MEMORY LEAK FIX: Clear asset handles before removing zone
                        loading_zone.clear_asset_handles();
                        
                        zone_events.write(ZoneEvent::Loaded(zone_data.zone_id));
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
    
    // Initialize cache if empty
    if zone_loader_cache.cache.is_empty() {
        zone_loader_cache
            .cache
            .resize_with(spawn_zone_params.game_data.zone_list.len(), || None);
    }
    
    // CRITICAL FIX: Check for already-loaded zones to prevent duplicates
    let already_loaded: std::collections::HashSet<u16> = existing_zones
        .iter()
        .map(|(_, zone)| zone.id.get())
        .collect();
    
    if !already_loaded.is_empty() {
        // log::info!("[ZONE LOADED FROM VFS] Currently loaded zones: {:?}",
        //    already_loaded.iter().collect::<Vec<_>>());
    }
    
    // log::info!("[ZONE LOADED FROM VFS] Processing {} zone events this frame", event_count);
    spawn_zone_params.memory_tracking.log_summary();
    
    let mut processed_count = 0;
    let mut success_count = 0;
    let mut failed_count = 0;
    let mut skipped_count = 0;
    
    // CRITICAL FIX: Deduplicate events - prevent duplicate zone IDs in same batch
    let mut seen_zone_ids: std::collections::HashSet<u16> = std::collections::HashSet::new();
    
    // Process events directly, skipping duplicates
    // DIAGNOSTIC: Track ZoneLoadedFromVfsEvent processing
    // log::info!("[ZONE LOADED FROM VFS DIAGNOSTIC] Processing {} ZoneLoadedFromVfsEvent(s)", events.len());
    
    for event in events.read() {
        // DIAGNOSTIC: Individual event details
        // log::info!("[ZONE LOADED FROM VFS DIAGNOSTIC] Processing event: zone_id={}", event.zone_id.get());
        
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
        // log::info!("[ZONE LOADED FROM VFS] ===========================================");
        // log::info!("[ZONE LOADED FROM VFS] Spawning zone {} from VFS (event {}/{})"
        //    , event.zone_id.get(), processed_count, event_count);
        // log::info!("[ZONE LOADED FROM VFS] ===========================================");
        
        let zone_index = event.zone_id.get() as usize;

        // CRITICAL FIX: Handle despawn_other_zones flag (matching AssetServer path behavior)
        // Default to true to match the typical behavior when loading a new zone
        let despawn_other_zones = true;
        
        if despawn_other_zones {
            // log::info!("[ZONE LOADED FROM VFS] Despawning other zones");
            for cached_zone in zone_loader_cache
                .cache
                .iter_mut()
                .filter_map(|x| x.as_mut())
            {
                if let Some(spawned_entity) = cached_zone.spawned_entity.take()
                {
                    log::warn!("[ZONE LOADED FROM VFS DIAGNOSTIC] ✗ Despawning existing zone entity: entity={:?}", spawned_entity);
                    spawn_zone_params
                            .commands
                            .entity(spawned_entity)
                            .despawn();
                    spawn_zone_params.memory_tracking.log_entity_despawned();
                }
            }

            spawn_zone_params.commands.remove_resource::<CurrentZone>();
        }
        
        // CRITICAL FIX: The zone asset was already added to the Assets collection in zone_loader_system
        // We just need to use the handle from the event to spawn the zone
        let zone_handle = event.zone_handle.clone();
        // log::info!("[ZONE LOADED FROM VFS] Using zone handle from event: {:?}", zone_handle);
        
        // Spawn the zone using the asset from the collection (via handle)
        // DIAGNOSTIC: About to call spawn_zone
        // log::info!("[ZONE LOADED FROM VFS DIAGNOSTIC] About to call spawn_zone for zone_id={}",
        //    event.zone_id.get());
        
        // Use raw pointer to work around borrow checker (same pattern as zone_loader_system line 1510-1516)
        // This is safe because spawn_zone doesn't modify zone_loader_assets
        let zone_asset_ref = match spawn_zone_params.zone_loader_assets.get(&zone_handle) {
            Some(asset) => asset,
            None => {
                log::error!("[ZONE LOADED FROM VFS] Zone asset not found in collection for handle: {:?}", zone_handle);
                failed_count += 1;
                continue;
            }
        };
        let zone_data_ptr: *const ZoneLoaderAsset = zone_asset_ref;
        let spawn_zone_params_ptr: *mut SpawnZoneParams = &mut spawn_zone_params;
        
        let spawn_result = unsafe {
            let zone_data_ref: &ZoneLoaderAsset = &*zone_data_ptr;
            let spawn_zone_params_ref: &mut SpawnZoneParams = &mut *spawn_zone_params_ptr;
            spawn_zone(spawn_zone_params_ref, zone_data_ref)
        };
        
        match spawn_result {
            Ok((entity, _zone_assets)) => {
                success_count += 1;
                // log::info!("[ZONE LOADED FROM VFS] Zone {} spawned successfully! entity={:?}",
                //    event.zone_id.get(), entity);
                
                // DIAGNOSTIC: Zone entity successfully created and returned
                // log::info!("[ZONE LOADED FROM VFS DIAGNOSTIC] ✓ Zone entity created in zone_loaded_from_vfs_system: entity={:?}, zone_id={}",
                //    entity, event.zone_id.get());

                // CRITICAL FIX: Cache VFS-loaded zones with the REAL handle (not placeholder)
                // The spawned_entity is what matters for despawning; handle is used for terrain height lookups
                zone_loader_cache.cache[zone_index] = Some(CachedZone {
                    data_handle: zone_handle.clone(),
                    spawned_entity: Some(entity),
                });

                // CRITICAL FIX: Set CurrentZone resource with the REAL handle
                // This allows collision_player_system to access zone data via zone_loader_assets.get(&current_zone.handle)
                spawn_zone_params.commands.insert_resource(CurrentZone {
                    id: event.zone_id,
                    handle: zone_handle,
                });
                
                // Send loaded event
                zone_events.write(ZoneEvent::Loaded(event.zone_id));
                
                // Update debug inspector
                debug_inspector_state.entity = Some(entity);
                
                // Log memory summary after zone spawn
                // MEMORY MONITOR: Log memory status after zone spawn completes
                log_memory_status(&format!("Zone {} spawned successfully", event.zone_id.get()));
            }
            Err(e) => {
                failed_count += 1;
                log::error!("[ZONE LOADED FROM VFS] Failed to spawn zone {}: {:?}",
                    event.zone_id.get(), e);
                
                // DIAGNOSTIC: Zone entity spawn failed
                log::error!("[ZONE LOADED FROM VFS DIAGNOSTIC] ✗ spawn_zone FAILED for zone_id={}: error={:?}",
                    event.zone_id.get(), e);
            }
        }
    }
    
    // log::info!("[ZONE LOADED FROM VFS] ===========================================");
    // log::info!("[ZONE LOADED FROM VFS] Processing complete: {} success, {} failed, {} skipped (duplicates) out of {}",
    //    success_count, failed_count, skipped_count, processed_count);
    spawn_zone_params.memory_tracking.log_summary();
    
    // MEMORY MONITOR: Log final memory status after all VFS zone processing
    if processed_count > 0 {
        log_memory_status("Zone loading batch complete");
    }
    
    // log::info!("[ZONE LOADED FROM VFS] ===========================================");
}

pub fn force_zone_visibility_system(
    mut zone_query: Query<&mut Visibility, With<Zone>>,
) {
    for mut visibility in zone_query.iter_mut() {
        if *visibility != Visibility::Visible {
            log::info!("[FORCE VISIBILITY] Forcing Zone to Visibility::Visible");
            *visibility = Visibility::Visible;
        }
    }
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
    
    // DIAGNOSTIC: Track function entry
    log::info!("[SPAWN ZONE DIAGNOSTIC] spawn_zone function ENTRY - about to spawn zone entity");

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
        standard_materials,
        terrain_materials,
        water_materials,
        effect_mesh_materials,
        object_materials,
        particle_materials,
        storage_buffers,
        zone_loader_assets: _,
        memory_tracking,
        ref mut water_spawned_events,
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
        // Use custom WaterMaterial with animated texture array support
        // Note: Uses default lighting values since we can't access zone_lighting (bind group 3)
        let material = water_materials.add(WaterMaterial {
            textures: water_material_textures,
            ..Default::default()
        });
        log::info!("[SPAWN ZONE] Water material created with {} textures (animated)", texture_count);
        log::info!("[MEMORY] Water material handle created");
        material
    };

    let mut zone_loading_assets: Vec<UntypedHandle> = Vec::default();
    let zone_entity = commands
        .spawn((
            Zone {
                id: zone_data.zone_id,
            },
            Visibility::Visible,
            ViewVisibility::default(),
            InheritedVisibility::default(),
            Transform::from_xyz(5200.0, 0.0, -5200.0),
            GlobalTransform::default(),
            NoFrustumCulling,
            Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
            RenderLayers::layer(0),
        ))
        .id();
    log::info!("[ZONE LOADER DEBUG] Spawned Zone entity {:?} with Visibility::Visible, NoFrustumCulling, and large Aabb", zone_entity);
   // info!("[ASSET LIFECYCLE] Zone entity spawned: {:?} (zone_id: {})", zone_entity, zone_data.zone_id.get());
    memory_tracking.log_entity_spawned("Zone", 0);
    log::info!("[SPAWN ZONE] Zone entity spawned: {:?}", zone_entity);
    log::info!("[MEMORY] Zone entity created: {:?}", zone_entity);
    
    // DIAGNOSTIC: Confirm zone entity was spawned
    log::info!("[SPAWN ZONE DIAGNOSTIC] ✓ Zone entity SUCCESSFULLY SPAWNED: entity={:?}, zone_id={}",
        zone_entity, zone_data.zone_id.get());

    // Cartoon sky removed - now using Bevy 0.16 built-in atmospheric scattering
    // The Atmosphere and AtmosphereSettings components are added to the camera instead
    // This provides physics-based Rayleigh and Mie scattering with dynamic time-of-day
    log::info!("[SPAWN ZONE] Using Bevy 0.16 built-in atmospheric scattering (cartoon sky disabled)");

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
                        let (water_entity, water_center, water_half_extents) = spawn_water(
                            commands,
                            meshes,
                            ifo.water_size,
                            Vec3::new(plane_start.x, plane_start.y, plane_start.z),
                            Vec3::new(plane_end.x, plane_end.y, plane_end.z),
                            &water_material,
                        );
                        commands.entity(zone_entity).add_child(water_entity);
                        water_count += 1;
                        
                        // Send event to spawn fish in this water
                        // log::info!("[FISH DEBUG] Sending WaterSpawnedEvent from zone_loader: water_entity={:?}, zone_entity={:?}, center={:?}, extents={:?}",
                        //     water_entity, zone_entity, water_center, water_half_extents);
                        water_spawned_events.write(WaterSpawnedEvent {
                            water_entity,
                            zone_entity,
                            water_center,
                            water_half_extents,
                        });
                    }

                    for (ifo_object_id, event_object) in ifo.event_objects.iter().enumerate() {
                        let event_entity = spawn_object(
                            commands,
                            asset_server,
                            &mut zone_loading_assets,
                            vfs_resource,
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

                    // Animated objects and effect objects
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
                            meshes,
                            storage_buffers.as_mut(),
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

    // DIAGNOSTIC: About to return from spawn_zone
    log::info!("[SPAWN ZONE DIAGNOSTIC] ✓ spawn_zone returning SUCCESS: entity={:?}, assets_count={}",
        zone_entity, zone_loading_assets.len());

    Ok((zone_entity, zone_loading_assets))
}

// REMOVED: CartoonSky - using Bevy 0.16 Atmosphere instead
// const SKY_DOME_RADIUS: f32 = 500.0;

// /// Spawns a cartoon procedural sky entity and returns the entity along with asset handles.
// /// Uses CartoonSkyMaterial for procedural sky rendering with day/night cycle support.
// fn spawn_cartoon_sky(
//     commands: &mut Commands,
//     meshes: &mut Assets<Mesh>,
//     cartoon_sky_materials: &mut Assets<CartoonSkyMaterial>,
// ) -> (Entity, Vec<UntypedHandle>) {
//     log::info!("[SPAWN CARTOON SKY] Creating procedural cartoon sky dome");
//
//     // Create a UV sphere mesh for the sky dome
//     // The sphere is inverted (rendered from inside) for sky rendering
//     let sky_dome_mesh = Mesh::from(bevy::math::primitives::Sphere {
//         radius: SKY_DOME_RADIUS,
//     });
//
//     let mesh_handle = meshes.add(sky_dome_mesh);
//     log::info!("[SPAWN CARTOON SKY] Created sky dome mesh with radius {}", SKY_DOME_RADIUS);
//
//     // Create the cartoon sky material with default settings
//     // The material will be updated by cartoon_sky_material_system based on ZoneTime
//     let material = cartoon_sky_materials.add(CartoonSkyMaterial::default());
//
//     log::info!("[SPAWN CARTOON SKY] Created cartoon sky material");
//
//     // Spawn the sky dome entity
//     // Note: No asset loading needed - everything is procedural
//     // CRITICAL: Include NotShadowCaster - sky should never cast shadows
//     let entity = commands
//         .spawn((
//             Mesh3d(mesh_handle),
//             MeshMaterial3d(material),
//             Transform::from_xyz(0.0, 0.0, 0.0),
//             GlobalTransform::default(),
//             ViewVisibility::default(),
//             Visibility::Visible,
//             InheritedVisibility::default(),
//             NoFrustumCulling,
//             Aabb::from_min_max(Vec3::splat(-SKY_DOME_RADIUS * 1.1), Vec3::splat(SKY_DOME_RADIUS * 1.1)),
//             RenderLayers::layer(0),
//             NotShadowCaster,  // Sky should never cast shadows
//         ))
//         .id();
//
//     log::info!("[SPAWN CARTOON SKY] Cartoon sky entity spawned: {:?}", entity);
//
//     // No external assets to load - everything is procedural
//     (entity, Vec::new())
// }

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

    // Build tile_texture_map for UV lookup
    let mut tile_texture_map = vec![0u32; tile_textures.len().max(1)];
    
    // First pass: build the texture mapping
    for tile_x in 0..16 {
        for tile_y in 0..16 {
            let tile_idx = tilemap
                .map(|tm| tm.get_clamped(tile_x, tile_y) as usize)
                .unwrap_or(0);
            
            if tile_idx >= zone_data.zon.tiles.len() {
                continue;
            }
            
            let tile = &zone_data.zon.tiles[tile_idx];
            let tile_array_index1 = (tile.layer1 + tile.offset1) as usize;
            let tile_array_index2 = (tile.layer2 + tile.offset2) as usize;

            if tile_array_index1 < tile_texture_map.len() {
                tile_texture_map[tile_array_index1] = tile_array_index1 as u32;
            } else {
                warn!(
                    "[SPAWN TERRAIN] Invalid tile layer1 id {} (max: {}), clamping",
                    tile_array_index1, tile_texture_map.len().saturating_sub(1)
                );
            }

            if tile_array_index2 < tile_texture_map.len() {
                tile_texture_map[tile_array_index2] = tile_array_index2 as u32;
            } else {
                warn!(
                    "[SPAWN TERRAIN] Invalid tile layer2 id {} (max: {}), clamping",
                    tile_array_index2, tile_texture_map.len().saturating_sub(1)
                );
            }
        }
    }

    // Second pass: build mesh vertices with tile info
    for tile_x in 0..16 {
        for tile_y in 0..16 {
            let tile_idx = tilemap
                .map(|tm| tm.get_clamped(tile_x, tile_y) as usize)
                .unwrap_or(0);
            
            let tile = if tile_idx < zone_data.zon.tiles.len() {
                &zone_data.zon.tiles[tile_idx]
            } else {
                continue;
            };
            
            // Get tile texture indices with bounds checking
            let tile_array_index1 = if ((tile.layer1 + tile.offset1) as usize) < tile_texture_map.len() {
                tile_texture_map[(tile.layer1 + tile.offset1) as usize]
            } else {
                0
            };
            let tile_array_index2 = if ((tile.layer2 + tile.offset2) as usize) < tile_texture_map.len() {
                tile_texture_map[(tile.layer2 + tile.offset2) as usize]
            } else {
                0
            };
            
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

                    // Pack tile info: layer1_id | layer2_id << 8 | rotation << 16
                    tile_ids.push(tile_array_index1 | (tile_array_index2 << 8) | ((tile_rotation as u32) << 16));
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

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let vertex_count = positions.len();
    let triangle_count = indices.len() / 3;
    mesh.insert_indices(Indices::U16(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs_lightmap);
    mesh.insert_attribute(MESH_ATTRIBUTE_UV_1, uvs_tile);
    
    // CRITICAL FIX: Insert tile_ids as custom vertex attribute for terrain texture mapping
    // This was the missing piece - tile_ids was computed but never added to the mesh!
    mesh.insert_attribute(
        crate::render::TERRAIN_MESH_ATTRIBUTE_TILE_INFO,
        tile_ids,
    );
    
    log::info!("[SPAWN TERRAIN] Block {}_{}: Mesh created with {} vertices, {} triangles (with tile_info attribute)",
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

    // Create TerrainMaterial with all tile textures for proper multi-texture terrain rendering
    // The shader uses binding_array to sample from up to 100 textures based on per-vertex tile_info
    let material_handle = terrain_materials.add(TerrainMaterial {
        textures: tile_textures.clone(),
    });

    // Split spawn to avoid Bundle tuple limit (15+ components not supported)
    let terrain_entity = commands
        .spawn((
            EditorSelectable,
            ZoneObject::Terrain(ZoneObjectTerrain {
                block_x: block_data.block_x as u32,
                block_y: block_data.block_y as u32,
            }),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material_handle),
            Transform::from_xyz(offset_x - 5200.0, 0.0, -offset_y + 5200.0),
            GlobalTransform::default(),
            Visibility::Visible,
            ViewVisibility::default(),
            InheritedVisibility::default(),
            NoFrustumCulling,
            Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
            RenderLayers::layer(0),
            NotShadowCaster,
        ))
        .insert((
            RigidBody::Fixed,
            Collider::trimesh(collider_verts, collider_indices).expect("Failed to create terrain collider"),
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
   // info!("[ASSET LIFECYCLE] Terrain entity spawned: {:?} block {}_{} with {} textures",
        //terrain_entity, block_data.block_x, block_data.block_y);
    log::info!("[SPAWN TERRAIN] Terrain entity created: {:?} at position ({}, 0, {})",
        terrain_entity, offset_x, offset_y);
    log::info!("[MEMORY] Terrain material created for block {}_{}",
        block_data.block_x, block_data.block_y);
    terrain_entity
}

fn spawn_water(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    water_size: f32,
    plane_start: Vec3,
    plane_end: Vec3,
    water_material: &Handle<WaterMaterial>,  // Use custom WaterMaterial
) -> (Entity, Vec3, Vec2) {
    let start = Vec3::new(
        plane_start.x / 100.0,
        plane_start.y / 100.0,
        -plane_start.z / 100.0,
    );
    let end = Vec3::new(
        plane_end.x / 100.0,
        plane_end.y / 100.0,
        -plane_end.z / 100.0,
    );
    let uv_x = (end.x - start.x) / (water_size / 100.0);
    let uv_y = (end.z - start.z) / (water_size / 100.0);

    // Calculate water center and half extents for fish spawning
    let water_center = (start + end) * 0.5;
    let water_half_extents = Vec2::new(
        (end.x - start.x).abs() * 0.5,
        (end.z - start.z).abs() * 0.5,
    );

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

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_indices(indices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    // Split spawn to avoid Bundle tuple limit (15+ components not supported)
    let water_entity = commands
        .spawn((
            EditorSelectable,
            ZoneObject::Water,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(water_material.clone()),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            NoFrustumCulling,
            Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
            RenderLayers::layer(0),
            NotShadowCaster,
        ))
        .insert((
            NotShadowReceiver,
            RigidBody::Fixed,
            Collider::trimesh(collider_verts, collider_indices).expect("Failed to create water collider"),
            CollisionGroups::new(COLLISION_GROUP_ZONE_WATER, COLLISION_FILTER_INSPECTABLE),
        ))
        .id();
    
   // info!("[ASSET LIFECYCLE] Water entity spawned: {:?}", water_entity);
    (water_entity, water_center, water_half_extents)
}

fn spawn_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    zone_loading_assets: &mut Vec<UntypedHandle>,
    vfs_resource: &VfsResource,
    object_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
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
    // log::info!("[SPAWN OBJECT] Spawning object: IFO id={}, ZSC id={}, parts={}",
    //     ifo_object_id, zsc_object_id, zsc.objects[zsc_object_id].parts.len());
    let object = &zsc.objects[zsc_object_id];
    let object_transform = Transform::default()
        .with_translation(
            Vec3::new(
                object_instance.position.x,
                object_instance.position.z,
                -object_instance.position.y,
            ) / 100.0,
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
        EditorSelectable,
        object_type(ZoneObjectId {
            ifo_object_id,
            zsc_object_id,
        }),
        object_transform,
        GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        NoFrustumCulling,
        Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
        bevy::render::view::RenderLayers::layer(0),
        RigidBody::Fixed,
    ));

    let object_entity = object_entity_commands.id();

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
                // log::warn!("[SPAWN OBJECT] Object {} part {} has invalid mesh_id {} (max: {}), skipping part",
                //     zsc_object_id, part_index, mesh_id, zsc.meshes.len().saturating_sub(1));
                continue;
            }

            // VALIDATION FIX: Check material_id bounds
            let material_id = object_part.material_id as usize;
            if material_id >= zsc.materials.len() {
                // log::warn!("[SPAWN OBJECT] Object {} part {} has invalid material_id {} (max: {}), skipping part",
                //     zsc_object_id, part_index, material_id, zsc.materials.len().saturating_sub(1));
                continue;
            }

            let mesh = mesh_cache[mesh_id].clone().unwrap_or_else(|| {
                let mesh_path = zsc.meshes[mesh_id].path().to_string_lossy().into_owned();
                let mesh_path_log = mesh_path.clone();
                // log::info!("[SPAWN OBJECT] Loading mesh: {}", mesh_path_log);
                let handle = asset_server.load(&mesh_path);
                mesh_cache.insert(mesh_id, Some(handle.clone()));
                //info!("[MEMORY TRACKING] Mesh handle created: {}", mesh_path_log);
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
                    let handle = asset_server.load::<bevy::prelude::Image>(&path_str);
                    //info!("[MEMORY TRACKING] Lightmap texture handle created: {}", path_str);
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

            //log::info!("[SPAWN OBJECT] Creating material: {}", material_path_log);
            let base_texture_handle = asset_server.load(&material_path);
            //info!("[MEMORY TRACKING] Object material base texture handle created: {}", material_path_log);

            let lightmap_count = lightmap_texture.as_ref().is_some() as usize;

            // Create ExtendedMaterial with RoseObjectExtension for zone lighting support
            // This applies zone lighting ambient color to darken objects to match the original game
            let material = object_materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color_texture: if material_path.is_empty() || material_path == "" || material_path == "NULL" {
                        log::warn!("[SPAWN OBJECT DEBUG] Empty or NULL texture path for mesh_id {}, using fallback", mesh_id);
                        Some(asset_server.load("ETC/SPECULAR_SPHEREMAP.DDS"))
                    } else {
                        Some(base_texture_handle.clone())
                    },
                    unlit: false,  // Enable PBR lighting for objects/decorations
                    double_sided: zsc_material.two_sided,
                    // PBR properties for realistic lighting on vegetation and outdoor objects
                    perceptual_roughness: 0.8,  // Higher roughness for matte vegetation/buildings
                    metallic: 0.0,              // Non-metallic for organic/building materials
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
                    lightmap_params: Vec3::new(lightmap_uv_offset.x, lightmap_uv_offset.y, lightmap_uv_scale).extend(0.0),
                    lightmap_texture: lightmap_texture.clone(),
                    specular_texture: Some(specular_texture.image.clone()),
                },
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
            // if material.is_weak() {
            //     log::error!("[SPAWN OBJECT] CRITICAL: Material is weak! Object {} part {} will not render!",
            //         zsc_object_id, part_index);
            // }

            // Determine if this part should cast shadows based on material transparency
            // Opaque and alpha-masked materials cast shadows, alpha-blended materials don't
            let is_transparent = zsc_material.alpha_enabled && zsc_material.alpha_test.is_none();
            
            let part_entity = commands.spawn((
                EditorSelectable,
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
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                part_transform,
                GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
                NoFrustumCulling,
                Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
                RenderLayers::layer(0),
                ColliderParent::new(object_entity),
                AsyncCollider(ComputedColliderShape::TriMesh(bevy_rapier3d::prelude::TriMeshFlags::FIX_INTERNAL_EDGES)),
                CollisionGroups::new(collision_group, collision_filter),
            )).id();
            
            // Only disable shadow casting for truly transparent (alpha-blended) materials
            // Opaque and alpha-masked materials should cast shadows
            if is_transparent {
                commands.entity(part_entity).insert(NotShadowCaster);
            }

            let active_motion = object_part.animation_path.as_ref().map(|animation_path| {
                TransformAnimation::repeat(asset_server.load(animation_path.path().to_string_lossy().into_owned()), None)
            });
            if let Some(active_motion) = active_motion {
                commands.entity(part_entity).insert(active_motion);
            }

            // Add wind sway effect to grass and tree leaf models based on mesh path
            let mesh_path_lower = zsc.meshes[mesh_id].path().to_string_lossy().to_lowercase();
            
            // Store the part's rotation to use as base_rotation for wind sway
            let part_base_rotation = part_transform.rotation;
            
            // Generate a random phase offset based on object position for natural variation
            let phase_offset = (object_instance.position.x * 0.1 + object_instance.position.y * 0.13).fract() * std::f32::consts::TAU;
            
            // Check for grass models (identified by "grass" in the mesh name)
            if mesh_path_lower.contains("grass") {
                commands.entity(part_entity).insert(
                    WindSway::for_grass()
                        .with_base_rotation(part_base_rotation)
                        .with_phase_offset(phase_offset)
                );
            }
            // Check for tree leaf models (identified by "leaf" or "leaves" in the mesh name)
            else if mesh_path_lower.contains("leaf") || mesh_path_lower.contains("leaves") {
                commands.entity(part_entity).insert(
                    WindSway::for_tree_leaves()
                        .with_base_rotation(part_base_rotation)
                        .with_phase_offset(phase_offset)
                );
            }
            // Check for tree foliage (alternative naming conventions)
            else if mesh_path_lower.contains("foliage") || mesh_path_lower.contains("canopy") {
                commands.entity(part_entity).insert(
                    WindSway::for_tree_leaves()
                        .with_base_rotation(part_base_rotation)
                        .with_phase_offset(phase_offset)
                );
            }
            // Check for bush/shrub models (similar swaying behavior to grass)
            else if mesh_path_lower.contains("bush") || mesh_path_lower.contains("shrub") || mesh_path_lower.contains("plant") {
                commands.entity(part_entity).insert(
                    WindSway::for_grass()
                        .with_base_rotation(part_base_rotation)
                        .with_phase_offset(phase_offset)
                );
            }
            // Check for tree models - apply wind sway to tree tops (leaves) but NOT trunks
            // Tree naming convention: TREE004.ZMS = top/leaves (sway), TREE004B.ZMS = trunk (no sway)
            // The "B" suffix indicates the trunk/base part which should remain static
            else if mesh_path_lower.contains("tree") {
                // Check if this is a trunk file (ends with "b.zms" or contains "b." before extension)
                let is_trunk = mesh_path_lower.ends_with("b.zms") ||
                               mesh_path_lower.ends_with("b") ||
                               mesh_path_lower.rsplit_once('.').map_or(false, |(name, _ext)| name.ends_with('b'));
                
                if !is_trunk {
                    // This is the tree top/leaves - apply wind sway
                    commands.entity(part_entity).insert(
                        WindSway::for_tree_leaves()
                            .with_base_rotation(part_base_rotation)
                            .with_phase_offset(phase_offset)
                    );
                }
                // If it's a trunk (ends with B), don't apply wind sway - trunk stays static
            }

            commands.entity(object_entity).add_child(part_entity);
            part_entities.push(part_entity);
        }

    // log::info!("[SPAWN OBJECT] Object entity created: {:?} with {} parts",
    //     object_entity, part_entities.len());
    let mesh_count = mesh_cache.iter().filter(|m| m.is_some()).count();
    //info!("[MEMORY TRACKING] Object entity created with {} mesh handles",
        //mesh_count);
   // log::info!("[MEMORY] Object entity created with {} mesh handles",
        //mesh_count);

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

        // Effect spawning temporarily disabled (use custom materials)
        /*
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
        */
    }

    object_entity
}

fn spawn_animated_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect_mesh_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseEffectExtension>>,
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
            ) / 100.0,
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
    
    // Handle NULL texture paths for animated objects
    let texture_handle = if texture_path.is_empty() || texture_path == "NULL" {
        log::warn!("[SPAWN ANIMATED OBJECT] NULL or empty texture path, using fallback");
        asset_server.load::<Image>("ETC/SPECULAR_SPHEREMAP.DDS")
    } else {
        asset_server.load::<Image>(&texture_path)
    };
    
    let motion_path_buf = ZmoTextureAssetLoader::convert_path(&motion_path);
    let motion_texture_handle = asset_server.load(ZmoTextureAssetLoader::convert_path_texture(&motion_path));
    let motion_handle = asset_server.load(motion_path_buf.to_string_lossy().into_owned());

    // Log asset creation
    //info!("[MEMORY TRACKING] Animated object mesh handle created: {}", mesh_path_str);
    //info!("[MEMORY TRACKING] Animated object texture handle created: {}", texture_path_str);
    //info!("[MEMORY TRACKING] Animated object motion texture handle created: {}",
       // ZmoTextureAssetLoader::convert_path_texture(&motion_path));
    //info!("[MEMORY TRACKING] Animated object motion handle created: {}",
        //motion_path_buf.display());

    let material = effect_mesh_materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color_texture: Some(texture_handle),
            // PBR properties for realistic lighting on animated objects
            perceptual_roughness: 0.8,  // Higher roughness for matte vegetation/outdoor objects
            metallic: 0.0,              // Non-metallic for organic materials
            alpha_mode: if alpha_test_enabled {
                AlphaMode::Mask(0.5)
            } else {
                AlphaMode::Opaque
            },
            double_sided: two_sided,
            ..Default::default()
        },
        extension: RoseEffectExtension {
            animation_texture: Some(motion_texture_handle.clone()),
        },
    });

    //info!("[MEMORY TRACKING] Animated object material created with 3 textures (base, motion texture, motion)");

    // Determine if this animated object should cast shadows based on material transparency
    // Opaque and alpha-masked materials cast shadows, alpha-blended materials don't
    let is_transparent = alpha_enabled && !alpha_test_enabled;

    let animated_entity = commands
        .spawn((
            EditorSelectable,
            ZoneObject::AnimatedObject(ZoneObjectAnimatedObject {
                mesh_path: mesh_path.to_string(),
                motion_path: motion_path.to_string(),
                texture_path: texture_path.to_string(),
            }),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            MeshAnimation::repeat(motion_handle, None),
            object_transform,
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            NoFrustumCulling,
            Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
            RenderLayers::layer(0),
            AsyncCollider(ComputedColliderShape::TriMesh(bevy_rapier3d::prelude::TriMeshFlags::empty())),
            CollisionGroups::new(COLLISION_GROUP_ZONE_OBJECT, COLLISION_FILTER_INSPECTABLE),
        ))
        .id();
    
    // Only disable shadow casting for truly transparent (alpha-blended) materials
    // Opaque and alpha-masked materials should cast shadows
    if is_transparent {
        commands.entity(animated_entity).insert(NotShadowCaster);
    }

   // info!("[ASSET LIFECYCLE] Animated object entity spawned: {:?}", animated_entity);
    animated_entity
}

fn spawn_effect_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    vfs_resource: &VfsResource,
    effect_mesh_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseEffectExtension>>,
    particle_materials: &mut Assets<ParticleMaterial>,
    meshes: &mut Assets<bevy::prelude::Mesh>,
    storage_buffers: &mut Assets<bevy::render::storage::ShaderStorageBuffer>,
    effect_object: &IfoEffectObject,
    ifo_object_id: usize,
) -> Entity {
    let object = &effect_object.object;
    let object_transform = Transform::default()
        .with_translation(
            Vec3::new(object.position.x, object.position.z, -object.position.y) / 100.0,
        )
        .with_rotation(Quat::from_xyzw(
            object.rotation.x,
            object.rotation.z,
            -object.rotation.y,
            object.rotation.w,
        ))
        .with_scale(Vec3::new(object.scale.x, object.scale.z, object.scale.y));

    let effect_path_str = effect_object.effect_path.path().to_string_lossy().to_string();
   // info!("[ASSET LIFECYCLE] Spawning effect object: {}", effect_path_str);

    let effect_object_entity = commands
        .spawn((
            EditorSelectable,
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
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
            RenderLayers::layer(0),
        ))
        .id();

   // info!("[ASSET LIFECYCLE] Effect object entity spawned: {:?}", effect_object_entity);

    spawn_effect(
        &vfs_resource.vfs,
        commands,
        asset_server,
        particle_materials,
        effect_mesh_materials,
        storage_buffers,
        meshes,
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
            Vec3::new(object.position.x, object.position.z, -object.position.y) / 100.0,
        )
        .with_rotation(Quat::from_xyzw(
            object.rotation.x,
            object.rotation.z,
            -object.rotation.y,
            object.rotation.w,
        ))
        .with_scale(Vec3::new(object.scale.x, object.scale.z, object.scale.y));

    let sound_path_str = sound_object.sound_path.path().to_string_lossy().to_string();
   // info!("[ASSET LIFECYCLE] Spawning sound object: {}", sound_path_str);
   
   // Handle NULL sound paths - skip loading if path is NULL or empty
   if sound_path_str.is_empty() || sound_path_str == "NULL" {
       log::warn!("[SPAWN SOUND OBJECT] NULL or empty sound path, skipping sound loading");
       let effect_object_entity = commands
           .spawn((
               EditorSelectable,
               ZoneObject::SoundObject {
                   ifo_object_id,
                   sound_path: sound_path_str.clone(),
               },
               object_transform,
               GlobalTransform::from(object_transform),
               Visibility::Visible,
               InheritedVisibility::default(),
               ViewVisibility::default(),
               NoFrustumCulling,
               Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
               RenderLayers::layer(0),
           ))
           .id();
       return effect_object_entity;
   }

    let effect_object_entity = commands
        .spawn((
            EditorSelectable,
            ZoneObject::SoundObject {
                ifo_object_id,
                sound_path: sound_path_str.clone(),
            },
            SpatialSound::new_repeating(asset_server.load(&sound_path_str)),
            SoundRadius::new(sound_object.range as f32 / 10.0),
            object_transform,
            GlobalTransform::from(object_transform),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            NoFrustumCulling,
            Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
            RenderLayers::layer(0),
        ))
        .id();
    
   // info!("[ASSET LIFECYCLE] Sound object entity spawned: {:?}", effect_object_entity);
    effect_object_entity
}
