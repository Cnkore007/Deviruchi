//! 装备系统

use super::data::ItemDatabase;
use super::inventory::InventorySlot;
use std::collections::HashMap;

/// 装备槽位
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipSlot {
    /// 头盔(上) - 掩码 0x0004
    HeadTop,
    /// 头盔(中) - 掩码 0x0100
    HeadMid,
    /// 头盔(下) - 掩码 0x0010
    HeadLow,
    /// 身体 - 掩码 0x0008
    Body,
    /// 右手武器 - 掩码 0x0001
    RightHand,
    /// 左手(武器/盾牌) - 掩码 0x0002
    LeftHand,
    /// 披风 - 掩码 0x0200
    Robe,
    /// 鞋子 - 掩码 0x0020
    Shoes,
    /// 饰品1 - 掩码 0x0040
    Accessory1,
    /// 饰品2 - 掩码 0x0080
    Accessory2,
}

impl EquipSlot {
    /// 转换为掩码值
    pub fn to_mask(&self) -> u32 {
        match self {
            EquipSlot::RightHand => 0x0001,
            EquipSlot::LeftHand => 0x0002,
            EquipSlot::HeadTop => 0x0004,
            EquipSlot::Body => 0x0008,
            EquipSlot::HeadLow => 0x0010,
            EquipSlot::Shoes => 0x0020,
            EquipSlot::Accessory1 => 0x0040,
            EquipSlot::Accessory2 => 0x0080,
            EquipSlot::HeadMid => 0x0100,
            EquipSlot::Robe => 0x0200,
        }
    }

    /// 从掩码解析槽位列表
    pub fn from_mask(mask: u32) -> Vec<EquipSlot> {
        let mut slots = Vec::new();

        if mask & 0x0001 != 0 {
            slots.push(EquipSlot::RightHand);
        }
        if mask & 0x0002 != 0 {
            slots.push(EquipSlot::LeftHand);
        }
        if mask & 0x0004 != 0 {
            slots.push(EquipSlot::HeadTop);
        }
        if mask & 0x0008 != 0 {
            slots.push(EquipSlot::Body);
        }
        if mask & 0x0010 != 0 {
            slots.push(EquipSlot::HeadLow);
        }
        if mask & 0x0020 != 0 {
            slots.push(EquipSlot::Shoes);
        }
        if mask & 0x0040 != 0 {
            slots.push(EquipSlot::Accessory1);
        }
        if mask & 0x0080 != 0 {
            slots.push(EquipSlot::Accessory2);
        }
        if mask & 0x0100 != 0 {
            slots.push(EquipSlot::HeadMid);
        }
        if mask & 0x0200 != 0 {
            slots.push(EquipSlot::Robe);
        }

        slots
    }
}

/// 装备管理
#[derive(Debug, Clone)]
pub struct Equipment {
    slots: HashMap<EquipSlot, InventorySlot>,
}

impl Equipment {
    /// 创建空装备管理器
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }

    /// 装备物品，返回被替换的旧物品（如果有）
    pub fn equip(&mut self, slot: EquipSlot, item: InventorySlot) -> Option<InventorySlot> {
        self.slots.insert(slot, item)
    }

    /// 卸下装备
    pub fn unequip(&mut self, slot: EquipSlot) -> Option<InventorySlot> {
        self.slots.remove(&slot)
    }

    /// 获取指定槽位的装备
    pub fn get(&self, slot: EquipSlot) -> Option<&InventorySlot> {
        self.slots.get(&slot)
    }

    /// 获取所有装备
    pub fn get_all(&self) -> &HashMap<EquipSlot, InventorySlot> {
        &self.slots
    }

    /// 清空所有装备
    pub fn clear(&mut self) {
        self.slots.clear();
    }

    /// 计算装备提供的总魔法防御
    pub fn total_magic_defense(&self, item_db: &ItemDatabase) -> u16 {
        self.slots
            .values()
            .filter_map(|slot| item_db.get(slot.item_id))
            .map(|item| item.magic_defense)
            .sum()
    }
}

impl Default for Equipment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::item::inventory::InventorySlot;

    #[test]
    fn test_equipment_new_is_empty() {
        let eq = Equipment::new();
        assert!(eq.get(EquipSlot::RightHand).is_none());
    }

    #[test]
    fn test_equip_item() {
        let mut eq = Equipment::new();
        let slot = InventorySlot {
            index: 0,
            item_id: 1201,
            amount: 1,
            identified: true,
            refine: 0,
            cards: [0; 4],
        };
        let old = eq.equip(EquipSlot::RightHand, slot.clone());
        assert!(old.is_none());
        assert_eq!(eq.get(EquipSlot::RightHand).unwrap().item_id, 1201);
    }

    #[test]
    fn test_equip_replace_old() {
        let mut eq = Equipment::new();
        let slot1 = InventorySlot {
            index: 0,
            item_id: 1201,
            amount: 1,
            identified: true,
            refine: 0,
            cards: [0; 4],
        };
        let slot2 = InventorySlot {
            index: 1,
            item_id: 1202,
            amount: 1,
            identified: true,
            refine: 0,
            cards: [0; 4],
        };
        eq.equip(EquipSlot::RightHand, slot1);
        let old = eq.equip(EquipSlot::RightHand, slot2);
        assert!(old.is_some());
        assert_eq!(old.unwrap().item_id, 1201);
        assert_eq!(eq.get(EquipSlot::RightHand).unwrap().item_id, 1202);
    }

    #[test]
    fn test_unequip() {
        let mut eq = Equipment::new();
        let slot = InventorySlot {
            index: 0,
            item_id: 1201,
            amount: 1,
            identified: true,
            refine: 0,
            cards: [0; 4],
        };
        eq.equip(EquipSlot::RightHand, slot);
        let removed = eq.unequip(EquipSlot::RightHand);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().item_id, 1201);
        assert!(eq.get(EquipSlot::RightHand).is_none());
    }

    #[test]
    fn test_equip_slot_mask() {
        assert_eq!(EquipSlot::RightHand.to_mask(), 0x0001);
        assert_eq!(EquipSlot::Body.to_mask(), 0x0008);
        let slots = EquipSlot::from_mask(0x0009); // RightHand + Body
        assert_eq!(slots.len(), 2);
        assert!(slots.contains(&EquipSlot::RightHand));
        assert!(slots.contains(&EquipSlot::Body));
    }
}
