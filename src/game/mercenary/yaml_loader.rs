//! rAthena mercenary_db.yml 格式加载器

use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs;

/// 佣兵模板（从 YAML 加载的静态数据）
#[derive(Debug, Clone)]
pub struct MercenaryTemplateYaml {
    pub id: u16,
    pub name: String,
    pub level: u16,
    pub hp: u32,
    pub sp: u32,
    pub attack: u32,
    pub attack2: u32,
    pub defense: u32,
    pub magic_defense: u32,
    pub str: u16,
    pub agi: u16,
    pub vit: u16,
    pub int: u16,
    pub dex: u16,
    pub luk: u16,
    pub attack_range: u16,
    pub walk_speed: u16,
    pub skills: Vec<MercenarySkillEntry>,
}

#[derive(Debug, Clone)]
pub struct MercenarySkillEntry {
    pub name: String,
    pub max_level: u8,
}

#[derive(Deserialize, Debug)]
struct MercDbFile {
    Header: MercDbHeader,
    Body: Option<Vec<MercDbEntry>>,
}

#[derive(Deserialize, Debug)]
struct MercDbHeader {
    #[serde(rename = "Type")]
    #[allow(dead_code)]
    _type: String,
    #[allow(dead_code)]
    Version: u32,
}

#[derive(Deserialize, Debug)]
struct MercDbEntry {
    Id: u16,
    #[serde(rename = "AegisName")]
    #[allow(dead_code)]
    aegis_name: Option<String>,
    Name: Option<String>,
    #[serde(default)]
    Level: u16,
    #[serde(default)]
    Hp: u32,
    #[serde(default)]
    Sp: u32,
    #[serde(default)]
    Attack: u32,
    #[serde(rename = "Attack2", default)]
    Attack2: u32,
    #[serde(default)]
    Defense: u32,
    #[serde(rename = "MagicDefense", default)]
    MagicDefense: u32,
    #[serde(default = "default_one_u16")]
    Str: u16,
    #[serde(default = "default_one_u16")]
    Agi: u16,
    #[serde(default = "default_one_u16")]
    Vit: u16,
    #[serde(default = "default_one_u16")]
    Int: u16,
    #[serde(default = "default_one_u16")]
    Dex: u16,
    #[serde(default = "default_one_u16")]
    Luk: u16,
    #[serde(rename = "AttackRange", default)]
    AttackRange: u16,
    #[serde(rename = "WalkSpeed", default = "default_walk_speed")]
    WalkSpeed: u16,
    #[serde(default)]
    Skills: Vec<MercSkillYaml>,
}

fn default_one_u16() -> u16 { 1 }
fn default_walk_speed() -> u16 { 200 }

#[derive(Deserialize, Debug)]
struct MercSkillYaml {
    Name: String,
    #[serde(rename = "MaxLevel", default = "default_one_u8")]
    MaxLevel: u8,
}

fn default_one_u8() -> u8 { 1 }

impl MercDbEntry {
    fn to_template(&self) -> MercenaryTemplateYaml {
        MercenaryTemplateYaml {
            id: self.Id,
            name: self.Name.clone().unwrap_or_else(|| format!("Mercenary_{}", self.Id)),
            level: if self.Level == 0 { 1 } else { self.Level },
            hp: if self.Hp == 0 { 1 } else { self.Hp },
            sp: if self.Sp == 0 { 1 } else { self.Sp },
            attack: self.Attack,
            attack2: self.Attack2,
            defense: self.Defense,
            magic_defense: self.MagicDefense,
            str: self.Str,
            agi: self.Agi,
            vit: self.Vit,
            int: self.Int,
            dex: self.Dex,
            luk: self.Luk,
            attack_range: self.AttackRange,
            walk_speed: self.WalkSpeed,
            skills: self.Skills.iter().map(|s| MercenarySkillEntry {
                name: s.Name.clone(),
                max_level: s.MaxLevel,
            }).collect(),
        }
    }
}

/// 从 rAthena mercenary_db.yml 加载佣兵模板
pub fn load_mercenary_db(path: &str) -> Result<HashMap<u16, MercenaryTemplateYaml>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let yaml: MercDbFile = serde_yaml::from_str(&content)?;
    let mut db = HashMap::new();
    if let Some(body) = yaml.Body {
        for entry in body {
            let template = entry.to_template();
            db.insert(entry.Id, template);
        }
    }
    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_mercenary_db_from_string() {
        let yaml_str = r#"
Header:
  Type: MERCENARY_DB
  Version: 1
Body:
  - Id: 2213
    AegisName: M_WANDER_MAN
    Name: Wander Man
    Level: 81
    Hp: 8614
    Sp: 220
    Attack: 1100
    Attack2: 1300
    Defense: 60
    MagicDefense: 20
    Str: 80
    Agi: 110
    Vit: 63
    Int: 51
    Dex: 85
    Luk: 90
    AttackRange: 2
    Skills:
      - Name: MER_CRASH
        MaxLevel: 5
      - Name: MER_MAGNIFICAT
        MaxLevel: 3
  - Id: 2214
    Name: Bow Guardian
    Level: 50
    Hp: 5000
    Attack: 500
"#;

        let tmp_path = "/tmp/test_merc_db.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();
        let mercs = load_mercenary_db(tmp_path).unwrap();
        assert_eq!(mercs.len(), 2);

        let wander = mercs.get(&2213).unwrap();
        assert_eq!(wander.name, "Wander Man");
        assert_eq!(wander.level, 81);
        assert_eq!(wander.hp, 8614);
        assert_eq!(wander.str, 80);
        assert_eq!(wander.skills.len(), 2);
        assert_eq!(wander.skills[0].name, "MER_CRASH");
        assert_eq!(wander.skills[0].max_level, 5);

        let bow = mercs.get(&2214).unwrap();
        assert_eq!(bow.name, "Bow Guardian");
        assert_eq!(bow.str, 1); // 默认值

        std::fs::remove_file(tmp_path).ok();
    }
}
