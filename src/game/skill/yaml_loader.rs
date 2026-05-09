//! rAthena skill_db.yml 格式加载器
//!
//! 从 `db/skill_db.yml` 加载技能数据，映射到 Deviruchi 的 Skill 结构。

use super::data::{Skill, SkillTarget, SkillType};
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs;

/// rAthena skill_db.yml 文件结构
#[derive(Deserialize, Debug)]
struct SkillDbFile {
    Header: SkillDbHeader,
    Body: Option<Vec<SkillDbEntry>>,
}

#[derive(Deserialize, Debug)]
struct SkillDbHeader {
    #[serde(rename = "Type")]
    #[allow(dead_code)]
    _type: String,
    #[allow(dead_code)]
    Version: u32,
}

/// rAthena skill_db.yml 中的技能条目
#[derive(Deserialize, Debug)]
struct SkillDbEntry {
    Id: u16,
    Name: String,
    Description: Option<String>,
    #[serde(rename = "MaxLevel", default)]
    max_level: u8,
    #[serde(rename = "Type")]
    type_: Option<String>,
    #[serde(rename = "TargetType")]
    target_type: Option<String>,
    #[serde(rename = "Range", default)]
    range: Option<LevelOrValue>,
    #[serde(rename = "HitCount", default)]
    hit_count: Option<LevelOrValue>,
    #[serde(rename = "Element", default)]
    element: Option<LevelOrString>,
    #[serde(rename = "SplashArea", default)]
    splash_area: Option<LevelOrValue>,
    #[serde(rename = "Requires")]
    requires: Option<SkillRequires>,
    #[serde(rename = "DamageFlags", default)]
    damage_flags: Option<HashMap<String, bool>>,
    #[serde(rename = "Flags", default)]
    flags: Option<HashMap<String, bool>>,
    #[serde(rename = "CastTime", default)]
    cast_time: Option<Vec<LevelTime>>,
    #[serde(rename = "AfterCastActDelay", default)]
    after_cast_delay: Option<Vec<LevelTime>>,
    #[serde(rename = "Cooldown", default)]
    cooldown: Option<Vec<LevelTime>>,
    #[serde(rename = "Duration1", default)]
    duration1: Option<Vec<LevelTime>>,
    #[serde(rename = "Duration2", default)]
    duration2: Option<Vec<LevelTime>>,
}

/// 值可以是单个数字或按等级列表（支持负数，如 Range: -1）
#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum LevelOrValue {
    Single(i32),
    PerLevel(Vec<LevelAmount>),
}

/// 值可以是单个字符串或按等级列表
#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum LevelOrString {
    Single(String),
    PerLevel(Vec<LevelElement>),
}

#[derive(Deserialize, Debug)]
struct LevelAmount {
    Level: u8,
    Amount: i32,
}

#[derive(Deserialize, Debug)]
struct LevelElement {
    Level: u8,
    Element: String,
}

/// 按等级的时间值（用于 CastTime/Cooldown/Duration 等）
#[derive(Deserialize, Debug)]
struct LevelTime {
    Level: u8,
    Time: u32,
}

#[derive(Deserialize, Debug)]
struct SkillRequires {
    #[serde(rename = "SpCost", default)]
    sp_cost: Option<Vec<LevelAmount>>,
    #[serde(rename = "HpCost", default)]
    hp_cost: Option<Vec<LevelAmount>>,
    #[serde(rename = "ZenyCost", default)]
    zeny_cost: Option<Vec<LevelAmount>>,
}

/// 从按等级时间列表中获取指定等级的时间值（毫秒）
fn get_level_time(v: &Option<Vec<LevelTime>>, level: u8) -> u32 {
    match v {
        Some(times) => times.iter()
            .find(|lt| lt.Level == level)
            .map(|lt| lt.Time)
            .unwrap_or(0),
        None => 0,
    }
}

/// 从 LevelOrValue 中获取指定等级的值
fn get_level_value(v: &Option<LevelOrValue>, level: u8) -> i32 {
    match v {
        Some(LevelOrValue::Single(val)) => *val,
        Some(LevelOrValue::PerLevel(levels)) => {
            levels.iter()
                .find(|la| la.Level == level)
                .map(|la| la.Amount)
                .unwrap_or(0)
        }
        None => 0,
    }
}

/// 从 LevelOrString 中获取指定等级的元素
fn get_level_element(v: &Option<LevelOrString>, level: u8) -> String {
    match v {
        Some(LevelOrString::Single(s)) => s.clone(),
        Some(LevelOrString::PerLevel(levels)) => {
            levels.iter()
                .find(|le| le.Level == level)
                .map(|le| le.Element.clone())
                .unwrap_or_else(|| "Neutral".to_string())
        }
        None => "Neutral".to_string(),
    }
}

/// 将 rAthena Type 映射到 Deviruchi SkillType
fn map_skill_type(type_str: &Option<String>, damage_flags: &Option<HashMap<String, bool>>) -> SkillType {
    match type_str.as_deref() {
        Some("Weapon") => SkillType::Attack,
        Some("Magic") => SkillType::Attack,
        Some("Heal") => SkillType::Healing,
        Some("Buff") => SkillType::Support,
        Some("Debuff") => SkillType::Debuff,
        _ => {
            // 从 DamageFlags 推断
            if let Some(flags) = damage_flags {
                if flags.get("NoDamage").copied() == Some(true) {
                    return SkillType::Support;
                }
            }
            SkillType::Active
        }
    }
}

