use serde::{Deserialize, Serialize};

/// 物品类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemType {
    Heal,           // 恢复道具
    Etc,            // 杂项
    Weapon,         // 武器
    Armor,          // 防具
    Card,           // 卡片
    PetEgg,         // 宠物蛋
    PetArmor,       // 宠物装备
}

/// 物品标志
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemFlag {
    None,
    Identified,     // 已鉴定
    Unique,         // 唯一
    NonTradable,    // 不可交易
    NoDrop,         // 不可丢弃
    NoTrade,        // 交易限制
}

/// 物品数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: u16,
    pub name: &'static str,
    pub type_: ItemType,
    pub price: u32,
    pub weight: u16,
    pub flags: u32,
    pub hp_restore: u16,      // HP恢复量
    pub sp_restore: u16,      // SP恢复量
    pub equip_mask: u32,     // 装备位置掩码
    pub atk: u16,             // 物理攻击
    pub matk: u16,            // 魔法攻击
    pub defense: u16,         // 防御
    pub magic_defense: u16,   // 魔法防御
    pub str_bonus: i16,       // STR加成
    pub agi_bonus: i16,       // AGI加成
    pub vit_bonus: i16,       // VIT加成
    pub int_bonus: i16,       // INT加成
    pub dex_bonus: i16,       // DEX加成
    pub luk_bonus: i16,       // LUK加成
}

impl Default for Item {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Item {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            name: "Unknown",
            type_: ItemType::Etc,
            price: 0,
            weight: 0,
            flags: 0,
            hp_restore: 0,
            sp_restore: 0,
            equip_mask: 0,
            atk: 0,
            matk: 0,
            defense: 0,
            magic_defense: 0,
            str_bonus: 0,
            agi_bonus: 0,
            vit_bonus: 0,
            int_bonus: 0,
            dex_bonus: 0,
            luk_bonus: 0,
        }
    }

    pub fn is_equip(&self) -> bool {
        matches!(self.type_, ItemType::Weapon | ItemType::Armor)
    }
}

/// 物品数据库
pub struct ItemDatabase {
    items: std::collections::HashMap<u16, Item>,
}

impl ItemDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            items: std::collections::HashMap::new(),
        };
        db.init_default_items();
        db
    }

    fn init_default_items(&mut self) {
        // 红色药水
        self.items.insert(501, Item {
            id: 501,
            name: "Red Potion",
            type_: ItemType::Heal,
            price: 50,
            weight: 7,
            flags: 0,
            hp_restore: 120,
            sp_restore: 0,
            equip_mask: 0,
            ..Default::default()
        });

        // 黄色药水
        self.items.insert(502, Item {
            id: 502,
            name: "Yellow Potion",
            type_: ItemType::Heal,
            price: 40,
            weight: 5,
            flags: 0,
            hp_restore: 60,
            sp_restore: 0,
            equip_mask: 0,
            ..Default::default()
        });

        // 蓝色药水
        self.items.insert(503, Item {
            id: 503,
            name: "Blue Potion",
            type_: ItemType::Heal,
            price: 50,
            weight: 7,
            flags: 0,
            hp_restore: 0,
            sp_restore: 40,
            equip_mask: 0,
            ..Default::default()
        });

        // 短剑
        self.items.insert(1201, Item {
            id: 1201,
            name: "Dagger",
            type_: ItemType::Weapon,
            price: 1000,
            weight: 50,
            flags: 0,
            hp_restore: 0,
            sp_restore: 0,
            equip_mask: 0x0001,  // 右手
            atk: 10,
            ..Default::default()
        });

        // 盗贼短剑
        self.items.insert(1202, Item {
            id: 1202,
            name: "Main Gauche",
            type_: ItemType::Weapon,
            price: 2500,
            weight: 60,
            flags: 0,
            hp_restore: 0,
            sp_restore: 0,
            equip_mask: 0x0001,
            atk: 15,
            ..Default::default()
        });

        // 布甲
        self.items.insert(1501, Item {
            id: 1501,
            name: "Clothes",
            type_: ItemType::Armor,
            price: 500,
            weight: 40,
            flags: 0,
            hp_restore: 0,
            sp_restore: 0,
            equip_mask: 0x0010,  // 身体
            defense: 2,
            ..Default::default()
        });
    }

    pub fn get(&self, item_id: u16) -> Option<&Item> {
        self.items.get(&item_id)
    }
}

impl Default for ItemDatabase {
    fn default() -> Self {
        Self::new()
    }
}
