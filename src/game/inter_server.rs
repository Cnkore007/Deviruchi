//! Inter-Server Protocol - 服务器间通信协议
//!
//! 定义Char↔Map、Login↔Char之间的数据包格式

use crate::game::server_registry::ServerInfo;
use serde::{Deserialize, Serialize};

/// 角色传输数据结构
/// Char服务器生成，Map服务器接收
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterTransfer {
    /// 角色ID
    pub char_id: u32,
    /// 账户ID
    pub account_id: u32,
    /// 角色名称
    pub name: String,
    /// 等级
    pub level: u16,
    /// 职业
    pub job: u16,
    /// 当前HP
    pub hp: u32,
    /// 最大HP
    pub max_hp: u32,
    /// 当前SP
    pub sp: u32,
    /// 最大SP
    pub max_sp: u32,
    /// 地图名称
    pub map_name: String,
    /// 当前位置X
    pub pos_x: u16,
    /// 当前位置Y
    pub pos_y: u16,
    /// 存储点地图
    pub save_map: String,
    /// 存储点X
    pub save_x: u16,
    /// 存储点Y
    pub save_y: u16,
    /// 力量
    pub str: u8,
    /// 敏捷
    pub agi: u8,
    /// 体质
    pub vit: u8,
    /// 智力
    pub int: u8,
    /// 灵巧
    pub dex: u8,
    /// 幸运
    pub luk: u8,
    /// 金钱
    pub zeny: u32,
    /// 性别 (0=女, 1=男)
    pub sex: u8,
    /// 头发颜色
    pub hair_color: u16,
    /// 头发类型
    pub hair: u16,
    /// 披风ID
    pub cloak_id: u16,
    /// 靴子ID
    pub boots_id: u16,
    /// 账户等级 (GM等级等)
    pub account_level: u32,
}

impl CharacterTransfer {
    /// 从数据库角色创建传输数据
    #[allow(dead_code)]
    pub fn from_character(
        char: &crate::storage::Character,
        account_id: u32,
        account_level: u32,
    ) -> Self {
        Self {
            char_id: char.char_id,
            account_id,
            name: char.name.clone(),
            level: char.base_level,
            job: char.class,
            hp: char.hp,
            max_hp: char.max_hp,
            sp: char.sp,
            max_sp: char.max_sp,
            map_name: char.last_map.clone(),
            pos_x: char.last_x as u16,
            pos_y: char.last_y as u16,
            save_map: char.last_map.clone(), // 使用last_map作为save_map的默认值
            save_x: char.last_x as u16,
            save_y: char.last_y as u16,
            str: char.str as u8,
            agi: char.agi as u8,
            vit: char.vit as u8,
            int: char.int as u8,
            dex: char.dex as u8,
            luk: char.luk as u8,
            zeny: char.zeny,
            sex: 1, // 默认男性，可根据需要扩展
            hair_color: char.hair_color,
            hair: char.hair,
            cloak_id: 0,
            boots_id: 0,
            account_level,
        }
    }
}

/// Inter-Server 数据包枚举
#[derive(Debug, Clone)]
pub enum InterServerPacket {
    /// Char → Map: 请求角色进入
    CharToMap {
        char_id: u32,
        account_id: u32,
        token: String,
        map_server_id: u32,
        character_data: CharacterTransfer,
    },

    /// Map → Char: 角色进入响应
    MapToChar {
        char_id: u32,
        map_server_id: u32,
        status: TransferStatus,
    },

    /// 心跳包
    Heartbeat {
        server_id: u32,
        server_type: ServerTypeProto,
        timestamp: u64,
        online_players: u32,
    },

    /// 服务器注册
    ServerRegister { server_info: ServerInfo },

    /// 服务器注销
    ServerUnregister { server_id: u32 },

    /// 角色离开地图（保存数据）
    CharLeaveMap {
        char_id: u32,
        map_server_id: u32,
        character_data: CharacterTransfer,
    },

