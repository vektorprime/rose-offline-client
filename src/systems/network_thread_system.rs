use std::net::SocketAddr;

use bevy::prelude::{Commands, MessageReader, MessageWriter, Res, Resource};

use rose_game_common::{
    data::Password,
    messages::{client::ClientMessage, server::ServerMessage},
};

use crate::{
    events::{MessageBoxEvent, NetworkEvent},
    protocol::{irose, ProtocolClient},
    resources::{
        GameConnection, LoginConnection, NetworkThread, NetworkThreadMessage, WorldConnection,
    },
};

pub fn start_protocol_client<T: ProtocolClient + Send + Sync + 'static>(
    network_thread: &NetworkThread,
    server_address: SocketAddr,
    construct_client: impl FnOnce(
        SocketAddr,
        tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
        crossbeam_channel::Sender<ServerMessage>,
    ) -> T,
) -> (
    tokio::sync::mpsc::UnboundedSender<ClientMessage>,
    crossbeam_channel::Receiver<ServerMessage>,
) {
    let (server_message_tx, server_message_rx) = crossbeam_channel::unbounded::<ServerMessage>();
    let (client_message_tx, client_message_rx) =
        tokio::sync::mpsc::unbounded_channel::<ClientMessage>();

    network_thread
        .control_tx
        .send(NetworkThreadMessage::RunProtocolClient(Box::new(
            construct_client(server_address, client_message_rx, server_message_tx),
        )))
        .ok();

    (client_message_tx, server_message_rx)
}

pub fn handle_connection_lost<R: Resource>(
    commands: &mut Commands,
    message_box_events: &mut MessageWriter<MessageBoxEvent>,
    server_name: &str,
    error: impl std::fmt::Display,
) {
    log::warn!("{} server connection error: {}", server_name, error);
    message_box_events.write(MessageBoxEvent::Show {
        message: format!("Connection to {} server lost: {}", server_name, error),
        modal: true,
        ok: None,
        cancel: None,
    });
    commands.remove_resource::<R>();
}

pub fn network_thread_system(
    mut commands: Commands,
    network_thread: Res<NetworkThread>,
    mut network_events: MessageReader<NetworkEvent>,
) {
    for event in network_events.read() {
        match *event {
            NetworkEvent::ConnectLogin { ref ip, port } => {
                let server_address = format!("{}:{}", ip, port).parse().unwrap();
                let (client_message_tx, server_message_rx) = start_protocol_client(
                    &network_thread,
                    server_address,
                    irose::LoginClient::new,
                );

                commands
                    .insert_resource(LoginConnection::new(client_message_tx, server_message_rx));
            }
            NetworkEvent::ConnectWorld {
                ref ip,
                port,
                packet_codec_seed,
                login_token,
                ref password,
            } => {
                let server_address = format!("{}:{}", ip, port).parse().unwrap();
                let (client_message_tx, server_message_rx) = start_protocol_client(
                    &network_thread,
                    server_address,
                    |server_address, client_message_rx, server_message_tx| {
                        irose::WorldClient::new(
                            server_address,
                            packet_codec_seed,
                            client_message_rx,
                            server_message_tx,
                        )
                    },
                );

                commands.insert_resource(WorldConnection::new(
                    client_message_tx,
                    server_message_rx,
                    login_token,
                    Password::Plaintext(password.clone()),
                ));
            }
            NetworkEvent::ConnectGame {
                ref ip,
                port,
                packet_codec_seed,
                login_token,
                ref password,
            } => {
                let server_address = format!("{}:{}", ip, port).parse().unwrap();
                let (client_message_tx, server_message_rx) = start_protocol_client(
                    &network_thread,
                    server_address,
                    |server_address, client_message_rx, server_message_tx| {
                        irose::GameClient::new(
                            server_address,
                            packet_codec_seed,
                            client_message_rx,
                            server_message_tx,
                        )
                    },
                );

                commands.insert_resource(GameConnection::new(
                    client_message_tx,
                    server_message_rx,
                    login_token,
                    Password::Plaintext(password.clone()),
                ));
            }
        }
    }
}
