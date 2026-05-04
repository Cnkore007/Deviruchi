//! Chat Command Parser
//!
//! Parses chat commands like /w, /whisper, /r, /t for private messaging.

/// Chat command types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatCommand {
    /// Whisper to a player: /w <player> <message>
    Whisper { target: String, message: String },
    /// Reply to last whisper: /r <message>
    Reply { message: String },
    /// Tell (alias for whisper): /t <player> <message>
    Tell { target: String, message: String },
    /// Regular chat (not a command)
    Regular(String),
    /// Unknown or invalid command
    Unknown,
}

impl ChatCommand {
    /// Get the message content if this is a regular message
    pub fn message(&self) -> Option<&str> {
        match self {
            ChatCommand::Regular(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Parse a chat message and extract any commands
///
/// # Arguments
/// * `input` - The raw chat input string
///
/// # Examples
/// ```
/// use deviruchi::game::chat::parser::{parse_chat, ChatCommand};
///
/// // Whisper command
/// let cmd = parse_chat("/w Player2 Hello there!");
/// assert!(matches!(cmd, ChatCommand::Whisper { .. }));
///
/// // Reply command
/// let cmd = parse_chat("/r Thanks for the info!");
/// assert!(matches!(cmd, ChatCommand::Reply { .. }));
///
/// // Regular message
/// let cmd = parse_chat("Hello everyone!");
/// assert!(matches!(cmd, ChatCommand::Regular(_)));
/// ```
pub fn parse_chat(input: &str) -> ChatCommand {
    let input = input.trim();

    if input.is_empty() {
        return ChatCommand::Unknown;
    }

    // Check if it starts with /
    if !input.starts_with('/') {
        return ChatCommand::Regular(input.to_string());
    }

    // Parse the command
    let rest = &input[1..]; // Remove the leading /
    let parts: Vec<&str> = rest.splitn(2, |c: char| c.is_whitespace()).collect();

    if parts.is_empty() {
        return ChatCommand::Unknown;
    }

    let command = parts[0].to_lowercase();
    let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

    match command.as_str() {
        "w" | "whisper" => parse_whisper(args),
        "t" | "tell" => parse_tell(args),
        "r" | "reply" => parse_reply(args),
        _ => ChatCommand::Unknown,
    }
}

/// Parse whisper command: /w <player> <message> or /whisper <player> <message>
fn parse_whisper(args: &str) -> ChatCommand {
    parse_target_message(args).map_or(ChatCommand::Unknown, |(target, msg)| ChatCommand::Whisper {
        target,
        message: msg,
    })
}

/// Parse tell command (alias for whisper): /t <player> <message> or /tell <player> <message>
fn parse_tell(args: &str) -> ChatCommand {
    parse_target_message(args).map_or(ChatCommand::Unknown, |(target, msg)| ChatCommand::Tell {
        target,
        message: msg,
    })
}

/// Parse reply command: /r <message> or /reply <message>
fn parse_reply(args: &str) -> ChatCommand {
    if args.is_empty() {
        return ChatCommand::Unknown;
    }

    ChatCommand::Reply {
        message: args.to_string(),
    }
}

/// Parse target and message from arguments string
/// Format: "<player> <message>"
/// The first word is the target, rest is the message
fn parse_target_message(args: &str) -> Option<(String, String)> {
    let args = args.trim();
    if args.is_empty() {
        return None;
    }

    // Find the first whitespace to split target and message
    // But player names could have spaces, so we need a smarter approach
    // For simplicity, assume first space-separated word is target
    // A more robust implementation would handle quoted names

    // Try to find target (first word) and message (rest)
    if let Some(space_idx) = args.find(|c: char| c.is_whitespace()) {
        let target = args[..space_idx].to_string();
        let message = args[space_idx..].trim().to_string();

        if !target.is_empty() && !message.is_empty() {
            return Some((target, message));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_regular_message() {
        let cmd = parse_chat("Hello everyone!");
        assert!(matches!(cmd, ChatCommand::Regular(msg) if msg == "Hello everyone!"));
    }

    #[test]
    fn test_parse_whisper_short() {
        let cmd = parse_chat("/w Player2 Hello!");
        match cmd {
            ChatCommand::Whisper { target, message } => {
                assert_eq!(target, "Player2");
                assert_eq!(message, "Hello!");
            }
            _ => panic!("Expected Whisper command"),
        }
    }

    #[test]
    fn test_parse_whisper_full() {
        let cmd = parse_chat("/whisper Player2 Hello there!");
        match cmd {
            ChatCommand::Whisper { target, message } => {
                assert_eq!(target, "Player2");
                assert_eq!(message, "Hello there!");
            }
            _ => panic!("Expected Whisper command"),
        }
    }

    #[test]
    fn test_parse_tell_short() {
        let cmd = parse_chat("/t Player2 Hello!");
        match cmd {
            ChatCommand::Tell { target, message } => {
                assert_eq!(target, "Player2");
                assert_eq!(message, "Hello!");
            }
            _ => panic!("Expected Tell command"),
        }
    }

    #[test]
    fn test_parse_tell_full() {
        let cmd = parse_chat("/tell Player2 Hello!");
        match cmd {
            ChatCommand::Tell { target, message } => {
                assert_eq!(target, "Player2");
                assert_eq!(message, "Hello!");
            }
            _ => panic!("Expected Tell command"),
        }
    }

    #[test]
    fn test_parse_reply_short() {
        let cmd = parse_chat("/r Thanks for the info!");
        match cmd {
            ChatCommand::Reply { message } => {
                assert_eq!(message, "Thanks for the info!");
            }
            _ => panic!("Expected Reply command"),
        }
    }

    #[test]
    fn test_parse_reply_full() {
        let cmd = parse_chat("/reply Thanks for the info!");
        match cmd {
            ChatCommand::Reply { message } => {
                assert_eq!(message, "Thanks for the info!");
            }
            _ => panic!("Expected Reply command"),
        }
    }

    #[test]
    fn test_parse_whisper_with_extra_spaces() {
        let cmd = parse_chat("/w   Player2   Hello!");
        match cmd {
            ChatCommand::Whisper { target, message } => {
                assert_eq!(target, "Player2");
                assert_eq!(message, "Hello!");
            }
            _ => panic!("Expected Whisper command"),
        }
    }

    #[test]
    fn test_parse_unknown_command() {
        let cmd = parse_chat("/unknown");
        assert!(matches!(cmd, ChatCommand::Unknown));
    }

    #[test]
    fn test_parse_empty_input() {
        let cmd = parse_chat("");
        assert!(matches!(cmd, ChatCommand::Unknown));
    }

    #[test]
    fn test_parse_whitespace_only() {
        let cmd = parse_chat("   ");
        assert!(matches!(cmd, ChatCommand::Unknown));
    }

    #[test]
    fn test_parse_incomplete_whisper() {
        // No target specified
        let cmd = parse_chat("/w");
        assert!(matches!(cmd, ChatCommand::Unknown));

        // No message specified
        let cmd = parse_chat("/w Player2");
        assert!(matches!(cmd, ChatCommand::Unknown));
    }

    #[test]
    fn test_parse_reply_incomplete() {
        let cmd = parse_chat("/r");
        assert!(matches!(cmd, ChatCommand::Unknown));
    }

    #[test]
    fn test_parse_command_case_insensitive() {
        let cmd = parse_chat("/W Player2 Hello!");
        assert!(matches!(cmd, ChatCommand::Whisper { .. }));

        let cmd = parse_chat("/WHISPER Player2 Hello!");
        assert!(matches!(cmd, ChatCommand::Whisper { .. }));

        let cmd = parse_chat("/R Hello!");
        assert!(matches!(cmd, ChatCommand::Reply { .. }));
    }

    #[test]
    fn test_message_getter() {
        let cmd = parse_chat("Hello!");
        assert_eq!(cmd.message(), Some("Hello!"));

        let cmd = parse_chat("/w Player2 Hi!");
        assert_eq!(cmd.message(), None);
    }
}
