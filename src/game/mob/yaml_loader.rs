#![allow(non_snake_case)]

//! Mob YAML 数据加载器
//!
//! 从 rAthena mob_db.yml 格式加载怪物模板数据。
//! rAthena 格式: Header (Type + Version) -> Body (条目列表) -> Footer (Imports)

use super::data::{
    MobBehavior, MobBehaviorFlags, MobDrop, MobRace, MobSkill, MobSkillCondition, MobSkillTarget,
    MobTemplate, MobType,
};
use crate::game::battle::element::{Element, ElementLevel, MobSize};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

/// 全局物品名称到 ID 映射（从 item_db YAML 动态加载）
static ITEM_NAME_TO_ID: once_cell::sync::Lazy<std::collections::HashMap<String, u16>> =
    once_cell::sync::Lazy::new(|| {
        crate::game::item::yaml_loader::load_item_name_to_id_map()
    });

/// 物品名称到 ID 的映射
///
/// 优先从全局映射（从 item_db YAML 加载）查找，
/// 未找到时返回 0 并输出警告。
pub fn item_name_to_id(name: &str) -> u32 {
    ITEM_NAME_TO_ID
        .get(name)
        .map(|&id| id as u32)
        .unwrap_or_else(|| {
            tracing::warn!("未知物品名称: {}，item_id 设为 0", name);
            0
        })
}

/// 全局技能名称到 ID 映射（从 skill_db.yml 动态加载）
///
/// rAthena 的 mob_db.yml 中 Skills 使用技能名称（如 "SM_BASH"），
/// 而系统内部使用数字 ID，此映射用于转换。
static SKILL_NAME_TO_ID: once_cell::sync::Lazy<std::collections::HashMap<String, u16>> =
    once_cell::sync::Lazy::new(load_skill_name_to_id_map);

/// 从 skill_db.yml 加载技能名称到 ID 的映射表
fn load_skill_name_to_id_map() -> std::collections::HashMap<String, u16> {
    let skill_db_paths = ["db/skill_db.yml"];
    for path in &skill_db_paths {
        if std::path::Path::new(path).exists() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    match serde_yaml::from_str::<SkillDbForMapping>(&content) {
                        Ok(yaml) => {
                            let mut map = std::collections::HashMap::new();
                            if let Some(body) = yaml.Body {
                                for entry in body {
                                    // Name 字段就是 rAthena 的 Aegis 技能名（如 "SM_BASH"）
                                    map.insert(entry.Name, entry.Id);
                                }
                            }
                            tracing::info!("加载了 {} 个技能名称映射", map.len());
                            return map;
                        }
                        Err(e) => {
                            tracing::warn!("解析 {} 失败: {}", path, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("读取 {} 失败: {}", path, e);
                }
            }
        }
    }
    tracing::warn!("未找到 skill_db.yml，技能名称映射为空");
    std::collections::HashMap::new()
}

/// 用于从 skill_db.yml 中提取 Id 和 Name 的简化结构
#[derive(Deserialize, Debug)]
struct SkillDbForMapping {
    #[allow(dead_code)] // rAthena YAML compat
    Header: serde_yaml::Value,
    Body: Option<Vec<SkillDbMappingEntry>>,
}

#[derive(Deserialize, Debug)]
struct SkillDbMappingEntry {
    Id: u16,
    Name: String,
}

/// 技能名称到 ID 的映射
///
/// 从 skill_db.yml 的 Name 字段查找对应的技能 ID。
/// 未找到时返回 0 并输出警告。
pub fn skill_name_to_id(name: &str) -> u16 {
    SKILL_NAME_TO_ID
        .get(name)
        .copied()
        .unwrap_or_else(|| {
            tracing::warn!("未知技能名称: {}，skill_id 设为 0", name);
            0
        })
}

/// rAthena mob_db.yml 文件结构
#[derive(Deserialize, Debug)]
struct MobYamlFile {
    #[allow(dead_code)] // rAthena YAML compat
    Header: MobYamlHeader,
    Body: Option<Vec<MobYamlEntry>>,
    #[allow(dead_code)] // rAthena YAML compat
    Footer: Option<MobYamlFooter>,
}

