//! 半魔娘和佣兵 handler

use super::MapServer;
use crate::network::session::Session;
use crate::protocol::map_packets::{CzHomMenu, CzMercenaryAction};

impl MapServer {
    /// 处理半魔娘操作 (0x022D)
    pub(super) fn handle_homunculus_menu(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzHomMenu::from_slice(data)?;

        tracing::info!(
            "Player {} 半魔娘操作: action={}",
            player_id, pkt.action
        );

        match pkt.action {
            0 => tracing::info!("Player {} 查看半魔娘信息", player_id),
            1 => tracing::info!("Player {} 喂食半魔娘", player_id),
            2 => tracing::info!("Player {} 放生半魔娘", player_id),
            3 => tracing::info!("Player {} 召回半魔娘", player_id),
            4 => tracing::info!("Player {} 半魔娘攻击", player_id),
            5 => tracing::info!("Player {} 半魔娘移动", player_id),
            _ => tracing::warn!("Player {} 未知半魔娘操作: {}", player_id, pkt.action),
        }

        None
    }

    /// 处理佣兵操作 (0x022F)
    pub(super) fn handle_mercenary_action(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzMercenaryAction::from_slice(data)?;

        tracing::info!(
            "Player {} 佣兵操作: action={}",
            player_id, pkt.action
        );

        match pkt.action {
            0 => tracing::info!("Player {} 查看佣兵信息", player_id),
            1 => tracing::info!("Player {} 召回佣兵", player_id),
            2 => tracing::info!("Player {} 放生佣兵", player_id),
            _ => tracing::warn!("Player {} 未知佣兵操作: {}", player_id, pkt.action),
        }

        None
    }
}
