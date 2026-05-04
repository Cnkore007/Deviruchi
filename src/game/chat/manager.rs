//! Chat Manager - Unified Interface
//!
//! Provides a unified interface for all chat operations including
//! map chat, party chat, and whisper messaging.

use std::sync::Arc;
use uuid::Uuid;

use crate::game::map::{ChannelBus, ChatType, GameEvent, MapState, Player};

use super::parser::{ChatCommand, parse_chat};
use super::rate_limit::WhisperRateLimiter;
use super::whisper::{OfflineMessage, WhisperManager, WhisperResult};

/// Unified chat manager for all chat operations
pub struct ChatManager {
    /// Whisper manager for private messaging
    whisper_manager: Arc<WhisperManager>,
    /// Rate limiter for whispers
    rate_limiter: WhisperRateLimiter,
}

impl ChatManager {
    /// Create a new ChatManager
    pub fn new() -> Self {
        Self {
            whisper_manager: Arc::new(WhisperManager::new()),
            rate_limiter: WhisperRateLimiter::default_mmorpg(),
        }
    }

    /// Create a ChatManager with custom whisper settings
    pub fn with_whisper_config(max_messages: u32, window_secs: u64) -> Self {
        Self {
            whisper_manager: Arc::new(WhisperManager::new()),
            rate_limiter: WhisperRateLimiter::new(max_messages, window_secs),
        }
    }

    /// Get the whisper manager
    pub fn whisper_manager(&self) -> Arc<WhisperManager> {
        self.whisper_manager.clone()
    }

    /// Handle a chat message from a player
    ///
    /// Parses the message for commands and routes appropriately.
    /// Returns the result of the operation.
    pub fn handle_message(
        &self,
        player: &Player,
        message: &str,
        _map_state: &MapState,
        channel_bus: &ChannelBus,
    ) -> ChatResult {
        let parsed = parse_chat(message);

        match parsed {
            ChatCommand::Whisper { target, message } => {
                self.handle_whisper(player, &target, &message, channel_bus)
            }
            ChatCommand::Tell { target, message } => {
                self.handle_whisper(player, &target, &message, channel_bus)
            }
            ChatCommand::Reply { message } => self.handle_reply(player, &message, channel_bus),
            ChatCommand::Regular(_) => {
                // Regular chat - this would be handled by the map/party system
                ChatResult::RegularMessage(message.to_string())
            }
            ChatCommand::Unknown => ChatResult::UnknownCommand,
        }
    }

    /// Handle a whisper message
    fn handle_whisper(
        &self,
        player: &Player,
        target: &str,
        message: &str,
        channel_bus: &ChannelBus,
    ) -> ChatResult {
        // Check rate limit
        if !self.rate_limiter.check(&player.id) {
            let reset_in = self.rate_limiter.reset_in(&player.id);
            return ChatResult::RateLimited {
                reset_in_secs: reset_in.map(|d| d.as_secs()),
            };
        }

        // Send whisper
        let result = self
            .whisper_manager
            .send_whisper(player, target, message, channel_bus);

        match result {
            WhisperResult::Success => ChatResult::WhisperSent {
                target: target.to_string(),
            },
            WhisperResult::PlayerNotFound => ChatResult::PlayerNotFound {
                name: target.to_string(),
            },
            WhisperResult::PlayerOffline { message_stored } => ChatResult::PlayerOffline {
                name: target.to_string(),
                message_stored,
            },
            WhisperResult::SelfWhisper => ChatResult::SelfWhisper,
            WhisperResult::Blocked => ChatResult::Blocked {
                target: target.to_string(),
            },
            WhisperResult::RateLimited => ChatResult::RateLimited {
                reset_in_secs: None,
            },
        }
    }

