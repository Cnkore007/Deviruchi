//! 状态效果周期处理
//!
//! 处理状态效果的定期更新，如持续伤害、回复等

use super::calculator::StatusCalculator;
use super::player_status::PlayerStatus;
use super::types::StatusChange;
use crate::game::map::{MapState, Player};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 状态Tick配置
#[derive(Debug, Clone)]
pub struct StatusTickConfig {
    /// 周期处理间隔（毫秒）
    pub tick_interval_ms: u64,
    /// DOT 伤害Tick间隔（毫秒）
    pub dot_tick_ms: u64,
    /// 隐身Tick间隔（毫秒）
    pub stealth_tick_ms: u64,
}

impl Default for StatusTickConfig {
    fn default() -> Self {
        Self {
            tick_interval_ms: 1000, // 1秒
            dot_tick_ms: 1000,      // 1秒
            stealth_tick_ms: 5000,  // 5秒
        }
    }
}

/// 状态Tick处理器
pub struct StatusTickProcessor {
    config: StatusTickConfig,
    /// DOT效果上次触发时间
    dot_last_tick: Instant,
    /// 隐身效果上次触发时间
    stealth_last_tick: Instant,
}

impl StatusTickProcessor {
    /// 创建新的Tick处理器
    pub fn new(config: StatusTickConfig) -> Self {
        Self {
            config,
            dot_last_tick: Instant::now(),
            stealth_last_tick: Instant::now(),
        }
    }

    /// 使用默认配置创建
    pub fn default_config() -> Self {
        Self::new(StatusTickConfig::default())
    }

    /// 处理玩家状态Tick
    pub fn tick_player(&mut self, player: &Player, status: &PlayerStatus) -> StatusTickResult {
        let mut result = StatusTickResult::default();

        // 1. 清理过期状态
        let expired = status.cleanup_expired();
        result.expired_count = expired.len();
        for status_type in &expired {
            result.expired.push(*status_type);
        }

        // 2. 处理DOT伤害（中毒、流血）
        if self.should_process_dot() {
            self.process_dot_effects(player, status, &mut result);
        }

        // 3. 处理回复效果
        self.process_regen_effects(player, status, &mut result);

        // 4. 处理隐身状态
        if self.should_process_stealth() {
            self.process_stealth_effects(player, status, &mut result);
        }

        result
    }

    /// 处理所有在线玩家
    pub fn tick_all_players(
        &mut self,
        players: &[Arc<Player>],
        status_manager: &impl Fn(&Arc<Player>) -> Arc<PlayerStatus>,
    ) -> Vec<(Arc<Player>, StatusTickResult)> {
        let mut results = Vec::new();

        for player in players {
            if player.is_alive() {
                let status = status_manager(player);
                let result = self.tick_player(player, &status);
                results.push((player.clone(), result));
            }
        }

        results
    }

    /// 是否应该处理DOT效果
    fn should_process_dot(&mut self) -> bool {
        if self.dot_last_tick.elapsed().as_millis() as u64 >= self.config.dot_tick_ms {
            self.dot_last_tick = Instant::now();
            true
        } else {
            false
        }
    }

    /// 是否应该处理隐身效果
    fn should_process_stealth(&mut self) -> bool {
        if self.stealth_last_tick.elapsed().as_millis() as u64 >= self.config.stealth_tick_ms {
            self.stealth_last_tick = Instant::now();
            true
        } else {
            false
        }
    }

    /// 处理持续伤害效果（DOT: Damage Over Time）
    fn process_dot_effects(
        &self,
        player: &Player,
        status: &PlayerStatus,
        result: &mut StatusTickResult,
    ) {
        // 中毒
        if let Some(effect) = status.get_status(StatusChange::Poison) {
            let damage = effect.val1 as u32; // val1: 每次Tick伤害
            result.total_dot_damage += damage;
            let new_hp = (player.hp()).saturating_sub(damage);
            player.combat_mut().hp = new_hp;

            tracing::trace!(
                "Player {} took {} poison damage, HP: {}",
                player.name,
                damage,
                new_hp
            );

            if new_hp == 0 {
                player.die();
                result.player_died = true;
            }
        }

        // 流血
        if let Some(effect) = status.get_status(StatusChange::Bleeding) {
            let damage = effect.val1 as u32; // val1: 每次Tick伤害
            result.total_dot_damage += damage;
            let new_hp = (player.hp()).saturating_sub(damage);
            player.combat_mut().hp = new_hp;

            tracing::trace!(
                "Player {} took {} bleeding damage, HP: {}",
                player.name,
                damage,
                new_hp
            );

            if new_hp == 0 {
                player.die();
                result.player_died = true;
            }
        }
    }

