//! Kafra 传送服务
//!
//! 使用 Kafra 点数进行传送服务

use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{info, warn};

use crate::game::cashshop::manager::CashShopManager;

/// Kafra 传送目的地
#[derive(Debug, Clone)]
pub struct TeleportDestination {
    /// 目的地名称
    pub name: String,
    /// 地图名称
    pub map: String,
    /// X 坐标
    pub x: u16,
    /// Y 坐标
    pub y: u16,
    /// 传送费用（Kafra 点数）
    pub cost: u32,
}

impl TeleportDestination {
    /// 创建新的传送目的地
    pub fn new(name: &str, map: &str, x: u16, y: u16, cost: u32) -> Self {
        Self {
            name: name.to_string(),
            map: map.to_string(),
            x,
            y,
            cost,
        }
    }
}

/// Kafra 服务类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KafraServiceType {
    /// 普通 Kafra（仓库）
    Storage,
    /// 传送 Kafra
    Teleport,
    /// 任意 Kafra
    Any,
}

/// Kafra 传送结果
#[derive(Debug, Clone)]
pub enum TeleportResult {
    /// 传送成功
    Success {
        destination: String,
        remaining_points: u32,
    },
    /// 点数不足
    NotEnoughPoints { required: u32, available: u32 },
    /// 目的地不存在
    DestinationNotFound,
    /// 玩家无法传送（状态限制）
    PlayerCannotTeleport,
    /// 已在该位置
    AlreadyAtDestination,
    /// 内部错误（功能未实现等）
    InternalError(String),
}

/// 仓库服务结果
#[derive(Debug, Clone)]
pub enum StorageResult {
    /// 成功打开仓库
    Success { remaining_points: u32 },
    /// 点数不足
    NotEnoughPoints { required: u32, available: u32 },
    /// 仓库已打开
    AlreadyOpen,
    /// 内部错误（功能未实现等）
    InternalError(String),
}

/// Kafra 服务
pub struct KafraService {
    /// 传送目的地列表
    teleport_destinations: RwLock<Vec<TeleportDestination>>,
    /// 基础传送费用
    base_cost: u32,
    /// 仓库使用费用
    storage_cost: u32,
    /// Cash Shop 管理器引用
    cash_shop: Arc<CashShopManager>,
}

impl KafraService {
    /// 创建新的 Kafra 服务
    pub fn new(cash_shop: Arc<CashShopManager>) -> Self {
        Self {
            teleport_destinations: RwLock::new(Vec::new()),
            base_cost: 10,   // 默认基础费用 10 Kafra 点数
            storage_cost: 5, // 默认仓库使用费用 5 Kafra 点数
            cash_shop,
        }
    }

    /// 初始化默认传送目的地
    pub fn init_default_destinations(&self) {
        let mut destinations = self.teleport_destinations.write();

        // 常用城市
        destinations.push(TeleportDestination::new(
            "Prontera", "prontera", 157, 187, 10,
        ));
        destinations.push(TeleportDestination::new("Morocc", "morocc", 156, 97, 10));
        destinations.push(TeleportDestination::new("Geffen", "geffen", 120, 130, 10));
        destinations.push(TeleportDestination::new(
            "Al De Baran",
            "aldebaran",
            140,
            110,
            10,
        ));
        destinations.push(TeleportDestination::new("Izlude", "izlude", 91, 104, 10));
        destinations.push(TeleportDestination::new("Payon", "payon", 160, 100, 10));
        destinations.push(TeleportDestination::new("Alberta", "alberta", 116, 57, 10));

        // 主要副本入口
        destinations.push(TeleportDestination::new(
            "Glast Heim",
            "glast_01",
            370,
            300,
            50,
        ));
        destinations.push(TeleportDestination::new(
            "Toy Factory",
            "factory",
            60,
            200,
            50,
        ));
        destinations.push(TeleportDestination::new(
            "Niflheim", "niflheim", 210, 220, 100,
        ));

        // 特殊地点
        destinations.push(TeleportDestination::new("Amatsu", "amatsu", 198, 84, 80));
        destinations.push(TeleportDestination::new("Comodo", "comodo", 209, 143, 60));
        destinations.push(TeleportDestination::new("Yuno", "yuno", 158, 125, 40));

        info!(
            "Initialized {} default teleport destinations",
            destinations.len()
        );
    }

