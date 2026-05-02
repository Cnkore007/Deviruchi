use parking_lot::RwLock;

/// NPC类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcType {
    Shop,          // 商店
    SkillTrainer,  // 技能训练师
    Quest,         // 任务NPC
    Warp,          // 传送门
    CashShop,      // 现金商店
}

/// NPC标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcFlag {
    None,
    NoWarp,        // 不能传送
    NoMob,         // 不刷怪
    NoSave,        // 不保存位置
    Private_,      // 私有NPC
}

/// NPC数据
#[derive(Debug)]
pub struct Npc {
    pub id: u32,
    pub name: String,
    pub display_name: String,
    pub type_: NpcType,
    pub pos_x: u16,
    pub pos_y: u16,
    pub map_name: String,
    pub sprite_id: u16,
    pub level: u16,
    pub flags: u32,

    // 商店物品 (如果是商店)
    pub shop_items: RwLock<Vec<ShopItem>>,

    // 技能列表 (如果是技能训练师)
    pub skills: RwLock<Vec<NpcSkill>>,
}

impl Npc {
    pub fn new(id: u32, name: &str, x: u16, y: u16, map: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            display_name: name.to_string(),
            type_: NpcType::Shop,
            pos_x: x,
            pos_y: y,
            map_name: map.to_string(),
            sprite_id: 100,
            level: 1,
            flags: 0,
            shop_items: RwLock::new(Vec::new()),
            skills: RwLock::new(Vec::new()),
        }
    }

    pub fn shop(id: u32, name: &str, x: u16, y: u16, map: &str) -> Self {
        let mut npc = Self::new(id, name, x, y, map);
        npc.type_ = NpcType::Shop;
        npc
    }

    pub fn skill_trainer(id: u32, name: &str, x: u16, y: u16, map: &str) -> Self {
        let mut npc = Self::new(id, name, x, y, map);
        npc.type_ = NpcType::SkillTrainer;
        npc
    }

    pub fn warp(id: u32, name: &str, x: u16, y: u16, map: &str, _dest_map: &str, _dest_x: u16, _dest_y: u16) -> Self {
        let mut npc = Self::new(id, name, x, y, map);
        npc.type_ = NpcType::Warp;
        npc
    }

    pub fn add_shop_item(&self, item_id: u16, price: u32) {
        self.shop_items.write().push(ShopItem { item_id, price });
    }

    pub fn add_skill(&self, skill_id: u16, sp_cost: u16, price: u32) {
        self.skills.write().push(NpcSkill { skill_id, sp_cost, price });
    }
}

/// 商店物品
#[derive(Debug, Clone, Copy)]
pub struct ShopItem {
    pub item_id: u16,
    pub price: u32,
}

/// NPC技能
#[derive(Debug, Clone, Copy)]
pub struct NpcSkill {
    pub skill_id: u16,
    pub sp_cost: u16,
    pub price: u32,
}

/// NPC数据库
pub struct NpcDatabase;

impl NpcDatabase {
    pub fn get_npc(id: u32) -> Option<Npc> {
        match id {
            1 => Some(Self::create_poring_merchant()),
            2 => Some(Self::create_basilisk_warrior()),
            3 => Some(Self::create_prontera_warp()),
            _ => None,
        }
    }

    fn create_poring_merchant() -> Npc {
        let mut npc = Npc::shop(1, "Poring Merchant", 50, 100, "new_1-1.gat");
        npc.display_name = "波利商人".to_string();
        npc.sprite_id = 124;
        npc.add_shop_item(501, 50);   // Red Potion
        npc.add_shop_item(502, 40);   // Yellow Potion
        npc.add_shop_item(503, 50);   // Blue Potion
        npc
    }

    fn create_basilisk_warrior() -> Npc {
        let mut npc = Npc::skill_trainer(2, "Basilisk Warrior", 100, 50, "new_1-1.gat");
        npc.display_name = "蜥蜴武士".to_string();
        npc.sprite_id = 404;
        npc.level = 10;
        npc.add_skill(1, 8, 1000);    // Bash
        npc.add_skill(25, 9, 2000);   // Fire Ball
        npc
    }

    fn create_prontera_warp() -> Npc {
        let mut npc = Npc::warp(3, "To Prontera", 150, 150, "new_1-1.gat", "prontera.gat", 150, 100);
        npc.display_name = "前往普隆德拉".to_string();
        npc.sprite_id = 405;
        npc
    }
}