    /// 请求角色数据（Map → Char）
    RequestCharData { char_id: u32, map_server_id: u32 },

    /// 响应角色数据（Char → Map）
    CharDataResponse {
        char_id: u32,
        character_data: Option<CharacterTransfer>,
    },

    /// Token验证请求（Map → Login）
    ValidateToken { token: String, account_id: u32 },

    /// Token验证响应（Login → Map）
    ValidateTokenResponse {
        valid: bool,
        account_id: u32,
        account_level: u32,
    },
}

/// 服务器类型枚举（协议层）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerTypeProto {
    Login,
    Char,
    Map,
}

/// 角色传输状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferStatus {
    Success,
    CharNotFound,
    InvalidToken,
    ServerFull,
    DatabaseError,
    Unknown,
}

impl TransferStatus {
    pub fn as_u8(&self) -> u8 {
        match self {
            TransferStatus::Success => 0,
            TransferStatus::CharNotFound => 1,
            TransferStatus::InvalidToken => 2,
            TransferStatus::ServerFull => 3,
            TransferStatus::DatabaseError => 4,
            TransferStatus::Unknown => 99,
        }
    }

    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => TransferStatus::Success,
            1 => TransferStatus::CharNotFound,
            2 => TransferStatus::InvalidToken,
            3 => TransferStatus::ServerFull,
            4 => TransferStatus::DatabaseError,
            _ => TransferStatus::Unknown,
        }
    }
}

/// Inter-Server 连接器 trait
/// 定义服务器间通信的接口
pub trait InterServerConnector: Send + Sync {
    /// 发送数据包
    fn send_packet(&self, packet: &InterServerPacket) -> Result<(), String>;

    /// 获取连接状态
    fn is_connected(&self) -> bool;

    /// 获取目标服务器ID
    fn target_server_id(&self) -> u32;
}

/// Inter-Server 通信管理器
pub struct InterServerComm {
    /// 到其他服务器的连接
    connections: RwLock<HashMap<u32, Box<dyn InterServerConnector>>>,
}

/// 服务器类型序列化映射
impl From<crate::game::server_registry::ServerType> for ServerTypeProto {
    fn from(t: crate::game::server_registry::ServerType) -> Self {
        match t {
            crate::game::server_registry::ServerType::Login => ServerTypeProto::Login,
            crate::game::server_registry::ServerType::Char => ServerTypeProto::Char,
            crate::game::server_registry::ServerType::Map => ServerTypeProto::Map,
        }
    }
}

impl From<ServerTypeProto> for crate::game::server_registry::ServerType {
    fn from(t: ServerTypeProto) -> Self {
        match t {
            ServerTypeProto::Login => crate::game::server_registry::ServerType::Login,
            ServerTypeProto::Char => crate::game::server_registry::ServerType::Char,
            ServerTypeProto::Map => crate::game::server_registry::ServerType::Map,
        }
    }
}

use parking_lot::RwLock;
use std::collections::HashMap;

impl InterServerComm {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
        }
    }

    /// 添加到服务器的连接
    #[allow(dead_code)]
    pub fn add_connection(&self, server_id: u32, connector: Box<dyn InterServerConnector>) {
        let mut connections = self.connections.write();
        connections.insert(server_id, connector);
    }

    /// 移除服务器连接
    #[allow(dead_code)]
    pub fn remove_connection(&self, server_id: u32) {
        let mut connections = self.connections.write();
        connections.remove(&server_id);
    }

    /// 发送数据包到指定服务器
    #[allow(dead_code)]
    pub fn send_to(&self, server_id: u32, packet: &InterServerPacket) -> Result<(), String> {
        let connections = self.connections.read();
        if let Some(conn) = connections.get(&server_id) {
            conn.send_packet(packet)
        } else {
            Err(format!("No connection to server {}", server_id))
        }
    }

    /// 检查到指定服务器的连接是否正常
    #[allow(dead_code)]
    pub fn is_connected_to(&self, server_id: u32) -> bool {
        let connections = self.connections.read();
        connections
            .get(&server_id)
            .map(|c| c.is_connected())
            .unwrap_or(false)
    }
}

