//! 频道系统
//! 
//! 实现公共/私人频道，支持玩家聊天、频道管理等功能。
//! 对应 rAthena 的 channel.cpp。

use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// 频道 ID
pub type ChannelId = u32;

/// 频道类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelType {
    /// 公共频道（全局可见）
    Public,
    /// 私人频道（仅邀请可见）
    Private,
    /// 公会频道
    Guild,
    /// 队伍频道
    Party,
    /// 系统频道（公告等）
    System,
}

/// 频道配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// 频道名称
    pub name: String,
    /// 频道类型
    pub channel_type: ChannelType,
    /// 频道密码（可选）
    pub password: Option<String>,
    /// 最大成员数
    pub max_members: u32,
    /// 是否允许匿名
    pub allow_anonymous: bool,
    /// 频道描述
    pub description: String,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            channel_type: ChannelType::Public,
            password: None,
            max_members: 1000,
            allow_anonymous: false,
            description: String::new(),
        }
    }
}

/// 频道成员
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMember {
    /// 玩家 ID
    pub player_id: u32,
    /// 玩家名称
    pub player_name: String,
    /// 是否为管理员
    pub is_admin: bool,
    /// 是否静音
    pub is_muted: bool,
    /// 加入时间
    pub joined_at: u64,
}

/// 频道消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// 消息 ID
    pub id: u64,
    /// 发送者 ID
    pub sender_id: u32,
    /// 发送者名称
    pub sender_name: String,
    /// 消息内容
    pub content: String,
    /// 发送时间戳
    pub timestamp: u64,
    /// 消息类型
    pub message_type: MessageType,
}

/// 消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// 普通消息
    Normal,
    /// 系统消息
    System,
    /// 公告
    Announcement,
    /// 私聊
    Whisper,
}

/// 频道状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    /// 频道 ID
    pub id: ChannelId,
    /// 频道配置
    pub config: ChannelConfig,
    /// 频道成员
    pub members: HashMap<u32, ChannelMember>,
    /// 频道消息历史
    pub messages: Vec<ChannelMessage>,
    /// 频道所有者 ID
    pub owner_id: Option<u32>,
    /// 创建时间
    pub created_at: u64,
    /// 是否活跃
    pub is_active: bool,
}

impl Channel {
    /// 创建新频道
    pub fn new(id: ChannelId, config: ChannelConfig, owner_id: Option<u32>) -> Self {
        Self {
            id,
            config,
            members: HashMap::new(),
            messages: Vec::new(),
            owner_id,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            is_active: true,
        }
    }

