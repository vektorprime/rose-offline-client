use bevy::prelude::Message;

#[derive(Message)]
pub enum CharacterSelectEvent {
    SelectCharacter(usize),
    PlaySelected,
    DeleteSelected,
    Disconnect,
}
