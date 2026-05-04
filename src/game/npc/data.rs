use parking_lot::RwLock;

/// NPC类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcType {
    Shop,         // 商店
    SkillTrainer, // 技能训练师
    Quest,        // 任务NPC
    Warp,         // 传送门
    CashShop,     // 现金商店
}

/// NPC标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcFlag {
    None,
    NoWarp,   // 不能传送
    NoMob,    // 不刷怪
    NoSave,   // 不保存位置
    Private_, // 私有NPC
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

    // NPC脚本 (如果有)
    pub script: Option<String>,

    // 传送目标（仅 Warp 类型 NPC 使用）
    pub dest_map: Option<String>,
    pub dest_x: u16,
    pub dest_y: u16,
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
            script: None,
            dest_map: None,
            dest_x: 0,
            dest_y: 0,
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

    pub fn warp(
        id: u32,
        name: &str,
        x: u16,
        y: u16,
        map: &str,
        dest_map: &str,
        dest_x: u16,
        dest_y: u16,
    ) -> Self {
        let mut npc = Self::new(id, name, x, y, map);
        npc.type_ = NpcType::Warp;
        npc.dest_map = Some(dest_map.to_string());
        npc.dest_x = dest_x;
        npc.dest_y = dest_y;
        npc
    }

    pub fn add_shop_item(&self, item_id: u16, buy_price: u32, sell_price: u32) {
        self.shop_items.write().push(ShopItem {
            item_id,
            buy_price,
            sell_price,
        });
    }

    pub fn add_skill(&self, skill_id: u16, sp_cost: u16, price: u32) {
        self.skills.write().push(NpcSkill {
            skill_id,
            sp_cost,
            price,
        });
    }
}

/// 商店物品
#[derive(Debug, Clone, Copy)]
pub struct ShopItem {
    pub item_id: u16,
    pub buy_price: u32,  // NPC卖给玩家的价格
    pub sell_price: u32, // NPC收购价格
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
            4 => Some(Self::create_quest_npc()),
            5 => Some(Self::create_cash_shop_npc()),
            6 => Some(Self::create_geffen_warp()),
            7 => Some(Self::create_healing_nurse()),
            _ => None,
        }
    }

    fn create_poring_merchant() -> Npc {
        let mut npc = Npc::shop(1, "Poring Merchant", 50, 100, "new_1-1.gat");
        npc.display_name = "波利商人".to_string();
        npc.sprite_id = 124;
        npc.add_shop_item(501, 50, 25); // Red Potion
        npc.add_shop_item(502, 40, 20); // Yellow Potion
        npc.add_shop_item(503, 50, 25); // Blue Potion
        npc.script = Some(
            r#"mes "[Poring Merchant]"
mes "Welcome to my shop!"
mes "What would you like to do?"
next
select "Buy:Sell:Talk:Leave"
close"#.to_string(),
        );
        npc
    }

    fn create_basilisk_warrior() -> Npc {
        let mut npc = Npc::skill_trainer(2, "Basilisk Warrior", 100, 50, "new_1-1.gat");
        npc.display_name = "蜥蜴武士".to_string();
        npc.sprite_id = 404;
        npc.level = 10;
        npc.add_skill(1, 8, 1000); // Bash
        npc.add_skill(25, 9, 2000); // Fire Ball
        npc
    }

    fn create_prontera_warp() -> Npc {
        let mut npc = Npc::warp(
            3,
            "To Prontera",
            150,
            150,
            "new_1-1.gat",
            "prontera.gat",
            150,
            100,
        );
        npc.display_name = "前往普隆德拉".to_string();
        npc.sprite_id = 405;
        npc
    }

    fn create_quest_npc() -> Npc {
        let mut npc = Npc::new(4, "Quest Master", 160, 130, "prontera.gat");
        npc.display_name = "任务大师".to_string();
        npc.type_ = NpcType::Quest;
        npc.sprite_id = 725;
        npc.script = Some(
            r#"mes "[Quest Master]"
mes "I have many tasks for brave adventurers!"
mes "What would you like to do?"
next
select("Accept Quest:Check Progress:Leave")
close"#.to_string(),
        );
        npc
    }

    fn create_cash_shop_npc() -> Npc {
        let mut npc = Npc::new(5, "Cash Shop", 150, 150, "prontera.gat");
        npc.display_name = "现金商店".to_string();
        npc.type_ = NpcType::CashShop;
        npc.sprite_id = 112;
        npc.script = Some(
            r#"mes "[Cash Shop]"
mes "Welcome to the Cash Shop!"
mes "Here you can purchase special items."
close"#.to_string(),
        );
        npc
    }

    fn create_geffen_warp() -> Npc {
        let mut npc = Npc::warp(
            6,
            "To Geffen",
            120,
            100,
            "prontera.gat",
            "geffen.gat",
            119,
            59,
        );
        npc.display_name = "前往吉芬".to_string();
        npc.sprite_id = 406;
        npc
    }

    fn create_healing_nurse() -> Npc {
        let mut npc = Npc::new(7, "Nurse", 150, 180, "prontera.gat");
        npc.display_name = "护士".to_string();
        npc.type_ = NpcType::SkillTrainer;
        npc.sprite_id = 114;
        npc.add_skill(28, 6, 0);  // Heal, 免费
        npc.add_skill(29, 10, 500); // Increase AGI, 500 Zeny
        npc.script = Some(
            r#"mes "[Nurse]"
mes "Hello! Do you need healing?"
next
select("Heal me:Learn Skills:Leave")
close"#.to_string(),
        );
        npc
    }
}
