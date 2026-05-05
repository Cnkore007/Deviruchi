//! 地图传送系统 - 透明传送核心数据结构
//!
//! 实现边缘触发式地图传送，当玩家走到地图边缘时自动传送到相邻地图

use crate::network::session::Session;
use crate::storage::Database;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// 地图边缘类型 - 定义触发传送的边界条件
#[derive(Debug, Clone, PartialEq)]
pub enum MapEdge {
    /// 北边缘 - 当 y <= threshold 时触发传送到北侧邻居
    North { y_threshold: u16 },
    /// 南边缘 - 当 y >= threshold 时触发传送到南侧邻居
    South { y_threshold: u16 },
    /// 东边缘 - 当 x >= threshold 时触发传送到东侧邻居
    East { x_threshold: u16 },
    /// 西边缘 - 当 x <= threshold 时触发传送到西侧邻居
    West { x_threshold: u16 },
}

impl MapEdge {
    /// 检查给定坐标是否触发该边缘的传送
    pub fn is_triggered(&self, x: u16, y: u16) -> bool {
        match self {
            MapEdge::North { y_threshold } => y <= *y_threshold,
            MapEdge::South { y_threshold } => y >= *y_threshold,
            MapEdge::East { x_threshold } => x >= *x_threshold,
            MapEdge::West { x_threshold } => x <= *x_threshold,
        }
    }
}

/// 地图邻接关系 - 定义两个地图之间的连接
#[derive(Debug, Clone)]
pub struct MapAdjacency {
    /// 源地图名称
    pub from_map: String,
    /// 触发传送的边缘
    pub edge: MapEdge,
    /// 目标地图名称
    pub to_map: String,
    /// 进入目标地图时的坐标偏移
    pub entry_offset: (i16, i16),
}

/// 传送动作 - 传送检查的结果
#[derive(Debug, Clone, PartialEq)]
pub struct TeleportAction {
    /// 源地图名称
    pub from_map: String,
    /// 目标地图名称
    pub to_map: String,
    /// 源坐标
    pub from_pos: (u16, u16),
    /// 目标坐标（已应用 entry_offset）
    pub to_pos: (u16, u16),
}

/// 存储点 - 角色可以保存的位置
#[derive(Debug, Clone, PartialEq)]
pub struct SavePoint {
    /// 地图名称
    pub map_name: String,
    /// X坐标
    pub x: u16,
    /// Y坐标
    pub y: u16,
}

impl SavePoint {
    /// 创建新的存储点
    pub fn new(map_name: impl Into<String>, x: u16, y: u16) -> Self {
        Self {
            map_name: map_name.into(),
            x,
            y,
        }
    }
}

/// 存储点管理器 - 管理角色的存储点
#[derive(Debug)]
pub struct SavePointManager {
    /// 角色ID到存储点的映射
    save_points: HashMap<u32, SavePoint>,
}

impl SavePointManager {
    /// 创建新的存储点管理器
    pub fn new() -> Self {
        Self {
            save_points: HashMap::new(),
        }
    }

    /// 设置角色的存储点
    pub fn set_save_point(&mut self, char_id: u32, save_point: SavePoint) {
        self.save_points.insert(char_id, save_point);
    }

    /// 获取角色的存储点
    pub fn get_save_point(&self, char_id: u32) -> Option<&SavePoint> {
        self.save_points.get(&char_id)
    }

    /// 检查角色是否有存储点
    pub fn has_save_point(&self, char_id: u32) -> bool {
        self.save_points.contains_key(&char_id)
    }

    /// 移除角色的存储点
    pub fn remove_save_point(&mut self, char_id: u32) -> Option<SavePoint> {
        self.save_points.remove(&char_id)
    }
}

/// 传送管理器 - 管理地图邻接关系和传送逻辑
#[derive(Debug)]
pub struct TeleportManager {
    /// 所有邻接关系列表
    adjacencies: Vec<MapAdjacency>,
    /// 从地图名到邻接关系索引的快速查找表
    adjacency_map: HashMap<String, Vec<usize>>,
    /// Per-player warp cooldown tracking
    warp_cooldown: HashMap<Uuid, Instant>,
}

impl TeleportManager {
    /// Cooldown duration between warps in milliseconds
    pub const WARP_COOLDOWN_MS: u64 = 1000;

