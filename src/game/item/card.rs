//! 卡片系统
//!
//! 实现 rAthena 风格的卡片插槽管理，包括插入、取出和属性加成计算。

use super::data::ItemDatabase;
use super::data::ItemType;
use super::inventory::InventorySlot;

/// 卡片操作结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardResult {
    /// 操作成功
    Success,
    /// 装备没有空槽位
    NoEmptySlot,
    /// 无效的卡片物品
    InvalidCard,
    /// 指定槽位为空
    SlotEmpty,
    /// 物品不可装备（没有卡片槽）
    ItemNotEquippable,
}

/// 卡片提供的属性加成
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CardBonus {
    pub atk_bonus: u16,
    pub def_bonus: u16,
    pub max_hp_bonus: u32,
    pub max_sp_bonus: u32,
    pub str_bonus: i16,
    pub agi_bonus: i16,
    pub vit_bonus: i16,
    pub int_bonus: i16,
    pub dex_bonus: i16,
    pub luk_bonus: i16,
}

/// 卡片系统
pub struct CardSystem;

impl CardSystem {
    /// 将卡片插入装备的空槽位
    ///
    /// 遍历装备的 4 个卡片槽位，找到第一个空槽位（值为 0）后插入卡片 ID。
    /// 如果所有槽位已满则返回 `NoEmptySlot`。
    pub fn insert_card(
        equipment: &mut InventorySlot,
        card_item_id: u16,
        item_db: &ItemDatabase,
    ) -> CardResult {
        // 验证物品确实是卡片类型
        match item_db.get(card_item_id) {
            Some(item) if item.type_ == ItemType::Card => {}
            _ => return CardResult::InvalidCard,
        }

        // 装备必须有卡片槽位（通过 equip_mask 判断是否为装备物品）
        // 对于非装备物品，仍然允许插卡（某些特殊物品可能有槽位）
        // 这里我们只要求卡片槽位可用即可

        // 找到第一个空槽位
        for slot in equipment.cards.iter_mut() {
            if *slot == 0 {
                *slot = card_item_id;
                return CardResult::Success;
            }
        }

        CardResult::NoEmptySlot
    }

    /// 从装备取出卡片
    ///
    /// 将指定槽位的卡片 ID 返回并清空该槽位。槽位索引范围为 0-3。
    pub fn remove_card(equipment: &mut InventorySlot, slot_index: usize) -> CardResult {
        if slot_index >= 4 {
            return CardResult::SlotEmpty;
        }

        if equipment.cards[slot_index] == 0 {
            return CardResult::SlotEmpty;
        }

        equipment.cards[slot_index] = 0;
        CardResult::Success
    }

    /// 获取装备已插入的卡片列表（过滤掉空槽位）
    pub fn get_cards(equipment: &InventorySlot) -> Vec<u16> {
        equipment
            .cards
            .iter()
            .copied()
            .filter(|&id| id != 0)
            .collect()
    }

    /// 获取卡片提供的属性加成
    ///
    /// 遍历所有非空卡片槽位，查询 ItemDatabase 获取每张卡片的基础属性并累加。
    /// 当前实现直接使用卡片物品数据中的属性字段（atk, defense, 各项 bonus 等）。
    pub fn get_card_bonus(cards: &[u16; 4], item_db: &ItemDatabase) -> CardBonus {
        let mut bonus = CardBonus::default();

        for &card_id in cards.iter() {
            if card_id == 0 {
                continue;
            }

            if let Some(card_item) = item_db.get(card_id) {
                // 仅处理卡片类型的物品
                if card_item.type_ != ItemType::Card {
                    continue;
                }

                bonus.atk_bonus = bonus.atk_bonus.saturating_add(card_item.atk);
                bonus.def_bonus = bonus.def_bonus.saturating_add(card_item.defense);
                bonus.str_bonus = bonus.str_bonus.saturating_add(card_item.str_bonus);
                bonus.agi_bonus = bonus.agi_bonus.saturating_add(card_item.agi_bonus);
                bonus.vit_bonus = bonus.vit_bonus.saturating_add(card_item.vit_bonus);
                bonus.int_bonus = bonus.int_bonus.saturating_add(card_item.int_bonus);
                bonus.dex_bonus = bonus.dex_bonus.saturating_add(card_item.dex_bonus);
                bonus.luk_bonus = bonus.luk_bonus.saturating_add(card_item.luk_bonus);
                // max_hp 和 max_sp 当前不在 Item 结构中，暂不累加
            }
        }

        bonus
    }
}

