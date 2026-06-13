#![allow(dead_code)]

//! 怪物技能数据库
//!
//! 从 `db/mob_skill_db.yml` 加载独立的怪物技能条目。
//! 这些条目可以按 mob_id 查询，返回该怪物可使用的所有技能。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

use super::data::{MobSkill, MobSkillCondition, MobSkillTarget};

/// 独立怪物技能数据库条目（YAML 映射结构）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MobSkillEntry {
    /// 怪物 ID
    pub mob_id: u16,
    /// 技能 ID（内部数字 ID）
    pub skill_id: u16,
    /// 技能等级
    pub level: u8,
    /// 触发概率（万分比，0-10000）
    pub rate: u16,
    /// 吟唱时间（毫秒）
    pub cast_time: u32,
    /// 冷却/延迟时间（毫秒）
    pub delay: u32,
    /// 技能目标类型
    pub target: MobSkillTarget,
    /// 触发条件类型
    pub condition: MobSkillCondition,
    /// 条件值（如 HP 百分比阈值）
    #[serde(default)]
    pub condition_value: u32,
}

/// 怪物技能数据库
///
/// 内部以 `mob_id -> Vec<MobSkillEntry>` 组织，支持按怪物查询技能列表。
pub struct MobSkillDatabase {
    entries: HashMap<u16, Vec<MobSkillEntry>>,
}

impl MobSkillDatabase {
    /// 创建空数据库
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// 从 YAML 文件加载怪物技能数据库
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let entries: Vec<MobSkillEntry> = serde_yaml::from_str(&content)?;

        let count = entries.len();
        let mut map: HashMap<u16, Vec<MobSkillEntry>> = HashMap::new();
        for entry in entries {
            map.entry(entry.mob_id).or_default().push(entry);
        }

        tracing::info!("从 {} 加载了 {} 条怪物技能（{} 种怪物）", path, count, map.len());

