//! 客户端接口
//! 
//! 处理客户端与服务器之间的通信。
//! 对应 rAthena 的 clif.cpp。

use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// 客户端操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClientOperation {
    /// 登录
    Login,
    /// 登出
    Logout,
    /// 移动
    Move,
    /// 攻击
    Attack,
    /// 聊天
    Chat,
    /// 使用技能
    UseSkill,
    /// 使用物品
    UseItem,
    /// 装备物品
    Equip,
    /// 卸下装备
    Unequip,
    /// 交易
    Trade,
    /// 组队
    Party,
    /// 公会
    Guild,
}

/// 客户端消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientMessage {
    /// 操作类型
    pub operation: ClientOperation,
    /// 玩家 ID
    pub player_id: u32,
    /// 时间戳
    pub timestamp: u64,
    /// 数据包
    pub packet: Vec<u8>,
}

/// 客户端连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientStatus {
    /// 未连接
    Disconnected,
    /// 连接中
    Connecting,
    /// 已连接
    Connected,
    /// 登录中
    LoggingIn,
    /// 已登录
    LoggedIn,
    /// 游戏中
    InGame,
}

/// 客户端连接
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConnection {
    /// 连接 ID
    pub id: u32,
    /// 客户端地址
    pub address: String,
    /// 客户端端口
    pub port: u16,
    /// 连接状态
    pub status: ClientStatus,
    /// 玩家 ID
    pub player_id: Option<u32>,
    /// 账号 ID
    pub account_id: Option<u32>,
    /// 最后活动时间
    pub last_activity: u64,
    /// 连接时间
    pub connected_at: u64,
}

/// 客户端管理器
pub struct ClientManager {
    /// 客户端连接
    connections: RwLock<HashMap<u32, ClientConnection>>,
    /// 消息队列
    message_queue: RwLock<Vec<ClientMessage>>,
    /// 下一个连接 ID
    next_connection_id: RwLock<u32>,
    /// 玩家连接映射（玩家 ID -> 连接 ID）
    player_connections: RwLock<HashMap<u32, u32>>,
}

impl ClientManager {
    /// 创建新的客户端管理器
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            message_queue: RwLock::new(Vec::new()),
            next_connection_id: RwLock::new(1),
            player_connections: RwLock::new(HashMap::new()),
        }
    }

    /// 添加客户端连接
    pub fn add_connection(
        &self,
        address: String,
        port: u16,
    ) -> u32 {
        let mut next_id = self.next_connection_id.write();
        let id = *next_id;
        *next_id += 1;

        let connection = ClientConnection {
            id,
            address,
            port,
            status: ClientStatus::Connecting,
            player_id: None,
            account_id: None,
            last_activity: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            connected_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.connections.write().insert(id, connection);
        id
    }

    /// 更新连接状态
    pub fn update_status(&self, connection_id: u32, status: ClientStatus) -> bool {
        let mut connections = self.connections.write();
        if let Some(conn) = connections.get_mut(&connection_id) {
            conn.status = status;
            conn.last_activity = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            true
        } else {
            false
        }
    }

    /// 设置玩家信息
    pub fn set_player_info(
        &self,
        connection_id: u32,
        player_id: u32,
        account_id: u32,
    ) -> bool {
        let mut connections = self.connections.write();
        if let Some(conn) = connections.get_mut(&connection_id) {
            conn.player_id = Some(player_id);
            conn.account_id = Some(account_id);
            self.player_connections.write().insert(player_id, connection_id);
            true
        } else {
            false
        }
    }

    /// 移除客户端连接
    pub fn remove_connection(&self, connection_id: u32) -> bool {
        let mut connections = self.connections.write();
        if let Some(conn) = connections.remove(&connection_id) {
            if let Some(player_id) = conn.player_id {
                self.player_connections.write().remove(&player_id);
            }
            true
        } else {
            false
        }
    }

    /// 发送消息到客户端
    pub fn send_message(&self, message: ClientMessage) {
        self.message_queue.write().push(message);
    }

    /// 处理消息队列
    pub fn process_messages(&self) -> Vec<ClientMessage> {
        let mut queue = self.message_queue.write();
        let messages: Vec<ClientMessage> = queue.drain(..).collect();
        messages
    }

    /// 获取连接信息
    pub fn get_connection(&self, connection_id: u32) -> Option<ClientConnection> {
        self.connections.read().get(&connection_id).cloned()
    }

    /// 获取玩家的连接
    pub fn get_player_connection(&self, player_id: u32) -> Option<u32> {
        self.player_connections.read().get(&player_id).copied()
    }

    /// 获取所有连接
    pub fn get_connections(&self) -> Vec<ClientConnection> {
        self.connections.read().values().cloned().collect()
    }

    /// 获取在线玩家数量
    pub fn online_player_count(&self) -> usize {
        self.connections
            .read()
            .values()
            .filter(|c| c.status == ClientStatus::InGame)
            .count()
    }

    /// 更新活动时间
    pub fn update_activity(&self, connection_id: u32) -> bool {
        let mut connections = self.connections.write();
        if let Some(conn) = connections.get_mut(&connection_id) {
            conn.last_activity = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            true
        } else {
            false
        }
    }

    /// 踢出玩家
    pub fn kick_player(&self, player_id: u32) -> bool {
        if let Some(conn_id) = self.get_player_connection(player_id) {
            self.update_status(conn_id, ClientStatus::Disconnected);
            self.remove_connection(conn_id);
            true
        } else {
            false
        }
    }

    /// 广播消息给所有在线玩家
    pub fn broadcast_message(&self, packet: Vec<u8>) {
        let connections = self.connections.read();
        for conn in connections.values() {
            if conn.status == ClientStatus::InGame {
                let message = ClientMessage {
                    operation: ClientOperation::Chat,
                    player_id: conn.player_id.unwrap_or(0),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    packet: packet.clone(),
                };
                self.message_queue.write().push(message);
            }
        }
    }
}

impl Default for ClientManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_connection() {
        let manager = ClientManager::new();
        let id = manager.add_connection("127.0.0.1".to_string(), 5000);
        
        assert_eq!(id, 1);
        let conn = manager.get_connection(id).unwrap();
        assert_eq!(conn.status, ClientStatus::Connecting);
    }

    #[test]
    fn test_player_connection() {
        let manager = ClientManager::new();
        let id = manager.add_connection("127.0.0.1".to_string(), 5000);
        
        manager.set_player_info(id, 1001, 1);
        assert_eq!(manager.get_player_connection(1001), Some(id));
    }

    #[test]
    fn test_remove_connection() {
        let manager = ClientManager::new();
        let id = manager.add_connection("127.0.0.1".to_string(), 5000);
        
        manager.set_player_info(id, 1001, 1);
        manager.remove_connection(id);
        
        assert!(manager.get_connection(id).is_none());
        assert!(manager.get_player_connection(1001).is_none());
    }

    #[test]
    fn test_kick_player() {
        let manager = ClientManager::new();
        let id = manager.add_connection("127.0.0.1".to_string(), 5000);
        
        manager.set_player_info(id, 1001, 1);
        assert!(manager.kick_player(1001));
        assert!(manager.get_player_connection(1001).is_none());
    }

    #[test]
    fn test_message_queue() {
        let manager = ClientManager::new();
        
        manager.send_message(ClientMessage {
            operation: ClientOperation::Chat,
            player_id: 1001,
            timestamp: 0,
            packet: vec![1, 2, 3],
        });
        
        let messages = manager.process_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].operation, ClientOperation::Chat);
    }
}
