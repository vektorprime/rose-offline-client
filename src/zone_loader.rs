use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, OnceLock},
    time::{Duration, Instant},
};

/// Memory monitoring for zone transitions
/// Tracks resident and virtual memory using Windows API
#[cfg(target_os = "windows")]
pub mod memory_monitor {
    use std::mem;

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
            log::info!(
                "[MEMORY MONITOR]   Physical: {} / {} (used)",
                crate::vfs_asset_io::format_bytes(sys.used_physical() as usize),
                crate::vfs_asset_io::format_bytes(sys.total_physical as usize)
            );
            log::info!(
                "[MEMORY MONITOR]   Virtual:  {} / {} (used)",
                crate::vfs_asset_io::format_bytes(sys.used_virtual() as usize),
                crate::vfs_asset_io::format_bytes(sys.total_virtual as usize)
            );
        }

        if let Some(proc) = get_process_memory() {
            log::info!("[MEMORY MONITOR] Process Memory:");
            log::info!(
                "[MEMORY MONITOR]   Resident (RAM):      {} (peak: {})",
                crate::vfs_asset_io::format_bytes(proc.working_set_size),
                crate::vfs_asset_io::format_bytes(proc.peak_working_set_size)
            );
            log::info!(
                "[MEMORY MONITOR]   Virtual (committed): {} (peak: {})",
                crate::vfs_asset_io::format_bytes(proc.pagefile_usage),
                crate::vfs_asset_io::format_bytes(proc.peak_pagefile_usage)
            );
            log::info!(
                "[MEMORY MONITOR]   Private bytes:       {}",
                crate::vfs_asset_io::format_bytes(proc.private_usage)
            );
        }

        log::info!("[MEMORY MONITOR] ==========================================");
    }
}

#[cfg(not(target_os = "windows"))]
pub mod memory_monitor {
    pub fn log_memory_status(context: &str) {
        log::info!(
            "[MEMORY MONITOR] Memory monitoring not available on this platform: {}",
            context
        );
    }
}

use bevy::prelude::Query;
use memory_monitor::log_memory_status;

use anyhow::Result;
use bevy::log::info_span;
use bevy::{
    asset::RenderAssetUsages,
    asset::{Asset, Assets, LoadState},
    camera::primitives::Aabb,
    camera::visibility::{InheritedVisibility, RenderLayers, ViewVisibility},
    ecs::system::SystemParam,
    image::ImageLoaderSettings,
    light::{NotShadowCaster, NotShadowReceiver},
    math::{Quat, Vec2, Vec3},
    mesh::{Indices, Mesh, PrimitiveTopology},
    pbr::{ExtendedMaterial, StandardMaterial},
    prelude::{
        AssetServer, Color, Commands, Entity, GlobalTransform, Handle, Image, Local, Mesh3d,
        MeshMaterial3d, MessageReader, MessageWriter, Res, ResMut, Resource, Transform,
        UntypedHandle, Visibility, With,
    },
    reflect::TypePath,
    render::alpha::AlphaMode,
    tasks::AsyncComputeTaskPool,
};
use bevy_rapier3d::prelude::{
    AsyncCollider, Collider, CollisionGroups, ComputedColliderShape, RigidBody,
};
use log::{info, warn};
use thiserror::Error;

use rose_data::{NpcId, WarpGateId, ZoneId, ZoneList};
use rose_file_readers::{
    HimFile, IfoEffectObject, IfoFile, IfoObject, IfoSoundObject, LitFile, LitObject, RoseFile,
    RoseFileReader, StbFile, TilFile, VfsPath, VirtualFilesystem, ZonFile, ZonTileRotation,
    ZscCollisionFlags, ZscFile,
};

