use super::packet_builder::{Packed, PacketBuilderCtx};

const NAME_LENGTH: usize = 24;

// ========== Client -> Server ==========

/// 客户端请求添加好友 (0x0201)
#[derive(Debug, Clone)]
pub struct CzFriendsListAdd {
    /// 好友角色ID
    pub char_id: u32,
}

impl CzFriendsListAdd {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let char_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        Some(Self { char_id })
    }
}

/// 客户端请求删除好友 (0x0203)
#[derive(Debug, Clone)]
pub struct CzFriendsListRemove {
    /// 好友角色ID
    pub char_id: u32,
}

impl CzFriendsListRemove {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let char_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        Some(Self { char_id })
    }
}

/// 客户端回复好友请求 (0x0208)
#[derive(Debug, Clone)]
pub struct CzFriendsListReply {
    /// 请求者角色ID
    pub char_id: u32,
    /// 回复：0=拒绝, 1=接受
    pub reply: u8,
}

impl CzFriendsListReply {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 5 {
            return None;
        }
        let char_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let reply = data[4];
        Some(Self { char_id, reply })
    }
}

// ========== Server -> Client ==========

/// 服务器发送好友列表 (0x0201)
#[derive(Debug, Clone)]
pub struct ZcFriendsList {
    /// 好友数量
    pub count: u8,
}

impl Packed for ZcFriendsList {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x0201)
            .put_u8(self.count)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 服务器发送好友请求 (0x0207)
#[derive(Debug, Clone)]
pub struct ZcFriendRequest {
    /// 请求者角色ID
    pub char_id: u32,
    /// 请求者名称
    pub name: String,
}

impl Packed for ZcFriendRequest {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x0207)
            .put_u32(self.char_id)
            .put_fixed_str(&self.name, NAME_LENGTH)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 服务器响应添加好友结果 (0x0209)
#[derive(Debug, Clone)]
pub struct ZcFriendAddAck {
    /// 好友角色ID
    pub char_id: u32,
    /// 结果：0=成功, 1=失败
    pub result: u8,
}

impl Packed for ZcFriendAddAck {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x0209)
            .put_u32(self.char_id)
            .put_u8(self.result)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}
