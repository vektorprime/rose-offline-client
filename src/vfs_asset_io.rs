use bevy::app::App;
use bevy::asset::{
    io::{AssetReader, AssetReaderError, AssetSourceBuilder, AssetSourceId, Reader, VecReader},
    AssetApp, AssetServer,
};
use bevy::prelude::{Plugin, Res, Resource};
use rose_file_readers::{VfsFile, VirtualFilesystem};
use std::{
    collections::HashMap,
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::resources::VfsResource;

/// Formats bytes into human-readable string (e.g., "1.5 MB", "256 KB")
pub fn format_bytes(bytes: usize) -> String {
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

/// Global file cache shared between all VfsAssetIo instances
/// This cache persists file data in memory to avoid repeated disk/VFS reads
static VFS_FILE_CACHE: std::sync::OnceLock<std::sync::RwLock<HashMap<String, Arc<Vec<u8>>>>> =
    std::sync::OnceLock::new();

/// Get or initialize the global file cache
fn get_file_cache() -> &'static std::sync::RwLock<HashMap<String, Arc<Vec<u8>>>> {
    VFS_FILE_CACHE.get_or_init(|| std::sync::RwLock::new(HashMap::new()))
}

/// Clear the global VFS file cache
/// Call this when switching zones to free memory
pub fn clear_vfs_file_cache() {
    if let Ok(mut cache) = get_file_cache().write() {
        let count = cache.len();
        cache.clear();
        log::info!("[VFS CACHE] Cleared {} cached files from memory", count);
    }
}

/// Returns (file count, total bytes) of the global VFS file cache.
/// Diagnostic helper for memory leak tracking.
pub fn vfs_file_cache_stats() -> (usize, usize) {
    if let Ok(cache) = get_file_cache().read() {
        let bytes = cache
            .values()
            .fold(0usize, |acc, data| acc.saturating_add(data.len()));
        (cache.len(), bytes)
    } else {
        (0, 0)
    }
}

#[derive(Resource)]
pub struct VfsAssetIo {
    vfs: Arc<VirtualFilesystem>,
    /// Base path for real filesystem fallback - files here take priority over VFS
    base_path: PathBuf,
    /// Whether to use the global file cache (default: true)
    use_cache: bool,
}

impl VfsAssetIo {
    pub fn new(vfs: Arc<VirtualFilesystem>, base_path: PathBuf) -> Self {
        log::info!(
            "[VFS ASSET IO] Creating new VfsAssetIo instance with base_path: {:?}",
            base_path
        );
        Self {
            vfs,
            base_path,
            use_cache: true,
        }
    }

    /// Try to get a file from the cache
    fn get_from_cache(&self, path: &str) -> Option<Arc<Vec<u8>>> {
        if !self.use_cache {
            return None;
        }

        if let Ok(cache) = get_file_cache().read() {
            cache.get(path).cloned()
        } else {
            None
        }
    }

    /// Store a file in the cache
    fn store_in_cache(&self, path: &str, data: Vec<u8>) -> Arc<Vec<u8>> {
        let arc_data = Arc::new(data);

        if self.use_cache {
            if let Ok(mut cache) = get_file_cache().write() {
                cache.insert(path.to_string(), arc_data.clone());
            }
        }

        arc_data
    }
}

impl AssetReader for VfsAssetIo {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<impl Reader + 'a, AssetReaderError>> + Send {
        async move {
            let path_str = path
                .to_str()
                .unwrap()
                .trim_end_matches(".no_skin")
                .trim_end_matches(".zmo_texture");

            // HACK: Exclude shaders from VFS to allow load_internal_asset! to work
            // These are local files in the src/render/shaders directory
            if path_str.contains("shaders/") || path_str.contains("shaders\\") {
                if let Ok(data) = std::fs::read(path) {
                    return Ok(VecReader::new(data));
                }
                log::warn!(
                    "[VFS DEBUG] Failed to read shader from local filesystem: \"{}\"",
                    path_str
                );
            }

            // CHECK CACHE FIRST - This is the key optimization!
            // If the file is already in memory, return it directly without disk/VFS access
            if let Some(cached_data) = self.get_from_cache(path_str) {
                // Log cache hit (only for DDS and model files to reduce noise)
                if path_str.to_uppercase().ends_with(".DDS")
                    || path_str.to_uppercase().ends_with(".ZMS")
                    || path_str.to_uppercase().ends_with(".ROSE")
                {
                    log::debug!(
                        "[VFS CACHE HIT] {} (size: {})",
                        path_str,
                        format_bytes(cached_data.len())
                    );
                }

                // Clone the Arc's data for VecReader
                return Ok(VecReader::new((*cached_data).clone()));
            }

            // PRIORITY: Real filesystem takes priority over VFS
            // This allows saved map editor modifications to be loaded instead of original VFS files
            let real_filesystem_path = self.base_path.join(path_str);
            if real_filesystem_path.exists() {
                match std::fs::read(&real_filesystem_path) {
                    Ok(data) => {
                        log::info!(
                            "[VFS] Loaded from real filesystem: {} (size: {})",
                            path_str,
                            format_bytes(data.len())
                        );

                        // Store in cache for future access
                        let cached = self.store_in_cache(path_str, data);
                        return Ok(VecReader::new((*cached).clone()));
                    }
                    Err(e) => {
                        log::warn!(
                            "[VFS] File exists at {:?} but failed to read: {}",
                            real_filesystem_path,
                            e
                        );
                    }
                }
            }

            // Try to read from VFS as fallback
            match self.vfs.open_file(path_str) {
                Ok(file) => {
                    match file {
                        VfsFile::Buffer(buffer) => {
                            // Store in cache for future access
                            let cached = self.store_in_cache(path_str, buffer);
                            Ok(VecReader::new((*cached).clone()))
                        }
                        VfsFile::View(view) => {
                            let data: Vec<u8> = view.into();

                            // Store in cache for future access
                            let cached = self.store_in_cache(path_str, data);
                            Ok(VecReader::new((*cached).clone()))
                        }
                    }
                }
                Err(e) => {
                    // Fallback to local filesystem if not found in VFS (for non-base_path files)
                    if let Ok(data) = std::fs::read(path) {
                        // Store in cache
                        let cached = self.store_in_cache(path_str, data);
                        return Ok(VecReader::new((*cached).clone()));
                    }

                    log::warn!("[VFS DIAGNOSTIC] VFS file not found for path: {}", path_str);
                    log::warn!("[VFS DIAGNOSTIC] Error: {:?}", e);
                    log::info!("[VFS DIAGNOSTIC] ===========================================");
                    Err(AssetReaderError::NotFound(path.into()))
                }
            }
        }
    }

    fn read_meta<'a>(
        &'a self,
        _path: &'a Path,
    ) -> impl Future<Output = Result<impl Reader + 'a, AssetReaderError>> + Send {
        async move {
            // Return NotFound for metadata - this is correct behavior since VFS files
            // don't have .meta files. Bevy will use default metadata.
            use bevy::asset::io::Reader;
            Err::<Box<dyn Reader + 'a>, AssetReaderError>(AssetReaderError::NotFound(_path.into()))
        }
    }

    fn read_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> impl Future<
        Output = Result<
            Box<dyn bevy::tasks::futures_lite::Stream<Item = PathBuf> + Send + Unpin + 'static>,
            AssetReaderError,
        >,
    > + Send {
        async move {
            // ============================================================================
            // CRITICAL FIX - DO NOT REMOVE - DO NOT MODIFY
            // ============================================================================
            // SAFETY: This MUST return an empty stream to prevent catastrophic memory
            // consumption and application crash.
            //
            // PREVIOUS BUG: Returning actual directory contents (e.g., .zone_loader files)
            // caused Bevy's asset system to continuously discover and reload assets,
            // resulting in memory allocation rates of 2GB/SECOND until system OOM crash.
            //
            // ROOT CAUSE: Bevy's asset system scans directories and triggers reloads for
            // any "new" files found. The previous implementation returned .zone_loader
            // references for ALL directories, causing endless reload cycles.
            //
            // CONSEQUENCES OF REMOVING THIS FIX:
            //   - Immediate 2GB/second memory leak
            //   - Complete system memory exhaustion (OOM crash)
            //   - All zone assets reloaded repeatedly in infinite loop
            //   - Application becomes unresponsive within seconds
            //
            // CORRECT BEHAVIOR: Zones must ONLY be loaded via LoadZoneEvent, never through
            // directory scanning. This empty stream prevents Bevy from discovering any
            // "directory contents" to watch/reload.
            //
            // IF YOU NEED DIRECTORY LISTING FUNCTIONALITY: Implement a separate,
            // non-AssetReader API that doesn't trigger Bevy's hot-reload system.
            // ============================================================================
            let stream = bevy::tasks::futures_lite::stream::iter(Vec::<PathBuf>::new());
            Ok(Box::new(stream)
                as Box<
                    dyn bevy::tasks::futures_lite::Stream<Item = PathBuf> + Send + Unpin + 'static,
                >)
        }
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<bool, AssetReaderError>> + Send {
        async move {
            log::info!("[VFS DIAGNOSTIC] is_directory called for path: {:?}", path);
            Ok(false)
        }
    }
}

