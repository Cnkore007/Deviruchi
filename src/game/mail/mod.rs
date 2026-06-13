pub mod data;

use std::collections::HashMap;
use parking_lot::RwLock;
use tracing::debug;
use uuid::Uuid;

pub use data::*;

/// 每个玩家的邮箱最大邮件数
const MAX_MAILBOX_SIZE: usize = 100;

/// 邮件系统
pub struct MailSystem {
    /// player_id -> 收件箱
    inboxes: RwLock<HashMap<Uuid, Vec<MailMessage>>>,
    /// player_id -> 发件箱
    sent: RwLock<HashMap<Uuid, Vec<MailMessage>>>,
}

impl MailSystem {
    pub fn new() -> Self {
        Self {
            inboxes: RwLock::new(HashMap::new()),
            sent: RwLock::new(HashMap::new()),
        }
    }

    /// 发送邮件
    pub fn send_mail(
        &self,
        sender_id: Uuid,
        sender_name: &str,
        recipient_id: Uuid,
        recipient_name: &str,
        title: &str,
        body: &str,
    ) -> Result<Uuid, MailError> {
        if sender_id == recipient_id {
            return Err(MailError::CannotMailSelf);
        }

        let mail = MailMessage::new(
            sender_id,
            sender_name.to_string(),
            recipient_id,
            recipient_name.to_string(),
            title.to_string(),
            body.to_string(),
        );

        let mail_id = mail.mail_id;

        // 放入收件人的收件箱
        {
            let mut inboxes = self.inboxes.write();
            let inbox = inboxes.entry(recipient_id).or_default();
            if inbox.len() >= MAX_MAILBOX_SIZE {
                return Err(MailError::MailboxFull);
            }
            inbox.push(mail.clone());
        }

        // 放入发件人的发件箱
        {
            let mut sent = self.sent.write();
            sent.entry(sender_id).or_default().push(mail);
        }

        debug!("Mail sent: {} -> {} ({})", sender_name, recipient_name, mail_id);
        Ok(mail_id)
    }

