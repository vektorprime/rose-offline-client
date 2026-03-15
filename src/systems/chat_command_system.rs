use bevy::prelude::*;

/// Chat type enumeration representing different chat channels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatType {
    Normal,
    Shout,
    Whisper,
    Party,
    Clan,
    Allied,
    Trade,
    Help,
}

/// Parsed chat input containing the chat type, target (for whispers), and message
#[derive(Debug, Clone)]
pub struct ParsedChatInput {
    pub chat_type: ChatType,
    pub target: Option<String>,
    pub message: String,
}

/// Prefixes for each chat type - both ASCII and full-width Unicode variants
const SHOUT_PREFIXES: &[char] = &['!', '！'];
const WHISPER_PREFIXES: &[char] = &['@', '＠'];
const PARTY_PREFIXES: &[char] = &['#', '#'];
const CLAN_PREFIXES: &[char] = &['&', '＆'];
const ALLIED_PREFIXES: &[char] = &['~', '〜', '～'];
const TRADE_PREFIXES: &[char] = &['$', '$'];
const HELP_PREFIXES: &[char] = &['?', '？', '/'];

/// Space characters for separating whisper target from message
const SPACE_CHARS: &[char] = &[' ', '\t', ' ', '\u{3000}'];

/// Calculate the byte width of a character for slicing strings
fn char_byte_width(c: char) -> usize {
    c.len_utf8()
}

/// Checks if a message is a help command (starts with / or ?)
pub fn is_help_command(message: &str) -> bool {
    let trimmed = message.trim();
    trimmed.starts_with('/') || trimmed.starts_with('?')
}

/// Parses chat input to determine the chat type and extract target/message
/// 
/// This function handles:
/// - Chat prefixes (!, @, #, &, ~, $) for routing to different chat channels
/// - Unicode variants of prefixes (full-width characters)
/// - Whisper name extraction (@name message format)
/// - Invalid commands default to regular chat
/// 
/// # Arguments
/// * `input` - The raw chat input string
/// 
/// # Returns
/// A ParsedChatInput containing the chat type, optional target (for whispers), and the message
/// 
/// # Examples
/// ```
/// let parsed = parse_chat_input("!Hello everyone!");
/// assert_eq!(parsed.chat_type, ChatType::Shout);
/// assert_eq!(parsed.message, "Hello everyone!");
///
/// let parsed = parse_chat_input("@John Hi there");
/// assert_eq!(parsed.chat_type, ChatType::Whisper);
/// assert_eq!(parsed.target, Some("John".to_string()));
/// assert_eq!(parsed.message, "Hi there");
///
/// let parsed = parse_chat_input("#Party meeting now");
/// assert_eq!(parsed.chat_type, ChatType::Party);
/// assert_eq!(parsed.message, "Party meeting now");
/// ```
pub fn parse_chat_input(input: &str) -> ParsedChatInput {
    let trimmed = input.trim();
    
    // Check if input is empty
    if trimmed.is_empty() {
        return ParsedChatInput {
            chat_type: ChatType::Normal,
            target: None,
            message: String::new(),
        };
    }
    
    let chars: Vec<char> = trimmed.chars().collect();
    let first_char = chars.first().copied();
    
    match first_char {
        Some(prefix) if SHOUT_PREFIXES.contains(&prefix) => {
            // Shout: !message or !message
            let prefix_width = char_byte_width(prefix);
            let message = if chars.len() > 1 {
                trimmed[prefix_width..].trim().to_string()
            } else {
                String::new()
            };
            
            ParsedChatInput {
                chat_type: ChatType::Shout,
                target: None,
                message,
            }
        }
        Some(prefix) if WHISPER_PREFIXES.contains(&prefix) => {
            // Whisper: @name message or @name message
            let prefix_width = char_byte_width(prefix);
            
            if chars.len() <= 1 {
                // No target or message, return as normal chat
                return ParsedChatInput {
                    chat_type: ChatType::Normal,
                    target: None,
                    message: trimmed.to_string(),
                };
            }
            
            // Find the first space character to separate target from message
            let remaining = &trimmed[prefix_width..];
            let mut target_end = remaining.len();
            let mut found_space = false;
            
            for (i, ch) in remaining.char_indices() {
                if SPACE_CHARS.contains(&ch) {
                    target_end = i;
                    found_space = true;
                    break;
                }
            }
            
            let target = &remaining[..target_end];
            let message = if found_space {
                remaining[target_end..].trim().to_string()
            } else {
                // No space found, treat the whole thing as target (no message)
                // or check if it's just a name
                String::new()
            };
            
            ParsedChatInput {
                chat_type: ChatType::Whisper,
                target: if !target.is_empty() {
                    Some(target.to_string())
                } else {
                    None
                },
                message,
            }
        }
        Some(prefix) if PARTY_PREFIXES.contains(&prefix) => {
            // Party: #message or #message
            let prefix_width = char_byte_width(prefix);
            let message = if chars.len() > 1 {
                trimmed[prefix_width..].trim().to_string()
            } else {
                String::new()
            };
            
            ParsedChatInput {
                chat_type: ChatType::Party,
                target: None,
                message,
            }
        }
        Some(prefix) if CLAN_PREFIXES.contains(&prefix) => {
            // Clan: &message or ＆message
            let prefix_width = char_byte_width(prefix);
            let message = if chars.len() > 1 {
                trimmed[prefix_width..].trim().to_string()
            } else {
                String::new()
            };
            
            ParsedChatInput {
                chat_type: ChatType::Clan,
                target: None,
                message,
            }
        }
        Some(prefix) if ALLIED_PREFIXES.contains(&prefix) => {
            // Allied/Shout: ~message or 〜message or ～message
            let prefix_width = char_byte_width(prefix);
            let message = if chars.len() > 1 {
                trimmed[prefix_width..].trim().to_string()
            } else {
                String::new()
            };
            
            ParsedChatInput {
                chat_type: ChatType::Allied,
                target: None,
                message,
            }
        }
        Some(prefix) if TRADE_PREFIXES.contains(&prefix) => {
            // Trade: $message or $message
            let prefix_width = char_byte_width(prefix);
            let message = if chars.len() > 1 {
                trimmed[prefix_width..].trim().to_string()
            } else {
                String::new()
            };
            
            ParsedChatInput {
                chat_type: ChatType::Trade,
                target: None,
                message,
            }
        }
        Some(prefix) if HELP_PREFIXES.contains(&prefix) => {
            // Help command: /help or ?help
            // These are server commands, return as normal chat
            ParsedChatInput {
                chat_type: ChatType::Help,
                target: None,
                message: trimmed.to_string(),
            }
        }
        _ => {
            // Normal chat
            ParsedChatInput {
                chat_type: ChatType::Normal,
                target: None,
                message: trimmed.to_string(),
            }
        }
    }
}

