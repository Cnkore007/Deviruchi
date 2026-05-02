use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::error::Error;
use super::data::{Item, ItemType};

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
