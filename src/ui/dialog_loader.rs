use bevy::{
    asset::{io::Reader, AssetId, AssetLoader, LoadContext},
    prelude::{AssetEvent, Assets, Local, MessageReader, Res, ResMut, TypePath},
};

use crate::{
    resources::UiResources,
    ui::widgets::{Dialog, LoadWidget},
};

#[derive(Default, TypePath)]
pub struct DialogLoader;

impl AssetLoader for DialogLoader {
    type Asset = Dialog;
    type Settings = ();
    type Error = anyhow::Error;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> impl std::future::Future<Output = Result<Self::Asset, Self::Error>> + Send {
        async move {
            let path = load_context.path().path().to_string_lossy().to_string();

            let mut bytes = Vec::new();
            use bevy::tasks::futures_lite::AsyncReadExt;
            reader.read_to_end(&mut bytes).await?;

            log::debug!(
                "[DIALOG LOADER] Loading dialog: {}, size: {} bytes",
                path,
                bytes.len(),
            );

            let bytes_str = std::str::from_utf8(&bytes)?;
            let dialog: Dialog = quick_xml::de::from_str(bytes_str)?;
            Ok(dialog)
        }
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
    mut ev_asset: MessageReader<AssetEvent<Dialog>>,
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

    if ui_resources.loaded_required_textures {
        for handle in load_state.pending_dialogs.drain(..) {
            if let Some(dialog) = assets.get_mut(handle) {
                dialog.widgets.load_widget(&ui_resources);
                dialog.loaded = true;
            }
        }
    } else {
        log::warn!("[DIALOG SYSTEM] loaded_required_textures=false, deferring widget loading for {} pending dialogs", load_state.pending_dialogs.len());
    }
}
