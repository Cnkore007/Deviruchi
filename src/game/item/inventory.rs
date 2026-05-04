use super::data::{Item, ItemDatabase};
use std::sync::Arc;

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
    total_weight: u32,
}

impl Inventory {
    pub fn new(max_size: u8, item_db: Arc<ItemDatabase>) -> Self {
        let slots: Vec<_> = (0..max_size).map(InventorySlot::empty).collect();

        Self {
            max_size,
            slots,
            item_db,
            total_weight: 0,
        }
    }

    /// 添加物品
    pub fn add_item(&mut self, item_id: u16, amount: u16) -> bool {
        if amount == 0 || amount > 300 {
            return false;
        }

        // 先找相同物品的空位
        for slot in &mut self.slots {
            if slot.item_id == item_id && slot.amount + amount <= 300 {
                slot.amount += amount;
                self.update_weight();
                return true;
            }
        }

        // 找空位
        for slot in &mut self.slots {
            if slot.is_empty() {
                slot.item_id = item_id;
                slot.amount = amount;
                slot.identified = true;
                self.update_weight();
                return true;
            }
        }

        false // 背包已满
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
            self.update_weight();
            return true;
        }

        false
    }

    /// 使用物品
    pub fn use_item(&mut self, index: u8) -> Option<Item> {
        if index >= self.max_size {
            return None;
        }

        let slot = &mut self.slots[index as usize];
        if slot.is_empty() {
            return None;
        }

        let item = self.item_db.get(slot.item_id)?.clone();
        if !matches!(item.type_, super::data::ItemType::Heal) {
            return None;
        }

        // 消耗物品
        slot.amount -= 1;
        if slot.amount == 0 {
            slot.item_id = 0;
        }
        self.update_weight();

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

    /// 计算总重量
    pub fn calc_weight(&self) -> u32 {
        self.slots
            .iter()
            .filter(|s| !s.is_empty())
            .filter_map(|s| {
                self.item_db
                    .get(s.item_id)
                    .map(|item| (item.weight as u32) * (s.amount as u32))
            })
            .sum()
    }

    /// 获取总重量
    pub fn total_weight(&self) -> u32 {
        self.total_weight
    }

    /// 检查能否添加物品（重量限制）
    pub fn can_carry_weight(&self, item_id: u16, amount: u16, max_weight: u32) -> bool {
        let item = match self.item_db.get(item_id) {
            Some(i) => i,
            None => return false,
        };
        let add_weight = (item.weight as u32) * (amount as u32);
        self.total_weight + add_weight <= max_weight
    }

    /// 更新重量
    pub fn update_weight(&mut self) {
        self.total_weight = self.calc_weight();
    }

    /// 获取物品重量
    pub fn get_item_weight(&self, item_id: u16) -> u16 {
        self.item_db.get(item_id).map(|i| i.weight).unwrap_or(0)
    }

    /// 获取物品数据库引用
    pub fn get_database(&self) -> &ItemDatabase {
        &self.item_db
    }

    /// 检查能否添加物品（仅检查空间，不检查重量）
    pub fn can_add_item(&self, item_id: u16, amount: u16) -> bool {
        if amount == 0 || amount > 300 {
            return false;
        }

        // 先找相同物品的空位
        for slot in &self.slots {
            if slot.item_id == item_id && slot.amount + amount <= 300 {
                return true;
            }
        }
        // 找空位
        for slot in &self.slots {
            if slot.is_empty() {
                return true;
            }
        }
        false
    }

    /// 从 CharacterInventoryData 创建 Inventory
    pub fn from_character_inventory(
        data: &[crate::storage::character::CharacterInventoryData],
        item_db: Arc<ItemDatabase>,
    ) -> Self {
        let max_size = 100u8;
        let mut inv = Self::new(max_size, item_db);

        for char_slot in data {
            if char_slot.index < max_size
                && let Some(slot) = inv.slots.get_mut(char_slot.index as usize)
            {
                slot.item_id = char_slot.item_id;
                slot.amount = char_slot.amount;
                slot.identified = char_slot.identified;
                slot.refine = char_slot.refine;
                slot.cards = char_slot.cards;
            }
        }

        inv.update_weight();
        inv
    }

    /// 转换为 CharacterInventoryData
    pub fn to_character_inventory(&self) -> Vec<crate::storage::character::CharacterInventoryData> {
        self.slots
            .iter()
            .filter(|slot| !slot.is_empty())
            .map(|slot| crate::storage::character::CharacterInventoryData {
                index: slot.index,
                item_id: slot.item_id,
                amount: slot.amount,
                identified: slot.identified,
                refine: slot.refine,
                cards: slot.cards,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_inventory_new_has_zero_weight() {
        let db = Arc::new(ItemDatabase::new());
        let inv = Inventory::new(10, db);
        assert_eq!(inv.total_weight(), 0);
    }

    #[test]
    fn test_add_item_updates_weight() {
        let db = Arc::new(ItemDatabase::new());
        let mut inv = Inventory::new(10, db);
        // Red Potion (501) weight = 7
        inv.add_item(501, 5);
        assert_eq!(inv.total_weight(), 35); // 7 * 5
    }

    #[test]
    fn test_remove_item_updates_weight() {
        let db = Arc::new(ItemDatabase::new());
        let mut inv = Inventory::new(10, db);
        inv.add_item(501, 5); // weight = 35
        inv.remove_item(0, 2); // remove 2
        assert_eq!(inv.total_weight(), 21); // 7 * 3
    }

    #[test]
    fn test_can_carry_weight() {
        let db = Arc::new(ItemDatabase::new());
        let inv = Inventory::new(10, db);
        // Red Potion weight = 7, max_weight = 100
        assert!(inv.can_carry_weight(501, 10, 100)); // 70 <= 100
        assert!(!inv.can_carry_weight(501, 20, 100)); // 140 > 100
    }

    #[test]
    fn test_get_item_weight() {
        let db = Arc::new(ItemDatabase::new());
        let inv = Inventory::new(10, db);
        assert_eq!(inv.get_item_weight(501), 7); // Red Potion
        assert_eq!(inv.get_item_weight(9999), 0); // Non-existent
    }
}
