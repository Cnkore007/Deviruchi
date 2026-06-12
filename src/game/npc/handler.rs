use super::data::Npc;
use crate::game::item::Inventory;
use crate::game::map::Player;
use crate::game::zeny::ZenyManager;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// NPC交互处理器
pub struct NpcHandler {
    npcs: std::collections::HashMap<u32, Arc<Npc>>,
    /// 已学习技能表: char_id -> Set<skill_id>
    learned_skills: RwLock<HashMap<u32, HashSet<u16>>>,
}

impl NpcHandler {
    pub fn new() -> Self {
        let mut handler = Self {
            npcs: std::collections::HashMap::new(),
            learned_skills: RwLock::new(HashMap::new()),
        };
        handler.init_default_npcs();
        handler
    }

    fn init_default_npcs(&mut self) {
        // 从 NpcDatabase 加载所有 NPC
        let db = super::data::NpcDatabase::default_instance();
        for (id, npc_ref) in db.all() {
            // Npc 没有 Clone，直接存引用会导致生命周期问题
            // 因此重新构造一个 Npc 副本
            let npc = super::data::Npc {
                id: npc_ref.id,
                name: npc_ref.name.clone(),
                display_name: npc_ref.display_name.clone(),
                type_: npc_ref.type_,
                pos_x: npc_ref.pos_x,
                pos_y: npc_ref.pos_y,
                map_name: npc_ref.map_name.clone(),
                sprite_id: npc_ref.sprite_id,
                level: npc_ref.level,
                flags: npc_ref.flags,
                shop_items: parking_lot::RwLock::new(npc_ref.shop_items.read().clone()),
                skills: parking_lot::RwLock::new(npc_ref.skills.read().clone()),
                script: npc_ref.script.clone(),
                dest_map: npc_ref.dest_map.clone(),
                dest_x: npc_ref.dest_x,
                dest_y: npc_ref.dest_y,
                event: npc_ref.event,
                trigger_radius: npc_ref.trigger_radius,
            };
            self.npcs.insert(*id, Arc::new(npc));
        }
    }

    /// 获取NPC
    pub fn get_npc(&self, npc_id: u32) -> Option<Arc<Npc>> {
        self.npcs.get(&npc_id).cloned()
    }

    /// 获取地图上的NPC
    pub fn get_npcs_on_map(&self, map_name: &str) -> Vec<Arc<Npc>> {
        self.npcs
            .values()
            .filter(|n| n.map_name == map_name)
            .cloned()
            .collect()
    }

    /// 处理NPC交互
    pub fn interact(&self, _player: &Player, npc_id: u32) -> NpcResponse {
        let npc = match self.get_npc(npc_id) {
            Some(n) => n,
            None => return NpcResponse::NotFound,
        };

        match npc.type_ {
            super::data::NpcType::Shop => NpcResponse::OpenShop {
                npc_id,
                items: npc.shop_items.read().clone(),
            },
            super::data::NpcType::SkillTrainer => {
                // 有脚本时优先使用脚本驱动对话
                if let Some(ref script) = npc.script {
                    NpcResponse::StartScript {
                        npc_id,
                        script: script.clone(),
                    }
                } else {
                    NpcResponse::SkillList {
                        npc_id,
                        skills: npc.skills.read().clone(),
                    }
                }
            }
            super::data::NpcType::Warp => {
                // 返回传送目标坐标（而非 NPC 自身坐标）
                let dest_map = npc.dest_map.clone()
                    .unwrap_or_else(|| npc.map_name.clone());
                NpcResponse::Warp {
                    map: dest_map,
                    x: npc.dest_x,
                    y: npc.dest_y,
                }
            }
            super::data::NpcType::Quest | super::data::NpcType::CashShop => {
                // Quest 和 CashShop 类型通过脚本驱动
                if let Some(ref script_text) = npc.script {
                    NpcResponse::StartScript {
                        npc_id,
                        script: script_text.clone(),
                    }
                } else {
                    NpcResponse::Message(npc.display_name.clone())
                }
            }
        }
    }

