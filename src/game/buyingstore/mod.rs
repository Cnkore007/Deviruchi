//! 收购商店系统
//!
//! 对应 rAthena 的 `src/map/buyingstore.cpp`，提供玩家收购商店功能。
//!
//! 收购商店与摆摊（vending）相反：玩家发布想要购买的物品和价格，
//! 其他玩家可以出售物品给收购商店。

use std::collections::HashMap;
use parking_lot::RwLock;
use uuid::Uuid;

/// 收购商品条目
#[derive(Debug, Clone)]
pub struct BuyingStoreItem {
    /// 物品 ID
    pub item_id: u16,
    /// 单价
    pub price: u32,
    /// 已收购数量
    pub bought: u16,
    /// 最大收购数量
    pub max_quantity: u16,
}

/// 收购商店
#[derive(Debug, Clone)]
pub struct BuyingStore {
    /// 商店 ID
    pub id: u32,
    /// 店主 ID
    pub owner_id: Uuid,
    /// 商店标题
    pub title: String,
    /// 收购商品列表
    pub items: Vec<BuyingStoreItem>,
    /// 店主剩余金币
    pub zeny: u32,
    /// 是否自动交易（离线摆摊）
    pub autotrade: bool,
    /// 创建时间
    pub created_at: i64,
}

impl BuyingStore {
    /// 创建新的收购商店
    pub fn new(id: u32, owner_id: Uuid, title: String, zeny: u32) -> Self {
        Self {
            id,
            owner_id,
            title,
            items: Vec::new(),
            zeny,
            autotrade: false,
            created_at: Self::current_timestamp(),
        }
    }

    /// 添加收购商品
    pub fn add_item(&mut self, item_id: u16, price: u32, max_quantity: u16) -> bool {
        // 检查是否已存在
        if self.items.iter().any(|i| i.item_id == item_id) {
            return false;
        }
        self.items.push(BuyingStoreItem {
            item_id,
            price,
            bought: 0,
            max_quantity,
        });
        true
    }

    /// 获取商品剩余收购数量
    pub fn remaining_quantity(&self, item_id: u16) -> u16 {
        self.items
            .iter()
            .find(|i| i.item_id == item_id)
            .map(|i| i.max_quantity.saturating_sub(i.bought))
            .unwrap_or(0)
    }

    /// 检查是否还能收购指定物品
    pub fn can_buy(&self, item_id: u16, quantity: u16) -> bool {
        self.remaining_quantity(item_id) >= quantity
    }

    /// 检查店主是否有足够金币
    pub fn has_zeny(&self, amount: u32) -> bool {
        self.zeny >= amount
    }

    /// 执行收购
    pub fn buy_item(&mut self, item_id: u16, quantity: u16, total_price: u32) -> bool {
        // 找到商品
        let item = match self.items.iter_mut().find(|i| i.item_id == item_id) {
            Some(i) => i,
            None => return false,
        };

        // 检查数量
        let remaining = item.max_quantity.saturating_sub(item.bought);
        if remaining < quantity {
            return false;
        }

        // 检查金币
        if self.zeny < total_price {
            return false;
        }

        // 执行收购
        item.bought += quantity;
        self.zeny -= total_price;

        true
    }

    /// 检查是否所有商品都已收购满
    pub fn is_full(&self) -> bool {
        self.items.iter().all(|i| i.bought >= i.max_quantity)
    }

    /// 获取当前时间戳
    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

/// 收购商店操作结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuyingStoreResult {
    /// 操作成功
    Success,
    /// 商店不存在
    NotFound,
    /// 商品不存在
    ItemNotFound,
    /// 库存已满
    Full,
    /// 金币不足
    NotEnoughZeny,
    /// 数量不足
    NotEnoughQuantity,
    /// 重量超限
    Overweight,
    /// 店主不在线
    OwnerOffline,
    /// 重复商品
    DuplicateItem,
}

/// 收购商店管理器
pub struct BuyingStoreManager {
    /// 商店列表 (store_id -> BuyingStore)
    stores: RwLock<HashMap<u32, BuyingStore>>,
    /// 玩家商店映射 (player_id -> store_id)
    player_stores: RwLock<HashMap<Uuid, u32>>,
    /// 下一个商店 ID
    next_id: RwLock<u32>,
}

