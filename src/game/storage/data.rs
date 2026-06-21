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
#[derive(Debug, Clone)]
pub struct Storage {
    char_id: u32,
    max_size: u16,
    slots: Vec<StorageSlot>,
}

impl Storage {
    pub fn new(max_size: u16) -> Self {
        let slots: Vec<_> = (0..max_size).map(StorageSlot::empty).collect();

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

    /// 添加物品到仓库（完整参数版本，支持精炼和卡片）
    ///
    /// # 参数
    /// - `item_id`: 物品 ID
    /// - `amount`: 数量
    /// - `identified`: 是否已鉴定
    /// - `refine`: 精炼等级
    /// - `cards`: 卡片插槽 [0-3]
    ///
    /// # 返回
    /// `true` 表示添加成功，`false` 表示仓库已满
    pub fn add_item_full(
        &mut self,
        item_id: u16,
        amount: u16,
        identified: bool,
        refine: u8,
        cards: [u16; 4],
    ) -> bool {
        // 精炼等级 > 0 或有卡片的通常是装备，不参与堆叠
        let is_equipment = refine > 0 || cards.iter().any(|&c| c != 0);

        if !is_equipment {
            // 非装备物品尝试堆叠
            for slot in &mut self.slots {
                if slot.item_id == item_id
                    && slot.amount + amount <= MAX_STACK_SIZE
                    && slot.refine == 0
                    && slot.cards == [0; 4]
                {
                    slot.amount += amount;
                    return true;
                }
            }
        }

        // 放入空格子
        for slot in &mut self.slots {
            if slot.is_empty() {
                slot.item_id = item_id;
                slot.amount = amount;
                slot.identified = identified;
                slot.refine = refine;
                slot.cards = cards;
                return true;
            }
        }

        false
    }

    /// 添加物品到仓库（简化版本，兼容原有接口）
    ///
    /// 用于消耗品等不需要精炼/卡片的物品。
    /// 默认 identified=true, refine=0, cards=[0;4]。
    pub fn add_item(&mut self, item_id: u16, amount: u16) -> bool {
        self.add_item_full(item_id, amount, true, 0, [0; 4])
    }

    /// 添加物品到仓库并返回存放的槽位索引
    ///
    /// 与 `add_item` 逻辑相同，但成功时返回物品最终所在的 `index`。
    pub fn add_item_and_get_index(
        &mut self,
        item_id: u16,
        amount: u16,
        identified: bool,
        refine: u8,
        cards: [u16; 4],
    ) -> Option<u16> {
        let is_equipment = refine > 0 || cards.iter().any(|&c| c != 0);

        if !is_equipment {
            for slot in &mut self.slots {
                if slot.item_id == item_id
                    && slot.amount + amount <= MAX_STACK_SIZE
                    && slot.refine == 0
                    && slot.cards == [0; 4]
                {
                    slot.amount += amount;
                    return Some(slot.index);
                }
            }
        }

        for slot in &mut self.slots {
            if slot.is_empty() {
                slot.item_id = item_id;
                slot.amount = amount;
                slot.identified = identified;
                slot.refine = refine;
                slot.cards = cards;
                return Some(slot.index);
            }
        }

        None
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn slots(&self) -> &[StorageSlot] {
        &self.slots
    }

    pub fn used_count(&self) -> usize {
        self.slots.iter().filter(|s| !s.is_empty()).count()
    }

    /// 检查仓库是否已满（所有格子都被占用）
    pub fn is_full(&self) -> bool {
        self.slots.iter().all(|s| !s.is_empty())
    }

    pub fn to_db_format(&self) -> Vec<(u16, u16, u16, bool, u8, [u16; 4])> {
        self.slots
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| {
                (
                    s.index,
                    s.item_id,
                    s.amount,
                    s.identified,
                    s.refine,
                    s.cards,
                )
            })
            .collect()
    }

    pub fn from_db_format(
        char_id: u32,
        max_size: u16,
        items: Vec<(u16, u16, u16, bool, u8, [u16; 4])>,
    ) -> Self {
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

    /// 从仓库数据创建 (用于数据库加载)
    pub fn from_slots(char_id: u32, max_size: u16, slots: Vec<StorageSlot>) -> Self {
        let mut storage = Self::new(max_size);
        storage.char_id = char_id;

        for (i, slot) in slots.into_iter().enumerate() {
            if i < max_size as usize {
                storage.slots[i] = slot;
            }
        }

        storage
    }

    /// 获取最大大小
    pub fn max_size(&self) -> u16 {
        self.max_size
    }

    /// 设置格子数据
    pub fn set_slot(&mut self, index: usize, slot: StorageSlot) {
        if index < self.slots.len() {
            self.slots[index] = slot;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== StorageSlot 测试 ==========

    /// 空格子的默认值验证
    #[test]
    fn empty_slot_has_zero_item_id() {
        let slot = StorageSlot::empty(5);
        assert_eq!(slot.index, 5);
        assert_eq!(slot.item_id, 0);
        assert_eq!(slot.amount, 0);
        assert!(!slot.identified);
        assert_eq!(slot.refine, 0);
        assert_eq!(slot.cards, [0; 4]);
        assert!(slot.is_empty());
    }

    // ========== Storage 基础测试 ==========

    /// 新建仓库的初始状态
    #[test]
    fn new_storage_has_correct_size() {
        let storage = Storage::new(100);
        assert_eq!(storage.len(), 100);
        assert_eq!(storage.max_size(), 100);
        assert_eq!(storage.used_count(), 0);
    }

    /// with_char_id 设置角色 ID
    #[test]
    fn with_char_id_sets_correctly() {
        let storage = Storage::new(10).with_char_id(42);
        assert_eq!(storage.char_id(), 42);
    }

    /// 空仓库 is_full 应返回 false
    #[test]
    fn is_full_returns_false_for_empty_storage() {
        let storage = Storage::new(10);
        assert!(!storage.is_full());
    }

    /// 所有格子占满时 is_full 应返回 true
    #[test]
    fn is_full_returns_true_when_all_slots_used() {
        let mut storage = Storage::new(3);
        storage.add_item(501, 1);
        storage.add_item(502, 1);
        storage.add_item(503, 1);
        assert!(storage.is_full());
    }

    /// 部分格子占用时 is_full 应返回 false
    #[test]
    fn is_full_returns_false_when_some_slots_used() {
        let mut storage = Storage::new(100);
        storage.add_item(501, 1);
        assert!(!storage.is_full());
    }

    // ========== get_slot / get_slot_mut 测试 ==========

    /// 越界索引返回 None
    #[test]
    fn get_slot_returns_none_for_out_of_bounds() {
        let storage = Storage::new(10);
        assert!(storage.get_slot(10).is_none());
        assert!(storage.get_slot(100).is_none());
    }

    /// 正常索引返回格子引用
    #[test]
    fn get_slot_returns_slot_within_bounds() {
        let storage = Storage::new(10);
        let slot = storage.get_slot(0).unwrap();
        assert!(slot.is_empty());
    }

    /// 通过 get_slot_mut 修改格子数据
    #[test]
    fn get_slot_mut_modifies_slot() {
        let mut storage = Storage::new(10);
        let slot = storage.get_slot_mut(0).unwrap();
        slot.item_id = 501;
        slot.amount = 10;
        assert_eq!(storage.get_slot(0).unwrap().item_id, 501);
        assert_eq!(storage.get_slot(0).unwrap().amount, 10);
    }

    // ========== add_item 测试 ==========

    /// 向空仓库添加物品
    #[test]
    fn add_item_to_empty_storage() {
        let mut storage = Storage::new(10);
        assert!(storage.add_item(501, 5));
        assert_eq!(storage.used_count(), 1);
        let slot = storage.get_slot(0).unwrap();
        assert_eq!(slot.item_id, 501);
        assert_eq!(slot.amount, 5);
        assert!(slot.identified); // 默认已鉴定
    }

    /// 相同物品自动堆叠
    #[test]
    fn add_item_stacks_same_item() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 10);
        storage.add_item(501, 20);
        assert_eq!(storage.used_count(), 1);
        assert_eq!(storage.get_slot(0).unwrap().amount, 30);
    }

    /// 堆叠溢出时放入新格子
    #[test]
    fn add_item_does_not_exceed_max_stack() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 29000);
        // 再加 2000 会导致堆叠溢出，应放到新格子
        assert!(storage.add_item(501, 2000));
        assert_eq!(storage.used_count(), 2);
        assert_eq!(storage.get_slot(0).unwrap().amount, 29000);
        assert_eq!(storage.get_slot(1).unwrap().amount, 2000);
    }

