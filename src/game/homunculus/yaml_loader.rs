#![allow(dead_code)]
#![allow(non_snake_case)]

//! rAthena homunculus_db.yml 格式加载器

use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs;

/// 生命体模板（从 YAML 加载的静态数据）
#[derive(Debug, Clone)]
pub struct HomunculusTemplateYaml {
    pub class_name: String,
    pub name: String,
    pub evolution_class: String,
    pub race: String,
    pub element: String,
    pub size: String,
    pub attack_delay: u32,
    pub hp_base: u32,
    pub hp_growth_min: u32,
    pub hp_growth_max: u32,
    pub sp_base: u32,
    pub sp_growth_min: u32,
    pub sp_growth_max: u32,
    pub str_base: u16,
    pub agi_base: u16,
    pub vit_base: u16,
    pub int_base: u16,
    pub dex_base: u16,
    pub luk_base: u16,
}

#[derive(Deserialize, Debug)]
struct HomunDbFile {
    Header: HomunDbHeader,
    Body: Option<Vec<HomunDbEntry>>,
}

#[derive(Deserialize, Debug)]
struct HomunDbHeader {
    #[serde(rename = "Type")]
    #[allow(dead_code)]
    _type: String,
    #[allow(dead_code)]
    Version: u32,
}

#[derive(Deserialize, Debug)]
struct HomunDbEntry {
    Class: String,
    Name: Option<String>,
    #[serde(rename = "EvolutionClass")]
    EvolutionClass: Option<String>,
    Race: Option<String>,
    Element: Option<String>,
    Size: Option<String>,
    #[serde(rename = "AttackDelay", default = "default_attack_delay")]
    AttackDelay: u32,
    #[serde(default)]
    Status: Vec<HomunStatusEntry>,
}

fn default_attack_delay() -> u32 { 700 }

#[derive(Deserialize, Debug)]
struct HomunStatusEntry {
    Type: String,
    #[serde(default)]
    Base: u32,
    #[serde(default)]
    GrowthMinimum: u32,
    #[serde(default)]
    GrowthMaximum: u32,
}

impl HomunDbEntry {
    fn to_template(&self) -> HomunculusTemplateYaml {
        let get_stat = |stat_type: &str| -> u32 {
            self.Status.iter()
                .find(|s| s.Type.eq_ignore_ascii_case(stat_type))
                .map(|s| s.Base)
                .unwrap_or(1)
        };

        HomunculusTemplateYaml {
            class_name: self.Class.clone(),
            name: self.Name.clone().unwrap_or_else(|| self.Class.clone()),
            evolution_class: self.EvolutionClass.clone().unwrap_or_default(),
            race: self.Race.clone().unwrap_or_else(|| "Demihuman".to_string()),
            element: self.Element.clone().unwrap_or_else(|| "Neutral".to_string()),
            size: self.Size.clone().unwrap_or_else(|| "Small".to_string()),
            attack_delay: self.AttackDelay,
            hp_base: get_stat("Hp"),
            hp_growth_min: self.Status.iter().find(|s| s.Type == "Hp").map(|s| s.GrowthMinimum).unwrap_or(0),
            hp_growth_max: self.Status.iter().find(|s| s.Type == "Hp").map(|s| s.GrowthMaximum).unwrap_or(0),
            sp_base: get_stat("Sp"),
            sp_growth_min: self.Status.iter().find(|s| s.Type == "Sp").map(|s| s.GrowthMinimum).unwrap_or(0),
            sp_growth_max: self.Status.iter().find(|s| s.Type == "Sp").map(|s| s.GrowthMaximum).unwrap_or(0),
            str_base: get_stat("Str") as u16,
            agi_base: get_stat("Agi") as u16,
            vit_base: get_stat("Vit") as u16,
            int_base: get_stat("Int") as u16,
            dex_base: get_stat("Dex") as u16,
            luk_base: get_stat("Luk") as u16,
        }
    }
}

/// 从 rAthena homunculus_db.yml 加载生命体模板
pub fn load_homunculus_db(path: &str) -> Result<HashMap<String, HomunculusTemplateYaml>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let yaml: HomunDbFile = serde_yaml::from_str(&content)?;
    let mut db = HashMap::new();
    if let Some(body) = yaml.Body {
        for entry in body {
            let template = entry.to_template();
            db.insert(entry.Class.clone(), template);
        }
    }
    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_homunculus_db_from_string() {
        let yaml_str = r#"
Header:
  Type: HOMUNCULUS_DB
  Version: 1
Body:
  - Class: Lif
    Name: Lif
    EvolutionClass: Lif_H
    Race: Demihuman
    Element: Neutral
    Size: Small
    AttackDelay: 800
    Status:
      - Type: Hp
        Base: 150
        GrowthMinimum: 60
        GrowthMaximum: 100
      - Type: Sp
        Base: 40
        GrowthMinimum: 4
        GrowthMaximum: 9
      - Type: Str
        Base: 17
      - Type: Agi
        Base: 15
      - Type: Vit
        Base: 12
      - Type: Int
        Base: 10
      - Type: Dex
        Base: 14
      - Type: Luk
        Base: 11
  - Class: Amistr
    Name: Amistr
    EvolutionClass: Amistr_H
"#;

        let tmp_path = "/tmp/test_homun_db.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();
        let homuns = load_homunculus_db(tmp_path).unwrap();
        assert_eq!(homuns.len(), 2);

        let lif = homuns.get("Lif").unwrap();
        assert_eq!(lif.name, "Lif");
        assert_eq!(lif.evolution_class, "Lif_H");
        assert_eq!(lif.hp_base, 150);
        assert_eq!(lif.str_base, 17);
        assert_eq!(lif.attack_delay, 800);

        let amistr = homuns.get("Amistr").unwrap();
        assert_eq!(amistr.name, "Amistr");
        assert_eq!(amistr.hp_base, 1); // 没有 Status 时默认值

        std::fs::remove_file(tmp_path).ok();
    }
}
