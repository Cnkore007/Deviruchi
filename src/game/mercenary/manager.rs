//! 雇佣兵管理器
//!
//! 负责雇佣兵的创建、召唤、合同管理、忠诚度等操作。

use super::data::{Mercenary, MercenaryDatabase, MercenaryData, MercenarySkill};
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use thiserror::Error;

/// 雇佣兵错误类型
#[derive(Debug, Error)]
pub enum MercenaryError {
    #[error("雇佣兵未找到: {0}")]
    NotFound(u32),

    #[error("玩家已有召唤的雇佣兵")]
    AlreadySummoned,

    #[error("雇佣兵未召唤")]
    NotSummoned,

    #[error("不是你的雇佣兵")]
    NotYours,

    #[error("合同已到期")]
    ContractExpired,

    #[error("雇佣兵已死亡")]
    Dead,

    #[error("数据库错误: {0}")]
    Database(String),
}

/// 雇佣兵管理器
pub struct MercenaryManager {
    /// 所有雇佣兵实例
    mercenaries: RwLock<HashMap<u32, Mercenary>>,
    /// 当前召唤的雇佣兵 (char_id -> mercenary_id)
    summoned: RwLock<HashMap<u32, u32>>,
    /// 雇佣兵模板数据库
    database: MercenaryDatabase,
    /// 下一个可用 ID
    next_id: RwLock<u32>,
}

impl MercenaryManager {
    pub fn new() -> Self {
        Self {
            mercenaries: RwLock::new(HashMap::new()),
            summoned: RwLock::new(HashMap::new()),
            database: MercenaryDatabase::new(),
            next_id: RwLock::new(1),
        }
    }

    /// 创建雇佣兵（使用模板数据）
    pub fn create(
        &self,
        owner_id: u32,
        class_id: u16,
    ) -> Result<Mercenary, MercenaryError> {
        let template = self
            .database
            .get(class_id)
            .ok_or(MercenaryError::NotFound(class_id as u32))?;

        let mut next_id = self.next_id.write();
        let mercenary_id = *next_id;
        *next_id += 1;

        let mercenary = Mercenary {
            mercenary_id,
            owner_id,
            mercenary_class: class_id,
            name: template.name.clone(),
            level: template.level,
            hp: template.hp,
            max_hp: template.hp,
            sp: template.sp,
            max_sp: template.sp,
            atk: template.atk,
            defense: template.defense,
            magic_defense: template.magic_defense,
            str: template.str,
            agi: template.agi,
            vit: template.vit,
            int: template.int,
            dex: template.dex,
            luk: template.luk,
            hit: template.dex as i16,
            flee: template.agi as i16,
            walk_speed: template.walk_speed,
            attack_range: template.attack_range,
            loyalty: 100,
            contract_end: Some(Utc::now() + Duration::hours(48)), // 48 小时合同
            contract_cost: template.contract_cost,
            alive: true,
            skills: template
                .skills
                .iter()
                .map(|(name, max)| MercenarySkill {
                    skill_name: name.clone(),
                    max_level: *max,
                    current_level: 0,
                })
                .collect(),
        };

        self.mercenaries
            .write()
            .insert(mercenary_id, mercenary.clone());
        Ok(mercenary)
    }

    /// 召唤雇佣兵
    pub fn summon(&self, char_id: u32, mercenary_id: u32) -> Result<(), MercenaryError> {
        if self.summoned.read().contains_key(&char_id) {
            return Err(MercenaryError::AlreadySummoned);
        }

        let mercenaries = self.mercenaries.read();
        let mercenary = mercenaries
            .get(&mercenary_id)
            .ok_or(MercenaryError::NotFound(mercenary_id))?;

        // 验证归属
        if mercenary.owner_id != char_id {
            return Err(MercenaryError::NotYours);
        }

        // 检查合同
        if mercenary.is_contract_expired() {
            return Err(MercenaryError::ContractExpired);
        }

        // 检查存活
        if !mercenary.alive {
            return Err(MercenaryError::Dead);
        }

        drop(mercenaries);
        self.summoned.write().insert(char_id, mercenary_id);
        Ok(())
    }

    /// 解散雇佣兵
    pub fn dismiss(&self, char_id: u32) -> Option<u32> {
        self.summoned.write().remove(&char_id)
    }

    /// 获取玩家召唤的雇佣兵
    pub fn get_summoned(&self, char_id: u32) -> Option<Mercenary> {
        let summoned = self.summoned.read();
        let mercenary_id = summoned.get(&char_id)?;
        self.mercenaries.read().get(mercenary_id).cloned()
    }

    /// 获取雇佣兵（通过 ID）
    pub fn get(&self, mercenary_id: u32) -> Option<Mercenary> {
        self.mercenaries.read().get(&mercenary_id).cloned()
    }

