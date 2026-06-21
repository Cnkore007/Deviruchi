//! 服务器间通信接口
//!
//! 对应 rAthena 的 `src/map/intif.cpp`，提供 Map Server 与 Char Server 之间的通信功能。
//! 包括组队、公会、聊天、仓库等跨服务器操作。

use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

type HandlerMap = HashMap<InterMessageType, Box<dyn Fn(&InterMessage) + Send + Sync>>;

/// 服务器间消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum InterMessageType {
    // 组队相关
    PartyCreate = 0x1000,
    PartyInfo = 0x1001,
    PartyAddMember = 0x1002,
    PartyLeave = 0x1003,
    PartyChangeOption = 0x1004,
    PartyMessage = 0x1005,
    PartyLeaderChange = 0x1006,
    PartyShareLevelUpdate = 0x1007,

    // 公会相关
    GuildCreate = 0x2000,
    GuildInfo = 0x2001,
    GuildAddMember = 0x2002,
    GuildLeave = 0x2003,
    GuildMessage = 0x2004,
    GuildAlliance = 0x2005,
    GuildCastleInfo = 0x2006,

    // 聊天相关
    WhisperMessage = 0x3000,
    WhisperReply = 0x3001,
    Broadcast = 0x3002,
    MainMessage = 0x3003,

    // 仓库相关
    StorageRequest = 0x4000,
    StorageSave = 0x4001,
    GuildStorageRequest = 0x4002,
    GuildStorageSave = 0x4003,

    // 角色数据
    SaveRegistry = 0x5000,
    RequestRegistry = 0x5001,
    Rename = 0x5002,

    // 宠物相关
    PetCreate = 0x6000,
    PetRequestData = 0x6001,
    PetSaveData = 0x6002,
    PetDelete = 0x6003,

    // 邮件相关
    MailSend = 0x7000,
    MailRequest = 0x7001,
    MailDelete = 0x7002,
    MailGetAttach = 0x7003,

    // 拍卖相关
    AuctionCreate = 0x8000,
    AuctionBid = 0x8001,
    AuctionSearch = 0x8002,
}

/// 服务器间消息
#[derive(Debug, Clone)]
pub struct InterMessage {
    /// 消息类型
    pub msg_type: InterMessageType,
    /// 发送者服务器 ID
    pub sender_id: u32,
    /// 目标服务器 ID（0 = 广播）
    pub target_id: u32,
    /// 消息数据
    pub data: Vec<u8>,
    /// 时间戳
    pub timestamp: u64,
}

impl InterMessage {
    /// 创建新消息
    pub fn new(msg_type: InterMessageType, sender_id: u32, data: Vec<u8>) -> Self {
        Self {
            msg_type,
            sender_id,
            target_id: 0,
            data,
            timestamp: current_time_ms(),
        }
    }

    /// 设置目标服务器
    pub fn with_target(mut self, target_id: u32) -> Self {
        self.target_id = target_id;
        self
    }
}

/// 服务器间通信错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterError {
    /// 目标服务器不存在
    ServerNotFound,
    /// 连接断开
    Disconnected,
    /// 消息发送失败
    SendFailed,
    /// 超时
    Timeout,
    /// 数据格式错误
    InvalidData,
    /// 权限不足
    PermissionDenied,
}

/// 服务器信息
#[derive(Debug, Clone)]
pub struct ServerInfo {
    /// 服务器 ID
    pub id: u32,
    /// 服务器名称
    pub name: String,
    /// 服务器类型
    pub server_type: ServerType,
    /// IP 地址
    pub ip: String,
    /// 端口
    pub port: u16,
    /// 是否在线
    pub online: bool,
    /// 玩家数量
    pub player_count: u32,
}

/// 服务器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerType {
    /// 登录服务器
    Login,
    /// 角色服务器
    Char,
    /// 地图服务器
    Map,
}

/// 组队数据
#[derive(Debug, Clone)]
pub struct PartyData {
    /// 组队 ID
    pub party_id: u32,
    /// 组队名称
    pub name: String,
    /// 队长 ID
    pub leader_id: Uuid,
    /// 成员列表
    pub members: Vec<PartyMember>,
    /// 组队选项
    pub options: PartyOptions,
}

/// 组队成员
#[derive(Debug, Clone)]
pub struct PartyMember {
    /// 角色 ID
    pub char_id: Uuid,
    /// 角色名
    pub name: String,
    /// 地图名
    pub map_name: String,
    /// 是否在线
    pub online: bool,
    /// 等级
    pub level: u16,
}

/// 组队选项
#[derive(Debug, Clone)]
pub struct PartyOptions {
    /// 经验分配模式
    pub exp_share: u8,
    /// 物品分配模式
    pub item_share: u8,
}

/// 公会数据
#[derive(Debug, Clone)]
pub struct GuildData {
    /// 公会 ID
    pub guild_id: u32,
    /// 公会名称
    pub name: String,
    /// 会长 ID
    pub leader_id: Uuid,
    /// 成员列表
    pub members: Vec<GuildMember>,
    /// 联盟列表
    pub alliances: Vec<GuildAlliance>,
}

