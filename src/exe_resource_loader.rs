use std::future::Future;

use bevy::{
    asset::{Asset, AssetLoader, io::Reader, LoadContext},
    reflect::{TypePath},
    tasks::futures_lite::AsyncReadExt,
};
use bevy_egui::egui::CursorIcon;

#[derive(Clone, Default)]
pub struct ExeResourceLoader;

#[derive(Debug, TypePath, Clone, Asset)]
pub struct ExeResourceCursor {
    pub cursor: CursorIcon,
}

impl AssetLoader for ExeResourceLoader {
    type Asset = ExeResourceCursor;
    type Settings = ();
    type Error = anyhow::Error;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> impl std::future::Future<Output = Result<Self::Asset, Self::Error>> + Send {
        async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            // TODO: CustomCursor was removed in Bevy 0.15
            // Need to find new way to load custom cursors
            // For now, just return Default cursor
            let cursor = CursorIcon::Default;

            Ok(ExeResourceCursor { cursor })
        }
    }

    fn extensions(&self) -> &[&str] {
        &["exe"]
    }
}