    /// 添加传送目的地
    pub fn add_destination(&self, destination: TeleportDestination) {
        self.teleport_destinations.write().push(destination);
    }

    /// 添加传送目的地（便捷方法）
    pub fn add_destination_new(&self, name: &str, map: &str, x: u16, y: u16, cost: u32) {
        self.add_destination(TeleportDestination::new(name, map, x, y, cost));
    }

    /// 获取所有传送目的地
    pub fn get_destinations(&self) -> Vec<TeleportDestination> {
        self.teleport_destinations.read().clone()
    }

    /// 获取可用的传送目的地（根据玩家点数）
    pub fn get_available_destinations(&self, char_id: u32) -> Vec<TeleportDestination> {
        let points = self.cash_shop.get_cash_points(char_id);
        self.teleport_destinations
            .read()
            .iter()
            .filter(|dest| points.kafra >= dest.cost)
            .cloned()
            .collect()
    }

    /// 根据名称查找传送目的地
    pub fn find_destination(&self, name: &str) -> Option<TeleportDestination> {
        let name_lower = name.to_lowercase();
        self.teleport_destinations
            .read()
            .iter()
            .find(|dest| dest.name.to_lowercase() == name_lower)
            .cloned()
    }

    /// 传送到指定目的地
    ///
    /// 在实际实现中，这里应该调用 Map 系统来移动玩家
    pub fn teleport(&self, char_id: u32, destination_name: &str) -> TeleportResult {
        // 查找目的地
        let destination = match self.find_destination(destination_name) {
            Some(dest) => dest,
            None => {
                warn!(
                    "Teleport failed: destination '{}' not found",
                    destination_name
                );
                return TeleportResult::DestinationNotFound;
            }
        };

        // 检查 Kafra 点数
        let points = self.cash_shop.get_cash_points(char_id);
        if points.kafra < destination.cost {
            return TeleportResult::NotEnoughPoints {
                required: destination.cost,
                available: points.kafra,
            };
        }

        // 传送系统尚未实现，不扣除点数
        warn!("Kafra teleport not yet implemented: char_id={}, destination='{}'", char_id, destination.name);
        TeleportResult::InternalError("Kafra teleport system not yet implemented".to_string())
    }

    /// 传送到指定地图坐标
    pub fn teleport_to_coords(&self, char_id: u32, map: &str, x: u16, y: u16) -> TeleportResult {
        let cost = self.calculate_teleport_cost(map);

        // 检查 Kafra 点数
        let points = self.cash_shop.get_cash_points(char_id);
        if points.kafra < cost {
            return TeleportResult::NotEnoughPoints {
                required: cost,
                available: points.kafra,
            };
        }

        // 传送系统尚未实现，不扣除点数
        warn!("Kafra coord teleport not yet implemented: char_id={}, map={}, ({}, {})", char_id, map, x, y);
        TeleportResult::InternalError("Kafra teleport system not yet implemented".to_string())
    }

    /// 计算传送到指定地图的费用
    ///
    /// 基于距离和危险区域计算
    fn calculate_teleport_cost(&self, map: &str) -> u32 {
        // 危险区域/副本增加费用
        let dangerous_maps = [
            "glast_01",
            "glast_02",
            "abbey",
            "lhz_dun01",
            "lhz_dun02",
            "lhz_dun03",
            "thor_v01",
            "thor_v02",
            "thor_v03",
            "niflheim",
        ];

        if dangerous_maps.contains(&map) {
            self.base_cost * 5
        } else {
            self.base_cost
        }
    }