    /// Handle a reply message
    fn handle_reply(&self, player: &Player, message: &str, channel_bus: &ChannelBus) -> ChatResult {
        // Check rate limit
        if !self.rate_limiter.check(&player.id) {
            return ChatResult::RateLimited {
                reset_in_secs: self.rate_limiter.reset_in(&player.id).map(|d| d.as_secs()),
            };
        }

        let result = self
            .whisper_manager
            .send_reply(player, message, channel_bus);

        match result {
            WhisperResult::Success => ChatResult::WhisperSent {
                target: "last whisper target".to_string(), // We don't track this in result
            },
            WhisperResult::PlayerNotFound => ChatResult::NoReplyTarget,
            _ => ChatResult::Error("Failed to send reply".to_string()),
        }
    }

    /// Register a player as online (call on player login)
    pub fn player_online(&self, name: &str, player_id: Uuid) {
        self.whisper_manager.register_player(name, player_id);
    }

    /// Unregister a player (call on player logout)
    pub fn player_offline(&self, name: &str) {
        self.whisper_manager.unregister_player(name);
    }

    /// Get offline messages for a player (call on player login)
    pub fn get_offline_messages(&self, name: &str) -> Vec<OfflineMessage> {
        self.whisper_manager.get_offline_messages_by_name(name)
    }

    /// Check if a player is online
    pub fn is_player_online(&self, name: &str) -> bool {
        self.whisper_manager.is_player_online(name)
    }

    /// Broadcast a map message
    pub fn broadcast_map(&self, player: &Player, message: &str, _channel_bus: &ChannelBus) {
        let _event = GameEvent::PlayerChat {
            player_id: player.id,
            message: message.to_string(),
            chat_type: ChatType::Map,
        };

        let channel_name = &player.map_name;
        tracing::debug!(
            "Broadcasting map chat from {} on {}",
            player.name,
            channel_name
        );
    }
}

impl Default for ChatManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a chat operation
#[derive(Debug, Clone)]
pub enum ChatResult {
    /// Whisper message sent successfully
    WhisperSent { target: String },
    /// Regular chat message (to be processed by map/party system)
    RegularMessage(String),
    /// Player not found
    PlayerNotFound { name: String },
    /// Player is offline
    PlayerOffline { name: String, message_stored: bool },
    /// Rate limited
    RateLimited { reset_in_secs: Option<u64> },
    /// Cannot whisper to self
    SelfWhisper,
    /// Target player has blocked you
    Blocked { target: String },
    /// No previous whisper target for reply
    NoReplyTarget,
    /// Unknown command
    UnknownCommand,
    /// Generic error
    Error(String),
}

