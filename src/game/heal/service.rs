use crate::core::Config;
use crate::game::map::{MapState, Player, PlayerState};
use crate::game::status::StatusChange;
use std::sync::Arc;

/// HP/SP 回复修正
#[derive(Debug, Clone)]
pub struct HealModifiers {
    /// HP 百分比加成 (100 = 正常, 200 = 2倍)
    pub hp_rate: i32,
    /// SP 百分比加成 (100 = 正常, 200 = 2倍)
    pub sp_rate: i32,
    /// 忽略阈值检测
    pub ignore_threshold: bool,
}

impl Default for HealModifiers {
    fn default() -> Self {
        Self {
            hp_rate: 100, // 100% = 正常
            sp_rate: 100, // 100% = 正常
            ignore_threshold: false,
        }
    }
}

#[derive(Clone)]
pub struct HealService {
    config: Arc<Config>,
}

impl HealService {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    pub fn start(&self, map_state: Arc<MapState>) {
        let interval_ms = self.config.battle.natural_heal_interval_ms;
        let service = self.clone();
        let ms = map_state.clone();

        crate::core::timer::Timer::add_interval(
            std::time::Duration::from_millis(interval_ms),
            move || {
                service.process_heal(&ms);
            },
        );

        tracing::debug!("HealService started with interval {}ms", interval_ms);
    }

    fn process_heal(&self, map_state: &MapState) {
        let threshold_hp = self.config.battle.natural_heal_threshold_hp;
        let threshold_sp = self.config.battle.natural_heal_threshold_sp;

        // 获取所有唯一地图名称
        let unique_maps = map_state.get_all_map_names();

        for map_name in &unique_maps {
            let players = map_state.get_players_on_map(map_name);

            for player in players {
                self.heal_player(&player, threshold_hp, threshold_sp);
            }
        }
    }

    fn heal_player(&self, player: &Player, threshold_hp: u32, threshold_sp: u32) {
        if *player.state.read() == PlayerState::Dead {
            return;
        }

        let is_sitting = player.is_sitting();

        // 一次性读取所有 HP/SP 相关值，避免竞态
        let (current_hp, max_hp, current_sp, max_sp) = {
            let hp = player.hp.read();
            let max_hp_val = *player.max_hp.read();
            let sp = player.sp.read();
            let max_sp_val = *player.max_sp.read();
            (*hp, max_hp_val, *sp, max_sp_val)
        };

        let mut changed = false;
        let mut new_hp = current_hp;
        let mut new_sp = current_sp;

        // HP 回复
        if current_hp < max_hp {
            let hp_threshold = (max_hp * threshold_hp) / 100;
            // 检查是否满足阈值条件
            let modifiers = self.get_heal_modifiers(player);
            if current_hp >= hp_threshold || modifiers.ignore_threshold {
                let base_heal = self.calculate_hp_heal(player, is_sitting);
                let (hp_base, _) = self.apply_all_modifiers(player, base_heal, 0, is_sitting);
                new_hp = (current_hp + hp_base).min(max_hp);
                changed = true;
                tracing::trace!(
                    "Player {} healed {} HP (sitting: {})",
                    player.name,
                    hp_base,
                    is_sitting
                );
            }
        }

        // SP 回复
        if current_sp < max_sp {
            let sp_threshold = (max_sp * threshold_sp) / 100;
            let modifiers = self.get_heal_modifiers(player);
            if current_sp >= sp_threshold || modifiers.ignore_threshold {
                let base_heal = self.calculate_sp_heal(player, is_sitting);
                let (_, sp_heal) = self.apply_all_modifiers(player, 0, base_heal, is_sitting);
                new_sp = (current_sp + sp_heal).min(max_sp);
                changed = true;
                tracing::trace!(
                    "Player {} healed {} SP (sitting: {})",
                    player.name,
                    sp_heal,
                    is_sitting
                );
            }
        }

        // 一次性写入
        if changed {
            *player.hp.write() = new_hp;
            *player.sp.write() = new_sp;
        }
    }

    pub fn calculate_hp_heal(&self, player: &Player, is_sitting: bool) -> u32 {
        let vit = *player.vit.read() as u32;
        let max_hp = *player.max_hp.read();

        // 基础回复: 1 + VIT/2 + max_hp/200
        let base_heal = 1u32.saturating_add(vit / 2).saturating_add(max_hp / 200);

        // 百分比回复
        let rate = if is_sitting {
            self.config.battle.sit_heal_hp_rate
        } else {
            self.config.battle.natural_heal_hp_rate
        };
        let rate_heal = (max_hp * rate) / 100;

        base_heal.saturating_add(rate_heal)
    }

    pub fn calculate_sp_heal(&self, player: &Player, is_sitting: bool) -> u32 {
        let int = *player.int.read() as u32;
        let max_sp = *player.max_sp.read();

        // 基础回复: 1 + INT/2 + max_sp/100
        let base_heal = 1u32.saturating_add(int / 2).saturating_add(max_sp / 100);

        // 百分比回复
        let rate = if is_sitting {
            self.config.battle.sit_heal_sp_rate
        } else {
            self.config.battle.natural_heal_sp_rate
        };
        let rate_heal = (max_sp * rate) / 100;

        base_heal.saturating_add(rate_heal)
    }

