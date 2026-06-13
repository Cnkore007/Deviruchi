use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::Instant;

/// Token 有效期（秒）
pub const TOKEN_EXPIRY_SECS: u64 = 30;

/// Token 数据结构
#[derive(Debug, Clone)]
pub struct TokenData {
    /// 账户ID
    pub account_id: u32,
    /// 角色ID
    pub char_id: u32,
    /// 目标 MapServer ID
    pub map_server_id: u32,
    /// 创建时间
    pub created_at: Instant,
    /// 过期时间（计算得出）
    pub expires_at: u64,
}

impl TokenData {
    /// 检查 token 是否过期
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() > TOKEN_EXPIRY_SECS
    }

    /// 获取剩余有效时间（秒）
    pub fn remaining_secs(&self) -> u64 {
        let elapsed = self.created_at.elapsed().as_secs();
        if elapsed >= TOKEN_EXPIRY_SECS {
            return 0;
        }
        TOKEN_EXPIRY_SECS - elapsed
    }
}

/// Token 条目（内部存储用）
struct TokenEntry {
    account_id: u32,
    char_id: u32,
    map_server_id: u32,
    created_at: Instant,
}

impl From<TokenData> for TokenEntry {
    fn from(data: TokenData) -> Self {
        Self {
            account_id: data.account_id,
            char_id: data.char_id,
            map_server_id: data.map_server_id,
            created_at: data.created_at,
        }
    }
}

impl From<&TokenEntry> for TokenData {
    fn from(entry: &TokenEntry) -> Self {
        Self {
            account_id: entry.account_id,
            char_id: entry.char_id,
            map_server_id: entry.map_server_id,
            created_at: entry.created_at,
            expires_at: TOKEN_EXPIRY_SECS,
        }
    }
}

/// Token 存储管理器
pub struct TokenStore {
    tokens: RwLock<HashMap<String, TokenEntry>>,
}

impl Default for TokenStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenStore {
    pub fn new() -> Self {
        Self {
            tokens: RwLock::new(HashMap::new()),
        }
    }

    /// Generate a one-time token for Char→Map transition
    /// Returns the generated token string
    ///
    /// # Arguments
    /// * `account_id` - 账户ID
    /// * `char_id` - 角色ID
    /// * `map_server_id` - 目标MapServer ID
    pub fn create(&self, account_id: u32, char_id: u32, map_server_id: u32) -> String {
        let token = Self::generate_token();
        let entry = TokenEntry {
            account_id,
            char_id,
            map_server_id,
            created_at: Instant::now(),
        };
        self.tokens.write().insert(token.clone(), entry);
        token
    }

    /// Verify and consume a one-time token
    /// Returns true if valid, false otherwise
    /// Token is removed after verification (one-time use)
    ///
    /// # Arguments
    /// * `token` - 待验证的token
    /// * `account_id` - 账户ID
    /// * `char_id` - 角色ID
    /// * `map_server_id` - 目标MapServer ID（可选，传入0表示不验证）
    pub fn verify(&self, token: &str, account_id: u32, char_id: u32, map_server_id: u32) -> bool {
        let mut tokens = self.tokens.write();
        if let Some(entry) = tokens.remove(token) {
            // Check if token is expired
            if entry.created_at.elapsed().as_secs() > TOKEN_EXPIRY_SECS {
                return false;
            }
            // Verify account and character
            if entry.account_id != account_id || entry.char_id != char_id {
                return false;
            }
            // Verify map server (if specified)
            if map_server_id != 0 && entry.map_server_id != map_server_id {
                return false;
            }
            true
        } else {
            false
        }
    }

    /// 验证token并返回关联的MapServer ID
    /// Returns Some(map_server_id) if valid, None otherwise
    pub fn verify_and_get_server(&self, token: &str, account_id: u32, char_id: u32) -> Option<u32> {
        let mut tokens = self.tokens.write();
        if let Some(entry) = tokens.remove(token) {
            if entry.created_at.elapsed().as_secs() > TOKEN_EXPIRY_SECS {
                return None;
            }
            if entry.account_id == account_id && entry.char_id == char_id {
                return Some(entry.map_server_id);
            }
        }
        None
    }

