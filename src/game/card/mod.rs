pub mod data;

use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

pub use data::*;

/// 每个装备最大卡片槽数（RO标准是4）
const MAX_CARD_SLOTS: usize = 4;

/// 卡片管理器
pub struct CardManager {
    /// 卡片数据库
    database: RwLock<CardDatabase>,
    /// 玩家装备卡片映射: player_id -> (equip_slot -> Vec<CardSlot>)
    player_cards: RwLock<HashMap<Uuid, HashMap<String, Vec<CardSlot>>>>,
}

impl CardManager {
    pub fn new() -> Self {
        let mut database = CardDatabase::new();
        database.register_default_cards();

        Self {
            database: RwLock::new(database),
            player_cards: RwLock::new(HashMap::new()),
        }
    }

    /// 获取卡片数据
    pub fn get_card(&self, card_id: u32) -> Option<CardData> {
        self.database.read().get_card(card_id).cloned()
    }

    /// 检查卡片是否可以插入指定装备位置
    pub fn can_insert(&self, card_id: u32, equip_slot: EquipSlotForCard) -> bool {
        if let Some(card) = self.get_card(card_id) {
            card.equip_slots.contains(&EquipSlotForCard::All)
                || card.equip_slots.contains(&equip_slot)
        } else {
            false
        }
    }

    /// 向装备插入卡片
    pub fn insert_card(
        &self,
        player_id: Uuid,
        equip_key: &str,
        card_id: u32,
        equip_slot: EquipSlotForCard,
    ) -> Result<usize, String> {
        let card = self.get_card(card_id).ok_or("Card not found")?;

        if !self.can_insert(card_id, equip_slot) {
            return Err(format!(
                "Card {} cannot be inserted into this equipment",
                card_id
            ));
        }

        let mut player_cards = self.player_cards.write();
        let equip_cards = player_cards
            .entry(player_id)
            .or_default()
            .entry(equip_key.to_string())
            .or_default();

        if equip_cards.len() >= MAX_CARD_SLOTS {
            return Err("No empty card slots".to_string());
        }

        let slot_index = equip_cards.len();
        equip_cards.push(CardSlot {
            slot_index,
            card_id,
            card_name: card.name,
        });

        Ok(slot_index)
    }

    /// 移除装备上的卡片
    pub fn remove_card(
        &self,
        player_id: Uuid,
        equip_key: &str,
        slot_index: usize,
    ) -> Option<CardSlot> {
        let mut player_cards = self.player_cards.write();
        let equip_cards = player_cards.get_mut(&player_id)?.get_mut(equip_key)?;

        if slot_index < equip_cards.len() {
            Some(equip_cards.remove(slot_index))
        } else {
            None
        }
    }

    /// 获取装备上的所有卡片
    pub fn get_equipment_cards(&self, player_id: &Uuid, equip_key: &str) -> Vec<CardSlot> {
        self.player_cards
            .read()
            .get(player_id)
            .and_then(|equips| equips.get(equip_key))
            .cloned()
            .unwrap_or_default()
    }

    /// 获取装备上所有卡片效果的总和
    pub fn get_equipment_card_effects(&self, player_id: &Uuid, equip_key: &str) -> Vec<CardEffect> {
        let cards = self.get_equipment_cards(player_id, equip_key);
        let database = self.database.read();
        let mut effects = Vec::new();

        for slot in &cards {
            if let Some(card) = database.get_card(slot.card_id) {
                effects.extend(card.effects.clone());
            }
        }

        effects
    }

    /// 获取玩家所有已插入卡片的装备摘要
    pub fn get_player_card_summary(&self, player_id: &Uuid) -> Vec<(String, Vec<CardSlot>)> {
        self.player_cards
            .read()
            .get(player_id)
            .map(|equips| equips.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }

    /// 清理玩家数据
    pub fn cleanup_player(&self, player_id: &Uuid) {
        self.player_cards.write().remove(player_id);
    }

    /// 数据库中的卡片数
    pub fn card_count(&self) -> usize {
        self.database.read().card_count()
    }
}

impl Default for CardManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_database() {
        let db = CardDatabase::new();
        let mut db = db;
        db.register_default_cards();
        assert!(db.card_count() >= 10);
    }

    #[test]
    fn test_insert_and_remove_card() {
        let manager = CardManager::new();
        let player_id = Uuid::new_v4();

        // Poring Card (4001) can be inserted into Armor
        assert!(manager.can_insert(4001, EquipSlotForCard::Armor));

        let result = manager.insert_card(
            player_id,
            "armor",
            4001, // Poring Card
            EquipSlotForCard::Armor,
        );
        assert!(result.is_ok());

        let cards = manager.get_equipment_cards(&player_id, "armor");
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].card_id, 4001);

        // Remove
        let removed = manager.remove_card(player_id, "armor", 0);
        assert!(removed.is_some());

        let cards = manager.get_equipment_cards(&player_id, "armor");
        assert!(cards.is_empty());
    }

    #[test]
    fn test_cannot_insert_wrong_slot() {
        let manager = CardManager::new();
        let _player_id = Uuid::new_v4();

        // Hydra Card (4035) only for Weapon
        assert!(manager.can_insert(4035, EquipSlotForCard::Weapon));
        assert!(!manager.can_insert(4035, EquipSlotForCard::Armor));
    }

    #[test]
    fn test_equipment_card_effects() {
        let manager = CardManager::new();
        let player_id = Uuid::new_v4();

        // Insert Hydra Card into weapon
        manager
            .insert_card(
                player_id,
                "weapon",
                4035, // Hydra Card - 20% DemiHuman damage
                EquipSlotForCard::Weapon,
            )
            .unwrap();

        let effects = manager.get_equipment_card_effects(&player_id, "weapon");
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            &effects[0],
            CardEffect::IncreaseDamage {
                race: MonsterRace::DemiHuman,
                percent: 20
            }
        ));
    }
}
