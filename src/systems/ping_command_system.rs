use std::time::Instant;

use bevy::prelude::*;

use crate::events::{ChatboxEvent, PingRequestEvent, PingResponseEvent, PingState};

/// Checks if a chat message is a ping command (case-insensitive)
/// Returns true if the message is a "/ping" command and should be consumed
pub fn is_ping_command(message: &str) -> bool {
    let trimmed = message.trim();
    trimmed.eq_ignore_ascii_case("/ping")
}

/// System that handles ping command detection and initiates ping measurement.
/// 
/// This system listens for PingRequestEvent and sends a ping message to the server
/// while recording the timestamp for RTT calculation.
pub fn ping_command_system(
    mut ping_request_events: EventReader<PingRequestEvent>,
    mut ping_state: ResMut<PingState>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
) {
    for _event in ping_request_events.read() {
        // Record the timestamp when we sent the ping
        ping_state.pending_ping_timestamp = Some(Instant::now());
        
        // Send a system message to let the user know we're pinging
        chatbox_events.write(ChatboxEvent::System(
            "Pinging server...".to_string()
        ));
    }
}

/// System that handles ping response from the server.
/// 
/// This calculates the round-trip time and displays it to the user.
pub fn ping_response_system(
    mut ping_response_events: EventReader<PingResponseEvent>,
    mut ping_state: ResMut<PingState>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
) {
    for event in ping_response_events.read() {
        ping_state.last_ping_ms = Some(event.ping_ms);
        
        // Display the ping result
        let ping_message = format!("Ping: {} ms", event.ping_ms);
        chatbox_events.write(ChatboxEvent::System(ping_message));
    }
}

/// System that processes server responses and calculates ping.
/// This should be called when we receive any server message if we have a pending ping.
pub fn ping_measurement_system(
    mut ping_state: ResMut<PingState>,
    mut ping_response_events: EventWriter<PingResponseEvent>,
    game_connection: Option<Res<crate::resources::GameConnection>>,
) {
    // If we have a pending ping and receive any server message, calculate RTT
    if let Some(timestamp) = ping_state.pending_ping_timestamp {
        // Check if we received a response from the server
        if let Some(game_connection) = game_connection.as_ref() {
            // Try to receive a message - if successful, we got a response
            if game_connection.server_message_rx.try_recv().is_ok() {
                let elapsed = timestamp.elapsed();
                let ping_ms = elapsed.as_millis() as u64;
                
                // Clear the pending ping
                ping_state.pending_ping_timestamp = None;
                
                // Send the response event
                ping_response_events.write(PingResponseEvent { ping_ms });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ping_command_valid() {
        assert!(is_ping_command("/ping"));
        assert!(is_ping_command("/PING"));
        assert!(is_ping_command("/Ping"));
        assert!(is_ping_command("/pInG"));
        assert!(is_ping_command(" /ping "));
        assert!(is_ping_command("\t/ping\t"));
    }

    #[test]
    fn test_is_ping_command_invalid() {
        assert!(!is_ping_command("/ping "));
        assert!(!is_ping_command(" /ping extra"));
        assert!(!is_ping_command("/ping extra"));
        assert!(!is_ping_command("/pingg"));
        assert!(!is_ping_command("ping"));
        assert!(!is_ping_command("/pin"));
        assert!(!is_ping_command("hello"));
        assert!(!is_ping_command(""));
    }
}
