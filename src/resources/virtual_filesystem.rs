use bevy::prelude::Resource;
use std::path::PathBuf;
use std::sync::Arc;

use rose_file_readers::VirtualFilesystem;

#[derive(Resource)]
pub struct VfsResource {
    pub vfs: Arc<VirtualFilesystem>,
    /// The base path where game data is stored on the real filesystem.
    /// This is used for saving files back to the game data directory.
    pub base_path: PathBuf,
}