use crate::{
    animation::{MeshAnimation, TransformAnimation, ZmoTextureAssetLoader},
    audio::{SoundRadius, SpatialSound},
    components::{
        ColliderParent, EventObject, MapEditorTerrainBlock, MapEditorWaterPlane,
        TerrainMeshForGrass, WarpObject, WaterSpawnedEvent, WindSway, Zone, ZoneObject,
        ZoneObjectAnimatedObject, ZoneObjectId, ZoneObjectPart, ZoneObjectTerrain,
        COLLISION_FILTER_CLICKABLE, COLLISION_FILTER_COLLIDABLE, COLLISION_FILTER_INSPECTABLE,
        COLLISION_FILTER_MOVEABLE, COLLISION_GROUP_PHYSICS_TOY, COLLISION_GROUP_ZONE_EVENT_OBJECT,
        COLLISION_GROUP_ZONE_OBJECT, COLLISION_GROUP_ZONE_TERRAIN,
        COLLISION_GROUP_ZONE_WARP_OBJECT, COLLISION_GROUP_ZONE_WATER,
    },
    effect_loader::{spawn_effect, EffectCache},
    events::{LoadZoneEvent, ZoneEvent, ZoneLoadedFromVfsEvent},
    map_editor::components::EditorSelectable,
    render::{
        ParticleMaterial, RoseEffectExtension, RoseObjectExtension, TerrainMaterial, WaterMaterial,
        MESH_ATTRIBUTE_UV_1,
    },
    resources::{CurrentZone, DebugInspector, GameData, SpecularTexture},
    vfs_asset_io::evict_zone_tagged_files,
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
    pub new_terrain_mesh: Option<Vec<u8>>,
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
pub struct ZoneLoadChannelSender(
    pub mpsc::Sender<(ZoneId, Result<ZoneLoaderAsset, anyhow::Error>)>,
);

/// Channel receiver for receiving loaded zone data from async tasks
#[derive(Resource)]
pub struct ZoneLoadChannelReceiver(
    pub std::sync::Mutex<mpsc::Receiver<(ZoneId, Result<ZoneLoaderAsset, anyhow::Error>)>>,
);

/// The most recently requested zone id (set while processing `LoadZoneEvent`s).
/// `zone_loaded_from_vfs_system` uses it to drop `ZoneLoadedFromVfsEvent`s whose
/// async load finished after a newer zone request superseded them (e.g. the
/// login screen's background zone load completing after the game zone request),
/// so a stale load can never replace the current zone.
#[derive(Resource, Default)]
pub struct LastRequestedZone(pub Option<ZoneId>);

/// Resource for tracking memory and asset lifecycle
#[derive(Resource, Default)]
pub struct MemoryTrackingResource {
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
    /// Log when a texture handle is created
    pub fn log_texture_handle_created(&mut self, path: &str) {
        let is_duplicate = !self.unique_asset_paths.insert(path.to_string());
        if is_duplicate {
            self.duplicate_asset_requests += 1;
        }
    }

    /// Log when an entity is spawned
    pub fn log_entity_spawned(&mut self) {
        self.entities_spawned += 1;
    }

    /// Log when an entity is despawned
    pub fn log_entity_despawned(&mut self) {
        self.entities_despawned += 1;
    }

    /// Log a summary of memory statistics
    pub fn log_summary(&mut self) {
        let now = Instant::now();
        let should_log = self.last_summary_time.map_or(true, |last| {
            now.duration_since(last) >= Duration::from_secs(5)
        });

        if should_log {
            self.last_summary_time = Some(now);

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

            let base_height = height_y0 * (1.0 - weight_y) + height_y1 * weight_y;

            // Apply procedural noise using thread-local generator
            // The input coordinates (x, y) are world coordinates in the game's coordinate system
            // We need to convert them to the same world coordinate system used during terrain spawning
            let world_x = x;
            let world_z = -y; // y is inverted in the game's coordinate system
            let noise_offset = crate::terrain::get_thread_local_noise(world_x, world_z);

            base_height + (noise_offset * 100.0) // Convert noise to same scale as heightmap
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

mod loading;
mod spawning;
mod systems;

pub use loading::ZoneLoader;
pub use spawning::spawn_zone;
pub use systems::{force_zone_visibility_system, zone_loaded_from_vfs_system, zone_loader_system};

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
    pub effect_mesh_materials:
        ResMut<'w, Assets<ExtendedMaterial<StandardMaterial, RoseEffectExtension>>>,
    pub object_materials:
        ResMut<'w, Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>>,
    pub particle_materials: ResMut<'w, Assets<ParticleMaterial>>,
    pub storage_buffers: ResMut<'w, Assets<bevy::render::storage::ShaderStorageBuffer>>,
    pub zone_loader_assets: ResMut<'w, Assets<ZoneLoaderAsset>>,
    pub render_config: Res<'w, crate::resources::RenderConfiguration>,
    pub memory_tracking: ResMut<'w, MemoryTrackingResource>,
    pub water_spawned_events: MessageWriter<'w, WaterSpawnedEvent>,
    pub terrain_noise: Res<'w, crate::terrain::GlobalTerrainNoise>,
    pub effect_cache: Res<'w, EffectCache>,
    pub current_zone: Option<Res<'w, CurrentZone>>,
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
    pub loading_via_async_task: bool, // Track if loading via async task vs AssetServer
    pub zone_id: Option<ZoneId>,      // Track zone_id for async-loaded zones
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
                log::info!(
                    "[MEMORY FIX] Clearing {} zone asset handles to prevent memory leak",
                    count
                );
                self.zone_assets.clear();
                self.zone_assets.shrink_to_fit();
                self.assets_cleared = true;
            }
        }
    }
}

#[derive(Default)]
pub struct ZoneLoaderCache {
    pub cache: Vec<Option<CachedZone>>,
}
