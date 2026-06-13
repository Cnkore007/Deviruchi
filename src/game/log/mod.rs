//! 游戏日志系统
//!
//! 对应 rAthena 的 `src/map/log.cpp`，提供游戏事件日志记录功能。
//! 包括物品获取/丢失、金币交易、聊天记录、GM 命令等。

use std::collections::HashMap;
use parking_lot::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Local};

/// 日志类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogType {
    /// 物品获取
    Pick,
    /// 物品丢失
    Drop,
    /// 金币交易
    Zeny,
    /// 聊天消息
    Chat,
    /// GM 命令
    AtCommand,
    /// NPC 交互
    Npc,
    /// 现金商店
    Cash,
    /// MVP 掉落
    MvpDrop,
    /// 喂养
    Feeding,
    /// 分支（如公会）
    Branch,
}

/// 物品获取来源
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickSource {
    /// 从地面捡取
    Floor,
    /// 从怪物掉落
    Monster,
    /// 从 NPC 获取
    Npc,
    /// 从商店购买
    Shop,
    /// 从交易获取
    Trade,
    /// 从仓库取出
    Storage,
    /// 从邮件获取
    Mail,
    /// 其他
    Other,
}

/// 聊天类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatType {
    /// 普通聊天
    Normal,
    /// 公会聊天
    Guild,
    /// 队伍聊天
    Party,
    /// 密语
    Whisper,
    /// 系统消息
    System,
}

/// 日志条目
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// 日志 ID
    pub id: u64,
    /// 时间戳
    pub timestamp: DateTime<Local>,
    /// 日志类型
    pub log_type: LogType,
    /// 相关玩家 ID
    pub player_id: Option<Uuid>,
    /// 相关账号 ID
    pub account_id: Option<u32>,
    /// 相关角色 ID
    pub char_id: Option<u32>,
    /// 详细信息
    pub details: LogDetails,
}

/// 日志详细信息
#[derive(Debug, Clone)]
pub enum LogDetails {
    /// 物品日志
    Item {
        item_id: u16,
        amount: i32,
        source: PickSource,
        map_name: String,
        x: u16,
        y: u16,
    },
    /// 金币日志
    Zeny {
        amount: i64,
        source: String,
        map_name: String,
    },
    /// 聊天日志
    Chat {
        chat_type: ChatType,
        message: String,
        map_name: String,
        x: u16,
        y: u16,
    },
    /// GM 命令日志
    AtCommand {
        command: String,
        arguments: String,
        map_name: String,
        x: u16,
        y: u16,
    },
    /// NPC 交互日志
    Npc {
        npc_id: u32,
        npc_name: String,
        action: String,
    },
    /// 现金商店日志
    Cash {
        item_id: u16,
        amount: u16,
        cash_type: String,
        points: u32,
    },
    /// MVP 掉落日志
    MvpDrop {
        mob_id: u16,
        item_id: u16,
        amount: u16,
    },
    /// 喂养日志
    Feeding {
        pet_id: u16,
        food_item_id: u16,
        intimacy: i32,
    },
}

/// 日志配置
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// 是否启用日志
    pub enabled: bool,
    /// 是否记录物品日志
    pub log_items: bool,
    /// 是否记录金币日志
    pub log_zeny: bool,
    /// 是否记录聊天日志
    pub log_chat: bool,
    /// 是否记录 GM 命令
    pub log_commands: bool,
    /// 是否记录 NPC 交互
    pub log_npc: bool,
    /// 是否记录现金商店
    pub log_cash: bool,
    /// 最小日志等级（0=全部，1=重要，2=仅错误）
    pub min_level: u8,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_items: true,
            log_zeny: true,
            log_chat: true,
            log_commands: true,
            log_npc: true,
            log_cash: true,
            min_level: 0,
        }
    }
}

/// 游戏日志管理器
///
/// 管理所有游戏事件日志，支持异步写入和批量持久化。
pub struct LogManager {
    /// 日志配置
    config: RwLock<LogConfig>,
    /// 内存日志缓冲区
    buffer: RwLock<Vec<LogEntry>>,
    /// 下一个日志 ID
    next_id: RwLock<u64>,
    /// 日志统计
    stats: RwLock<HashMap<LogType, u64>>,
}

impl LogManager {
    /// 创建日志管理器
    pub fn new() -> Self {
        Self {
            config: RwLock::new(LogConfig::default()),
            buffer: RwLock::new(Vec::new()),
            next_id: RwLock::new(1),
            stats: RwLock::new(HashMap::new()),
        }
    }

    /// 使用指定配置创建
    pub fn with_config(config: LogConfig) -> Self {
        Self {
            config: RwLock::new(config),
            buffer: RwLock::new(Vec::new()),
            next_id: RwLock::new(1),
            stats: RwLock::new(HashMap::new()),
        }
    }

