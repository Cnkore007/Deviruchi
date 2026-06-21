//! 宠物数据模块

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 宠物实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pet {
    pub pet_id: u32,
    pub owner_id: u32,    // Character ID
    pub owner_uuid: Uuid, // Player session UUID
    pub monster_id: u16,  // Mob ID this pet is based on
    pub name: String,
    pub renamed: bool,
    pub intimacy: u32, // 0-100000, tameness/loyalty
    pub hunger: u32,   // 0-1000, hunger level
    pub level: u16,
    pub egg_id: u16,   // Egg item ID
    pub equip_id: u16, // Accessory item ID
    pub born_date: DateTime<Utc>,
}

impl Pet {
    pub fn new(
        pet_id: u32,
        owner_id: u32,
        owner_uuid: Uuid,
        monster_id: u16,
        name: String,
        egg_id: u16,
        equip_id: u16,
    ) -> Self {
        Self {
            pet_id,
            owner_id,
            owner_uuid,
            monster_id,
            name,
            renamed: false,
            intimacy: 10000, // Start with some initial intimacy
            hunger: 500,     // Start at 50% hunger
            level: 1,
            egg_id,
            equip_id,
            born_date: Utc::now(),
        }
    }

    /// 喂食宠物
    pub fn feed(&mut self, hunger_restore: u32) {
        self.hunger = self.hunger.saturating_add(hunger_restore).min(1000);
    }

    /// 消耗饥饿值
    pub fn decrease_hunger(&mut self, amount: u32) {
        self.hunger = self.hunger.saturating_sub(amount);
    }

    /// 增加亲密度
    pub fn increase_intimacy(&mut self, amount: u32) {
        self.intimacy = self.intimacy.saturating_add(amount).min(100000);
    }

    /// 降低亲密度
    pub fn decrease_intimacy(&mut self, amount: u32) {
        self.intimacy = self.intimacy.saturating_sub(amount);
    }

    /// 检查宠物是否饥饿（饥饿值低于20%）
    pub fn is_hungry(&self) -> bool {
        self.hunger < 200
    }

    /// 检查宠物是否非常饥饿（饥饿值低于10%）
    pub fn is_starving(&self) -> bool {
        self.hunger < 100
    }

    /// 获取亲密度等级（0-10）
    pub fn intimacy_level(&self) -> u8 {
        match self.intimacy {
            0..=999 => 0,       // About to run away
            1000..=9999 => 1,   // Shy
            10000..=29999 => 2, // Neutral
            30000..=49999 => 3, // Friendly
            50000..=69999 => 4, // Loyal
            70000..=84999 => 5, // Faithful
            85000..=94999 => 6, // Very loyal
            95000..=98999 => 7, // Devoted
            99000..=99999 => 8, // Extremely devoted
            _ => 9,             // Fully devoted (100000)
        }
    }

    /// 重命名宠物
    pub fn rename(&mut self, new_name: &str) {
        self.name = new_name.to_string();
        self.renamed = true;
    }

    /// 检查宠物是否会背叛（亲密度为0）
    pub fn will_abandon(&self) -> bool {
        self.intimacy == 0
    }
}

/// 宠物数据库数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetData {
    pub pet_class: u16,
    pub name: String,
    pub mob_id: u16,
    pub egg_id: u16,
    pub equip_id: u16,
    pub capture_rate: u16,      // 捕获概率（百分比）
    pub food: Vec<u16>,         // Item IDs that are pet food
    pub hungry_delay: u32,      // Seconds between hunger increase
    pub hunger_decrease: u32,   // Amount to decrease per interval
    pub intimacy_decrease: u32, // Intimacy decrease rate
    pub full_ratio: u32,        // At this hunger, intimacy decreases faster
    pub hungry_delay_min: u32,  // Minimum delay before hunger starts
    pub script: Option<String>, // Pet script (buffs, etc.)
    pub talk_convert: bool,     // Whether pet can talk
}

