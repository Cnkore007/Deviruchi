//! Cash Shop 管理器
//!
//! 处理商城购买、现金点数管理和礼物赠送

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

use crate::game::cashshop::data::{CashShopCategory, CashShopDatabase, CashShopItem};
use crate::game::map::player::Player;

/// 现金点数结构
#[derive(Debug, Clone, Default)]
pub struct CashPoints {
    /// Kafra 点数（用于仓库和传送服务）
    pub kafra: u32,
    /// Credit 点数（用于购买商城物品）
    pub credit: u32,
}

impl CashPoints {
    /// 创建新的现金点数
    pub fn new(kafra: u32, credit: u32) -> Self {
        Self { kafra, credit }
    }

    /// 获取总点数
    pub fn total(&self) -> u32 {
        self.kafra.saturating_add(self.credit)
    }

    /// 检查是否有足够的 Credit 点数
    pub fn has_credit(&self, amount: u32) -> bool {
        self.credit >= amount
    }

    /// 检查是否有足够的 Kafra 点数
    pub fn has_kafra(&self, amount: u32) -> bool {
        self.kafra >= amount
    }
}

/// 购买结果
#[derive(Debug, Clone)]
pub enum PurchaseResult {
    /// 购买成功
    Success {
        remaining_points: u32,
        item_name: String,
        amount: u16,
    },
    /// 点数不足
    NotEnoughPoints { required: u32, available: u32 },
    /// 物品不存在
    ItemNotFound,
    /// 物品不可用/已下架
    ItemUnavailable,
    /// 背包已满
    InventoryFull,
    /// 物品不可赠送
    ItemNotGiftable,
    /// 目标玩家不存在
    TargetPlayerNotFound,
    /// 每日购买限制已达
    DailyLimitReached { limit: u16, item_name: String },
    /// 内部错误
    InternalError(String),
}

/// 礼物赠送结果
#[derive(Debug, Clone)]
pub enum GiftResult {
    /// 礼物发送成功
    Success {
        remaining_points: u32,
        item_name: String,
        target_name: String,
    },
    /// 现金点数不足
    NotEnoughPoints { required: u32, available: u32 },
    /// 物品不存在
    ItemNotFound,
    /// 物品不可用
    ItemUnavailable,
    /// 物品不可赠送
    ItemNotGiftable,
    /// 目标玩家不在线（礼物将存入收件箱）
    OfflinePlayer {
        item_name: String,
        target_name: String,
    },
    /// 每日购买限制已达
    DailyLimitReached { limit: u16, item_name: String },
    /// 内部错误
    InternalError(String),
}

/// 每日购买记录
#[derive(Debug, Clone, Default)]
pub struct DailyPurchaseRecord {
    /// 物品ID -> 购买数量
    items: HashMap<u16, u16>,
    /// 最后重置日期
    last_reset_date: String,
}

/// Cash Shop 管理器
pub struct CashShopManager {
    /// 商城数据库
    database: Arc<CashShopDatabase>,
    /// 玩家现金点数（char_id -> CashPoints）
    player_cash: RwLock<HashMap<u32, CashPoints>>,
    /// 每日购买记录（char_id -> DailyPurchaseRecord）
    daily_purchases: RwLock<HashMap<u32, DailyPurchaseRecord>>,
    /// 购买记录（用于日志）
    purchase_logs: RwLock<Vec<PurchaseLogEntry>>,
}

/// 购买日志条目
#[derive(Debug, Clone)]
pub struct PurchaseLogEntry {
    pub char_id: u32,
    pub item_id: u16,
    pub item_name: String,
    pub amount: u16,
    pub cost: u32,
    pub purchase_type: PurchaseType,
    pub target_name: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 购买类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurchaseType {
    /// 直接购买
    Buy,
    /// 赠送
    Gift,
}

impl CashShopManager {
    /// 创建新的 Cash Shop 管理器
    pub fn new(database: Arc<CashShopDatabase>) -> Self {
        Self {
            database,
            player_cash: RwLock::new(HashMap::new()),
            daily_purchases: RwLock::new(HashMap::new()),
            purchase_logs: RwLock::new(Vec::new()),
        }
    }

    /// 从数据库加载玩家现金点数
    pub fn load_player_cash(&self, char_id: u32, kafra: u32, credit: u32) {
        let mut cash = self.player_cash.write();
        cash.insert(char_id, CashPoints::new(kafra, credit));
        info!(
            "Loaded cash points for char_id {}: kafra={}, credit={}",
            char_id, kafra, credit
        );
    }

