use super::PlayerCooldown;
use super::data::SkillDatabase;
use super::effect::{SkillEffect, SkillResult};
use crate::game::map::{MapState, Player};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct SkillHandler {
    db: Arc<SkillDatabase>,
    // Note: cooldowns is not cloned - each clone shares the same underlying map
    cooldowns: Arc<RwLock<HashMap<Uuid, PlayerCooldown>>>,
}

impl SkillHandler {
    pub fn new() -> Self {
        Self {
            db: Arc::new(SkillDatabase::new()),
            cooldowns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 检查是否能使用技能（完整检查）
    pub fn can_use_skill(
        &self,
        player: &Player,
        skill_id: u16,
        level: u8,
        target_id: u32,
        map_state: &MapState,
    ) -> SkillError {
        let skill = match self.db.get(skill_id) {
            Some(s) => s,
            None => return SkillError::SkillNotFound,
        };

        // 检查SP
        let sp_cost = skill.sp_cost as u32 * level as u32;
        if player.sp() < sp_cost {
            return SkillError::NotEnoughSP;
        }

        // 检查HP
        if player.hp() < skill.hp_cost {
            return SkillError::NotEnoughHP;
        }

        // 检查冷却时间
        if let Err(e) = self.check_cooldown(player.id, skill.id) {
            return e;
        }

        // 检查施法距离
        if target_id != 0 && !self.is_target_in_range(player, target_id, skill.range, map_state) {
            return SkillError::OutOfRange;
        }

        SkillError::None
    }

    /// 使用技能
    pub fn use_skill(
        &self,
        caster: Arc<Player>,
        skill_id: u16,
        level: u8,
        target_id: u32,
        map_state: &MapState,
    ) -> Result<SkillResult, SkillError> {
        // 完整检查
        let error = self.can_use_skill(&caster, skill_id, level, target_id, map_state);
        if error != SkillError::None {
            return Err(error);
        }

        let skill = self.db.get(skill_id).ok_or(SkillError::SkillNotFound)?;

        // 消耗SP/HP
        let sp_cost = skill.sp_cost as u32 * level as u32;
        {
            let mut c = caster.combat_mut();
            c.sp -= sp_cost;
            if skill.hp_cost > 0 {
                c.hp -= skill.hp_cost;
            }
        }

        // 获取目标玩家
        let target = if target_id != 0 {
            Some(self.find_target_by_char_id(caster.clone(), target_id, map_state)?)
        } else {
            None
        };

        // 执行效果
        let result = if let Some(ref t) = target {
            SkillEffect::apply(skill, &caster, t, level)
        } else {
            SkillEffect::apply(skill, &caster, &caster, level)
        };

        // 应用伤害/治疗
        if let SkillResult::Damage { damage, .. } = &result
            && let Some(ref t) = target
        {
            let died = t.take_damage(*damage as u32);
            tracing::info!("Skill {} dealt {} damage to {}", skill.name, damage, t.name);
            if died {
                tracing::info!("Player {} was killed", t.name);
            }
        }

        // 设置冷却
        if skill.cooldown > 0 {
            self.set_cooldown(caster.id, skill.id, skill.cooldown);
        }

        Ok(result)
    }

    /// 检查目标是否在技能范围内
    pub fn is_in_range(&self, caster: &Player, target: &Player, range: u16) -> bool {
        if caster.map_name != target.map_name {
            return false;
        }
        let (cx, cy) = caster.get_position();
        let (tx, ty) = target.get_position();
        let dx = (cx as i32 - tx as i32).unsigned_abs() as u16;
        let dy = (cy as i32 - ty as i32).unsigned_abs() as u16;
        dx <= range && dy <= range
    }

    /// 检查玩家是否在范围内
    pub fn is_target_in_range(
        &self,
        caster: &Player,
        target_char_id: u32,
        range: u16,
        map_state: &MapState,
    ) -> bool {
        let map_name = caster.map_name.clone();
        let players = map_state.get_players_on_map(&map_name);

        if let Some(target) = players.iter().find(|p| p.char_id == target_char_id) {
            self.is_in_range(caster, target, range)
        } else {
            false
        }
    }

    /// 根据 char_id 查找目标玩家
    fn find_target_by_char_id(
        &self,
        caster: Arc<Player>,
        target_char_id: u32,
        map_state: &MapState,
    ) -> Result<Player, SkillError> {
        let map_name = caster.map_name.clone();
        let players = map_state.get_players_on_map(&map_name);

        players
            .into_iter()
            .find(|p| p.char_id == target_char_id)
            .ok_or(SkillError::InvalidTarget)
    }

    /// 获取技能数据库
    pub fn get_database(&self) -> Arc<SkillDatabase> {
        self.db.clone()
    }

    /// 检查技能冷却
    pub fn check_cooldown(&self, player_id: Uuid, skill_id: u16) -> Result<(), SkillError> {
        let cooldowns = self.cooldowns.read();
        if let Some(cd) = cooldowns.get(&player_id)
            && !cd.is_ready(skill_id)
        {
            return Err(SkillError::Cooldown);
        }
        Ok(())
    }

    /// 设置技能冷却
    pub fn set_cooldown(&self, player_id: Uuid, skill_id: u16, duration_ms: u32) {
        let mut cooldowns = self.cooldowns.write();
        let cd = cooldowns
            .entry(player_id)
            .or_insert_with(|| PlayerCooldown::new(player_id));
        cd.set_cooldown(skill_id, duration_ms);
    }

    /// 获取冷却剩余时间
    pub fn get_cooldown_remaining(&self, player_id: Uuid, skill_id: u16) -> u64 {
        let cooldowns = self.cooldowns.read();
        cooldowns
            .get(&player_id)
            .map(|cd| cd.remaining_ms(skill_id))
            .unwrap_or(0)
    }

    /// 清除玩家所有冷却
    pub fn clear_cooldowns(&self, player_id: Uuid) {
        let mut cooldowns = self.cooldowns.write();
        if let Some(cd) = cooldowns.get_mut(&player_id) {
            cd.clear_all();
        }
    }

    /// 学习技能
    pub fn learn_skill(&self, player: &Player, skill_id: u16) -> Result<(), SkillError> {
        // 检查技能是否存在
        let skill = match self.db.get(skill_id) {
            Some(s) => s,
            None => {
                tracing::warn!("Skill {} not found in database", skill_id);
                return Err(SkillError::SkillNotFound);
            }
        };

        // 技能学习系统尚未实现，不返回假成功
        tracing::warn!(
            "Skill learning not yet implemented: player={}, skill_id={}, skill={}",
            player.name,
            skill_id,
            skill.name
        );

        Err(SkillError::NotImplemented)
    }
}

impl Default for SkillHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 技能错误
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SkillError {
    None,
    SkillNotFound,
    NotEnoughSP,
    NotEnoughHP,
    OutOfRange,
    Cooldown,
    InvalidTarget,
    NoTarget,
    NotImplemented,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_cooldown_new() {
        let player_id = Uuid::new_v4();
        let cd = PlayerCooldown::new(player_id);
        assert_eq!(cd.player_id(), player_id);
    }

    #[test]
    fn test_player_cooldown_ready_initially() {
        let cd = PlayerCooldown::new(Uuid::new_v4());
        assert!(cd.is_ready(1)); // 新技能无冷却
        assert!(cd.is_ready(100));
    }

    #[test]
    fn test_player_cooldown_set() {
        let cd = PlayerCooldown::new(Uuid::new_v4());
        cd.set_cooldown(1, 5000);
        assert!(!cd.is_ready(1)); // 刚设置的技能应该冷却中
        assert!(cd.remaining_ms(1) > 0);
    }

    #[test]
    fn test_player_cooldown_others_ready() {
        let cd = PlayerCooldown::new(Uuid::new_v4());
        cd.set_cooldown(1, 5000);
        assert!(cd.is_ready(2)); // 其他技能不受影响
    }

    #[test]
    fn test_player_cooldown_clear() {
        let cd = PlayerCooldown::new(Uuid::new_v4());
        cd.set_cooldown(1, 5000);
        cd.set_cooldown(2, 5000);
        cd.clear_all();
        assert!(cd.is_ready(1));
        assert!(cd.is_ready(2));
    }

    #[test]
    fn test_skill_handler_new() {
        let handler = SkillHandler::new();
        assert!(handler.get_database().get(1).is_some()); // 默认技能存在
    }

    #[test]
    fn test_skill_handler_default() {
        let handler = SkillHandler::default();
        assert!(handler.get_database().get(1).is_some());
    }

    #[test]
    fn test_skill_cooldown_check() {
        let handler = SkillHandler::new();
        let player_id = Uuid::new_v4();

        // 新技能无冷却
        assert!(handler.check_cooldown(player_id, 1).is_ok());

        // 设置冷却
        handler.set_cooldown(player_id, 1, 5000);
        assert!(handler.check_cooldown(player_id, 1).is_err());

        // 其他技能不受影响
        assert!(handler.check_cooldown(player_id, 2).is_ok());
    }

    #[test]
    fn test_skill_cooldown_remaining() {
        let handler = SkillHandler::new();
        let player_id = Uuid::new_v4();

        assert_eq!(handler.get_cooldown_remaining(player_id, 1), 0);

        handler.set_cooldown(player_id, 1, 5000);
        let remaining = handler.get_cooldown_remaining(player_id, 1);
        assert!(remaining > 0 && remaining <= 5000);
    }

    #[test]
    fn test_skill_clear_cooldowns() {
        let handler = SkillHandler::new();
        let player_id = Uuid::new_v4();

        handler.set_cooldown(player_id, 1, 5000);
        handler.set_cooldown(player_id, 2, 5000);
        handler.clear_cooldowns(player_id);

        assert!(handler.check_cooldown(player_id, 1).is_ok());
        assert!(handler.check_cooldown(player_id, 2).is_ok());
    }
}
