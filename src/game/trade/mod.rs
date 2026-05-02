pub mod data;

use std::collections::HashMap;
use uuid::Uuid;
use parking_lot::RwLock;
use crate::game::map::Player;
use crate::game::item::{Inventory, ItemDatabase};
use crate::game::zeny::MAX_ZENY;

/// 交易物品
#[derive(Debug, Clone, Copy)]
pub struct TradeItem {
    pub inventory_index: u8,
    pub item_id: u16,
    pub amount: u16,
}

/// 交易会话
#[derive(Debug)]
pub struct TradeSession {
    pub id: Uuid,
    pub player1_id: Uuid,
    pub player2_id: Uuid,
    pub items1: RwLock<Vec<TradeItem>>,
    pub items2: RwLock<Vec<TradeItem>>,
    pub zeny1: RwLock<u32>,
    pub zeny2: RwLock<u32>,
    pub confirmed1: RwLock<bool>,
    pub confirmed2: RwLock<bool>,
}

impl Clone for TradeSession {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            player1_id: self.player1_id,
            player2_id: self.player2_id,
            items1: RwLock::new(self.items1.read().clone()),
            items2: RwLock::new(self.items2.read().clone()),
            zeny1: RwLock::new(*self.zeny1.read()),
            zeny2: RwLock::new(*self.zeny2.read()),
            confirmed1: RwLock::new(*self.confirmed1.read()),
            confirmed2: RwLock::new(*self.confirmed2.read()),
        }
    }
}

impl TradeSession {
    pub fn new(player1_id: Uuid, player2_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            player1_id,
            player2_id,
            items1: RwLock::new(Vec::new()),
            items2: RwLock::new(Vec::new()),
            zeny1: RwLock::new(0),
            zeny2: RwLock::new(0),
            confirmed1: RwLock::new(false),
            confirmed2: RwLock::new(false),
        }
    }

    /// 计算重量增加
    fn calc_weight_gain(items: &[TradeItem], item_db: &ItemDatabase) -> u32 {
        items.iter()
            .map(|item| {
                let item_data = item_db.get(item.item_id);
                item_data.map(|i| (i.weight as u32) * (item.amount as u32)).unwrap_or(0)
            })
            .sum()
    }

    /// 检查交易有效性（包括重量）
    pub fn validate(
        &self,
        player1: &Player,
        _inv1: &Inventory,
        player2: &Player,
        _inv2: &Inventory,
        item_db: &ItemDatabase,
    ) -> Result<(), TradeError> {
        // 计算双方将要增加的重量
        let weight_gain_1 = Self::calc_weight_gain(&*self.items2.read(), item_db);
        let weight_gain_2 = Self::calc_weight_gain(&*self.items1.read(), item_db);

        let max_weight_1 = *player1.max_weight.read();
        let max_weight_2 = *player2.max_weight.read();
        let current_weight_1 = *player1.current_weight.read();
        let current_weight_2 = *player2.current_weight.read();

        // 检查玩家1是否会超重
        if current_weight_1 + weight_gain_1 > max_weight_1 {
            return Err(TradeError::Overweight(player1.name.clone()));
        }

        // 检查玩家2是否会超重
        if current_weight_2 + weight_gain_2 > max_weight_2 {
            return Err(TradeError::Overweight(player2.name.clone()));
        }

        // 检查Zeny是否足够
        let zeny1 = *self.zeny1.read();
        let zeny2 = *self.zeny2.read();
        let current_zeny_1 = *player1.zeny.read();
        let current_zeny_2 = *player2.zeny.read();

        if current_zeny_1 < zeny1 {
            return Err(TradeError::NotEnoughZeny(player1.name.clone()));
        }
        if current_zeny_2 < zeny2 {
            return Err(TradeError::NotEnoughZeny(player2.name.clone()));
        }

        // 检查Zeny是否会导致溢出
        if current_zeny_1 - zeny1 + zeny2 > MAX_ZENY {
            return Err(TradeError::ZenyOverflow(player1.name.clone()));
        }
        if current_zeny_2 - zeny2 + zeny1 > MAX_ZENY {
            return Err(TradeError::ZenyOverflow(player2.name.clone()));
        }

        Ok(())
    }

    /// 确认交易
    pub fn confirm(&self, player_id: Uuid) -> bool {
        if player_id == self.player1_id {
            *self.confirmed1.write() = true;
        } else if player_id == self.player2_id {
            *self.confirmed2.write() = true;
        } else {
            return false;
        }
        // 检查双方是否都已确认
        *self.confirmed1.read() && *self.confirmed2.read()
    }

    /// 取消确认
    pub fn unconfirm(&self, player_id: Uuid) {
        if player_id == self.player1_id {
            *self.confirmed1.write() = false;
        } else if player_id == self.player2_id {
            *self.confirmed2.write() = false;
        }
    }

    /// 添加物品到交易
    pub fn add_item(&self, player_id: Uuid, item: TradeItem) -> bool {
        if player_id == self.player1_id {
            self.items1.write().push(item);
            true
        } else if player_id == self.player2_id {
            self.items2.write().push(item);
            true
        } else {
            false
        }
    }

    /// 设置Zeny
    pub fn set_zeny(&self, player_id: Uuid, amount: u32) -> bool {
        if player_id == self.player1_id {
            *self.zeny1.write() = amount;
            true
        } else if player_id == self.player2_id {
            *self.zeny2.write() = amount;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub enum TradeError {
    PlayerNotFound(String),
    InvalidTradeState,
    Overweight(String),
    NotEnoughZeny(String),
    ZenyOverflow(String),
    InventoryFull(String),
    ItemNotFound(String),
}

/// 交易管理器
#[derive(Debug)]
pub struct TradeManager {
    sessions: RwLock<HashMap<Uuid, TradeSession>>,
}

impl TradeManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// 发起交易请求
    pub fn request_trade(&self, player1_id: Uuid, player2_id: Uuid) -> Uuid {
        let session = TradeSession::new(player1_id, player2_id);
        let id = session.id;
        self.sessions.write().insert(id, session);
        id
    }

    /// 获取交易会话
    pub fn get_session(&self, id: Uuid) -> Option<TradeSession> {
        self.sessions.read().get(&id).cloned()
    }

    /// 结束交易
    pub fn end_trade(&self, id: Uuid) {
        self.sessions.write().remove(&id);
    }
}

impl Default for TradeManager {
    fn default() -> Self {
        Self::new()
    }
}
