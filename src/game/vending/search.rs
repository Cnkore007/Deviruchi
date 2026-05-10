//! 商店搜索功能

use super::shop::VendingShop;
use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

/// 商店搜索条件
#[derive(Debug, Clone)]
pub struct ShopSearch {
    /// 关键词搜索（商店标题或店主名）
    pub keyword: Option<String>,
    /// 指定地图
    pub map_name: Option<String>,
    /// 指定物品ID
    pub item_id: Option<u16>,
    /// 最高价格限制
    pub max_price: Option<u32>,
}

impl ShopSearch {
    /// 创建新的搜索条件
    pub fn new() -> Self {
        Self {
            keyword: None,
            map_name: None,
            item_id: None,
            max_price: None,
        }
    }

    /// 设置关键词
    pub fn with_keyword(mut self, keyword: &str) -> Self {
        self.keyword = Some(keyword.to_lowercase());
        self
    }

    /// 设置地图
    pub fn with_map(mut self, map_name: &str) -> Self {
        self.map_name = Some(map_name.to_string());
        self
    }

    /// 设置物品ID
    pub fn with_item_id(mut self, item_id: u16) -> Self {
        self.item_id = Some(item_id);
        self
    }

    /// 设置最高价格
    pub fn with_max_price(mut self, max_price: u32) -> Self {
        self.max_price = Some(max_price);
        self
    }

    /// 检查商店是否匹配搜索条件
    pub fn matches(&self, shop: &VendingShop) -> bool {
        // 检查地图
        if let Some(ref map) = self.map_name
            && &shop.map_name != map
        {
            return false;
        }

        // 检查关键词
        if let Some(ref keyword) = self.keyword {
            let title_match = shop.shop_title.to_lowercase().contains(keyword);
            let owner_match = shop.owner_name.to_lowercase().contains(keyword);
            if !title_match && !owner_match {
                return false;
            }
        }

        // 检查物品ID
        if let Some(item_id) = self.item_id {
            let has_item = shop.items.iter().any(|item| item.item_id == item_id);
            if !has_item {
                return false;
            }
        }

        // 检查价格
        if let Some(max_price) = self.max_price {
            let within_price = shop
                .items
                .iter()
                .any(|item| item.price_per_unit <= max_price);
            if !within_price {
                return false;
            }
        }

        true
    }
}

impl Default for ShopSearch {
    fn default() -> Self {
        Self::new()
    }
}

/// 商店搜索结果
#[derive(Debug, Clone)]
pub struct ShopSearchResult {
    /// 商店信息
    pub shop: VendingShop,
    /// 匹配到的物品
    pub matching_items: Vec<usize>, // 物品索引列表
}

/// 商店搜索引擎
pub struct ShopSearchEngine {
    /// 物品ID到商店的索引
    item_index: RwLock<HashMap<u16, Vec<Uuid>>>,
}

impl ShopSearchEngine {
    /// 创建新的搜索引擎
    pub fn new() -> Self {
        Self {
            item_index: RwLock::new(HashMap::new()),
        }
    }

    /// 索引商店
    pub fn index_shop(&self, shop: &VendingShop) {
        let mut index = self.item_index.write();
        for item in &shop.items {
            index.entry(item.item_id).or_default().push(shop.shop_id);
        }
    }

    /// 移除商店索引
    pub fn unindex_shop(&self, shop: &VendingShop) {
        let mut index = self.item_index.write();
        for item in &shop.items {
            if let Some(shop_ids) = index.get_mut(&item.item_id) {
                shop_ids.retain(|id| *id != shop.shop_id);
            }
        }
    }

    /// 搜索包含特定物品的商店ID
    pub fn find_shops_with_item(&self, item_id: u16) -> Vec<Uuid> {
        self.item_index
            .read()
            .get(&item_id)
            .cloned()
            .unwrap_or_default()
    }

    /// 执行搜索
    pub fn search(&self, shops: &[VendingShop], criteria: &ShopSearch) -> Vec<ShopSearchResult> {
        shops
            .iter()
            .filter(|shop| criteria.matches(shop))
            .map(|shop| {
                let matching_items: Vec<usize> = shop
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| {
                        // 如果指定了物品ID，必须匹配
                        if let Some(search_item_id) = criteria.item_id
                            && item.item_id != search_item_id
                        {
                            return false;
                        }
                        // 如果指定了最高价格，必须匹配
                        if let Some(max_price) = criteria.max_price
                            && item.price_per_unit > max_price
                        {
                            return false;
                        }
                        true
                    })
                    .map(|(idx, _)| idx)
                    .collect();

                ShopSearchResult {
                    shop: shop.clone(),
                    matching_items,
                }
            })
            .filter(|result| !result.matching_items.is_empty())
            .collect()
    }
}

impl Default for ShopSearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::map::Player;
    use crate::storage::Character;

    fn create_test_shop(owner_name: &str, title: &str, map: &str) -> VendingShop {
        let char = Character {
            char_id: 1,
            char_num: 0,
            name: owner_name.to_string(),
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
            last_map: map.to_string(),
            last_x: 50,
            last_y: 50,
            save_map: map.to_string(),
            save_x: 50,
            save_y: 50,
            delete_timer: 0,
            status_point: 0,
            skill_point: 0,
            created_at: 0,
            updated_at: 0,
        };
        let mut player = Player::from_character(char);
        player.map_name = map.to_string();
        VendingShop::new(&player, title)
    }

    #[test]
    fn test_search_by_keyword() {
        let shops = vec![
            create_test_shop("Player1", "Potion Shop", "prontera"),
            create_test_shop("Player2", "Weapon Store", "prontera"),
            create_test_shop("Player3", "Potion Paradise", "geffen"),
        ];

        let criteria = ShopSearch::new().with_keyword("potion");
        let results = criteria.filter_shops(&shops);

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_by_map() {
        let shops = vec![
            create_test_shop("Player1", "Shop 1", "prontera"),
            create_test_shop("Player2", "Shop 2", "geffen"),
            create_test_shop("Player3", "Shop 3", "prontera"),
        ];

        let criteria = ShopSearch::new().with_map("prontera");
        let results = criteria.filter_shops(&shops);

        assert_eq!(results.len(), 2);
    }

    impl ShopSearch {
        fn filter_shops(&self, shops: &[VendingShop]) -> Vec<VendingShop> {
            shops
                .iter()
                .filter(|shop| self.matches(shop))
                .cloned()
                .collect()
        }
    }
}
