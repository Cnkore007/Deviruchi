use std::collections::HashMap;

/// 最大堆叠数量
const MAX_STACK_SIZE: u16 = 30000;

/// 仓库格子
#[derive(Debug, Clone)]
pub struct StorageSlot {
    /// 格子索引
    pub index: u16,
    /// 物品ID
    pub item_id: u16,
    /// 物品数量
    pub amount: u16,
    /// 是否已鉴定
    pub identified: bool,
    /// 精炼等级
    pub refine: u8,
    /// 卡片插槽 [0-3]
    pub cards: [u16; 4],
}

impl StorageSlot {
    pub fn empty(index: u16) -> Self {
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

/// 角色仓库
pub struct Storage {
    char_id: u32,
    max_size: u16,
    slots: Vec<StorageSlot>,
}

impl Storage {
    pub fn new(max_size: u16) -> Self {
        let slots: Vec<_> = (0..max_size)
            .map(StorageSlot::empty)
            .collect();

        Self {
            char_id: 0,
            max_size,
            slots,
        }
    }

    pub fn with_char_id(mut self, char_id: u32) -> Self {
        self.char_id = char_id;
        self
    }

    pub fn char_id(&self) -> u32 {
        self.char_id
    }

    pub fn get_slot(&self, index: u16) -> Option<&StorageSlot> {
        self.slots.get(index as usize)
    }

    pub fn get_slot_mut(&mut self, index: u16) -> Option<&mut StorageSlot> {
        self.slots.get_mut(index as usize)
    }

    pub fn find_item_slot(&self, item_id: u16) -> Option<&StorageSlot> {
        self.slots.iter().find(|s| s.item_id == item_id)
    }

    pub fn add_item(&mut self, item_id: u16, amount: u16) -> bool {
        for slot in &mut self.slots {
            if slot.item_id == item_id && slot.amount + amount <= MAX_STACK_SIZE {
                slot.amount += amount;
                return true;
            }
        }

        for slot in &mut self.slots {
            if slot.is_empty() {
                slot.item_id = item_id;
                slot.amount = amount;
                slot.identified = true;
                return true;
            }
        }

        false
    }

    pub fn remove_item(&mut self, index: u16, amount: u16) -> bool {
        if index >= self.max_size {
            return false;
        }

        let slot = &mut self.slots[index as usize];
        if slot.amount >= amount {
            slot.amount -= amount;
            if slot.amount == 0 {
                slot.item_id = 0;
                slot.identified = false;
                slot.refine = 0;
                slot.cards = [0; 4];
            }
            return true;
        }

        false
    }

    pub fn move_item(&mut self, from_index: u16, to_index: u16) -> bool {
        if from_index >= self.max_size || to_index >= self.max_size {
            return false;
        }

        if from_index == to_index {
            return true;
        }

        // Check if items can be merged first (no mutable borrow yet)
        let from_slot = self.slots[from_index as usize].clone();
        let to_slot = &self.slots[to_index as usize];

        let should_merge = from_slot.item_id == to_slot.item_id
            && from_slot.item_id != 0
            && from_slot.amount + to_slot.amount <= MAX_STACK_SIZE;

        if should_merge {
            let total = from_slot.amount + to_slot.amount;
            self.slots[to_index as usize].amount = total;
            self.slots[from_index as usize] = StorageSlot::empty(from_index);
            return true;
        }

        // Perform swap - clone to_slot since we need it for the swap
        let to_slot = to_slot.clone();

        self.slots[to_index as usize] = StorageSlot {
            index: to_index,
            ..from_slot
        };
        self.slots[from_index as usize] = StorageSlot {
            index: from_index,
            ..to_slot
        };

        true
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn slots(&self) -> &[StorageSlot] {
        &self.slots
    }

    pub fn used_count(&self) -> usize {
        self.slots.iter().filter(|s| !s.is_empty()).count()
    }

    pub fn to_db_format(&self) -> Vec<(u16, u16, u16, bool, u8, [u16; 4])> {
        self.slots
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| (s.index, s.item_id, s.amount, s.identified, s.refine, s.cards))
            .collect()
    }

    pub fn from_db_format(char_id: u32, max_size: u16, items: Vec<(u16, u16, u16, bool, u8, [u16; 4])>) -> Self {
        let mut storage = Self::new(max_size);
        storage.char_id = char_id;

        for (index, item_id, amount, identified, refine, cards) in items {
            if index < max_size {
                storage.slots[index as usize] = StorageSlot {
                    index,
                    item_id,
                    amount,
                    identified,
                    refine,
                    cards,
                };
            }
        }

        storage
    }
}