impl ChatResult {
    /// Get a user-friendly error message
    pub fn error_message(&self) -> Option<String> {
        match self {
            ChatResult::WhisperSent { .. } => None,
            ChatResult::RegularMessage(_) => None,
            ChatResult::PlayerNotFound { name } => Some(format!("Player '{}' not found", name)),
            ChatResult::PlayerOffline {
                name,
                message_stored,
            } => {
                if *message_stored {
                    Some(format!("Player '{}' is offline. Message stored.", name))
                } else {
                    Some(format!("Player '{}' is offline.", name))
                }
            }
            ChatResult::RateLimited { reset_in_secs } => {
                if let Some(secs) = reset_in_secs {
                    Some(format!("Rate limited. Try again in {} seconds.", secs))
                } else {
                    Some("Rate limited. Try again later.".to_string())
                }
            }
            ChatResult::SelfWhisper => Some("Cannot whisper to yourself.".to_string()),
            ChatResult::Blocked { target } => Some(format!("Player '{}' has blocked you.", target)),
            ChatResult::NoReplyTarget => {
                Some("No one to reply to. Use /w <player> <message>".to_string())
            }
            ChatResult::UnknownCommand => Some("Unknown chat command.".to_string()),
            ChatResult::Error(msg) => Some(msg.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Character;

    fn create_test_player(name: &str, char_id: u32) -> Player {
        let char = Character {
            char_id,
            char_num: 0,
            name: name.to_string(),
            class: 0,
            base_level: 1,
            job_level: 1,
            base_exp: 0,
            job_exp: 0,
            zeny: 0,
            str: 1,
            agi: 1,
            vit: 1,
            int: 1,
            dex: 1,
            luk: 1,
            hp: 100,
            max_hp: 100,
            sp: 50,
            max_sp: 50,
            hair: 1,
            hair_color: 0,
            clothes_color: 0,
            weapon: 0,
            shield: 0,
            head_top: 0,
            head_mid: 0,
            head_bottom: 0,
            last_map: "test".to_string(),
            last_x: 100,
            last_y: 100,
            save_map: "test".to_string(),
            save_x: 100,
            save_y: 100,
            delete_timer: 0,
            created_at: 0,
            updated_at: 0,
        };
        Player::from_character(char)
    }

    #[test]
    fn test_handle_whisper_to_online_player() {
        let manager = ChatManager::new();
        let channel_bus = ChannelBus::new();
        let map_state = Arc::new(MapState::new());

        let player1 = create_test_player("Player1", 1);
        let player2 = create_test_player("Player2", 2);

        // Register player2 as online
        manager.player_online("Player2", player2.id);

        // Player1 whispers to Player2
        let result =
            manager.handle_message(&player1, "/w Player2 Hello!", &map_state, &channel_bus);

        match result {
            ChatResult::WhisperSent { target } => {
                assert_eq!(target, "Player2");
            }
            _ => panic!("Expected WhisperSent result"),
        }
    }

    #[test]
    fn test_handle_whisper_to_offline_player() {
        let manager = ChatManager::new();
        let channel_bus = ChannelBus::new();
        let map_state = Arc::new(MapState::new());

        let player1 = create_test_player("Player1", 1);

        // Player1 whispers to offline Player2
        let result =
            manager.handle_message(&player1, "/w Player2 Hello!", &map_state, &channel_bus);

        match result {
            ChatResult::PlayerOffline {
                name,
                message_stored,
            } => {
                assert_eq!(name, "Player2");
                assert!(message_stored);
            }
            _ => panic!("Expected PlayerOffline result"),
        }
    }

    #[test]
    fn test_handle_regular_message() {
        let manager = ChatManager::new();
        let channel_bus = ChannelBus::new();
        let map_state = Arc::new(MapState::new());

        let player1 = create_test_player("Player1", 1);

        let result = manager.handle_message(&player1, "Hello everyone!", &map_state, &channel_bus);

        match result {
            ChatResult::RegularMessage(msg) => {
                assert_eq!(msg, "Hello everyone!");
            }
            _ => panic!("Expected RegularMessage result"),
        }
    }

    #[test]
    fn test_offline_messages_delivered_on_login() {
        let manager = ChatManager::new();
        let channel_bus = ChannelBus::new();
        let map_state = Arc::new(MapState::new());

        let player1 = create_test_player("Player1", 1);

        // Player1 whispers to offline Player2
        manager.handle_message(
            &player1,
            "/w Player2 Hello offline!",
            &map_state,
            &channel_bus,
        );

        // Player2 logs in
        let player2 = create_test_player("Player2", 2);
        manager.player_online("Player2", player2.id);

        // Player2 retrieves offline messages
        let messages = manager.get_offline_messages("Player2");

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].from_name, "Player1");
        assert_eq!(messages[0].message, "Hello offline!");
    }

    #[test]
    fn test_chat_result_error_message() {
        let result = ChatResult::PlayerNotFound {
            name: "TestPlayer".to_string(),
        };
        assert_eq!(
            result.error_message(),
            Some("Player 'TestPlayer' not found".to_string())
        );

        let result = ChatResult::RateLimited {
            reset_in_secs: Some(5),
        };
        assert_eq!(
            result.error_message(),
            Some("Rate limited. Try again in 5 seconds.".to_string())
        );
    }
}
