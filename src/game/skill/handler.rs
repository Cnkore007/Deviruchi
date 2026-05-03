use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use parking_lot::RwLock;
use crate::game::map::{Player, MapState};
use super::data::SkillDatabase;
use super::effect::{SkillEffect, SkillResult};
use super::PlayerCooldown;

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
        if *player.sp.read() < sp_cost {
            return SkillError::NotEnoughSP;
        }

        // 检查HP
        if *player.hp.read() < skill.hp_cost {
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

        let skill = self.db.get(skill_id)
            .ok_or(SkillError::SkillNotFound)?;

        // 消耗SP/HP
        let sp_cost = skill.sp_cost as u32 * level as u32;
        *caster.sp.write() -= sp_cost;

        if skill.hp_cost > 0 {
            *caster.hp.write() -= skill.hp_cost;
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
        if let SkillResult::Damage { damage, .. } = &result {
            if let Some(ref t) = target {
                let died = t.take_damage(*damage as u32);
                tracing::info!("Skill {} dealt {} damage to {}", skill.name, damage, t.name);
                if died {
                    tracing::info!("Player {} was killed", t.name);
                }
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
    pub fn is_target_in_range(&self, caster: &Player, target_char_id: u32, range: u16, map_state: &MapState) -> bool {
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
        if let Some(cd) = cooldowns.get(&player_id) {
            if !cd.is_ready(skill_id) {
                return Err(SkillError::Cooldown);
            }
        }
        Ok(())
    }

    /// 设置技能冷却
    pub fn set_cooldown(&self, player_id: Uuid, skill_id: u16, duration_ms: u32) {
        let mut cooldowns = self.cooldowns.write();
        let cd = cooldowns.entry(player_id).or_insert_with(|| PlayerCooldown::new(player_id));
        cd.set_cooldown(skill_id, duration_ms);
    }

    /// 获取冷却剩余时间
    pub fn get_cooldown_remaining(&self, player_id: Uuid, skill_id: u16) -> u64 {
        let cooldowns = self.cooldowns.read();
        cooldowns.get(&player_id)
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
}