    /// 购买物品
    pub fn buy_item(
        &self,
        player: &Player,
        inventory: &mut Inventory,
        npc_id: u32,
        item_id: u16,
        amount: u8,
    ) -> BuyResult {
        let npc = match self.get_npc(npc_id) {
            Some(n) => n,
            None => return BuyResult::NpcNotFound,
        };

        let shop_item = npc
            .shop_items
            .read()
            .iter()
            .find(|i| i.item_id == item_id)
            .copied();

        let shop_item = match shop_item {
            Some(i) => i,
            None => return BuyResult::ItemNotFound,
        };

        let total_price = shop_item.buy_price * amount as u32;

        // 检查金币
        if !ZenyManager::can_spend(player, total_price) {
            return BuyResult::NotEnoughZeny;
        }

        // 检查重量
        let max_weight = player.max_weight();
        if !inventory.can_carry_weight(item_id, amount as u16, max_weight) {
            return BuyResult::Overweight;
        }

        // 检查背包空间
        if !inventory.can_add_item(item_id, amount as u16) {
            return BuyResult::InventoryFull;
        }

        // 扣除金币
        ZenyManager::sub(player, total_price);

        // 添加物品
        inventory.add_item(item_id, amount as u16);

        BuyResult::Success {
            item_id,
            amount,
            remaining_zeny: ZenyManager::get(player),
        }
    }

    /// 出售物品
    pub fn sell_item(
        &self,
        player: &Player,
        inventory: &mut Inventory,
        inventory_index: u8,
        amount: u8,
    ) -> SellResult {
        // 获取物品
        let slot = match inventory.slots().get(inventory_index as usize) {
            Some(s) if !s.is_empty() => s.clone(),
            _ => return SellResult::Failed(SellError::InvalidSlot),
        };

        if slot.amount < amount as u16 {
            return SellResult::Failed(SellError::NotEnoughItems);
        }

        // 获取物品数据
        let item = match inventory.get_database().get(slot.item_id) {
            Some(i) => i,
            None => return SellResult::Failed(SellError::InvalidItem),
        };

        let total_gold = item.sell_price * amount as u32;

        // 检查是否会超过Zeny上限
        let current_zeny = ZenyManager::get(player);
        if current_zeny + total_gold > crate::game::zeny::MAX_ZENY {
            return SellResult::Failed(SellError::ZenyOverflow);
        }

        // 移除物品
        if !inventory.remove_item(inventory_index, amount as u16) {
            return SellResult::Failed(SellError::RemoveFailed);
        }

        // 增加Zeny
        ZenyManager::add(player, total_gold);

        SellResult::Success {
            item_id: slot.item_id,
            amount,
            gained_zeny: total_gold,
        }
    }

    /// 学习技能（完整流程：验证 NPC -> 检查技能 -> 检查已学习 -> 扣除 Zeny）
    pub fn learn_skill(&self, player: &Player, npc_id: u32, skill_id: u16) -> LearnResult {
        // 1. 检查 NPC 存在
        let npc = match self.get_npc(npc_id) {
            Some(n) => n,
            None => return LearnResult::NpcNotFound,
        };

        // 2. 检查 NPC 上有该技能
        let npc_skill = npc
            .skills
            .read()
            .iter()
            .find(|s| s.skill_id == skill_id)
            .copied();

        let npc_skill = match npc_skill {
            Some(s) => s,
            None => return LearnResult::SkillNotFound,
        };

        // 3. 检查是否已学习
        {
            let learned = self.learned_skills.read();
            if let Some(skills) = learned.get(&player.char_id) {
                if skills.contains(&skill_id) {
                    return LearnResult::AlreadyLearned;
                }
            }
        }

        // 4. 检查 Zeny 是否足够
        if !ZenyManager::can_spend(player, npc_skill.price) {
            return LearnResult::NotEnoughZeny;
        }

        // 5. 扣除 Zeny
        ZenyManager::sub(player, npc_skill.price);

        // 6. 记录已学习
        {
            let mut learned = self.learned_skills.write();
            learned
                .entry(player.char_id)
                .or_insert_with(HashSet::new)
                .insert(skill_id);
        }

        LearnResult::Success { skill_id }
    }
}

