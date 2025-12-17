use bevy::{
    asset::{Asset, AssetLoader, LoadContext, LoadedAsset, AsyncReadExt},
    asset::io::Reader,
    reflect::{TypePath, TypeUuid},
    utils::BoxedFuture,
    window::{CursorIcon},
};

#[derive(Clone, Default)]
pub struct ExeResourceLoader;

#[derive(Debug, TypeUuid, TypePath, Clone, Asset)]
#[uuid = "dda4ba39-576d-4863-a8b4-ca73cedcfbcd"]
pub struct ExeResourceCursor {
    pub cursor: CursorIcon,
}

impl AssetLoader for ExeResourceLoader {
    type Asset = ExeResourceCursor;
    type Settings = ();
    type Error = anyhow::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let file = pelite::PeFile::from_bytes(&bytes)?;
            for cursor in file.resources()?.cursors() {
                let Ok((name, cursor)) = cursor else {
                    continue;
                };
                let pelite::resources::Name::Id(id) = name else {
                    continue;
                };

                let mut buffer = Vec::new();
                cursor.write(&mut buffer)?;

                let mut reader = image::io::Reader::new(std::io::Cursor::new(&buffer));
                reader.set_format(image::ImageFormat::Ico);
                reader.no_limits();
                let dyn_img = reader.decode()?;

                let image::DynamicImage::ImageRgba8(image_buffer) = dyn_img else {
                    return Err(anyhow::anyhow!("Unexpected .ico format"));
                };

                let (hotspot_x, hotspot_y) = cursor.hotspot(0).unwrap();
                let bgra: Vec<u8> = image_buffer
                    .chunks_exact(4)
                    .flat_map(|rgba| [rgba[2], rgba[1], rgba[0], rgba[3]])
                    .collect();

                // TODO: Fix cursor loading for Bevy 0.12
                // let cursor = CursorIcon::Custom(CursorIconCustom {
                //     hotspot_x: hotspot_x as u32,
                //     hotspot_y: hotspot_y as u32,
                //     width: image_buffer.width(),
                //     height: image_buffer.height(),
                //     data: bgra.into(),
                // });

                // For now, just return a default cursor
                // TODO: Implement proper cursor loading
            }

            Ok(ExeResourceCursor {
                cursor: CursorIcon::Default,
            })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["exe"]
    }
}
