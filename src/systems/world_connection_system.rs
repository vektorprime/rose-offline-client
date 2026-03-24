use bevy::prelude::{Commands, MessageWriter, NextState, Res, ResMut, State};

use rose_game_common::messages::{client::ClientMessage, server::ServerMessage};
use rose_network_common::ConnectionError;

use crate::{
    events::{MessageBoxEvent, NetworkEvent, WorldConnectionEvent},
    resources::{Account, AppState, CharacterList, WorldConnection},
};

pub fn world_connection_system(
    mut commands: Commands,
    world_connection: Option<Res<WorldConnection>>,
    account: Option<Res<Account>>,
    app_state_current: Res<State<AppState>>,
    mut app_state_next: ResMut<NextState<AppState>>,
    mut network_events: MessageWriter<NetworkEvent>,
    mut world_connection_events: MessageWriter<WorldConnectionEvent>,
    mut message_box_events: MessageWriter<MessageBoxEvent>,
) {
    let world_connection = if let Some(world_connection) = world_connection {
        world_connection
    } else {
        return;
    };

    let account = if let Some(account) = account {
        account
    } else {
        return;
    };

    let result: Result<(), anyhow::Error> = loop {
        match world_connection.server_message_rx.try_recv() {
            Ok(ServerMessage::ConnectionRequestSuccess {
                packet_sequence_id: _,
            }) => {
                world_connection
                    .client_message_tx
                    .send(ClientMessage::GetCharacterList)
                    .ok();
            }
            Ok(ServerMessage::ConnectionRequestError { error: _ }) => {
                break Err(ConnectionError::ConnectionLost.into());
            }
            Ok(ServerMessage::CharacterList {
                character_list: characters,
            }) => {
                if !matches!(app_state_current.get(), AppState::GameCharacterSelect) {
                    app_state_next.set(AppState::GameCharacterSelect);
                }

                commands.insert_resource(CharacterList { characters });
            }
            Ok(ServerMessage::SelectCharacterSuccess {
                login_token,
                packet_codec_seed,
                ip,
                port,
            }) => {
                network_events.write(NetworkEvent::ConnectGame {
                    ip,
                    port,
                    packet_codec_seed,
                    login_token,
                    password: account.password.clone(),
                });
            }
            Ok(ServerMessage::SelectCharacterError) => {
                break Err(ConnectionError::ConnectionLost.into());
            }
            Ok(ServerMessage::CreateCharacterSuccess { character_slot }) => {
                world_connection_events
                    .write(WorldConnectionEvent::CreateCharacterSuccess { character_slot });
            }
            Ok(ServerMessage::CreateCharacterError { error }) => {
                world_connection_events.write(WorldConnectionEvent::CreateCharacterError { error });
            }
            Ok(ServerMessage::DeleteCharacterStart { name, delete_time }) => {
                world_connection_events
                    .write(WorldConnectionEvent::DeleteCharacterStart { name, delete_time });
            }
            Ok(ServerMessage::DeleteCharacterCancel { name }) => {
                world_connection_events.write(WorldConnectionEvent::DeleteCharacterCancel { name });
            }
            Ok(ServerMessage::DeleteCharacterError { name }) => {
                world_connection_events.write(WorldConnectionEvent::DeleteCharacterError { name });
            }
            // ServerMessage::ReturnToCharacterSelect
            Ok(message) => {
                log::warn!("Received unexpected world server message: {:#?}", message);
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                break Err(ConnectionError::ConnectionLost.into());
            }
            Err(crossbeam_channel::TryRecvError::Empty) => break Ok(()),
        }
    };

    if let Err(error) = result {
        log::warn!("World server connection error: {}", error);
        message_box_events.write(MessageBoxEvent::Show {
            message: format!("Connection to world server lost: {}", error),
            modal: true,
            ok: None,
            cancel: None,
        });
        commands.remove_resource::<WorldConnection>();
    }
}
