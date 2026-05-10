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
    pub fn new(packet_id: PacketId, data: Vec<u8>) -> Option<Self> {
        let total = data.len() + 4; // 4 = header size
        if total > u16::MAX as usize {
            return None;
        }
        let length = total as u16;
        Some(Self {
            header: PacketHeader { length, packet_id },
            data,
        })
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

        if length < 4 {
            return None;
        }

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
    pub const PACKET_CZ_REQUEST_TIME: PacketId = 0x00A7;
    pub const PACKET_ZC_ACK_TIME: PacketId = 0x007F;
    pub const PACKET_CZ_REQUEST_QUIT: PacketId = 0x00F3;
    pub const PACKET_ZC_ACK_REQ_DISCONNECT: PacketId = 0x018A;
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

    // 交易相关
    pub const CZ_TRADE_REQUEST: PacketId = 0x00E4;
    pub const CZ_TRADE_ACK: PacketId = 0x00E6;
    pub const CZ_TRADE_ADD_ITEM: PacketId = 0x00B0;
    pub const CZ_TRADE_ADD_ZENY: PacketId = 0x00B1;
    pub const CZ_TRADE_LOCK: PacketId = 0x00EF;

    // 方向改变
    pub const CZ_REQUEST_CHANGE_DIRECTION: PacketId = 0x00D9;

    // 私聊/密语
    pub const CZ_WHISPER: PacketId = 0x00F7;
    pub const ZC_WHISPER: PacketId = 0x0097;
    pub const ZC_ACK_WHISPER: PacketId = 0x0098;

    // 状态点分配
    pub const CZ_STATUS_CHANGE: PacketId = 0x014D;
    pub const ZC_STATUS_CHANGE_ACK: PacketId = 0x00BC;

    // 技能点分配
    pub const CZ_SKILL_UP: PacketId = 0x010B;
    pub const ZC_SKILLINFO_UPDATE: PacketId = 0x010E;

    // 转职相关
    pub const CZ_REQ_CHANGEJOB: PacketId = 0x019D;
    pub const ZC_ACK_CHANGEJOB: PacketId = 0x019E;
    pub const ZC_TRADE_REQUEST: PacketId = 0x00E5;
    pub const ZC_TRADE_ACK: PacketId = 0x00E7;
    pub const ZC_TRADE_ADD_ITEM: PacketId = 0x00E8;
    pub const ZC_TRADE_ADD_ZENY: PacketId = 0x00E9;
    pub const ZC_TRADE_LOCK: PacketId = 0x00EC;
    pub const ZC_TRADE_COMMIT: PacketId = 0x00F0;
    pub const ZC_TRADE_CANCEL: PacketId = 0x00F1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_new_rejects_oversized_data() {
        let big_data = vec![0u8; 65533]; // 65533 + 4 = 65537 > u16::MAX
        let packet = Packet::new(0x0064, big_data);
        // After fix, should return None for oversized packets
        assert!(packet.is_none());
    }

    #[test]
    fn test_packet_from_bytes_rejects_length_lt_4() {
        // Malicious packet with length field = 2
        let mut bytes = vec![2, 0, 0x64, 0x00]; // length=2, packet_id=0x0064
        bytes.extend_from_slice(&[0, 0]);
        let result = Packet::from_bytes(&bytes);
        assert!(result.is_none());
    }

    #[test]
    fn test_packet_new_accepts_normal_data() {
        let data = vec![0u8; 100];
        let packet = Packet::new(0x0064, data);
        assert!(packet.is_some());
        assert_eq!(packet.unwrap().header.length, 104);
    }

    #[test]
    fn test_packet_from_bytes_normal() {
        let mut bytes = vec![8, 0, 0x64, 0x00]; // length=8, packet_id=0x0064
        bytes.extend_from_slice(&[1, 2, 3, 4]); // 4 bytes data
        let result = Packet::from_bytes(&bytes);
        assert!(result.is_some());
        let packet = result.unwrap();
        assert_eq!(packet.data, vec![1, 2, 3, 4]);
    }
}