    /// 使用仓库服务
    ///
    /// 在实际实现中，这里应该调用 Storage 系统来打开仓库
    pub fn use_storage(&self, char_id: u32) -> StorageResult {
        // 检查 Kafra 点数
        let points = self.cash_shop.get_cash_points(char_id);
        if points.kafra < self.storage_cost {
            return StorageResult::NotEnoughPoints {
                required: self.storage_cost,
                available: points.kafra,
            };
        }

        // 仓库系统尚未实现，不扣除点数
        warn!("Kafra storage not yet implemented: char_id={}", char_id);
        StorageResult::InternalError("Kafra storage system not yet implemented".to_string())
    }

    /// 获取仓库使用费用
    pub fn get_storage_cost(&self) -> u32 {
        self.storage_cost
    }

    /// 设置仓库使用费用
    pub fn set_storage_cost(&mut self, cost: u32) {
        self.storage_cost = cost;
    }

    /// 获取基础传送费用
    pub fn get_base_cost(&self) -> u32 {
        self.base_cost
    }

    /// 设置基础传送费用
    pub fn set_base_cost(&mut self, cost: u32) {
        self.base_cost = cost;
    }

    /// 估算传送到目的地的费用
    pub fn estimate_teleport_cost(&self, destination_name: &str) -> Option<u32> {
        self.find_destination(destination_name)
            .map(|dest| dest.cost)
    }

    /// 检查玩家是否可以使用传送服务
    pub fn can_teleport(&self, char_id: u32) -> bool {
        let points = self.cash_shop.get_cash_points(char_id);
        points.kafra >= self.base_cost
    }

