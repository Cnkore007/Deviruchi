//! 角色服务器日志接口
//! 
//! 处理角色服务器的日志记录。
//! 对应 rAthena 的 char_logif.cpp。

use std::collections::VecDeque;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// 角色日志事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharLogEvent {
    /// 角色创建
    CharCreate,
    /// 角色删除
    CharDelete,
    /// 角色选择
    CharSelect,
    /// 角色上线
    CharOnline,
    /// 角色下线
    CharOffline,
    /// 角色重命名
    CharRename,
    /// 角色转移
    CharTransfer,
}

/// 角色日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharLogEntry {
    /// 日志 ID
    pub id: u64,
    /// 角色 ID
    pub char_id: u32,
    /// 账号 ID
    pub account_id: u32,
    /// 角色名称
    pub char_name: String,
    /// 事件类型
    pub event: CharLogEvent,
    /// 时间戳
    pub timestamp: u64,
    /// 附加信息
    pub message: String,
}

/// 角色日志管理器
pub struct CharLogManager {
    /// 日志队列
    logs: RwLock<VecDeque<CharLogEntry>>,
    /// 最大日志数量
    max_logs: usize,
    /// 下一个日志 ID
    next_id: RwLock<u64>,
}

impl CharLogManager {
    /// 创建新的角色日志管理器
    pub fn new(max_logs: usize) -> Self {
        Self {
            logs: RwLock::new(VecDeque::new()),
            max_logs,
            next_id: RwLock::new(1),
        }
    }

    /// 记录角色事件
    pub fn log_event(
        &self,
        char_id: u32,
        account_id: u32,
        char_name: String,
        event: CharLogEvent,
        message: String,
    ) {
        let mut next_id = self.next_id.write();
        let id = *next_id;
        *next_id += 1;

        let entry = CharLogEntry {
            id,
            char_id,
            account_id,
            char_name,
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
    pub fn get_recent_logs(&self, count: usize) -> Vec<CharLogEntry> {
        let logs = self.logs.read();
        let start = if logs.len() > count {
            logs.len() - count
        } else {
            0
        };
        logs.range(start..).cloned().collect()
    }

    /// 获取角色的日志
    pub fn get_char_logs(&self, char_id: u32) -> Vec<CharLogEntry> {
        self.logs
            .read()
            .iter()
            .filter(|entry| entry.char_id == char_id)
            .cloned()
            .collect()
    }

    /// 获取账号的日志
    pub fn get_account_logs(&self, account_id: u32) -> Vec<CharLogEntry> {
        self.logs
            .read()
            .iter()
            .filter(|entry| entry.account_id == account_id)
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

impl Default for CharLogManager {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_event() {
        let manager = CharLogManager::new(100);
        
        manager.log_event(
            1001,
            1,
            "测试角色".to_string(),
            CharLogEvent::CharCreate,
            "角色创建成功".to_string(),
        );
        
        assert_eq!(manager.count(), 1);
        
        let logs = manager.get_recent_logs(10);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].event, CharLogEvent::CharCreate);
    }

    #[test]
    fn test_max_logs() {
        let manager = CharLogManager::new(2);
        
        for i in 0..3 {
            manager.log_event(
                i,
                1,
                format!("角色_{}", i),
                CharLogEvent::CharCreate,
                String::new(),
            );
        }
        
        assert_eq!(manager.count(), 2);
    }

    #[test]
    fn test_filter_by_char() {
        let manager = CharLogManager::new(100);
        
        manager.log_event(1001, 1, "角色1".to_string(), CharLogEvent::CharOnline, String::new());
        manager.log_event(1002, 1, "角色2".to_string(), CharLogEvent::CharOnline, String::new());
        manager.log_event(1001, 1, "角色1".to_string(), CharLogEvent::CharOffline, String::new());
        
        let logs = manager.get_char_logs(1001);
        assert_eq!(logs.len(), 2);
    }

    #[test]
    fn test_filter_by_account() {
        let manager = CharLogManager::new(100);
        
        manager.log_event(1001, 1, "角色1".to_string(), CharLogEvent::CharOnline, String::new());
        manager.log_event(1002, 2, "角色2".to_string(), CharLogEvent::CharOnline, String::new());
        
        let logs = manager.get_account_logs(1);
        assert_eq!(logs.len(), 1);
    }
}
