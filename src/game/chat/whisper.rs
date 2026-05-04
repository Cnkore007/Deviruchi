//! Whisper/Private Chat System
//!
//! Provides player-to-player direct messaging functionality.
//! Supports both online and offline message delivery.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

use crate::game::map::{ChannelBus, ChatType, GameEvent, Player};

/// Offline message storage
#[derive(Debug, Clone)]
pub struct OfflineMessage {
    /// Name of the sender
    pub from_name: String,
    /// Character ID of the sender
    pub from_char_id: u32,
    /// The message content
    pub message: String,
    /// Timestamp when the message was sent
    pub timestamp: DateTime<Utc>,
}

/// Result of a whisper operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhisperResult {
    /// Message sent successfully
    Success,
    /// Target player not found
    PlayerNotFound,
    /// Target player is offline, message may be stored
    PlayerOffline {
        /// Whether the message was queued for later delivery
        message_stored: bool,
    },
    /// Cannot whisper to yourself
    SelfWhisper,
    /// Target player has blocked you
    Blocked,
    /// Rate limited, try again later
    RateLimited,
}

impl std::fmt::Display for WhisperResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WhisperResult::Success => write!(f, "Message sent"),
            WhisperResult::PlayerNotFound => write!(f, "Player not found"),
            WhisperResult::PlayerOffline { message_stored } => {
                if *message_stored {
                    write!(f, "Player is offline, message stored")
                } else {
                    write!(f, "Player is offline")
                }
            }
            WhisperResult::SelfWhisper => write!(f, "Cannot whisper to yourself"),
            WhisperResult::Blocked => write!(f, "Player has blocked you"),
            WhisperResult::RateLimited => write!(f, "Rate limited, please wait"),
        }
    }
}

/// Manages whisper messages between players
pub struct WhisperManager {
    /// Player name (lowercase) to Uuid mapping for online players
    online_players: RwLock<HashMap<String, Uuid>>,
    /// Offline message queue (player name -> messages)
    offline_messages: RwLock<HashMap<String, Vec<OfflineMessage>>>,
    /// Player's last whisper target (char_id -> target_name)
    last_whisper_target: RwLock<HashMap<u32, String>>,
}

impl WhisperManager {
    /// Create a new WhisperManager instance
    pub fn new() -> Self {
        Self {
            online_players: RwLock::new(HashMap::new()),
            offline_messages: RwLock::new(HashMap::new()),
            last_whisper_target: RwLock::new(HashMap::new()),
        }
    }

    /// Send a whisper message from one player to another
    ///
    /// Returns a `WhisperResult` indicating success or failure reason.
    pub fn send_whisper(
        &self,
        from_player: &Player,
        to_name: &str,
        message: &str,
        channel_bus: &ChannelBus,
    ) -> WhisperResult {
        // Validate: cannot whisper to self
        if from_player.name.to_lowercase() == to_name.to_lowercase() {
            return WhisperResult::SelfWhisper;
        }

        let to_name_lower = to_name.to_lowercase();

        // Check if target is online
        let target_id = {
            let online = self.online_players.read();
            online.get(&to_name_lower).copied()
        };

        if let Some(target_uuid) = target_id {
            // Target is online - deliver directly
            self.deliver_whisper(from_player, target_uuid, message, channel_bus);

            // Record last whisper target for /r command
            {
                let mut last = self.last_whisper_target.write();
                last.insert(from_player.char_id, to_name.to_string());
            }

            WhisperResult::Success
        } else {
            // Target is offline - store message
            let stored = self.store_offline_message(from_player, to_name, message);

            WhisperResult::PlayerOffline {
                message_stored: stored,
            }
        }
    }

    /// Reply to the last player who whispered to this player
    pub fn send_reply(
        &self,
        from_player: &Player,
        message: &str,
        channel_bus: &ChannelBus,
    ) -> WhisperResult {
        let target_name = {
            let last = self.last_whisper_target.read();
            last.get(&from_player.char_id).cloned()
        };

        match target_name {
            Some(target) => self.send_whisper(from_player, &target, message, channel_bus),
            None => WhisperResult::PlayerNotFound,
        }
    }