/// 公会成员
#[derive(Debug, Clone)]
pub struct GuildMember {
    /// 角色 ID
    pub char_id: Uuid,
    /// 角色名
    pub name: String,
    /// 职位
    pub position: String,
    /// 是否在线
    pub online: bool,
}

/// 公会联盟
#[derive(Debug, Clone)]
pub struct GuildAlliance {
    /// 公会 ID
    pub guild_id: u32,
    /// 关系类型
    pub relation: AllianceRelation,
}

/// 联盟关系
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllianceRelation {
    /// 同盟
    Ally,
    /// 敌对
    Opposition,
}

/// 服务器间通信管理器
///
/// 管理 Map Server 与其他服务器之间的通信。
pub struct InterServerManager {
    /// 已注册的服务器
    servers: RwLock<HashMap<u32, ServerInfo>>,
    /// 待处理消息队列
    message_queue: RwLock<Vec<InterMessage>>,
    /// 消息处理器
    handlers: RwLock<HandlerMap>,
    /// 本地服务器 ID
    local_server_id: u32,
}

impl InterServerManager {
    /// 创建服务器间通信管理器
    pub fn new(local_server_id: u32) -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
            message_queue: RwLock::new(Vec::new()),
            handlers: RwLock::new(HashMap::new()),
            local_server_id,
        }
    }

    /// 注册服务器
    pub fn register_server(&self, info: ServerInfo) {
        self.servers.write().insert(info.id, info);
    }

    /// 移除服务器
    pub fn unregister_server(&self, server_id: u32) {
        self.servers.write().remove(&server_id);
    }

    /// 获取服务器信息
    pub fn get_server(&self, server_id: u32) -> Option<ServerInfo> {
        self.servers.read().get(&server_id).cloned()
    }

    /// 获取所有在线服务器
    pub fn get_online_servers(&self) -> Vec<ServerInfo> {
        self.servers
            .read()
            .values()
            .filter(|s| s.online)
            .cloned()
            .collect()
    }

    /// 发送消息
    pub fn send_message(&self, message: InterMessage) -> Result<(), InterError> {
        // 检查目标服务器
        if message.target_id != 0 {
            let servers = self.servers.read();
            if !servers.contains_key(&message.target_id) {
                return Err(InterError::ServerNotFound);
            }
        }

        // 添加到消息队列
        self.message_queue.write().push(message);
        Ok(())
    }

    /// 处理消息队列
    pub fn process_messages(&self) {
        let messages: Vec<InterMessage> = {
            let mut queue = self.message_queue.write();
            queue.drain(..).collect()
        };

        for message in messages {
            let handlers = self.handlers.read();
            if let Some(handler) = handlers.get(&message.msg_type) {
                handler(&message);
            }
        }
    }

    /// 注册消息处理器
    pub fn register_handler<F>(&self, msg_type: InterMessageType, handler: F)
    where
        F: Fn(&InterMessage) + Send + Sync + 'static,
    {
        self.handlers.write().insert(msg_type, Box::new(handler));
    }

    /// 发送组队创建请求
    pub fn party_create(&self, name: &str, leader_id: Uuid) -> Result<(), InterError> {
        let data = format!("{}:{}", name, leader_id).into_bytes();
        let message = InterMessage::new(InterMessageType::PartyCreate, self.local_server_id, data);
        self.send_message(message)
    }

    /// 发送组队消息
    pub fn party_message(
        &self,
        party_id: u32,
        sender_id: Uuid,
        message: &str,
    ) -> Result<(), InterError> {
        let data = format!("{}:{}:{}", party_id, sender_id, message).into_bytes();
        let msg = InterMessage::new(InterMessageType::PartyMessage, self.local_server_id, data);
        self.send_message(msg)
    }

    /// 发送公会创建请求
    pub fn guild_create(&self, name: &str, leader_id: Uuid) -> Result<(), InterError> {
        let data = format!("{}:{}", name, leader_id).into_bytes();
        let message = InterMessage::new(InterMessageType::GuildCreate, self.local_server_id, data);
        self.send_message(message)
    }

    /// 发送公会消息
    pub fn guild_message(
        &self,
        guild_id: u32,
        sender_id: Uuid,
        message: &str,
    ) -> Result<(), InterError> {
        let data = format!("{}:{}:{}", guild_id, sender_id, message).into_bytes();
        let msg = InterMessage::new(InterMessageType::GuildMessage, self.local_server_id, data);
        self.send_message(msg)
    }

    /// 发送密语消息
    pub fn whisper_message(
        &self,
        sender_id: Uuid,
        target_name: &str,
        message: &str,
    ) -> Result<(), InterError> {
        let data = format!("{}:{}:{}", sender_id, target_name, message).into_bytes();
        let msg = InterMessage::new(InterMessageType::WhisperMessage, self.local_server_id, data);
        self.send_message(msg)
    }

    /// 发送广播消息
    pub fn broadcast(&self, message: &str, color: u32) -> Result<(), InterError> {
        let data = format!("{}:{}", color, message).into_bytes();
        let msg = InterMessage::new(InterMessageType::Broadcast, self.local_server_id, data);
        self.send_message(msg)
    }

    /// 请求仓库数据
    pub fn request_storage(&self, char_id: Uuid) -> Result<(), InterError> {
        let data = char_id.to_string().into_bytes();
        let message =
            InterMessage::new(InterMessageType::StorageRequest, self.local_server_id, data);
        self.send_message(message)
    }

    /// 保存仓库数据
    pub fn save_storage(&self, char_id: Uuid, storage_data: &[u8]) -> Result<(), InterError> {
        let mut data = char_id.to_string().into_bytes();
        data.extend_from_slice(storage_data);
        let message = InterMessage::new(InterMessageType::StorageSave, self.local_server_id, data);
        self.send_message(message)
    }

    /// 获取本地服务器 ID
    pub fn local_server_id(&self) -> u32 {
        self.local_server_id
    }

    /// 获取消息队列长度
    pub fn queue_size(&self) -> usize {
        self.message_queue.read().len()
    }
}