#[derive(Deserialize, Debug)]
struct MobYamlHeader {
    #[serde(rename = "Type")]
    #[allow(dead_code)] // rAthena YAML compat
    _type: String,
    #[allow(dead_code)] // rAthena YAML compat
    Version: u32,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)] // rAthena YAML compat
struct MobYamlFooter {
    Imports: Option<Vec<MobYamlImport>>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)] // rAthena YAML compat
struct MobYamlImport {
    Path: String,
    Mode: Option<String>,
}

/// rAthena mob_db.yml 中的怪物条目
#[derive(Deserialize, Debug)]
struct MobYamlEntry {
    Id: u16,
    #[serde(rename = "AegisName")]
    #[allow(dead_code)] // rAthena YAML compat
    _aegis_name: String,
    Name: String,
    #[serde(default)]
    Level: u16,
    #[serde(default = "default_hp")]
    Hp: u32,
    #[serde(default)]
    Sp: u32,
    #[serde(default)]
    BaseExp: u64,
    #[serde(default)]
    JobExp: u64,
    #[serde(default)]
    Attack: u16,
    #[serde(default)]
    Attack2: u16,
    #[serde(default)]
    Defense: u16,
    #[serde(default)]
    MagicDefense: u16,
    #[serde(default = "default_stat")]
    #[allow(dead_code)] // rAthena YAML compat
    Str: u16,
    #[serde(default = "default_stat")]
    Agi: u16,
    #[serde(default = "default_stat")]
    #[allow(dead_code)] // rAthena YAML compat
    Vit: u16,
    #[serde(default = "default_stat")]
    #[allow(dead_code)] // rAthena YAML compat
    Int: u16,
    #[serde(default = "default_stat")]
    Dex: u16,
    #[serde(default = "default_stat")]
    Luk: u16,
    #[serde(default)]
    AttackRange: u16,
    #[serde(default)]
    SkillRange: u16,
    #[serde(default)]
    ChaseRange: u16,
    #[serde(default = "default_size")]
    Size: String,
    #[serde(default)]
    Race: String,
    #[serde(default)]
    Element: String,
    #[serde(default = "default_element_level")]
    ElementLevel: u8,
    #[serde(default = "default_walk_speed")]
    WalkSpeed: u16,
    #[serde(default)]
    AttackDelay: u32,
    #[serde(default)]
    #[allow(dead_code)] // rAthena YAML compat
    AttackMotion: u32,
    #[serde(default)]
    #[allow(dead_code)] // rAthena YAML compat
    DamageMotion: u32,
    #[serde(default)]
    Ai: String,
    #[serde(default)]
    #[allow(dead_code)] // rAthena YAML compat
    Class: String,
    #[serde(default)]
    Modes: Option<HashMap<String, bool>>,
    #[serde(default)]
    Drops: Option<Vec<MobYamlDrop>>,
    #[serde(default)]
    MvpDrops: Option<Vec<MobYamlDrop>>,
    /// 怪物技能列表
    #[serde(default)]
    Skills: Option<Vec<MobSkillEntry>>,
}

/// rAthena 掉落条目
#[derive(Deserialize, Debug)]
struct MobYamlDrop {
    Item: String,
    #[serde(default)]
    Rate: u32,
    #[serde(default)]
    #[allow(dead_code)] // rAthena YAML compat
    StealProtected: Option<bool>,
}

/// rAthena mob_db.yml 中的技能条目
///
/// 对应 rAthena 格式:
///   - Id: SM_BASH
///     Lv: 5
///     Rate: 500
///     CastTime: 0
///     Delay: 5000
///     Emotion: 0
///     Target: target
///     Condition: any
///     ConditionValue: 0
#[derive(Deserialize, Debug)]
struct MobSkillEntry {
    /// 技能名称（rAthena Aegis 名称，如 "SM_BASH"）
    Id: String,
    /// 技能等级
    #[serde(default = "default_skill_level")]
    Lv: u8,
    /// 使用概率（万分比，10000 = 100%）
    #[serde(default)]
    Rate: u32,
    /// 吟唱时间（毫秒）
    #[serde(default)]
    #[allow(dead_code)] // rAthena YAML compat
    CastTime: u32,
    /// 冷却/延迟时间（毫秒）
    #[serde(default)]
    Delay: u32,
    /// 触发时的表情
    #[serde(default)]
    #[allow(dead_code)] // rAthena YAML compat
    Emotion: u32,
    /// 技能目标："target"（敌人）或 "self"（自身）
    #[serde(default = "default_skill_target")]
    Target: String,
    /// 触发条件："any"、"rudeattacked"、"longrange"、"hpcertain"
    #[serde(default = "default_skill_condition")]
    Condition: String,
    /// 条件值（如 HP 百分比阈值）
    #[serde(default)]
    ConditionValue: u32,
}

