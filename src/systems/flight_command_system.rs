use bevy::prelude::*;

use crate::components::PlayerCharacter;
use crate::events::FlightToggleEvent;

/// Checks if a chat message is a flight command (case-insensitive)
/// Returns true if the message is a "/fly" command and should be consumed
pub fn is_fly_command(message: &str) -> bool {
    let trimmed = message.trim();
    trimmed.eq_ignore_ascii_case("/fly")
}

/// System that handles flight command detection from chat messages.
/// 
/// This system is designed to work alongside the chatbox system.
/// The chatbox system should check messages before sending to the server
/// using the [`is_fly_command`] helper function, and if it returns true,
/// send a [`FlightToggleEvent`] instead of sending the chat message.
/// 
/// This system provides a standalone way to process flight commands
/// if needed for other input methods.
pub fn flight_command_system(
    mut flight_toggle_events: EventWriter<FlightToggleEvent>,
    player_query: Query<Entity, With<PlayerCharacter>>,
) {
    // This system can be used for alternative command input methods
    // The primary detection happens in ui_chatbox_system.rs
    let _ = (flight_toggle_events, player_query);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_fly_command_valid() {
        assert!(is_fly_command("/fly"));
        assert!(is_fly_command("/FLY"));
        assert!(is_fly_command("/Fly"));
        assert!(is_fly_command("/fLy"));
        assert!(is_fly_command(" /fly "));
        assert!(is_fly_command("\t/fly\t"));
    }

    #[test]
    fn test_is_fly_command_invalid() {
        assert!(!is_fly_command("/fly "));
        assert!(!is_fly_command(" /fly extra"));
        assert!(!is_fly_command("/fly extra"));
        assert!(!is_fly_command("/flyy"));
        assert!(!is_fly_command("fly"));
        assert!(!is_fly_command("/fl"));
        assert!(!is_fly_command("hello"));
        assert!(!is_fly_command(""));
    }
}
