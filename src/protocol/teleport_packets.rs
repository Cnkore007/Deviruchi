//! 传送系统数据包协议
//!
//! 包含玩家回城、GM传送、地图切换等相关协议

use super::packet_builder::{PacketBuilder, Packed, parse_fixed_string, parse_string};

const NAME_LENGTH: usize = 24;
const MAP_NAME_LENGTH: usize = 16;

/// 客户端请求使用回城 (0x0119)
/// 玩家使用蝴蝶翅膀/回城卷轴等道具回到存储点
#[derive(Debug, Clone)]
pub struct CZUseReturn;

impl Packed for CZUseReturn {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0119).build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 服务器确认传送请求 (0x0088)
/// 通知客户端即将进行地图传送
#[derive(Debug, Clone)]
pub struct ZCWarpAck {
    pub warp_type: u8, // 1 = 传送门, 2 = 回城, 3 = GM传送
}

impl Packed for ZCWarpAck {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0088)
            .put_u8(self.warp_type)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 1 {
            return None;
        }
        Some(Self { warp_type: slice[0] })
    }
}

/// 服务器通知客户端切换到新地图 (0x0091)
/// 包含新地图的名称和初始坐标
#[derive(Debug, Clone)]
pub struct ZCChangeMap {
    pub map_name: String,
    pub x: u16,
    pub y: u16,
}

impl Packed for ZCChangeMap {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0091)
            .put_fixed_str(&self.map_name, MAP_NAME_LENGTH)
            .put_u16(self.x)
            .put_u16(self.y)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None // 服务器包不需要解析
    }
}

/// GM命令：传送到指定坐标 (0x0138)
/// @warp <map_name> <x> <y>
#[derive(Debug, Clone)]
pub struct CZGmWarp {
    pub map_name: String,
    pub x: u16,
    pub y: u16,
}

impl Packed for CZGmWarp {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0138)
            .put_fixed_str(&self.map_name, MAP_NAME_LENGTH)
            .put_u16(self.x)
            .put_u16(self.y)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < MAP_NAME_LENGTH + 4 {
            return None;
        }
        let mut offset = 0;
        let map_name = parse_fixed_string(slice, &mut offset, MAP_NAME_LENGTH)?;
        let x = u16::from_be_bytes([slice[offset], slice[offset + 1]]);
        let y = u16::from_be_bytes([slice[offset + 2], slice[offset + 3]]);
        Some(Self { map_name, x, y })
    }
}

/// GM命令：传送到指定玩家 (0x013A)
/// @goto <player_name>
#[derive(Debug, Clone)]
pub struct CZGmGoto {
    pub target_name: String,
}

impl Packed for CZGmGoto {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x013A)
            .put_fixed_str(&self.target_name, NAME_LENGTH)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < NAME_LENGTH {
            return None;
        }
        let mut offset = 0;
        let target_name = parse_fixed_string(slice, &mut offset, NAME_LENGTH)?;
        Some(Self { target_name })
    }
}

/// GM命令：召唤指定玩家到当前位置 (0x013B)
/// @summon <player_name>
#[derive(Debug, Clone)]
pub struct CZGmSummon {
    pub target_name: String,
}

impl Packed for CZGmSummon {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x013B)
            .put_fixed_str(&self.target_name, NAME_LENGTH)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < NAME_LENGTH {
            return None;
        }
        let mut offset = 0;
        let target_name = parse_fixed_string(slice, &mut offset, NAME_LENGTH)?;
        Some(Self { target_name })
    }
}

/// GM命令：设置存储点 (0x013C)
/// @savepoint - 将当前位置设为存储点
#[derive(Debug, Clone)]
pub struct CZGmSavePoint;

impl Packed for CZGmSavePoint {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x013C).build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 传送错误响应 (0x0084)
/// 当传送失败时返回错误码
#[derive(Debug, Clone)]
pub struct ZCWarpError {
    pub error_code: u8,
}

