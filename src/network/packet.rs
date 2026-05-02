use serde::{Deserialize, Serialize};

/// 数据包 ID
pub type PacketId = u16;

/// 数据包头部
#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PacketHeader {
    pub length: u16,
    pub packet_id: u16,
}

/// 数据包
#[derive(Debug, Clone)]
pub struct Packet {
    pub header: PacketHeader,
    pub data: Vec<u8>,
}

impl Packet {
    pub fn new(packet_id: PacketId, data: Vec<u8>) -> Self {
        let length = (data.len() + 4) as u16; // 4 = header size
        Self {
            header: PacketHeader { length, packet_id },
            data,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.header.length as usize);
        bytes.extend_from_slice(&self.header.length.to_le_bytes());
        bytes.extend_from_slice(&self.header.packet_id.to_le_bytes());
        bytes.extend_from_slice(&self.data);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }

        let length = u16::from_le_bytes([bytes[0], bytes[1]]);
        let packet_id = u16::from_le_bytes([bytes[2], bytes[3]]);

        if bytes.len() < length as usize {
            return None;
        }

        let data = bytes[4..length as usize].to_vec();

        Some(Self {
            header: PacketHeader { length, packet_id },
            data,
        })
    }
}

/// 常用数据包 ID 定义
pub mod id {
    use super::PacketId;

    // 登录服务器包
    pub const PACKET_SC_NOTIFY_BAN: PacketId = 0x0081;
    pub const PACKET_AC_ACCEPT_LOGIN: PacketId = 0x0069;
    pub const PACKET_AC_REFUSE_LOGIN: PacketId = 0x006A;

    // 字符服务器包
    pub const PACKET_CA_LOGIN: PacketId = 0x0064;
    pub const PACKET_CH_ENTER: PacketId = 0x0065;
    pub const PACKET_CS_UPDATE_NEXTCHARPOS: PacketId = 0x02D1;

    // 地图服务器包
    pub const PACKET_CZ_ENTER: PacketId = 0x007C;
    pub const PACKET_ZC_ACCEPT_ENTER: PacketId = 0x02D3;
    pub const PACKET_ZC_NOTIFY_ACT: PacketId = 0x02D5;
    pub const PACKET_CZ_REQUEST_MOVE: PacketId = 0x0085;
    pub const PACKET_ZC_MOVE: PacketId = 0x0086;
    pub const PACKET_CZ_USE_SKILL: PacketId = 0x0112;

    // 仓库相关
    pub const CZ_REQ_STORAGE_OPEN: PacketId = 0x0213;
    pub const CZ_REQ_STORAGE_CLOSE: PacketId = 0x0214;
    pub const CZ_REQ_STORAGE_MOVE_ITEM: PacketId = 0x0215;
    pub const ZC_STORAGE_OPEN: PacketId = 0x01F3;
    pub const ZC_STORAGE_CLOSE: PacketId = 0x01F4;
    pub const ZC_STORAGE_ITEMS: PacketId = 0x01F5;
    pub const ZC_STORAGE_ITEM_ADD: PacketId = 0x01F6;
    pub const ZC_STORAGE_ITEM_REMOVE: PacketId = 0x01F7;
}