    /// 获取收件箱邮件列表
    pub fn get_inbox_list(&self, player_id: &Uuid) -> Vec<MailListEntry> {
        let inboxes = self.inboxes.read();
        inboxes
            .get(player_id)
            .map(|mails| {
                mails
                    .iter()
                    .map(|m| MailListEntry {
                        mail_id: m.mail_id,
                        sender_name: m.sender_name.clone(),
                        title: m.title.clone(),
                        sent_time: m.sent_time,
                        read: m.read,
                        has_zeny: m.zeny > 0,
                        item_count: m.items.len() as u32,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取发件箱邮件列表
    pub fn get_sent_list(&self, player_id: &Uuid) -> Vec<MailListEntry> {
        let sent = self.sent.read();
        sent.get(player_id)
            .map(|mails| {
                mails
                    .iter()
                    .map(|m| MailListEntry {
                        mail_id: m.mail_id,
                        sender_name: m.recipient_name.clone(),
                        title: format!("Re: {}", m.title),
                        sent_time: m.sent_time,
                        read: m.read,
                        has_zeny: m.zeny > 0,
                        item_count: m.items.len() as u32,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 读取邮件详情
    pub fn read_mail(&self, player_id: &Uuid, mail_id: &Uuid) -> Option<MailMessage> {
        let mut inboxes = self.inboxes.write();
        if let Some(mails) = inboxes.get_mut(player_id)
            && let Some(mail) = mails.iter_mut().find(|m| &m.mail_id == mail_id) {
                mail.read = true;
                return Some(mail.clone());
            }
        None
    }

    /// 领取邮件附件
    pub fn claim_attachment(
        &self,
        player_id: &Uuid,
        mail_id: &Uuid,
    ) -> Result<MailMessage, MailError> {
        let mut inboxes = self.inboxes.write();
        let mails = inboxes
            .get_mut(player_id)
            .ok_or(MailError::MailNotFound)?;

        let mail = mails
            .iter_mut()
            .find(|m| &m.mail_id == mail_id)
            .ok_or(MailError::MailNotFound)?;

        if mail.claimed {
            return Err(MailError::AlreadyClaimed);
        }

        if mail.is_expired() {
            return Err(MailError::Expired);
        }

        mail.claimed = true;
        debug!("Attachment claimed for mail {}", mail_id);
        Ok(mail.clone())
    }

    /// 删除收件箱中的邮件
    pub fn delete_mail(&self, player_id: &Uuid, mail_id: &Uuid) -> bool {
        let mut inboxes = self.inboxes.write();
        if let Some(mails) = inboxes.get_mut(player_id) {
            let len_before = mails.len();
            mails.retain(|m| &m.mail_id != mail_id);
            return mails.len() < len_before;
        }
        false
    }

    /// 获取收件箱邮件数
    pub fn inbox_count(&self, player_id: &Uuid) -> usize {
        self.inboxes
            .read()
            .get(player_id)
            .map(|mails| mails.len())
            .unwrap_or(0)
    }

    /// 获取未读邮件数
    pub fn unread_count(&self, player_id: &Uuid) -> usize {
        self.inboxes
            .read()
            .get(player_id)
            .map(|mails| mails.iter().filter(|m| !m.read).count())
            .unwrap_or(0)
    }

    /// 清理过期邮件
    pub fn purge_expired(&self) {
        let mut inboxes = self.inboxes.write();
        for (_, mails) in inboxes.iter_mut() {
            mails.retain(|m| !m.is_expired() || m.has_attachments());
        }

        let mut sent = self.sent.write();
        for (_, mails) in sent.iter_mut() {
            mails.retain(|m| !m.is_expired() || m.has_attachments());
        }

        debug!("Expired mails purged");
    }

    /// 清理玩家数据（玩家离线/下线时）
    pub fn cleanup_player(&self, player_id: &Uuid) {
        self.inboxes.write().remove(player_id);
        self.sent.write().remove(player_id);
        debug!("Mail data cleaned up for player {}", player_id);
    }
}

impl Default for MailSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_id(seed: u64) -> Uuid {
        Uuid::from_u64_pair(seed, seed)
    }

    #[test]
    fn test_send_and_read_mail() {
        let system = MailSystem::new();
        let sender = make_id(1);
        let recipient = make_id(2);

        let mail_id = system
            .send_mail(sender, "Sender", recipient, "Receiver", "Hello", "Body text")
            .unwrap();

        let inbox = system.get_inbox_list(&recipient);
        assert_eq!(inbox.len(), 1);
        assert!(!inbox[0].read);

        let mail = system.read_mail(&recipient, &mail_id).unwrap();
        assert_eq!(mail.title, "Hello");
        assert_eq!(mail.sender_name, "Sender");
    }

    #[test]
    fn test_cannot_mail_self() {
        let system = MailSystem::new();
        let player = make_id(1);

        let result = system.send_mail(player, "Self", player, "Self", "Hi", "Body");
        assert!(matches!(result, Err(MailError::CannotMailSelf)));
    }

    #[test]
    fn test_mailbox_limit() {
        let system = MailSystem::new();
        let recipient = make_id(2);

        for i in 0..100 {
            let sender = make_id(100 + i);
            system
                .send_mail(sender, &format!("Sender{}", i), recipient, "Recv", "Hi", "Body")
                .unwrap();
        }

        // 第101封应该失败
        let sender = make_id(999);
        let result = system.send_mail(sender, "Overflow", recipient, "Recv", "Hi", "Body");
        assert!(matches!(result, Err(MailError::MailboxFull)));
    }

    #[test]
    fn test_claim_attachment() {
        let system = MailSystem::new();
        let sender = make_id(1);
        let recipient = make_id(2);

        let mail_id = system
            .send_mail(sender, "Sender", recipient, "Receiver", "With items", "Body")
            .unwrap();

        let result = system.claim_attachment(&recipient, &mail_id);
        assert!(result.is_ok());

        // 重复领取应该失败
        let result2 = system.claim_attachment(&recipient, &mail_id);
        assert!(matches!(result2, Err(MailError::AlreadyClaimed)));
    }

    #[test]
    fn test_delete_mail() {
        let system = MailSystem::new();
        let sender = make_id(1);
        let recipient = make_id(2);

        let mail_id = system
            .send_mail(sender, "Sender", recipient, "Receiver", "Delete me", "Body")
            .unwrap();

        assert!(system.delete_mail(&recipient, &mail_id));
        assert_eq!(system.get_inbox_list(&recipient).len(), 0);
    }

    #[test]
    fn test_unread_count() {
        let system = MailSystem::new();
        let sender = make_id(1);
        let recipient = make_id(2);

        for i in 0..5 {
            system
                .send_mail(sender, "S", recipient, "R", &format!("T{}", i), "B")
                .unwrap();
        }

        assert_eq!(system.unread_count(&recipient), 5);
        assert_eq!(system.inbox_count(&recipient), 5);
    }
}
