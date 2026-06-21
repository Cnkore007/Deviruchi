//! 角色服务器接口
//!
//! 处理角色服务器与地图服务器之间的通信。
//! 对应 rAthena 的 chrif.cpp。

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 角色服务器操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CharServerOperation {
    /// 角色上线
    Online,
    /// 角色下线
    Offline,
    /// 角色数据同步
    DataSync,
    /// 角色传送
    Transfer,
    /// 角色删除
    Delete,
    /// 角色恢复
    Restore,
}

/// 角色服务器消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharServerMessage {
    /// 操作类型
    pub operation: CharServerOperation,
    /// 角色 ID
    pub char_id: u32,
    /// 账号 ID
    pub account_id: u32,
    /// 角色名称
    pub char_name: String,
    /// 地图服务器 ID
    pub map_server_id: u32,
    /// 时间戳
    pub timestamp: u64,
    /// 附加数据
    pub data: Vec<u8>,
}

/// 角色服务器连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharServerStatus {
    /// 未连接
    Disconnected,
    /// 连接中
    Connecting,
    /// 已连接
    Connected,
    /// 认证中
    Authenticating,
    /// 已认证
    Authenticated,
}

/// 角色服务器连接
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharServerConnection {
    /// 连接 ID
    pub id: u32,
    /// 服务器地址
    pub address: String,
    /// 服务器端口
    pub port: u16,
    /// 连接状态
    pub status: CharServerStatus,
    /// 最后心跳时间
    pub last_heartbeat: u64,
    /// 已注册的角色
    pub registered_chars: Vec<u32>,
}

/// 角色服务器管理器
pub struct CharServerManager {
    /// 角色服务器连接
    connections: RwLock<HashMap<u32, CharServerConnection>>,
    /// 消息队列
    message_queue: RwLock<Vec<CharServerMessage>>,
    /// 在线角色映射（角色 ID -> 地图服务器 ID）
    online_chars: RwLock<HashMap<u32, u32>>,
    /// 下一个连接 ID
    next_connection_id: RwLock<u32>,
}