    /// 获取token信息（不消费）
    pub fn get_token_data(&self, token: &str) -> Option<TokenData> {
        let tokens = self.tokens.read();
        tokens.get(token).map(|entry| entry.into())
    }

    /// Clean up expired tokens (called by GameLoop tick)
    pub fn cleanup_expired(&self) {
        let mut tokens = self.tokens.write();
        tokens.retain(|_, entry| entry.created_at.elapsed().as_secs() <= TOKEN_EXPIRY_SECS);
    }

    /// 获取当前活跃token数量
    pub fn active_token_count(&self) -> usize {
        let tokens = self.tokens.read();
        tokens.len()
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
        let token = store.create(1, 100, 10);
        assert_eq!(token.len(), 32);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn verify_succeeds_with_correct_ids() {
        let store = TokenStore::new();
        let token = store.create(1, 100, 10);
        assert!(store.verify(&token, 1, 100, 10));
    }

    #[test]
    fn verify_fails_with_wrong_account_id() {
        let store = TokenStore::new();
        let token = store.create(1, 100, 10);
        assert!(!store.verify(&token, 2, 100, 10));
    }

    #[test]
    fn verify_fails_with_wrong_char_id() {
        let store = TokenStore::new();
        let token = store.create(1, 100, 10);
        assert!(!store.verify(&token, 1, 200, 10));
    }

    #[test]
    fn verify_fails_with_wrong_map_server_id() {
        let store = TokenStore::new();
        let token = store.create(1, 100, 10);
        // 验证时传入不同的 map_server_id
        assert!(!store.verify(&token, 1, 100, 20));
    }

    #[test]
    fn token_is_one_time_use() {
        let store = TokenStore::new();
        let token = store.create(1, 100, 10);
        assert!(store.verify(&token, 1, 100, 10));
        // Second verify should fail — token already consumed
        assert!(!store.verify(&token, 1, 100, 10));
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
                    map_server_id: 10,
                    created_at: Instant::now() - std::time::Duration::from_secs(31),
                },
            );
            tokens.insert(
                "valid_token".to_string(),
                TokenEntry {
                    account_id: 2,
                    char_id: 200,
                    map_server_id: 10,
                    created_at: Instant::now(),
                },
            );
        }
        store.cleanup_expired();
        let tokens = store.tokens.read();
        assert!(!tokens.contains_key("expired_token"));
        assert!(tokens.contains_key("valid_token"));
    }

    #[test]
    fn verify_and_get_server_returns_map_server_id() {
        let store = TokenStore::new();
        let token = store.create(1, 100, 42);
        assert_eq!(store.verify_and_get_server(&token, 1, 100), Some(42));
        // Token consumed, should return None
        assert_eq!(store.verify_and_get_server(&token, 1, 100), None);
    }

    #[test]
    fn token_data_is_expired() {
        let mut data = TokenData {
            account_id: 1,
            char_id: 100,
            map_server_id: 10,
            created_at: Instant::now(),
            expires_at: TOKEN_EXPIRY_SECS,
        };
        assert!(!data.is_expired());

        // 模拟过期
        data.created_at = Instant::now() - std::time::Duration::from_secs(TOKEN_EXPIRY_SECS + 1);
        assert!(data.is_expired());
    }

    #[test]
    fn token_data_remaining_secs() {
        let data = TokenData {
            account_id: 1,
            char_id: 100,
            map_server_id: 10,
            created_at: Instant::now(),
            expires_at: TOKEN_EXPIRY_SECS,
        };
        assert!(data.remaining_secs() > 0);
        assert!(data.remaining_secs() <= TOKEN_EXPIRY_SECS);
    }

    #[test]
    fn get_token_data() {
        let store = TokenStore::new();
        let token = store.create(1, 100, 10);
        let data = store.get_token_data(&token);
        assert!(data.is_some());
        let data = data.unwrap();
        assert_eq!(data.account_id, 1);
        assert_eq!(data.char_id, 100);
        assert_eq!(data.map_server_id, 10);
    }

    #[test]
    fn active_token_count() {
        let store = TokenStore::new();
        assert_eq!(store.active_token_count(), 0);
        store.create(1, 100, 10);
        store.create(2, 200, 10);
        assert_eq!(store.active_token_count(), 2);
    }
}
