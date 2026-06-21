//! 摆摊管理器

use super::error::VendingError;
use super::shop::{MAX_SHOP_ITEMS, VendingShop};
use crate::game::item::Inventory;
use crate::game::map::Player;
use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

/// 摆摊管理器
pub struct VendingManager {
    /// 所有商店列表
    shops: RwLock<HashMap<Uuid, VendingShop>>,
    /// 玩家ID到商店ID的映射
    player_shops: RwLock<HashMap<Uuid, Uuid>>,
}

impl VendingManager {
    /// 创建新的管理器
    pub fn new() -> Self {
        Self {
            shops: RwLock::new(HashMap::new()),
            player_shops: RwLock::new(HashMap::new()),
        }
    }

    /// 开店
    pub fn open_shop(&self, player: &Player, title: &str) -> Result<VendingShop, VendingError> {
        // 检查是否已有商店
        if self.player_shops.read().contains_key(&player.id) {
            return Err(VendingError::AlreadyHasShop);
        }

        // 创建商店
        let shop = VendingShop::new(player, title);

        // 存储商店
        let shop_id = shop.shop_id;
        self.shops.write().insert(shop_id, shop.clone());
        self.player_shops.write().insert(player.id, shop_id);

        Ok(shop)
    }

    /// 关店
    pub fn close_shop(&self, player_id: Uuid) -> Result<VendingShop, VendingError> {
        let shop_id = match self.player_shops.write().remove(&player_id) {
            Some(id) => id,
            None => return Err(VendingError::NoShop),
        };

        let shop = self
            .shops
            .write()
            .remove(&shop_id)
            .ok_or(VendingError::NoShop)?;

        Ok(shop)
    }

    /// 获取商店
    pub fn get_shop(&self, shop_id: Uuid) -> Option<VendingShop> {
        self.shops.read().get(&shop_id).cloned()
    }

    /// 获取玩家商店
    pub fn get_player_shop(&self, player_id: Uuid) -> Option<VendingShop> {
        let shop_id = *self.player_shops.read().get(&player_id)?;
        self.get_shop(shop_id)
    }

    /// 获取地图上的所有商店
    pub fn get_shops_on_map(&self, map_name: &str) -> Vec<VendingShop> {
        self.shops
            .read()
            .values()
            .filter(|shop| shop.map_name == map_name && shop.is_open)
            .cloned()
            .collect()
    }

    /// 检查玩家是否正在摆摊
    pub fn is_player_shopping(&self, player_id: Uuid) -> bool {
        self.player_shops.read().contains_key(&player_id)
    }

    /// 添加物品到玩家商店
    pub fn add_item_to_shop(
        &self,
        player_id: Uuid,
        slot_index: u16,
        item_id: u16,
        amount: u16,
        price: u32,
    ) -> Result<(), VendingError> {
        let shop_id = match self.player_shops.read().get(&player_id) {
            Some(id) => *id,
            None => return Err(VendingError::NoShop),
        };

        let mut shops = self.shops.write();
        let shop = shops.get_mut(&shop_id).ok_or(VendingError::NoShop)?;

        if shop.items.len() >= MAX_SHOP_ITEMS {
            return Err(VendingError::ShopFull);
        }

        if shop.add_item(slot_index, item_id, amount, price) {
            Ok(())
        } else {
            Err(VendingError::InvalidSlot)
        }
    }

    /// 从玩家商店移除物品
    pub fn remove_item_from_shop(
        &self,
        player_id: Uuid,
        slot_index: u16,
    ) -> Result<(), VendingError> {
        let shop_id = match self.player_shops.read().get(&player_id) {
            Some(id) => *id,
            None => return Err(VendingError::NoShop),
        };

        let mut shops = self.shops.write();
        let shop = shops.get_mut(&shop_id).ok_or(VendingError::NoShop)?;

        if shop.remove_item(slot_index) {
            Ok(())
        } else {
            Err(VendingError::ItemNotFound)
        }
    }

    /// 更新商店物品价格
    pub fn update_item_price(
        &self,
        player_id: Uuid,
        slot_index: u16,
        new_price: u32,
    ) -> Result<(), VendingError> {
        let shop_id = match self.player_shops.read().get(&player_id) {
            Some(id) => *id,
            None => return Err(VendingError::NoShop),
        };

        let mut shops = self.shops.write();
        let shop = shops.get_mut(&shop_id).ok_or(VendingError::NoShop)?;

        if shop.update_price(slot_index, new_price) {
            Ok(())
        } else {
            Err(VendingError::ItemNotFound)
        }
    }

    /// 购买物品
    pub fn buy_item(
        &self,
        shop_id: Uuid,
        item_index: usize,
        amount: u16,
        buyer: &Player,
        inventory: &mut Inventory,
    ) -> Result<u64, VendingError> {
        let mut shops = self.shops.write();
        let shop = shops.get_mut(&shop_id).ok_or(VendingError::NoShop)?;

        shop.buy_item(item_index, amount, buyer, inventory)
    }