impl Default for InterServerComm {
    fn default() -> Self {
        Self::new()
    }
}

/// 简化的进程内 Inter-Server 通信实现
/// 用于单进程多线程模式的服务器通信
/// 角色传输事件
#[derive(Debug, Clone)]
pub struct CharTransferEvent {
    pub char_id: u32,
    pub account_id: u32,
    pub token: String,
    pub map_server_id: u32,
    pub character_data: CharacterTransfer,
}

/// 角色离开事件
#[derive(Debug, Clone)]
pub struct CharLeaveEvent {
    pub char_id: u32,
    pub map_server_id: u32,
    pub character_data: CharacterTransfer,
}

/// Inter-Server Channel 类型枚举
#[derive(Debug, Clone)]
pub enum InterServerEvent {
    /// 角色传输请求
    CharTransfer(CharTransferEvent),
    /// 角色离开
    CharLeave(CharLeaveEvent),
    /// 心跳
    Heartbeat(u32, u64),
}

/// 进程内 Inter-Server 通信通道（简化版）
/// 使用单一 channel 发送所有类型的事件
#[derive(Clone)]
pub struct InterServerChannel {
    /// 发送端
    tx: std::sync::mpsc::SyncSender<InterServerEvent>,
}

impl InterServerChannel {
    /// 创建一对通信通道
    pub fn channel() -> (
        InterServerChannel,
        std::sync::mpsc::Receiver<InterServerEvent>,
    ) {
        let (tx, rx) = std::sync::mpsc::sync_channel(100);
        (Self { tx }, rx)
    }

    /// 发送角色传输请求
    #[allow(dead_code)]
    pub fn send_char_transfer(&self, event: CharTransferEvent) -> Result<(), String> {
        self.tx
            .send(InterServerEvent::CharTransfer(event))
            .map_err(|e| e.to_string())
    }

    /// 发送角色离开通知
    #[allow(dead_code)]
    pub fn send_char_leave(&self, event: CharLeaveEvent) -> Result<(), String> {
        self.tx
            .send(InterServerEvent::CharLeave(event))
            .map_err(|e| e.to_string())
    }

    /// 发送心跳
    #[allow(dead_code)]
    pub fn send_heartbeat(&self, server_id: u32, timestamp: u64) -> Result<(), String> {
        self.tx
            .send(InterServerEvent::Heartbeat(server_id, timestamp))
            .map_err(|e| e.to_string())
    }

    /// 尝试接收事件（非阻塞）
    #[allow(dead_code)]
    pub fn try_recv(rx: &std::sync::mpsc::Receiver<InterServerEvent>) -> Option<InterServerEvent> {
        rx.try_recv().ok()
    }

    /// 接收事件
    #[allow(dead_code)]
    pub fn recv(
        rx: &std::sync::mpsc::Receiver<InterServerEvent>,
    ) -> Result<InterServerEvent, String> {
        rx.recv().map_err(|e| e.to_string())
    }
}

impl Default for InterServerChannel {
    fn default() -> Self {
        let (tx, _) = std::sync::mpsc::sync_channel(1);
        Self { tx }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_transfer_serialization() {
        let transfer = CharacterTransfer {
            char_id: 1001,
            account_id: 1,
            name: "TestChar".to_string(),
            level: 99,
            job: 0,
            hp: 5000,
            max_hp: 5000,
            sp: 1000,
            max_sp: 1000,
            map_name: "prontera".to_string(),
            pos_x: 100,
            pos_y: 200,
            save_map: "new_1-1".to_string(),
            save_x: 53,
            save_y: 111,
            str: 99,
            agi: 99,
            vit: 99,
            int: 99,
            dex: 99,
            luk: 99,
            zeny: 1000000,
            sex: 1,
            hair_color: 0,
            hair: 1,
            cloak_id: 0,
            boots_id: 0,
            account_level: 0,
        };

        // 测试序列化/反序列化
        let json = serde_json::to_string(&transfer).unwrap();
        let restored: CharacterTransfer = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.char_id, 1001);
        assert_eq!(restored.name, "TestChar");
        assert_eq!(restored.level, 99);
    }

