use super::packet_builder::{Packed, PacketBuilderCtx};

// ========== Client -> Server ==========

/// 客户端请求成就奖励 (0x0A24)
#[derive(Debug, Clone)]
pub struct CzAchievementCheckReward {
    /// 成就ID
    pub achievement_id: u32,
}

impl CzAchievementCheckReward {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let achievement_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        Some(Self { achievement_id })
    }
}

// ========== Server -> Client ==========

/// 服务器发送成就列表 (0x0A25)
#[derive(Debug, Clone)]
pub struct ZcAchievementList {
    /// 成就数量
    pub count: u32,
}

impl Packed for ZcAchievementList {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x0A25)
            .put_u32(self.count)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 服务器通知更新成就 (0x0A26)
#[derive(Debug, Clone)]
pub struct ZcAchievementUpdate {
    /// 成就ID
    pub achievement_id: u32,
    /// 是否完成：0=未完成, 1=完成
    pub completed: u8,
}

impl Packed for ZcAchievementUpdate {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x0A26)
            .put_u32(self.achievement_id)
            .put_u8(self.completed)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}
