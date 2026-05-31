//! 单位移动系统
//!
//! 对应 rAthena 的 `src/map/unit.cpp`，提供单位移动、碰撞检测、寻路等功能。
//! 用于玩家移动、怪物 AI、NPC 移动等。

use std::collections::VecDeque;
use parking_lot::RwLock;
use uuid::Uuid;
use crate::game::path::{Direction, PathSearcher, Point};

/// 移动状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveState {
    /// 静止
    Idle,
    /// 正在移动
    Walking,
    /// 被击退
    Knockback,
    /// 传送中
    Warping,
}

/// 移动类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveType {
    /// 正常行走
    Walk,
    /// 跑步（速度 x2）
    Run,
    /// 飞行（忽略碰撞）
    Fly,
    /// 瞬移（无动画）
    Teleport,
}

/// 单位移动数据
#[derive(Debug, Clone)]
pub struct UnitMovement {
    /// 单位 ID
    pub unit_id: Uuid,
    /// 当前位置
    pub position: Point,
    /// 目标位置
    pub target: Option<Point>,
    /// 移动路径
    pub path: VecDeque<Point>,
    /// 移动状态
    pub state: MoveState,
    /// 移动类型
    pub move_type: MoveType,
    /// 移动速度（毫秒/格）
    pub speed: u32,
    /// 方向
    pub direction: Direction,
    /// 上次移动时间
    pub last_move_time: u64,
    /// 移动开始时间
    pub move_start_time: u64,
}

impl UnitMovement {
    /// 创建新的单位移动数据
    pub fn new(unit_id: Uuid, x: i32, y: i32) -> Self {
        Self {
            unit_id,
            position: Point::new(x, y),
            target: None,
            path: VecDeque::new(),
            state: MoveState::Idle,
            move_type: MoveType::Walk,
            speed: 150, // 默认 150ms/格
            direction: Direction::South,
            last_move_time: 0,
            move_start_time: 0,
        }
    }

    /// 设置目标位置
    pub fn set_target(&mut self, target: Point) {
        self.target = Some(target);
    }

    /// 设置路径
    pub fn set_path(&mut self, path: Vec<Point>) {
        self.path = path.into_iter().collect();
        self.state = MoveState::Walking;
    }

    /// 清除路径
    pub fn clear_path(&mut self) {
        self.path.clear();
        self.target = None;
        self.state = MoveState::Idle;
    }

    /// 获取下一个路径点
    pub fn next_waypoint(&mut self) -> Option<Point> {
        self.path.pop_front()
    }

    /// 检查是否正在移动
    pub fn is_moving(&self) -> bool {
        self.state == MoveState::Walking && !self.path.is_empty()
    }

    /// 更新位置
    pub fn update_position(&mut self, x: i32, y: i32) {
        self.position = Point::new(x, y);
    }

    /// 更新方向
    pub fn update_direction(&mut self, dir: Direction) {
        self.direction = dir;
    }
}

/// 移动验证器
///
/// 验证移动是否合法，检查碰撞、速度等。
pub struct MovementValidator {
    /// 最大移动速度（防止加速作弊）
    max_speed: u32,
    /// 最小移动速度
    min_speed: u32,
    /// 允许飞行
    allow_fly: bool,
}

impl MovementValidator {
    /// 创建移动验证器
    pub fn new() -> Self {
        Self {
            max_speed: 100, // 最快 100ms/格
            min_speed: 1000, // 最慢 1000ms/格
            allow_fly: false,
        }
    }

    /// 验证移动速度
    pub fn validate_speed(&self, speed: u32) -> bool {
        speed >= self.max_speed && speed <= self.min_speed
    }

    /// 验证移动目标
    pub fn validate_target(&self, from: Point, to: Point, max_distance: i32) -> bool {
        let distance = PathSearcher::distance(from, to);
        distance <= max_distance
    }

    /// 验证方向
    pub fn validate_direction(&self, from: Point, to: Point) -> Direction {
        let dx = to.x - from.x;
        let dy = to.y - from.y;

        if dx == 0 && dy == 0 {
            return Direction::South; // 默认方向
        }

        let angle = (dy as f64).atan2(dx as f64);
        let degrees = angle * 180.0 / std::f64::consts::PI;

        // 将角度转换为 8 方向
        if degrees >= -22.5 && degrees < 22.5 {
            Direction::East
        } else if degrees >= 22.5 && degrees < 67.5 {
            Direction::SouthEast
        } else if degrees >= 67.5 && degrees < 112.5 {
            Direction::South
        } else if degrees >= 112.5 && degrees < 157.5 {
            Direction::SouthWest
        } else if degrees >= 157.5 || degrees < -157.5 {
            Direction::West
        } else if degrees >= -157.5 && degrees < -112.5 {
            Direction::NorthWest
        } else if degrees >= -112.5 && degrees < -67.5 {
            Direction::North
        } else {
            Direction::NorthEast
        }
    }
}

