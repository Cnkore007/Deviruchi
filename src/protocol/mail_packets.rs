use super::packet_builder::{Packed, PacketBuilderCtx, parse_string};

// ========== Client -> Server ==========

/// 客户端请求打开邮箱 (0x0260)
#[derive(Debug, Clone)]
pub struct CzMailOpen;

impl CzMailOpen {
    pub fn from_slice(_data: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 客户端请求发送邮件 (0x0261)
#[derive(Debug, Clone)]
pub struct CzMailSend {
    /// 收件人名称
    pub receiver: String,
    /// 邮件标题
    pub title: String,
    /// 邮件内容
    pub body: String,
}

impl CzMailSend {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 24 {
            return None;
        }
        let receiver = String::from_utf8_lossy(&data[0..24])
            .trim_matches('\0')
            .to_string();
        let mut offset = 24;
        let title = parse_string(data, &mut offset)?;
        let body = parse_string(data, &mut offset)?;
        Some(Self {
            receiver,
            title,
            body,
        })
    }
}

// ========== Server -> Client ==========

/// 服务器发送邮件列表 (0x0240)
#[derive(Debug, Clone)]
pub struct ZcMailList {
    /// 邮件数量
    pub count: u8,
}

impl Packed for ZcMailList {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x0240)
            .put_u8(self.count)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 服务器通知收到邮件 (0x0241)
#[derive(Debug, Clone)]
pub struct ZcMailReceive {
    /// 邮件ID
    pub mail_id: u32,
}

impl Packed for ZcMailReceive {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x0241)
            .put_u32(self.mail_id)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 服务器响应发送邮件结果 (0x0249)
#[derive(Debug, Clone)]
pub struct ZcMailSendAck {
    /// 结果：0=成功, 1=失败
    pub result: u8,
}

impl Packed for ZcMailSendAck {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilderCtx::new(0x0249)
            .put_u8(self.result)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}
