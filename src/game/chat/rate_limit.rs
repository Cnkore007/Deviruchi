//! Chat Rate Limiting
//!
//! Prevents spam by limiting whisper messages per player.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Rate limiter for whisper messages
pub struct WhisperRateLimiter {
    /// Player ID -> (message_count, window_start)
    limits: RwLock<HashMap<Uuid, (u32, Instant)>>,
    /// Maximum messages per window
    max_messages: u32,
    /// Time window in seconds
    window_secs: u64,
}

impl WhisperRateLimiter {
    /// Create a new rate limiter with the specified limits
    ///
    /// # Arguments
    /// * `max_messages` - Maximum messages allowed per window
    /// * `window_secs` - Time window in seconds
    pub fn new(max_messages: u32, window_secs: u64) -> Self {
        Self {
            limits: RwLock::new(HashMap::new()),
            max_messages,
            window_secs,
        }
    }

    /// Check if a message from the player is allowed
    ///
    /// Returns `true` if the message is allowed, `false` if rate limited.
    pub fn check(&self, player_id: &Uuid) -> bool {
        let mut limits = self.limits.write();

        let now = Instant::now();
        let window_duration = Duration::from_secs(self.window_secs);

        // Check if player exists and get values
        if let Some((count, start)) = limits.get(player_id).copied() {
            if now.duration_since(start) < window_duration {
                // Within the current window
                if count >= self.max_messages {
                    return false; // Rate limited
                }
                // Increment count - remove and re-insert to update
                limits.remove(player_id);
                limits.insert(*player_id, (count + 1, start));
            } else {
                // Window has expired, reset
                limits.insert(*player_id, (1, now));
            }
        } else {
            // First message from this player
            limits.insert(*player_id, (1, now));
        }

        true
    }

    /// Record a message (same as check but doesn't return status)
    pub fn record(&self, player_id: &Uuid) {
        self.check(player_id);
    }

    /// Reset rate limit for a player (e.g., after they apologize)
    pub fn reset(&self, player_id: &Uuid) {
        let mut limits = self.limits.write();
        limits.remove(player_id);
    }

    /// Get remaining messages for a player in the current window
    pub fn remaining(&self, player_id: &Uuid) -> u32 {
        let limits = self.limits.read();
        let now = Instant::now();
        let window_duration = Duration::from_secs(self.window_secs);

        if let Some((count, start)) = limits.get(player_id)
            && now.duration_since(*start) < window_duration
        {
            return self.max_messages.saturating_sub(*count);
        }

        self.max_messages
    }

    /// Get time until the rate limit resets for a player
    pub fn reset_in(&self, player_id: &Uuid) -> Option<Duration> {
        let limits = self.limits.read();
        let now = Instant::now();
        let window_duration = Duration::from_secs(self.window_secs);

        if let Some((count, start)) = limits.get(player_id)
            && *count >= self.max_messages
        {
            let elapsed = now.duration_since(*start);
            if elapsed < window_duration {
                return Some(window_duration - elapsed);
            }
        }

        None
    }

    /// Cleanup expired entries
    pub fn cleanup(&self) {
        let mut limits = self.limits.write();
        let now = Instant::now();
        let window_duration = Duration::from_secs(self.window_secs);

        limits.retain(|_, (_, start)| now.duration_since(*start) < window_duration);
    }

    /// Get current limit configuration
    pub fn config(&self) -> (u32, u64) {
        (self.max_messages, self.window_secs)
    }
}

impl Default for WhisperRateLimiter {
    fn default() -> Self {
        // Default: 10 messages per 10 seconds
        Self::new(10, 10)
    }
}

/// Global default rate limiter configuration
impl WhisperRateLimiter {
    /// Create a rate limiter with default MMORPG settings
    /// Typically: 10 whispers per 10 seconds
    pub fn default_mmorpg() -> Self {
        Self::new(10, 10)
    }

    /// Create a rate limiter with strict settings
    /// Typically: 5 whispers per 10 seconds
    pub fn strict() -> Self {
        Self::new(5, 10)
    }

    /// Create a rate limiter with lenient settings
    /// Typically: 20 whispers per 10 seconds
    pub fn lenient() -> Self {
        Self::new(20, 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_message_allowed() {
        let limiter = WhisperRateLimiter::new(10, 10);
        let player_id = Uuid::new_v4();

        assert!(limiter.check(&player_id));
        assert_eq!(limiter.remaining(&player_id), 9);
    }

    #[test]
    fn test_rate_limit_reached() {
        let limiter = WhisperRateLimiter::new(3, 10);
        let player_id = Uuid::new_v4();

        // First 3 messages allowed
        assert!(limiter.check(&player_id));
        assert!(limiter.check(&player_id));
        assert!(limiter.check(&player_id));

        // 4th message blocked
        assert!(!limiter.check(&player_id));
        assert_eq!(limiter.remaining(&player_id), 0);
    }

    #[test]
    fn test_reset_clears_limit() {
        let limiter = WhisperRateLimiter::new(10, 10);
        let player_id = Uuid::new_v4();

        // Exhaust limit
        for _ in 0..10 {
            assert!(limiter.check(&player_id));
        }
        assert!(!limiter.check(&player_id));

        // Reset
        limiter.reset(&player_id);

        // Should be allowed again
        assert!(limiter.check(&player_id));
        assert_eq!(limiter.remaining(&player_id), 9);
    }

    #[test]
    fn test_different_players_independent() {
        let limiter = WhisperRateLimiter::new(10, 10);
        let player1 = Uuid::new_v4();
        let player2 = Uuid::new_v4();

        // Exhaust player1's limit
        for _ in 0..10 {
            assert!(limiter.check(&player1));
        }
        assert!(!limiter.check(&player1));

        // Player2 should still be allowed
        assert!(limiter.check(&player2));
        assert_eq!(limiter.remaining(&player2), 9);
    }

    #[test]
    fn test_reset_in_returns_none_when_not_limited() {
        let limiter = WhisperRateLimiter::new(2, 10);
        let player_id = Uuid::new_v4();

        assert!(limiter.reset_in(&player_id).is_none());

        // Use some messages
        limiter.check(&player_id);
        assert!(limiter.reset_in(&player_id).is_none());
    }
}