/// Plugin that registers the VFS as the default asset source.
///
/// # Requirements
/// This plugin requires that `VfsResource` is already inserted into the app before
/// this plugin is built. The plugin retrieves the VFS from the resource rather than
/// holding its own Arc, eliminating redundant Arc clones.
///
/// # Optimization Note
/// Previously, this plugin held its own `Arc<VirtualFilesystem>` which required an
/// extra clone at initialization. Now it retrieves the VFS from `VfsResource`,
/// reducing Arc clones from 2 to 1 during VFS initialization.
pub struct VfsAssetReaderPlugin;

impl VfsAssetReaderPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VfsAssetReaderPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for VfsAssetReaderPlugin {
    fn build(&self, app: &mut App) {
        // Get the VFS and base_path from VfsResource
        let vfs = app
            .world()
            .get_resource::<VfsResource>()
            .expect("VfsResource must be inserted before VfsAssetReaderPlugin is built")
            .vfs
            .clone();
        let base_path = app
            .world()
            .get_resource::<VfsResource>()
            .expect("VfsResource must be inserted before VfsAssetReaderPlugin is built")
            .base_path
            .clone();

        // Register VFS as the default asset source
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(move || {
                let vfs_clone = vfs.clone();
                let base_path_clone = base_path.clone();
                Box::new(VfsAssetIo::new(vfs_clone, base_path_clone))
            }),
        );

        // Add a Startup system to verify the asset source was registered
        app.add_systems(bevy::app::Startup, |asset_server: Res<AssetServer>| {
            match asset_server.get_source(AssetSourceId::Default) {
                Ok(source) => {
                    log::info!("[VFS ASSET READER PLUGIN] Default asset source found!");
                    let reader = source.reader();
                    let reader_type = std::any::type_name_of_val(reader);
                    log::info!("[VFS ASSET READER PLUGIN] Reader type: {}", reader_type);
                }
                Err(e) => {
                    log::error!(
                        "[VFS ASSET READER PLUGIN] Failed to get default asset source: {:?}",
                        e
                    );
                }
            }
        });
    }
}
