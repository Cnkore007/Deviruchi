#![allow(non_snake_case)]

use super::data::{Item, ItemType};
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs;

// ============================================================
// rAthena item_db 格式加载器
// ============================================================

/// rAthena item_db 文件结构（适用于 item_db_equip.yml / item_db_usable.yml / item_db_etc.yml）
#[derive(Deserialize, Debug)]
struct ItemRathenaFile {
    #[allow(dead_code)] // rAthena YAML compat
    Header: ItemRathenaHeader,
    Body: Option<Vec<ItemRathenaEntry>>,
}

#[derive(Deserialize, Debug)]
struct ItemRathenaHeader {
    #[serde(rename = "Type")]
    #[allow(dead_code)] // rAthena YAML compat
    _type: String,
    #[allow(dead_code)] // rAthena YAML compat
    Version: u32,
}

/// rAthena item_db 中的物品条目
#[derive(Deserialize, Debug)]
struct ItemRathenaEntry {
    Id: u16,
    #[serde(rename = "AegisName")]
    aegis_name: String,
    Name: String,
    #[serde(rename = "Type")]
    type_: Option<String>,
    #[serde(rename = "SubType")]
    sub_type: Option<String>,
    #[serde(rename = "Buy", default)]
    buy: u32,
    #[serde(rename = "Sell", default)]
    sell: u32,
    #[serde(rename = "Weight", default)]
    weight: u16,
    #[serde(rename = "Attack", default)]
    attack: u16,
    #[serde(rename = "MagicAttack", default)]
    magic_attack: u16,
    #[serde(rename = "Defense", default)]
    defense: u16,
    #[serde(rename = "Range", default)]
    #[allow(dead_code)] // rAthena YAML compat
    range: u16,
    #[serde(rename = "Slots", default)]
    #[allow(dead_code)] // rAthena YAML compat
    slots: u16,
    #[serde(rename = "Locations", default)]
    locations: Option<HashMap<String, bool>>,
    #[serde(rename = "WeaponLevel", default)]
    #[allow(dead_code)] // rAthena YAML compat
    weapon_level: u8,
    #[serde(rename = "ArmorLevel", default)]
    #[allow(dead_code)] // rAthena YAML compat
    armor_level: u8,
    #[serde(rename = "EquipLevelMin", default)]
    #[allow(dead_code)] // rAthena YAML compat
    equip_level_min: u8,
    #[serde(rename = "Refineable", default)]
    #[allow(dead_code)] // rAthena YAML compat
    refineable: bool,
}

/// 将 rAthena Type 字段映射到 Deviruchi ItemType
fn map_item_type(type_str: &Option<String>, _sub_type: &Option<String>) -> ItemType {
    match type_str.as_deref().unwrap_or("Etc") {
        "Healing" => ItemType::Heal,
        "Weapon" => ItemType::Weapon,
        "Armor" => ItemType::Armor,
        "Card" => ItemType::Card,
        "PetEgg" => ItemType::PetEgg,
        "PetArmor" => ItemType::PetArmor,
        _ => ItemType::Etc,
    }
}

/// 将 rAthena Locations 映射到装备位置掩码
///
/// rAthena 位置名称 → Deviruchi 位掩码：
///   Head_Top    = 0x0100
///   Head_Mid    = 0x0200
///   Head_Low    = 0x0400
///   Armor       = 0x0008 (身体)
///   Right_Hand  = 0x0001 (右手)
///   Left_Hand   = 0x0002 (左手)
///   Shoes       = 0x0040
///   Garment     = 0x0010
///   Right_Accessory = 0x0004
///   Left_Accessory  = 0x0080
fn map_locations(locations: &Option<HashMap<String, bool>>) -> u32 {
    let Some(locs) = locations else { return 0 };
    let mut mask: u32 = 0;
    for (name, enabled) in locs {
        if !enabled { continue; }
        match name.as_str() {
            "Head_Top" => mask |= 0x0100,
            "Head_Mid" => mask |= 0x0200,
            "Head_Low" => mask |= 0x0400,
            "Armor" => mask |= 0x0008,
            "Right_Hand" => mask |= 0x0001,
            "Left_Hand" => mask |= 0x0002,
            "Shoes" => mask |= 0x0040,
            "Garment" => mask |= 0x0010,
            "Right_Accessory" => mask |= 0x0004,
            "Left_Accessory" => mask |= 0x0080,
            "Both_Hand" => mask |= 0x0003,
            _ => {}
        }
    }
    mask
}

