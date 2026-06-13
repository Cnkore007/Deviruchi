use crate::protocol::packet_builder::PacketBuilderCtx;

// ========== Client -> Server ==========

/// 请求交易 (0x00E4)
pub struct CZTradeRequest {
    pub target_account_id: u32,
}

impl CZTradeRequest {
    pub fn from_packet(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        Some(Self {
            target_account_id: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
        })
    }
}

/// 接受/拒绝交易 (0x00E6)
pub struct CZTradeAck {
    pub accept: bool,
}

impl CZTradeAck {
    pub fn from_packet(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        Some(Self {
            accept: data[0] != 0,
        })
    }
}

/// 添加物品到交易 (0x00B0)
pub struct CZTradeAddItem {
    pub inventory_index: u16,
    pub amount: u32,
}

impl CZTradeAddItem {
    pub fn from_packet(data: &[u8]) -> Option<Self> {
        if data.len() < 6 {
            return None;
        }
        Some(Self {
            inventory_index: u16::from_le_bytes([data[0], data[1]]),
            amount: u32::from_le_bytes([data[2], data[3], data[4], data[5]]),
        })
    }
}

/// 添加 Zeny (0x00B1)
pub struct CZTradeAddZeny {
    pub amount: u32,
}

impl CZTradeAddZeny {
    pub fn from_packet(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        Some(Self {
            amount: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
        })
    }
}

/// 锁定交易 (0x00EF)
pub struct CZTradeLock;

impl CZTradeLock {
    pub fn from_packet(_data: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

// ========== Server -> Client ==========

/// 交易请求通知 (0x00E5)
pub struct ZCTradeRequest {
    pub requester_id: u32,
    pub requester_name: String,
}

impl ZCTradeRequest {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x00E5)
            .put_u32(self.requester_id)
            .put_fixed_str(&self.requester_name, 24)
            .build()
    }
}

/// 交易接受确认 (0x00E7)
pub struct ZCTradeAck {
    pub accept: bool,
}

impl ZCTradeAck {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x00E7)
            .put_u8(if self.accept { 1 } else { 0 })
            .build()
    }
}

/// 对方添加物品通知 (0x00E8)
pub struct ZCTradeAddItem {
    pub amount: u32,
    pub item_id: u16,
    pub identified: bool,
    pub damaged: bool,
    pub refine: u8,
    pub cards: [u16; 4],
}

impl ZCTradeAddItem {
    pub fn to_packet(&self) -> Vec<u8> {
        let mut builder = PacketBuilderCtx::new(0x00E8)
            .put_u32(self.amount)
            .put_u16(self.item_id)
            .put_u8(if self.identified { 1 } else { 0 })
            .put_u8(if self.damaged { 1 } else { 0 })
            .put_u8(self.refine);
        for card in &self.cards {
            builder = builder.put_u16(*card);
        }
        builder.build()
    }
}

/// 对方添加 Zeny 通知 (0x00E9)
pub struct ZCTradeAddZeny {
    pub amount: u32,
}

impl ZCTradeAddZeny {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x00E9).put_u32(self.amount).build()
    }
}

/// 对方锁定通知 (0x00EC)
pub struct ZCTradeLock;

impl ZCTradeLock {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x00EC).build()
    }
}

/// 交易成功 (0x00F0)
pub struct ZCTradeCommit;

impl ZCTradeCommit {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x00F0).build()
    }
}

/// 交易取消 (0x00F1)
pub struct ZCTradeCancel {
    pub reason: u8, // 0 = 对方取消, 1 = 拒绝, 2 = 其他原因
}

impl ZCTradeCancel {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x00F1).put_u8(self.reason).build()
    }
}