/// 获取当前时间戳（毫秒）
fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inter_server_manager_new() {
        let manager = InterServerManager::new(1);
        assert_eq!(manager.local_server_id(), 1);
    }

    #[test]
    fn test_register_server() {
        let manager = InterServerManager::new(1);

        let info = ServerInfo {
            id: 2,
            name: "Char Server".to_string(),
            server_type: ServerType::Char,
            ip: "127.0.0.1".to_string(),
            port: 6000,
            online: true,
            player_count: 0,
        };

        manager.register_server(info);
        assert!(manager.get_server(2).is_some());
    }

    #[test]
    fn test_unregister_server() {
        let manager = InterServerManager::new(1);

        let info = ServerInfo {
            id: 2,
            name: "Char Server".to_string(),
            server_type: ServerType::Char,
            ip: "127.0.0.1".to_string(),
            port: 6000,
            online: true,
            player_count: 0,
        };

        manager.register_server(info);
        manager.unregister_server(2);
        assert!(manager.get_server(2).is_none());
    }

    #[test]
    fn test_send_message() {
        let manager = InterServerManager::new(1);

        let info = ServerInfo {
            id: 2,
            name: "Char Server".to_string(),
            server_type: ServerType::Char,
            ip: "127.0.0.1".to_string(),
            port: 6000,
            online: true,
            player_count: 0,
        };

        manager.register_server(info);

        let message =
            InterMessage::new(InterMessageType::PartyCreate, 1, vec![1, 2, 3]).with_target(2);

        let result = manager.send_message(message);
        assert!(result.is_ok());
        assert_eq!(manager.queue_size(), 1);
    }

    #[test]
    fn test_send_message_target_not_found() {
        let manager = InterServerManager::new(1);

        let message =
            InterMessage::new(InterMessageType::PartyCreate, 1, vec![1, 2, 3]).with_target(99);

        let result = manager.send_message(message);
        assert_eq!(result, Err(InterError::ServerNotFound));
    }

    #[test]
    fn test_party_create() {
        let manager = InterServerManager::new(1);
        let leader_id = Uuid::new_v4();

        let result = manager.party_create("Test Party", leader_id);
        assert!(result.is_ok());
        assert_eq!(manager.queue_size(), 1);
    }

    #[test]
    fn test_guild_create() {
        let manager = InterServerManager::new(1);
        let leader_id = Uuid::new_v4();

        let result = manager.guild_create("Test Guild", leader_id);
        assert!(result.is_ok());
        assert_eq!(manager.queue_size(), 1);
    }

    #[test]
    fn test_whisper_message() {
        let manager = InterServerManager::new(1);
        let sender_id = Uuid::new_v4();

        let result = manager.whisper_message(sender_id, "TargetPlayer", "Hello!");
        assert!(result.is_ok());
        assert_eq!(manager.queue_size(), 1);
    }

    #[test]
    fn test_broadcast() {
        let manager = InterServerManager::new(1);

        let result = manager.broadcast("Server announcement!", 0x00FF00);
        assert!(result.is_ok());
        assert_eq!(manager.queue_size(), 1);
    }

    #[test]
    fn test_process_messages() {
        let manager = InterServerManager::new(1);
        let _received = false;

        manager.register_handler(InterMessageType::Broadcast, move |_msg| {
            // 处理消息
        });

        manager.broadcast("Test", 0).unwrap();
        manager.process_messages();

        assert_eq!(manager.queue_size(), 0);
    }

    #[test]
    fn test_get_online_servers() {
        let manager = InterServerManager::new(1);

        manager.register_server(ServerInfo {
            id: 2,
            name: "Char Server".to_string(),
            server_type: ServerType::Char,
            ip: "127.0.0.1".to_string(),
            port: 6000,
            online: true,
            player_count: 10,
        });

        manager.register_server(ServerInfo {
            id: 3,
            name: "Offline Server".to_string(),
            server_type: ServerType::Map,
            ip: "127.0.0.1".to_string(),
            port: 6121,
            online: false,
            player_count: 0,
        });

        let online = manager.get_online_servers();
        assert_eq!(online.len(), 1);
        assert_eq!(online[0].id, 2);
    }
}
