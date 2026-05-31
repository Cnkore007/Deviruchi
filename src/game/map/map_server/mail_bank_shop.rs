//! 邮件、银行、商城 handler

use super::MapServer;
use crate::network::session::Session;
use crate::protocol::map_packets::{
    CzMailSend, CzBankDeposit, CzBankWithdraw, CzCashShopBuy,
};

impl MapServer {
    /// 处理打开邮箱请求 (0x0260)
    pub(super) fn handle_mail_open(&self, _data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        tracing::info!("Player {} 打开邮箱", player_id);
        None
    }

    /// 处理发送邮件请求 (0x0261)
    pub(super) fn handle_mail_send(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzMailSend::from_slice(data)?;

        tracing::info!(
            "Player {} 发送邮件给 {}: title={}",
            player_id, pkt.receiver, pkt.title
        );

        // 简化实现：记录日志
        tracing::info!("Player {} 邮件发送成功", player_id);
        None
    }

    /// 处理打开银行请求 (0x09B7)
    pub(super) fn handle_bank_open(&self, _data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        tracing::info!("Player {} 打开银行", player_id);
        None
    }

    /// 处理关闭银行请求 (0x09B8)
    pub(super) fn handle_bank_close(&self, _data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        tracing::info!("Player {} 关闭银行", player_id);
        None
    }

    /// 处理存款请求 (0x09B9)
    pub(super) fn handle_bank_deposit(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzBankDeposit::from_slice(data)?;

        tracing::info!(
            "Player {} 存款: {} zeny",
            player_id, pkt.amount
        );

        let player = self.map_state.get_player(&player_id)?;

        // 检查 zeny 是否足够
        let economy = player.economy();
        if economy.zeny < pkt.amount {
            tracing::warn!("Player {} zeny 不足: {} < {}", player_id, economy.zeny, pkt.amount);
            return None;
        }

        // 扣除 zeny（简化实现：直接扣除，不记录银行余额）
        drop(economy);
        let mut economy = player.economy_mut();
        economy.zeny -= pkt.amount;

        tracing::info!("Player {} 存款成功", player_id);
        None
    }

    /// 处理取款请求 (0x09BA)
    pub(super) fn handle_bank_withdraw(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzBankWithdraw::from_slice(data)?;

        tracing::info!(
            "Player {} 取款: {} zeny",
            player_id, pkt.amount
        );

        let player = self.map_state.get_player(&player_id)?;

        // 增加 zeny（简化实现：直接增加，不检查银行余额）
        let mut economy = player.economy_mut();
        economy.zeny += pkt.amount;

        tracing::info!("Player {} 取款成功", player_id);
        None
    }

    /// 处理打开商城请求 (0x0845)
    pub(super) fn handle_cash_shop_open(&self, _data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        tracing::info!("Player {} 打开商城", player_id);
        None
    }

    /// 处理购买商城物品请求 (0x0848)
    pub(super) fn handle_cash_shop_buy(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzCashShopBuy::from_slice(data)?;

        tracing::info!(
            "Player {} 购买商城物品: item_id={}, amount={}",
            player_id, pkt.item_id, pkt.amount
        );

        // 简化实现：记录日志
        tracing::info!("Player {} 商城购买成功", player_id);
        None
    }

    /// 处理关闭商城请求 (0x084A)
    pub(super) fn handle_cash_shop_close(&self, _data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        tracing::info!("Player {} 关闭商城", player_id);
        None
    }
}