impl BuyingStoreManager {
    /// 创建空的管理器
    pub fn new() -> Self {
        Self {
            stores: RwLock::new(HashMap::new()),
            player_stores: RwLock::new(HashMap::new()),
            next_id: RwLock::new(1),
        }
    }

    /// 创建收购商店
    pub fn create_store(
        &self,
        owner_id: Uuid,
        title: String,
        zeny: u32,
    ) -> Result<u32, BuyingStoreResult> {
        // 检查是否已有商店
        if self.player_stores.read().contains_key(&owner_id) {
            return Err(BuyingStoreResult::DuplicateItem);
        }

        let mut next_id = self.next_id.write();
        let store_id = *next_id;
        *next_id += 1;

        let store = BuyingStore::new(store_id, owner_id, title, zeny);

        self.stores.write().insert(store_id, store);
        self.player_stores.write().insert(owner_id, store_id);

        tracing::info!(
            "BuyingStore {} created by player {:?}",
            store_id,
            owner_id
        );
        Ok(store_id)
    }

    /// 添加收购商品
    pub fn add_item(
        &self,
        store_id: u32,
        item_id: u16,
        price: u32,
        max_quantity: u16,
    ) -> BuyingStoreResult {
        let mut stores = self.stores.write();
        let store = match stores.get_mut(&store_id) {
            Some(s) => s,
            None => return BuyingStoreResult::NotFound,
        };

        if store.add_item(item_id, price, max_quantity) {
            BuyingStoreResult::Success
        } else {
            BuyingStoreResult::DuplicateItem
        }
    }

    /// 关闭收购商店
    pub fn close_store(&self, store_id: u32, owner_id: &Uuid) -> BuyingStoreResult {
        let mut stores = self.stores.write();
        let store = match stores.get(&store_id) {
            Some(s) => s,
            None => return BuyingStoreResult::NotFound,
        };

        if store.owner_id != *owner_id {
            return BuyingStoreResult::NotFound;
        }

        stores.remove(&store_id);
        self.player_stores.write().remove(owner_id);

        tracing::info!("BuyingStore {} closed", store_id);
        BuyingStoreResult::Success
    }

    /// 出售物品给收购商店
    pub fn sell_to_store(
        &self,
        store_id: u32,
        seller_id: Uuid,
        item_id: u16,
        quantity: u16,
    ) -> BuyingStoreResult {
        let mut stores = self.stores.write();
        let store = match stores.get_mut(&store_id) {
            Some(s) => s,
            None => return BuyingStoreResult::NotFound,
        };

        // 不能卖给自己
        if store.owner_id == seller_id {
            return BuyingStoreResult::NotFound;
        }

        // 检查收购数量
        if !store.can_buy(item_id, quantity) {
            return BuyingStoreResult::NotEnoughQuantity;
        }

        // 计算总价
        let item = match store.items.iter().find(|i| i.item_id == item_id) {
            Some(item) => item,
            None => return BuyingStoreResult::ItemNotFound,
        };
        let total_price = item.price * quantity as u32;

        // 检查金币
        if !store.has_zeny(total_price) {
            return BuyingStoreResult::NotEnoughZeny;
        }

        // 执行收购
        store.buy_item(item_id, quantity, total_price);

        tracing::info!(
            "Player {:?} sold {}x item {} to BuyingStore {} for {} zeny",
            seller_id,
            quantity,
            item_id,
            store_id,
            total_price
        );

        BuyingStoreResult::Success
    }

    /// 搜索收购商店
    pub fn search_stores(&self, item_id: u16) -> Vec<(u32, u32, u16)> {
        let stores = self.stores.read();
        stores
            .values()
            .filter_map(|store| {
                store.items.iter().find(|i| i.item_id == item_id).map(|i| {
                    let remaining = i.max_quantity.saturating_sub(i.bought);
                    (store.id, i.price, remaining)
                })
            })
            .collect()
    }

    /// 获取商店信息
    pub fn get_store(&self, store_id: u32) -> Option<BuyingStore> {
        self.stores.read().get(&store_id).cloned()
    }

    /// 获取玩家的商店
    pub fn get_player_store(&self, player_id: &Uuid) -> Option<u32> {
        self.player_stores.read().get(player_id).copied()
    }

    /// 清理所有商店
    pub fn clear(&self) {
        self.stores.write().clear();
        self.player_stores.write().clear();
    }