impl Default for MovementValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// 单位移动管理器
///
/// 管理所有单位的移动状态和验证。
pub struct UnitManager {
    /// 单位移动数据 (unit_id -> UnitMovement)
    units: RwLock<dashmap::DashMap<Uuid, UnitMovement>>,
    /// 移动验证器
    validator: MovementValidator,
    /// 路径搜索器
    pathfinder: PathSearcher,
}

impl UnitManager {
    /// 创建单位管理器
    pub fn new() -> Self {
        Self {
            units: RwLock::new(dashmap::DashMap::new()),
            validator: MovementValidator::new(),
            pathfinder: PathSearcher::new(),
        }
    }

    /// 注册单位
    pub fn register_unit(&self, unit_id: Uuid, x: i32, y: i32) {
        let movement = UnitMovement::new(unit_id, x, y);
        self.units.write().insert(unit_id, movement);
    }

    /// 移除单位
    pub fn remove_unit(&self, unit_id: &Uuid) -> Option<UnitMovement> {
        self.units.write().remove(unit_id).map(|(_, v)| v)
    }

    /// 获取单位位置
    pub fn get_position(&self, unit_id: &Uuid) -> Option<Point> {
        self.units.read().get(unit_id).map(|u| u.position)
    }

    /// 获取单位移动状态
    pub fn get_movement(&self, unit_id: &Uuid) -> Option<UnitMovement> {
        self.units.read().get(unit_id).map(|u| u.clone())
    }

    /// 设置单位目标位置
    pub fn set_target(&self, unit_id: &Uuid, target: Point) -> Result<(), MoveError> {
        let units = self.units.read();
        let mut unit = units.get_mut(unit_id).ok_or(MoveError::UnitNotFound)?;

        // 验证目标距离
        if !self.validator.validate_target(unit.position, target, 50) {
            return Err(MoveError::TargetTooFar);
        }

        unit.set_target(target);
        Ok(())
    }

    /// 设置单位路径
    pub fn set_path(&self, unit_id: &Uuid, path: Vec<Point>) -> Result<(), MoveError> {
        let units = self.units.read();
        let mut unit = units.get_mut(unit_id).ok_or(MoveError::UnitNotFound)?;

        unit.set_path(path);
        Ok(())
    }

    /// 搜索单位到目标的路径
    pub fn find_path(&self, unit_id: &Uuid, target: Point) -> Result<Vec<Point>, MoveError> {
        let units = self.units.read();
        let unit = units.get(unit_id).ok_or(MoveError::UnitNotFound)?;

        let path_result = self.pathfinder.search(
            &NullCollisionMap, // 需要实际的地图碰撞检测
            unit.position,
            target,
        );

        if path_result.reached {
            Ok(path_result.path)
        } else {
            Err(MoveError::NoPath)
        }
    }

    /// 移动单位到下一个路径点
    pub fn move_to_next(&self, unit_id: &Uuid) -> Result<Point, MoveError> {
        let units = self.units.read();
        let mut unit = units.get_mut(unit_id).ok_or(MoveError::UnitNotFound)?;

        if let Some(next) = unit.next_waypoint() {
            let dir = self.validator.validate_direction(unit.position, next);
            unit.update_direction(dir);
            unit.update_position(next.x, next.y);
            unit.last_move_time = current_time_ms();
            Ok(next)
        } else {
            unit.state = MoveState::Idle;
            Err(MoveError::NoPath)
        }
    }

    /// 停止单位移动
    pub fn stop_movement(&self, unit_id: &Uuid) {
        if let Some(mut unit) = self.units.write().get_mut(unit_id) {
            unit.clear_path();
        }
    }

    /// 击退单位
    pub fn knockback(&self, unit_id: &Uuid, distance: i32, direction: Direction) -> Result<Point, MoveError> {
        let units = self.units.read();
        let mut unit = units.get_mut(unit_id).ok_or(MoveError::UnitNotFound)?;

        let (dx, dy) = direction.offset();
        let new_x = unit.position.x + dx * distance;
        let new_y = unit.position.y + dy * distance;

        unit.state = MoveState::Knockback;
        unit.update_position(new_x, new_y);
        unit.update_direction(direction.opposite());

        Ok(Point::new(new_x, new_y))
    }

    /// 传送单位
    pub fn warp_unit(&self, unit_id: &Uuid, target: Point) -> Result<(), MoveError> {
        let units = self.units.read();
        let mut unit = units.get_mut(unit_id).ok_or(MoveError::UnitNotFound)?;

        unit.state = MoveState::Warping;
        unit.update_position(target.x, target.y);
        unit.clear_path();

        Ok(())
    }

    /// 更新单位移动（每帧调用）
    pub fn update_movement(&self, unit_id: &Uuid, current_time: u64) -> Option<Point> {
        let units = self.units.read();
        let mut unit = units.get_mut(unit_id)?;

        if !unit.is_moving() {
            return None;
        }

        let elapsed = current_time.saturating_sub(unit.last_move_time);
        if elapsed >= unit.speed as u64 {
            if let Some(next) = unit.next_waypoint() {
                let dir = self.validator.validate_direction(unit.position, next);
                unit.update_direction(dir);
                unit.update_position(next.x, next.y);
                unit.last_move_time = current_time;
                return Some(next);
            }
        }

        None
    }

