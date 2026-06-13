//! 公会系统数据包协议

use super::packet_builder::{Packed, PacketBuilderCtx, parse_fixed_string};

const GUILD_NAME_LENGTH: usize = 24;
const MEMBER_NAME_LENGTH: usize = 24;
const NOTICE_LENGTH: usize = 120;
const MES_LENGTH: usize = 40;

// ========== Client -> Server ==========

/// 客户端创建公会 (0x0165)
#[derive(Debug, Clone)]
pub struct CZGuildCreate {
    pub name: String,
}

impl Packed for CZGuildCreate {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x0165)
            .put_fixed_str(&self.name, GUILD_NAME_LENGTH)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        let mut offset = 0;
        let name = parse_fixed_string(slice, &mut offset, GUILD_NAME_LENGTH)?;
        Some(Self { name })
    }
}

/// 客户端邀请加入公会 (0x0168)
#[derive(Debug, Clone)]
pub struct CZGuildInvite {
    pub target_name: String,
}

impl Packed for CZGuildInvite {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x0168)
            .put_fixed_str(&self.target_name, MEMBER_NAME_LENGTH)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        let mut offset = 0;
        let target_name = parse_fixed_string(slice, &mut offset, MEMBER_NAME_LENGTH)?;
        Some(Self { target_name })
    }
}

/// 客户端回应公会邀请 (0x0169)
#[derive(Debug, Clone)]
pub struct CZGuildJoin {
    pub guild_id: u32,
    pub accept: bool,
}

impl Packed for CZGuildJoin {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x0169)
            .put_u32(self.guild_id)
            .put_u8(if self.accept { 1 } else { 0 })
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 5 {
            return None;
        }
        let guild_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        let accept = slice[4] != 0;
        Some(Self { guild_id, accept })
    }
}

/// 客户端离开公会 (0x016B)
#[derive(Debug, Clone)]
pub struct CZGuildLeave;

impl Packed for CZGuildLeave {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x016B).build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 客户端踢出成员 (0x016C)
#[derive(Debug, Clone)]
pub struct CZGuildExpel {
    pub target_name: String,
    pub reason: String,
}

impl Packed for CZGuildExpel {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x016C)
            .put_fixed_str(&self.target_name, MEMBER_NAME_LENGTH)
            .put_fixed_str(&self.reason, MES_LENGTH)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        let mut offset = 0;
        let target_name = parse_fixed_string(slice, &mut offset, MEMBER_NAME_LENGTH)?;
        let reason = parse_fixed_string(slice, &mut offset, MES_LENGTH)?;
        Some(Self {
            target_name,
            reason,
        })
    }
}

/// 客户端修改公会公告 (0x0183)
#[derive(Debug, Clone)]
pub struct CZGuildChangeNotice {
    pub notice: String,
}

impl Packed for CZGuildChangeNotice {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x0183)
            .put_fixed_str(&self.notice, NOTICE_LENGTH)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        let mut offset = 0;
        let notice = parse_fixed_string(slice, &mut offset, NOTICE_LENGTH)?;
        Some(Self { notice })
    }
}

/// 客户端请求公会信息 (0x01B7)
#[derive(Debug, Clone)]
pub struct CZGuildRequestInfo {
    pub guild_id: u32,
}

impl Packed for CZGuildRequestInfo {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x01B7).put_u32(self.guild_id).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        let guild_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        Some(Self { guild_id })
    }
}

/// 客户端请求成员信息 (0x01B8)
#[derive(Debug, Clone)]
pub struct CZGuildRequestMemberInfo {
    pub guild_id: u32,
}

impl Packed for CZGuildRequestMemberInfo {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x01B8).put_u32(self.guild_id).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        let guild_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        Some(Self { guild_id })
    }
}

/// 客户端请求职位信息 (0x01B9)
#[derive(Debug, Clone)]
pub struct CZGuildRequestPosInfo {
    pub guild_id: u32,
}

impl Packed for CZGuildRequestPosInfo {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x01B9).put_u32(self.guild_id).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        let guild_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        Some(Self { guild_id })
    }
}

/// 客户端公会聊天 (0x01EC)
#[derive(Debug, Clone)]
pub struct CZGuildChat {
    pub message: String,
}

impl Packed for CZGuildChat {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x01EC).put_str(&self.message).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        let mut offset = 0;
        let message = super::packet_builder::parse_string(slice, &mut offset)?;
        Some(Self { message })
    }
}

// ========== Server -> Client ==========

/// 公会创建结果 (0x014C)
#[derive(Debug, Clone)]
pub struct ZCGuildCreated {
    pub result: u8, // 0 = 成功, 1 = 名称已存在, 2 = 其他错误
    pub guild_id: u32,
}

impl Packed for ZCGuildCreated {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x014C)
            .put_u8(self.result)
            .put_u32(self.guild_id)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None // 服务器包不需要解析
    }
}

/// 公会信息 (0x014D)
#[derive(Debug, Clone)]
pub struct ZCGuildInfo {
    pub guild_id: u32,
    pub level: u8,
    pub member_count: u32,
    pub max_members: u32,
    pub average_level: u16,
    pub exp: u64,
    pub max_exp: u64,
    pub notice: String,
}

impl Packed for ZCGuildInfo {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x014D)
            .put_u32(self.guild_id)
            .put_u8(self.level)
            .put_u32(self.member_count)
            .put_u32(self.max_members)
            .put_u16(self.average_level)
            .put_u64(self.exp)
            .put_u64(self.max_exp)
            .put_fixed_str(&self.notice, NOTICE_LENGTH)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 成员信息条目
