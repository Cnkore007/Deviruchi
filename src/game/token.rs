use std::collections::HashMap;
use std::time::Instant;
use parking_lot::RwLock;

pub struct TokenEntry {
    pub account_id: u32,
    pub char_id: u32,
    pub created_at: Instant,
}

pub struct TokenStore {
    tokens: RwLock<HashMap<String, TokenEntry>>,
}

impl TokenStore {
    pub fn new() -> Self {
        Self {
            tokens: RwLock::new(HashMap::new()),
        }
    }

    /// Generate a one-time token for Char→Map transition
    /// Returns the generated token string
    pub fn create(&self, account_id: u32, char_id: u32) -> String {
        let token = Self::generate_token();
        let entry = TokenEntry {
            account_id,
            char_id,
            created_at: Instant::now(),
        };
        self.tokens.write().insert(token.clone(), entry);
        token
    }

    /// Verify and consume a one-time token
    /// Returns true if valid, false otherwise
    /// Token is removed after verification (one-time use)
    pub fn verify(&self, token: &str, account_id: u32, char_id: u32) -> bool {
        let mut tokens = self.tokens.write();
        if let Some(entry) = tokens.remove(token) {
            // Check if token is expired (30 seconds)
            if entry.created_at.elapsed().as_secs() > 30 {
                return false;
            }
            entry.account_id == account_id && entry.char_id == char_id
        } else {
            false
        }
    }

    /// Clean up expired tokens (called by GameLoop tick)
    pub fn cleanup_expired(&self) {
        let mut tokens = self.tokens.write();
        tokens.retain(|_, entry| entry.created_at.elapsed().as_secs() <= 30);
    }

    fn generate_token() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: [u8; 16] = rng.r#gen();
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_32_char_hex_string() {
        let store = TokenStore::new();
        let token = store.create(1, 100);
        assert_eq!(token.len(), 32);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn verify_succeeds_with_correct_ids() {
        let store = TokenStore::new();
        let token = store.create(1, 100);
        assert!(store.verify(&token, 1, 100));
    }

    #[test]
    fn verify_fails_with_wrong_account_id() {
        let store = TokenStore::new();
        let token = store.create(1, 100);
        assert!(!store.verify(&token, 2, 100));
    }

    #[test]
    fn verify_fails_with_wrong_char_id() {
        let store = TokenStore::new();
        let token = store.create(1, 100);
        assert!(!store.verify(&token, 1, 200));
    }

    #[test]
    fn token_is_one_time_use() {
        let store = TokenStore::new();
        let token = store.create(1, 100);
        assert!(store.verify(&token, 1, 100));
        // Second verify should fail — token already consumed
        assert!(!store.verify(&token, 1, 100));
    }

    #[test]
    fn expired_tokens_are_cleaned_up() {
        let store = TokenStore::new();
        // Insert a token with an already-expired Instant
        {
            let mut tokens = store.tokens.write();
            tokens.insert(
                "expired_token".to_string(),
                TokenEntry {
                    account_id: 1,
                    char_id: 100,
                    created_at: Instant::now() - std::time::Duration::from_secs(31),
                },
            );
            tokens.insert(
                "valid_token".to_string(),
                TokenEntry {
                    account_id: 2,
                    char_id: 200,
                    created_at: Instant::now(),
                },
            );
        }
        store.cleanup_expired();
        let tokens = store.tokens.read();
        assert!(!tokens.contains_key("expired_token"));
        assert!(tokens.contains_key("valid_token"));
    }
}