impl Default for PetData {
    fn default() -> Self {
        Self {
            pet_class: 0,
            name: String::new(),
            mob_id: 0,
            egg_id: 0,
            equip_id: 0,
            capture_rate: 0,
            food: Vec::new(),
            hungry_delay: 60,
            hunger_decrease: 10,
            intimacy_decrease: 1,
            full_ratio: 500,
            hungry_delay_min: 60,
            script: None,
            talk_convert: false,
        }
    }
}

/// 物品名称到蛋 ID 的映射
fn item_name_to_egg_id(name: &str) -> u16 {
    match name {
        "Poring_Egg" => 22001,
        "Lunatic_Egg" => 22002,
        "Fabre_Egg" => 22003,
        "Picky_Egg" => 22004,
        "PecoPeco_Egg" => 22005,
        "Petit_Egg" => 22006,
        "Deviruchi_Egg" => 22007,
        "Bapho_Jr_Egg" => 22008,
        "Desert_Wolf_B_Egg" => 22009,
        "Drops_Egg" => 22010,
        "Poporing_Egg" => 22011,
        "Rocker_Egg" => 22012,
        "Spore_Egg" => 22013,
        "Chonchon_Egg" => 22014,
        "Steel_Chonchon_Egg" => 22015,
        "Hunter_Fly_Egg" => 22016,
        "Orc_Warrior_Egg" => 22017,
        "Baphomet_Egg" => 22018,
        "Goblin_Egg" => 22019,
        _ => 0,
    }
}

/// 怪物名称到 ID 的映射
fn mob_name_to_id(name: &str) -> u16 {
    match name {
        "PORING" => 1002,
        "LUNATIC" => 1003,
        "FABRE" => 1007,
        "PICKY" => 1004,
        "PECO_EGG" | "PECOPECO" => 1019,
        "PETIT" => 1155,
        "DEVIRUCHI" => 1109,
        "BAPHOMET_JR" => 1299,
        "DESERT_WOLF_B" => 1106,
        "DROPS" => 1113,
        "POPORING" => 1031,
        "ROCKER" => 1052,
        "SPORE" => 1012,
        "CHONCHON" => 1010,
        "STEEL_CHONCHON" => 1209,
        "HUNTER_FLY" => 1035,
        "ORC_WARRIOR" => 1023,
        "BAPHOMET" => 1039,
        "GOBLIN" => 1122,
        _ => 0,
    }
}

/// 装备名称到 ID 的映射
fn item_name_to_equip_id(name: &str) -> u16 {
    match name {
        "Backpack" => 10000,
        "Boy's_Cap" => 10001,
        "Hair_Band" => 10002,
        "Ribbon" => 10003,
        "Bao" => 10004,
        "Model_Festival_Hat" => 10005,
        _ => 0,
    }
}

/// 食物名称到 ID 列表的映射
fn item_name_to_food_ids(name: &str) -> Vec<u16> {
    match name {
        "Apple_Juice" => vec![531],
        "Orange_Juice" => vec![532],
        "Yellow_Herb" => vec![503],
        "Red_Herb" => vec![507],
        "Green_Herb" => vec![510],
        "Meat" => vec![517],
        "Monster's_Feed" => vec![528],
        "Pet_Food" => vec![530],
        _ => Vec::new(),
    }
}

/// 宠物数据库
pub struct PetDatabase {
    pets: std::collections::HashMap<u16, PetData>,
}

