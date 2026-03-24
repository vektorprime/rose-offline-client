use bevy::prelude::Message;

use rose_data::ZoneId;

#[derive(Message)]
pub enum GameConnectionEvent {
    Connected(ZoneId),
}
