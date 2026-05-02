use std::sync::Arc;
use super::data::{Item, ItemDatabase};

/// 背包格子
#[derive(Debug, Clone)]
pub struct InventorySlot {
    pub index: u8,
    pub item_id: u16,
    pub amount: u16,
    pub identified: bool,
    pub refine: u8,
    pub cards: [u16; 4],
}

impl InventorySlot {
    pub fn empty(index: u8) -> Self {
        Self {
            index,
            item_id: 0,
            amount: 0,
            identified: false,
            refine: 0,
            cards: [0; 4],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.item_id == 0
    }
}

/// 背包管理
pub struct Inventory {
    max_size: u8,
    slots: Vec<InventorySlot>,
    item_db: Arc<ItemDatabase>,
}

impl Inventory {
    pub fn new(max_size: u8, item_db: Arc<ItemDatabase>) -> Self {
        let slots: Vec<_> = (0..max_size)
            .map(InventorySlot::empty)
            .collect();

        Self {
            max_size,
            slots,
            item_db,
        }
    }

    /// 添加物品
    pub fn add_item(&mut self, item_id: u16, amount: u16) -> bool {
        // 先找相同物品的空位
        for slot in &mut self.slots {
            if slot.item_id == item_id && slot.amount + amount <= 300 {
                slot.amount += amount;
                return true;
            }
        }

        // 找空位
        for slot in &mut self.slots {
            if slot.is_empty() {
                slot.item_id = item_id;
                slot.amount = amount;
                slot.identified = true;
                return true;
            }
        }

        false  // 背包已满
    }

    /// 移除物品
    pub fn remove_item(&mut self, index: u8, amount: u16) -> bool {
        if index >= self.max_size {
            return false;
        }

        let slot = &mut self.slots[index as usize];
        if slot.amount >= amount {
            slot.amount -= amount;
            if slot.amount == 0 {
                slot.item_id = 0;
            }
            return true;
        }

        false
    }

    /// 使用物品
    pub fn use_item(&mut self, index: u8) -> Option<&Item> {
        if index >= self.max_size {
            return None;
        }

        let slot = &mut self.slots[index as usize];
        if slot.is_empty() {
            return None;
        }

        let item = self.item_db.get(slot.item_id)?;
        if !matches!(item.type_, super::data::ItemType::Heal) {
            return None;
        }

        // 消耗物品
        slot.amount -= 1;
        if slot.amount == 0 {
            slot.item_id = 0;
        }

        Some(item)
    }

    /// 获取格子数量
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// 获取所有格子
    pub fn slots(&self) -> &[InventorySlot] {
        &self.slots
    }
}
