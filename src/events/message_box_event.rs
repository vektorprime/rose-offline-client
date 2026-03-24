use bevy::prelude::{Commands, Message};

#[derive(Message)]
pub enum MessageBoxEvent {
    Show {
        message: String,
        modal: bool,
        ok: Option<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
        cancel: Option<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
    },
}