    /// 获取商店总数
    pub fn store_count(&self) -> usize {
        self.stores.read().len()
    }
}

impl Default for BuyingStoreManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buying_store_create() {
        let manager = BuyingStoreManager::new();
        let owner = Uuid::new_v4();

        let store_id = manager.create_store(owner, "Buying Ores".to_string(), 100000).unwrap();
        assert!(store_id > 0);
        assert_eq!(manager.store_count(), 1);
    }

    #[test]
    fn test_buying_store_add_item() {
        let manager = BuyingStoreManager::new();
        let owner = Uuid::new_v4();

        let store_id = manager.create_store(owner, "Test".to_string(), 100000).unwrap();

        let result = manager.add_item(store_id, 1001, 1000, 100);
        assert_eq!(result, BuyingStoreResult::Success);

        // 重复商品
        let result = manager.add_item(store_id, 1001, 2000, 50);
        assert_eq!(result, BuyingStoreResult::DuplicateItem);
    }

    #[test]
    fn test_buying_store_sell() {
        let manager = BuyingStoreManager::new();
        let owner = Uuid::new_v4();
        let seller = Uuid::new_v4();

        let store_id = manager.create_store(owner, "Test".to_string(), 100000).unwrap();
        manager.add_item(store_id, 1001, 1000, 100);

        let result = manager.sell_to_store(store_id, seller, 1001, 10);
        assert_eq!(result, BuyingStoreResult::Success);

        let store = manager.get_store(store_id).unwrap();
        assert_eq!(store.items[0].bought, 10);
        assert_eq!(store.zeny, 100000 - 10000);
    }

    #[test]
    fn test_buying_store_cannot_sell_to_self() {
        let manager = BuyingStoreManager::new();
        let owner = Uuid::new_v4();

        let store_id = manager.create_store(owner, "Test".to_string(), 100000).unwrap();
        manager.add_item(store_id, 1001, 1000, 100);

        let result = manager.sell_to_store(store_id, owner, 1001, 10);
        assert_eq!(result, BuyingStoreResult::NotFound);
    }

    #[test]
    fn test_buying_store_not_enough_zeny() {
        let manager = BuyingStoreManager::new();
        let owner = Uuid::new_v4();
        let seller = Uuid::new_v4();

        let store_id = manager.create_store(owner, "Test".to_string(), 500).unwrap();
        manager.add_item(store_id, 1001, 1000, 100);

        let result = manager.sell_to_store(store_id, seller, 1001, 1);
        assert_eq!(result, BuyingStoreResult::NotEnoughZeny);
    }

    #[test]
    fn test_buying_store_full() {
        let manager = BuyingStoreManager::new();
        let owner = Uuid::new_v4();
        let seller = Uuid::new_v4();

        let store_id = manager.create_store(owner, "Test".to_string(), 100000).unwrap();
        manager.add_item(store_id, 1001, 1000, 2);

        manager.sell_to_store(store_id, seller, 1001, 1);
        manager.sell_to_store(store_id, seller, 1001, 1);

        let result = manager.sell_to_store(store_id, seller, 1001, 1);
        assert_eq!(result, BuyingStoreResult::NotEnoughQuantity);
    }

    #[test]
    fn test_buying_store_close() {
        let manager = BuyingStoreManager::new();
        let owner = Uuid::new_v4();

        let store_id = manager.create_store(owner, "Test".to_string(), 100000).unwrap();

        let result = manager.close_store(store_id, &owner);
        assert_eq!(result, BuyingStoreResult::Success);
        assert_eq!(manager.store_count(), 0);
    }

    #[test]
    fn test_buying_store_search() {
        let manager = BuyingStoreManager::new();
        let owner1 = Uuid::new_v4();
        let owner2 = Uuid::new_v4();

        let store1 = manager.create_store(owner1, "Store1".to_string(), 100000).unwrap();
        let store2 = manager.create_store(owner2, "Store2".to_string(), 200000).unwrap();

        manager.add_item(store1, 1001, 1000, 100);
        manager.add_item(store2, 1001, 2000, 50);
        manager.add_item(store2, 1002, 3000, 30);

        let results = manager.search_stores(1001);
        assert_eq!(results.len(), 2);

        let results = manager.search_stores(1002);
        assert_eq!(results.len(), 1);
    }
}
