//! IP 封禁系统
//! 
//! 处理 IP 地址的封禁和解封。
//! 对应 rAthena 的 ipban.cpp。

use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// 封禁原因
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BanReason {
    /// 暴力破解
    BruteForce,
    /// 恶意行为
    MaliciousBehavior,
    /// 违规操作
    Violation,
    /// 管理员封禁
    AdminBan,
    /// 其他
    Other(String),
}

/// IP 封禁记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpBan {
    /// IP 地址
    pub ip: String,
    /// 封禁原因
    pub reason: BanReason,
    /// 封禁时间
    pub banned_at: u64,
    /// 解封时间（None 表示永久封禁）
    pub expires_at: Option<u64>,
    /// 封禁者
    pub banned_by: String,
    /// 是否活跃
    pub is_active: bool,
}

/// IP 封禁管理器
pub struct IpBanManager {
    /// 封禁列表
    bans: RwLock<HashMap<String, IpBan>>,
    /// 尝试次数（用于暴力破解检测）
    attempts: RwLock<HashMap<String, u32>>,
    /// 最大尝试次数
    max_attempts: u32,
    /// 尝试重置时间（秒）
    _attempt_reset_time: u64,
}

impl IpBanManager {
    /// 创建新的 IP 封禁管理器
    pub fn new(max_attempts: u32, attempt_reset_time: u64) -> Self {
        Self {
            bans: RwLock::new(HashMap::new()),
            attempts: RwLock::new(HashMap::new()),
            max_attempts,
            _attempt_reset_time: attempt_reset_time,
        }
    }

    /// 检查 IP 是否被封禁
    pub fn is_banned(&self, ip: &str) -> bool {
        let bans = self.bans.read();
        if let Some(ban) = bans.get(ip) {
            if !ban.is_active {
                return false;
            }
            if let Some(expires_at) = ban.expires_at {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                return now < expires_at;
            }
            true
        } else {
            false
        }
    }

    /// 封禁 IP
    pub fn ban_ip(
        &self,
        ip: String,
        reason: BanReason,
        duration: Option<u64>,
        banned_by: String,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let ban = IpBan {
            ip: ip.clone(),
            reason,
            banned_at: now,
            expires_at: duration.map(|d| now + d),
            banned_by,
            is_active: true,
        };

        self.bans.write().insert(ip, ban);
    }

    /// 解封 IP
    pub fn unban_ip(&self, ip: &str) -> bool {
        let mut bans = self.bans.write();
        if let Some(ban) = bans.get_mut(ip) {
            ban.is_active = false;
            true
        } else {
            false
        }
    }

    /// 记录登录尝试
    pub fn record_attempt(&self, ip: &str) -> bool {
        let mut attempts = self.attempts.write();
        let count = attempts.entry(ip.to_string()).or_insert(0);
        *count += 1;
        
        if *count >= self.max_attempts {
            self.ban_ip(
                ip.to_string(),
                BanReason::BruteForce,
                Some(3600), // 封禁 1 小时
                "系统".to_string(),
            );
            true
        } else {
            false
        }
    }

    /// 重置尝试次数
    pub fn reset_attempts(&self, ip: &str) {
        self.attempts.write().remove(ip);
    }

    /// 获取封禁信息
    pub fn get_ban_info(&self, ip: &str) -> Option<IpBan> {
        self.bans.read().get(ip).cloned()
    }

    /// 获取所有封禁
    pub fn get_all_bans(&self) -> Vec<IpBan> {
        self.bans.read().values().cloned().collect()
    }

    /// 清理过期封禁
    pub fn cleanup_expired(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut bans = self.bans.write();
        bans.retain(|_, ban| {
            if !ban.is_active {
                return false;
            }
            if let Some(expires_at) = ban.expires_at {
                now < expires_at
            } else {
                true
            }
        });
    }
}

impl Default for IpBanManager {
    fn default() -> Self {
        Self::new(5, 300)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ban_unban() {
        let manager = IpBanManager::new(5, 300);
        
        manager.ban_ip("192.168.1.1".to_string(), BanReason::BruteForce, None, "admin".to_string());
        assert!(manager.is_banned("192.168.1.1"));
        
        manager.unban_ip("192.168.1.1");
        assert!(!manager.is_banned("192.168.1.1"));
    }

    #[test]
    fn test_brute_force_detection() {
        let manager = IpBanManager::new(3, 300);
        
        for _ in 0..2 {
            manager.record_attempt("192.168.1.1");
        }
        assert!(!manager.is_banned("192.168.1.1"));
        
        manager.record_attempt("192.168.1.1");
        assert!(manager.is_banned("192.168.1.1"));
    }

    #[test]
    fn test_reset_attempts() {
        let manager = IpBanManager::new(3, 300);
        
        manager.record_attempt("192.168.1.1");
        manager.record_attempt("192.168.1.1");
        manager.reset_attempts("192.168.1.1");
        
        for _ in 0..2 {
            manager.record_attempt("192.168.1.1");
        }
        assert!(!manager.is_banned("192.168.1.1"));
    }
}
