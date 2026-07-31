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
    mut ping_request_events: MessageReader<PingRequestEvent>,
    mut ping_state: ResMut<PingState>,
    mut chatbox_events: MessageWriter<ChatboxEvent>,
) {
    for _event in ping_request_events.read() {
        // Record the timestamp when we sent the ping
        ping_state.pending_ping_timestamp = Some(Instant::now());

        // Send a system message to let the user know we're pinging
        chatbox_events.write(ChatboxEvent::System("Pinging server...".to_string()));
    }
}

/// System that handles ping response from the server.
///
/// This calculates the round-trip time and displays it to the user.
pub fn ping_response_system(
    mut ping_response_events: MessageReader<PingResponseEvent>,
    mut ping_state: ResMut<PingState>,
    mut chatbox_events: MessageWriter<ChatboxEvent>,
) {
    for event in ping_response_events.read() {
        ping_state.last_ping_ms = Some(event.ping_ms);

        // Display the ping result
        let ping_message = format!("Ping: {} ms", event.ping_ms);
        chatbox_events.write(ChatboxEvent::System(ping_message));
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