fn default_skill_level() -> u8 {
    1
}

fn default_skill_target() -> String {
    "target".to_string()
}

fn default_skill_condition() -> String {
    "any".to_string()
}

fn default_hp() -> u32 {
    1
}
fn default_stat() -> u16 {
    1
}
fn default_size() -> String {
    "Small".to_string()
}
fn default_element_level() -> u8 {
    1
}
fn default_walk_speed() -> u16 {
    200
}

/// 解析元素类型字符串
fn parse_element(s: &str) -> Element {
    match s.to_lowercase().as_str() {
        "neutral" => Element::Neutral,
        "water" => Element::Water,
        "earth" => Element::Earth,
        "fire" => Element::Fire,
        "wind" => Element::Wind,
        "poison" => Element::Poison,
        "holy" => Element::Holy,
        "dark" => Element::Dark,
        "ghost" => Element::Ghost,
        "undead" => Element::Undead,
        _ => Element::Neutral,
    }
}

/// 解析元素等级
fn parse_element_level(level: u8) -> ElementLevel {
    match level {
        1 => ElementLevel::Level1,
        2 => ElementLevel::Level2,
        3 => ElementLevel::Level3,
        4 => ElementLevel::Level4,
        _ => ElementLevel::Level1,
    }
}

/// 解析体型字符串
fn parse_size(s: &str) -> MobSize {
    match s.to_lowercase().as_str() {
        "small" => MobSize::Small,
        "medium" => MobSize::Medium,
        "large" => MobSize::Large,
        _ => MobSize::Medium,
    }
}

/// 解析怪物种族
fn parse_race(s: &str) -> MobRace {
    match s.to_lowercase().as_str() {
        "formless" => MobRace::Formless,
        "undead" => MobRace::Undead,
        "brute" => MobRace::Brute,
        "plant" => MobRace::Plant,
        "insect" => MobRace::Insect,
        "fish" => MobRace::Fish,
        "demon" => MobRace::Demon,
        "demihuman" | "demi_human" => MobRace::DemiHuman,
        "angel" => MobRace::Angel,
        "dragon" => MobRace::Dragon,
        _ => MobRace::Formless,
    }
}

/// 解析 AI 行为类型
///
/// rAthena AI 类型:
/// 01 = 攻击性 (Aggressive)
/// 03 = 被动 (Passive)
/// 04 = 协助 (Assist)
/// 05 = 被动 + 协助
/// 06 = 默认（被动）
fn parse_behavior(ai: &str, modes: &Option<HashMap<String, bool>>) -> MobBehavior {
    match ai {
        "01" => MobBehavior::Aggressive,
        "03" => MobBehavior::Passive,
        "04" => MobBehavior::Assist,
        "05" => MobBehavior::PassiveAssist,
        _ => {
            // 检查 Modes 中的 CanMove 字段
            if let Some(modes) = modes
                && modes.get("CanMove").copied() == Some(false) {
                    return MobBehavior::Immobile;
                }
            MobBehavior::Passive
        }
    }
}

/// 解析怪物行为标记（Modes 字段）
fn parse_modes(modes: &Option<HashMap<String, bool>>) -> MobBehaviorFlags {
    let mut flags = MobBehaviorFlags::default();
    if let Some(m) = modes {
        if m.get("CanMove").copied() == Some(false) {
            flags.can_move = false;
        }
        if m.get("CanAttack").copied() == Some(false) {
            flags.can_attack = false;
        }
        if m.get("Detector").copied() == Some(true) {
            flags.detector = true;
        }
        if m.get("Boss").copied() == Some(true) {
            flags.boss = true;
        }
        if m.get("Plant").copied() == Some(true) {
            flags.plant = true;
        }
        if m.get("CanChase").copied() == Some(false) {
            flags.can_chase = false;
        }
    }
    flags
}

