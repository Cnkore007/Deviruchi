use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 邮件附件中的物品
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailItem {
    pub item_id: u32,
    pub name: String,
    pub amount: u32,
    pub refined: u8,
    pub cards: [u16; 4],
    pub identified: bool,
}

impl Default for MailItem {
    fn default() -> Self {
        Self {
            item_id: 0,
            name: String::new(),
            amount: 0,
            refined: 0,
            cards: [0; 4],
            identified: true,
        }
    }
}

/// 邮件消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailMessage {
    pub mail_id: Uuid,
    pub sender_id: Uuid,
    pub sender_name: String,
    pub recipient_id: Uuid,
    pub recipient_name: String,
    pub title: String,
    pub body: String,
    pub zeny: u32,
    pub items: Vec<MailItem>,
    pub sent_time: u64,
    pub read: bool,
    pub claimed: bool,
    pub expires_at: u64,
}

impl MailMessage {
    pub fn new(
        sender_id: Uuid,
        sender_name: String,
        recipient_id: Uuid,
        recipient_name: String,
        title: String,
        body: String,
    ) -> Self {
        let now = crate::util::unix_timestamp_secs();
        Self {
            mail_id: Uuid::new_v4(),
            sender_id,
            sender_name,
            recipient_id,
            recipient_name,
            title,
            body,
            zeny: 0,
            items: Vec::new(),
            sent_time: now,
            read: false,
            claimed: false,
            expires_at: now + 30 * 24 * 3600, // 30天后过期
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = crate::util::unix_timestamp_secs();
        now > self.expires_at
    }

    pub fn has_attachments(&self) -> bool {
        self.zeny > 0 || !self.items.is_empty()
    }
}

/// 邮件操作结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailError {
    /// 收件人不存在
    RecipientNotFound,
    /// 邮箱已满
    MailboxFull,
    /// 邮件不存在
    MailNotFound,
    /// 附件已被领取
    AlreadyClaimed,
    /// 邮件已过期
    Expired,
    /// 不能给自己发邮件
    CannotMailSelf,
    /// 背包空间不足
    InventoryFull,
    /// 超重
    OverWeight,
    /// 物品不可交易
    Untradeable,
    /// 成功
    Success,
}

/// 邮件附件操作结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailAttachResult {
    Success = 0,
    OverWeight = 1,
    Error = 2,
    InventoryFull = 3,
    Untradeable = 4,
}

/// 邮件列表条目（客户端显示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailListEntry {
    pub mail_id: Uuid,
    pub sender_name: String,
    pub title: String,
    pub sent_time: u64,
    pub read: bool,
    pub has_zeny: bool,
    pub item_count: u32,
}