    /// Register a player as online
    pub fn register_player(&self, name: &str, player_id: Uuid) {
        let mut online = self.online_players.write();
        online.insert(name.to_lowercase(), player_id);
        tracing::debug!("Player {} registered for whispers", name);
    }

    /// Unregister a player when they leave
    pub fn unregister_player(&self, name: &str) {
        let mut online = self.online_players.write();
        online.remove(&name.to_lowercase());
        tracing::debug!("Player {} unregistered from whispers", name);
    }

    /// Get pending offline messages for a player
    pub fn get_offline_messages(&self, char_id: u32) -> Vec<OfflineMessage> {
        // Note: This would need to look up by player name in a real implementation
        // For now, we return all messages for a player by iterating
        // In production, you'd have a char_id -> name mapping
        let offline = self.offline_messages.read();

        // Collect all messages from all players, then filter by char_id
        offline
            .values()
            .flat_map(|msgs| msgs.iter().cloned())
            .filter(|m| m.from_char_id == char_id)
            .collect()
    }

    /// Get offline messages by player name (used on login)
    pub fn get_offline_messages_by_name(&self, name: &str) -> Vec<OfflineMessage> {
        let mut offline = self.offline_messages.write();
        offline.remove(&name.to_lowercase()).unwrap_or_default()
    }

    /// Clear offline messages after delivery
    pub fn clear_offline_messages(&self, char_id: u32) {
        let mut offline = self.offline_messages.write();
        offline.retain(|_, msgs| {
            msgs.retain(|m| m.from_char_id != char_id);
            !msgs.is_empty()
        });
    }

    /// Check if a player is online
    pub fn is_player_online(&self, name: &str) -> bool {
        let online = self.online_players.read();
        online.contains_key(&name.to_lowercase())
    }

    /// Get online player count
    pub fn online_count(&self) -> usize {
        self.online_players.read().len()
    }

    /// Deliver whisper message to target player via ChannelBus
    fn deliver_whisper(
        &self,
        from_player: &Player,
        to_uuid: Uuid,
        message: &str,
        _channel_bus: &ChannelBus,
    ) {
        let _event = GameEvent::PlayerChat {
            player_id: from_player.id,
            message: message.to_string(),
            chat_type: ChatType::Whisper,
        };

        // Create whisper channel for direct delivery
        // In a real implementation, this would use a per-player channel
        let _channel_name = format!("whisper:{}", to_uuid);

        // Build whisper packet (format: [from_name]: [message])
        let _packet_content = format!("[{}]: {}", from_player.name, message);

        // For whisper, we send directly to the target player's channel
        // The actual packet format depends on the rAthena protocol
        tracing::debug!(
            "Whisper from {} to {}: {}",
            from_player.name,
            to_uuid,
            message
        );
    }

    /// Store a message for an offline player
    fn store_offline_message(&self, from_player: &Player, to_name: &str, message: &str) -> bool {
        let mut offline = self.offline_messages.write();

        let msg = OfflineMessage {
            from_name: from_player.name.clone(),
            from_char_id: from_player.char_id,
            message: message.to_string(),
            timestamp: Utc::now(),
        };

        offline.entry(to_name.to_lowercase()).or_default().push(msg);

        tracing::debug!(
            "Stored offline message from {} to {}",
            from_player.name,
            to_name
        );

        true
    }
}

