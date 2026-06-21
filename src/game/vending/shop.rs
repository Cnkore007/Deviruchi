//! 摆摊商店结构

use super::error::VendingError;
use crate::game::item::Inventory;
use crate::game::map::Player;
use std::time::Instant;
use uuid::Uuid;

/// 单个商店物品
#[derive(Debug, Clone)]
pub struct ShopItem {
    /// 物品在背包中的栏位
    pub slot_index: u16,
    /// 物品ID
    pub item_id: u16,
    /// 出售数量
    pub amount: u16,
    /// 单价 (Zeny)
    pub price_per_unit: u32,
}

impl ShopItem {
    /// 创建新的商店物品
    pub fn new(slot_index: u16, item_id: u16, amount: u16, price_per_unit: u32) -> Self {
        Self {
            slot_index,
            item_id,
            amount,
            price_per_unit,
        }
    }

    /// 计算总价
    pub fn total_price(&self) -> u64 {
        (self.price_per_unit as u64) * (self.amount as u64)
    }
}

/// 摆摊商店
#[derive(Debug, Clone)]
pub struct VendingShop {
    /// 商店唯一ID
    pub shop_id: Uuid,
    /// 店主玩家ID
    pub owner_id: Uuid,
    /// 店主名称
    pub owner_name: String,
    /// 商店标题
    pub shop_title: String,
    /// 商店位置
    pub position: (u16, u16),
    /// 所在地图
    pub map_name: String,
    /// 出售的物品列表
    pub items: Vec<ShopItem>,
    /// 创建时间
    pub created_at: Instant,
    /// 商店是否开放
    pub is_open: bool,
}

/// 最大商店物品数量
pub const MAX_SHOP_ITEMS: usize = 10;

impl VendingShop {
    /// 创建新的摆摊商店
    pub fn new(owner: &Player, title: &str) -> Self {
        let (x, y) = owner.get_position();
        Self {
            shop_id: Uuid::new_v4(),
            owner_id: owner.id,
            owner_name: owner.name.clone(),
            shop_title: title.to_string(),
            position: (x, y),
            map_name: owner.map_name.clone(),
            items: Vec::new(),
            created_at: Instant::now(),
            is_open: true,
        }
    }

    /// 添加物品到商店
    pub fn add_item(&mut self, slot_index: u16, item_id: u16, amount: u16, price: u32) -> bool {
        if self.items.len() >= MAX_SHOP_ITEMS {
            return false;
        }

        // 检查是否已存在该栏位的物品
        if self.items.iter().any(|i| i.slot_index == slot_index) {
            return false;
        }

        let item = ShopItem::new(slot_index, item_id, amount, price);
        self.items.push(item);
        true
    }

    /// 移除商店物品
    pub fn remove_item(&mut self, slot_index: u16) -> bool {
        let initial_len = self.items.len();
        self.items.retain(|i| i.slot_index != slot_index);
        self.items.len() < initial_len
    }

    /// 更新物品价格
    pub fn update_price(&mut self, slot_index: u16, new_price: u32) -> bool {
        if let Some(item) = self.items.iter_mut().find(|i| i.slot_index == slot_index) {
            item.price_per_unit = new_price;
            true
        } else {
            false
        }
    }

    /// 根据索引获取物品
    pub fn get_item(&self, index: usize) -> Option<&ShopItem> {
        self.items.get(index)
    }

    /// 购买物品
    pub fn buy_item(
        &mut self,
        item_index: usize,
        amount: u16,
        buyer: &Player,
        inventory: &mut Inventory,
    ) -> Result<u64, VendingError> {
        // 检查商店是否开放
        if !self.is_open {
            return Err(VendingError::ShopNotOpen);
        }

        // 检查是否购买自己的商店
        if buyer.id == self.owner_id {
            return Err(VendingError::CannotBuyOwnShop);
        }

        // 检查地图是否一致
        if buyer.map_name != self.map_name {
            return Err(VendingError::MapMismatch);
        }

        // 获取物品
        let item = self
            .items
            .get_mut(item_index)
            .ok_or(VendingError::ItemNotFound)?;

        // 检查数量
        if amount > item.amount {
            return Err(VendingError::ExceededMaxAmount);
        }

        // 计算总价
        let total_price = (item.price_per_unit as u64) * (amount as u64);

        // 检查买家Zeny
        if !crate::game::zeny::ZenyManager::can_spend(buyer, total_price as u32) {
            return Err(VendingError::NotEnoughZeny);
        }

        // 扣除买家Zeny
        if !crate::game::zeny::ZenyManager::sub(buyer, total_price as u32) {
            return Err(VendingError::NotEnoughZeny);
        }

        // 从背包移除物品
        if !inventory.remove_item(item.slot_index as u8, amount) {
            // 回滚Zeny
            crate::game::zeny::ZenyManager::add(buyer, total_price as u32);
            return Err(VendingError::NotEnoughItems);
        }

        // 添加物品到买家背包
        if !inventory.add_item(item.item_id, amount) {
            // 回滚
            inventory.add_item(item.item_id, amount);
            crate::game::zeny::ZenyManager::add(buyer, total_price as u32);
            return Err(VendingError::InventoryFull);
        }

        // 更新商店物品数量
        item.amount -= amount;

        // 如果物品卖完，移除该物品
        if item.amount == 0 {
            self.items.remove(item_index);
        }

        Ok(total_price)
    }

    /// 计算商店总价值
    pub fn total_value(&self) -> u64 {
        self.items.iter().map(|i| i.total_price()).sum()
    }

    /// 获取物品数量
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// 检查商店是否为空
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 开启商店
    pub fn open(&mut self) {
        self.is_open = true;
    }

    /// 关闭商店
    pub fn close(&mut self) {
        self.is_open = false;
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
            name: "ShopOwner".to_string(),
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
    fn test_shop_creation() {
        let player = create_test_player();
        let shop = VendingShop::new(&player, "My Shop");

        assert_eq!(shop.owner_name, "ShopOwner");
        assert_eq!(shop.shop_title, "My Shop");
        assert!(shop.is_open);
        assert!(shop.is_empty());
    }

    #[test]
    fn test_add_item() {
        let player = create_test_player();
        let mut shop = VendingShop::new(&player, "Test Shop");

        assert!(shop.add_item(0, 501, 10, 100));
        assert_eq!(shop.item_count(), 1);

        // 测试不能添加超过最大数量
        for i in 1..=MAX_SHOP_ITEMS {
            shop.add_item(i as u16, 501, 10, 100);
        }
        assert_eq!(shop.item_count(), MAX_SHOP_ITEMS);

        // 添加第11个应该失败
        assert!(!shop.add_item(11, 501, 10, 100));
    }

    #[test]
    fn test_remove_item() {
        let player = create_test_player();
        let mut shop = VendingShop::new(&player, "Test Shop");

        shop.add_item(0, 501, 10, 100);
        assert!(shop.remove_item(0));
        assert!(!shop.remove_item(0));
    }

    #[test]
    fn test_total_value() {
        let player = create_test_player();
        let mut shop = VendingShop::new(&player, "Test Shop");

        shop.add_item(0, 501, 10, 100); // 10 * 100 = 1000
        shop.add_item(1, 502, 5, 200); // 5 * 200 = 1000

        assert_eq!(shop.total_value(), 2000);
    }
}