    /// 更新合同（检查到期，自动解散）
    pub fn update_contracts(&self) -> Vec<u32> {
        let mut dismissed = Vec::new();
        let summoned = self.summoned.read();
        let mercenaries = self.mercenaries.read();

        for (char_id, mercenary_id) in summoned.iter() {
            if let Some(mercenary) = mercenaries.get(mercenary_id) {
                if mercenary.is_contract_expired() {
                    dismissed.push(*char_id);
                }
            }
        }

        drop(summoned);
        drop(mercenaries);

        // 解散到期的雇佣兵
        for char_id in &dismissed {
            self.summoned.write().remove(char_id);
        }

        dismissed
    }

    /// 增加忠诚度
    pub fn increase_loyalty(
        &self,
        mercenary_id: u32,
        amount: u32,
    ) -> Result<(), MercenaryError> {
        let mut mercenaries = self.mercenaries.write();
        let mercenary = mercenaries
            .get_mut(&mercenary_id)
            .ok_or(MercenaryError::NotFound(mercenary_id))?;
        mercenary.increase_loyalty(amount);
        Ok(())
    }

    /// 获取雇佣兵模板数据库
    pub fn database(&self) -> &MercenaryDatabase {
        &self.database
    }
}

impl Default for MercenaryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mercenary() {
        let manager = MercenaryManager::new();
        let merc = manager.create(100, 6017).unwrap();

        assert_eq!(merc.owner_id, 100);
        assert_eq!(merc.name, "Mina");
        assert_eq!(merc.level, 20);
        assert!(merc.alive);
        assert!(merc.contract_end.is_some());
        assert_eq!(merc.loyalty, 100);
    }

    #[test]
    fn test_create_mercenary_not_found() {
        let manager = MercenaryManager::new();
        assert!(matches!(
            manager.create(100, 9999),
            Err(MercenaryError::NotFound(9999))
        ));
    }

    #[test]
    fn test_summon_dismiss() {
        let manager = MercenaryManager::new();
        let merc = manager.create(100, 6017).unwrap();

        assert!(manager.summon(100, merc.mercenary_id).is_ok());
        assert!(manager.get_summoned(100).is_some());

        manager.dismiss(100);
        assert!(manager.get_summoned(100).is_none());
    }

    #[test]
    fn test_cannot_summon_twice() {
        let manager = MercenaryManager::new();
        let merc1 = manager.create(100, 6017).unwrap();
        let merc2 = manager.create(100, 6018).unwrap();

        manager.summon(100, merc1.mercenary_id).unwrap();
        assert!(matches!(
            manager.summon(100, merc2.mercenary_id),
            Err(MercenaryError::AlreadySummoned)
        ));
    }

    #[test]
    fn test_wrong_owner() {
        let manager = MercenaryManager::new();
        let merc = manager.create(100, 6017).unwrap();

        assert!(matches!(
            manager.summon(999, merc.mercenary_id),
            Err(MercenaryError::NotYours)
        ));
    }

    #[test]
    fn test_contract_expired() {
        let manager = MercenaryManager::new();
        let merc = manager.create(100, 6017).unwrap();

        // 手动设置合同已过期
        {
            let mut mercenaries = manager.mercenaries.write();
            let m = mercenaries.get_mut(&merc.mercenary_id).unwrap();
            m.contract_end = Some(Utc::now() - Duration::hours(1));
        }

        assert!(matches!(
            manager.summon(100, merc.mercenary_id),
            Err(MercenaryError::ContractExpired)
        ));
    }

    #[test]
    fn test_loyalty() {
        let manager = MercenaryManager::new();
        let merc = manager.create(100, 6017).unwrap();

        manager
            .increase_loyalty(merc.mercenary_id, 50)
            .unwrap();

        let m = manager.get(merc.mercenary_id).unwrap();
        assert_eq!(m.loyalty, 150); // 100 + 50
    }

    #[test]
    fn test_update_contracts() {
        let manager = MercenaryManager::new();
        let merc = manager.create(100, 6017).unwrap();
        manager.summon(100, merc.mercenary_id).unwrap();

        // 设置合同即将到期
        {
            let mut mercenaries = manager.mercenaries.write();
            let m = mercenaries.get_mut(&merc.mercenary_id).unwrap();
            m.contract_end = Some(Utc::now() - Duration::seconds(1));
        }

        let dismissed = manager.update_contracts();
        assert_eq!(dismissed.len(), 1);
        assert_eq!(dismissed[0], 100);
        assert!(manager.get_summoned(100).is_none());
    }

    #[test]
    fn test_mercenary_skills_from_template() {
        let manager = MercenaryManager::new();
        let merc = manager.create(100, 6017).unwrap();

        assert_eq!(merc.skills.len(), 2);
        assert_eq!(merc.skills[0].skill_name, "MA_DOUBLE");
        assert_eq!(merc.skills[0].max_level, 2);
    }
}