    /// 处理回复效果
    fn process_regen_effects(
        &self,
        player: &Player,
        status: &PlayerStatus,
        result: &mut StatusTickResult,
    ) {
        let modifiers = StatusCalculator::calculate_from_status(status);

        // HP 回复
        let max_hp = player.max_hp();
        let current_hp = player.hp();

        if current_hp < max_hp {
            let base_hp_regen = self.calculate_base_hp_regen(player);
            let hp_regen = StatusCalculator::calculate_hp_regen(base_hp_regen, &modifiers);

            if hp_regen > 0 {
                let new_hp = (current_hp + hp_regen as u32).min(max_hp);
                player.combat_mut().hp = new_hp;
                result.hp_healed = hp_regen as u32;
            }
        }

        // SP 回复
        let max_sp = player.max_sp();
        let current_sp = player.sp();

        if current_sp < max_sp {
            let base_sp_regen = self.calculate_base_sp_regen(player);
            let sp_regen = StatusCalculator::calculate_sp_regen(base_sp_regen, &modifiers);

            if sp_regen > 0 {
                let new_sp = (current_sp + sp_regen as u32).min(max_sp);
                player.combat_mut().sp = new_sp;
                result.sp_healed = sp_regen as u32;
            }
        }
    }

    /// 计算基础HP回复量
    fn calculate_base_hp_regen(&self, player: &Player) -> i32 {
        let vit = player.vit() as i32;
        let max_hp = player.max_hp() as i32;

        // 基础回复: 1 + VIT/6 + max_hp/1000
        (1 + vit / 6 + max_hp / 1000).max(1)
    }

    /// 计算基础SP回复量
    fn calculate_base_sp_regen(&self, player: &Player) -> i32 {
        let int = player.int() as i32;
        let max_sp = player.max_sp() as i32;

        // 基础回复: 1 + INT/6 + max_sp/1000
        (1 + int / 6 + max_sp / 1000).max(1)
    }

    /// 处理隐身效果
    fn process_stealth_effects(
        &self,
        player: &Player,
        status: &PlayerStatus,
        result: &mut StatusTickResult,
    ) {
        let stealth_statuses = [
            StatusChange::Hide,
            StatusChange::Cloak,
            StatusChange::ChaseWalk,
        ];

        for stealth_type in stealth_statuses {
            if let Some(effect) = status.get_status(stealth_type) {
                if effect.is_expired() {
                    // 隐身效果已过期，移除
                    status.remove_status(stealth_type);
                    result.expired_count += 1;
                    result.expired.push(stealth_type);

                    tracing::trace!(
                        "Player {} stealth status {:?} expired, removed",
                        player.name,
                        stealth_type
                    );
                } else {
                    // 隐身效果仍然有效，玩家保持隐藏
                    tracing::trace!(
                        "Player {} remains in stealth ({:?}), {}ms remaining",
                        player.name,
                        stealth_type,
                        effect.remaining_ms()
                    );
                }
            }
        }
    }

    /// 重置DOT计时器
    pub fn reset_dot_timer(&mut self) {
        self.dot_last_tick = Instant::now();
    }

    /// 重置隐身计时器
    pub fn reset_stealth_timer(&mut self) {
        self.stealth_last_tick = Instant::now();
    }
}

/// 状态Tick处理结果
#[derive(Debug, Clone, Default)]
pub struct StatusTickResult {
    /// 过期状态数量
    pub expired_count: usize,
    /// 过期状态列表
    pub expired: Vec<StatusChange>,
    /// DOT总伤害
    pub total_dot_damage: u32,
    /// HP回复量
    pub hp_healed: u32,
    /// SP回复量
    pub sp_healed: u32,
    /// 玩家是否死亡
    pub player_died: bool,
}

impl StatusTickResult {
    /// 是否造成任何影响
    pub fn has_effect(&self) -> bool {
        self.total_dot_damage > 0
            || self.hp_healed > 0
            || self.sp_healed > 0
            || self.expired_count > 0
    }
}

/// 状态Tick服务
pub struct StatusTickService {
    processor: StatusTickProcessor,
    config: StatusTickConfig,
}

impl StatusTickService {
    /// 创建新的Tick服务
    pub fn new(config: StatusTickConfig) -> Self {
        Self {
            processor: StatusTickProcessor::new(config.clone()),
            config,
        }
    }

