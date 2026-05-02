use std::collections::HashMap;

/// 仓库格子
#[derive(Debug, Clone)]
pub struct StorageSlot {
    pub index: u16,
    pub item_id: u16,
    pub amount: u16,
    pub identified: bool,
    pub refine: u8,
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
            if slot.item_id == item_id && slot.amount + amount <= 30000 {
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

        let from_slot = self.slots[from_index as usize].clone();
        let to_slot = self.slots[to_index as usize].clone();

        if from_slot.item_id == to_slot.item_id && from_slot.item_id != 0 {
            let total = from_slot.amount + to_slot.amount;
            if total <= 30000 {
                self.slots[to_index as usize].amount = total;
                self.slots[from_index as usize] = StorageSlot::empty(from_index);
                return true;
            }
        }

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