#[derive(Debug, Clone)]
pub struct GuildMemberInfo {
    pub position_id: u8,
    pub name: String,
    pub level: u16,
    pub job: u16,
    pub online: bool,
}

/// 公会成员信息 (0x014E)
#[derive(Debug, Clone)]
pub struct ZCGuildMemberInfo {
    pub member_count: u16,
    pub members: Vec<GuildMemberInfo>,
}

impl Packed for ZCGuildMemberInfo {
    fn to_packet(&self) -> Vec<u8> {
        let mut builder = PacketBuilderCtx::new(0x014E);
        builder = builder.put_u16(self.member_count);
        for member in &self.members {
            builder = builder
                .put_u8(member.position_id)
                .put_fixed_str(&member.name, MEMBER_NAME_LENGTH)
                .put_u16(member.level)
                .put_u16(member.job)
                .put_u8(if member.online { 1 } else { 0 });
        }
        builder.build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 公会邀请通知 (0x0150)
#[derive(Debug, Clone)]
pub struct ZCGuildInvite {
    pub guild_id: u32,
    pub guild_name: String,
    pub inviter_name: String,
}

impl Packed for ZCGuildInvite {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x0150)
            .put_u32(self.guild_id)
            .put_fixed_str(&self.guild_name, GUILD_NAME_LENGTH)
            .put_fixed_str(&self.inviter_name, MEMBER_NAME_LENGTH)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 离开公会结果 (0x0154)
#[derive(Debug, Clone)]
pub struct ZCGuildLeaveResult {
    pub result: u8, // 0 = 成功, 1 = 失败
}

impl Packed for ZCGuildLeaveResult {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x0154).put_u8(self.result).build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 踢出结果 (0x015A)
#[derive(Debug, Clone)]
pub struct ZCGuildExpelResult {
    pub result: u8,
    pub target_name: String,
    pub reason: String,
}

impl Packed for ZCGuildExpelResult {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x015A)
            .put_u8(self.result)
            .put_fixed_str(&self.target_name, MEMBER_NAME_LENGTH)
            .put_fixed_str(&self.reason, MES_LENGTH)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 公会职位信息 (0x0162)
#[derive(Debug, Clone)]
pub struct GuildPositionInfo {
    pub position_id: u32,
    pub name: String,
    pub mode: u32,
}

#[derive(Debug, Clone)]
pub struct ZCGuildPositionInfo {
    pub position_count: u16,
    pub positions: Vec<GuildPositionInfo>,
}

impl Packed for ZCGuildPositionInfo {
    fn to_packet(&self) -> Vec<u8> {
        let mut builder = PacketBuilderCtx::new(0x0162);
        builder = builder.put_u16(self.position_count);
        for pos in &self.positions {
            builder = builder
                .put_u32(pos.position_id)
                .put_fixed_str(&pos.name, GUILD_NAME_LENGTH)
                .put_u32(pos.mode);
        }
        builder.build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 公会公告 (0x017F)
#[derive(Debug, Clone)]
pub struct ZCGuildNotice {
    pub notice: String,
}

impl Packed for ZCGuildNotice {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x017F)
            .put_fixed_str(&self.notice, NOTICE_LENGTH)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 公会聊天 (0x01EC)
#[derive(Debug, Clone)]
pub struct ZCGuildChat {
    pub sender_name: String,
    pub message: String,
}

impl Packed for ZCGuildChat {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x01EC)
            .put_fixed_str(&self.sender_name, MEMBER_NAME_LENGTH)
            .put_str(&self.message)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cz_guild_create_roundtrip() {
        let pkt = CZGuildCreate {
            name: "TestGuild".to_string(),
        };
        let bytes = pkt.to_packet();
        let parsed = CZGuildCreate::from_slice(&bytes[4..]).unwrap();
        assert_eq!(parsed.name, "TestGuild");
    }

    #[test]
    fn test_cz_guild_invite_roundtrip() {
        let pkt = CZGuildInvite {
            target_name: "TestPlayer".to_string(),
        };
        let bytes = pkt.to_packet();
        let parsed = CZGuildInvite::from_slice(&bytes[4..]).unwrap();
        assert_eq!(parsed.target_name, "TestPlayer");
    }

    #[test]
    fn test_cz_guild_join_accept() {
        let data = vec![1, 0, 0, 0, 1];
        let pkt = CZGuildJoin::from_slice(&data).unwrap();
        assert_eq!(pkt.guild_id, 1);
        assert!(pkt.accept);
    }

    #[test]
    fn test_cz_guild_join_decline() {
        let data = vec![1, 0, 0, 0, 0];
        let pkt = CZGuildJoin::from_slice(&data).unwrap();
        assert_eq!(pkt.guild_id, 1);
        assert!(!pkt.accept);
    }

    #[test]
    fn test_zc_guild_created_packet_id() {
        let pkt = ZCGuildCreated {
            result: 0,
            guild_id: 42,
        };
        let bytes = pkt.to_packet();
        let packet_id = u16::from_le_bytes([bytes[2], bytes[3]]);
        assert_eq!(packet_id, 0x014C);
    }

    #[test]
    fn test_zc_guild_info_packet() {
        let pkt = ZCGuildInfo {
            guild_id: 1,
            level: 5,
            member_count: 10,
            max_members: 24,
            average_level: 50,
            exp: 5000,
            max_exp: 25000,
            notice: "Welcome!".to_string(),
        };
        let bytes = pkt.to_packet();
        let packet_id = u16::from_le_bytes([bytes[2], bytes[3]]);
        assert_eq!(packet_id, 0x014D);
    }
}