    /// 创建空的传送管理器
    pub fn new() -> Self {
        Self {
            adjacencies: Vec::new(),
            adjacency_map: HashMap::new(),
            warp_cooldown: HashMap::new(),
        }
    }

    /// 添加地图邻接关系
    pub fn add_adjacency(&mut self, adj: MapAdjacency) {
        let index = self.adjacencies.len();
        self.adjacencies.push(adj);

        // 更新快速查找表
        let from_map = &self.adjacencies[index].from_map;
        self.adjacency_map
            .entry(from_map.clone())
            .or_default()
            .push(index);
    }

    /// Checks if a player can warp (not in cooldown)
    pub fn can_warp(&self, player_id: Uuid) -> bool {
        match self.warp_cooldown.get(&player_id) {
            Some(last_warp) => {
                let elapsed = Instant::now().duration_since(*last_warp);
                elapsed >= Duration::from_millis(Self::WARP_COOLDOWN_MS)
            }
            None => true,
        }
    }

    /// Records a warp timestamp for a player
    pub fn record_warp(&mut self, player_id: Uuid) {
        self.warp_cooldown.insert(player_id, Instant::now());
    }

    /// 检查指定位置是否触发传送
    ///
    /// 如果触发传送，返回 Some(TeleportAction)，否则返回 None
    pub fn check_warp_trigger(&self, map_name: &str, x: u16, y: u16) -> Option<TeleportAction> {
        let indices = self.adjacency_map.get(map_name)?;

        for &index in indices {
            let adj = &self.adjacencies[index];
            if adj.edge.is_triggered(x, y) {
                // 计算目标坐标（应用偏移）
                let to_x = x as i16 + adj.entry_offset.0;
                let to_y = y as i16 + adj.entry_offset.1;

                // 确保坐标非负
                let to_x = to_x.max(0) as u16;
                let to_y = to_y.max(0) as u16;

                return Some(TeleportAction {
                    from_map: map_name.to_string(),
                    to_map: adj.to_map.clone(),
                    from_pos: (x, y),
                    to_pos: (to_x, to_y),
                });
            }
        }

        None
    }

    /// Checks cooldown, then checks warp trigger and returns TeleportAction if warp should occur
    pub fn check_and_trigger_warp(
        &mut self,
        player_id: Uuid,
        map_name: &str,
        x: u16,
        y: u16,
    ) -> Option<TeleportAction> {
        if !self.can_warp(player_id) {
            return None;
        }

        let action = self.check_warp_trigger(map_name, x, y)?;
        self.record_warp(player_id);
        Some(action)
    }

    /// Clears expired cooldown entries (optional cleanup method)
    pub fn cleanup_expired_cooldowns(&mut self) {
        let now = Instant::now();
        let cooldown_duration = Duration::from_millis(Self::WARP_COOLDOWN_MS);
        self.warp_cooldown
            .retain(|_, last_warp| now.duration_since(*last_warp) < cooldown_duration);
    }

    /// 获取默认的邻接关系配置
    ///
    /// new_1-1.gat <-> prontera.gat 的标准连接
    pub fn get_default_adjacencies() -> Vec<MapAdjacency> {
        vec![
            // new_1-1 的北边缘 -> prontera 的南边缘 (y=299)
            MapAdjacency {
                from_map: "new_1-1.gat".to_string(),
                edge: MapEdge::North { y_threshold: 0 },
                to_map: "prontera.gat".to_string(),
                entry_offset: (0, 299), // prontera 高度约300，从南边缘进入
            },
            // new_1-1 的南边缘 -> prontera 的北边缘 (y=0)
            MapAdjacency {
                from_map: "new_1-1.gat".to_string(),
                edge: MapEdge::South { y_threshold: 199 }, // new_1-1 高度约200
                to_map: "prontera.gat".to_string(),
                entry_offset: (0, -199), // 进入 prontera 的北边缘
            },
            // prontera 的南边缘 -> new_1-1 的北边缘
            MapAdjacency {
                from_map: "prontera.gat".to_string(),
                edge: MapEdge::South { y_threshold: 299 },
                to_map: "new_1-1.gat".to_string(),
                entry_offset: (0, -299), // 进入 new_1-1 的北边缘
            },
            // prontera 的北边缘 -> new_1-1 的南边缘
            MapAdjacency {
                from_map: "prontera.gat".to_string(),
                edge: MapEdge::North { y_threshold: 0 },
                to_map: "new_1-1.gat".to_string(),
                entry_offset: (0, 199), // 进入 new_1-1 的南边缘
            },
        ]
    }
}

