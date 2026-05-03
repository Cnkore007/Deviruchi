//! 技能系统

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use uuid::Uuid;

pub mod data;
pub mod effect;
pub mod handler;

pub use data::{Skill, SkillType, SkillTarget, SkillDatabase};
pub use handler::SkillHandler;

/// 每个玩家的技能冷却状态
pub struct PlayerCooldown {
    player_id: Uuid,
    cooldowns: Arc<RwLock<HashMap<u16, Instant>>>, // skill_id -> ready_time
}

impl Clone for PlayerCooldown {
    fn clone(&self) -> Self {
        Self {
            player_id: self.player_id,
            cooldowns: self.cooldowns.clone(),
        }
    }
}

impl PlayerCooldown {
    pub fn new(player_id: Uuid) -> Self {
        Self {
            player_id,
            cooldowns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 检查技能是否冷却完成
    pub fn is_ready(&self, skill_id: u16) -> bool {
        let cooldowns = self.cooldowns.read();
        match cooldowns.get(&skill_id) {
            Some(ready_time) => Instant::now() >= *ready_time,
            None => true,
        }
    }

    /// 设置技能冷却
    pub fn set_cooldown(&self, skill_id: u16, duration_ms: u32) {
        let ready_time = Instant::now() + Duration::from_millis(duration_ms as u64);
        self.cooldowns.write().insert(skill_id, ready_time);
    }

    /// 获取剩余冷却时间（毫秒）
    pub fn remaining_ms(&self, skill_id: u16) -> u64 {
        let cooldowns = self.cooldowns.read();
        match cooldowns.get(&skill_id) {
            Some(ready_time) => {
                let remaining = ready_time.saturating_duration_since(Instant::now());
                remaining.as_millis() as u64
            }
            None => 0,
        }
    }

    /// 清除所有冷却
    pub fn clear_all(&self) {
        self.cooldowns.write().clear();
    }
}
