use bevy::prelude::{Commands, Message};

#[derive(Message)]
pub enum NumberInputDialogEvent {
    Show {
        max_value: Option<usize>,
        modal: bool,
        ok: Option<Box<dyn FnOnce(&mut Commands, usize) + Send + Sync>>,
        cancel: Option<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
    },
}