    #[test]
    fn transfer_status_conversion() {
        assert_eq!(TransferStatus::Success.as_u8(), 0);
        assert_eq!(TransferStatus::from_u8(0), TransferStatus::Success);
        assert_eq!(TransferStatus::from_u8(99), TransferStatus::Unknown);
    }

    #[test]
    fn inter_server_channel_char_transfer() {
        let (tx, rx) = InterServerChannel::channel();

        let event = CharTransferEvent {
            char_id: 1001,
            account_id: 1,
            token: "test_token_123".to_string(),
            map_server_id: 10,
            character_data: CharacterTransfer {
                char_id: 1001,
                account_id: 1,
                name: "TestChar".to_string(),
                level: 50,
                job: 0,
                hp: 3000,
                max_hp: 3000,
                sp: 500,
                max_sp: 500,
                map_name: "geffen".to_string(),
                pos_x: 50,
                pos_y: 100,
                save_map: "new_1-1".to_string(),
                save_x: 53,
                save_y: 111,
                str: 50,
                agi: 50,
                vit: 50,
                int: 50,
                dex: 50,
                luk: 50,
                zeny: 100000,
                sex: 1,
                hair_color: 0,
                hair: 1,
                cloak_id: 0,
                boots_id: 0,
                account_level: 0,
            },
        };

        tx.send_char_transfer(event.clone()).unwrap();

        let received = InterServerChannel::recv(&rx).unwrap();
        match received {
            InterServerEvent::CharTransfer(e) => {
                assert_eq!(e.char_id, 1001);
                assert_eq!(e.map_server_id, 10);
            }
            _ => panic!("Expected CharTransfer event"),
        }
    }

    #[test]
    fn inter_server_channel_char_leave() {
        let (tx, rx) = InterServerChannel::channel();

        let event = CharLeaveEvent {
            char_id: 1001,
            map_server_id: 10,
            character_data: CharacterTransfer {
                char_id: 1001,
                account_id: 1,
                name: "TestChar".to_string(),
                level: 50,
                job: 0,
                hp: 3000,
                max_hp: 3000,
                sp: 500,
                max_sp: 500,
                map_name: "geffen".to_string(),
                pos_x: 50,
                pos_y: 100,
                save_map: "new_1-1".to_string(),
                save_x: 53,
                save_y: 111,
                str: 50,
                agi: 50,
                vit: 50,
                int: 50,
                dex: 50,
                luk: 50,
                zeny: 100000,
                sex: 1,
                hair_color: 0,
                hair: 1,
                cloak_id: 0,
                boots_id: 0,
                account_level: 0,
            },
        };

        tx.send_char_leave(event.clone()).unwrap();

        let received = InterServerChannel::recv(&rx).unwrap();
        match received {
            InterServerEvent::CharLeave(e) => {
                assert_eq!(e.char_id, 1001);
            }
            _ => panic!("Expected CharLeave event"),
        }
    }

    #[test]
    fn inter_server_channel_heartbeat() {
        let (tx, rx) = InterServerChannel::channel();

        tx.send_heartbeat(1, 1234567890).unwrap();

        let received = InterServerChannel::recv(&rx).unwrap();
        match received {
            InterServerEvent::Heartbeat(server_id, timestamp) => {
                assert_eq!(server_id, 1);
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Expected Heartbeat event"),
        }
    }
}