/// 将 rAthena TargetType 映射到 Deviruchi SkillTarget
fn map_target_type(target: &Option<String>) -> SkillTarget {
    match target.as_deref() {
        Some("Attack") | Some("Enemy") => SkillTarget::Enemy,
        Some("Support") | Some("Party") => SkillTarget::Party,
        Some("Self") => SkillTarget::Self_,
        Some("Ground") => SkillTarget::Ground,
        _ => SkillTarget::Enemy,
    }
}

/// 将元素名称映射到 u8
fn map_element(name: &str) -> u8 {
    match name {
        "Neutral" => 0,
        "Water" => 1,
        "Earth" => 2,
        "Fire" => 3,
        "Wind" => 4,
        "Poison" => 5,
        "Holy" => 6,
        "Dark" => 7,
        "Ghost" => 8,
        "Undead" => 9,
        "Weapon" => 0, // 使用武器元素，映射为无
        _ => 0,
    }
}

impl SkillDbEntry {
    fn to_skill(&self) -> Skill {
        let level = self.max_level.max(1);
        // SP 消耗：从 Requires.SpCost 的第一级获取
        let sp_cost = self.requires.as_ref()
            .and_then(|r| r.sp_cost.as_ref())
            .and_then(|costs| costs.first())
            .map(|la| la.Amount.max(0) as u16)
            .unwrap_or(0);
        let range_val = get_level_value(&self.range, 1);
        let element_str = get_level_element(&self.element, 1);

        // HP 消耗：从 Requires.HpCost 的第一级获取
        let hp_cost = self.requires.as_ref()
            .and_then(|r| r.hp_cost.as_ref())
            .and_then(|costs| costs.first())
            .map(|la| la.Amount.max(0) as u32)
            .unwrap_or(0);

        // 时间相关字段：取第一级的值
        let cast_time = get_level_time(&self.cast_time, 1);
        let cooldown = get_level_time(&self.cooldown, 1)
            .max(get_level_time(&self.after_cast_delay, 1));
        let skill_time = get_level_time(&self.duration1, 1)
            .max(get_level_time(&self.duration2, 1));

        Skill {
            id: self.Id,
            name: self.Description.clone().unwrap_or_else(|| self.Name.clone()),
            type_: map_skill_type(&self.type_, &self.damage_flags),
            target: map_target_type(&self.target_type),
            level,
            sp_cost,
            hp_cost,
            cast_time,
            cooldown,
            range: if range_val < 0 { 0 } else { range_val as u16 },
            skill_time,
            damage: 0,
            hit: 0,
            element: map_element(&element_str),
            flags: 0,
        }
    }
}

/// 从 rAthena skill_db.yml 加载技能数据库
pub fn load_skill_db(path: &str) -> Result<HashMap<u16, Skill>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let yaml: SkillDbFile = serde_yaml::from_str(&content)?;
    let mut db = HashMap::new();
    if let Some(body) = yaml.Body {
        for entry in body {
            db.insert(entry.Id, entry.to_skill());
        }
    }
    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_skill_db_from_string() {
        let yaml_str = r#"
Header:
  Type: SKILL_DB
  Version: 4
Body:
  - Id: 1
    Name: NV_BASIC
    Description: Basic Skill
    MaxLevel: 9
  - Id: 5
    Name: SM_BASH
    Description: Bash
    MaxLevel: 10
    Type: Weapon
    TargetType: Attack
    Range: -1
    HitCount: 1
    Element: Weapon
    DamageFlags:
      NoDamage: false
    Requires:
      SpCost:
        - Level: 1
          Amount: 8
        - Level: 10
          Amount: 15
  - Id: 28
    Name: AL_HEAL
    Description: Heal
    MaxLevel: 10
    Type: Heal
    TargetType: Party
    Element: Holy
    Requires:
      SpCost:
        - Level: 1
          Amount: 13
        - Level: 10
          Amount: 40
"#;

        let tmp_path = "/tmp/test_skill_db.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();
        let skills = load_skill_db(tmp_path).unwrap();
        assert_eq!(skills.len(), 3);

        let basic = skills.get(&1).unwrap();
        assert_eq!(basic.name, "Basic Skill");
        assert_eq!(basic.level, 9);
        assert_eq!(basic.type_, SkillType::Active);

        let bash = skills.get(&5).unwrap();
        assert_eq!(bash.name, "Bash");
        assert_eq!(bash.level, 10);
        assert_eq!(bash.type_, SkillType::Attack);
        assert_eq!(bash.target, SkillTarget::Enemy);
        assert_eq!(bash.sp_cost, 8);
        assert_eq!(bash.range, 0); // -1 映射为 0（武器范围）

        let heal = skills.get(&28).unwrap();
        assert_eq!(heal.name, "Heal");
        assert_eq!(heal.type_, SkillType::Healing);
        assert_eq!(heal.target, SkillTarget::Party);
        assert_eq!(heal.element, 6); // Holy
        assert_eq!(heal.sp_cost, 13);

        std::fs::remove_file(tmp_path).ok();
    }

    #[test]
    fn test_map_element() {
        assert_eq!(map_element("Neutral"), 0);
        assert_eq!(map_element("Fire"), 3);
        assert_eq!(map_element("Holy"), 6);
        assert_eq!(map_element("Weapon"), 0);
    }

    #[test]
    fn test_map_skill_type() {
        assert_eq!(map_skill_type(&Some("Weapon".to_string()), &None), SkillType::Attack);
        assert_eq!(map_skill_type(&Some("Heal".to_string()), &None), SkillType::Healing);
        assert_eq!(map_skill_type(&Some("Buff".to_string()), &None), SkillType::Support);
        assert_eq!(map_skill_type(&None, &None), SkillType::Active);
    }
}
