//! 复活点系统 - 管理玩家死亡后的复活位置
//!
//! 支持两种复活类型：
//! - Normal: 返回存储点（save point）或者地图默认点
//! - InstantCall: 返回地图出生点

use crate::game::map::player::Player;
use std::collections::HashMap;

/// 复活类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespawnType {
    /// 普通复活：返回存储点，如果没有存储点则返回地图默认点
    Normal,
    /// 即时复活：返回地图出生点（出生点）
    InstantCall,
}

/// 复活点信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RespawnPoint {
    /// 地图名称
    pub map_name: String,
    /// X坐标
    pub x: u16,
    /// Y坐标
    pub y: u16,
}

impl RespawnPoint {
    /// 创建新的复活点
    pub fn new(map_name: impl Into<String>, x: u16, y: u16) -> Self {
        Self {
            map_name: map_name.into(),
            x,
            y,
        }
    }
}

/// 复活服务 - 管理玩家的复活位置
#[derive(Debug, Clone)]
pub struct RespawnService {
    /// 全局默认复活点（当没有其他可用复活点时使用）
    default_point: RespawnPoint,
    /// 各地图的默认复活点
    map_defaults: HashMap<String, RespawnPoint>,
}

impl RespawnService {
    /// 创建新的复活服务
    pub fn new() -> Self {
        Self {
            // 全局默认复活点：prontera 157, 183 (RO经典复活点)
            default_point: RespawnPoint::new("prontera", 157, 183),
            map_defaults: HashMap::new(),
        }
    }

    /// 设置默认复活点并返回修改后的服务
    pub fn with_default(mut self, map: &str, x: u16, y: u16) -> Self {
        self.default_point = RespawnPoint::new(map, x, y);
        self
    }

    /// 添加地图默认复活点
    pub fn add_map_default(&mut self, map: &str, x: u16, y: u16) {
        self.map_defaults
            .insert(map.to_string(), RespawnPoint::new(map, x, y));
    }

    /// 获取玩家的复活位置
    ///
    /// 根据复活类型返回合适的复活坐标：
    /// - Normal: 返回玩家的存储点，如果没有则返回地图默认点，最后才用全局默认
    /// - InstantCall: 返回地图的出生点（spawn point，等同于地图默认点）
    pub fn get_respawn_position(
        &self,
        player: &Player,
        respawn_type: RespawnType,
    ) -> (u16, u16, String) {
        let player_map = &player.map_name;

        match respawn_type {
            RespawnType::Normal => {
                // 优先使用玩家的存储点
                // 注意：这里简化处理，实际应该从 SavePointManager 获取玩家的存储点
                // 当前实现使用玩家当前位置所在的地图默认点

                // 检查是否有地图默认点
                if let Some(map_default) = self.map_defaults.get(player_map) {
                    return (map_default.x, map_default.y, map_default.map_name.clone());
                }

                // 尝试使用玩家当前位置作为复活点（模拟存储点效果）
                let (x, y) = player.get_position();
                (x, y, player_map.clone())
            }
            RespawnType::InstantCall => {
                // InstantCall 返回地图的出生点（spawn point）
                // 对于 MVP，使用地图默认点
                if let Some(map_default) = self.map_defaults.get(player_map) {
                    return (map_default.x, map_default.y, map_default.map_name.clone());
                }

                // 如果没有地图默认点，使用玩家当前位置
                let (x, y) = player.get_position();
                (x, y, player_map.clone())
            }
        }
    }

    /// 获取全局默认复活点
    pub fn get_global_default(&self) -> &RespawnPoint {
        &self.default_point
    }

    /// 获取指定地图的默认复活点
    pub fn get_map_default(&self, map_name: &str) -> Option<&RespawnPoint> {
        self.map_defaults.get(map_name)
    }
}