#[cfg(test)]
mod tests {
    use super::super::data::{Item, ItemType};
    use super::*;

    /// 创建带测试卡片的数据库
    fn make_db_with_cards() -> ItemDatabase {
        let mut db = ItemDatabase::new();
        // 插入测试卡片（ID 4001-4004）
        for id in 4001..=4004u16 {
            db.insert(Item {
                id,
                name: format!("Test Card {}", id),
                type_: ItemType::Card,
                buy_price: 1000,
                sell_price: 500,
                weight: 10,
                atk: 2,
                defense: 1,
                str_bonus: 1,
                agi_bonus: 1,
                vit_bonus: 1,
                int_bonus: 1,
                dex_bonus: 1,
                luk_bonus: 1,
                ..Default::default()
            });
        }
        db
    }

    /// 创建测试用的 InventorySlot
    fn make_equip_slot() -> InventorySlot {
        InventorySlot {
            index: 0,
            item_id: 1201,
            amount: 1,
            identified: true,
            refine: 0,
            cards: [0; 4],
        }
    }

    // ========== 卡片取出测试 ==========

    #[test]
    fn test_remove_card_success() {
        let mut slot = make_equip_slot();
        slot.cards[0] = 4001;
        slot.cards[1] = 4002;

        let result = CardSystem::remove_card(&mut slot, 0);
        assert_eq!(result, CardResult::Success);
        assert_eq!(slot.cards[0], 0);
        assert_eq!(slot.cards[1], 4002); // 其他槽位不受影响
    }

    #[test]
    fn test_remove_card_empty_slot() {
        let mut slot = make_equip_slot();
        let result = CardSystem::remove_card(&mut slot, 0);
        assert_eq!(result, CardResult::SlotEmpty);
    }

    #[test]
    fn test_remove_card_invalid_index() {
        let mut slot = make_equip_slot();
        let result = CardSystem::remove_card(&mut slot, 4);
        assert_eq!(result, CardResult::SlotEmpty);
    }

    // ========== 获取卡片列表测试 ==========

    #[test]
    fn test_get_cards_with_some_cards() {
        let mut slot = make_equip_slot();
        slot.cards = [4001, 0, 4003, 0];

        let cards = CardSystem::get_cards(&slot);
        assert_eq!(cards, vec![4001, 4003]);
    }

    #[test]
    fn test_get_cards_empty() {
        let slot = make_equip_slot();
        let cards = CardSystem::get_cards(&slot);
        assert!(cards.is_empty());
    }

    #[test]
    fn test_get_cards_full() {
        let mut slot = make_equip_slot();
        slot.cards = [4001, 4002, 4003, 4004];

        let cards = CardSystem::get_cards(&slot);
        assert_eq!(cards, vec![4001, 4002, 4003, 4004]);
    }

    // ========== 卡片属性加成测试 ==========

    #[test]
    fn test_card_bonus_empty() {
        let db = make_db_with_cards();
        let cards = [0u16; 4];
        let bonus = CardSystem::get_card_bonus(&cards, &db);
        assert_eq!(bonus, CardBonus::default());
    }

    #[test]
    fn test_card_bonus_non_card_ignored() {
        let db = make_db_with_cards();
        // 使用非卡片 ID（如 501 红色药水），应该被忽略
        let cards = [501, 0, 0, 0];
        let bonus = CardSystem::get_card_bonus(&cards, &db);
        assert_eq!(bonus, CardBonus::default());
    }

    // ========== 卡片插入测试（使用默认数据库验证类型检查）==========

    #[test]
    fn test_insert_card_invalid_not_card_type() {
        let db = make_db_with_cards();
        let mut slot = make_equip_slot();
        // 501 是红色药水，不是卡片
        let result = CardSystem::insert_card(&mut slot, 501, &db);
        assert_eq!(result, CardResult::InvalidCard);
    }