    /// 记录物品日志
    #[allow(clippy::too_many_arguments)]
    pub fn log_pick(
        &self,
        player_id: Uuid,
        account_id: u32,
        char_id: u32,
        item_id: u16,
        amount: i32,
        source: PickSource,
        map_name: &str,
        x: u16,
        y: u16,
    ) {
        let config = self.config.read();
        if !config.enabled || !config.log_items {
            return;
        }

        let entry = LogEntry {
            id: self.next_id(),
            timestamp: Local::now(),
            log_type: LogType::Pick,
            player_id: Some(player_id),
            account_id: Some(account_id),
            char_id: Some(char_id),
            details: LogDetails::Item {
                item_id,
                amount,
                source,
                map_name: map_name.to_string(),
                x,
                y,
            },
        };

        self.add_entry(entry);
    }

    /// 记录金币日志
    pub fn log_zeny(
        &self,
        player_id: Uuid,
        account_id: u32,
        char_id: u32,
        amount: i64,
        source: &str,
        map_name: &str,
    ) {
        let config = self.config.read();
        if !config.enabled || !config.log_zeny {
            return;
        }

        let entry = LogEntry {
            id: self.next_id(),
            timestamp: Local::now(),
            log_type: LogType::Zeny,
            player_id: Some(player_id),
            account_id: Some(account_id),
            char_id: Some(char_id),
            details: LogDetails::Zeny {
                amount,
                source: source.to_string(),
                map_name: map_name.to_string(),
            },
        };

        self.add_entry(entry);
    }

    /// 记录聊天日志
    #[allow(clippy::too_many_arguments)]
    pub fn log_chat(
        &self,
        player_id: Uuid,
        account_id: u32,
        char_id: u32,
        chat_type: ChatType,
        message: &str,
        map_name: &str,
        x: u16,
        y: u16,
    ) {
        let config = self.config.read();
        if !config.enabled || !config.log_chat {
            return;
        }

        let entry = LogEntry {
            id: self.next_id(),
            timestamp: Local::now(),
            log_type: LogType::Chat,
            player_id: Some(player_id),
            account_id: Some(account_id),
            char_id: Some(char_id),
            details: LogDetails::Chat {
                chat_type,
                message: message.to_string(),
                map_name: map_name.to_string(),
                x,
                y,
            },
        };

        self.add_entry(entry);
    }

    /// 记录 GM 命令日志
    #[allow(clippy::too_many_arguments)]
    pub fn log_atcommand(
        &self,
        player_id: Uuid,
        account_id: u32,
        char_id: u32,
        command: &str,
        arguments: &str,
        map_name: &str,
        x: u16,
        y: u16,
    ) {
        let config = self.config.read();
        if !config.enabled || !config.log_commands {
            return;
        }

        let entry = LogEntry {
            id: self.next_id(),
            timestamp: Local::now(),
            log_type: LogType::AtCommand,
            player_id: Some(player_id),
            account_id: Some(account_id),
            char_id: Some(char_id),
            details: LogDetails::AtCommand {
                command: command.to_string(),
                arguments: arguments.to_string(),
                map_name: map_name.to_string(),
                x,
                y,
            },
        };

        self.add_entry(entry);
    }

    /// 记录 NPC 交互日志
    pub fn log_npc(
        &self,
        player_id: Uuid,
        account_id: u32,
        char_id: u32,
        npc_id: u32,
        npc_name: &str,
        action: &str,
    ) {
        let config = self.config.read();
        if !config.enabled || !config.log_npc {
            return;
        }

        let entry = LogEntry {
            id: self.next_id(),
            timestamp: Local::now(),
            log_type: LogType::Npc,
            player_id: Some(player_id),
            account_id: Some(account_id),
            char_id: Some(char_id),
            details: LogDetails::Npc {
                npc_id,
                npc_name: npc_name.to_string(),
                action: action.to_string(),
            },
        };

        self.add_entry(entry);
    }

    /// 记录现金商店日志
    #[allow(clippy::too_many_arguments)]
    pub fn log_cash(
        &self,
        player_id: Uuid,
        account_id: u32,
        char_id: u32,
        item_id: u16,
        amount: u16,
        cash_type: &str,
        points: u32,
    ) {
        let config = self.config.read();
        if !config.enabled || !config.log_cash {
            return;
        }

        let entry = LogEntry {
            id: self.next_id(),
            timestamp: Local::now(),
            log_type: LogType::Cash,
            player_id: Some(player_id),
            account_id: Some(account_id),
            char_id: Some(char_id),
            details: LogDetails::Cash {
                item_id,
                amount,
                cash_type: cash_type.to_string(),
                points,
            },
        };

        self.add_entry(entry);
    }

    /// 记录 MVP 掉落日志
    pub fn log_mvpdrop(
        &self,
        player_id: Uuid,
        account_id: u32,
        char_id: u32,
        mob_id: u16,
        item_id: u16,
        amount: u16,
    ) {
        let config = self.config.read();
        if !config.enabled {
            return;
        }

        let entry = LogEntry {
            id: self.next_id(),
            timestamp: Local::now(),
            log_type: LogType::MvpDrop,
            player_id: Some(player_id),
            account_id: Some(account_id),
            char_id: Some(char_id),
            details: LogDetails::MvpDrop {
                mob_id,
                item_id,
                amount,
            },
        };

        self.add_entry(entry);
    }

    /// 获取缓冲区中的日志数量
    pub fn buffer_size(&self) -> usize {
        self.buffer.read().len()
    }

