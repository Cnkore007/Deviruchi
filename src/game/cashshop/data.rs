//! Cash Shop 数据定义
//!
//! 定义商城物品、分类和数据库结构

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 商城物品分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum CashShopCategory {
    /// 推广/活动物品
    Promotion = 0,
    /// 回复药品
    Healing = 1,
    /// 实用物品
    Useful = 2,
    /// 装备
    Equipment = 3,
    /// 埃皮克皮（Etcerkera/ETC）
    Etcerkera = 4,
    /// 宠物相关
    Pet = 5,
    /// 高级/特殊物品
    Premium = 6,
    /// 礼盒
    GiftBox = 7,
}

impl CashShopCategory {
    /// 从 u8 值创建分类
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(CashShopCategory::Promotion),
            1 => Some(CashShopCategory::Healing),
            2 => Some(CashShopCategory::Useful),
            3 => Some(CashShopCategory::Equipment),
            4 => Some(CashShopCategory::Etcerkera),
            5 => Some(CashShopCategory::Pet),
            6 => Some(CashShopCategory::Premium),
            7 => Some(CashShopCategory::GiftBox),
            _ => None,
        }
    }

    /// 获取分类名称
    pub fn name(&self) -> &'static str {
        match self {
            CashShopCategory::Promotion => "Promotion",
            CashShopCategory::Healing => "Healing",
            CashShopCategory::Useful => "Useful",
            CashShopCategory::Equipment => "Equipment",
            CashShopCategory::Etcerkera => "Etcerkera",
            CashShopCategory::Pet => "Pet",
            CashShopCategory::Premium => "Premium",
            CashShopCategory::GiftBox => "Gift Box",
        }
    }

    /// 获取所有分类
    pub fn all() -> Vec<CashShopCategory> {
        vec![
            CashShopCategory::Promotion,
            CashShopCategory::Healing,
            CashShopCategory::Useful,
            CashShopCategory::Equipment,
            CashShopCategory::Etcerkera,
            CashShopCategory::Pet,
            CashShopCategory::Premium,
            CashShopCategory::GiftBox,
        ]
    }
}

/// 商城物品结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashShopItem {
    /// 物品ID
    pub item_id: u16,
    /// 分类
    pub category: CashShopCategory,
    /// 原价（现金点数）
    pub price: u32,
    /// 折扣价（0 表示无折扣）
    pub discount_price: u32,
    /// 物品名称
    pub name: String,
    /// 物品描述
    pub description: String,
    /// 是否可赠送
    pub is_giftable: bool,
    /// 是否可用/上架
    pub is_available: bool,
    /// 是否为限时商品
    pub is_limited: bool,
    /// 每日购买限制（0 表示无限制）
    pub daily_limit: u16,
}

impl CashShopItem {
    /// 创建新的商城物品
    pub fn new(
        item_id: u16,
        category: CashShopCategory,
        price: u32,
        name: String,
        description: String,
    ) -> Self {
        Self {
            item_id,
            category,
            price,
            discount_price: 0,
            name,
            description,
            is_giftable: true,
            is_available: true,
            is_limited: false,
            daily_limit: 0,
        }
    }

    /// 获取实际购买价格（优先使用折扣价）
    pub fn actual_price(&self) -> u32 {
        if self.discount_price > 0 && self.discount_price < self.price {
            self.discount_price
        } else {
            self.price
        }
    }

    /// 是否正在打折
    pub fn is_on_sale(&self) -> bool {
        self.discount_price > 0 && self.discount_price < self.price
    }

    /// 获取折扣百分比（如果没有折扣返回 100）
    pub fn discount_percent(&self) -> u8 {
        if !self.is_on_sale() {
            return 100;
        }
        ((self.discount_price as f64 / self.price as f64) * 100.0) as u8
    }
}

/// 商城数据库
#[derive(Debug, Clone, Default)]
pub struct CashShopDatabase {
    /// 所有物品（按ID索引）
    items: HashMap<u16, CashShopItem>,
    /// 按分类索引的物品
    items_by_category: HashMap<CashShopCategory, Vec<CashShopItem>>,
}