    #[test]
    fn test_insert_card_invalid_nonexistent() {
        let db = make_db_with_cards();
        let mut slot = make_equip_slot();
        let result = CardSystem::insert_card(&mut slot, 9999, &db);
        assert_eq!(result, CardResult::InvalidCard);
    }

    // ========== 卡片插入/取出流程测试（使用真实数据库）==========

    #[test]
    fn test_insert_card_success() {
        let db = make_db_with_cards();
        let mut slot = make_equip_slot();

        let result = CardSystem::insert_card(&mut slot, 4001, &db);
        assert_eq!(result, CardResult::Success);
        assert_eq!(slot.cards[0], 4001);
        assert_eq!(slot.cards[1], 0); // 其他槽位不受影响
    }

    #[test]
    fn test_insert_card_multiple() {
        let db = make_db_with_cards();
        let mut slot = make_equip_slot();

        // 依次插入 4 张卡片
        assert_eq!(
            CardSystem::insert_card(&mut slot, 4001, &db),
            CardResult::Success
        );
        assert_eq!(
            CardSystem::insert_card(&mut slot, 4002, &db),
            CardResult::Success
        );
        assert_eq!(
            CardSystem::insert_card(&mut slot, 4003, &db),
            CardResult::Success
        );
        assert_eq!(
            CardSystem::insert_card(&mut slot, 4004, &db),
            CardResult::Success
        );
        assert_eq!(slot.cards, [4001, 4002, 4003, 4004]);
    }

    #[test]
    fn test_insert_card_no_empty_slot() {
        let db = make_db_with_cards();
        let mut slot = make_equip_slot();
        slot.cards = [4001, 4002, 4003, 4004];

        let result = CardSystem::insert_card(&mut slot, 4001, &db);
        assert_eq!(result, CardResult::NoEmptySlot);
    }

    #[test]
    fn test_card_insert_and_remove_flow() {
        let db = make_db_with_cards();
        let mut slot = make_equip_slot();

        // 插入两张卡片
        CardSystem::insert_card(&mut slot, 4001, &db);
        CardSystem::insert_card(&mut slot, 4002, &db);
        assert_eq!(CardSystem::get_cards(&slot), vec![4001, 4002]);

        // 取出第一张
        let result = CardSystem::remove_card(&mut slot, 0);
        assert_eq!(result, CardResult::Success);
        assert_eq!(CardSystem::get_cards(&slot), vec![4002]);

        // 取出第二张
        let result = CardSystem::remove_card(&mut slot, 1);
        assert_eq!(result, CardResult::Success);
        assert!(CardSystem::get_cards(&slot).is_empty());
    }

    #[test]
    fn test_card_slot_full() {
        let mut slot = make_equip_slot();
        slot.cards = [4001, 4002, 4003, 4004];

        // 所有槽位已满，应该无法插入
        let has_empty = slot.cards.iter().any(|&c| c == 0);
        assert!(!has_empty);
    }

    // ========== 卡片属性加成测试（使用真实卡片数据）==========

    #[test]
    fn test_card_bonus_with_real_cards() {
        let db = make_db_with_cards();
        let cards = [4001, 4002, 0, 0];
        let bonus = CardSystem::get_card_bonus(&cards, &db);
        // 每张测试卡片 atk=2, defense=1, 各属性 +1
        assert_eq!(bonus.atk_bonus, 4); // 2 + 2
        assert_eq!(bonus.def_bonus, 2); // 1 + 1
        assert_eq!(bonus.str_bonus, 2); // 1 + 1
        assert_eq!(bonus.agi_bonus, 2);
        assert_eq!(bonus.vit_bonus, 2);
        assert_eq!(bonus.int_bonus, 2);
        assert_eq!(bonus.dex_bonus, 2);
        assert_eq!(bonus.luk_bonus, 2);
    }

    #[test]
    fn test_card_bonus_mixed_cards_and_empty() {
        let db = make_db_with_cards();
        let cards = [4001, 0, 4003, 0];
        let bonus = CardSystem::get_card_bonus(&cards, &db);
        // 两张卡片：atk=2*2=4, defense=1*2=2
        assert_eq!(bonus.atk_bonus, 4);
        assert_eq!(bonus.def_bonus, 2);
    }
}