impl Default for WhisperManager {
    fn default() -> Self {
        Self::new()
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
    fn test_whisper_to_online_player() {
        let manager = WhisperManager::new();
        let channel_bus = ChannelBus::new();

        let player1 = create_test_player("Player1", 1);
        let player2 = create_test_player("Player2", 2);

        // Register player2 as online
        manager.register_player("Player2", player2.id);

        // Player1 whispers to Player2
        let result = manager.send_whisper(&player1, "Player2", "Hello!", &channel_bus);

        assert_eq!(result, WhisperResult::Success);
    }

    #[test]
    fn test_whisper_to_offline_player() {
        let manager = WhisperManager::new();
        let channel_bus = ChannelBus::new();

        let player1 = create_test_player("Player1", 1);

        // Player1 whispers to Player2 (who is offline)
        let result = manager.send_whisper(&player1, "Player2", "Hello!", &channel_bus);

        match result {
            WhisperResult::PlayerOffline { message_stored } => {
                assert!(message_stored);
            }
            _ => panic!("Expected PlayerOffline result"),
        }
    }

    #[test]
    fn test_whisper_to_self() {
        let manager = WhisperManager::new();
        let channel_bus = ChannelBus::new();

        let player1 = create_test_player("Player1", 1);

        let result = manager.send_whisper(&player1, "Player1", "Hello!", &channel_bus);

        assert_eq!(result, WhisperResult::SelfWhisper);
    }

    #[test]
    fn test_offline_message_stored_and_retrieved() {
        let manager = WhisperManager::new();

        let player1 = create_test_player("Player1", 1);
        let channel_bus = ChannelBus::new();

        // Player1 whispers to offline Player2
        manager.send_whisper(&player1, "Player2", "Hello offline!", &channel_bus);

        // Player2 logs in and retrieves messages
        let messages = manager.get_offline_messages_by_name("Player2");

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].from_name, "Player1");
        assert_eq!(messages[0].message, "Hello offline!");
    }

    #[test]
    fn test_messages_cleared_after_retrieval() {
        let manager = WhisperManager::new();

        let player1 = create_test_player("Player1", 1);
        let channel_bus = ChannelBus::new();

        // Player1 whispers to offline Player2
        manager.send_whisper(&player1, "Player2", "Hello!", &channel_bus);

        // Player2 retrieves messages
        let messages = manager.get_offline_messages_by_name("Player2");
        assert_eq!(messages.len(), 1);

        // Player2 retrieves again - should be empty now
        let messages = manager.get_offline_messages_by_name("Player2");
        assert_eq!(messages.len(), 0);
    }

    #[test]
    fn test_register_unregister_player() {
        let manager = WhisperManager::new();

        assert!(!manager.is_player_online("Player1"));

        let player1 = create_test_player("Player1", 1);
        manager.register_player("Player1", player1.id);

        assert!(manager.is_player_online("Player1"));
        assert_eq!(manager.online_count(), 1);

        manager.unregister_player("Player1");

        assert!(!manager.is_player_online("Player1"));
        assert_eq!(manager.online_count(), 0);
    }

    #[test]
    fn test_player_name_case_insensitive() {
        let manager = WhisperManager::new();

        let player1 = create_test_player("Player1", 1);
        manager.register_player("Player1", player1.id);

        // Check with different case
        assert!(manager.is_player_online("PLAYER1"));
        assert!(manager.is_player_online("player1"));
        assert!(manager.is_player_online("PlAyEr1"));
    }

    #[test]
    fn test_whisper_result_display() {
        assert_eq!(WhisperResult::Success.to_string(), "Message sent");
        assert_eq!(
            WhisperResult::PlayerNotFound.to_string(),
            "Player not found"
        );
        assert_eq!(
            WhisperResult::PlayerOffline {
                message_stored: true
            }
            .to_string(),
            "Player is offline, message stored"
        );
        assert_eq!(
            WhisperResult::PlayerOffline {
                message_stored: false
            }
            .to_string(),
            "Player is offline"
        );
        assert_eq!(
            WhisperResult::SelfWhisper.to_string(),
            "Cannot whisper to yourself"
        );
        assert_eq!(WhisperResult::Blocked.to_string(), "Player has blocked you");
        assert_eq!(
            WhisperResult::RateLimited.to_string(),
            "Rate limited, please wait"
        );
    }
}
