use bevy::{
    asset::{Asset, AssetId, AssetLoader, io::Reader, BoxedFuture, LoadContext},
    prelude::{AssetEvent, Assets, EventReader, Local, Res, ResMut},
};

use crate::{
    resources::UiResources,
    ui::widgets::{Dialog, LoadWidget},
};

#[derive(Default)]
pub struct DialogLoader;

impl AssetLoader for DialogLoader {
    type Asset = Dialog;
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
            use bevy::tasks::futures_lite::AsyncReadExt;
            reader.read_to_end(&mut bytes).await?;
            
            let bytes_str = std::str::from_utf8(&bytes)?;
            let dialog: Dialog = quick_xml::de::from_str(bytes_str)?;
            Ok(dialog)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["xml"]
    }
}

pub struct DialogInstance {
    pub filename: String,
    pub instance: Option<Dialog>,
}

impl DialogInstance {
    pub fn new(filename: impl Into<String>) -> DialogInstance {
        DialogInstance {
            filename: filename.into(),
            instance: None,
        }
    }

    pub fn get_mut(
        &mut self,
        dialog_assets: &Assets<Dialog>,
        ui_resources: &UiResources,
    ) -> Option<&mut Dialog> {
        if self.instance.is_none() {
            if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_files[&self.filename]) {
                if dialog.loaded {
                    self.instance = Some(dialog.clone());
                }
            }
        }

        self.instance.as_mut()
    }
}

#[derive(Default)]
pub struct DialogsLoadState {
    pending_dialogs: Vec<AssetId<Dialog>>,
}

pub fn load_dialog_sprites_system(
    mut ev_asset: EventReader<AssetEvent<Dialog>>,
    mut assets: ResMut<Assets<Dialog>>,
    mut load_state: Local<DialogsLoadState>,
    ui_resources: Res<UiResources>,
) {
    for ev in ev_asset.read() {
        match ev {
            AssetEvent::LoadedWithDependencies { id } | AssetEvent::Modified { id } => {
                load_state.pending_dialogs.push(*id);
            }
            _ => {}
        }
    }

    if ui_resources.loaded_all_textures {
        for handle in load_state.pending_dialogs.drain(..) {
            if let Some(dialog) = assets.get_mut(handle) {
                dialog.widgets.load_widget(&ui_resources);
                dialog.loaded = true;
            }
        }
    }
}