    /// 添加成员
    pub fn add_member(&mut self, player_id: u32, player_name: String, is_admin: bool) -> bool {
        if self.members.len() >= self.config.max_members as usize {
            return false;
        }

        let member = ChannelMember {
            player_id,
            player_name,
            is_admin,
            is_muted: false,
            joined_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        self.members.insert(player_id, member);
        true
    }

    /// 移除成员
    pub fn remove_member(&mut self, player_id: u32) -> bool {
        self.members.remove(&player_id).is_some()
    }

    /// 发送消息
    pub fn send_message(
        &mut self,
        sender_id: u32,
        sender_name: String,
        content: String,
        message_type: MessageType,
    ) -> Option<ChannelMessage> {
        // 检查发送者是否为成员
        if let Some(member) = self.members.get(&sender_id) {
            if member.is_muted {
                return None;
            }
        } else {
            return None;
        }

        let message = ChannelMessage {
            id: self.messages.len() as u64 + 1,
            sender_id,
            sender_name,
            content,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            message_type,
        };

        self.messages.push(message.clone());
        Some(message)
    }

    /// 获取成员列表
    pub fn get_members(&self) -> Vec<&ChannelMember> {
        self.members.values().collect()
    }

    /// 获取消息历史
    pub fn get_messages(&self, limit: usize) -> Vec<&ChannelMessage> {
        let start = if self.messages.len() > limit {
            self.messages.len() - limit
        } else {
            0
        };
        self.messages[start..].iter().collect()
    }

    /// 检查玩家是否为成员
    pub fn is_member(&self, player_id: u32) -> bool {
        self.members.contains_key(&player_id)
    }

    /// 检查玩家是否为管理员
    pub fn is_admin(&self, player_id: u32) -> bool {
        self.members
            .get(&player_id)
            .map(|m| m.is_admin)
            .unwrap_or(false)
    }

    /// 设置玩家静音状态
    pub fn set_muted(&mut self, player_id: u32, muted: bool) -> bool {
        if let Some(member) = self.members.get_mut(&player_id) {
            member.is_muted = muted;
            true
        } else {
            false
        }
    }
}

/// 频道管理器
pub struct ChannelManager {
    /// 所有频道
    channels: RwLock<HashMap<ChannelId, Channel>>,
    /// 下一个频道 ID
    next_id: RwLock<ChannelId>,
    /// 玩家频道映射（玩家 ID -> 频道 ID 列表）
    player_channels: RwLock<HashMap<u32, Vec<ChannelId>>>,
}

impl ChannelManager {
    /// 创建新的频道管理器
    pub fn new() -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
            next_id: RwLock::new(1),
            player_channels: RwLock::new(HashMap::new()),
        }
    }

    /// 创建频道
    pub fn create_channel(
        &self,
        config: ChannelConfig,
        owner_id: Option<u32>,
    ) -> ChannelId {
        let mut next_id = self.next_id.write();
        let id = *next_id;
        *next_id += 1;

        let channel = Channel::new(id, config, owner_id);
        self.channels.write().insert(id, channel);

        // 如果有所有者，自动加入
        if let Some(owner) = owner_id {
            self.join_channel(id, owner, String::new(), true);
        }

        id
    }

    /// 加入频道
    pub fn join_channel(
        &self,
        channel_id: ChannelId,
        player_id: u32,
        player_name: String,
        is_admin: bool,
    ) -> bool {
        let mut channels = self.channels.write();
        if let Some(channel) = channels.get_mut(&channel_id) {
            if channel.add_member(player_id, player_name, is_admin) {
                // 更新玩家频道映射
                let mut player_channels = self.player_channels.write();
                player_channels
                    .entry(player_id)
                    .or_insert_with(Vec::new)
                    .push(channel_id);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// 离开频道
    pub fn leave_channel(&self, channel_id: ChannelId, player_id: u32) -> bool {
        let mut channels = self.channels.write();
        if let Some(channel) = channels.get_mut(&channel_id) {
            if channel.remove_member(player_id) {
                // 更新玩家频道映射
                let mut player_channels = self.player_channels.write();
                if let Some(channels) = player_channels.get_mut(&player_id) {
                    channels.retain(|&id| id != channel_id);
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// 发送消息到频道
    pub fn send_message(
        &self,
        channel_id: ChannelId,
        sender_id: u32,
        sender_name: String,
        content: String,
        message_type: MessageType,
    ) -> Option<ChannelMessage> {
        let mut channels = self.channels.write();
        if let Some(channel) = channels.get_mut(&channel_id) {
            channel.send_message(sender_id, sender_name, content, message_type)
        } else {
            None
        }
    }

    /// 获取频道信息
    pub fn get_channel(&self, channel_id: ChannelId) -> Option<Channel> {
        self.channels.read().get(&channel_id).cloned()
    }

    /// 获取玩家所在的频道
    pub fn get_player_channels(&self, player_id: u32) -> Vec<ChannelId> {
        self.player_channels
            .read()
            .get(&player_id)
            .cloned()
            .unwrap_or_default()
    }

    /// 获取所有公共频道
    pub fn get_public_channels(&self) -> Vec<Channel> {
        self.channels
            .read()
            .values()
            .filter(|c| c.config.channel_type == ChannelType::Public)
            .cloned()
            .collect()
    }

    /// 删除频道
    pub fn delete_channel(&self, channel_id: ChannelId) -> bool {
        let mut channels = self.channels.write();
        if let Some(channel) = channels.remove(&channel_id) {
            // 清理玩家频道映射
            let mut player_channels = self.player_channels.write();
            for member in channel.members.keys() {
                if let Some(channels) = player_channels.get_mut(member) {
                    channels.retain(|&id| id != channel_id);
                }
            }
            true
        } else {
            false
        }
    }

    /// 获取频道数量
    pub fn channel_count(&self) -> usize {
        self.channels.read().len()
    }

    /// 获取总成员数
    pub fn total_members(&self) -> usize {
        self.channels
            .read()
            .values()
            .map(|c| c.members.len())
            .sum()
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_channel() {
        let manager = ChannelManager::new();
        let config = ChannelConfig {
            name: "测试频道".to_string(),
            channel_type: ChannelType::Public,
            ..Default::default()
        };

        let id = manager.create_channel(config, Some(1));
        assert_eq!(id, 1);

        let channel = manager.get_channel(id).unwrap();
        assert_eq!(channel.config.name, "测试频道");
        assert!(channel.is_member(1));
    }

    #[test]
    fn test_join_leave_channel() {
        let manager = ChannelManager::new();
        let config = ChannelConfig::default();
        let id = manager.create_channel(config, Some(1));

        assert!(manager.join_channel(id, 2, "玩家2".to_string(), false));
        assert!(manager.get_channel(id).unwrap().is_member(2));

        assert!(manager.leave_channel(id, 2));
        assert!(!manager.get_channel(id).unwrap().is_member(2));
    }

    #[test]
    fn test_send_message() {
        let manager = ChannelManager::new();
        let config = ChannelConfig::default();
        let id = manager.create_channel(config, Some(1));

        let msg = manager.send_message(
            id,
            1,
            "玩家1".to_string(),
            "你好世界".to_string(),
            MessageType::Normal,
        );

        assert!(msg.is_some());
        let msg = msg.unwrap();
        assert_eq!(msg.content, "你好世界");

        let channel = manager.get_channel(id).unwrap();
        let messages = channel.get_messages(10);
    }

    #[test]
    fn test_muted_player() {
        let manager = ChannelManager::new();
        let config = ChannelConfig::default();
        let id = manager.create_channel(config, Some(1));

        manager.join_channel(id, 2, "玩家2".to_string(), false);
        
        // 静音玩家
        {
            let mut channels = manager.channels.write();
            let channel = channels.get_mut(&id).unwrap();
            channel.set_muted(2, true);
        }

        // 静音玩家无法发送消息
        let msg = manager.send_message(
            id,
            2,
            "玩家2".to_string(),
            "这条消息应该被拒绝".to_string(),
            MessageType::Normal,
        );
        assert!(msg.is_none());
    }

    #[test]
    fn test_max_members() {
        let manager = ChannelManager::new();
        let config = ChannelConfig {
            max_members: 2,
            ..Default::default()
        };
        let id = manager.create_channel(config, Some(1));

        assert!(manager.join_channel(id, 2, "玩家2".to_string(), false));
        assert!(!manager.join_channel(id, 3, "玩家3".to_string(), false));
    }

    #[test]
    fn test_player_channels() {
        let manager = ChannelManager::new();
        let config1 = ChannelConfig::default();
        let config2 = ChannelConfig::default();

        let id1 = manager.create_channel(config1, Some(1));
        let id2 = manager.create_channel(config2, Some(1));

        let channels = manager.get_player_channels(1);
        assert_eq!(channels.len(), 2);
        assert!(channels.contains(&id1));
        assert!(channels.contains(&id2));
    }

    #[test]
    fn test_delete_channel() {
        let manager = ChannelManager::new();
        let config = ChannelConfig::default();
        let id = manager.create_channel(config, Some(1));

        assert!(manager.delete_channel(id));
        assert!(manager.get_channel(id).is_none());
        assert_eq!(manager.get_player_channels(1).len(), 0);
    }
}