impl Default for NpcHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// NPC响应
#[derive(Debug, Clone)]
pub enum NpcResponse {
    NotFound,
    Message(String),
    OpenShop {
        npc_id: u32,
        items: Vec<super::data::ShopItem>,
    },
    SkillList {
        npc_id: u32,
        skills: Vec<super::data::NpcSkill>,
    },
    Warp {
        map: String,
        x: u16,
        y: u16,
    },
    /// 启动脚本对话
    StartScript {
        npc_id: u32,
        script: String,
    },
}

/// 购买结果
#[derive(Debug, Clone)]
pub enum BuyResult {
    Success {
        item_id: u16,
        amount: u8,
        remaining_zeny: u32,
    },
    NpcNotFound,
    ItemNotFound,
    NotEnoughZeny,
    InventoryFull,
    Overweight,
}

/// 出售结果
#[derive(Debug, Clone)]
pub enum SellResult {
    Success {
        item_id: u16,
        amount: u8,
        gained_zeny: u32,
    },
    Failed(SellError),
}

/// 出售错误
#[derive(Debug, Clone, Copy)]
pub enum SellError {
    InvalidSlot,
    NotEnoughItems,
    InvalidItem,
    ZenyOverflow,
    RemoveFailed,
}

/// 学习技能结果
#[derive(Debug, Clone)]
pub enum LearnResult {
    Success { skill_id: u16 },
    NpcNotFound,
    SkillNotFound,
    NotEnoughZeny,
    AlreadyLearned,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::map::Player;
    use crate::storage::Character;

    fn create_test_player() -> Player {
        let char = Character {
            char_id: 1,
            char_num: 0,
            name: "TestPlayer".to_string(),
            class: 0,
            base_level: 1,
            job_level: 1,
            base_exp: 0,
            job_exp: 0,
            zeny: 10000,
            str: 10,
            agi: 10,
            vit: 10,
            int: 10,
            dex: 10,
            luk: 10,
            hp: 100,
            max_hp: 100,
            sp: 50,
            max_sp: 50,
            hair: 0,
            hair_color: 0,
            clothes_color: 0,
            weapon: 0,
            shield: 0,
            head_top: 0,
            head_mid: 0,
            head_bottom: 0,
            last_map: "new_1-1.gat".to_string(),
            last_x: 50,
            last_y: 50,
            save_map: "new_1-1.gat".to_string(),
            save_x: 50,
            save_y: 50,
            delete_timer: 0,
            status_point: 0,
            skill_point: 0,
            created_at: 0,
            updated_at: 0,
        };
        Player::from_character(char)
    }

    #[test]
    fn test_warp_npc_returns_dest_coordinates() {
        let handler = NpcHandler::new();
        let player = create_test_player();

        // NPC 3 是 Prontera 传送门
        let response = handler.interact(&player, 3);
        match response {
            NpcResponse::Warp { map, x, y } => {
                assert_eq!(map, "prontera.gat", "应返回目标地图，而非 NPC 所在地图");
                assert_eq!(x, 150, "应返回目标 X 坐标");
                assert_eq!(y, 100, "应返回目标 Y 坐标");
            }
            other => panic!("期望 Warp 响应，实际: {:?}", other),
        }
    }

    #[test]
    fn test_warp_npc_stores_dest_fields() {
        let npc = crate::game::npc::data::Npc::warp(
            100,
            "Test Warp",
            50,
            50,
            "prontera.gat",
            "geffen.gat",
            120,
            100,
        );
        assert_eq!(npc.dest_map.as_deref(), Some("geffen.gat"));
        assert_eq!(npc.dest_x, 120);
        assert_eq!(npc.dest_y, 100);
    }

    #[test]
    fn test_npc_not_found() {
        let handler = NpcHandler::new();
        let player = create_test_player();
        let response = handler.interact(&player, 9999);
        assert!(matches!(response, NpcResponse::NotFound));
    }