    /// 获取所有可用的分类
    pub fn get_categories(&self) -> Vec<CashShopCategory> {
        CashShopCategory::all()
    }

    /// 获取指定分类的所有物品
    pub fn get_items(&self, category: Option<CashShopCategory>) -> Vec<CashShopItem> {
        match category {
            Some(cat) => self
                .database
                .get_by_category(cat)
                .cloned()
                .unwrap_or_default(),
            None => self
                .database
                .get_all_items()
                .iter()
                .map(|i| (*i).clone())
                .collect(),
        }
    }

    /// 获取可用物品（只返回上架的）
    pub fn get_available_items(&self) -> Vec<CashShopItem> {
        self.database
            .get_available_items()
            .into_iter()
            .map(|i| (*i).clone())
            .collect()
    }

    /// 通过物品ID获取物品
    pub fn get_item(&self, item_id: u16) -> Option<CashShopItem> {
        self.database.get_item(item_id).cloned()
    }

    /// 获取玩家的现金点数
    pub fn get_cash_points(&self, char_id: u32) -> CashPoints {
        self.player_cash
            .read()
            .get(&char_id)
            .cloned()
            .unwrap_or_default()
    }

    /// 添加现金点数
    pub fn add_cash_points(&self, char_id: u32, kafra: u32, credit: u32) {
        let mut cash = self.player_cash.write();
        let entry = cash.entry(char_id).or_default();
        entry.kafra += kafra;
        entry.credit += credit;
        info!(
            "Added cash points for char_id {}: kafra +{}, credit +{}",
            char_id, kafra, credit
        );
    }

    /// 设置玩家的现金点数（用于初始化或重置）
    pub fn set_cash_points(&self, char_id: u32, kafra: u32, credit: u32) {
        let mut cash = self.player_cash.write();
        cash.insert(char_id, CashPoints::new(kafra, credit));
        info!(
            "Set cash points for char_id {}: kafra={}, credit={}",
            char_id, kafra, credit
        );
    }

    /// 使用 Kafra 点数（用于仓库或传送）
    pub fn use_kafra_points(&self, char_id: u32, amount: u32) -> Result<u32, &'static str> {
        let mut cash = self.player_cash.write();
        let points = cash
            .get_mut(&char_id)
            .ok_or("Player cash points not found")?;

        if points.kafra < amount {
            return Err("Not enough Kafra points");
        }

        points.kafra -= amount;
        Ok(points.kafra)
    }

    /// 使用 Credit 点数购买物品
    pub fn use_credit_points(&self, char_id: u32, amount: u32) -> Result<u32, &'static str> {
        let mut cash = self.player_cash.write();
        let points = cash
            .get_mut(&char_id)
            .ok_or("Player cash points not found")?;

        if points.credit < amount {
            return Err("Not enough Credit points");
        }