    /// 更新商店位置
    pub fn update_shop_position(
        &self,
        player_id: Uuid,
        x: u16,
        y: u16,
    ) -> Result<(), VendingError> {
        let shop_id = match self.player_shops.read().get(&player_id) {
            Some(id) => *id,
            None => return Err(VendingError::NoShop),
        };

        let mut shops = self.shops.write();
        let shop = shops.get_mut(&shop_id).ok_or(VendingError::NoShop)?;

        shop.position = (x, y);
        Ok(())
    }

    /// 获取所有商店数量
    pub fn shop_count(&self) -> usize {
        self.shops.read().len()
    }

    /// 获取地图商店数量
    pub fn shop_count_on_map(&self, map_name: &str) -> usize {
        self.shops
            .read()
            .values()
            .filter(|shop| shop.map_name == map_name && shop.is_open)
            .count()
    }

    /// 关闭地图上所有商店
    pub fn close_all_shops_on_map(&self, map_name: &str) {
        let mut shops = self.shops.write();
        for shop in shops.values_mut() {
            if shop.map_name == map_name {
                shop.close();
            }
        }
    }

    /// 获取商店列表（管理员用）
    pub fn get_all_shops(&self) -> Vec<VendingShop> {
        self.shops.read().values().cloned().collect()
    }
}

impl Default for VendingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Character;

    fn create_test_player() -> Player {
        let char = Character {
            char_id: 1,
            char_num: 0,
            name: "TestPlayer".to_string(),
            class: 0,
            base_level: 1,
            job_level: 1,
            base_exp: 0,
            job_exp: 0,
            zeny: 1000,
            str: 10,
            agi: 10,
            vit: 10,
            int: 10,
            dex: 10,
            luk: 10,
            hp: 100,
            max_hp: 100,
            sp: 50,
            max_sp: 50,
            hair: 0,
            hair_color: 0,
            clothes_color: 0,
            weapon: 0,
            shield: 0,
            head_top: 0,
            head_mid: 0,
            head_bottom: 0,
            last_map: "new_1-1.gat".to_string(),
            last_x: 50,
            last_y: 50,
            save_map: "new_1-1.gat".to_string(),
            save_x: 50,
            save_y: 50,
            delete_timer: 0,
            status_point: 0,
            skill_point: 0,
            created_at: 0,
            updated_at: 0,
        };
        let mut player = Player::from_character(char);
        player.map_name = "new_1-1.gat".to_string();
        player
    }

    #[test]
    fn test_open_and_close_shop() {
        let manager = VendingManager::new();
        let player = create_test_player();

        // 开店
        let _shop = manager.open_shop(&player, "Test Shop").unwrap();
        assert!(manager.is_player_shopping(player.id));
        assert_eq!(
            manager.get_player_shop(player.id).unwrap().shop_title,
            "Test Shop"
        );

        // 关店
        let closed_shop = manager.close_shop(player.id).unwrap();
        assert_eq!(closed_shop.owner_name, "TestPlayer");
        assert!(!manager.is_player_shopping(player.id));
    }

    #[test]
    fn test_cannot_open_multiple_shops() {
        let manager = VendingManager::new();
        let player = create_test_player();

        manager.open_shop(&player, "Shop 1").unwrap();
        let result = manager.open_shop(&player, "Shop 2");
        assert!(matches!(result, Err(VendingError::AlreadyHasShop)));
    }

    #[test]
    fn test_get_shops_on_map() {
        let manager = VendingManager::new();
        let player1 = create_test_player();

        let mut player2 = create_test_player();
        player2.char_id = 2;
        player2.name = "Player2".to_string();
        player2.map_name = "new_1-1.gat".to_string();

        let mut player3 = create_test_player();
        player3.char_id = 3;
        player3.name = "Player3".to_string();
        player3.map_name = "prontera.gat".to_string();

        manager.open_shop(&player1, "Shop 1").unwrap();
        manager.open_shop(&player2, "Shop 2").unwrap();
        manager.open_shop(&player3, "Shop 3").unwrap();

        let map_shops = manager.get_shops_on_map("new_1-1.gat");
        assert_eq!(map_shops.len(), 2);
    }

    #[test]
    fn test_add_and_remove_items() {
        let manager = VendingManager::new();
        let player = create_test_player();

        manager.open_shop(&player, "Test Shop").unwrap();

        // 添加物品
        manager
            .add_item_to_shop(player.id, 0, 501, 10, 100)
            .unwrap();

        let shop = manager.get_player_shop(player.id).unwrap();
        assert_eq!(shop.item_count(), 1);

        // 移除物品
        manager.remove_item_from_shop(player.id, 0).unwrap();

        let shop = manager.get_player_shop(player.id).unwrap();
        assert_eq!(shop.item_count(), 0);
    }
}
