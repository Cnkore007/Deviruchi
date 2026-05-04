//! 宠物AI模块
//!
//! 宠物AI负责宠物的跟随、战斗协助和拾取行为

use crate::game::map::player::Player;
use crate::game::pet::data::Pet;
use parking_lot::RwLock;
use std::collections::VecDeque;
use uuid::Uuid;

/// 宠物AI状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetAIState {
    Idle,   // 空闲状态
    Follow, // 跟随主人
    Attack, // 协助攻击
    Pickup, // 拾取物品
    Return, // 返回主人身边
}

/// 宠物AI行为
pub struct PetAI {
    /// 所属玩家ID
    player_id: Uuid,
    /// 当前状态
    state: RwLock<PetAIState>,
    /// 目标位置（宠物应到达的位置）
    target_pos: RwLock<(u16, u16)>,
    /// 移动路径
    path: RwLock<VecDeque<(u16, u16)>>,
    /// 跟随距离
    follow_distance: u16,
    /// 拾取范围
    pickup_range: u16,
    /// 攻击冷却
    attack_cooldown: RwLock<u32>,
    /// 最后更新时间
    last_update: RwLock<u64>,
}

impl PetAI {
    pub fn new(player_id: Uuid) -> Self {
        Self {
            player_id,
            state: RwLock::new(PetAIState::Idle),
            target_pos: RwLock::new((0, 0)),
            path: RwLock::new(VecDeque::new()),
            follow_distance: 3, // 3格距离
            pickup_range: 2,    // 2格范围内拾取
            attack_cooldown: RwLock::new(0),
            last_update: RwLock::new(0),
        }
    }

    /// 获取当前状态
    pub fn get_state(&self) -> PetAIState {
        *self.state.read()
    }

    /// 设置目标位置
    pub fn set_target(&self, x: u16, y: u16) {
        *self.target_pos.write() = (x, y);
    }

    /// 获取目标位置
    pub fn get_target(&self) -> (u16, u16) {
        *self.target_pos.read()
    }

    /// 检查是否需要移动到目标
    pub fn needs_move(&self, current_pos: (u16, u16)) -> bool {
        let target = *self.target_pos.read();
        let dx = (current_pos.0 as i32 - target.0 as i32).abs();
        let dy = (current_pos.1 as i32 - target.1 as i32).abs();
        dx > self.follow_distance as i32 || dy > self.follow_distance as i32
    }

    /// 更新跟随目标（基于主人位置）
    pub fn update_follow_target(&self, owner_pos: (u16, u16)) {
        // 宠物站在主人身后1格
        let (ox, oy) = owner_pos;
        *self.target_pos.write() = (ox.saturating_sub(1), oy.saturating_sub(1));
    }

    /// 进入攻击状态
    pub fn enter_attack_state(&self) {
        *self.state.write() = PetAIState::Attack;
        *self.attack_cooldown.write() = 0;
    }

    /// 进入拾取状态
    pub fn enter_pickup_state(&self) {
        *self.state.write() = PetAIState::Pickup;
    }

    /// 进入空闲状态
    pub fn enter_idle_state(&self) {
        *self.state.write() = PetAIState::Idle;
    }

    /// 进入跟随状态
    pub fn enter_follow_state(&self) {
        *self.state.write() = PetAIState::Follow;
    }

    /// 冷却攻击
    pub fn cooldown_attack(&self) {
        *self.attack_cooldown.write() = 3; // 3秒冷却
    }

    /// 检查是否可以攻击
    pub fn can_attack(&self) -> bool {
        *self.attack_cooldown.read() == 0
    }

    /// 减少攻击冷却
    pub fn tick_cooldown(&self) {
        let cooldown = *self.attack_cooldown.read();
        if cooldown > 0 {
            *self.attack_cooldown.write() = cooldown - 1;
        }
    }

    /// 更新最后更新时间
    pub fn update_timestamp(&self, timestamp: u64) {
        *self.last_update.write() = timestamp;
    }

    /// 获取最后更新时间
    pub fn get_last_update(&self) -> u64 {
        *self.last_update.read()
    }

    /// 计算移动路径（A*算法，这里简化使用直线移动）
    pub fn calculate_path(&self, current_pos: (u16, u16)) {
        let target = *self.target_pos.read();
        let mut path = VecDeque::new();

        let mut cx = current_pos.0;
        let mut cy = current_pos.1;

        // 简单的直线移动
        while cx != target.0 || cy != target.1 {
            if cx < target.0 {
                cx += 1;
            } else if cx > target.0 {
                cx -= 1;
            }
            if cy < target.1 {
                cy += 1;
            } else if cy > target.1 {
                cy -= 1;
            }

            if cx != target.0 || cy != target.1 {
                path.push_back((cx, cy));
            }
        }

        *self.path.write() = path;
    }

    /// 获取下一个路径点
    pub fn get_next_path_point(&self) -> Option<(u16, u16)> {
        self.path.write().pop_front()
    }

    /// 清空路径
    pub fn clear_path(&self) {
        self.path.write().clear();
    }

    /// 检查是否在拾取范围内
    pub fn in_pickup_range(&self, item_pos: (u16, u16), owner_pos: (u16, u16)) -> bool {
        let dx = (item_pos.0 as i32 - owner_pos.0 as i32).abs();
        let dy = (item_pos.1 as i32 - owner_pos.1 as i32).abs();
        dx <= self.pickup_range as i32 && dy <= self.pickup_range as i32
    }