    /// 启动状态Tick服务
    pub fn start(&self, map_state: Arc<MapState>) {
        let mut processor = self.processor.clone();
        let interval = Duration::from_millis(self.config.tick_interval_ms);

        std::thread::spawn(move || {
            loop {
                let player_ids = map_state.get_all_player_ids();

                for player_id in player_ids {
                    let Some(player) = map_state.get_player(&player_id) else {
                        // 玩家已断线，跳过
                        continue;
                    };

                    if !player.is_alive() {
                        continue;
                    }

                    let result = processor.tick_player(&player, &player.status);

                    if result.player_died {
                        tracing::info!(
                            "Player {} died from status effect DOT",
                            player.name
                        );
                    }

                    if result.has_effect() {
                        tracing::trace!(
                            "Player {} tick: {} DOT dmg, {} HP healed, {} SP healed, {} expired",
                            player.name,
                            result.total_dot_damage,
                            result.hp_healed,
                            result.sp_healed,
                            result.expired_count
                        );
                    }
                }

                std::thread::sleep(interval);
            }
        });

        tracing::info!(
            "StatusTickService started with interval {}ms",
            self.config.tick_interval_ms
        );
    }
}

impl Clone for StatusTickProcessor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            dot_last_tick: self.dot_last_tick,
            stealth_last_tick: self.stealth_last_tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_result_has_effect() {
        let mut result = StatusTickResult::default();
        assert!(!result.has_effect());

        result.hp_healed = 10;
        assert!(result.has_effect());

        result = StatusTickResult::default();
        result.total_dot_damage = 5;
        assert!(result.has_effect());
    }

    #[test]
    fn test_status_tick_config_default() {
        let config = StatusTickConfig::default();
        assert_eq!(config.tick_interval_ms, 1000);
        assert_eq!(config.dot_tick_ms, 1000);
    }

    #[test]
    fn test_processor_creation() {
        let processor = StatusTickProcessor::default_config();
        assert_eq!(processor.config.tick_interval_ms, 1000);
    }

    #[test]
    fn test_calculate_base_hp_regen() {
        use std::sync::Arc;

        // 创建一个简单的Player用于测试
        let player = Arc::new(Player::from_character(crate::storage::Character {
            char_id: 1,
            char_num: 0,
            name: "Test".to_string(),
            class: 0,
            base_level: 10,
            job_level: 5,
            str: 10,
            agi: 10,
            vit: 10,
            int: 10,
            dex: 10,
            luk: 10,
            hp: 1000,
            max_hp: 1000,
            sp: 100,
            max_sp: 100,
            base_exp: 0,
            job_exp: 0,
            zeny: 0,
            hair: 1,
            hair_color: 0,
            clothes_color: 0,
            weapon: 0,
            shield: 0,
            head_top: 0,
            head_mid: 0,
            head_bottom: 0,
            last_map: "prontera".to_string(),
            last_x: 100,
            last_y: 100,
            save_map: "prontera".to_string(),
            save_x: 100,
            save_y: 100,
            delete_timer: 0,
            status_point: 0,
            skill_point: 0,
            created_at: 0,
            updated_at: 0,
        }));

        let processor = StatusTickProcessor::default_config();
        let regen = processor.calculate_base_hp_regen(&player);
        // 1 + 10/6 + 1000/1000 = 1 + 1 + 1 = 3
        assert_eq!(regen, 3);
    }

    #[test]
    fn test_calculate_base_sp_regen() {
        use std::sync::Arc;

        let player = Arc::new(Player::from_character(crate::storage::Character {
            char_id: 1,
            char_num: 0,
            name: "Test".to_string(),
            class: 0,
            base_level: 10,
            job_level: 5,
            str: 10,
            agi: 10,
            vit: 10,
            int: 10,
            dex: 10,
            luk: 10,
            hp: 1000,
            max_hp: 1000,
            sp: 100,
            max_sp: 100,
            base_exp: 0,
            job_exp: 0,
            zeny: 0,
            hair: 1,
            hair_color: 0,
            clothes_color: 0,
            weapon: 0,
            shield: 0,
            head_top: 0,
            head_mid: 0,
            head_bottom: 0,
            last_map: "prontera".to_string(),
            last_x: 100,
            last_y: 100,
            save_map: "prontera".to_string(),
            save_x: 100,
            save_y: 100,
            delete_timer: 0,
            status_point: 0,
            skill_point: 0,
            created_at: 0,
            updated_at: 0,
        }));

        let processor = StatusTickProcessor::default_config();
        let regen = processor.calculate_base_sp_regen(&player);
        // 1 + 10/6 + 100/1000 = 1 + 1 + 0 = 2
        assert_eq!(regen, 2);
    }
}
