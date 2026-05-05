//! Mob YAML 数据加载器
//!
//! 从 rAthena mob_db.yml 格式加载怪物模板数据。
//! rAthena 格式: Header (Type + Version) -> Body (条目列表) -> Footer (Imports)

use super::data::{MobBehavior, MobDrop, MobRace, MobSkill, MobTemplate, MobType};
use crate::game::battle::element::{Element, ElementLevel, MobSize};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

/// 物品名称到 ID 的映射（rAthena 常用物品）
/// 完整映射应从 item_db.yml 动态加载，此处为最小集保证掉落表可用
fn item_name_to_id(name: &str) -> u32 {
    match name {
        // 消耗品
        "Red_Potion" | "Red_Potion_" => 501,
        "Orange_Potion" => 502,
        "Yellow_Potion" => 503,
        "White_Potion" => 504,
        "Blue_Potion" => 505,
        "Green_Potion" => 506,
        // 材料
        "Jellopy" => 909,
        "Fluff" => 914,
        "Feather" => 949,
        "Sticky_Mucus" => 938,
        "Scale_Shell" => 947,
        "Boody_Red" => 990,
        "Scorpion_Tail" => 904,
        "Shell" => 935,
        "Worm_Peelings" => 955,
        "Mushroom_Spore" => 921,
        "Tree_Root" => 902,
        "Resin" => 907,
        "Clover" => 705,
        "Four_Leaf_Clover" => 706,
        // 装备
        "Knife" => 1202,
        "Dagger" => 1201,
        "Main_Gauche" => 1207,
        "Sword" => 1101,
        "Falchion" => 1104,
        // 其他
        _ => {
            tracing::warn!("未知物品名称: {}，item_id 设为 0", name);
            0
        }
    }
}

/// rAthena mob_db.yml 文件结构
#[derive(Deserialize, Debug)]
struct MobYamlFile {
    Header: MobYamlHeader,
    Body: Option<Vec<MobYamlEntry>>,
    #[allow(dead_code)]
    Footer: Option<MobYamlFooter>,
}

#[derive(Deserialize, Debug)]
struct MobYamlHeader {
    #[serde(rename = "Type")]
    #[allow(dead_code)]
    _type: String,
    #[allow(dead_code)]
    Version: u32,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct MobYamlFooter {
    Imports: Option<Vec<MobYamlImport>>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct MobYamlImport {
    Path: String,
    Mode: Option<String>,
}

/// rAthena mob_db.yml 中的怪物条目
#[derive(Deserialize, Debug)]
struct MobYamlEntry {
    Id: u16,
    #[serde(rename = "AegisName")]
    #[allow(dead_code)]
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
    Str: u16,
    #[serde(default = "default_stat")]
    Agi: u16,
    #[serde(default = "default_stat")]
    Vit: u16,
    #[serde(default = "default_stat")]
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
    #[allow(dead_code)]
    AttackMotion: u32,
    #[serde(default)]
    #[allow(dead_code)]
    DamageMotion: u32,
    #[serde(default)]
    Ai: String,
    #[serde(default)]
    #[allow(dead_code)]
    Class: String,
    #[serde(default)]
    Modes: Option<HashMap<String, bool>>,
    #[serde(default)]
    Drops: Option<Vec<MobYamlDrop>>,
    #[serde(default)]
    #[allow(dead_code)]
    MvpDrops: Option<Vec<MobYamlDrop>>,
}

/// rAthena 掉落条目
#[derive(Deserialize, Debug)]
struct MobYamlDrop {
    Item: String,
    #[serde(default)]
    Rate: u32,
    #[serde(default)]
    #[allow(dead_code)]
    StealProtected: Option<bool>,
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
            if let Some(modes) = modes {
                if modes.get("CanMove").copied() == Some(false) {
                    return MobBehavior::Immobile;
                }
            }
            MobBehavior::Passive
        }
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
                sight_range: 12, // 默认视野范围
                chase_range: if entry.ChaseRange > 0 {
                    entry.ChaseRange
                } else {
                    12
                },
                aggro_rate: 0,
                spawn_delay: entry.AttackDelay,
                respawn_time: 60000, // 默认重生时间
                behavior: parse_behavior(&entry.Ai, &entry.Modes),
                skills: Vec::new(), // TODO: 从 Skills 字段加载
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
}