impl PetDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            pets: std::collections::HashMap::new(),
        };

        // 尝试从 YAML 加载
        let yaml_paths = ["db/pet_db.yml"];
        for path in &yaml_paths {
            if std::path::Path::new(path).exists() {
                match super::yaml_loader::load_pet_db(path) {
                    Ok(templates) if !templates.is_empty() => {
                        let count = templates.len();
                        // 将 YAML 模板转换为 PetData 并加载
                        // rAthena pet_db 使用物品名称，需要转换为 ID
                        // 先加载硬编码数据作为基础映射
                        db.init_default_pets();
                        // 将 YAML 数据覆盖/补充硬编码数据
                        let mut yaml_loaded = 0;
                        for template in templates.values() {
                            // 尝试从 egg_item 名称推断 pet_class
                            // YAML 中的物品名称格式如 "Poring_Egg"，对应 egg_id 22001
                            let egg_id = item_name_to_egg_id(&template.egg_item);
                            let mob_id = mob_name_to_id(&template.mob_name);
                            if egg_id > 0 && mob_id > 0 {
                                let pet_class = egg_id; // 使用 egg_id 作为 pet_class
                                db.pets.insert(
                                    pet_class,
                                    PetData {
                                        pet_class,
                                        name: template.mob_name.clone(),
                                        mob_id,
                                        egg_id,
                                        equip_id: item_name_to_equip_id(&template.equip_item),
                                        capture_rate: template.capture_rate as u16,
                                        food: item_name_to_food_ids(&template.food_item),
                                        hungry_delay: 60,
                                        hunger_decrease: template.fullness,
                                        intimacy_decrease: 1,
                                        full_ratio: 500,
                                        hungry_delay_min: 60,
                                        script: None,
                                        talk_convert: false,
                                    },
                                );
                                yaml_loaded += 1;
                            }
                        }
                        tracing::info!(
                            "从 {} 加载了 {} 个宠物模板（{} 个已转换）",
                            path,
                            count,
                            yaml_loaded
                        );
                        return db;
                    }
                    Ok(_) => {
                        tracing::warn!("{} 解析结果为空", path);
                    }
                    Err(e) => {
                        tracing::warn!("加载 {} 失败: {}", path, e);
                    }
                }
            }
        }

        // 回退到硬编码
        tracing::info!("使用硬编码宠物数据");
        db.init_default_pets();
        db
    }

    fn init_default_pets(&mut self) {
        // Poring pet
        self.pets.insert(
            1042,
            PetData {
                pet_class: 1042,
                name: "Poring".to_string(),
                mob_id: 1001,
                egg_id: 22001,
                equip_id: 10000,
                capture_rate: 50,
                food: vec![530, 531, 532], // Various jellopies
                hungry_delay: 60,
                hunger_decrease: 5,
                intimacy_decrease: 1,
                full_ratio: 500,
                hungry_delay_min: 60,
                script: None,
                talk_convert: false,
            },
        );

        // Lunatic pet
        self.pets.insert(
            1043,
            PetData {
                pet_class: 1043,
                name: "Lunatic".to_string(),
                mob_id: 1002,
                egg_id: 22002,
                equip_id: 10001,
                capture_rate: 45,
                food: vec![505, 506, 507], // Various potions
                hungry_delay: 60,
                hunger_decrease: 6,
                intimacy_decrease: 1,
                full_ratio: 500,
                hungry_delay_min: 60,
                script: None,
                talk_convert: false,
            },
        );

        // Fabre pet
        self.pets.insert(
            1044,
            PetData {
                pet_class: 1044,
                name: "Fabre".to_string(),
                mob_id: 1312,
                egg_id: 22003,
                equip_id: 10002,
                capture_rate: 40,
                food: vec![914, 915, 916], // Fluffs
                hungry_delay: 60,
                hunger_decrease: 7,
                intimacy_decrease: 2,
                full_ratio: 500,
                hungry_delay_min: 60,
                script: None,
                talk_convert: false,
            },
        );

        // Peco Peco pet (for骑士)
        self.pets.insert(
            1045,
            PetData {
                pet_class: 1045,
                name: "Peco Peco".to_string(),
                mob_id: 1019,
                egg_id: 22004,
                equip_id: 10003,
                capture_rate: 35,
                food: vec![551, 552, 553], // Meats
                hungry_delay: 90,
                hunger_decrease: 8,
                intimacy_decrease: 2,
                full_ratio: 600,
                hungry_delay_min: 90,
                script: Some("bonus bSpeed,10;".to_string()),
                talk_convert: false,
            },
        );
    }

    pub fn get(&self, pet_class: u16) -> Option<&PetData> {
        self.pets.get(&pet_class)
    }

    pub fn get_by_mob_id(&self, mob_id: u16) -> Option<&PetData> {
        self.pets.values().find(|p| p.mob_id == mob_id)
    }

    pub fn get_by_egg_id(&self, egg_id: u16) -> Option<&PetData> {
        self.pets.values().find(|p| p.egg_id == egg_id)
    }

    pub fn get_by_equip_id(&self, equip_id: u16) -> Option<&PetData> {
        self.pets.values().find(|p| p.equip_id == equip_id)
    }

    pub fn is_pet_food(&self, item_id: u16) -> Option<&PetData> {
        self.pets.values().find(|p| p.food.contains(&item_id))
    }

    /// 仅使用硬编码数据创建（供测试使用）
    pub fn new_hardcoded() -> Self {
        let mut db = Self {
            pets: std::collections::HashMap::new(),
        };
        db.init_default_pets();
        db
    }
}

