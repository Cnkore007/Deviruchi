use super::data::Npc;
use crate::game::item::Inventory;
use crate::game::map::Player;
use crate::game::zeny::ZenyManager;
use std::sync::Arc;

/// NPC交互处理器
pub struct NpcHandler {
    npcs: std::collections::HashMap<u32, Arc<Npc>>,
}

impl NpcHandler {
    pub fn new() -> Self {
        let mut handler = Self {
            npcs: std::collections::HashMap::new(),
        };
        handler.init_default_npcs();
        handler
    }

    fn init_default_npcs(&mut self) {
        if let Some(npc) = super::data::NpcDatabase::get_npc(1) {
            self.npcs.insert(1, Arc::new(npc));
        }
        if let Some(npc) = super::data::NpcDatabase::get_npc(2) {
            self.npcs.insert(2, Arc::new(npc));
        }
        if let Some(npc) = super::data::NpcDatabase::get_npc(3) {
            self.npcs.insert(3, Arc::new(npc));
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
            super::data::NpcType::SkillTrainer => NpcResponse::SkillList {
                npc_id,
                skills: npc.skills.read().clone(),
            },
            super::data::NpcType::Warp => NpcResponse::Warp {
                map: npc.map_name.clone(),
                x: npc.pos_x,
                y: npc.pos_y,
            },
            _ => NpcResponse::Message(npc.display_name.clone()),
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

    /// 学习技能
    pub fn learn_skill(&self, _player: &Player, npc_id: u32, skill_id: u16) -> LearnResult {
        let npc = match self.get_npc(npc_id) {
            Some(n) => n,
            None => return LearnResult::NpcNotFound,
        };

        let npc_skill = npc
            .skills
            .read()
            .iter()
            .find(|s| s.skill_id == skill_id)
            .copied();

        match npc_skill {
            Some(_) => LearnResult::Success { skill_id },
            None => LearnResult::SkillNotFound,
        }
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