    /// 仓库满且无相同物品可堆叠时返回 false
    #[test]
    fn add_item_returns_false_when_full() {
        let mut storage = Storage::new(2);
        assert!(storage.add_item(501, 1));
        assert!(storage.add_item(502, 1));
        // 仓库已满，不同物品无法添加
        assert!(!storage.add_item(503, 1));
    }

    /// 仓库满但相同物品仍可堆叠
    #[test]
    fn add_item_same_item_can_still_stack_when_full() {
        let mut storage = Storage::new(2);
        storage.add_item(501, 1);
        storage.add_item(502, 1);
        // 相同物品如果能堆叠，仍然可以添加
        assert!(storage.add_item(501, 5));
        assert_eq!(storage.get_slot(0).unwrap().amount, 6);
    }

    /// 物品放入第一个空格子
    #[test]
    fn add_item_fills_first_empty_slot() {
        let mut storage = Storage::new(5);
        storage.add_item(501, 1); // slot 0
        storage.add_item(502, 1); // slot 1
        storage.remove_item(0, 1); // slot 0 清空
        storage.add_item(503, 1); // 应该放到 slot 0
        assert_eq!(storage.get_slot(0).unwrap().item_id, 503);
    }

    // ========== remove_item 测试 ==========

    /// 减少物品数量
    #[test]
    fn remove_item_decreases_amount() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 10);
        assert!(storage.remove_item(0, 3));
        assert_eq!(storage.get_slot(0).unwrap().amount, 7);
        assert_eq!(storage.get_slot(0).unwrap().item_id, 501);
    }

    /// 数量归零时清空格子
    #[test]
    fn remove_item_clears_slot_when_amount_reaches_zero() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5);
        assert!(storage.remove_item(0, 5));
        assert!(storage.get_slot(0).unwrap().is_empty());
    }

    /// 数量不足时返回 false
    #[test]
    fn remove_item_returns_false_for_insufficient_amount() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5);
        assert!(!storage.remove_item(0, 10));
        // 数量不变
        assert_eq!(storage.get_slot(0).unwrap().amount, 5);
    }

    /// 越界索引返回 false
    #[test]
    fn remove_item_returns_false_for_out_of_bounds() {
        let mut storage = Storage::new(10);
        assert!(!storage.remove_item(10, 1));
        assert!(!storage.remove_item(100, 1));
    }

    /// 空格子移除返回 false
    #[test]
    fn remove_item_returns_false_for_empty_slot() {
        let mut storage = Storage::new(10);
        assert!(!storage.remove_item(0, 1));
    }

    // ========== move_item 测试 ==========

    /// 交换两个格子的物品
    #[test]
    fn move_item_swaps_two_slots() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5); // slot 0
        storage.add_item(502, 3); // slot 1

        assert!(storage.move_item(0, 1));
        assert_eq!(storage.get_slot(0).unwrap().item_id, 502);
        assert_eq!(storage.get_slot(0).unwrap().amount, 3);
        assert_eq!(storage.get_slot(1).unwrap().item_id, 501);
        assert_eq!(storage.get_slot(1).unwrap().amount, 5);
    }

    /// 相同物品移动时合并
    #[test]
    fn move_item_merges_same_item() {
        // 手动设置两个格子为相同物品，避免 add_item 自动堆叠
        let mut storage = Storage::new(10);
        storage.get_slot_mut(0).unwrap().item_id = 501;
        storage.get_slot_mut(0).unwrap().amount = 5;
        storage.get_slot_mut(1).unwrap().item_id = 501;
        storage.get_slot_mut(1).unwrap().amount = 3;

        assert!(storage.move_item(0, 1));
        assert!(storage.get_slot(0).unwrap().is_empty());
        assert_eq!(storage.get_slot(1).unwrap().amount, 8);
    }

    /// 移动到自身是空操作
    #[test]
    fn move_item_same_index_is_noop() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5);
        assert!(storage.move_item(0, 0));
        assert_eq!(storage.get_slot(0).unwrap().item_id, 501);
    }

    /// 越界索引返回 false
    #[test]
    fn move_item_returns_false_for_out_of_bounds() {
        let mut storage = Storage::new(10);
        assert!(!storage.move_item(0, 10));
        assert!(!storage.move_item(10, 0));
    }

    /// 移动到空格子
    #[test]
    fn move_item_to_empty_slot() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5); // slot 0
        assert!(storage.move_item(0, 5));
        assert!(storage.get_slot(0).unwrap().is_empty());
        assert_eq!(storage.get_slot(5).unwrap().item_id, 501);
        assert_eq!(storage.get_slot(5).unwrap().amount, 5);
    }

    // ========== find_item_slot 测试 ==========

    /// 查找已存在的物品
    #[test]
    fn find_item_slot_finds_existing_item() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5);
        storage.add_item(502, 3);
        let slot = storage.find_item_slot(502).unwrap();
        assert_eq!(slot.index, 1);
        assert_eq!(slot.amount, 3);
    }

    /// 查找不存在的物品返回 None
    #[test]
    fn find_item_slot_returns_none_for_missing() {
        let storage = Storage::new(10);
        assert!(storage.find_item_slot(501).is_none());
    }

    // ========== 序列化/反序列化测试 ==========

    /// to_db_format 只包含非空格子
    #[test]
    fn to_db_format_only_includes_non_empty() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5);
        storage.add_item(502, 3);
        let db_data = storage.to_db_format();
        assert_eq!(db_data.len(), 2);
        assert_eq!(db_data[0].1, 501); // item_id
        assert_eq!(db_data[1].1, 502);
    }

    /// from_db_format 往返一致性
    #[test]
    fn from_db_format_roundtrip() {
        let mut storage = Storage::new(10).with_char_id(42);
        storage.add_item(501, 5);
        storage.add_item(502, 3);
        let db_data = storage.to_db_format();

        let restored = Storage::from_db_format(42, 10, db_data);
        assert_eq!(restored.char_id(), 42);
        assert_eq!(restored.get_slot(0).unwrap().item_id, 501);
        assert_eq!(restored.get_slot(0).unwrap().amount, 5);
        assert_eq!(restored.get_slot(1).unwrap().item_id, 502);
    }

    /// from_slots 往返一致性（注意：from_slots 按 vec 位置放置，不按 index 字段）
    #[test]
    fn from_slots_roundtrip() {
        let slots = vec![
            StorageSlot {
                index: 0,
                item_id: 501,
                amount: 5,
                identified: true,
                refine: 0,
                cards: [0; 4],
            },
            StorageSlot {
                index: 1,
                item_id: 601,
                amount: 1,
                identified: true,
                refine: 7,
                cards: [4001, 0, 0, 0],
            },
        ];
        let storage = Storage::from_slots(42, 10, slots);
        assert_eq!(storage.used_count(), 2);
        assert_eq!(storage.get_slot(0).unwrap().item_id, 501);
        assert!(storage.get_slot(2).unwrap().is_empty());
        assert_eq!(storage.get_slot(1).unwrap().item_id, 601);
        assert_eq!(storage.get_slot(1).unwrap().refine, 7);
    }

    /// from_slots 忽略超出 max_size 的数据
    #[test]
    fn from_slots_ignores_out_of_bounds() {
        let slots = vec![
            StorageSlot {
                index: 0,
                item_id: 501,
                amount: 1,
                identified: true,
                refine: 0,
                cards: [0; 4],
            },
            StorageSlot {
                index: 1,
                item_id: 502,
                amount: 1,
                identified: true,
                refine: 0,
                cards: [0; 4],
            },
            StorageSlot {
                index: 2,
                item_id: 503,
                amount: 1,
                identified: true,
                refine: 0,
                cards: [0; 4],
            },
        ];
        // max_size=2，第三个元素应被忽略
        let storage = Storage::from_slots(1, 2, slots);
        assert_eq!(storage.used_count(), 2);
    }

    // ========== slots() 测试 ==========

    /// slots() 返回所有格子
    #[test]
    fn slots_returns_all_slots() {
        let storage = Storage::new(5);
        assert_eq!(storage.slots().len(), 5);
    }

    // ========== set_slot 测试 ==========

    /// set_slot 替换格子数据
    #[test]
    fn set_slot_replaces_slot_data() {
        let mut storage = Storage::new(10);
        let slot = StorageSlot {
            index: 3,
            item_id: 601,
            amount: 1,
            identified: true,
            refine: 10,
            cards: [4001, 4002, 4003, 4004],
        };
        storage.set_slot(3, slot);
        assert_eq!(storage.get_slot(3).unwrap().item_id, 601);
        assert_eq!(storage.get_slot(3).unwrap().refine, 10);
    }

    /// set_slot 越界不 panic
    #[test]
    fn set_slot_ignores_out_of_bounds() {
        let mut storage = Storage::new(5);
        let slot = StorageSlot::empty(0);
        // 不应该 panic
        storage.set_slot(10, slot);
    }

    // ========== add_item_full 测试 ==========

    /// 装备入库带精炼和卡片
    #[test]
    fn add_item_with_refine_and_cards() {
        let mut storage = Storage::new(10);
        // 装备入库，带精炼和卡片
        assert!(storage.add_item_full(1101, 1, true, 7, [4001, 4002, 0, 0]));
        let slot = storage.get_slot(0).unwrap();
        assert_eq!(slot.item_id, 1101);
        assert_eq!(slot.amount, 1);
        assert!(slot.identified);
        assert_eq!(slot.refine, 7);
        assert_eq!(slot.cards, [4001, 4002, 0, 0]);
    }

    /// 消耗品堆叠不受 refine/cards 影响
    #[test]
    fn add_item_full_stacks_same_item_ignoring_refine() {
        let mut storage = Storage::new(10);
        storage.add_item_full(501, 10, true, 0, [0; 4]);
        // 消耗品堆叠（refine/cards 不影响堆叠判断）
        assert!(storage.add_item_full(501, 5, true, 0, [0; 4]));
        assert_eq!(storage.used_count(), 1);
        assert_eq!(storage.get_slot(0).unwrap().amount, 15);
    }

    /// 装备不参与堆叠（不同精炼放在不同格子）
    #[test]
    fn add_item_equipment_does_not_stack() {
        let mut storage = Storage::new(10);
        // 装备通常 amount=1，带不同精炼，应该放在不同格子
        storage.add_item_full(1101, 1, true, 7, [4001, 0, 0, 0]);
        storage.add_item_full(1101, 1, true, 10, [4002, 0, 0, 0]);
        assert_eq!(storage.used_count(), 2);
        assert_eq!(storage.get_slot(0).unwrap().refine, 7);
        assert_eq!(storage.get_slot(1).unwrap().refine, 10);
    }

    /// 原有 add_item 接口保持兼容
    #[test]
    fn add_item_legacy_still_works() {
        let mut storage = Storage::new(10);
        assert!(storage.add_item(501, 10));
        assert_eq!(storage.get_slot(0).unwrap().item_id, 501);
        assert_eq!(storage.get_slot(0).unwrap().amount, 10);
        // 默认值
        assert_eq!(storage.get_slot(0).unwrap().refine, 0);
        assert_eq!(storage.get_slot(0).unwrap().cards, [0; 4]);
    }
}