    /// 获取跟随距离
    pub fn get_follow_distance(&self) -> u16 {
        self.follow_distance
    }

    /// 设置跟随距离
    pub fn set_follow_distance(&mut self, distance: u16) {
        self.follow_distance = distance.min(10); // 最大10格
    }

    /// AI主循环更新
    pub fn update(&self, owner: &Player, _pet: &Pet) {
        // 更新时间戳
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.update_timestamp(now);

        // 更新攻击冷却
        self.tick_cooldown();

        // 根据主人状态决定行为
        if !owner.is_alive() {
            // 主人死亡，宠物进入空闲
            self.enter_idle_state();
            return;
        }

        let owner_pos = owner.get_position();
        self.update_follow_target(owner_pos);

        // 检查是否需要移动
        let current_pos = *self.target_pos.read();
        if self.needs_move(current_pos) {
            self.enter_follow_state();
        } else {
            self.enter_idle_state();
        }
    }
}

/// 宠物AI管理器
pub struct PetAIManager {
    /// 所有宠物AI实例 (player_id -> PetAI)
    ai_instances: RwLock<std::collections::HashMap<Uuid, PetAI>>,
}

impl PetAIManager {
    pub fn new() -> Self {
        Self {
            ai_instances: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// 为玩家创建宠物AI
    pub fn create_ai(&self, player_id: Uuid) -> PetAI {
        let ai = PetAI::new(player_id);
        self.ai_instances.write().insert(player_id, ai.clone());
        ai
    }

    /// 获取玩家的宠物AI
    pub fn get_ai(&self, player_id: Uuid) -> Option<PetAI> {
        self.ai_instances.read().get(&player_id).cloned()
    }

    /// 移除玩家的宠物AI
    pub fn remove_ai(&self, player_id: Uuid) {
        self.ai_instances.write().remove(&player_id);
    }

    /// 更新所有宠物AI
    pub fn update_all(&self, player: &Player, pet: &Pet) {
        if let Some(ai) = self.get_ai(player.id) {
            ai.update(player, pet);
        }
    }

    /// 获取AI状态
    pub fn get_ai_state(&self, player_id: Uuid) -> Option<PetAIState> {
        self.get_ai(player_id).map(|ai| ai.get_state())
    }
}

impl Default for PetAIManager {
    fn default() -> Self {
        Self::new()
    }
}

// 为PetAI实现Clone以支持上述用法
impl Clone for PetAI {
    fn clone(&self) -> Self {
        Self {
            player_id: self.player_id,
            state: RwLock::new(*self.state.read()),
            target_pos: RwLock::new(*self.target_pos.read()),
            path: RwLock::new(self.path.read().clone()),
            follow_distance: self.follow_distance,
            pickup_range: self.pickup_range,
            attack_cooldown: RwLock::new(*self.attack_cooldown.read()),
            last_update: RwLock::new(*self.last_update.read()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pet_ai_creation() {
        let player_id = Uuid::new_v4();
        let ai = PetAI::new(player_id);
        assert_eq!(ai.get_state(), PetAIState::Idle);
    }

    #[test]
    fn test_pet_ai_state_transitions() {
        let ai = PetAI::new(Uuid::new_v4());

        ai.enter_attack_state();
        assert_eq!(ai.get_state(), PetAIState::Attack);

        ai.enter_pickup_state();
        assert_eq!(ai.get_state(), PetAIState::Pickup);

        ai.enter_follow_state();
        assert_eq!(ai.get_state(), PetAIState::Follow);

        ai.enter_idle_state();
        assert_eq!(ai.get_state(), PetAIState::Idle);
    }

    #[test]
    fn test_pet_ai_target() {
        let ai = PetAI::new(Uuid::new_v4());
        ai.set_target(100, 200);
        assert_eq!(ai.get_target(), (100, 200));
    }

    #[test]
    fn test_pet_ai_needs_move() {
        let ai = PetAI::new(Uuid::new_v4());
        ai.set_target(100, 100);

        // 在范围内，不需要移动
        assert!(!ai.needs_move((102, 102)));

        // 超出范围，需要移动
        assert!(ai.needs_move((110, 110)));
    }

    #[test]
    fn test_pet_ai_attack_cooldown() {
        let ai = PetAI::new(Uuid::new_v4());

        assert!(ai.can_attack());

        ai.cooldown_attack();
        assert!(!ai.can_attack());

        ai.tick_cooldown();
        assert!(!ai.can_attack());

        ai.tick_cooldown();
        assert!(!ai.can_attack());

        ai.tick_cooldown();
        assert!(ai.can_attack());
    }

    #[test]
    fn test_pet_ai_manager() {
        let manager = PetAIManager::new();
        let player_id = Uuid::new_v4();

        // 创建AI
        let ai = manager.create_ai(player_id);
        assert!(manager.get_ai(player_id).is_some());

        // 获取状态
        assert_eq!(manager.get_ai_state(player_id), Some(PetAIState::Idle));

        // 移除AI
        manager.remove_ai(player_id);
        assert!(manager.get_ai(player_id).is_none());
    }

    #[test]
    fn test_pet_ai_pickup_range() {
        let ai = PetAI::new(Uuid::new_v4());

        // 在拾取范围内
        assert!(ai.in_pickup_range((100, 100), (100, 100)));

        // 边界
        assert!(ai.in_pickup_range((102, 102), (100, 100)));

        // 超出范围
        assert!(!ai.in_pickup_range((105, 105), (100, 100)));
    }
}