    /// 检查玩家是否可以使用仓库服务
    pub fn can_use_storage(&self, char_id: u32) -> bool {
        let points = self.cash_shop.get_cash_points(char_id);
        points.kafra >= self.storage_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::cashshop::data::CashShopDatabase;
    use std::sync::Arc;

    fn create_test_kafra_service() -> KafraService {
        let db = Arc::new(CashShopDatabase::new());
        let cash_shop = Arc::new(CashShopManager::new(db));
        let service = KafraService::new(cash_shop);
        service.init_default_destinations();
        service
    }

    #[test]
    fn test_add_and_find_destination() {
        let service = create_test_kafra_service();

        // 测试查找默认目的地
        let prontera = service.find_destination("prontera");
        assert!(prontera.is_some());
        assert_eq!(prontera.unwrap().map, "prontera");

        // 测试不区分大小写
        let morocc = service.find_destination("MOROCC");
        assert!(morocc.is_some());

        // 测试不存在
        let not_found = service.find_destination("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_destinations() {
        let service = create_test_kafra_service();
        let destinations = service.get_destinations();
        assert!(!destinations.is_empty());
    }

    #[test]
    fn test_teleport_returns_not_implemented() {
        let db = Arc::new(CashShopDatabase::new());
        let cash_shop = Arc::new(CashShopManager::new(db.clone()));
        let service = KafraService::new(cash_shop.clone());

        // 添加测试目的地
        service.add_destination_new("Test City", "test_map", 100, 100, 10);

        // 设置 Kafra 点数
        cash_shop.set_cash_points(1, 100, 0);

        // 传送系统未实现时应返回 InternalError，且不扣除点数
        let result = service.teleport(1, "Test City");
        assert!(matches!(result, TeleportResult::InternalError(_)));

        // 点数不应被扣除
        let points = cash_shop.get_cash_points(1);
        assert_eq!(points.kafra, 100);
    }

    #[test]
    fn test_teleport_not_enough_points() {
        let db = Arc::new(CashShopDatabase::new());
        let cash_shop = Arc::new(CashShopManager::new(db.clone()));
        let service = KafraService::new(cash_shop.clone());

        service.add_destination_new("Test City", "test_map", 100, 100, 50);
        cash_shop.set_cash_points(1, 30, 0); // 只有 30 点，不够 50

        let result = service.teleport(1, "Test City");

        match result {
            TeleportResult::NotEnoughPoints {
                required,
                available,
            } => {
                assert_eq!(required, 50);
                assert_eq!(available, 30);
            }
            _ => panic!("Expected NotEnoughPoints"),
        }
    }

    #[test]
    fn test_teleport_destination_not_found() {
        let service = create_test_kafra_service();
        let result = service.teleport(1, "Nonexistent City");
        assert!(matches!(result, TeleportResult::DestinationNotFound));
    }

    #[test]
    fn test_use_storage_returns_not_implemented() {
        let db = Arc::new(CashShopDatabase::new());
        let cash_shop = Arc::new(CashShopManager::new(db.clone()));
        let service = KafraService::new(cash_shop.clone());

        cash_shop.set_cash_points(1, 100, 0);

        // 仓库系统未实现时应返回 InternalError，且不扣除点数
        let result = service.use_storage(1);
        assert!(matches!(result, StorageResult::InternalError(_)));

        // 点数不应被扣除
        let points = cash_shop.get_cash_points(1);
        assert_eq!(points.kafra, 100);
    }

    #[test]
    fn test_use_storage_not_enough_points() {
        let db = Arc::new(CashShopDatabase::new());
        let cash_shop = Arc::new(CashShopManager::new(db.clone()));
        let service = KafraService::new(cash_shop.clone());

        cash_shop.set_cash_points(1, 3, 0); // 只有 3 点，低于 5 点费用

        let result = service.use_storage(1);

        match result {
            StorageResult::NotEnoughPoints {
                required,
                available,
            } => {
                assert_eq!(required, 5);
                assert_eq!(available, 3);
            }
            _ => panic!("Expected NotEnoughPoints"),
        }
    }

    #[test]
    fn test_can_teleport() {
        let db = Arc::new(CashShopDatabase::new());
        let cash_shop = Arc::new(CashShopManager::new(db.clone()));
        let service = KafraService::new(cash_shop.clone());

        cash_shop.set_cash_points(1, 100, 0);
        assert!(service.can_teleport(1));

        cash_shop.set_cash_points(1, 5, 0);
        assert!(!service.can_teleport(1)); // 5 < 10 base cost

        cash_shop.set_cash_points(1, 0, 0);
        assert!(!service.can_teleport(1));
    }

    #[test]
    fn test_can_use_storage() {
        let db = Arc::new(CashShopDatabase::new());
        let cash_shop = Arc::new(CashShopManager::new(db.clone()));
        let service = KafraService::new(cash_shop.clone());

        cash_shop.set_cash_points(1, 100, 0);
        assert!(service.can_use_storage(1));

        cash_shop.set_cash_points(1, 4, 0);
        assert!(!service.can_use_storage(1)); // 4 < 5

        cash_shop.set_cash_points(1, 5, 0);
        assert!(service.can_use_storage(1)); // 5 >= 5
    }

    #[test]
    fn test_get_available_destinations() {
        let db = Arc::new(CashShopDatabase::new());
        let cash_shop = Arc::new(CashShopManager::new(db.clone()));
        let service = KafraService::new(cash_shop.clone());

        service.add_destination_new("Cheap Place", "cheap_map", 10, 10, 5);
        service.add_destination_new("Expensive Place", "expensive_map", 20, 20, 100);

        // 只有 50 点，只能去便宜的地方
        cash_shop.set_cash_points(1, 50, 0);
        let available = service.get_available_destinations(1);
        assert!(available.iter().any(|d| d.name == "Cheap Place"));
        assert!(!available.iter().any(|d| d.name == "Expensive Place"));

        // 有 150 点，两个地方都能去
        cash_shop.set_cash_points(1, 150, 0);
        let available = service.get_available_destinations(1);
        assert!(available.iter().any(|d| d.name == "Cheap Place"));
        assert!(available.iter().any(|d| d.name == "Expensive Place"));
    }
}