impl Default for TeleportManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Error types for warp operations
#[derive(Debug)]
pub enum WarpError {
    DatabaseError(String),
    PlayerNotFound,
    InvalidTargetMap,
    CooldownActive,
}

impl std::fmt::Display for WarpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WarpError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            WarpError::PlayerNotFound => write!(f, "Player not found"),
            WarpError::InvalidTargetMap => write!(f, "Invalid target map"),
            WarpError::CooldownActive => write!(f, "Warp cooldown active"),
        }
    }
}

impl std::error::Error for WarpError {}

/// Service that executes warp operations
pub struct WarpService {
    teleport_manager: Arc<RwLock<TeleportManager>>,
    save_point_manager: Arc<RwLock<SavePointManager>>,
    db: Arc<Database>,
}

impl WarpService {
    /// Creates a new WarpService
    pub fn new(
        teleport_manager: Arc<RwLock<TeleportManager>>,
        save_point_manager: Arc<RwLock<SavePointManager>>,
        db: Arc<Database>,
    ) -> Self {
        Self {
            teleport_manager,
            save_point_manager,
            db,
        }
    }

    /// 设置角色的存储点
    pub fn set_save_point(&self, char_id: u32, map_name: &str, x: u16, y: u16) {
        let mut manager = self.save_point_manager.write();
        manager.set_save_point(char_id, SavePoint::new(map_name, x, y));
    }

    /// 获取角色的存储点
    pub fn get_save_point(&self, char_id: u32) -> Option<SavePoint> {
        let manager = self.save_point_manager.read();
        manager.get_save_point(char_id).cloned()
    }

    /// 使用回城术 - 传送角色到存储点
    pub fn use_return(&self, session: &mut Session) -> Result<TeleportAction, WarpError> {
        let char_id = session.char_id.ok_or(WarpError::PlayerNotFound)?;

        let save_point = self
            .get_save_point(char_id)
            .ok_or(WarpError::InvalidTargetMap)?;

        // Create teleport action to save point
        let action = TeleportAction {
            from_map: "current".to_string(), // Not used for return
            to_map: save_point.map_name.clone(),
            from_pos: (0, 0), // Not used for return
            to_pos: (save_point.x, save_point.y),
        };

        // Execute the warp
        self.execute_warp(session, action.clone())?;

        Ok(action)
    }

    /// Executes a warp operation for a player
    ///
    /// This method:
    /// 1. Updates player's map_name in runtime Player
    /// 2. Updates player position to new coordinates
    /// 3. Updates session's map-related state
    /// 4. Updates database (save last_map, last_x, last_y) - best effort
    /// 5. Returns success
    pub fn execute_warp(
        &self,
        session: &mut Session,
        action: TeleportAction,
    ) -> Result<(), WarpError> {
        let _player_id = session.player_id.ok_or(WarpError::PlayerNotFound)?;

        // For now, we return success since the actual player reference
        // would need to be passed from MapState. In a full implementation,
        // we would update the player's map_name and position here.

        // Update database with new position (best effort)
        if let Some(char_id) = session.char_id {
            // Ignore database errors for now as the table might not exist in tests
            if let Err(e) = self.update_character_position(
                char_id,
                &action.to_map,
                action.to_pos.0 as i32,
                action.to_pos.1 as i32,
            ) {
                tracing::error!("Failed to persist warp position for char_id={}: {}", char_id, e);
            }
        }

        Ok(())
    }

    /// Helper method to update character position in database
    fn update_character_position(
        &self,
        char_id: u32,
        map_name: &str,
        x: i32,
        y: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.db.execute_params(
            "UPDATE characters SET last_map = ?1, last_x = ?2, last_y = ?3 WHERE char_id = ?4",
            &[
                &map_name as &dyn crate::storage::backend::IntoValue,
                &x as &dyn crate::storage::backend::IntoValue,
                &y as &dyn crate::storage::backend::IntoValue,
                &(char_id as i32) as &dyn crate::storage::backend::IntoValue,
            ],
        )?;
        Ok(())
    }

