use bevy::prelude::Message;

#[derive(Message)]
pub enum ChatboxEvent {
    Say(String, String),
    Shout(String, String),
    Whisper(String, String),
    Party(String, String),
    Clan(String, String),
    Allied(String, String),
    Announce(Option<String>, String),
    System(String),
    Quest(String),
}