impl Default for PetDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uuid() -> Uuid {
        Uuid::new_v4()
    }

    #[test]
    fn test_pet_creation() {
        let pet = Pet::new(
            1,
            100,
            test_uuid(),
            1001,
            "My Poring".to_string(),
            22001,
            10000,
        );
        assert_eq!(pet.pet_id, 1);
        assert_eq!(pet.owner_id, 100);
        assert_eq!(pet.monster_id, 1001);
        assert_eq!(pet.name, "My Poring");
        assert_eq!(pet.intimacy, 10000);
        assert_eq!(pet.hunger, 500);
        assert_eq!(pet.level, 1);
    }

    #[test]
    fn test_pet_feed() {
        let mut pet = Pet::new(1, 100, test_uuid(), 1001, "Test".to_string(), 22001, 10000);
        pet.hunger = 200;
        pet.feed(100);
        assert_eq!(pet.hunger, 300);
    }

    #[test]
    fn test_pet_feed_max() {
        let mut pet = Pet::new(1, 100, test_uuid(), 1001, "Test".to_string(), 22001, 10000);
        pet.hunger = 950;
        pet.feed(100);
        assert_eq!(pet.hunger, 1000); // Max is 1000
    }

    #[test]
    fn test_pet_hunger() {
        let pet = Pet::new(1, 100, test_uuid(), 1001, "Test".to_string(), 22001, 10000);
        assert!(!pet.is_hungry()); // 500 > 200
        assert!(!pet.is_starving()); // 500 > 100
    }

    #[test]
    fn test_pet_intimacy_level() {
        let mut pet = Pet::new(1, 100, test_uuid(), 1001, "Test".to_string(), 22001, 10000);
        assert_eq!(pet.intimacy_level(), 2); // 10000 is neutral

        pet.intimacy = 50000;
        assert_eq!(pet.intimacy_level(), 4); // Loyal

        pet.intimacy = 99000;
        assert_eq!(pet.intimacy_level(), 8); // Extremely devoted
    }

    #[test]
    fn test_pet_rename() {
        let mut pet = Pet::new(
            1,
            100,
            test_uuid(),
            1001,
            "Old Name".to_string(),
            22001,
            10000,
        );
        assert!(!pet.renamed);
        pet.rename("New Name");
        assert_eq!(pet.name, "New Name");
        assert!(pet.renamed);
    }

    #[test]
    fn test_pet_database_lookup() {
        let db = PetDatabase::new_hardcoded();
        let poring = db.get_by_mob_id(1001);
        assert!(poring.is_some());
        assert_eq!(poring.unwrap().name, "Poring");

        let egg = db.get_by_egg_id(22001);
        assert!(egg.is_some());
        assert_eq!(egg.unwrap().mob_id, 1001);
    }

    #[test]
    fn test_pet_food_check() {
        let db = PetDatabase::new_hardcoded();
        // Jellopy (530) should be food for Poring
        let food_pet = db.is_pet_food(530);
        assert!(food_pet.is_some());
    }
}