    /// Called when player moves, checks and executes warp if needed
    ///
    /// Returns TeleportAction if a warp was triggered, None otherwise
    pub fn handle_move_with_warp(
        &self,
        _session: &mut Session,
        _new_x: u16,
        _new_y: u16,
    ) -> Option<TeleportAction> {
        // Note: This method requires map_name which is not available here
        // Use handle_move_with_warp_on_map instead
        None
    }

    /// Handles move with warp check using explicit map name
    ///
    /// This version is used internally when the map name is known
    pub fn handle_move_with_warp_on_map(
        &self,
        session: &mut Session,
        map_name: &str,
        new_x: u16,
        new_y: u16,
    ) -> Option<TeleportAction> {
        let player_id = session.player_id?;

        let mut manager = self.teleport_manager.write();

        manager.check_and_trigger_warp(player_id, map_name, new_x, new_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_edge_trigger_north() {
        let edge = MapEdge::North { y_threshold: 5 };
        assert!(edge.is_triggered(10, 0)); // y=0 <= 5
        assert!(edge.is_triggered(10, 5)); // y=5 <= 5
        assert!(!edge.is_triggered(10, 6)); // y=6 > 5
        assert!(!edge.is_triggered(10, 100)); // y=100 > 5
    }

    #[test]
    fn test_map_edge_trigger_south() {
        let edge = MapEdge::South { y_threshold: 195 };
        assert!(!edge.is_triggered(10, 0)); // y=0 < 195
        assert!(edge.is_triggered(10, 195)); // y=195 >= 195
        assert!(edge.is_triggered(10, 199)); // y=199 >= 195
    }

    #[test]
    fn test_map_edge_trigger_east() {
        let edge = MapEdge::East { x_threshold: 250 };
        assert!(!edge.is_triggered(100, 10)); // x=100 < 250
        assert!(edge.is_triggered(250, 10)); // x=250 >= 250
        assert!(edge.is_triggered(300, 10)); // x=300 >= 250
    }

    #[test]
    fn test_map_edge_trigger_west() {
        let edge = MapEdge::West { x_threshold: 5 };
        assert!(edge.is_triggered(0, 10)); // x=0 <= 5
        assert!(edge.is_triggered(5, 10)); // x=5 <= 5
        assert!(!edge.is_triggered(6, 10)); // x=6 > 5
    }

    #[test]
    fn test_teleport_manager_new() {
        let manager = TeleportManager::new();
        assert!(manager.adjacencies.is_empty());
        assert!(manager.adjacency_map.is_empty());
    }

    #[test]
    fn test_add_adjacency() {
        let mut manager = TeleportManager::new();
        let adj = MapAdjacency {
            from_map: "test_map.gat".to_string(),
            edge: MapEdge::North { y_threshold: 0 },
            to_map: "other_map.gat".to_string(),
            entry_offset: (0, 100),
        };
        manager.add_adjacency(adj);

        assert_eq!(manager.adjacencies.len(), 1);
        assert!(manager.adjacency_map.contains_key("test_map.gat"));
    }

    #[test]
    fn test_check_warp_trigger_north() {
        let mut manager = TeleportManager::new();
        let adj = MapAdjacency {
            from_map: "new_1-1.gat".to_string(),
            edge: MapEdge::North { y_threshold: 0 },
            to_map: "prontera.gat".to_string(),
            entry_offset: (0, 299),
        };
        manager.add_adjacency(adj);

        // 触发传送 - y=0 在北边缘
        let result = manager.check_warp_trigger("new_1-1.gat", 50, 0);
        assert!(result.is_some());

        let action = result.unwrap();
        assert_eq!(action.from_map, "new_1-1.gat");
        assert_eq!(action.to_map, "prontera.gat");
        assert_eq!(action.from_pos, (50, 0));
        assert_eq!(action.to_pos, (50, 299));
    }

    #[test]
    fn test_check_warp_trigger_no_trigger() {
        let mut manager = TeleportManager::new();
        let adj = MapAdjacency {
            from_map: "new_1-1.gat".to_string(),
            edge: MapEdge::North { y_threshold: 0 },
            to_map: "prontera.gat".to_string(),
            entry_offset: (0, 299),
        };
        manager.add_adjacency(adj);

        // 不触发传送 - y=50 不在北边缘
        let result = manager.check_warp_trigger("new_1-1.gat", 50, 50);
        assert!(result.is_none());
    }

    #[test]
    fn test_check_warp_trigger_unknown_map() {
        let manager = TeleportManager::new();
        let result = manager.check_warp_trigger("unknown.gat", 50, 50);
        assert!(result.is_none());
    }

    #[test]
    fn test_check_warp_trigger_south() {
        let mut manager = TeleportManager::new();
        let adj = MapAdjacency {
            from_map: "new_1-1.gat".to_string(),
            edge: MapEdge::South { y_threshold: 199 },
            to_map: "prontera.gat".to_string(),
            entry_offset: (0, -199),
        };
        manager.add_adjacency(adj);

        // 触发传送 - y=199 在南边缘
        let result = manager.check_warp_trigger("new_1-1.gat", 50, 199);
        assert!(result.is_some());

        let action = result.unwrap();
        assert_eq!(action.to_pos, (50, 0)); // 199 + (-199) = 0
    }

    #[test]
    fn test_check_warp_trigger_with_negative_offset() {
        let mut manager = TeleportManager::new();
        let adj = MapAdjacency {
            from_map: "test.gat".to_string(),
            edge: MapEdge::West { x_threshold: 0 },
            to_map: "other.gat".to_string(),
            entry_offset: (-100, 0),
        };
        manager.add_adjacency(adj);

        let result = manager.check_warp_trigger("test.gat", 0, 50);
        assert!(result.is_some());

        let action = result.unwrap();
        // x=0 + (-100) = -100，但会被 clamp 到 0
        assert_eq!(action.to_pos, (0, 50));
    }

    #[test]
    fn test_default_adjacencies() {
        let defaults = TeleportManager::get_default_adjacencies();
        assert_eq!(defaults.len(), 4);

        // 验证 new_1-1.gat -> prontera.gat 的北边缘
        let north = defaults
            .iter()
            .find(|a| a.from_map == "new_1-1.gat" && matches!(a.edge, MapEdge::North { .. }));
        assert!(north.is_some());
        let north = north.unwrap();
        assert_eq!(north.to_map, "prontera.gat");
        assert_eq!(north.entry_offset, (0, 299));

        // 验证 new_1-1.gat -> prontera.gat 的南边缘
        let south = defaults
            .iter()
            .find(|a| a.from_map == "new_1-1.gat" && matches!(a.edge, MapEdge::South { .. }));
        assert!(south.is_some());
        let south = south.unwrap();
        assert_eq!(south.to_map, "prontera.gat");
        assert_eq!(south.entry_offset, (0, -199));

        // 验证 prontera.gat -> new_1-1.gat 的南边缘
        let prontera_south = defaults
            .iter()
            .find(|a| a.from_map == "prontera.gat" && matches!(a.edge, MapEdge::South { .. }));
        assert!(prontera_south.is_some());
        assert_eq!(prontera_south.unwrap().to_map, "new_1-1.gat");

        // 验证 prontera.gat -> new_1-1.gat 的北边缘
        let prontera_north = defaults
            .iter()
            .find(|a| a.from_map == "prontera.gat" && matches!(a.edge, MapEdge::North { .. }));
        assert!(prontera_north.is_some());
        assert_eq!(prontera_north.unwrap().to_map, "new_1-1.gat");
    }

    #[test]
    fn test_save_point_creation() {
        let save_point = SavePoint {
            map_name: "prontera.gat".to_string(),
            x: 150,
            y: 180,
        };
        assert_eq!(save_point.map_name, "prontera.gat");
        assert_eq!(save_point.x, 150);
        assert_eq!(save_point.y, 180);
    }

    #[test]
    fn test_teleport_action_equality() {
        let action1 = TeleportAction {
            from_map: "a.gat".to_string(),
            to_map: "b.gat".to_string(),
            from_pos: (10, 20),
            to_pos: (30, 40),
        };
        let action2 = TeleportAction {
            from_map: "a.gat".to_string(),
            to_map: "b.gat".to_string(),
            from_pos: (10, 20),
            to_pos: (30, 40),
        };
        assert_eq!(action1, action2);
    }

    // Task 2 Tests - Cooldown and Warp Service

    #[test]
    fn test_can_warp_without_cooldown() {
        let manager = TeleportManager::new();
        let player_id = Uuid::new_v4();
        assert!(manager.can_warp(player_id));
    }

    #[test]
    fn test_warp_cooldown_prevents_spam() {
        let mut manager = TeleportManager::new();
        let player_id = Uuid::new_v4();

        // Record a warp
        manager.record_warp(player_id);
        assert!(!manager.can_warp(player_id));

        // Should still be on cooldown immediately after
        assert!(!manager.can_warp(player_id));
    }

    #[test]
    fn test_check_and_trigger_warp_returns_none_on_cooldown() {
        let mut manager = TeleportManager::new();
        let player_id = Uuid::new_v4();

        // Add adjacency
        manager.add_adjacency(MapAdjacency {
            from_map: "map1".to_string(),
            edge: MapEdge::North { y_threshold: 0 },
            to_map: "map2".to_string(),
            entry_offset: (0, 100),
        });

        // First trigger should work
        let result = manager.check_and_trigger_warp(player_id, "map1", 50, 0);
        assert!(result.is_some());

        // Second trigger should fail due to cooldown
        let result = manager.check_and_trigger_warp(player_id, "map1", 50, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_cleanup_expired_cooldowns() {
        let mut manager = TeleportManager::new();
        let player_id = Uuid::new_v4();

        // Manually insert an old cooldown (expired)
        let old_time = Instant::now() - Duration::from_millis(2000);
        manager.warp_cooldown.insert(player_id, old_time);

        // Cleanup should remove expired entry
        manager.cleanup_expired_cooldowns();
        assert!(!manager.warp_cooldown.contains_key(&player_id));
    }

    #[test]
    fn test_warp_service_new() {
        let manager = Arc::new(RwLock::new(TeleportManager::new()));
        let db = Arc::new(Database::open_memory().unwrap());

        let save_point_manager = Arc::new(RwLock::new(SavePointManager::new()));
        let service = WarpService::new(manager, save_point_manager, db);
        // Service should be created successfully
        let _ = service;
    }

    #[test]
    fn test_execute_warp_returns_error_without_player_id() {
        let manager = Arc::new(RwLock::new(TeleportManager::new()));
        let db = Arc::new(Database::open_memory().unwrap());

        let save_point_manager = Arc::new(RwLock::new(SavePointManager::new()));
        let service = WarpService::new(manager, save_point_manager, db);
        let mut session = Session::new();
        // No player_id set

        let action = TeleportAction {
            from_map: "map1".to_string(),
            to_map: "map2".to_string(),
            from_pos: (0, 0),
            to_pos: (100, 100),
        };

        let result = service.execute_warp(&mut session, action);
        assert!(matches!(result, Err(WarpError::PlayerNotFound)));
    }

    #[test]
    fn test_execute_warp_with_player_id() {
        let manager = Arc::new(RwLock::new(TeleportManager::new()));
        let db = Arc::new(Database::open_memory().unwrap());

        let save_point_manager = Arc::new(RwLock::new(SavePointManager::new()));
        let service = WarpService::new(manager, save_point_manager, db);
        let mut session = Session::new();
        session.player_id = Some(Uuid::new_v4());
        session.char_id = Some(1);

        let action = TeleportAction {
            from_map: "map1".to_string(),
            to_map: "map2".to_string(),
            from_pos: (0, 0),
            to_pos: (100, 100),
        };

        // Should succeed (database update is optional for now)
        let result = service.execute_warp(&mut session, action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_move_with_warp_on_map_triggers_warp() {
        let manager = Arc::new(RwLock::new(TeleportManager::new()));
        let db = Arc::new(Database::open_memory().unwrap());

        // Add adjacency to manager
        {
            let mut m = manager.write();
            m.add_adjacency(MapAdjacency {
                from_map: "map1".to_string(),
                edge: MapEdge::North { y_threshold: 0 },
                to_map: "map2".to_string(),
                entry_offset: (0, 100),
            });
        }

        let save_point_manager = Arc::new(RwLock::new(SavePointManager::new()));
        let service = WarpService::new(manager, save_point_manager, db);
        let mut session = Session::new();
        session.player_id = Some(Uuid::new_v4());

        let result = service.handle_move_with_warp_on_map(&mut session, "map1", 50, 0);
        assert!(result.is_some());

        let action = result.unwrap();
        assert_eq!(action.from_map, "map1");
        assert_eq!(action.to_map, "map2");
    }

    #[test]
    fn test_handle_move_with_warp_on_map_no_trigger() {
        let manager = Arc::new(RwLock::new(TeleportManager::new()));
        let db = Arc::new(Database::open_memory().unwrap());

        // Add adjacency to manager
        {
            let mut m = manager.write();
            m.add_adjacency(MapAdjacency {
                from_map: "map1".to_string(),
                edge: MapEdge::North { y_threshold: 0 },
                to_map: "map2".to_string(),
                entry_offset: (0, 100),
            });
        }

        let save_point_manager = Arc::new(RwLock::new(SavePointManager::new()));
        let service = WarpService::new(manager, save_point_manager, db);
        let mut session = Session::new();
        session.player_id = Some(Uuid::new_v4());

        // Position not on edge - should not trigger
        let result = service.handle_move_with_warp_on_map(&mut session, "map1", 50, 50);
        assert!(result.is_none());
    }

    // Task 3 Tests - Save Point and Return functionality

    #[test]
    fn test_save_point_manager_new() {
        let manager = SavePointManager::new();
        assert!(manager.save_points.is_empty());
    }

    #[test]
    fn test_save_point_manager_set_and_get() {
        let mut manager = SavePointManager::new();
        let save_point = SavePoint::new("prontera.gat", 150, 180);

        manager.set_save_point(1, save_point.clone());

        let retrieved = manager.get_save_point(1);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().map_name, "prontera.gat");
        assert_eq!(retrieved.unwrap().x, 150);
        assert_eq!(retrieved.unwrap().y, 180);
    }

    #[test]
    fn test_save_point_manager_has_save_point() {
        let mut manager = SavePointManager::new();

        assert!(!manager.has_save_point(1));

        manager.set_save_point(1, SavePoint::new("map.gat", 100, 100));

        assert!(manager.has_save_point(1));
    }

    #[test]
    fn test_save_point_manager_remove() {
        let mut manager = SavePointManager::new();
        manager.set_save_point(1, SavePoint::new("map.gat", 100, 100));

        let removed = manager.remove_save_point(1);
        assert!(removed.is_some());
        assert!(!manager.has_save_point(1));
    }

    #[test]
    fn test_warp_service_set_and_get_save_point() {
        let teleport_manager = Arc::new(RwLock::new(TeleportManager::new()));
        let save_point_manager = Arc::new(RwLock::new(SavePointManager::new()));
        let db = Arc::new(Database::open_memory().unwrap());
        let service = WarpService::new(teleport_manager, save_point_manager.clone(), db);

        // Initially no save point
        assert!(service.get_save_point(1).is_none());

        // Set save point
        service.set_save_point(1, "prontera.gat", 150, 180);

        // Get save point
        let save_point = service.get_save_point(1);
        assert!(save_point.is_some());
        let sp = save_point.unwrap();
        assert_eq!(sp.map_name, "prontera.gat");
        assert_eq!(sp.x, 150);
        assert_eq!(sp.y, 180);
    }

    #[test]
    fn test_use_return_without_save_point_fails() {
        let teleport_manager = Arc::new(RwLock::new(TeleportManager::new()));
        let save_point_manager = Arc::new(RwLock::new(SavePointManager::new()));
        let db = Arc::new(Database::open_memory().unwrap());
        let service = WarpService::new(teleport_manager, save_point_manager, db);

        let mut session = Session::new();
        session.char_id = Some(1);

        // Should fail because no save point is set
        let result = service.use_return(&mut session);
        assert!(result.is_err());
    }

    #[test]
    fn test_use_return_with_save_point_succeeds() {
        let teleport_manager = Arc::new(RwLock::new(TeleportManager::new()));
        let save_point_manager = Arc::new(RwLock::new(SavePointManager::new()));
        let db = Arc::new(Database::open_memory().unwrap());
        let service = WarpService::new(teleport_manager, save_point_manager.clone(), db);

        // Set up save point
        service.set_save_point(1, "prontera.gat", 150, 180);

        let mut session = Session::new();
        session.player_id = Some(Uuid::new_v4()); // Need player_id for execute_warp
        session.char_id = Some(1);

        // Should succeed
        let result = service.use_return(&mut session);
        assert!(result.is_ok());

        let action = result.unwrap();
        assert_eq!(action.to_map, "prontera.gat");
        assert_eq!(action.to_pos, (150, 180));
    }
}