impl CashShopDatabase {
    /// 创建新的商城数据库
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            items_by_category: HashMap::new(),
        }
    }

    /// 添加物品到数据库
    pub fn add_item(&mut self, item: CashShopItem) {
        let item_id = item.item_id;
        let category = item.category;

        // 添加到主索引
        self.items.insert(item_id, item.clone());

        // 添加到分类索引
        self.items_by_category
            .entry(category)
            .or_default()
            .push(item);
    }

    /// 通过物品ID获取物品
    pub fn get_item(&self, item_id: u16) -> Option<&CashShopItem> {
        self.items.get(&item_id)
    }

    /// 获取指定分类的所有物品
    pub fn get_by_category(&self, category: CashShopCategory) -> Option<&Vec<CashShopItem>> {
        self.items_by_category.get(&category)
    }

    /// 获取所有可用物品
    pub fn get_available_items(&self) -> Vec<&CashShopItem> {
        self.items
            .values()
            .filter(|item| item.is_available)
            .collect()
    }

    /// 获取所有物品
    pub fn get_all_items(&self) -> Vec<&CashShopItem> {
        self.items.values().collect()
    }

    /// 获取物品总数
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// 检查物品是否存在
    pub fn contains(&self, item_id: u16) -> bool {
        self.items.contains_key(&item_id)
    }

    /// 移除物品
    pub fn remove_item(&mut self, item_id: u16) -> Option<CashShopItem> {
        if let Some(item) = self.items.remove(&item_id) {
            // 从分类索引中移除
            if let Some(items) = self.items_by_category.get_mut(&item.category) {
                items.retain(|i| i.item_id != item_id);
            }
            Some(item)
        } else {
            None
        }
    }

    /// 设置物品可用状态
    pub fn set_availability(&mut self, item_id: u16, available: bool) -> bool {
        if let Some(item) = self.items.get_mut(&item_id) {
            item.is_available = available;
            true
        } else {
            false
        }
    }

    /// 设置物品折扣
    pub fn set_discount(&mut self, item_id: u16, discount_price: u32) -> bool {
        if let Some(item) = self.items.get_mut(&item_id) {
            item.discount_price = discount_price;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cash_shop_category_from_u8() {
        assert_eq!(
            CashShopCategory::from_u8(0),
            Some(CashShopCategory::Promotion)
        );
        assert_eq!(
            CashShopCategory::from_u8(1),
            Some(CashShopCategory::Healing)
        );
        assert_eq!(
            CashShopCategory::from_u8(7),
            Some(CashShopCategory::GiftBox)
        );
        assert_eq!(CashShopCategory::from_u8(8), None);
    }

    #[test]
    fn test_cash_shop_category_name() {
        assert_eq!(CashShopCategory::Promotion.name(), "Promotion");
        assert_eq!(CashShopCategory::GiftBox.name(), "Gift Box");
    }

    #[test]
    fn test_cash_shop_item_new() {
        let item = CashShopItem::new(
            123,
            CashShopCategory::Healing,
            100,
            "Red Potion".to_string(),
            "Restores 100 HP".to_string(),
        );

        assert_eq!(item.item_id, 123);
        assert_eq!(item.category, CashShopCategory::Healing);
        assert_eq!(item.price, 100);
        assert_eq!(item.discount_price, 0);
        assert!(item.is_giftable);
        assert!(item.is_available);
    }

    #[test]
    fn test_cash_shop_item_actual_price() {
        let mut item = CashShopItem::new(
            123,
            CashShopCategory::Healing,
            100,
            "Red Potion".to_string(),
            "Description".to_string(),
        );

        // 无折扣
        assert_eq!(item.actual_price(), 100);

        // 有折扣
        item.discount_price = 80;
        assert_eq!(item.actual_price(), 80);

        // 折扣价大于原价（无效折扣）
        item.discount_price = 120;
        assert_eq!(item.actual_price(), 100);
    }

    #[test]
    fn test_cash_shop_item_is_on_sale() {
        let mut item = CashShopItem::new(
            123,
            CashShopCategory::Healing,
            100,
            "Red Potion".to_string(),
            "Description".to_string(),
        );

        assert!(!item.is_on_sale());

        item.discount_price = 80;
        assert!(item.is_on_sale());

        item.discount_price = 0;
        assert!(!item.is_on_sale());

        item.discount_price = 100;
        assert!(!item.is_on_sale());
    }

    #[test]
    fn test_cash_shop_database_add_and_get() {
        let mut db = CashShopDatabase::new();

        db.add_item(CashShopItem::new(
            1001,
            CashShopCategory::Healing,
            50,
            "White Potion".to_string(),
            "Restores 50 HP".to_string(),
        ));

        db.add_item(CashShopItem::new(
            1002,
            CashShopCategory::Useful,
            100,
            "Fly Wing".to_string(),
            "Teleport to save point".to_string(),
        ));

        assert_eq!(db.item_count(), 2);
        assert!(db.contains(1001));
        assert!(db.contains(1002));
        assert!(!db.contains(1003));

        let item = db.get_item(1001).unwrap();
        assert_eq!(item.name, "White Potion");
    }

    #[test]
    fn test_cash_shop_database_get_by_category() {
        let mut db = CashShopDatabase::new();

        db.add_item(CashShopItem::new(
            1001,
            CashShopCategory::Healing,
            50,
            "White Potion".to_string(),
            "Description".to_string(),
        ));

        db.add_item(CashShopItem::new(
            1002,
            CashShopCategory::Healing,
            80,
            "Blue Potion".to_string(),
            "Description".to_string(),
        ));

        db.add_item(CashShopItem::new(
            2001,
            CashShopCategory::Useful,
            100,
            "Fly Wing".to_string(),
            "Description".to_string(),
        ));

        let healing_items = db.get_by_category(CashShopCategory::Healing).unwrap();
        assert_eq!(healing_items.len(), 2);

        let useful_items = db.get_by_category(CashShopCategory::Useful).unwrap();
        assert_eq!(useful_items.len(), 1);

        assert!(db.get_by_category(CashShopCategory::Pet).is_none());
    }

    #[test]
    fn test_cash_shop_database_remove_item() {
        let mut db = CashShopDatabase::new();

        db.add_item(CashShopItem::new(
            1001,
            CashShopCategory::Healing,
            50,
            "White Potion".to_string(),
            "Description".to_string(),
        ));

        assert!(db.contains(1001));

        let removed = db.remove_item(1001).unwrap();
        assert_eq!(removed.item_id, 1001);
        assert!(!db.contains(1001));
        assert!(db.remove_item(9999).is_none());
    }

    #[test]
    fn test_cash_shop_database_set_availability() {
        let mut db = CashShopDatabase::new();

        db.add_item(CashShopItem::new(
            1001,
            CashShopCategory::Healing,
            50,
            "White Potion".to_string(),
            "Description".to_string(),
        ));

        assert!(db.get_item(1001).unwrap().is_available);

        db.set_availability(1001, false);
        assert!(!db.get_item(1001).unwrap().is_available);

        // 设置不存在的物品
        assert!(!db.set_availability(9999, true));
    }

    #[test]
    fn test_cash_shop_database_set_discount() {
        let mut db = CashShopDatabase::new();

        db.add_item(CashShopItem::new(
            1001,
            CashShopCategory::Healing,
            100,
            "White Potion".to_string(),
            "Description".to_string(),
        ));

        assert!(!db.get_item(1001).unwrap().is_on_sale());

        db.set_discount(1001, 80);
        assert!(db.get_item(1001).unwrap().is_on_sale());
        assert_eq!(db.get_item(1001).unwrap().discount_price, 80);
    }

    #[test]
    fn test_cash_shop_database_get_available_items() {
        let mut db = CashShopDatabase::new();

        db.add_item(CashShopItem::new(
            1001,
            CashShopCategory::Healing,
            50,
            "Available Item".to_string(),
            "Description".to_string(),
        ));

        let mut unavailable_item = CashShopItem::new(
            1002,
            CashShopCategory::Healing,
            80,
            "Unavailable Item".to_string(),
            "Description".to_string(),
        );
        unavailable_item.is_available = false;
        db.add_item(unavailable_item);

        let available = db.get_available_items();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].name, "Available Item");
    }
}
