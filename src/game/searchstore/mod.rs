//! 全服商店搜索系统
//!
//! 对应 rAthena 的 `src/map/searchstore.cpp`，提供跨商店搜索功能。

use uuid::Uuid;

type StoreEntry = (u32, Uuid, String, Vec<(u16, u32, u16)>);

/// 搜索结果条目
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// 商店 ID
    pub store_id: u32,
    /// 商店类型
    pub store_type: StoreType,
    /// 店主 ID
    pub owner_id: Uuid,
    /// 店主名称
    pub owner_name: String,
    /// 商店标题
    pub title: String,
    /// 物品 ID
    pub item_id: u16,
    /// 单价
    pub price: u32,
    /// 剩余数量
    pub quantity: u16,
}

/// 商店类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreType {
    /// 摆摊
    Vending,
    /// 收购商店
    BuyingStore,
}

/// 搜索过滤条件
#[derive(Debug, Clone)]
pub struct SearchFilter {
    /// 物品 ID 列表（空 = 不限）
    pub item_ids: Vec<u16>,
    /// 最低价格（0 = 不限）
    pub min_price: u32,
    /// 最高价格（0 = 不限）
    pub max_price: u32,
    /// 商店类型（None = 不限）
    pub store_type: Option<StoreType>,
}

impl SearchFilter {
    /// 创建空的过滤条件
    pub fn new() -> Self {
        Self {
            item_ids: Vec::new(),
            min_price: 0,
            max_price: 0,
            store_type: None,
        }
    }

    /// 检查价格是否符合过滤条件
    pub fn matches_price(&self, price: u32) -> bool {
        if self.min_price > 0 && price < self.min_price {
            return false;
        }
        if self.max_price > 0 && price > self.max_price {
            return false;
        }
        true
    }

    /// 检查物品 ID 是否符合过滤条件
    pub fn matches_item(&self, item_id: u16) -> bool {
        self.item_ids.is_empty() || self.item_ids.contains(&item_id)
    }
}

impl Default for SearchFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// 商店搜索管理器
///
/// 提供跨摆摊和收购商店的搜索功能。
pub struct SearchStoreManager {
    /// 最大搜索结果数
    max_results: usize,
}

impl SearchStoreManager {
    /// 创建搜索管理器
    pub fn new() -> Self {
        Self {
            max_results: 100,
        }
    }

    /// 设置最大搜索结果数
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// 执行搜索
    ///
    /// 从摆摊和收购商店中搜索符合条件的商品。
    pub fn search(
        &self,
        filter: &SearchFilter,
        vending_stores: &[StoreEntry],
        buying_stores: &[StoreEntry],
    ) -> Vec<SearchResult> {
        let mut results = Vec::new();

        // 搜索摆摊
        if filter.store_type.is_none() || filter.store_type == Some(StoreType::Vending) {
            for (store_id, owner_id, title, items) in vending_stores {
                for (item_id, price, quantity) in items {
                    if !filter.matches_item(*item_id) || !filter.matches_price(*price) {
                        continue;
                    }
                    results.push(SearchResult {
                        store_id: *store_id,
                        store_type: StoreType::Vending,
                        owner_id: *owner_id,
                        owner_name: String::new(),
                        title: title.clone(),
                        item_id: *item_id,
                        price: *price,
                        quantity: *quantity,
                    });
                }
            }
        }

        // 搜索收购商店
        if filter.store_type.is_none() || filter.store_type == Some(StoreType::BuyingStore) {
            for (store_id, owner_id, title, items) in buying_stores {
                for (item_id, price, quantity) in items {
                    if !filter.matches_item(*item_id) || !filter.matches_price(*price) {
                        continue;
                    }
                    results.push(SearchResult {
                        store_id: *store_id,
                        store_type: StoreType::BuyingStore,
                        owner_id: *owner_id,
                        owner_name: String::new(),
                        title: title.clone(),
                        item_id: *item_id,
                        price: *price,
                        quantity: *quantity,
                    });
                }
            }
        }

        // 按价格排序
        results.sort_by_key(|a| a.price);

        // 限制结果数
        results.truncate(self.max_results);

        results
    }
}

impl Default for SearchStoreManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_filter_matches() {
        let mut filter = SearchFilter::new();
        assert!(filter.matches_price(1000));
        assert!(filter.matches_item(1001));

        filter.min_price = 500;
        filter.max_price = 2000;
        assert!(filter.matches_price(1000));
        assert!(!filter.matches_price(400));
        assert!(!filter.matches_price(3000));

        filter.item_ids = vec![1001, 1002];
        assert!(filter.matches_item(1001));
        assert!(!filter.matches_item(1003));
    }

    #[test]
    fn test_search_basic() {
        let manager = SearchStoreManager::new();

        let vending = vec![
            (1, Uuid::new_v4(), "Shop1".to_string(), vec![(1001, 1000, 10)]),
            (2, Uuid::new_v4(), "Shop2".to_string(), vec![(1001, 2000, 5)]),
        ];
        let buying = vec![];

        let filter = SearchFilter::new();
        let results = manager.search(&filter, &vending, &buying);
        assert_eq!(results.len(), 2);
        // 按价格排序
        assert_eq!(results[0].price, 1000);
        assert_eq!(results[1].price, 2000);
    }

    #[test]
    fn test_search_with_filter() {
        let manager = SearchStoreManager::new();

        let vending = vec![
            (1, Uuid::new_v4(), "Shop1".to_string(), vec![(1001, 1000, 10)]),
            (2, Uuid::new_v4(), "Shop2".to_string(), vec![(1002, 2000, 5)]),
        ];
        let buying = vec![];

        let mut filter = SearchFilter::new();
        filter.item_ids = vec![1001];
        let results = manager.search(&filter, &vending, &buying);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_mixed_stores() {
        let manager = SearchStoreManager::new();

        let vending = vec![
            (1, Uuid::new_v4(), "Vending".to_string(), vec![(1001, 1000, 10)]),
        ];
        let buying = vec![
            (2, Uuid::new_v4(), "Buying".to_string(), vec![(1001, 500, 20)]),
        ];

        let filter = SearchFilter::new();
        let results = manager.search(&filter, &vending, &buying);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_limit() {
        let manager = SearchStoreManager::new().with_max_results(1);

        let vending = vec![
            (1, Uuid::new_v4(), "Shop1".to_string(), vec![(1001, 1000, 10)]),
            (2, Uuid::new_v4(), "Shop2".to_string(), vec![(1001, 2000, 5)]),
        ];
        let buying = vec![];

        let filter = SearchFilter::new();
        let results = manager.search(&filter, &vending, &buying);
        assert_eq!(results.len(), 1);
    }
}
