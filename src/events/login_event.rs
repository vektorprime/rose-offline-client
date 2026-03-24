use bevy::prelude::Message;

#[derive(Message)]
pub enum LoginEvent {
    Login { username: String, password: String },
    SelectServer { server_id: usize, channel_id: usize },
}
