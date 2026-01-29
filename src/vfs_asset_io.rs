use bevy::asset::{
    io::{AssetReader, AssetReaderError, AssetSource, AssetSourceId},
    AssetApp, AssetServer,
};
use bevy::app::App;
use bevy::prelude::{Plugin, Res, Resource};
use std::{
    io::{Cursor, Seek},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use rose_file_readers::{VfsFile, VirtualFilesystem};

struct CursorWrapper {
    data: Vec<u8>,
    position: u64,
}

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
}

impl VfsAssetIo {
    pub fn new(vfs: Arc<VirtualFilesystem>) -> Self {
        log::info!("[VFS ASSET IO] Creating new VfsAssetIo instance");
        Self { vfs }
    }
}

impl AssetReader for VfsAssetIo {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Box<dyn bevy::tasks::futures_lite::AsyncRead + Send + Sync + Unpin + 'a>, AssetReaderError>> + Send + 'a>> {
        Box::pin(async move {
            // Log ALL read calls to see what's happening
            let path_str = path.to_str().unwrap_or("");
            log::info!("[VFS DIAGNOSTIC] ===========================================");
            log::info!("[VFS DIAGNOSTIC] VfsAssetIo::read ENTRY POINT REACHED");
            log::info!("[VFS DIAGNOSTIC] Path: {:?}", path);
            log::info!("[VFS DIAGNOSTIC] Path as string: {:?}", path.to_str());

            // bevy plsssss whyyy
            // HACK: zone_loader.rs relies on a custom asset loader with extension .zone_loader
            let path_str = path
                .to_str()
                .unwrap()
                .trim_end_matches(".no_skin")
                .trim_end_matches(".zmo_texture");

            log::info!("[VFS DIAGNOSTIC] Normalized path_str: {}", path_str);

            // DIAGNOSTIC: Check if this is a .zone_loader file
            if path_str.ends_with(".zone_loader") {
                log::info!("[VFS DIAGNOSTIC] ===========================================");
                log::info!("[VFS DIAGNOSTIC] Processing .zone_loader file: {}", path_str);
                let zone_id = path_str.trim_end_matches(".zone_loader").parse::<u8>().unwrap();
                log::info!("[VFS DIAGNOSTIC] Parsed zone_id: {}", zone_id);
                let data = vec![zone_id];
                log::info!("[VFS DIAGNOSTIC] Returning zone_loader data for zone_id: {}", zone_id);
                log::info!("[VFS DIAGNOSTIC] Data length: {} bytes", data.len());
                log::info!("[VFS DIAGNOSTIC] ===========================================");
                return Ok(Box::new(CursorWrapper::new(data)) as Box<dyn bevy::tasks::futures_lite::AsyncRead + Send + Sync + Unpin + 'a>);
            }

            // Try to read from VFS
            match self.vfs.open_file(path_str) {
                Ok(file) => {
                    match file {
                        VfsFile::Buffer(buffer) => {
                            log::info!("[VFS DIAGNOSTIC] Returning VFS file from buffer for path: {}", path_str);
                            log::info!("[VFS DIAGNOSTIC] Buffer size: {} bytes", buffer.len());
                            log::info!("[VFS DIAGNOSTIC] ===========================================");
                            Ok(Box::new(CursorWrapper::new(buffer)) as Box<dyn bevy::tasks::futures_lite::AsyncRead + Send + Sync + Unpin + 'a>)
                        }
                        VfsFile::View(view) => {
                            log::info!("[VFS DIAGNOSTIC] Returning VFS file from view for path: {}", path_str);
                            log::info!("[VFS DIAGNOSTIC] View size: {} bytes", view.len());
                            log::info!("[VFS DIAGNOSTIC] ===========================================");
                            Ok(Box::new(CursorWrapper::new(view.into())) as Box<dyn bevy::tasks::futures_lite::AsyncRead + Send + Sync + Unpin + 'a>)
                        }
                    }
                }
                Err(e) => {
                    log::warn!("[VFS DIAGNOSTIC] VFS file not found for path: {}", path_str);
                    log::warn!("[VFS DIAGNOSTIC] Error: {:?}", e);
                    log::info!("[VFS DIAGNOSTIC] ===========================================");
                    Err(AssetReaderError::NotFound(path.into()))
                }
            }
        })
    }

    fn read_meta<'a>(
        &'a self,
        _path: &'a Path,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Box<dyn bevy::tasks::futures_lite::AsyncRead + Send + Sync + Unpin + 'a>, AssetReaderError>> + Send + 'a>> {
        Box::pin(async move {
            // Return NotFound for metadata - this is correct behavior since VFS files
            // don't have .meta files. Bevy will use default metadata.
            Err(AssetReaderError::NotFound(_path.into()))
        })
    }

    fn read_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Box<dyn bevy::tasks::futures_lite::Stream<Item = PathBuf> + Send + Unpin + 'static>, AssetReaderError>> + Send + 'a>> {
        Box::pin(async move {
            // FIX: Return empty directory listing to prevent continuous asset reloading.
            // Previously, this returned .zone_loader files for ALL directories, causing
            // Bevy's asset system to continuously discover and reload zone assets,
            // leading to massive memory allocations (2GB/second) as all assets were
            // reloaded repeatedly.
            //
            // Zones should only be loaded via LoadZoneEvent, not through directory scanning.
            let stream = bevy::tasks::futures_lite::stream::iter(Vec::<PathBuf>::new());
            Ok(Box::new(stream) as Box<dyn bevy::tasks::futures_lite::Stream<Item = PathBuf> + Send + Unpin + 'static>)
        })
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<bool, AssetReaderError>> + Send + 'a>> {
        Box::pin(async move {
            log::info!("[VFS DIAGNOSTIC] is_directory called for path: {:?}", path);
            let path_str = path.to_str().unwrap_or("");
            
            // FIX: Return false for .zone_loader files (they're files, not directories)
            if path_str.ends_with(".zone_loader") {
                log::info!("[VFS DIAGNOSTIC] Returning false for .zone_loader file: {}", path_str);
                return Ok(false);
            }
            
            Ok(false)
        })
    }
}

/// Plugin that registers the VFS as the default asset source
/// This must be added AFTER DefaultPlugins to ensure the asset server is properly initialized
pub struct VfsAssetReaderPlugin {
    vfs: Arc<VirtualFilesystem>,
}

impl VfsAssetReaderPlugin {
    pub fn new(vfs: Arc<VirtualFilesystem>) -> Self {
        Self { vfs }
    }
}

impl Plugin for VfsAssetReaderPlugin {
    fn build(&self, app: &mut App) {
        log::info!("[VFS ASSET READER PLUGIN] ===========================================");
        log::info!("[VFS ASSET READER PLUGIN] build() called, registering VFS as default asset source");
        let vfs = self.vfs.clone();
        log::info!("[VFS ASSET READER PLUGIN] VFS Arc pointer: {:p}", vfs.as_ref());
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
        let vfs_for_zone_loader = self.vfs.clone();
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
