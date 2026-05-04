//! 生命体管理器
//!
//! 负责生命体的创建、召唤、喂食、经验、进化等操作。

use super::data::{Homunculus, HomunculusDatabase, HomunculusTemplate, HomunculusType};
use parking_lot::RwLock;
use std::collections::HashMap;
use thiserror::Error;

/// 生命体错误类型
#[derive(Debug, Error)]
pub enum HomunculusError {
    #[error("生命体未找到: {0}")]
    NotFound(u32),

    #[error("玩家未找到: {0}")]
    PlayerNotFound(u32),

    #[error("玩家已有召唤的生命体")]
    AlreadySummoned,

    #[error("生命体已死亡")]
    Dead,

    #[error("生命体未召唤")]
    NotSummoned,

    #[error("不是你的生命体")]
    NotYours,

    #[error("食物类型不匹配")]
    WrongFood,

    #[error("进化条件不满足")]
    EvolutionFailed,

    #[error("技能前置条件不满足")]
    SkillPrereqNotMet,

    #[error("数据库错误: {0}")]
    Database(String),
}

/// 生命体管理器
pub struct HomunculusManager {
    /// 所有生命体实例 (homun_id -> Homunculus)
    homunculi: RwLock<HashMap<u32, Homunculus>>,
    /// 当前召唤的生命体 (char_id -> homun_id)
    summoned: RwLock<HashMap<u32, u32>>,
    /// 生命体模板数据库
    database: HomunculusDatabase,
    /// 下一个可用 ID
    next_id: RwLock<u32>,
}

impl HomunculusManager {
    pub fn new() -> Self {
        Self {
            homunculi: RwLock::new(HashMap::new()),
            summoned: RwLock::new(HashMap::new()),
            database: HomunculusDatabase::new(),
            next_id: RwLock::new(1),
        }
    }

    /// 创建新生命体
    pub fn create(
        &self,
        owner_id: u32,
        htype: HomunculusType,
        name: &str,
    ) -> Result<Homunculus, HomunculusError> {
        let template = self
            .database
            .get_by_type(htype)
            .ok_or(HomunculusError::NotFound(0))?;

        let mut next_id = self.next_id.write();
        let homun_id = *next_id;
        *next_id += 1;

        let homun = Homunculus::from_template(homun_id, owner_id, template, name.to_string());

        self.homunculi.write().insert(homun_id, homun.clone());
        Ok(homun)
    }

    /// 召唤生命体（修复原有 BUG: 原来 insert(char_id, char_id)）
    pub fn summon(&self, char_id: u32, homun_id: u32) -> Result<(), HomunculusError> {
        // 检查是否已有召唤
        if self.summoned.read().contains_key(&char_id) {
            return Err(HomunculusError::AlreadySummoned);
        }

        let homunculi = self.homunculi.read();
        let homun = homunculi
            .get(&homun_id)
            .ok_or(HomunculusError::NotFound(homun_id))?;

        // 验证归属
        if homun.owner_id != char_id {
            return Err(HomunculusError::NotYours);
        }

        // 检查是否存活
        if homun.is_dead() {
            return Err(HomunculusError::Dead);
        }

        drop(homunculi);

        // BUG FIX: 原来是 insert(char_id, char_id)，现在正确插入 homun_id
        self.summoned.write().insert(char_id, homun_id);
        Ok(())
    }

    /// 解散生命体
    pub fn dismiss(&self, char_id: u32) {
        self.summoned.write().remove(&char_id);
    }

    /// 获取玩家召唤的生命体
    pub fn get_summoned(&self, char_id: u32) -> Option<Homunculus> {
        let summoned = self.summoned.read();
        let homun_id = summoned.get(&char_id)?;
        self.homunculi.read().get(homun_id).cloned()
    }

    /// 喂食生命体
    pub fn feed(&self, char_id: u32, _item_id: u16) -> Result<(), HomunculusError> {
        let summoned = self.summoned.read();
        let homun_id = summoned
            .get(&char_id)
            .ok_or(HomunculusError::NotSummoned)?;

        let mut homunculi = self.homunculi.write();
        let homun = homunculi
            .get_mut(homun_id)
            .ok_or(HomunculusError::NotFound(*homun_id))?;

        // TODO: 检查食物类型是否匹配
        homun.feed(20);
        homun.increase_intimacy(10);
        Ok(())
    }

    /// 增加经验（返回是否升级）
    pub fn add_exp(&self, char_id: u32, exp: u64) -> Result<bool, HomunculusError> {
        let summoned = self.summoned.read();
        let homun_id = summoned
            .get(&char_id)
            .ok_or(HomunculusError::NotSummoned)?;

        let mut homunculi = self.homunculi.write();
        let homun = homunculi
            .get_mut(homun_id)
            .ok_or(HomunculusError::NotFound(*homun_id))?;

        homun.exp += exp;

        // 检查升级
        let exp_needed = Self::exp_for_level(homun.level + 1);
        if homun.exp >= exp_needed {
            homun.level += 1;
            homun.exp -= exp_needed;

            // 属性成长（简化版）
            homun.max_hp += 20;
            homun.hp = homun.max_hp;
            homun.max_sp += 5;
            homun.sp = homun.max_sp;
            homun.str += 1;
            homun.agi += 1;
            homun.vit += 1;
            homun.int += 1;
            homun.dex += 1;
            homun.luk += 1;

            // 每 5 级获得技能点
            if homun.level % 5 == 0 {
                homun.skill_points += 1;
            }

            return Ok(true); // 升级了
        }

        Ok(false) // 未升级
    }

