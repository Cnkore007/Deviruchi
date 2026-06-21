use super::packet_builder::{Packed, PacketBuilderCtx};

// ========== Client -> Server ==========

/// 客户端请求打开银行 (0x09B7)
#[derive(Debug, Clone)]
pub struct CzBankOpen;

impl CzBankOpen {
    pub fn from_slice(_data: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 客户端请求关闭银行 (0x09B8)
#[derive(Debug, Clone)]
pub struct CzBankClose;

impl CzBankClose {
    pub fn from_slice(_data: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 客户端请求存款 (0x09B9)
#[derive(Debug, Clone)]
pub struct CzBankDeposit {
    /// 存款金额
    pub amount: u32,
}

impl CzBankDeposit {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let amount = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        Some(Self { amount })
    }
}

/// 客户端请求取款 (0x09BA)
#[derive(Debug, Clone)]
pub struct CzBankWithdraw {
    /// 取款金额
    pub amount: u32,
}

impl CzBankWithdraw {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let amount = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        Some(Self { amount })
    }
}

// ========== Server -> Client ==========

/// 服务器发送银行检查结果 (0x09A6)
#[derive(Debug, Clone)]
pub struct ZcBankCheck {
    /// Zeny 金额
    pub zeny: u64,
    /// 原因/结果码
    pub reason: u32,
}

impl Packed for ZcBankCheck {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x09A6)
            .put_u64(self.zeny)
            .put_u32(self.reason)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 服务器响应打开银行 (0x09B9)
#[derive(Debug, Clone)]
pub struct ZcBankOpen {
    /// 结果：0=成功, 1=失败
    pub result: u8,
}

impl Packed for ZcBankOpen {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x09B9)
            .put_u8(self.result)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}
