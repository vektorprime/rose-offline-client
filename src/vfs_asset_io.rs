use bevy::asset::io::{AssetReader, AssetReaderError, PathStream, Reader};
use bevy::utils::BoxedFuture;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rose_file_readers::{VfsFile, VirtualFilesystem};

pub struct VfsAssetReader {
    vfs: Arc<VirtualFilesystem>,
}

impl VfsAssetReader {
    pub fn new(vfs: Arc<VirtualFilesystem>) -> Self {
        Self { vfs }
    }
}

impl AssetReader for VfsAssetReader {
    fn read<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        let vfs = self.vfs.clone();
        Box::pin(async move {
            // bevy plsssss whyyy
            // HACK: zone_loader.rs relies on a custom asset loader with extension .zone_loader
            let path = path
                .to_str()
                .unwrap()
                .trim_end_matches(".no_skin")
                .trim_end_matches(".zmo_texture");
            if path.ends_with(".zone_loader") {
                let zone_id = path.trim_end_matches(".zone_loader").parse::<u8>().unwrap();
                Ok(Box::new(Reader::from_bytes(vec![zone_id])))
            } else if let Ok(file) = vfs.open_file(path) {
                match file {
                    VfsFile::Buffer(buffer) => Ok(Box::new(Reader::from_bytes(buffer))),
                    VfsFile::View(view) => Ok(Box::new(Reader::from_bytes(view.into()))),
                }
            } else {
                Err(AssetReaderError::NotFound(path.into()))
            }
        })
    }

    fn read_meta<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(async move { Err(AssetReaderError::NotFound(path.into())) })
    }

    fn read_directory<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        Box::pin(async move { Ok(Box::new(PathStream::empty())) })
    }

    fn is_directory<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<bool, AssetReaderError>> {
        Box::pin(async move { Ok(false) })
    }
}
