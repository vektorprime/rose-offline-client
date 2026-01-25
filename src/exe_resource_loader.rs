use bevy::{
    asset::{Asset, AssetLoader, io::Reader, LoadContext},
    reflect::{TypePath},
    utils::BoxedFuture,
    tasks::futures_lite::AsyncReadExt,
    window::CursorIcon,
};

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

    fn load<'a, 'b>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext<'b>,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            // TODO: CursorIcon::Custom was removed in Bevy 0.13
            // Need to find new way to load custom cursors
            // For now, just return Default cursor
            let cursor = CursorIcon::Default;

            Ok(ExeResourceCursor { cursor })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["exe"]
    }
}