impl ItemRathenaEntry {
    fn to_item(&self) -> Item {
        let type_ = map_item_type(&self.type_, &self.sub_type);
        let equip_mask = map_locations(&self.locations);

        // 买卖价格：rAthena 中 Sell 默认为 Buy/2
        let buy_price = if self.buy > 0 { self.buy } else { self.sell * 2 };
        let sell_price = if self.sell > 0 { self.sell } else { self.buy / 2 };

        Item {
            id: self.Id,
            name: self.Name.clone(),
            type_,
            buy_price,
            sell_price,
            weight: self.weight,
            flags: 0,
            hp_restore: 0,
            sp_restore: 0,
            equip_mask,
            atk: self.attack,
            matk: self.magic_attack,
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

/// 从单个 rAthena item_db 文件加载物品
fn load_item_file(path: &str) -> Result<HashMap<u16, Item>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let yaml: ItemRathenaFile = serde_yaml::from_str(&content)?;
    let mut db = HashMap::new();
    if let Some(body) = yaml.Body {
        for entry in body {
            db.insert(entry.Id, entry.to_item());
        }
    }
    Ok(db)
}

/// 从 rAthena item_db 文件加载 AegisName -> ID 映射
///
/// 用于将 rAthena YAML 中的物品名称（如 "Red_Potion"）转换为数字 ID。
/// 依次加载 equip/usable/etc 三个文件，构建完整映射。
pub fn load_item_name_to_id_map() -> HashMap<String, u16> {
    let rathena_paths = [
        "db/item_db_equip.yml",
        "db/item_db_usable.yml",
        "db/item_db_etc.yml",
    ];

    let mut name_to_id = HashMap::new();

    for path in &rathena_paths {
        if !std::path::Path::new(path).exists() {
            continue;
        }
        match fs::read_to_string(path) {
            Ok(content) => {
                match serde_yaml::from_str::<ItemRathenaFile>(&content) {
                    Ok(yaml) => {
                        if let Some(body) = yaml.Body {
                            for entry in body {
                                name_to_id.insert(entry.aegis_name, entry.Id);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("解析 {} 构建名称映射失败: {}", path, e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("读取 {} 构建名称映射失败: {}", path, e);
            }
        }
    }

    tracing::info!("构建物品名称映射: {} 个条目", name_to_id.len());
    name_to_id
}

/// 物品数据库加载器
///
/// 按优先级尝试加载以下文件：
/// 1. `db/item_db_equip.yml` + `db/item_db_usable.yml` + `db/item_db_etc.yml` (rAthena 格式)
/// 2. `db/item_db.yml` (旧自定义格式，兼容)
pub struct ItemDbLoader;

impl ItemDbLoader {
    pub fn load_from_yaml(path: &str) -> Result<HashMap<u16, Item>, Box<dyn Error>> {
        // 尝试加载 rAthena 格式的三个文件
        let rathena_paths = [
            "db/item_db_equip.yml",
            "db/item_db_usable.yml",
            "db/item_db_etc.yml",
        ];

        let mut all_items = HashMap::new();
        let mut loaded_any = false;

        for rpath in &rathena_paths {
            if std::path::Path::new(rpath).exists() {
                match load_item_file(rpath) {
                    Ok(items) => {
                        let count = items.len();
                        all_items.extend(items);
                        tracing::info!("从 {} 加载了 {} 个物品", rpath, count);
                        loaded_any = true;
                    }
                    Err(e) => {
                        tracing::warn!("加载 {} 失败: {}", rpath, e);
                    }
                }
            }
        }

        if loaded_any {
            return Ok(all_items);
        }

        // 回退到旧自定义格式
        load_legacy_format(path)
    }
}

/// 旧自定义格式加载器（兼容 db/item_db.yml）
#[derive(Deserialize, Debug)]
struct ItemLegacyYaml {
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

fn load_legacy_format(path: &str) -> Result<HashMap<u16, Item>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let yaml_items: Vec<ItemLegacyYaml> = serde_yaml::from_str(&content)?;
    let mut db = HashMap::new();
    for y in yaml_items {
        db.insert(y.id, Item {
            id: y.id,
            name: y.name,
            type_: match y.type_.as_str() {
                "Heal" => ItemType::Heal,
                "Weapon" => ItemType::Weapon,
                "Armor" => ItemType::Armor,
                "Card" => ItemType::Card,
                "PetEgg" => ItemType::PetEgg,
                "PetArmor" => ItemType::PetArmor,
                _ => ItemType::Etc,
            },
            buy_price: y.buy_price,
            sell_price: y.sell_price,
            weight: y.weight,
            flags: 0,
            hp_restore: y.hp_restore,
            sp_restore: y.sp_restore,
            equip_mask: y.equip_mask,
            atk: y.atk,
            matk: 0,
            defense: y.defense,
            magic_defense: 0,
            str_bonus: 0,
            agi_bonus: 0,
            vit_bonus: 0,
            int_bonus: 0,
            dex_bonus: 0,
            luk_bonus: 0,
        });
    }
    Ok(db)
}

// ============================================================
// rAthena item_db.yml AegisName → ID 映射（用于掉落表解析）
// ============================================================

#[derive(Deserialize, Debug)]
struct ItemNameMapFile {
    #[allow(dead_code)] // rAthena YAML compat
    Header: ItemNameMapHeader,
    Body: Option<Vec<ItemNameMapEntry>>,
}

#[derive(Deserialize, Debug)]
struct ItemNameMapHeader {
    #[serde(rename = "Type")]
    #[allow(dead_code)] // rAthena YAML compat
    _type: String,
    #[allow(dead_code)] // rAthena YAML compat
    Version: u32,
}

#[derive(Deserialize, Debug)]
struct ItemNameMapEntry {
    Id: u32,
    #[serde(rename = "AegisName")]
    AegisName: String,
}

/// 从 rAthena item_db 文件加载 AegisName → ID 映射
pub fn load_item_db(path: &str) -> Result<HashMap<String, u32>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let yaml: ItemNameMapFile = serde_yaml::from_str(&content)?;
    let mut map = HashMap::new();
    if let Some(body) = yaml.Body {
        for entry in body {
            map.insert(entry.AegisName, entry.Id);
        }
    }
    Ok(map)
}

/// 从多个 rAthena item_db 文件加载 AegisName → ID 映射
pub fn load_item_db_all() -> HashMap<String, u32> {
    let paths = [
        "db/item_db_equip.yml",
        "db/item_db_usable.yml",
        "db/item_db_etc.yml",
    ];
    let mut map = HashMap::new();
    for path in &paths {
        if std::path::Path::new(path).exists() {
            match load_item_db(path) {
                Ok(m) => map.extend(m),
                Err(e) => tracing::warn!("加载 {} 物品名称映射失败: {}", path, e),
            }
        }
    }
    map
}

/// 混合查找：先查动态映射，再查全局映射
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
    fn test_load_rathena_equip_from_string() {
        let yaml_str = r#"
Header:
  Type: ITEM_DB
  Version: 3
Body:
  - Id: 1101
    AegisName: Sword
    Name: Sword
    Type: Weapon
    SubType: 1hSword
    Buy: 100
    Weight: 500
    Attack: 25
    Range: 1
    Slots: 3
    Locations:
      Right_Hand: true
    WeaponLevel: 1
    Refineable: true
  - Id: 2301
    AegisName: Cotton_Shirt
    Name: Cotton Shirt
    Type: Armor
    Buy: 10
    Weight: 100
    Defense: 1
    Locations:
      Armor: true
"#;

        let tmp_path = "/tmp/test_rathena_equip.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();
        let items = load_item_file(tmp_path).unwrap();
        assert_eq!(items.len(), 2);

        let sword = items.get(&1101).unwrap();
        assert_eq!(sword.name, "Sword");
        assert_eq!(sword.type_, ItemType::Weapon);
        assert_eq!(sword.atk, 25);
        assert_eq!(sword.equip_mask, 0x0001); // Right_Hand

        let shirt = items.get(&2301).unwrap();
        assert_eq!(shirt.name, "Cotton Shirt");
        assert_eq!(shirt.type_, ItemType::Armor);
        assert_eq!(shirt.defense, 1);
        assert_eq!(shirt.equip_mask, 0x0008); // Armor

        std::fs::remove_file(tmp_path).ok();
    }

    #[test]
    fn test_load_rathena_usable_from_string() {
        let yaml_str = r#"
Header:
  Type: ITEM_DB
  Version: 3
Body:
  - Id: 501
    AegisName: Red_Potion
    Name: Red Potion
    Type: Healing
    Buy: 50
    Weight: 70
"#;

        let tmp_path = "/tmp/test_rathena_usable.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();
        let items = load_item_file(tmp_path).unwrap();
        assert_eq!(items.len(), 1);

        let potion = items.get(&501).unwrap();
        assert_eq!(potion.name, "Red Potion");
        assert_eq!(potion.type_, ItemType::Heal);
        assert_eq!(potion.buy_price, 50);

        std::fs::remove_file(tmp_path).ok();
    }

    #[test]
    fn test_map_locations() {
        let mut locs = HashMap::new();
        locs.insert("Right_Hand".to_string(), true);
        locs.insert("Left_Hand".to_string(), true);
        assert_eq!(map_locations(&Some(locs)), 0x0003); // Both_Hand

        let mut locs = HashMap::new();
        locs.insert("Armor".to_string(), true);
        assert_eq!(map_locations(&Some(locs)), 0x0008);

        assert_eq!(map_locations(&None), 0);
    }

    #[test]
    fn test_item_name_to_id_dynamic() {
        let mut map = HashMap::new();
        map.insert("Custom_Item".to_string(), 9999u32);

        // 动态映射优先
        assert_eq!(item_name_to_id_dynamic("Custom_Item", &map), 9999);
        // 未知物品返回 0（全局映射在测试环境中可能为空）
        let result = item_name_to_id_dynamic("Unknown_Item_XYZ", &map);
        assert_eq!(result, 0);
    }
}