        Ok(Self { entries: map })
    }

    /// 从默认路径 `db/mob_skill_db.yml` 加载，不存在则返回空数据库
    pub fn load_default() -> Self {
        let path = "db/mob_skill_db.yml";
        if std::path::Path::new(path).exists() {
            match Self::load(path) {
                Ok(db) => return db,
                Err(e) => tracing::warn!("加载 {} 失败: {}", path, e),
            }
        }
        Self::new()
    }

    /// 获取指定怪物的所有技能条目
    pub fn get_skills(&self, mob_id: u16) -> &[MobSkillEntry] {
        self.entries
            .get(&mob_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 将指定怪物的技能条目转换为运行时 `MobSkill` 列表
    pub fn to_mob_skills(&self, mob_id: u16, rate_multiplier: f64, delay_multiplier: f64) -> Vec<MobSkill> {
        self.get_skills(mob_id)
            .iter()
            .map(|e| e.to_mob_skill(rate_multiplier, delay_multiplier))
            .collect()
    }

    /// 获取数据库中的技能条目总数
    pub fn count(&self) -> usize {
        self.entries.values().map(|v| v.len()).sum()
    }

    /// 获取覆盖的怪物种类数
    pub fn mob_count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for MobSkillDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl MobSkillEntry {
    /// 将数据库条目转换为运行时 `MobSkill` 结构
    ///
    /// `rate_multiplier` 和 `delay_multiplier` 来自 BattleConfig（百分比，1.0 = 100%）。
    pub fn to_mob_skill(&self, rate_multiplier: f64, delay_multiplier: f64) -> MobSkill {
        let adjusted_rate = ((self.rate as f64) * rate_multiplier) as u32;
        let adjusted_cooldown = ((self.delay as f64) * delay_multiplier) as u64;

        MobSkill {
            skill_id: self.skill_id,
            level: self.level,
            chance: adjusted_rate,
            target: self.target,
            condition: self.condition,
            condition_value: self.condition_value,
            cooldown_ms: adjusted_cooldown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_database() {
        let db = MobSkillDatabase::new();
        assert_eq!(db.count(), 0);
        assert_eq!(db.mob_count(), 0);
        assert!(db.get_skills(1001).is_empty());
    }

    #[test]
    fn test_load_from_yaml() {
        let yaml = r#"
- mob_id: 1002
  skill_id: 28
  level: 3
  rate: 1000
  cast_time: 0
  delay: 10000
  target: self
  condition: HpCertain
  condition_value: 50
- mob_id: 1002
  skill_id: 5
  level: 5
  rate: 500
  cast_time: 800
  delay: 5000
  target: target
  condition: Any
  condition_value: 0
"#;
        let entries: Vec<MobSkillEntry> = serde_yaml::from_str(yaml).unwrap();
        let mut map: HashMap<u16, Vec<MobSkillEntry>> = HashMap::new();
        for entry in entries {
            map.entry(entry.mob_id).or_default().push(entry);
        }
        let db = MobSkillDatabase { entries: map };

        assert_eq!(db.count(), 2);
        assert_eq!(db.mob_count(), 1);

        let skills = db.get_skills(1002);
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].skill_id, 28);
        assert_eq!(skills[0].level, 3);
        assert_eq!(skills[0].rate, 1000);
        assert_eq!(skills[0].delay, 10000);
        assert_eq!(skills[0].target, MobSkillTarget::Self_);
        assert_eq!(skills[0].condition, MobSkillCondition::HpCertain);
        assert_eq!(skills[0].condition_value, 50);

        assert_eq!(skills[1].skill_id, 5);
        assert_eq!(skills[1].target, MobSkillTarget::Target);
        assert_eq!(skills[1].condition, MobSkillCondition::Any);
    }

    #[test]
    fn test_to_mob_skill_conversion() {
        let entry = MobSkillEntry {
            mob_id: 1001,
            skill_id: 5,
            level: 3,
            rate: 1000,
            cast_time: 800,
            delay: 5000,
            target: MobSkillTarget::Target,
            condition: MobSkillCondition::Any,
            condition_value: 0,
        };

        // 100% rate, 100% delay
        let skill = entry.to_mob_skill(1.0, 1.0);
        assert_eq!(skill.skill_id, 5);
        assert_eq!(skill.level, 3);
        assert_eq!(skill.chance, 1000);
        assert_eq!(skill.cooldown_ms, 5000);

        // 150% rate, 80% delay
        let skill = entry.to_mob_skill(1.5, 0.8);
        assert_eq!(skill.chance, 1500);
        assert_eq!(skill.cooldown_ms, 4000);
    }

    #[test]
    fn test_to_mob_skills_list() {
        let yaml = r#"
- mob_id: 1312
  skill_id: 5
  level: 3
  rate: 500
  cast_time: 0
  delay: 5000
  target: target
  condition: Any
- mob_id: 1312
  skill_id: 28
  level: 1
  rate: 300
  cast_time: 0
  delay: 8000
  target: self
  condition: HpCertain
  condition_value: 30
"#;
        let entries: Vec<MobSkillEntry> = serde_yaml::from_str(yaml).unwrap();
        let mut map: HashMap<u16, Vec<MobSkillEntry>> = HashMap::new();
        for entry in entries {
            map.entry(entry.mob_id).or_default().push(entry);
        }
        let db = MobSkillDatabase { entries: map };

        let skills = db.to_mob_skills(1312, 1.0, 1.0);
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].skill_id, 5);
        assert_eq!(skills[1].skill_id, 28);
    }

    #[test]
    fn test_nonexistent_mob_returns_empty() {
        let db = MobSkillDatabase::new();
        assert!(db.get_skills(9999).is_empty());
        assert!(db.to_mob_skills(9999, 1.0, 1.0).is_empty());
    }

    #[test]
    fn test_serialize_roundtrip() {
        let entry = MobSkillEntry {
            mob_id: 1001,
            skill_id: 5,
            level: 3,
            rate: 500,
            cast_time: 0,
            delay: 5000,
            target: MobSkillTarget::Target,
            condition: MobSkillCondition::Any,
            condition_value: 0,
        };

        let yaml_str = serde_yaml::to_string(&entry).unwrap();
        let deserialized: MobSkillEntry = serde_yaml::from_str(&yaml_str).unwrap();
        assert_eq!(deserialized.mob_id, 1001);
        assert_eq!(deserialized.skill_id, 5);
        assert_eq!(deserialized.rate, 500);
    }
}