    /// 获取玩家的回复修正（基于状态效果）
    pub fn get_heal_modifiers(&self, player: &Player) -> HealModifiers {
        let mut modifiers = HealModifiers::default();

        // 如果配置禁用了状态效果修饰，直接返回
        if !self.config.battle.status_heal_modifier {
            return modifiers;
        }

        // 饱食度 (Satisfy) - 增加HP/SP回复
        // 注意: 这里我们使用 Regen 和 SpRegen 状态表示有增益回复效果
        if player.has_status(StatusChange::Regen) || player.has_status(StatusChange::Soul) {
            modifiers.hp_rate += 50; // +50%
            modifiers.sp_rate += 50; // +50%
        }

        // 饥饿 (Hunger) - 减少HP/SP回复
        if player.has_status(StatusChange::Hunger) {
            modifiers.hp_rate -= 50; // -50%
            modifiers.sp_rate -= 50; // -50%
        }

        // 祝福 (Blessing) - 增加回复
        if player.has_status(StatusChange::Blessing) {
            modifiers.hp_rate += 25; // +25%
            modifiers.sp_rate += 25; // +25%
        }

        // 中毒/出血 - 停止自然回复
        if player.has_status(StatusChange::Poison) || player.has_status(StatusChange::Bleeding) {
            modifiers.hp_rate = 0;
            modifiers.sp_rate = 0;
        }

        modifiers
    }

    /// 应用战斗惩罚
    pub fn apply_battle_penalty(
        &self,
        player: &Player,
        base_hp_heal: u32,
        base_sp_heal: u32,
    ) -> (u32, u32) {
        // 如果配置禁用了战斗惩罚，直接返回
        if !self.config.battle.battle_heal_penalty {
            return (base_hp_heal, base_sp_heal);
        }

        // 战斗中HP/SP回复减少
        let in_combat = player.is_in_combat();
        let in_battle_status = player.has_status(StatusChange::Battle);

        if in_combat || in_battle_status {
            (base_hp_heal / 2, base_sp_heal / 2) // 战斗中断半回复
        } else {
            (base_hp_heal, base_sp_heal)
        }
    }

    /// 应用超重惩罚
    pub fn apply_overweight_penalty(
        &self,
        player: &Player,
        base_hp_heal: u32,
        base_sp_heal: u32,
    ) -> (u32, u32) {
        // 如果配置禁用了超重惩罚，直接返回
        if !self.config.battle.overweight_heal_penalty {
            return (base_hp_heal, base_sp_heal);
        }

        let mut hp = base_hp_heal;
        let mut sp = base_sp_heal;

        // 超重50%: HP回复减半
        if player.is_overweight() {
            hp /= 2;
        }

        // 超重90%: SP回复减半
        if player.is_overweight_90() {
            sp /= 2;
        }

        (hp, sp)
    }

