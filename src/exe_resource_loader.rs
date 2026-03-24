use std::future::Future;

use bevy::{
    asset::{Asset, AssetLoader, io::Reader, LoadContext},
    reflect::{TypePath},
    tasks::futures_lite::AsyncReadExt,
};

#[derive(Clone, Default, TypePath)]
pub struct ExeResourceLoader;

/// Cursor image data loaded from .exe resource
#[derive(Debug, TypePath, Clone, Asset)]
pub struct ExeResourceCursor {
    /// Whether the cursor has been processed
    pub processed: bool,
}

impl AssetLoader for ExeResourceLoader {
    type Asset = ExeResourceCursor;
    type Settings = ();
    type Error = anyhow::Error;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> impl std::future::Future<Output = Result<Self::Asset, Self::Error>> + Send {
        async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            let path = load_context.path().path().to_string_lossy().to_string();
            
            // Note: Custom cursor loading is handled directly in ui_resources.rs
            // to properly use Bevy 0.16's CustomCursorImage API
            log::debug!("[EXE RESOURCE LOADER] Cursor requested from {}: {} bytes", path, bytes.len());
            
            Ok(ExeResourceCursor { 
                processed: true
            })
        }
    }

    fn extensions(&self) -> &[&str] {
        &["exe"]
    }
}