/// 解析技能目标类型字符串
fn parse_skill_target(s: &str) -> MobSkillTarget {
    match s.to_lowercase().as_str() {
        "self" => MobSkillTarget::Self_,
        _ => MobSkillTarget::Target,
    }
}

/// 解析技能触发条件字符串
fn parse_skill_condition(s: &str) -> MobSkillCondition {
    match s.to_lowercase().as_str() {
        "rudeattacked" => MobSkillCondition::RudeAttacked,
        "longrange" => MobSkillCondition::LongRange,
        "hpcertain" => MobSkillCondition::HpCertain,
        _ => MobSkillCondition::Any,
    }
}

/// 将 rAthena MobSkillEntry 转换为内部 MobSkill 结构
fn convert_mob_skill(entry: &MobSkillEntry) -> MobSkill {
    let skill_id = skill_name_to_id(&entry.Id);
    MobSkill {
        skill_id,
        level: entry.Lv,
        chance: entry.Rate,
        target: parse_skill_target(&entry.Target),
        condition: parse_skill_condition(&entry.Condition),
        condition_value: entry.ConditionValue,
        cooldown_ms: entry.Delay as u64,
    }
}

/// 从 rAthena mob_db.yml 加载怪物模板
///
/// 返回 (mob_id -> MobTemplate) 映射
pub fn load_mob_db(path: &str) -> Result<HashMap<u16, MobTemplate>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let yaml: MobYamlFile = serde_yaml::from_str(&content)?;

    let mut mobs = HashMap::new();

    if let Some(body) = yaml.Body {
        for entry in body {
            let template = MobTemplate {
                name: entry.Name.clone(),
                level: if entry.Level == 0 { 1 } else { entry.Level },
                hp: entry.Hp,
                sp: entry.Sp,
                atk: entry.Attack,
                matk: entry.Attack2,
                defense: entry.Defense,
                magic_defense: entry.MagicDefense,
                hit: entry.Dex as i16,   // rAthena 用 Dex 作为 hit
                flee: entry.Agi as i16,  // rAthena 用 Agi 作为 flee
                crit: entry.Luk as i16 / 3, // 大约的 crit 值
                walk_speed: entry.WalkSpeed,
                atk_range: entry.AttackRange,
                sight_range: if entry.SkillRange > 0 {
                    entry.SkillRange
                } else {
                    12
                },
                chase_range: if entry.ChaseRange > 0 {
                    entry.ChaseRange
                } else {
                    12
                },
                // Aggressive AI -> aggro_rate=100，其他默认 0
                aggro_rate: match entry.Ai.as_str() {
                    "01" => 100,
                    _ => 0,
                },
                spawn_delay: entry.AttackDelay,
                respawn_time: 60000, // 默认重生时间（后续可从 Spawn 数据读取）
                behavior: parse_behavior(&entry.Ai, &entry.Modes),
                skills: entry
                    .Skills
                    .as_ref()
                    .map(|skills| skills.iter().map(convert_mob_skill).collect())
                    .unwrap_or_default(),
                drops: entry
                    .Drops
                    .as_ref()
                    .map(|drops| {
                        drops
                            .iter()
                            .map(|d| {
                                // 将物品名称映射到 ID，Rate 是万分比 (10000 = 100%)
                                let item_id = item_name_to_id(&d.Item);
                                MobDrop::new(item_id, d.Rate)
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                base_exp: entry.BaseExp,
                job_exp: entry.JobExp,
                zeny: None,
                element: parse_element(&entry.Element),
                element_level: parse_element_level(entry.ElementLevel),
                size: parse_size(&entry.Size),
                race: parse_race(&entry.Race),
                mob_type: if entry
                    .Modes
                    .as_ref()
                    .and_then(|m| m.get("Boss").copied())
                    .unwrap_or(false)
                {
                    MobType::Boss
                } else {
                    MobType::Normal
                },
                mvp_drops: entry
                    .MvpDrops
                    .as_ref()
                    .map(|drops| {
                        drops
                            .iter()
                            .map(|d| {
                                let item_id = item_name_to_id(&d.Item);
                                MobDrop::new(item_id, d.Rate)
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                behavior_flags: parse_modes(&entry.Modes),
            };

            mobs.insert(entry.Id, template);
        }
    }

    Ok(mobs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_element() {
        assert_eq!(parse_element("Neutral"), Element::Neutral);
        assert_eq!(parse_element("Fire"), Element::Fire);
        assert_eq!(parse_element("Water"), Element::Water);
        assert_eq!(parse_element("EARTH"), Element::Earth);
        assert_eq!(parse_element("unknown"), Element::Neutral);
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("Small"), MobSize::Small);
        assert_eq!(parse_size("Medium"), MobSize::Medium);
        assert_eq!(parse_size("Large"), MobSize::Large);
        assert_eq!(parse_size("unknown"), MobSize::Medium);
    }

    #[test]
    fn test_parse_race() {
        assert_eq!(parse_race("Formless"), MobRace::Formless);
        assert_eq!(parse_race("Undead"), MobRace::Undead);
        assert_eq!(parse_race("Brute"), MobRace::Brute);
        assert_eq!(parse_race("Plant"), MobRace::Plant);
        assert_eq!(parse_race("Insect"), MobRace::Insect);
        assert_eq!(parse_race("Fish"), MobRace::Fish);
        assert_eq!(parse_race("Demon"), MobRace::Demon);
        assert_eq!(parse_race("DemiHuman"), MobRace::DemiHuman);
        assert_eq!(parse_race("Demi_Human"), MobRace::DemiHuman);
        assert_eq!(parse_race("Angel"), MobRace::Angel);
        assert_eq!(parse_race("Dragon"), MobRace::Dragon);
        assert_eq!(parse_race("unknown"), MobRace::Formless);
    }

    #[test]
    fn test_parse_behavior() {
        assert_eq!(parse_behavior("01", &None), MobBehavior::Aggressive);
        assert_eq!(parse_behavior("03", &None), MobBehavior::Passive);
        assert_eq!(parse_behavior("04", &None), MobBehavior::Assist);
        assert_eq!(parse_behavior("05", &None), MobBehavior::PassiveAssist);
        assert_eq!(parse_behavior("06", &None), MobBehavior::Passive);
    }

    #[test]
    fn test_parse_modes_default() {
        let flags = parse_modes(&None);
        assert!(flags.can_move);
        assert!(flags.can_attack);
        assert!(!flags.detector);
        assert!(!flags.boss);
        assert!(!flags.plant);
        assert!(flags.can_chase);
    }

    #[test]
    fn test_parse_modes_boss_detector() {
        let mut modes = HashMap::new();
        modes.insert("Boss".to_string(), true);
        modes.insert("Detector".to_string(), true);
        modes.insert("CanMove".to_string(), false);
        modes.insert("CanChase".to_string(), false);
        let flags = parse_modes(&Some(modes));
        assert!(!flags.can_move);
        assert!(flags.can_attack);
        assert!(flags.detector);
        assert!(flags.boss);
        assert!(!flags.plant);
        assert!(!flags.can_chase);
    }

    #[test]
    fn test_parse_behavior_immobile() {
        let mut modes = HashMap::new();
        modes.insert("CanMove".to_string(), false);
        assert_eq!(parse_behavior("06", &Some(modes)), MobBehavior::Immobile);
    }

    #[test]
    fn test_parse_element_level() {
        assert_eq!(parse_element_level(1), ElementLevel::Level1);
        assert_eq!(parse_element_level(2), ElementLevel::Level2);
        assert_eq!(parse_element_level(3), ElementLevel::Level3);
        assert_eq!(parse_element_level(4), ElementLevel::Level4);
        assert_eq!(parse_element_level(5), ElementLevel::Level1); // 超范围回退
    }

    #[test]
    fn test_load_mob_db_from_string() {
        let yaml_str = r#"
Header:
  Type: MOB_DB
  Version: 5
Body:
  - Id: 1001
    AegisName: SCORPION
    Name: Scorpion
    Level: 16
    Hp: 136
    BaseExp: 169
    JobExp: 115
    Attack: 7
    Attack2: 7
    Defense: 16
    MagicDefense: 5
    Str: 12
    Agi: 15
    Vit: 10
    Int: 5
    Dex: 19
    Luk: 5
    AttackRange: 1
    Size: Small
    Race: Insect
    Element: Fire
    ElementLevel: 1
    WalkSpeed: 200
    Ai: "01"
    Drops:
      - Item: Boody_Red
        Rate: 35
      - Item: Scorpion_Tail
        Rate: 2750
  - Id: 1002
    AegisName: PORING
    Name: Poring
    Level: 1
    Hp: 55
    BaseExp: 150
    JobExp: 40
    Attack: 1
    Attack2: 1
    Defense: 2
    MagicDefense: 5
    Size: Medium
    Race: Plant
    Element: Water
    ElementLevel: 1
    WalkSpeed: 400
    Ai: "06"
"#;

        let yaml: MobYamlFile = serde_yaml::from_str(yaml_str).unwrap();
        let body = yaml.Body.unwrap();
        assert_eq!(body.len(), 2);

        // 验证第一个条目
        assert_eq!(body[0].Id, 1001);
        assert_eq!(body[0].Name, "Scorpion");
        assert_eq!(body[0].Hp, 136);
        assert_eq!(body[0].Level, 16);
        assert_eq!(body[0].Attack, 7);
        assert_eq!(body[0].Defense, 16);
        assert_eq!(body[0].Element, "Fire");
        assert_eq!(body[0].ElementLevel, 1);
        assert!(body[0].Drops.is_some());
        assert_eq!(body[0].Drops.as_ref().unwrap().len(), 2);

        // 验证第二个条目
        assert_eq!(body[1].Id, 1002);
        assert_eq!(body[1].Name, "Poring");
        assert_eq!(body[1].Hp, 55);
    }

    #[test]
    fn test_load_mob_db_full_pipeline() {
        let yaml_str = r#"
Header:
  Type: MOB_DB
  Version: 5
Body:
  - Id: 1001
    AegisName: SCORPION
    Name: Scorpion
    Level: 16
    Hp: 136
    BaseExp: 169
    JobExp: 115
    Attack: 7
    Attack2: 7
    Defense: 16
    MagicDefense: 5
    Dex: 19
    Agi: 15
    Luk: 5
    AttackRange: 1
    Size: Small
    Element: Fire
    ElementLevel: 1
    WalkSpeed: 200
    Ai: "01"
    Drops:
      - Item: Boody_Red
        Rate: 35
"#;

        // 写入临时文件并加载
        let tmp_path = "/tmp/test_mob_db.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();

        let mobs = load_mob_db(tmp_path).unwrap();
        assert_eq!(mobs.len(), 1);

        let scorpion = mobs.get(&1001).unwrap();
        assert_eq!(scorpion.name, "Scorpion");
        assert_eq!(scorpion.level, 16);
        assert_eq!(scorpion.hp, 136);
        assert_eq!(scorpion.atk, 7);
        assert_eq!(scorpion.element, Element::Fire);
        assert_eq!(scorpion.element_level, ElementLevel::Level1);
        assert_eq!(scorpion.size, MobSize::Small);
        assert_eq!(scorpion.behavior, MobBehavior::Aggressive);
        assert_eq!(scorpion.drops.len(), 1);
        assert_eq!(scorpion.drops[0].chance, 35);

        // 清理
        std::fs::remove_file(tmp_path).ok();
    }

    #[test]
    fn test_parse_skill_target() {
        assert_eq!(parse_skill_target("target"), MobSkillTarget::Target);
        assert_eq!(parse_skill_target("self"), MobSkillTarget::Self_);
        assert_eq!(parse_skill_target("TARGET"), MobSkillTarget::Target);
        assert_eq!(parse_skill_target("SELF"), MobSkillTarget::Self_);
        assert_eq!(parse_skill_target("unknown"), MobSkillTarget::Target);
    }

    #[test]
    fn test_parse_skill_condition() {
        assert_eq!(parse_skill_condition("any"), MobSkillCondition::Any);
        assert_eq!(parse_skill_condition("hpcertain"), MobSkillCondition::HpCertain);
        assert_eq!(parse_skill_condition("rudeattacked"), MobSkillCondition::RudeAttacked);
        assert_eq!(parse_skill_condition("longrange"), MobSkillCondition::LongRange);
        assert_eq!(parse_skill_condition("ANY"), MobSkillCondition::Any);
        assert_eq!(parse_skill_condition("unknown"), MobSkillCondition::Any);
    }

    #[test]
    fn test_mob_skill_entry_defaults() {
        // 验证默认值
        assert_eq!(default_skill_level(), 1);
        assert_eq!(default_skill_target(), "target");
        assert_eq!(default_skill_condition(), "any");
    }

    #[test]
    fn test_load_mob_db_with_skills() {
        let yaml_str = r#"
Header:
  Type: MOB_DB
  Version: 5
Body:
  - Id: 1002
    AegisName: LUNATIC
    Name: Lunatic
    Level: 3
    Hp: 80
    BaseExp: 6
    JobExp: 4
    Attack: 12
    Defense: 0
    Agi: 10
    Dex: 12
    Luk: 5
    AttackRange: 1
    Size: Small
    Race: Brute
    Element: Neutral
    ElementLevel: 1
    WalkSpeed: 200
    Ai: "03"
    Skills:
      - Id: AL_HEAL
        Lv: 3
        Rate: 1000
        Delay: 10000
        Target: self
        Condition: hpcertain
        ConditionValue: 50
      - Id: SM_BASH
        Lv: 5
        Rate: 500
        Delay: 5000
        Target: target
        Condition: any
"#;

        let tmp_path = "/tmp/test_mob_db_skills.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();

        let mobs = load_mob_db(tmp_path).unwrap();
        assert_eq!(mobs.len(), 1);

        let lunatic = mobs.get(&1002).unwrap();
        assert_eq!(lunatic.name, "Lunatic");
        assert_eq!(lunatic.skills.len(), 2);

        // 验证第一个技能（Heal）
        let heal_skill = &lunatic.skills[0];
        // AL_HEAL 的 ID 从 skill_db.yml 查找（如果加载成功），否则为 0
        // 由于测试环境可能没有 skill_db.yml，这里主要验证解析逻辑
        assert_eq!(heal_skill.level, 3);
        assert_eq!(heal_skill.chance, 1000);
        assert_eq!(heal_skill.target, MobSkillTarget::Self_);
        assert_eq!(heal_skill.condition, MobSkillCondition::HpCertain);
        assert_eq!(heal_skill.condition_value, 50);
        assert_eq!(heal_skill.cooldown_ms, 10000);

        // 验证第二个技能（Bash）
        let bash_skill = &lunatic.skills[1];
        assert_eq!(bash_skill.level, 5);
        assert_eq!(bash_skill.chance, 500);
        assert_eq!(bash_skill.target, MobSkillTarget::Target);
        assert_eq!(bash_skill.condition, MobSkillCondition::Any);
        assert_eq!(bash_skill.condition_value, 0);
        assert_eq!(bash_skill.cooldown_ms, 5000);

        // 清理
        std::fs::remove_file(tmp_path).ok();
    }

    #[test]
    fn test_load_mob_db_without_skills() {
        // 没有 Skills 字段的条目应该正常加载，skills 为空
        let yaml_str = r#"
Header:
  Type: MOB_DB
  Version: 5
Body:
  - Id: 1001
    AegisName: PORING
    Name: Poring
    Level: 1
    Hp: 50
    Attack: 7
    Defense: 0
    WalkSpeed: 400
    Ai: "06"
"#;

        let tmp_path = "/tmp/test_mob_db_no_skills.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();

        let mobs = load_mob_db(tmp_path).unwrap();
        let poring = mobs.get(&1001).unwrap();
        assert!(poring.skills.is_empty());

        std::fs::remove_file(tmp_path).ok();
    }
}