    /// 获取指定类型的日志统计
    pub fn get_stats(&self, log_type: LogType) -> u64 {
        self.stats.read().get(&log_type).copied().unwrap_or(0)
    }

    /// 获取所有日志统计
    pub fn get_all_stats(&self) -> HashMap<LogType, u64> {
        self.stats.read().clone()
    }

    /// 刷新缓冲区（返回并清空）
    pub fn flush(&self) -> Vec<LogEntry> {
        let mut buffer = self.buffer.write();
        
        buffer.drain(..).collect()
    }

    /// 更新配置
    pub fn update_config(&self, config: LogConfig) {
        *self.config.write() = config;
    }

    /// 获取配置
    pub fn get_config(&self) -> LogConfig {
        self.config.read().clone()
    }

    /// 内部：获取下一个日志 ID
    fn next_id(&self) -> u64 {
        let mut id = self.next_id.write();
        let current = *id;
        *id += 1;
        current
    }

    /// 内部：添加日志条目
    fn add_entry(&self, entry: LogEntry) {
        let log_type = entry.log_type;
        self.buffer.write().push(entry);
        *self.stats.write().entry(log_type).or_insert(0) += 1;
    }
}

impl Default for LogManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_manager_new() {
        let manager = LogManager::new();
        assert_eq!(manager.buffer_size(), 0);
    }

    #[test]
    fn test_log_pick() {
        let manager = LogManager::new();
        let player_id = Uuid::new_v4();

        manager.log_pick(
            player_id, 1001, 2001, 501, 10, PickSource::Floor, "prontera", 150, 150,
        );

        assert_eq!(manager.buffer_size(), 1);
        assert_eq!(manager.get_stats(LogType::Pick), 1);
    }

    #[test]
    fn test_log_zeny() {
        let manager = LogManager::new();
        let player_id = Uuid::new_v4();

        manager.log_zeny(player_id, 1001, 2001, 10000, "shop_buy", "prontera");

        assert_eq!(manager.buffer_size(), 1);
        assert_eq!(manager.get_stats(LogType::Zeny), 1);
    }

    #[test]
    fn test_log_chat() {
        let manager = LogManager::new();
        let player_id = Uuid::new_v4();

        manager.log_chat(
            player_id, 1001, 2001, ChatType::Normal, "Hello!", "prontera", 150, 150,
        );

        assert_eq!(manager.buffer_size(), 1);
        assert_eq!(manager.get_stats(LogType::Chat), 1);
    }

    #[test]
    fn test_log_atcommand() {
        let manager = LogManager::new();
        let player_id = Uuid::new_v4();

        manager.log_atcommand(
            player_id, 1001, 2001, "@warp", "prontera 150 150", "prontera", 150, 150,
        );

        assert_eq!(manager.buffer_size(), 1);
        assert_eq!(manager.get_stats(LogType::AtCommand), 1);
    }

    #[test]
    fn test_log_disabled() {
        let config = LogConfig {
            enabled: false,
            ..Default::default()
        };
        let manager = LogManager::with_config(config);
        let player_id = Uuid::new_v4();

        manager.log_pick(
            player_id, 1001, 2001, 501, 10, PickSource::Floor, "prontera", 150, 150,
        );

        assert_eq!(manager.buffer_size(), 0);
    }

    #[test]
    fn test_log_items_disabled() {
        let config = LogConfig {
            log_items: false,
            ..Default::default()
        };
        let manager = LogManager::with_config(config);
        let player_id = Uuid::new_v4();

        manager.log_pick(
            player_id, 1001, 2001, 501, 10, PickSource::Floor, "prontera", 150, 150,
        );

        assert_eq!(manager.buffer_size(), 0);

        // 其他类型应该正常记录
        manager.log_zeny(player_id, 1001, 2001, 1000, "test", "prontera");
        assert_eq!(manager.buffer_size(), 1);
    }

    #[test]
    fn test_flush() {
        let manager = LogManager::new();
        let player_id = Uuid::new_v4();

        manager.log_pick(
            player_id, 1001, 2001, 501, 10, PickSource::Floor, "prontera", 150, 150,
        );
        manager.log_zeny(player_id, 1001, 2001, 1000, "test", "prontera");

        assert_eq!(manager.buffer_size(), 2);

        let entries = manager.flush();
        assert_eq!(entries.len(), 2);
        assert_eq!(manager.buffer_size(), 0);
    }

    #[test]
    fn test_log_stats() {
        let manager = LogManager::new();
        let player_id = Uuid::new_v4();

        manager.log_pick(
            player_id, 1001, 2001, 501, 10, PickSource::Floor, "prontera", 150, 150,
        );
        manager.log_pick(
            player_id, 1001, 2001, 502, 5, PickSource::Monster, "gef_fild01", 100, 100,
        );
        manager.log_zeny(player_id, 1001, 2001, 1000, "test", "prontera");

        let stats = manager.get_all_stats();
        assert_eq!(stats.get(&LogType::Pick), Some(&2));
        assert_eq!(stats.get(&LogType::Zeny), Some(&1));
    }
}
