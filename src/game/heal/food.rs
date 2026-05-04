//! 食物/药水效果系统
//!
//! 管理食物使用和持续回复效果

use crate::game::map::Player;
use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

/// 食物效果
#[derive(Debug, Clone)]
pub struct FoodEffect {
    /// 物品ID
    pub item_id: u32,
    /// 饱食度恢复量
    pub hunger_restore: u32,
    /// 每tick HP回复增量
    pub hp_per_tick: u32,
    /// 每tick SP回复增量
    pub sp_per_tick: u32,
    /// 效果持续时间(毫秒)
    pub duration_ms: u64,
    /// 冷却时间(毫秒)
    pub cooldown_ms: u64,
}

/// 玩家的食物效果状态
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PlayerFoodState {
    /// 玩家ID
    player_id: Uuid,
    /// 活跃的食物效果
    active_effects: Vec<ActiveFoodEffect>,
    /// 上次使用食物时间
    last_use_time: HashMap<u32, u64>, // item_id -> last_use_timestamp
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ActiveFoodEffect {
    /// 物品ID
    item_id: u32,
    /// 开始时间戳
    start_time: u64,
    /// 持续时间
    duration_ms: u64,
    /// 每tick HP回复
    hp_per_tick: u32,
    /// 每tick SP回复
    sp_per_tick: u32,
}

impl PlayerFoodState {
    pub fn new(player_id: Uuid) -> Self {
        Self {
            player_id,
            active_effects: Vec::new(),
            last_use_time: HashMap::new(),
        }
    }

    /// 使用食物，返回是否成功（可能在冷却中）
    pub fn use_food(&mut self, item_id: u32, effect: &FoodEffect, current_time: u64) -> bool {
        // 检查冷却
        if let Some(&last_use) = self.last_use_time.get(&item_id)
            && current_time - last_use < effect.cooldown_ms
        {
            return false; // 还在冷却中
        }

        // 添加活跃效果
        self.active_effects.push(ActiveFoodEffect {
            item_id,
            start_time: current_time,
            duration_ms: effect.duration_ms,
            hp_per_tick: effect.hp_per_tick,
            sp_per_tick: effect.sp_per_tick,
        });

        // 更新冷却时间
        self.last_use_time.insert(item_id, current_time);

        true
    }

    /// 处理食物效果，返回(HP回复, SP回复)
    pub fn process_effects(&mut self, current_time: u64) -> (u32, u32) {
        let mut total_hp = 0u32;
        let mut total_sp = 0u32;

        // 移除过期的效果
        self.active_effects
            .retain(|effect| current_time - effect.start_time < effect.duration_ms);

        // 计算所有活跃效果的回复量
        for effect in &self.active_effects {
            total_hp += effect.hp_per_tick;
            total_sp += effect.sp_per_tick;
        }

        (total_hp, total_sp)
    }

    /// 获取活跃效果数量
    pub fn active_count(&self) -> usize {
        self.active_effects.len()
    }
}

/// 食物管理器
pub struct FoodManager {
    /// 食物效果表
    effects: RwLock<HashMap<u32, FoodEffect>>,
    /// 玩家食物状态
    player_states: RwLock<HashMap<Uuid, PlayerFoodState>>,
}

impl FoodManager {
    pub fn new() -> Self {
        let manager = Self {
            effects: RwLock::new(HashMap::new()),
            player_states: RwLock::new(HashMap::new()),
        };
        manager.register_default_foods();
        manager
    }

    /// 注册默认食物效果
    fn register_default_foods(&self) {
        // Apple/Juice - 低级回复
        self.register_food(
            512,
            FoodEffect {
                item_id: 512, // Apple
                hunger_restore: 20,
                hp_per_tick: 5,
                sp_per_tick: 3,
                duration_ms: 30000,
                cooldown_ms: 1000,
            },
        );

        self.register_food(
            515,
            FoodEffect {
                item_id: 515, // Carrot
                hunger_restore: 25,
                hp_per_tick: 6,
                sp_per_tick: 4,
                duration_ms: 30000,
                cooldown_ms: 1000,
            },
        );

        // High HP/SP recovery foods
        self.register_food(
            529,
            FoodEffect {
                item_id: 529, // Candy
                hunger_restore: 30,
                hp_per_tick: 20,
                sp_per_tick: 20,
                duration_ms: 60000,
                cooldown_ms: 3000,
            },
        );

        // Banana Juice
        self.register_food(
            534,
            FoodEffect {
                item_id: 534,
                hunger_restore: 28,
                hp_per_tick: 15,
                sp_per_tick: 15,
                duration_ms: 60000,
                cooldown_ms: 3000,
            },
        );

        // 面包
        self.register_food(
            530,
            FoodEffect {
                item_id: 530,
                hunger_restore: 50,
                hp_per_tick: 10,
                sp_per_tick: 5,
                duration_ms: 30000,
                cooldown_ms: 1000,
            },
        );

        // 牛奶
        self.register_food(
            519,
            FoodEffect {
                item_id: 519,
                hunger_restore: 20,
                hp_per_tick: 5,
                sp_per_tick: 10,
                duration_ms: 30000,
                cooldown_ms: 1000,
            },
        );

        tracing::debug!(
            "Registered {} default food effects",
            self.effects.read().len()
        );
    }

    /// 注册食物效果
    pub fn register_food(&self, item_id: u32, effect: FoodEffect) {
        self.effects.write().insert(item_id, effect);
    }

    /// 获取食物效果
    pub fn get_effect(&self, item_id: u32) -> Option<FoodEffect> {
        self.effects.read().get(&item_id).cloned()
    }

    /// 使用食物
    pub fn use_food(&self, player: &Player, item_id: u32) -> Option<FoodEffect> {
        let effect = self.get_effect(item_id)?;

        let mut states = self.player_states.write();

        // 获取或创建玩家状态
        let player_state = states
            .entry(player.id)
            .or_insert_with(|| PlayerFoodState::new(player.id));

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        if player_state.use_food(item_id, &effect, current_time) {
            Some(effect)
        } else {
            None
        }
    }

    /// 处理玩家食物效果
    pub fn process_food_effects(&self, player: &Player) -> (u32, u32) {
        let mut states = self.player_states.write();

        if let Some(state) = states.get_mut(&player.id) {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            state.process_effects(current_time)
        } else {
            (0, 0)
        }
    }

    /// 获取玩家活跃食物效果数量
    pub fn get_active_count(&self, player_id: &Uuid) -> usize {
        self.player_states
            .read()
            .get(player_id)
            .map(|s| s.active_count())
            .unwrap_or(0)
    }

    /// 清除玩家食物状态
    pub fn clear_player_state(&self, player_id: &Uuid) {
        self.player_states.write().remove(player_id);
    }
}

impl Default for FoodManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_food_manager_register() {
        let manager = FoodManager::new();

        // 检查默认食物是否注册
        assert!(manager.get_effect(512).is_some()); // Apple
        assert!(manager.get_effect(529).is_some()); // Candy

        // 检查不存在的食物
        assert!(manager.get_effect(99999).is_none());
    }

    #[test]
    fn test_register_custom_food() {
        let manager = FoodManager::new();

        manager.register_food(
            99999,
            FoodEffect {
                item_id: 99999,
                hunger_restore: 100,
                hp_per_tick: 50,
                sp_per_tick: 50,
                duration_ms: 60000,
                cooldown_ms: 5000,
            },
        );

        assert!(manager.get_effect(99999).is_some());
    }
}