    /// 应用所有回复修正到基础回复值
    pub fn apply_all_modifiers(
        &self,
        player: &Player,
        base_hp: u32,
        base_sp: u32,
        _is_sitting: bool,
    ) -> (u32, u32) {
        // 1. 获取状态效果修正
        let modifiers = self.get_heal_modifiers(player);

        // 2. 计算百分比加成后的回复值
        let mut hp = (base_hp as i32 * modifiers.hp_rate / 100) as u32;
        let mut sp = (base_sp as i32 * modifiers.sp_rate / 100) as u32;

        // 3. 应用战斗惩罚
        (hp, sp) = self.apply_battle_penalty(player, hp, sp);

        // 4. 应用超重惩罚
        (hp, sp) = self.apply_overweight_penalty(player, hp, sp);

        (hp, sp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Character;
    use uuid::Uuid;

    fn create_test_player() -> Player {
        let char = Character {
            char_id: 1,
            account_id: 1,
            name: "TestPlayer".to_string(),
            base_level: 50,
            job_level: 50,
            str: 50,
            agi: 50,
            vit: 50,
            int: 50,
            dex: 50,
            luk: 50,
            class: 0,
            base_exp: 0,
            job_exp: 0,
            hp: 1000,
            max_hp: 1000,
            sp: 500,
            max_sp: 500,
            last_map: "prontera".to_string(),
            last_x: 100,
            last_y: 100,
            save_map: "prontera".to_string(),
            save_x: 100,
            save_y: 100,
            zeny: 10000,
        };
        Player::from_character(char)
    }

    fn create_test_config() -> Config {
        Config::default()
    }

    fn create_test_heal_service() -> (HealService, Player) {
        let config = Arc::new(create_test_config());
        let service = HealService::new(config);
        let player = create_test_player();
        (service, player)
    }

    #[test]
    fn test_sitting_heals_more_than_standing() {
        let (service, player) = create_test_heal_service();

        let standing_heal = service.calculate_hp_heal(&player, false);
        let sitting_heal = service.calculate_hp_heal(&player, true);

        // 坐下回复应该大于站立回复
        assert!(
            sitting_heal > standing_heal,
            "Sitting heal {} should be greater than standing heal {}",
            sitting_heal,
            standing_heal
        );
    }

    #[test]
    fn test_overweight_reduces_heal() {
        let (service, player) = create_test_heal_service();

        // 设置玩家为超重状态 (超过50%)
        *player.current_weight.write() = player.max_weight.read() * 60 / 100;
        assert!(player.is_overweight());

        let base_hp = service.calculate_hp_heal(&player, false);
        let base_sp = service.calculate_sp_heal(&player, false);

        let (hp, _) = service.apply_overweight_penalty(&player, base_hp, base_sp);

        // 超重时 HP 回复应该减半
        assert!(
            hp < base_hp,
            "Overweight HP {} should be less than base HP {}",
            hp,
            base_hp
        );
    }

    #[test]
    fn test_overweight_90_reduces_sp() {
        let (service, player) = create_test_heal_service();

        // 设置玩家为严重超重状态 (超过90%)
        *player.current_weight.write() = player.max_weight.read() * 95 / 100;
        assert!(player.is_overweight_90());

        let base_hp = service.calculate_hp_heal(&player, false);
        let base_sp = service.calculate_sp_heal(&player, false);

        let (_, sp) = service.apply_overweight_penalty(&player, base_hp, base_sp);

        // 严重超重时 SP 回复应该减半
        assert!(
            sp < base_sp,
            "Overweight 90% SP {} should be less than base SP {}",
            sp,
            base_sp
        );
    }

    #[test]
    fn test_battle_reduces_heal() {
        let (service, player) = create_test_heal_service();

        // 设置玩家为战斗状态
        player.set_combat(true);

        let base_hp = 100;
        let base_sp = 50;

        let (hp, sp) = service.apply_battle_penalty(&player, base_hp, base_sp);

        // 战斗中回复应该减半
        assert_eq!(hp, base_hp / 2);
        assert_eq!(sp, base_sp / 2);
    }

    #[test]
    fn test_blessing_increases_heal_rate() {
        let (service, player) = create_test_heal_service();

        // 添加 Blessing 状态
        player.apply_blessing(1);

        let modifiers = service.get_heal_modifiers(&player);

        // 祝福应该增加回复率
        assert!(
            modifiers.hp_rate > 100,
            "Blessing HP rate {} should be greater than 100",
            modifiers.hp_rate
        );
    }

    #[test]
    fn test_hunger_decreases_heal_rate() {
        let (service, player) = create_test_heal_service();

        // 添加 Hunger 状态
        player.add_status(crate::game::status::StatusEffect::new(
            StatusChange::Hunger,
            10000,
            crate::game::status::StatusSource::Auto,
        ));

        let modifiers = service.get_heal_modifiers(&player);

        // 饥饿应该减少回复率
        assert!(
            modifiers.hp_rate < 100,
            "Hunger HP rate {} should be less than 100",
            modifiers.hp_rate
        );
    }

    #[test]
    fn test_poison_stops_heal() {
        let (service, player) = create_test_heal_service();

        // 添加 Poison 状态
        player.add_status(crate::game::status::StatusEffect::new(
            StatusChange::Poison,
            10000,
            crate::game::status::StatusSource::Auto,
        ));

        let modifiers = service.get_heal_modifiers(&player);

        // 中毒应该完全停止回复
        assert_eq!(
            modifiers.hp_rate, 0,
            "Poison HP rate should be 0, got {}",
            modifiers.hp_rate
        );
        assert_eq!(
            modifiers.sp_rate, 0,
            "Poison SP rate should be 0, got {}",
            modifiers.sp_rate
        );
    }

    #[test]
    fn test_regen_status_boosts_heal() {
        let (service, player) = create_test_heal_service();

        // 添加 Regen 状态
        player.add_status(crate::game::status::StatusEffect::new(
            StatusChange::Regen,
            10000,
            crate::game::status::StatusSource::Auto,
        ));

        let modifiers = service.get_heal_modifiers(&player);

        // Regen 应该增加回复率
        assert!(
            modifiers.hp_rate > 100,
            "Regen HP rate {} should be greater than 100",
            modifiers.hp_rate
        );
    }

    #[test]
    fn test_apply_all_modifiers() {
        let (service, player) = create_test_heal_service();

        // 添加祝福状态
        player.apply_blessing(1);

        let base_hp = 100;
        let base_sp = 50;

        let (hp, sp) = service.apply_all_modifiers(&player, base_hp, base_sp, false);

        // 应用祝福加成后回复应该增加
        assert!(
            hp > base_hp,
            "With blessing, HP {} should be greater than base {}",
            hp,
            base_hp
        );
        assert!(
            sp > base_sp,
            "With blessing, SP {} should be greater than base {}",
            sp,
            base_sp
        );
    }

    #[test]
    fn test_heal_modifiers_default() {
        let modifiers = HealModifiers::default();
        assert_eq!(modifiers.hp_rate, 100);
        assert_eq!(modifiers.sp_rate, 100);
        assert!(!modifiers.ignore_threshold);
    }
}