impl ZCWarpError {
    /// 冷却中
    pub const COOLDOWN: u8 = 1;
    /// 目标地图不存在
    pub const INVALID_MAP: u8 = 2;
    /// 坐标无效
    pub const INVALID_COORDS: u8 = 3;
    /// 目标玩家不存在
    pub const TARGET_NOT_FOUND: u8 = 4;
    /// 权限不足
    pub const NO_PERMISSION: u8 = 5;
}

impl Packed for ZCWarpError {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0084)
            .put_u8(self.error_code)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None // 服务器包不需要解析
    }
}

/// 客户端请求记录存储点 (0x01B8)
/// 与NPC对话记录存储点或特殊道具
#[derive(Debug, Clone)]
pub struct CZSetSavePoint;

impl Packed for CZSetSavePoint {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x01B8).build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 服务器通知存储点已设置 (0x01B9)
#[derive(Debug, Clone)]
pub struct ZCSavePointSet {
    pub map_name: String,
    pub x: u16,
    pub y: u16,
}

impl Packed for ZCSavePointSet {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x01B9)
            .put_fixed_str(&self.map_name, MAP_NAME_LENGTH)
            .put_u16(self.x)
            .put_u16(self.y)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None // 服务器包不需要解析
    }
}

/// 服务器通知玩家正在进行传送 (0x0840)
/// 用于显示传送动画
#[derive(Debug, Clone)]
pub struct ZCWarpStart {
    pub warp_type: u8,
}

impl Packed for ZCWarpStart {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0840)
            .put_u8(self.warp_type)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None // 服务器包不需要解析
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cz_use_return_packet_id() {
        let pkt = CZUseReturn;
        let bytes = pkt.to_packet();
        let packet_id = u16::from_le_bytes([bytes[2], bytes[3]]);
        assert_eq!(packet_id, 0x0119);
    }

    #[test]
    fn test_cz_gm_warp_roundtrip() {
        let pkt = CZGmWarp {
            map_name: "prontera".to_string(),
            x: 150,
            y: 180,
        };
        let bytes = pkt.to_packet();
        let parsed = CZGmWarp::from_slice(&bytes[4..]).unwrap();
        assert_eq!(parsed.map_name, "prontera");
        assert_eq!(parsed.x, 150);
        assert_eq!(parsed.y, 180);
    }

    #[test]
    fn test_cz_gm_goto_roundtrip() {
        let pkt = CZGmGoto {
            target_name: "TestPlayer".to_string(),
        };
        let bytes = pkt.to_packet();
        let parsed = CZGmGoto::from_slice(&bytes[4..]).unwrap();
        assert_eq!(parsed.target_name, "TestPlayer");
    }

    #[test]
    fn test_cz_gm_summon_roundtrip() {
        let pkt = CZGmSummon {
            target_name: "TargetPlayer".to_string(),
        };
        let bytes = pkt.to_packet();
        let parsed = CZGmSummon::from_slice(&bytes[4..]).unwrap();
        assert_eq!(parsed.target_name, "TargetPlayer");
    }

    #[test]
    fn test_zc_warp_ack_packet_id() {
        let pkt = ZCWarpAck { warp_type: 2 };
        let bytes = pkt.to_packet();
        let packet_id = u16::from_le_bytes([bytes[2], bytes[3]]);
        assert_eq!(packet_id, 0x0088);
    }

    #[test]
    fn test_zc_change_map_packet_id() {
        let pkt = ZCChangeMap {
            map_name: "prontera".to_string(),
            x: 100,
            y: 200,
        };
        let bytes = pkt.to_packet();
        let packet_id = u16::from_le_bytes([bytes[2], bytes[3]]);
        assert_eq!(packet_id, 0x0091);
    }

    #[test]
    fn test_zc_warp_error_codes() {
        assert_eq!(ZCWarpError::COOLDOWN, 1);
        assert_eq!(ZCWarpError::INVALID_MAP, 2);
        assert_eq!(ZCWarpError::INVALID_COORDS, 3);
        assert_eq!(ZCWarpError::TARGET_NOT_FOUND, 4);
        assert_eq!(ZCWarpError::NO_PERMISSION, 5);
    }
}