/// Converts a ParsedChatInput to a ClientMessage for sending to the server
/// 
/// # Arguments
/// * `parsed` - The parsed chat input
/// * `username` - The username of the sender (for whispers)
/// 
/// # Returns
/// A ClientMessage that can be sent to the server
impl ParsedChatInput {
    pub fn as_client_message(&self) -> &'static str {
        // This is a placeholder - the actual message should be built by the caller
        // The chat_command_system just parses, the ui_chatbox_system handles sending
        ""
    }
    
    /// Check if this is a client-side only command
    pub fn is_client_command(&self) -> bool {
        matches!(self.chat_type, ChatType::Help) && {
            let msg = self.message.to_lowercase();
            msg == "/fly" || msg == "/ping"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shout_prefix() {
        let parsed = parse_chat_input("!Hello everyone!");
        assert_eq!(parsed.chat_type, ChatType::Shout);
        assert_eq!(parsed.target, None);
        assert_eq!(parsed.message, "Hello everyone!");
    }

    #[test]
    fn test_shout_prefix_unicode() {
        let parsed = parse_chat_input("!Hello!");
        assert_eq!(parsed.chat_type, ChatType::Shout);
        assert_eq!(parsed.message, "Hello!");
    }

    #[test]
    fn test_whisper_with_target() {
        let parsed = parse_chat_input("@John Hi there");
        assert_eq!(parsed.chat_type, ChatType::Whisper);
        assert_eq!(parsed.target, Some("John".to_string()));
        assert_eq!(parsed.message, "Hi there");
    }

    #[test]
    fn test_whisper_without_message() {
        let parsed = parse_chat_input("@John");
        assert_eq!(parsed.chat_type, ChatType::Whisper);
        assert_eq!(parsed.target, Some("John".to_string()));
        assert_eq!(parsed.message, "");
    }

    #[test]
    fn test_whisper_prefix_only() {
        let parsed = parse_chat_input("@");
        assert_eq!(parsed.chat_type, ChatType::Normal);
        assert_eq!(parsed.message, "@");
    }

    #[test]
    fn test_party_prefix() {
        let parsed = parse_chat_input("#Party meeting now");
        assert_eq!(parsed.chat_type, ChatType::Party);
        assert_eq!(parsed.target, None);
        assert_eq!(parsed.message, "Party meeting now");
    }

    #[test]
    fn test_clan_prefix() {
        let parsed = parse_chat_input("&Clan announcement");
        assert_eq!(parsed.chat_type, ChatType::Clan);
        assert_eq!(parsed.target, None);
        assert_eq!(parsed.message, "Clan announcement");
    }

    #[test]
    fn test_clan_prefix_unicode() {
        let parsed = parse_chat_input("＆Clan message");
        assert_eq!(parsed.chat_type, ChatType::Clan);
        assert_eq!(parsed.message, "Clan message");
    }

    #[test]
    fn test_allied_prefix() {
        let parsed = parse_chat_input("~Alliance greeting");
        assert_eq!(parsed.chat_type, ChatType::Allied);
        assert_eq!(parsed.target, None);
        assert_eq!(parsed.message, "Alliance greeting");
    }

    #[test]
    fn test_allied_prefix_unicode_var1() {
        let parsed = parse_chat_input("〜Alliance!");
        assert_eq!(parsed.chat_type, ChatType::Allied);
        assert_eq!(parsed.message, "Alliance!");
    }

    #[test]
    fn test_allied_prefix_unicode_var2() {
        let parsed = parse_chat_input("～Alliance!");
        assert_eq!(parsed.chat_type, ChatType::Allied);
        assert_eq!(parsed.message, "Alliance!");
    }

    #[test]
    fn test_normal_chat() {
        let parsed = parse_chat_input("Hello world");
        assert_eq!(parsed.chat_type, ChatType::Normal);
        assert_eq!(parsed.target, None);
        assert_eq!(parsed.message, "Hello world");
    }

    #[test]
    fn test_command_prefix() {
        let parsed = parse_chat_input("/help");
        assert_eq!(parsed.chat_type, ChatType::Help);
        assert_eq!(parsed.message, "/help");
    }

    #[test]
    fn test_command_prefix_question() {
        let parsed = parse_chat_input("?help");
        assert_eq!(parsed.chat_type, ChatType::Help);
        assert_eq!(parsed.message, "?help");
    }

    #[test]
    fn test_empty_input() {
        let parsed = parse_chat_input("");
        assert_eq!(parsed.chat_type, ChatType::Normal);
        assert_eq!(parsed.message, "");
    }

    #[test]
    fn test_whitespace_only() {
        let parsed = parse_chat_input("   ");
        assert_eq!(parsed.chat_type, ChatType::Normal);
        assert_eq!(parsed.message, "");
    }

    #[test]
    fn test_whitespace_handling() {
        let parsed = parse_chat_input("!  Hello  ");
        assert_eq!(parsed.chat_type, ChatType::Shout);
        assert_eq!(parsed.message, "Hello");
    }

    #[test]
    fn test_target_with_underscore() {
        let parsed = parse_chat_input("@player_1 Hello");
        assert_eq!(parsed.chat_type, ChatType::Whisper);
        assert_eq!(parsed.target, Some("player_1".to_string()));
        assert_eq!(parsed.message, "Hello");
    }

    #[test]
    fn test_target_with_numbers() {
        let parsed = parse_chat_input("@User123 Message");
        assert_eq!(parsed.chat_type, ChatType::Whisper);
        assert_eq!(parsed.target, Some("User123".to_string()));
        assert_eq!(parsed.message, "Message");
    }

    #[test]
    fn test_is_help_command() {
        assert!(is_help_command("/help"));
        assert!(is_help_command("?commands"));
        assert!(is_help_command("/fly"));
        assert!(!is_help_command("hello"));
        assert!(!is_help_command("!shout"));
    }
}
