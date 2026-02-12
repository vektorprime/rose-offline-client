use bevy::asset::{
    io::{AssetReader, AssetReaderError, AsyncSeekForward, AssetSource, AssetSourceId, Reader, VecReader},
    AssetApp, AssetServer,
};
use bevy::app::App;
use bevy::prelude::{Plugin, Res, Resource};
use std::{
    future::Future,
    io::{Cursor, Seek},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use rose_file_readers::{VfsFile, VirtualFilesystem};

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

/// Tracks cumulative VFS read statistics
#[derive(Default, Debug)]
pub struct VfsReadStats {
    pub total_files_read: usize,
    pub total_bytes_read: usize,
    pub largest_file_size: usize,
    pub largest_file_path: String,
}

impl VfsReadStats {
    pub fn log_file_read(&mut self, path: &str, size: usize) {
        self.total_files_read += 1;
        self.total_bytes_read += size;
        
        if size > self.largest_file_size {
            self.largest_file_size = size;
            self.largest_file_path = path.to_string();
        }
        
        // Log large files (>10MB) for memory leak investigation
        if size > 10 * 1024 * 1024 {
            log::warn!("[VFS MEMORY] Large file loaded: {} (size: {})", path, format_bytes(size));
        }
        
        // Log every 100 files and every 100MB
        if self.total_files_read % 100 == 0 {
            log::info!(
                "[VFS MEMORY] Cumulative stats: {} files, {} total, largest: {} ({})",
                self.total_files_read,
                format_bytes(self.total_bytes_read),
                self.largest_file_path,
                format_bytes(self.largest_file_size)
            );
        }
    }
    
    pub fn log_summary(&self) {
        //log::info!("[VFS MEMORY] ==========================================");
        //log::info!("[VFS MEMORY] VFS Read Statistics Summary");
        //log::info!("[VFS MEMORY] ==========================================");
        //log::info!("[VFS MEMORY] Total files read: {}", self.total_files_read);
        //log::info!("[VFS MEMORY] Total bytes read: {}", format_bytes(self.total_bytes_read));
        //log::info!("[VFS MEMORY] Largest file: {} ({})", self.largest_file_path, format_bytes(self.largest_file_size));
        //log::info!("[VFS MEMORY] Average file size: {}", format_bytes(self.total_bytes_read / self.total_files_read.max(1)));
        //log::info!("[VFS MEMORY] ==========================================");
    }
}

struct CursorWrapper {
    data: Vec<u8>,
    position: u64,
}

impl Unpin for CursorWrapper {}

impl CursorWrapper {
    fn new(data: Vec<u8>) -> Self {
        Self { data, position: 0 }
    }
}

impl bevy::tasks::futures_lite::AsyncRead for CursorWrapper {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let pos = self.position as usize;
        let available = self.data.len().saturating_sub(pos);
        let to_read = std::cmp::min(buf.len(), available);
        
        if to_read > 0 {
            buf[..to_read].copy_from_slice(&self.data[pos..pos + to_read]);
            self.position += to_read as u64;
        }
        
        Poll::Ready(Ok(to_read))
    }
}

impl bevy::tasks::futures_lite::AsyncSeek for CursorWrapper {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        pos: std::io::SeekFrom,
    ) -> Poll<std::io::Result<u64>> {
        use std::io::SeekFrom;

        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => self.data.len() as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };

        if new_pos < 0 {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid seek to a negative position",
            )));
        }

        self.position = new_pos as u64;
        Poll::Ready(Ok(self.position))
    }
}



impl std::io::Read for CursorWrapper {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let pos = self.position as usize;
        let available = self.data.len().saturating_sub(pos);
        let to_read = std::cmp::min(buf.len(), available);
        
        if to_read > 0 {
            buf[..to_read].copy_from_slice(&self.data[pos..pos + to_read]);
            self.position += to_read as u64;
        }
        
        Ok(to_read)
    }
}

impl std::io::Seek for CursorWrapper {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        use std::io::SeekFrom;
        
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => self.data.len() as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };
        
        if new_pos < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid seek to a negative position",
            ));
        }
        
        self.position = new_pos as u64;
        Ok(self.position)
    }
}

#[derive(Resource)]
pub struct VfsAssetIo {
    vfs: Arc<VirtualFilesystem>,
    read_stats: std::sync::Mutex<VfsReadStats>,
}

