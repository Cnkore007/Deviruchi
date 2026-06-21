//! 登录日志系统
//!
//! 记录登录相关的日志信息。
//! 对应 rAthena 的 loginlog.cpp。

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// 登录事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoginEvent {
    /// 登录成功
    LoginSuccess,
    /// 登录失败
    LoginFailed,
    /// 登出
    Logout,
    /// 封禁
    Banned,
    /// 解封
    Unbanned,
    /// 密码错误
    WrongPassword,
    /// 账号不存在
    AccountNotFound,
    /// 账号已登录
    AlreadyLoggedIn,
    /// 服务器满员
    ServerFull,
}

/// 登录日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginLogEntry {
    /// 日志 ID
    pub id: u64,
    /// 账号 ID
    pub account_id: u32,
    /// 用户名
    pub username: String,
    /// IP 地址
    pub ip: String,
    /// 事件类型
    pub event: LoginEvent,
    /// 时间戳
    pub timestamp: u64,
    /// 附加信息
    pub message: String,
}

/// 登录日志管理器
pub struct LoginLogManager {
    /// 日志队列
    logs: RwLock<VecDeque<LoginLogEntry>>,
    /// 最大日志数量
    max_logs: usize,
    /// 下一个日志 ID
    next_id: RwLock<u64>,
}

impl LoginLogManager {
    /// 创建新的登录日志管理器
    pub fn new(max_logs: usize) -> Self {
        Self {
            logs: RwLock::new(VecDeque::new()),
            max_logs,
            next_id: RwLock::new(1),
        }
    }

    /// 记录登录事件
    pub fn log_event(
        &self,
        account_id: u32,
        username: String,
        ip: String,
        event: LoginEvent,
        message: String,
    ) {
        let mut next_id = self.next_id.write();
        let id = *next_id;
        *next_id += 1;

        let entry = LoginLogEntry {
            id,
            account_id,
            username,
            ip,
            event,
            timestamp: crate::util::unix_timestamp_secs(),
            message,
        };

        let mut logs = self.logs.write();
        logs.push_back(entry);

        // 限制日志数量
        while logs.len() > self.max_logs {
            logs.pop_front();
        }
    }

    /// 获取最近的日志
    pub fn get_recent_logs(&self, count: usize) -> Vec<LoginLogEntry> {
        let logs = self.logs.read();
        let start = if logs.len() > count {
            logs.len() - count
        } else {
            0
        };
        logs.range(start..).cloned().collect()
    }

    /// 获取账号的日志
    pub fn get_account_logs(&self, account_id: u32) -> Vec<LoginLogEntry> {
        self.logs
            .read()
            .iter()
            .filter(|entry| entry.account_id == account_id)
            .cloned()
            .collect()
    }

    /// 获取 IP 的日志
    pub fn get_ip_logs(&self, ip: &str) -> Vec<LoginLogEntry> {
        self.logs
            .read()
            .iter()
            .filter(|entry| entry.ip == ip)
            .cloned()
            .collect()
    }

    /// 清空日志
    pub fn clear(&self) {
        self.logs.write().clear();
    }

    /// 获取日志数量
    pub fn count(&self) -> usize {
        self.logs.read().len()
    }
}

impl Default for LoginLogManager {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_event() {
        let manager = LoginLogManager::new(100);

        manager.log_event(
            1,
            "test_user".to_string(),
            "192.168.1.1".to_string(),
            LoginEvent::LoginSuccess,
            "登录成功".to_string(),
        );

        assert_eq!(manager.count(), 1);

        let logs = manager.get_recent_logs(10);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].event, LoginEvent::LoginSuccess);
    }

    #[test]
    fn test_max_logs() {
        let manager = LoginLogManager::new(2);

        for i in 0..3 {
            manager.log_event(
                i,
                format!("user_{}", i),
                "192.168.1.1".to_string(),
                LoginEvent::LoginSuccess,
                String::new(),
            );
        }

        assert_eq!(manager.count(), 2);
    }

    #[test]
    fn test_filter_by_account() {
        let manager = LoginLogManager::new(100);

        manager.log_event(
            1,
            "user1".to_string(),
            "192.168.1.1".to_string(),
            LoginEvent::LoginSuccess,
            String::new(),
        );
        manager.log_event(
            2,
            "user2".to_string(),
            "192.168.1.2".to_string(),
            LoginEvent::LoginSuccess,
            String::new(),
        );
        manager.log_event(
            1,
            "user1".to_string(),
            "192.168.1.1".to_string(),
            LoginEvent::Logout,
            String::new(),
        );

        let logs = manager.get_account_logs(1);
        assert_eq!(logs.len(), 2);
    }

    #[test]
    fn test_filter_by_ip() {
        let manager = LoginLogManager::new(100);

        manager.log_event(
            1,
            "user1".to_string(),
            "192.168.1.1".to_string(),
            LoginEvent::LoginSuccess,
            String::new(),
        );
        manager.log_event(
            2,
            "user2".to_string(),
            "192.168.1.2".to_string(),
            LoginEvent::LoginSuccess,
            String::new(),
        );

        let logs = manager.get_ip_logs("192.168.1.1");
        assert_eq!(logs.len(), 1);
    }
}
