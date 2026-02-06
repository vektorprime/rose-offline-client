use bevy::{
    asset::{AssetId, AssetLoader, io::Reader, LoadContext},
    prelude::{AssetEvent, Assets, EventReader, Local, Res, ResMut},
};

use crate::{
    resources::UiResources,
    ui::widgets::{Dialog, LoadWidget},
};

#[derive(Default)]
pub struct DialogLoader;

/// Counter to track how many times dialog assets are loaded
static mut DIALOG_LOAD_COUNT: usize = 0;
static mut DIALOG_LOAD_BYTES: usize = 0;

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
            let path = load_context.path().to_string_lossy().to_string();
            
            // SAFETY: This is only for diagnostic logging during single-threaded asset loading
            unsafe {
                DIALOG_LOAD_COUNT += 1;
                if DIALOG_LOAD_COUNT % 100 == 0 {
                    log::warn!(
                        "[DIALOG LOADER] WARNING: DialogLoader invoked {} times, total bytes loaded: {} MB",
                        DIALOG_LOAD_COUNT,
                        DIALOG_LOAD_BYTES / 1024 / 1024
                    );
                }
            }
            
            let mut bytes = Vec::new();
            use bevy::tasks::futures_lite::AsyncReadExt;
            reader.read_to_end(&mut bytes).await?;
            
            // SAFETY: Diagnostic logging
            unsafe {
                DIALOG_LOAD_BYTES += bytes.len();
            }
            
            log::debug!(
                "[DIALOG LOADER] Loading dialog: {}, size: {} bytes (load count: {})",
                path,
                bytes.len(),
                unsafe { DIALOG_LOAD_COUNT }
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
    mut ev_asset: EventReader<AssetEvent<Dialog>>,
    mut assets: ResMut<Assets<Dialog>>,
    mut load_state: Local<DialogsLoadState>,
    ui_resources: Res<UiResources>,
) {
    let mut loaded_count = 0;
    let mut modified_count = 0;

    for ev in ev_asset.read() {
        match ev {
            AssetEvent::LoadedWithDependencies { id } => {
                log::debug!("[DIALOG SYSTEM] Dialog loaded: {:?}", id);
                load_state.pending_dialogs.push(*id);
                loaded_count += 1;
            }
            AssetEvent::Modified { id } => {
                log::warn!("[DIALOG SYSTEM] Dialog MODIFIED event received: {:?} - this may indicate repeated reloads!", id);
                load_state.pending_dialogs.push(*id);
                modified_count += 1;
            }
            AssetEvent::Removed { id } => {
                log::debug!("[DIALOG SYSTEM] Dialog removed: {:?}", id);
            }
            AssetEvent::Added { id } => {
                log::debug!("[DIALOG SYSTEM] Dialog added: {:?}", id);
            }
            _ => {
                // Other unused events
            }
        }
    }

    if loaded_count > 0 || modified_count > 0 {
        log::info!("[DIALOG SYSTEM] Processing {} Loaded and {} Modified dialog events", loaded_count, modified_count);
    }

    if ui_resources.loaded_required_textures {
        log::debug!("[DIALOG SYSTEM] loaded_required_textures=true, processing {} pending dialogs", load_state.pending_dialogs.len());
        for handle in load_state.pending_dialogs.drain(..) {
            if let Some(dialog) = assets.get_mut(handle) {
                log::debug!("[DIALOG SYSTEM] Loading widgets for dialog with {} widgets", dialog.widgets.len());
                dialog.widgets.load_widget(&ui_resources);
                dialog.loaded = true;
                log::debug!("[DIALOG SYSTEM] Dialog loaded and marked as loaded: {:?}", handle);
            }
        }
    } else {
        log::debug!("[DIALOG SYSTEM] loaded_required_textures=false, deferring widget loading for {} pending dialogs", load_state.pending_dialogs.len());
    }
}