impl VfsAssetIo {
    pub fn new(vfs: Arc<VirtualFilesystem>) -> Self {
        log::info!("[VFS ASSET IO] Creating new VfsAssetIo instance");
        Self {
            vfs,
            read_stats: std::sync::Mutex::new(VfsReadStats::default()),
        }
    }
    
    /// Log the current VFS read statistics summary
    pub fn log_read_stats(&self) {
        if let Ok(stats) = self.read_stats.lock() {
            stats.log_summary();
        }
    }
    
    /// Get a copy of the current read statistics
    pub fn get_read_stats(&self) -> Option<VfsReadStats> {
        self.read_stats.lock().ok().map(|s| VfsReadStats {
            total_files_read: s.total_files_read,
            total_bytes_read: s.total_bytes_read,
            largest_file_size: s.largest_file_size,
            largest_file_path: s.largest_file_path.clone(),
        })
    }
}

impl AssetReader for VfsAssetIo {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<impl Reader + 'a, AssetReaderError>> + Send {
        async move {
            // Log ALL read calls to see what's happening
            let path_str = path.to_str().unwrap_or("");
            // log::info!("[VFS DIAGNOSTIC] ===========================================");
            // log::info!("[VFS DIAGNOSTIC] VfsAssetIo::read ENTRY POINT REACHED");
            // log::info!("[VFS DIAGNOSTIC] Path: {:?}", path);
            // log::info!("[VFS DIAGNOSTIC] Path as string: {:?}", path.to_str());

            // bevy plsssss whyyy
            // HACK: zone_loader.rs relies on a custom asset loader with extension .zone_loader
            let path_str = path
                .to_str()
                .unwrap()
                .trim_end_matches(".no_skin")
                .trim_end_matches(".zmo_texture");

            //log::info!("[VFS DIAGNOSTIC] Normalized path_str: {}", path_str);

            // DIAGNOSTIC: Check if this is a .zone_loader file
            if path_str.ends_with(".zone_loader") {
                //log::info!("[VFS DIAGNOSTIC] ===========================================");
                //log::info!("[VFS DIAGNOSTIC] Processing .zone_loader file: {}", path_str);
                let zone_id = path_str.trim_end_matches(".zone_loader").parse::<u8>().unwrap();
                //log::info!("[VFS DIAGNOSTIC] Parsed zone_id: {}", zone_id);
                let data = vec![zone_id];
                // log::info!("[VFS DIAGNOSTIC] Returning zone_loader data for zone_id: {}", zone_id);
                // log::info!("[VFS DIAGNOSTIC] Data length: {} bytes", data.len());
                // log::info!("[VFS DIAGNOSTIC] ===========================================");
                return Ok(VecReader::new(data));
            }

            // HACK: Exclude shaders from VFS to allow load_internal_asset! to work
            // These are local files in the src/render/shaders directory
            if path_str.contains("shaders/") || path_str.contains("shaders\\") {
                //log::info!("[VFS DEBUG] Path contains 'shaders/', bypassing VFS for: \"{}\"", path_str);
                if let Ok(data) = std::fs::read(path) {
                    //log::info!("[VFS DEBUG] Successfully read shader from local filesystem: \"{}\"", path_str);
                    return Ok(VecReader::new(data));
                }
                log::warn!("[VFS DEBUG] Failed to read shader from local filesystem: \"{}\"", path_str);
            }

            // Try to read from VFS
            //log::info!("[VFS DEBUG] Requesting path: \"{}\"", path_str);
            match self.vfs.open_file(path_str) {
                Ok(file) => {
                    match file {
                        VfsFile::Buffer(buffer) => {
                            let size = buffer.len();
                            log::debug!("[VFS MEMORY] File loaded from VFS buffer: {} (size: {})", path_str, format_bytes(size));
                            
                            // Track read statistics
                            if let Ok(mut stats) = self.read_stats.lock() {
                                stats.log_file_read(path_str, size);
                            }
                            
                            Ok(VecReader::new(buffer))
                        }
                        VfsFile::View(view) => {
                            let size = view.len();
                            let data: Vec<u8> = view.into();
                            log::debug!("[VFS MEMORY] File loaded from VFS view: {} (size: {})", path_str, format_bytes(size));
                            
                            // Track read statistics
                            if let Ok(mut stats) = self.read_stats.lock() {
                                stats.log_file_read(path_str, size);
                            }
                            
                            Ok(VecReader::new(data))
                        }
                    }
                }
                Err(e) => {
                    // Fallback to local filesystem if not found in VFS
                    if let Ok(data) = std::fs::read(path) {
                        //log::info!("[VFS DEBUG] File not in VFS, but found on local filesystem: \"{}\"", path_str);
                        return Ok(VecReader::new(data));
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
    ) -> impl Future<Output = Result<Box<dyn bevy::tasks::futures_lite::Stream<Item = PathBuf> + Send + Unpin + 'static>, AssetReaderError>> + Send {
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
            Ok(Box::new(stream) as Box<dyn bevy::tasks::futures_lite::Stream<Item = PathBuf> + Send + Unpin + 'static>)
        }
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<bool, AssetReaderError>> + Send {
        async move {
            log::info!("[VFS DIAGNOSTIC] is_directory called for path: {:?}", path);
            let path_str = path.to_str().unwrap_or("");
            
            // FIX: Return false for .zone_loader files (they're files, not directories)
            if path_str.ends_with(".zone_loader") {
                //log::info!("[VFS DIAGNOSTIC] Returning false for .zone_loader file: {}", path_str);
                return Ok(false);
            }
            
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
        log::info!("[VFS ASSET READER PLUGIN] ===========================================");
        log::info!("[VFS ASSET READER PLUGIN] build() called, registering VFS as default asset source");

        // Get the VFS from VfsResource instead of holding our own Arc.
        // This eliminates the need for a separate Arc clone for the plugin.
        let vfs = app.world().get_resource::<VfsResource>()
            .expect("VfsResource must be inserted before VfsAssetReaderPlugin is built")
            .vfs
            .clone();
        
        log::info!("[VFS ASSET READER PLUGIN] VFS retrieved from VfsResource, Arc pointer: {:p}", vfs.as_ref());
        log::info!("[VFS ASSET READER PLUGIN] About to call register_asset_source()");

        // Register VFS as the default asset source
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSource::build().with_reader(move || {
                log::info!("[VFS ASSET READER PLUGIN] Creating new VfsAssetIo instance");
                let vfs_clone = vfs.clone();
                Box::new(VfsAssetIo::new(vfs_clone))
            }),
        );

        log::info!("[VFS ASSET READER PLUGIN] register_asset_source() completed successfully");
        log::info!("[VFS ASSET READER PLUGIN] ===========================================");

        // FIX: Register a custom asset source specifically for .zone_loader files
        // This bypasses the file existence check that prevents .zone_loader files from being loaded
        // We need to get the VFS again for this second registration since the first closure captured it
        let vfs_for_zone_loader = app.world().get_resource::<VfsResource>()
            .expect("VfsResource must be inserted before VfsAssetReaderPlugin is built")
            .vfs
            .clone();
        app.register_asset_source(
            AssetSourceId::from("zone_loader"),
            AssetSource::build().with_reader(move || {
                log::info!("[VFS ASSET READER PLUGIN] Creating VfsAssetIo for zone_loader source");
                let vfs_clone = vfs_for_zone_loader.clone();
                Box::new(VfsAssetIo::new(vfs_clone))
            }),
        );
        log::info!("[VFS ASSET READER PLUGIN] zone_loader asset source registered");

        // Add a Startup system to verify the asset source was registered
        app.add_systems(bevy::app::Startup, |asset_server: Res<AssetServer>| {
            log::info!("[VFS ASSET READER PLUGIN] Verifying asset source registration...");
            match asset_server.get_source(AssetSourceId::Default) {
                Ok(source) => {
                    log::info!("[VFS ASSET READER PLUGIN] Default asset source found!");
                    let reader = source.reader();
                    let reader_type = std::any::type_name_of_val(reader);
                    log::info!("[VFS ASSET READER PLUGIN] Reader type: {}", reader_type);
                }
                Err(e) => {
                    log::error!("[VFS ASSET READER PLUGIN] Failed to get default asset source: {:?}", e);
                }
            }
            match asset_server.get_source(AssetSourceId::from("zone_loader")) {
                Ok(source) => {
                    log::info!("[VFS ASSET READER PLUGIN] zone_loader asset source found!");
                    let reader = source.reader();
                    let reader_type = std::any::type_name_of_val(reader);
                    log::info!("[VFS ASSET READER PLUGIN] zone_loader Reader type: {}", reader_type);
                }
                Err(e) => {
                    log::error!("[VFS ASSET READER PLUGIN] Failed to get zone_loader asset source: {:?}", e);
                }
            }
        });
    }
}
