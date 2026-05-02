use std::sync::Arc;
use crate::game::map::{Player, MapState};
use super::data::SkillDatabase;
use super::effect::{SkillEffect, SkillResult};

pub struct SkillHandler {
    db: Arc<SkillDatabase>,
}

impl SkillHandler {
    pub fn new() -> Self {
        Self {
            db: Arc::new(SkillDatabase::new()),
        }
    }

    /// 检查是否能使用技能
    pub fn can_use_skill(&self, player: &Player, skill_id: u16, level: u8) -> SkillError {
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
        if *player.hp.read() <= skill.hp_cost {
            return SkillError::NotEnoughHP;
        }

        // 检查施法距离 (后续与地图集成)
        // 检查冷却时间

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
        let skill = self.db.get(skill_id)
            .ok_or(SkillError::SkillNotFound)?;

        // 消耗SP/HP
        let sp_cost = skill.sp_cost as u32 * level as u32;
        *caster.sp.write() -= sp_cost;

        if skill.hp_cost > 0 {
            *caster.hp.write() -= skill.hp_cost;
        }

        // 获取目标玩家 (通过 char_id)
        let target = self.find_target_by_char_id(caster.clone(), target_id, map_state)?;

        Ok(SkillEffect::apply(skill, &caster, &target, level))
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
}

impl Default for SkillHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 技能错误
#[derive(Debug, Clone, Copy)]
pub enum SkillError {
    None,
    SkillNotFound,
    NotEnoughSP,
    NotEnoughHP,
    OutOfRange,
    Cooldown,
    InvalidTarget,
}