    /// 获取单位总数
    pub fn unit_count(&self) -> usize {
        self.units.read().len()
    }
}

impl Default for UnitManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 移动错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveError {
    /// 单位不存在
    UnitNotFound,
    /// 目标太远
    TargetTooFar,
    /// 无法到达
    NoPath,
    /// 移动被阻止
    Blocked,
    /// 速度无效
    InvalidSpeed,
}

/// 空碰撞地图（用于测试）
struct NullCollisionMap;

impl crate::game::path::CollisionMap for NullCollisionMap {
    fn is_walkable(&self, _x: i32, _y: i32) -> bool {
        true
    }

    fn width(&self) -> i32 {
        1000
    }

    fn height(&self) -> i32 {
        1000
    }
}

/// 获取当前时间戳（毫秒）
fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_movement_new() {
        let id = Uuid::new_v4();
        let movement = UnitMovement::new(id, 100, 200);

        assert_eq!(movement.position, Point::new(100, 200));
        assert_eq!(movement.state, MoveState::Idle);
        assert_eq!(movement.speed, 150);
    }

    #[test]
    fn test_unit_movement_path() {
        let id = Uuid::new_v4();
        let mut movement = UnitMovement::new(id, 0, 0);

        let path = vec![
            Point::new(1, 0),
            Point::new(2, 0),
            Point::new(3, 0),
        ];

        movement.set_path(path);
        assert_eq!(movement.state, MoveState::Walking);
        assert_eq!(movement.path.len(), 3);

        assert_eq!(movement.next_waypoint(), Some(Point::new(1, 0)));
        assert_eq!(movement.path.len(), 2);
    }

    #[test]
    fn test_movement_validator() {
        let validator = MovementValidator::new();

        assert!(validator.validate_speed(150));
        assert!(!validator.validate_speed(50)); // 太快
        assert!(!validator.validate_speed(2000)); // 太慢
    }

    #[test]
    fn test_movement_validator_direction() {
        let validator = MovementValidator::new();

        let from = Point::new(0, 0);
        let to = Point::new(1, 0);
        assert_eq!(validator.validate_direction(from, to), Direction::East);

        let to = Point::new(0, 1);
        assert_eq!(validator.validate_direction(from, to), Direction::South);
    }

    #[test]
    fn test_unit_manager_register() {
        let manager = UnitManager::new();
        let id = Uuid::new_v4();

        manager.register_unit(id, 100, 200);
        assert_eq!(manager.unit_count(), 1);

        let pos = manager.get_position(&id);
        assert_eq!(pos, Some(Point::new(100, 200)));
    }

    #[test]
    fn test_unit_manager_remove() {
        let manager = UnitManager::new();
        let id = Uuid::new_v4();

        manager.register_unit(id, 100, 200);
        manager.remove_unit(&id);

        assert_eq!(manager.unit_count(), 0);
    }

    #[test]
    fn test_unit_manager_set_target() {
        let manager = UnitManager::new();
        let id = Uuid::new_v4();

        manager.register_unit(id, 0, 0);
        let result = manager.set_target(&id, Point::new(10, 10));

        assert!(result.is_ok());
    }

    #[test]
    fn test_unit_manager_set_path() {
        let manager = UnitManager::new();
        let id = Uuid::new_v4();

        manager.register_unit(id, 0, 0);

        let path = vec![
            Point::new(1, 0),
            Point::new(2, 0),
        ];

        let result = manager.set_path(&id, path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unit_manager_move_to_next() {
        let manager = UnitManager::new();
        let id = Uuid::new_v4();

        manager.register_unit(id, 0, 0);

        let path = vec![Point::new(1, 0)];
        manager.set_path(&id, path).unwrap();

        let result = manager.move_to_next(&id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Point::new(1, 0));
    }

    #[test]
    fn test_unit_manager_stop() {
        let manager = UnitManager::new();
        let id = Uuid::new_v4();

        manager.register_unit(id, 0, 0);
        manager.set_path(&id, vec![Point::new(1, 0)]).unwrap();

        manager.stop_movement(&id);

        let movement = manager.get_movement(&id).unwrap();
        assert_eq!(movement.state, MoveState::Idle);
    }

    #[test]
    fn test_unit_manager_knockback() {
        let manager = UnitManager::new();
        let id = Uuid::new_v4();

        manager.register_unit(id, 5, 5);

        let result = manager.knockback(&id, 3, Direction::North);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Point::new(5, 2));
    }

    #[test]
    fn test_unit_manager_warp() {
        let manager = UnitManager::new();
        let id = Uuid::new_v4();

        manager.register_unit(id, 0, 0);

        let result = manager.warp_unit(&id, Point::new(100, 100));
        assert!(result.is_ok());

        let pos = manager.get_position(&id).unwrap();
        assert_eq!(pos, Point::new(100, 100));
    }
}
