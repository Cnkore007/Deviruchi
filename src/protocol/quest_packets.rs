use super::packet_builder::{Packed, PacketBuilderCtx};

// ========== Client -> Server ==========

/// 客户端请求任务状态 (0x02B5)
#[derive(Debug, Clone)]
pub struct CzQuestStateAck {
    /// 任务ID
    pub quest_id: u32,
    /// 状态：0=进行中, 1=完成
    pub state: u8,
}

impl CzQuestStateAck {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 5 {
            return None;
        }
        let quest_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let state = data[4];
        Some(Self { quest_id, state })
    }
}

// ========== Server -> Client ==========

/// 服务器发送任务列表 (0x02B1)
#[derive(Debug, Clone)]
pub struct ZcQuestList {
    /// 任务数量
    pub count: u32,
}

impl Packed for ZcQuestList {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x02B1)
            .put_u32(self.count)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 服务器通知添加任务 (0x02B3)
#[derive(Debug, Clone)]
pub struct ZcQuestAdd {
    /// 任务ID
    pub quest_id: u32,
    /// 状态：0=进行中, 1=完成
    pub state: u8,
}

impl Packed for ZcQuestAdd {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x02B3)
            .put_u32(self.quest_id)
            .put_u8(self.state)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 服务器通知更新任务 (0x02B4)
#[derive(Debug, Clone)]
pub struct ZcQuestUpdate {
    /// 任务ID
    pub quest_id: u32,
    /// 状态：0=进行中, 1=完成
    pub state: u8,
}

impl Packed for ZcQuestUpdate {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x02B4)
            .put_u32(self.quest_id)
            .put_u8(self.state)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 服务器通知删除任务 (0x02B2)
#[derive(Debug, Clone)]
pub struct ZcQuestDelete {
    /// 任务ID
    pub quest_id: u32,
}

impl Packed for ZcQuestDelete {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x02B2)
            .put_u32(self.quest_id)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}
