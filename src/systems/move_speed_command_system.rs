use bevy::prelude::*;

use crate::components::PlayerCharacter;
use crate::events::MoveSpeedSetEvent;

/// Checks if a chat message is a move speed command (case-insensitive)
/// Returns Some(speed) if the message is a "/mspeed <value>" command, None otherwise
pub fn parse_move_speed_command(message: &str) -> Option<f32> {
    let trimmed = message.trim();
    let lower = trimmed.to_lowercase();
    
    if !lower.starts_with("/mspeed") {
        return None;
    }
    
    // Parse the speed value after "/mspeed"
    let rest = trimmed[7..].trim();
    
    if rest.is_empty() {
        return None;
    }
    
    rest.parse::<f32>().ok()
}

/// System that handles move speed command detection from chat messages.
/// 
/// This system is designed to work alongside the chatbox system.
/// The chatbox system should check messages before sending to the server
/// using the [`parse_move_speed_command`] helper function, and if it returns Some,
/// send a [`MoveSpeedSetEvent`] instead of sending the chat message.
/// 
/// This system provides a standalone way to process move speed commands
/// if needed for other input methods.
pub fn move_speed_command_system(
    mut move_speed_events: EventWriter<MoveSpeedSetEvent>,
    player_query: Query<Entity, With<PlayerCharacter>>,
) {
    // This system can be used for alternative command input methods
    // The primary detection happens in ui_chatbox_system.rs
    let _ = (move_speed_events, player_query);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_move_speed_command_valid() {
        assert_eq!(parse_move_speed_command("/mspeed 1000"), Some(1000.0));
        assert_eq!(parse_move_speed_command("/mspeed 500"), Some(500.0));
        assert_eq!(parse_move_speed_command("/mspeed 1.5"), Some(1.5));
        assert_eq!(parse_move_speed_command("/MSPEED 1000"), Some(1000.0));
        assert_eq!(parse_move_speed_command("/Mspeed 1000"), Some(1000.0));
        assert_eq!(parse_move_speed_command(" /mspeed 1000 "), Some(1000.0));
        assert_eq!(parse_move_speed_command("/mspeed 100.5"), Some(100.5));
        assert_eq!(parse_move_speed_command("/mspeed 0"), Some(0.0));
    }

    #[test]
    fn test_parse_move_speed_command_invalid() {
        assert_eq!(parse_move_speed_command("/mspeed"), None);
        assert_eq!(parse_move_speed_command("/mspeed "), None);
        assert_eq!(parse_move_speed_command("/mspeed abc"), None);
        assert_eq!(parse_move_speed_command("/mspeed 1000 extra"), None);
        assert_eq!(parse_move_speed_command("/mspeedy 1000"), None);
        assert_eq!(parse_move_speed_command("mspeed 1000"), None);
        assert_eq!(parse_move_speed_command("/speed 1000"), None);
        assert_eq!(parse_move_speed_command(""), None);
        assert_eq!(parse_move_speed_command("hello"), None);
    }
}
