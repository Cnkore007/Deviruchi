//! 宠物 handler：捕捉、菜单、选择蛋

use super::MapServer;
use crate::network::session::Session;
use crate::protocol::map_packets::{CzCatchPet, CzPetMenu, CzSelectEgg};

impl MapServer {
    /// 处理捕捉宠物请求 (0x019F)
    pub(super) fn handle_catch_pet(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzCatchPet::from_slice(data)?;

        tracing::info!(
            "Player {} 请求捕捉宠物: mob_id={}",
            player_id, pkt.mob_id
        );

        // 简化实现：记录日志
        // 完整实现需要检查捕捉概率、消耗捕捉道具等
        tracing::info!("Player {} 尝试捕捉宠物", player_id);

        None
    }

    /// 处理宠物菜单请求 (0x01A9)
    pub(super) fn handle_pet_menu(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzPetMenu::from_slice(data)?;

        tracing::info!(
            "Player {} 请求宠物菜单: action={}",
            player_id, pkt.action
        );

        match pkt.action {
            0 => tracing::info!("Player {} 查看宠物信息", player_id),
            1 => tracing::info!("Player {} 喂食宠物", player_id),
            2 => tracing::info!("Player {} 放生宠物", player_id),
            3 => tracing::info!("Player {} 召回宠物", player_id),
            _ => tracing::warn!("Player {} 未知宠物操作: {}", player_id, pkt.action),
        }

        None
    }

    /// 处理选择宠物蛋请求 (0x01A7)
    pub(super) fn handle_select_egg(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzSelectEgg::from_slice(data)?;

        tracing::info!(
            "Player {} 选择宠物蛋: index={}",
            player_id, pkt.egg_index
        );

        // 简化实现：记录日志
        tracing::info!("Player {} 孵化宠物蛋", player_id);

        None
    }
}