impl Default for RespawnService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::map::player::{Player, PlayerState};
    use parking_lot::RwLock;
    use uuid::Uuid;

    fn make_player(map: &str, x: u16, y: u16) -> Player {
        Player {
            id: Uuid::new_v4(),
            char_id: 1,
            account_id: 1,
            name: "TestPlayer".to_string(),
            pos_x: RwLock::new(x),
            pos_y: RwLock::new(y),
            map_name: map.to_string(),
            hp: RwLock::new(100),
            max_hp: RwLock::new(100),
            sp: RwLock::new(50),
            max_sp: RwLock::new(50),
            base_level: RwLock::new(10),
            job_level: RwLock::new(5),
            base_exp: RwLock::new(0),
            job_exp: RwLock::new(0),
            state: RwLock::new(PlayerState::Alive),
            str: RwLock::new(10),
            agi: RwLock::new(10),
            vit: RwLock::new(10),
            int: RwLock::new(10),
            dex: RwLock::new(10),
            luk: RwLock::new(10),
            walk_speed: RwLock::new(150),
            zeny: RwLock::new(0),
            current_weight: RwLock::new(0),
            max_weight: RwLock::new(20000),
            equipment: RwLock::new(crate::game::item::Equipment::new()),
            is_sitting: RwLock::new(false),
            status: crate::game::status::PlayerStatus::new(Uuid::new_v4()),
            shop_id: RwLock::new(None),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
            save_map: RwLock::new(map.to_string()),
            save_x: RwLock::new(50),
            save_y: RwLock::new(50),
            job: RwLock::new(0),
            in_combat: RwLock::new(false),
            group_id: RwLock::new(0),
        }
    }

    #[test]
    fn test_respawn_service_new() {
        let service = RespawnService::new();
        let default = service.get_global_default();
        assert_eq!(default.map_name, "prontera");
        assert_eq!(default.x, 157);
        assert_eq!(default.y, 183);
    }

    #[test]
    fn test_with_default() {
        let service = RespawnService::new().with_default("new_1-1", 53, 111);
        let default = service.get_global_default();
        assert_eq!(default.map_name, "new_1-1");
        assert_eq!(default.x, 53);
        assert_eq!(default.y, 111);
    }

    #[test]
    fn test_add_map_default() {
        let mut service = RespawnService::new();
        service.add_map_default("prontera", 150, 180);

        let map_default = service.get_map_default("prontera");
        assert!(map_default.is_some());
        let md = map_default.unwrap();
        assert_eq!(md.x, 150);
        assert_eq!(md.y, 180);
    }

    #[test]
    fn test_get_respawn_position_normal_with_map_default() {
        let mut service = RespawnService::new();
        service.add_map_default("prontera", 150, 180);

        let player = make_player("prontera", 100, 100);
        let (x, y, map) = service.get_respawn_position(&player, RespawnType::Normal);

        assert_eq!(x, 150);
        assert_eq!(y, 180);
        assert_eq!(map, "prontera");
    }

    #[test]
    fn test_get_respawn_position_normal_without_map_default_returns_player_position() {
        let service = RespawnService::new();

        let player = make_player("unknown_map", 100, 100);
        let (x, y, map) = service.get_respawn_position(&player, RespawnType::Normal);

        // 没有地图默认点时，返回玩家当前位置
        assert_eq!(x, 100);
        assert_eq!(y, 100);
        assert_eq!(map, "unknown_map");
    }

    #[test]
    fn test_get_respawn_position_instant_call() {
        let mut service = RespawnService::new();
        service.add_map_default("prontera", 150, 180);

        let player = make_player("prontera", 100, 100);
        let (x, y, map) = service.get_respawn_position(&player, RespawnType::InstantCall);

        assert_eq!(x, 150);
        assert_eq!(y, 180);
        assert_eq!(map, "prontera");
    }

    #[test]
    fn test_get_respawn_position_instant_call_without_map_default() {
        let service = RespawnService::new();

        let player = make_player("unknown_map", 100, 100);
        let (x, y, map) = service.get_respawn_position(&player, RespawnType::InstantCall);

        // 没有地图默认点时，返回玩家当前位置
        assert_eq!(x, 100);
        assert_eq!(y, 100);
        assert_eq!(map, "unknown_map");
    }

    #[test]
    fn test_respawn_point_creation() {
        let point = RespawnPoint::new("test_map", 100, 200);
        assert_eq!(point.map_name, "test_map");
        assert_eq!(point.x, 100);
        assert_eq!(point.y, 200);
    }

    #[test]
    fn test_respawn_point_clone() {
        let point1 = RespawnPoint::new("test", 1, 2);
        let point2 = point1.clone();
        assert_eq!(point1, point2);
    }
}
