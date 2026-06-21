//! 装备系统 handler：穿戴、卸下

use super::MapServer;
use crate::game::item::EquipSlot;
use crate::network::session::Session;
use crate::protocol::map_packets::{
    CzReqTakeoffEquip, CzReqWearEquip, ZcReqTakeoffEquipAck, ZcReqWearEquipAck,
};
use crate::protocol::packet_builder::Packed;

impl MapServer {
    /// 处理穿戴装备请求 (0x00A9)
    pub(super) fn handle_equip_item(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzReqWearEquip::from_slice(data)?;

        tracing::info!(
            "Player {} 请求穿戴装备: index={}, position={}",
            player_id,
            pkt.index,
            pkt.position
        );

        let player = self.map_state.get_player(&player_id)?;

        // 解析装备槽位
        let slots = EquipSlot::from_mask(pkt.position as u32);
        if slots.is_empty() {
            tracing::warn!("无效的装备位置: {}", pkt.position);
            return Some(
                ZcReqWearEquipAck {
                    index: pkt.index,
                    position: pkt.position,
                    result: 1, // 失败
                }
                .to_packet(),
            );
        }

        // 获取背包中的物品
        let inventory = player.inventory.read();
        let item_data = inventory.get(pkt.index as usize)?;

        // 转换为 InventorySlot
        let item = crate::game::item::InventorySlot {
            index: item_data.index,
            item_id: item_data.item_id,
            amount: item_data.amount,
            identified: item_data.identified,
            refine: item_data.refine,
            cards: item_data.cards,
        };

        // 穿戴装备到第一个槽位
        let slot = slots[0];
        let mut equipment = player.equipment.write();
        let _old_item = equipment.equip(slot, item);

        tracing::info!("Player {} 成功穿戴装备到 {:?}", player_id, slot);

        Some(
            ZcReqWearEquipAck {
                index: pkt.index,
                position: pkt.position,
                result: 0, // 成功
            }
            .to_packet(),
        )
    }

    /// 处理卸下装备请求 (0x00AB)
    pub(super) fn handle_unequip_item(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzReqTakeoffEquip::from_slice(data)?;

        tracing::info!(
            "Player {} 请求卸下装备: position={}",
            player_id,
            pkt.position
        );

        let player = self.map_state.get_player(&player_id)?;

        // 解析装备槽位
        let slots = EquipSlot::from_mask(pkt.position as u32);
        if slots.is_empty() {
            tracing::warn!("无效的装备位置: {}", pkt.position);
            return Some(
                ZcReqTakeoffEquipAck {
                    position: pkt.position,
                    result: 1, // 失败
                }
                .to_packet(),
            );
        }

        // 卸下装备
        let slot = slots[0];
        let mut equipment = player.equipment.write();
        let removed = equipment.unequip(slot);

        if removed.is_some() {
            tracing::info!("Player {} 成功卸下 {:?} 的装备", player_id, slot);
            Some(
                ZcReqTakeoffEquipAck {
                    position: pkt.position,
                    result: 0, // 成功
                }
                .to_packet(),
            )
        } else {
            tracing::warn!("Player {} 在 {:?} 没有装备", player_id, slot);
            Some(
                ZcReqTakeoffEquipAck {
                    position: pkt.position,
                    result: 1, // 失败
                }
                .to_packet(),
            )
        }
    }
}