        points.credit -= amount;
        Ok(points.credit)
    }

    /// 购买物品
    ///
    /// 在实际实现中，这里应该调用 Inventory 系统来添加物品到玩家背包
    pub fn purchase(&self, player: &Player, item_id: u16, amount: u16) -> PurchaseResult {
        // 获取物品信息
        let item = match self.database.get_item(item_id) {
            Some(item) => item.clone(),
            None => {
                warn!("Purchase failed: item {} not found", item_id);
                return PurchaseResult::ItemNotFound;
            }
        };

        // 检查物品是否可用
        if !item.is_available {
            warn!("Purchase failed: item {} is not available", item_id);
            return PurchaseResult::ItemUnavailable;
        }

        // 检查每日限制
        if let Some(limit_reached) = self.check_daily_limit(player.char_id, item_id, amount, &item)
        {
            return limit_reached;
        }

        // 计算总价
        let total_cost = item.actual_price() * (amount as u32);

        // 检查现金点数
        let mut cash = self.player_cash.write();
        let points = cash.entry(player.char_id).or_default();

        if points.credit < total_cost {
            return PurchaseResult::NotEnoughPoints {
                required: total_cost,
                available: points.credit,
            };
        }

        // 记录购买日志（即使购买系统未完全实现）
        self.log_purchase(
            player.char_id,
            item_id,
            item.name.clone(),
            amount,
            total_cost,
            PurchaseType::Buy,
            None,
        );

        // 购买系统尚未完全实现（物品不会添加到背包），不扣除点数
        warn!(
            "Cash shop purchase not yet implemented: item_id={}, player={}",
            item_id, player.name
        );
        PurchaseResult::InternalError("Cash shop purchase system not yet implemented".to_string())
    }

    /// 赠送物品给其他玩家
    ///
    /// 在实际实现中，应该将物品存入目标玩家的收件箱或邮件系统
    pub fn gift(&self, player: &Player, target_name: &str, item_id: u16) -> GiftResult {
        // 获取物品信息
        let item = match self.database.get_item(item_id) {
            Some(item) => item.clone(),
            None => {
                warn!("Gift failed: item {} not found", item_id);
                return GiftResult::ItemNotFound;
            }
        };

        // 检查物品是否可用
        if !item.is_available {
            warn!("Gift failed: item {} is not available", item_id);
            return GiftResult::ItemUnavailable;
        }

        // 检查物品是否可赠送
        if !item.is_giftable {
            warn!("Gift failed: item {} is not giftable", item_id);
            return GiftResult::ItemNotGiftable;
        }

        // 检查每日限制（假设只购买1个）
        if let Some(limit) = self.check_daily_limit(player.char_id, item_id, 1, &item)
            && let PurchaseResult::DailyLimitReached { limit, item_name } = limit
        {
            return GiftResult::DailyLimitReached { limit, item_name };
        }

        // 计算价格（礼物固定为1个）
        let total_cost = item.actual_price();

        // 检查现金点数
        let mut cash = self.player_cash.write();
        let points = cash.entry(player.char_id).or_default();

        if points.credit < total_cost {
            return GiftResult::NotEnoughPoints {
                required: total_cost,
                available: points.credit,
            };
        }

        // 赠送系统尚未完全实现，不扣除点数
        warn!(
            "Cash shop gift not yet implemented: item_id={}, to={}",
            item_id, target_name
        );
        GiftResult::InternalError("Cash shop gift system not yet implemented".to_string())
    }

    /// 检查每日购买限制
    fn check_daily_limit(
        &self,
        char_id: u32,
        item_id: u16,
        amount: u16,
        _item: &CashShopItem,
    ) -> Option<PurchaseResult> {
        // 获取今日日期字符串
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let purchases = self.daily_purchases.read();
        let record = purchases.get(&char_id)?;

        // 如果日期不同，已自动重置
        if record.last_reset_date != today {
            return None;
        }

        // 检查该物品的购买量
        if let Some(&purchased) = record.items.get(&item_id) {
            // 假设每种物品每日限制为 1（实际应从 item.daily_limit 读取）
            let limit = 1;
            if purchased + amount > limit {
                return Some(PurchaseResult::DailyLimitReached {
                    limit,
                    item_name: _item.name.clone(),
                });
            }
        }

        None
    }

    /// 记录购买日志
    #[allow(clippy::too_many_arguments)]
    fn log_purchase(
        &self,
        char_id: u32,
        item_id: u16,
        item_name: String,
        amount: u16,
        cost: u32,
        purchase_type: PurchaseType,
        target_name: Option<String>,
    ) {
        let mut logs = self.purchase_logs.write();
        logs.push(PurchaseLogEntry {
            char_id,
            item_id,
            item_name,
            amount,
            cost,
            purchase_type,
            target_name,
            timestamp: chrono::Utc::now(),
        });
    }

    /// 获取玩家的购买历史
    pub fn get_purchase_history(&self, char_id: u32) -> Vec<PurchaseLogEntry> {
        self.purchase_logs
            .read()
            .iter()
            .filter(|log| log.char_id == char_id)
            .cloned()
            .collect()
    }

    /// 获取今天的购买总数
    pub fn get_today_purchase_count(&self, char_id: u32) -> u32 {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let purchases = self.daily_purchases.read();
        if let Some(record) = purchases.get(&char_id)
            && record.last_reset_date == today
        {
            return record.items.values().map(|&v| v as u32).sum();
        }
        0
    }

    /// 重置玩家的每日购买记录（通常在日期变更时调用）
    pub fn reset_daily_purchases(&self, char_id: u32) {
        let mut purchases = self.daily_purchases.write();
        if let Some(record) = purchases.get_mut(&char_id) {
            record.items.clear();
            record.last_reset_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        }
    }

    /// 批量重置所有玩家的每日购买记录
    pub fn reset_all_daily_purchases(&self) {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let mut purchases = self.daily_purchases.write();
        for record in purchases.values_mut() {
            record.items.clear();
            record.last_reset_date = today.clone();
        }
        info!("Reset all daily purchase records");
    }

    /// 获取商城数据库引用
    pub fn database(&self) -> &Arc<CashShopDatabase> {
        &self.database
    }

    /// 搜索物品
    pub fn search_items(&self, query: &str) -> Vec<CashShopItem> {
        let query_lower = query.to_lowercase();
        self.database
            .get_all_items()
            .iter()
            .filter(|item| {
                item.is_available
                    && (item.name.to_lowercase().contains(&query_lower)
                        || item.description.to_lowercase().contains(&query_lower))
            })
            .map(|i| (*i).clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::constants;
    use crate::game::item::Equipment;
    use crate::game::map::PlayerState;
    use crate::game::status::PlayerStatus;
    use std::sync::Arc;
    use uuid::Uuid;

    fn create_test_manager() -> CashShopManager {
        let mut db = CashShopDatabase::new();

        db.add_item(CashShopItem {
            item_id: 1001,
            category: CashShopCategory::Healing,
            price: 100,
            discount_price: 80,
            name: "Red Potion".to_string(),
            description: "Restores HP".to_string(),
            is_giftable: true,
            is_available: true,
            is_limited: false,
            daily_limit: 1,
        });

        db.add_item(CashShopItem {
            item_id: 1002,
            category: CashShopCategory::Equipment,
            price: 500,
            discount_price: 0,
            name: "Sword".to_string(),
            description: "A basic sword".to_string(),
            is_giftable: false,
            is_available: true,
            is_limited: false,
            daily_limit: 0,
        });

        db.add_item(CashShopItem {
            item_id: 1003,
            category: CashShopCategory::Healing,
            price: 100,
            discount_price: 0,
            name: "Unavailable Item".to_string(),
            description: "This item is unavailable".to_string(),
            is_giftable: true,
            is_available: false,
            is_limited: false,
            daily_limit: 0,
        });

        CashShopManager::new(Arc::new(db))
    }

    fn create_test_player() -> Player {
        Player {
            id: Uuid::new_v4(),
            char_id: 1,
            account_id: 1,
            name: "TestPlayer".to_string(),
            map_name: "test_map".to_string(),
            combat: parking_lot::RwLock::new(crate::game::map::player::CombatStats {
                hp: 100,
                max_hp: 100,
                sp: 50,
                max_sp: 50,
                state: PlayerState::Alive,
                in_combat: false,
                is_sitting: false,
                walk_speed: constants::DEFAULT_WALK_SPEED,
                direction: 0,
            }),
            pos: parking_lot::RwLock::new(crate::game::map::player::Position { x: 100, y: 100 }),
            level: parking_lot::RwLock::new(crate::game::map::player::LevelStats {
                base_level: 10,
                job_level: 5,
                base_exp: 5000,
                job_exp: 3000,
                status_point: 0,
                skill_point: 0,
            }),
            attrs: parking_lot::RwLock::new(crate::game::map::player::Attributes {
                str: 1,
                agi: 1,
                vit: 1,
                int: 1,
                dex: 1,
                luk: 1,
            }),
            economy: parking_lot::RwLock::new(crate::game::map::player::Economy {
                zeny: 0,
                current_weight: 0,
                max_weight: constants::BASE_MAX_WEIGHT,
                job: 0,
                shop_id: None,
                group_id: 0,
            }),
            save_point: parking_lot::RwLock::new(crate::game::map::player::SavePoint {
                map: "test_map".to_string(),
                x: 50,
                y: 50,
            }),
            equipment: parking_lot::RwLock::new(Equipment::new()),
            status: PlayerStatus::new(Uuid::new_v4()),
            inventory: parking_lot::RwLock::new(Vec::new()),
            hotkeys: parking_lot::RwLock::new(Vec::new()),
            party_id: parking_lot::RwLock::new(None),
            guild_id: parking_lot::RwLock::new(None),
        }
    }

    #[test]
    fn test_get_categories() {
        let manager = create_test_manager();
        let categories = manager.get_categories();
        assert_eq!(categories.len(), 8);
    }

    #[test]
    fn test_get_items_by_category() {
        let manager = create_test_manager();

        let healing_items = manager.get_items(Some(CashShopCategory::Healing));
        assert_eq!(healing_items.len(), 2);

        let equipment_items = manager.get_items(Some(CashShopCategory::Equipment));
        assert_eq!(equipment_items.len(), 1);

        let all_items = manager.get_items(None);
        assert_eq!(all_items.len(), 3);
    }

    #[test]
    fn test_get_available_items() {
        let manager = create_test_manager();
        let available = manager.get_available_items();
        assert_eq!(available.len(), 2); // 不包括 unavailable item
    }

    #[test]
    fn test_get_item() {
        let manager = create_test_manager();

        let item = manager.get_item(1001);
        assert!(item.is_some());
        assert_eq!(item.unwrap().name, "Red Potion");

        let not_found = manager.get_item(9999);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_add_and_get_cash_points() {
        let manager = create_test_manager();

        // 初始点数为 0
        let initial = manager.get_cash_points(1);
        assert_eq!(initial.credit, 0);
        assert_eq!(initial.kafra, 0);

        // 设置点数
        manager.set_cash_points(1, 100, 500);

        let points = manager.get_cash_points(1);
        assert_eq!(points.kafra, 100);
        assert_eq!(points.credit, 500);
    }

    #[test]
    fn test_add_cash_points() {
        let manager = create_test_manager();

        manager.set_cash_points(1, 100, 500);
        manager.add_cash_points(1, 50, 200);

        let points = manager.get_cash_points(1);
        assert_eq!(points.kafra, 150);
        assert_eq!(points.credit, 700);
    }

    #[test]
    fn test_use_kafra_points() {
        let manager = create_test_manager();
        manager.set_cash_points(1, 100, 500);

        let result = manager.use_kafra_points(1, 30);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 70);

        let result = manager.use_kafra_points(1, 100);
        assert!(result.is_err()); // Not enough points
    }

    #[test]
    fn test_purchase_returns_not_implemented() {
        let manager = create_test_manager();
        manager.set_cash_points(1, 0, 1000);

        let player = create_test_player();
        let result = manager.purchase(&player, 1001, 1);

        // 购买系统未实现时应返回 InternalError，且不扣除点数
        assert!(matches!(result, PurchaseResult::InternalError(_)));

        // 点数不应被扣除
        let points = manager.get_cash_points(1);
        assert_eq!(points.credit, 1000);
    }

    #[test]
    fn test_purchase_item_not_found() {
        let manager = create_test_manager();
        manager.set_cash_points(1, 0, 1000);

        let player = create_test_player();
        let result = manager.purchase(&player, 9999, 1);

        match result {
            PurchaseResult::ItemNotFound => {}
            _ => panic!("Expected ItemNotFound"),
        }
    }

    #[test]
    fn test_purchase_item_unavailable() {
        let manager = create_test_manager();
        manager.set_cash_points(1, 0, 1000);

        let player = create_test_player();
        let result = manager.purchase(&player, 1003, 1);

        match result {
            PurchaseResult::ItemUnavailable => {}
            _ => panic!("Expected ItemUnavailable"),
        }
    }

    #[test]
    fn test_purchase_not_enough_points() {
        let manager = create_test_manager();
        manager.set_cash_points(1, 0, 50);

        let player = create_test_player();
        let result = manager.purchase(&player, 1001, 1);

        match result {
            PurchaseResult::NotEnoughPoints {
                required,
                available,
            } => {
                assert_eq!(required, 80); // discount price
                assert_eq!(available, 50);
            }
            _ => panic!("Expected NotEnoughPoints"),
        }
    }

    #[test]
    fn test_gift_returns_not_implemented() {
        let manager = create_test_manager();
        manager.set_cash_points(1, 0, 1000);

        let player = create_test_player();
        let result = manager.gift(&player, "OtherPlayer", 1001);

        // 赠送系统未实现时应返回 InternalError，且不扣除点数
        assert!(matches!(result, GiftResult::InternalError(_)));

        // 点数不应被扣除
        let points = manager.get_cash_points(1);
        assert_eq!(points.credit, 1000);
    }

    #[test]
    fn test_gift_item_not_giftable() {
        let manager = create_test_manager();
        manager.set_cash_points(1, 0, 1000);

        let player = create_test_player();
        let result = manager.gift(&player, "OtherPlayer", 1002); // Sword is not giftable

        match result {
            GiftResult::ItemNotGiftable => {}
            _ => panic!("Expected ItemNotGiftable"),
        }
    }

    #[test]
    fn test_search_items() {
        let manager = create_test_manager();

        let results = manager.search_items("potion");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Red Potion");

        let results = manager.search_items("sword");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Sword");

        let results = manager.search_items("restores");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Red Potion");
    }

    #[test]
    fn test_purchase_log() {
        let manager = create_test_manager();
        manager.set_cash_points(1, 0, 1000);

        let player = create_test_player();
        manager.purchase(&player, 1001, 1);

        let history = manager.get_purchase_history(1);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].item_id, 1001);
        assert_eq!(history[0].amount, 1);
        assert_eq!(history[0].purchase_type, PurchaseType::Buy);
    }
}