    #[test]
    fn test_learn_skill_success() {
        let handler = NpcHandler::new();
        let player = create_test_player();

        // NPC 2 是技能训练师，技能 1 (Bash) 价格 1000
        let result = handler.learn_skill(&player, 2, 1);
        assert!(matches!(result, LearnResult::Success { skill_id: 1 }));
    }

    #[test]
    fn test_learn_skill_not_enough_zeny() {
        let handler = NpcHandler::new();
        let player = create_test_player();
        // 设置 Zeny 为 0
        crate::game::zeny::ZenyManager::set(&player, 0);

        let result = handler.learn_skill(&player, 2, 1);
        assert!(matches!(result, LearnResult::NotEnoughZeny));
    }

    #[test]
    fn test_learn_skill_deducts_zeny() {
        let handler = NpcHandler::new();
        let player = create_test_player();
        let initial_zeny = crate::game::zeny::ZenyManager::get(&player);

        handler.learn_skill(&player, 2, 1); // Bash, price 1000

        let remaining = crate::game::zeny::ZenyManager::get(&player);
        assert_eq!(remaining, initial_zeny - 1000);
    }

    #[test]
    fn test_learn_skill_already_learned() {
        let handler = NpcHandler::new();
        let player = create_test_player();

        // 第一次学习
        handler.learn_skill(&player, 2, 1);
        // 第二次学习应返回已学习
        let result = handler.learn_skill(&player, 2, 1);
        assert!(matches!(result, LearnResult::AlreadyLearned));
    }

    #[test]
    fn test_learn_skill_not_found_on_npc() {
        let handler = NpcHandler::new();
        let player = create_test_player();

        // NPC 1 是商店，没有技能
        let result = handler.learn_skill(&player, 1, 1);
        assert!(matches!(result, LearnResult::SkillNotFound));
    }

    #[test]
    fn test_learn_skill_npc_not_found() {
        let handler = NpcHandler::new();
        let player = create_test_player();

        let result = handler.learn_skill(&player, 9999, 1);
        assert!(matches!(result, LearnResult::NpcNotFound));
    }

    #[test]
    fn test_quest_npc_exists() {
        let handler = NpcHandler::new();
        let npc = handler.get_npc(4);
        assert!(npc.is_some());
        let npc = npc.unwrap();
        assert_eq!(npc.type_, crate::game::npc::data::NpcType::Quest);
        assert!(npc.script.is_some());
    }

    #[test]
    fn test_cash_shop_npc_exists() {
        let handler = NpcHandler::new();
        let npc = handler.get_npc(5);
        assert!(npc.is_some());
        let npc = npc.unwrap();
        assert_eq!(npc.type_, crate::game::npc::data::NpcType::CashShop);
    }

    #[test]
    fn test_quest_npc_interaction_returns_script() {
        let handler = NpcHandler::new();
        let player = create_test_player();
        let response = handler.interact(&player, 4);
        assert!(matches!(response, NpcResponse::StartScript { .. }));
    }

    #[test]
    fn test_geffen_warp_target() {
        let handler = NpcHandler::new();
        let player = create_test_player();
        let response = handler.interact(&player, 6);
        match response {
            NpcResponse::Warp { map, x, y } => {
                assert_eq!(map, "geffen.gat");
                assert_eq!(x, 119);
                assert_eq!(y, 59);
            }
            other => panic!("期望 Warp 响应，实际: {:?}", other),
        }
    }

    #[test]
    fn test_healing_nurse_has_skills() {
        let handler = NpcHandler::new();
        let npc = handler.get_npc(7).unwrap();
        let skills = npc.skills.read();
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].skill_id, 28); // Heal
        assert_eq!(skills[0].price, 0);     // 免费
    }

    #[test]
    fn test_get_npcs_on_map() {
        let handler = NpcHandler::new();
        let prontera_npcs = handler.get_npcs_on_map("prontera.gat");
        // prontera.gat 应该有: Quest(4), CashShop(5), GeffenWarp(6), Nurse(7)
        assert!(prontera_npcs.len() >= 4);
    }
}