impl CharServerManager {
    /// 创建新的角色服务器管理器
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            message_queue: RwLock::new(Vec::new()),
            online_chars: RwLock::new(HashMap::new()),
            next_connection_id: RwLock::new(1),
        }
    }

    /// 注册角色服务器连接
    pub fn register_connection(&self, address: String, port: u16) -> u32 {
        let mut next_id = self.next_connection_id.write();
        let id = *next_id;
        *next_id += 1;

        let connection = CharServerConnection {
            id,
            address,
            port,
            status: CharServerStatus::Connecting,
            last_heartbeat: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            registered_chars: Vec::new(),
        };

        self.connections.write().insert(id, connection);
        id
    }

    /// 更新连接状态
    pub fn update_connection_status(&self, connection_id: u32, status: CharServerStatus) -> bool {
        let mut connections = self.connections.write();
        if let Some(conn) = connections.get_mut(&connection_id) {
            conn.status = status;
            if status == CharServerStatus::Authenticated {
                conn.last_heartbeat = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
            }
            true
        } else {
            false
        }
    }

    /// 发送消息到角色服务器
    pub fn send_message(&self, message: CharServerMessage) {
        self.message_queue.write().push(message);
    }

    /// 处理消息队列
    pub fn process_messages(&self) -> Vec<CharServerMessage> {
        let mut queue = self.message_queue.write();
        let messages: Vec<CharServerMessage> = queue.drain(..).collect();
        messages
    }

    /// 角色上线
    pub fn char_online(&self, char_id: u32, account_id: u32, map_server_id: u32) {
        self.online_chars.write().insert(char_id, map_server_id);

        let message = CharServerMessage {
            operation: CharServerOperation::Online,
            char_id,
            account_id,
            char_name: String::new(),
            map_server_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            data: Vec::new(),
        };
        self.send_message(message);
    }

    /// 角色下线
    pub fn char_offline(&self, char_id: u32) {
        self.online_chars.write().remove(&char_id);

        let message = CharServerMessage {
            operation: CharServerOperation::Offline,
            char_id,
            account_id: 0,
            char_name: String::new(),
            map_server_id: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            data: Vec::new(),
        };
        self.send_message(message);
    }

    /// 同步角色数据
    pub fn sync_char_data(&self, char_id: u32, account_id: u32, data: Vec<u8>) {
        let message = CharServerMessage {
            operation: CharServerOperation::DataSync,
            char_id,
            account_id,
            char_name: String::new(),
            map_server_id: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            data,
        };
        self.send_message(message);
    }

    /// 检查角色是否在线
    pub fn is_char_online(&self, char_id: u32) -> bool {
        self.online_chars.read().contains_key(&char_id)
    }

    /// 获取角色所在的地图服务器
    pub fn get_char_map_server(&self, char_id: u32) -> Option<u32> {
        self.online_chars.read().get(&char_id).copied()
    }

    /// 获取在线角色数量
    pub fn online_char_count(&self) -> usize {
        self.online_chars.read().len()
    }

    /// 获取连接状态
    pub fn get_connection_status(&self, connection_id: u32) -> Option<CharServerStatus> {
        self.connections
            .read()
            .get(&connection_id)
            .map(|c| c.status)
    }

    /// 移除连接
    pub fn remove_connection(&self, connection_id: u32) -> bool {
        let mut connections = self.connections.write();
        if let Some(conn) = connections.remove(&connection_id) {
            // 清理该连接上的在线角色
            let mut online_chars = self.online_chars.write();
            for char_id in &conn.registered_chars {
                online_chars.remove(char_id);
            }
            true
        } else {
            false
        }
    }

    /// 获取所有连接
    pub fn get_connections(&self) -> Vec<CharServerConnection> {
        self.connections.read().values().cloned().collect()
    }

    /// 更新心跳
    pub fn update_heartbeat(&self, connection_id: u32) -> bool {
        let mut connections = self.connections.write();
        if let Some(conn) = connections.get_mut(&connection_id) {
            conn.last_heartbeat = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            true
        } else {
            false
        }
    }
}

impl Default for CharServerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_connection() {
        let manager = CharServerManager::new();
        let id = manager.register_connection("127.0.0.1".to_string(), 6000);

        assert_eq!(id, 1);
        assert_eq!(
            manager.get_connection_status(id),
            Some(CharServerStatus::Connecting)
        );
    }

    #[test]
    fn test_char_online_offline() {
        let manager = CharServerManager::new();

        manager.char_online(1001, 1, 1);
        assert!(manager.is_char_online(1001));
        assert_eq!(manager.get_char_map_server(1001), Some(1));

        manager.char_offline(1001);
        assert!(!manager.is_char_online(1001));
    }

    #[test]
    fn test_message_queue() {
        let manager = CharServerManager::new();

        manager.char_online(1001, 1, 1);
        manager.char_online(1002, 2, 1);

        let messages = manager.process_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].char_id, 1001);
        assert_eq!(messages[1].char_id, 1002);
    }

    #[test]
    fn test_sync_char_data() {
        let manager = CharServerManager::new();

        let data = vec![1, 2, 3, 4, 5];
        manager.sync_char_data(1001, 1, data.clone());

        let messages = manager.process_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].operation, CharServerOperation::DataSync);
        assert_eq!(messages[0].data, data);
    }

    #[test]
    fn test_multiple_connections() {
        let manager = CharServerManager::new();

        let id1 = manager.register_connection("127.0.0.1".to_string(), 6000);
        let id2 = manager.register_connection("127.0.0.2".to_string(), 6001);

        assert_eq!(manager.get_connections().len(), 2);

        manager.remove_connection(id1);
        assert_eq!(manager.get_connections().len(), 1);
    }

    #[test]
    fn test_update_heartbeat() {
        let manager = CharServerManager::new();
        let id = manager.register_connection("127.0.0.1".to_string(), 6000);

        let initial_status = manager.get_connection_status(id);
        assert!(manager.update_heartbeat(id));
    }
}
