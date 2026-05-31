//! NPC 商店 handler：购买、出售

use super::MapServer;
use crate::network::session::Session;
use crate::protocol::map_packets::{
    CzNpcBuyListSend, CzNpcSellListSend, ZcPcPurchaseResult, ZcPcSellResult,
};
use crate::protocol::packet_builder::Packed;

impl MapServer {
    /// 处理NPC购买请求 (0x00C8)
    pub(super) fn handle_npc_buy(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzNpcBuyListSend::from_slice(data)?;

        tracing::info!(
            "Player {} 请求购买 {} 个物品",
            player_id, pkt.count
        );

        let player = self.map_state.get_player(&player_id)?;

        // 计算总价（简化实现：每个物品 100 zeny）
        let total_cost = pkt.count as u32 * 100;

        // 检查 zeny 是否足够
        let economy = player.economy();
        if economy.zeny < total_cost {
            tracing::warn!("Player {} zeny 不足: {} < {}", player_id, economy.zeny, total_cost);
            return Some(ZcPcPurchaseResult { result: 2 }.to_packet()); // zeny不足
        }

        // 扣除 zeny
        drop(economy);
        let mut economy = player.economy_mut();
        economy.zeny -= total_cost;

        // 添加物品到背包（简化实现）
        tracing::info!(
            "Player {} 成功购买 {} 个物品，花费 {} zeny",
            player_id, pkt.count, total_cost
        );

        Some(ZcPcPurchaseResult { result: 0 }.to_packet()) // 成功
    }

    /// 处理NPC出售请求 (0x00C9)
    pub(super) fn handle_npc_sell(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzNpcSellListSend::from_slice(data)?;

        tracing::info!(
            "Player {} 请求出售 {} 个物品",
            player_id, pkt.count
        );

        let player = self.map_state.get_player(&player_id)?;

        // 计算总收入（简化实现：每个物品 50 zeny）
        let total_income = pkt.count as u32 * 50;

        // 增加 zeny
        let mut economy = player.economy_mut();
        economy.zeny += total_income;

        // 从背包移除物品（简化实现）
        tracing::info!(
            "Player {} 成功出售 {} 个物品，获得 {} zeny",
            player_id, pkt.count, total_income
        );

        Some(ZcPcSellResult { result: 0 }.to_packet()) // 成功
    }
}
