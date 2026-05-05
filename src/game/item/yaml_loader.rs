use super::data::{Item, ItemType};
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs;

#[derive(Deserialize, Debug)]
struct ItemYaml {
    #[serde(rename = "Id")]
    id: u16,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Type")]
    type_: String,
    #[serde(rename = "BuyPrice")]
    buy_price: u32,
    #[serde(rename = "SellPrice")]
    sell_price: u32,
    #[serde(rename = "Weight")]
    weight: u16,
    #[serde(rename = "HpRestore", default)]
    hp_restore: u16,
    #[serde(rename = "SpRestore", default)]
    sp_restore: u16,
    #[serde(rename = "Atk", default)]
    atk: u16,
    #[serde(rename = "EquipMask", default)]
    equip_mask: u32,
    #[serde(rename = "Defense", default)]
    defense: u16,
}

impl ItemYaml {
    fn to_item(&self) -> Item {
        Item {
            id: self.id,
            name: Box::leak(self.name.clone().into_boxed_str()),
            type_: match self.type_.as_str() {
                "Heal" => ItemType::Heal,
                "Weapon" => ItemType::Weapon,
                "Armor" => ItemType::Armor,
                "Card" => ItemType::Card,
                "PetEgg" => ItemType::PetEgg,
                "PetArmor" => ItemType::PetArmor,
                _ => ItemType::Etc,
            },
            buy_price: self.buy_price,
            sell_price: self.sell_price,
            weight: self.weight,
            flags: 0,
            hp_restore: self.hp_restore,
            sp_restore: self.sp_restore,
            equip_mask: self.equip_mask,
            atk: self.atk,
            matk: 0,
            defense: self.defense,
            magic_defense: 0,
            str_bonus: 0,
            agi_bonus: 0,
            vit_bonus: 0,
            int_bonus: 0,
            dex_bonus: 0,
            luk_bonus: 0,
        }
    }
}

pub struct ItemDbLoader;

impl ItemDbLoader {
    pub fn load_from_yaml(path: &str) -> Result<HashMap<u16, Item>, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let yaml_items: Vec<ItemYaml> = serde_yaml::from_str(&content)?;

        let mut db = HashMap::new();
        for y in yaml_items {
            db.insert(y.id, y.to_item());
        }

        Ok(db)
    }
}

// ============================================================
// rAthena item_db.yml 格式加载器（AegisName -> ID 映射）
// ============================================================

/// rAthena item_db.yml 文件结构
#[derive(Deserialize, Debug)]
struct ItemRathenaYamlFile {
    Header: ItemRathenaYamlHeader,
    Body: Option<Vec<ItemRathenaYamlEntry>>,
}

#[derive(Deserialize, Debug)]
struct ItemRathenaYamlHeader {
    #[serde(rename = "Type")]
    #[allow(dead_code)]
    _type: String,
    #[allow(dead_code)]
    Version: u32,
}

/// rAthena item_db.yml 中的物品条目（仅解析 Id 和 AegisName 用于名称映射）
#[derive(Deserialize, Debug)]
struct ItemRathenaYamlEntry {
    Id: u32,
    #[serde(rename = "AegisName")]
    AegisName: String,
    #[allow(dead_code)]
    Name: String,
}

/// 从 rAthena item_db.yml 加载物品名称到 ID 的映射
///
/// 返回 (AegisName -> Id) 映射，用于掉落表中的物品名称解析
pub fn load_item_db(path: &str) -> Result<HashMap<String, u32>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let yaml: ItemRathenaYamlFile = serde_yaml::from_str(&content)?;
    let mut map = HashMap::new();
    if let Some(body) = yaml.Body {
        for entry in body {
            map.insert(entry.AegisName, entry.Id);
        }
    }
    Ok(map)
}

/// 混合查找：先查动态映射，再查硬编码回退
///
/// 优先从 item_db.yml 加载的动态映射中查找，未找到则回退到
/// `super::super::mob::yaml_loader::item_name_to_id` 的硬编码映射
pub fn item_name_to_id_dynamic(name: &str, item_map: &HashMap<String, u32>) -> u32 {
    item_map
        .get(name)
        .copied()
        .unwrap_or_else(|| crate::game::mob::yaml_loader::item_name_to_id(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_rathena_item_db_from_string() {
        let yaml_str = r#"
Header:
  Type: ITEM_DB
  Version: 5
Body:
  - Id: 501
    AegisName: Red_Potion
    Name: Red Potion
  - Id: 502
    AegisName: Orange_Potion
    Name: Orange Potion
  - Id: 909
    AegisName: Jellopy
    Name: Jellopy
"#;

        let tmp_path = "/tmp/test_rathena_item_db.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();
        let map = load_item_db(tmp_path).unwrap();
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("Red_Potion"), Some(&501));
        assert_eq!(map.get("Jellopy"), Some(&909));
        std::fs::remove_file(tmp_path).ok();
    }

    #[test]
    fn test_item_name_to_id_dynamic() {
        let mut map = HashMap::new();
        map.insert("Custom_Item".to_string(), 9999u32);

        // 动态映射优先
        assert_eq!(item_name_to_id_dynamic("Custom_Item", &map), 9999);
        // 硬编码回退
        assert_eq!(item_name_to_id_dynamic("Red_Potion", &map), 501);
        assert_eq!(item_name_to_id_dynamic("Jellopy", &map), 909);
    }
}