    /// 经验表查询
    fn exp_for_level(level: u16) -> u64 {
        match level {
            1..=10 => (level as u64) * 100,
            11..=50 => (level as u64) * 500,
            51..=99 => (level as u64) * 2000,
            _ => 999999,
        }
    }

    /// 进化生命体
    pub fn evolve(&self, char_id: u32) -> Result<(), HomunculusError> {
        let summoned = self.summoned.read();
        let homun_id = summoned
            .get(&char_id)
            .ok_or(HomunculusError::NotSummoned)?;

        let mut homunculi = self.homunculi.write();
        let homun = homunculi
            .get_mut(homun_id)
            .ok_or(HomunculusError::NotFound(*homun_id))?;

        // 进化条件: 等级 >= 99, 亲密度 >= 910
        if homun.level < 99 || homun.intimacy < 910 {
            return Err(HomunculusError::EvolutionFailed);
        }

        if homun.evolved {
            return Err(HomunculusError::EvolutionFailed);
        }

        homun.evolved = true;
        homun.evolution_stage = super::data::EvolutionStage::Evolved;

        // 进化属性加成
        homun.max_hp += 500;
        homun.hp = homun.max_hp;
        homun.max_sp += 100;
        homun.sp = homun.max_sp;
        homun.str += 10;
        homun.agi += 10;
        homun.vit += 10;
        homun.int += 10;
        homun.dex += 10;
        homun.luk += 10;

        Ok(())
    }

    /// 获取模板数据库引用
    pub fn database(&self) -> &HomunculusDatabase {
        &self.database
    }
}

impl Default for HomunculusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_homunculus() {
        let manager = HomunculusManager::new();
        let homun = manager.create(100, HomunculusType::Lif, "MyLif").unwrap();

        assert_eq!(homun.owner_id, 100);
        assert_eq!(homun.name, "MyLif");
        assert_eq!(homun.level, 1);
        assert!(homun.alive);
        assert_eq!(homun.homunculus_type, HomunculusType::Lif);
    }

    #[test]
    fn test_summon_dismiss() {
        let manager = HomunculusManager::new();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();

        // 召唤
        assert!(manager.summon(100, homun.homun_id).is_ok());
        assert!(manager.get_summoned(100).is_some());

        // 解散
        manager.dismiss(100);
        assert!(manager.get_summoned(100).is_none());
    }

    #[test]
    fn test_cannot_summon_twice() {
        let manager = HomunculusManager::new();
        let homun1 = manager.create(100, HomunculusType::Lif, "Lif1").unwrap();
        let homun2 = manager
            .create(100, HomunculusType::Amistr, "Ami1")
            .unwrap();

        manager.summon(100, homun1.homun_id).unwrap();
        assert!(matches!(
            manager.summon(100, homun2.homun_id),
            Err(HomunculusError::AlreadySummoned)
        ));
    }

    #[test]
    fn test_summon_wrong_owner() {
        let manager = HomunculusManager::new();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();

        assert!(matches!(
            manager.summon(999, homun.homun_id),
            Err(HomunculusError::NotYours)
        ));
    }

    #[test]
    fn test_feed() {
        let manager = HomunculusManager::new();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        // 降低饥饿度
        {
            let mut homunculi = manager.homunculi.write();
            let h = homunculi.get_mut(&homun.homun_id).unwrap();
            h.hunger = 50;
        }

        manager.feed(100, 0).unwrap();

        let h = manager.get_summoned(100).unwrap();
        assert_eq!(h.hunger, 70); // 50 + 20
    }

    #[test]
    fn test_add_exp_and_level_up() {
        let manager = HomunculusManager::new();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        // 添加足够升级的经验（level 1->2 需要 200）
        let leveled = manager.add_exp(100, 200).unwrap();
        assert!(leveled);

        let h = manager.get_summoned(100).unwrap();
        assert_eq!(h.level, 2);
    }

    #[test]
    fn test_evolve() {
        let manager = HomunculusManager::new();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        // 设置进化条件
        {
            let mut homunculi = manager.homunculi.write();
            let h = homunculi.get_mut(&homun.homun_id).unwrap();
            h.level = 99;
            h.intimacy = 910;
        }

        assert!(manager.evolve(100).is_ok());

        let h = manager.get_summoned(100).unwrap();
        assert!(h.evolved);
        assert_eq!(h.evolution_stage, super::super::data::EvolutionStage::Evolved);
    }

    #[test]
    fn test_evolve_insufficient_level() {
        let manager = HomunculusManager::new();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        assert!(matches!(
            manager.evolve(100),
            Err(HomunculusError::EvolutionFailed)
        ));
    }
}
