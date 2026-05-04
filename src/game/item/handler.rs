use super::data::{ItemDatabase, ItemType};
use super::equipment::EquipSlot;
use super::inventory::{Inventory, InventorySlot};
use crate::game::map::Player;
use std::sync::Arc;

/// 物品处理器
pub struct ItemHandler {
    db: Arc<ItemDatabase>,
}

impl ItemHandler {
    pub fn new(db: Arc<ItemDatabase>) -> Self {
        Self { db }
    }

    /// 使用物品（治疗类）
    pub fn use_item(
        &self,
        player: &Player,
        inventory: &mut Inventory,
        slot_index: u8,
    ) -> Option<super::data::Item> {
        let slot = inventory.slots().get(slot_index as usize)?;
        if slot.is_empty() {
            return None;
        }

        let item = self.db.get(slot.item_id)?;

        if !matches!(item.type_, ItemType::Heal) {
            return None;
        }

        // 恢复HP
        if item.hp_restore > 0 {
            let current = player.hp();
            let max = player.max_hp();
            let new_hp = (current + item.hp_restore as u32).min(max);
            player.combat_mut().hp = new_hp;
        }

        // 恢复SP
        if item.sp_restore > 0 {
            let current = player.sp();
            let max = player.max_sp();
            let new_sp = (current + item.sp_restore as u32).min(max);
            player.combat_mut().sp = new_sp;
        }

        // 消耗物品
        inventory.use_item(slot_index);
        Some(item.clone())
    }

    /// 装备物品
    pub fn equip_item(
        &self,
        player: &Player,
        inventory: &mut Inventory,
        slot_index: u8,
        equip_slot: EquipSlot,
    ) -> EquipResult {
        let item = match inventory.slots().get(slot_index as usize) {
            Some(s) if !s.is_empty() => match self.db.get(s.item_id) {
                Some(i) => i.clone(),
                None => return EquipResult::Failed(EquipError::InvalidItem),
            },
            _ => return EquipResult::Failed(EquipError::InvalidSlot),
        };

        // 检查是否是装备
        if !matches!(item.type_, ItemType::Weapon | ItemType::Armor) {
            return EquipResult::Failed(EquipError::NotEquipable);
        }

        // 检查槽位兼容性
        let valid_slots = EquipSlot::from_mask(item.equip_mask);
        if !valid_slots.contains(&equip_slot) {
            return EquipResult::Failed(EquipError::WrongSlot);
        }

        // 从背包移除
        if !inventory.remove_item(slot_index, 1) {
            return EquipResult::Failed(EquipError::InvalidSlot);
        }

        // 装备到玩家
        let mut equipment = player.equipment.write();
        let old_item = equipment.equip(
            equip_slot,
            InventorySlot {
                index: slot_index,
                item_id: item.id,
                amount: 1,
                identified: true,
                refine: 0,
                cards: [0; 4],
            },
        );

        // 如果有旧装备，返还到背包
        if let Some(old) = old_item {
            inventory.add_item(old.item_id, 1);
        }

        EquipResult::Success {
            slot: equip_slot,
            item_id: item.id,
        }
    }

    /// 卸下装备
    pub fn unequip_item(
        &self,
        player: &Player,
        inventory: &mut Inventory,
        equip_slot: EquipSlot,
    ) -> UnequipResult {
        let mut equipment = player.equipment.write();
        let item = match equipment.unequip(equip_slot) {
            Some(i) => i,
            None => return UnequipResult::Failed(UnequipError::NoItemEquipped),
        };

        // 返还到背包
        if !inventory.add_item(item.item_id, item.amount) {
            // 背包满了，重新装备回去
            equipment.equip(equip_slot, item);
            return UnequipResult::Failed(UnequipError::InventoryFull);
        }

        UnequipResult::Success {
            slot: equip_slot,
            item_id: item.item_id,
        }
    }
}

/// 装备结果
#[derive(Debug, Clone)]
pub enum EquipResult {
    Success { slot: EquipSlot, item_id: u16 },
    Failed(EquipError),
}

/// 装备错误
#[derive(Debug, Clone, Copy)]
pub enum EquipError {
    InvalidSlot,
    InvalidItem,
    NotEquipable,
    WrongSlot,
    LevelTooLow,
    WrongJob,
}

/// 卸下装备结果
#[derive(Debug, Clone)]
pub enum UnequipResult {
    Success { slot: EquipSlot, item_id: u16 },
    Failed(UnequipError),
}

/// 卸下装备错误
#[derive(Debug, Clone, Copy)]
pub enum UnequipError {
    NoItemEquipped,
    InventoryFull,
}
