//! 物品操作 handler：插卡、鉴定、精炼

use super::MapServer;
use crate::network::session::Session;
use crate::protocol::map_packets::{
    CzInsertCard, CzItemIdentify, CzWeaponRefine, ZcItemIdentifyAck, ZcWeaponRefineAck,
};
use crate::protocol::packet_builder::Packed;

impl MapServer {
    /// 处理插入卡片请求 (0x017C)
    pub(super) fn handle_insert_card(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzInsertCard::from_slice(data)?;

        tracing::info!(
            "Player {} 请求插入卡片: card_index={}, equip_index={}",
            player_id,
            pkt.card_index,
            pkt.equip_index
        );

        // 简化实现：记录日志并返回成功
        // 完整实现需要检查卡片类型、装备槽位等
        tracing::info!("Player {} 成功插入卡片", player_id);

        None
    }

    /// 处理鉴定物品请求 (0x01DD)
    pub(super) fn handle_item_identify(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzItemIdentify::from_slice(data)?;

        tracing::info!("Player {} 请求鉴定物品: index={}", player_id, pkt.index);

        // 简化实现：直接标记为已鉴定
        let player = self.map_state.get_player(&player_id)?;
        let mut inventory = player.inventory.write();
        if let Some(item) = inventory.get_mut(pkt.index as usize) {
            item.identified = true;
            tracing::info!("Player {} 成功鉴定物品", player_id);
            Some(
                ZcItemIdentifyAck {
                    index: pkt.index,
                    result: 0, // 成功
                }
                .to_packet(),
            )
        } else {
            tracing::warn!("Player {} 鉴定失败：物品不存在", player_id);
            Some(
                ZcItemIdentifyAck {
                    index: pkt.index,
                    result: 1, // 失败
                }
                .to_packet(),
            )
        }
    }

    /// 处理精炼武器请求 (0x0222)
    pub(super) fn handle_weapon_refine(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzWeaponRefine::from_slice(data)?;

        tracing::info!("Player {} 请求精炼武器: index={}", player_id, pkt.index);

        // 简化实现：随机成功/失败
        let success = rand::random::<bool>();

        let player = self.map_state.get_player(&player_id)?;
        let mut inventory = player.inventory.write();
        if let Some(item) = inventory.get_mut(pkt.index as usize) {
            if success && item.refine < 10 {
                item.refine += 1;
                tracing::info!(
                    "Player {} 精炼成功，当前精炼等级: {}",
                    player_id,
                    item.refine
                );
                Some(
                    ZcWeaponRefineAck {
                        result: 0, // 成功
                        index: pkt.index,
                    }
                    .to_packet(),
                )
            } else if item.refine >= 10 {
                tracing::warn!("Player {} 精炼失败：已达上限", player_id);
                Some(
                    ZcWeaponRefineAck {
                        result: 2, // 已达上限
                        index: pkt.index,
                    }
                    .to_packet(),
                )
            } else {
                tracing::info!("Player {} 精炼失败", player_id);
                Some(
                    ZcWeaponRefineAck {
                        result: 1, // 失败
                        index: pkt.index,
                    }
                    .to_packet(),
                )
            }
        } else {
            tracing::warn!("Player {} 精炼失败：物品不存在", player_id);
            Some(
                ZcWeaponRefineAck {
                    result: 1, // 失败
                    index: pkt.index,
                }
                .to_packet(),
            )
        }
    }
}
