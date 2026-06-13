#![allow(non_snake_case)]

//! rAthena pet_db.yml 格式加载器

use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs;

/// 宠物模板（从 YAML 加载的静态数据）
#[derive(Debug, Clone, Deserialize)]
pub struct PetTemplate {
    pub mob_name: String,
    pub tame_item: String,
    pub egg_item: String,
    pub equip_item: String,
    pub food_item: String,
    pub fullness: u32,
    pub intimacy_fed: u32,
    pub capture_rate: u32,
}

#[derive(Deserialize, Debug)]
struct PetDbFile {
    #[allow(dead_code)] // rAthena YAML compat
    Header: PetDbHeader,
    Body: Option<Vec<PetDbEntry>>,
}

#[derive(Deserialize, Debug)]
struct PetDbHeader {
    #[serde(rename = "Type")]
    #[allow(dead_code)] // rAthena YAML compat
    _type: String,
    #[allow(dead_code)] // rAthena YAML compat
    Version: u32,
}

#[derive(Deserialize, Debug)]
struct PetDbEntry {
    Mob: String,
    #[serde(rename = "TameItem")]
    TameItem: Option<String>,
    #[serde(rename = "EggItem")]
    EggItem: Option<String>,
    #[serde(rename = "EquipItem")]
    EquipItem: Option<String>,
    #[serde(rename = "FoodItem")]
    FoodItem: Option<String>,
    #[serde(rename = "Fullness", default = "default_fullness")]
    Fullness: u32,
    #[serde(rename = "IntimacyFed", default = "default_intimacy")]
    IntimacyFed: u32,
    #[serde(rename = "CaptureRate", default)]
    CaptureRate: u32,
}

fn default_fullness() -> u32 { 3 }
fn default_intimacy() -> u32 { 50 }

impl PetDbEntry {
    fn to_template(&self) -> PetTemplate {
        PetTemplate {
            mob_name: self.Mob.clone(),
            tame_item: self.TameItem.clone().unwrap_or_default(),
            egg_item: self.EggItem.clone().unwrap_or_default(),
            equip_item: self.EquipItem.clone().unwrap_or_default(),
            food_item: self.FoodItem.clone().unwrap_or_default(),
            fullness: self.Fullness,
            intimacy_fed: self.IntimacyFed,
            capture_rate: self.CaptureRate,
        }
    }
}

/// 从 rAthena pet_db.yml 加载宠物模板
pub fn load_pet_db(path: &str) -> Result<HashMap<String, PetTemplate>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let yaml: PetDbFile = serde_yaml::from_str(&content)?;
    let mut db = HashMap::new();
    if let Some(body) = yaml.Body {
        for entry in body {
            let template = entry.to_template();
            db.insert(entry.Mob.clone(), template);
        }
    }
    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_pet_db_from_string() {
        let yaml_str = r#"
Header:
  Type: PET_DB
  Version: 1
Body:
  - Mob: PORING
    TameItem: Unripe_Apple
    EggItem: Poring_Egg
    EquipItem: Backpack
    FoodItem: Apple_Juice
    Fullness: 3
    IntimacyFed: 50
    CaptureRate: 2000
  - Mob: DROPS
    TameItem: Orange_Juice
    EggItem: Drops_Egg
    FoodItem: Yellow_Herb
    Fullness: 4
    IntimacyFed: 40
    CaptureRate: 1500
"#;

        let tmp_path = "/tmp/test_pet_db.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();
        let pets = load_pet_db(tmp_path).unwrap();
        assert_eq!(pets.len(), 2);

        let poring = pets.get("PORING").unwrap();
        assert_eq!(poring.tame_item, "Unripe_Apple");
        assert_eq!(poring.egg_item, "Poring_Egg");
        assert_eq!(poring.capture_rate, 2000);

        let drops = pets.get("DROPS").unwrap();
        assert_eq!(drops.tame_item, "Orange_Juice");
        assert_eq!(drops.fullness, 4);

        std::fs::remove_file(tmp_path).ok();
    }
}
