pub mod data;

use crate::game::item::{Inventory, ItemDatabase};
use crate::game::map::Player;
use crate::game::zeny::MAX_ZENY;
use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

/// 交易状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeState {
    Requesting, // 请求中，等待对方接受
    Trading,    // 交易中，双方可以添加物品
    Completed,  // 已完成
    Cancelled,  // 已取消
}

/// 交易物品
#[derive(Debug, Clone, Copy)]
pub struct TradeItem {
    pub inventory_index: u16,
    pub item_id: u16,
    pub amount: u16,
}

/// 交易会话（使用内部可变性，允许多线程安全访问）
#[derive(Debug)]
pub struct TradeSession {
    pub id: Uuid,
    pub player1_id: Uuid,
    pub player2_id: Uuid,
    pub state: RwLock<TradeState>,
    pub items1: RwLock<Vec<TradeItem>>,
    pub items2: RwLock<Vec<TradeItem>>,
    pub zeny1: RwLock<u32>,
    pub zeny2: RwLock<u32>,
    pub locked1: RwLock<bool>,
    pub locked2: RwLock<bool>,
}

impl Clone for TradeSession {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            player1_id: self.player1_id,
            player2_id: self.player2_id,
            state: RwLock::new(*self.state.read()),
            items1: RwLock::new(self.items1.read().clone()),
            items2: RwLock::new(self.items2.read().clone()),
            zeny1: RwLock::new(*self.zeny1.read()),
            zeny2: RwLock::new(*self.zeny2.read()),
            locked1: RwLock::new(*self.locked1.read()),
            locked2: RwLock::new(*self.locked2.read()),
        }
    }
}

impl TradeSession {
    pub fn new(player1_id: Uuid, player2_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            player1_id,
            player2_id,
            state: RwLock::new(TradeState::Requesting),
            items1: RwLock::new(Vec::new()),
            items2: RwLock::new(Vec::new()),
            zeny1: RwLock::new(0),
            zeny2: RwLock::new(0),
            locked1: RwLock::new(false),
            locked2: RwLock::new(false),
        }
    }

    /// 检查是否为交易参与者
    pub fn is_participant(&self, player_id: Uuid) -> bool {
        self.player1_id == player_id || self.player2_id == player_id
    }

    /// 获取对方玩家ID
    pub fn get_partner_id(&self, player_id: Uuid) -> Option<Uuid> {
        if player_id == self.player1_id {
            Some(self.player2_id)
        } else if player_id == self.player2_id {
            Some(self.player1_id)
        } else {
            None
        }
    }

    /// 开始交易（从 Requesting 转为 Trading）
    pub fn start(&self) -> bool {
        let mut state = self.state.write();
        if *state == TradeState::Requesting {
            *state = TradeState::Trading;
            true
        } else {
            false
        }
    }

    /// 添加物品到交易
    pub fn add_item(&self, player_id: Uuid, item: TradeItem) -> bool {
        if *self.state.read() != TradeState::Trading {
            return false;
        }
        // 已锁定的玩家不能添加物品
        if self.is_locked(player_id) {
            return false;
        }
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

    /// 设置交易 Zeny
    pub fn set_zeny(&self, player_id: Uuid, amount: u32) -> bool {
        if *self.state.read() != TradeState::Trading {
            return false;
        }
        if self.is_locked(player_id) {
            return false;
        }
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

    /// 锁定交易（玩家点击确认）
    pub fn lock(&self, player_id: Uuid) -> bool {
        if *self.state.read() != TradeState::Trading {
            return false;
        }
        if player_id == self.player1_id {
            *self.locked1.write() = true;
            true
        } else if player_id == self.player2_id {
            *self.locked2.write() = true;
            true
        } else {
            false
        }
    }

    /// 检查玩家是否已锁定
    pub fn is_locked(&self, player_id: Uuid) -> bool {
        if player_id == self.player1_id {
            *self.locked1.read()
        } else if player_id == self.player2_id {
            *self.locked2.read()
        } else {
            false
        }
    }

    /// 检查双方是否都已锁定
    pub fn is_fully_locked(&self) -> bool {
        *self.locked1.read() && *self.locked2.read()
    }

    /// 取消交易
    pub fn cancel(&self) {
        *self.state.write() = TradeState::Cancelled;
    }

    /// 计算重量增加
    fn calc_weight_gain(items: &[TradeItem], item_db: &ItemDatabase) -> u32 {
        items
            .iter()
            .map(|item| {
                let item_data = item_db.get(item.item_id);
                item_data
                    .map(|i| (i.weight as u32) * (item.amount as u32))
                    .unwrap_or(0)
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
        let weight_gain_1 = Self::calc_weight_gain(&self.items2.read(), item_db);
        let weight_gain_2 = Self::calc_weight_gain(&self.items1.read(), item_db);

        let max_weight_1 = player1.max_weight();
        let max_weight_2 = player2.max_weight();
        let current_weight_1 = player1.current_weight();
        let current_weight_2 = player2.current_weight();

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
        let current_zeny_1 = player1.zeny();
        let current_zeny_2 = player2.zeny();

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

    /// 执行交易。双方都已锁定后调用，返回交易执行结果。
    pub fn execute(&self) -> Result<TradeExecution, TradeError> {
        if *self.state.read() != TradeState::Trading {
            return Err(TradeError::InvalidTradeState);
        }
        if !self.is_fully_locked() {
            return Err(TradeError::InvalidTradeState);
        }
        *self.state.write() = TradeState::Completed;
        Ok(TradeExecution {
            items_for_player1: self.items2.read().clone(),
            items_for_player2: self.items1.read().clone(),
            zeny_from_player1: *self.zeny1.read(),
            zeny_from_player2: *self.zeny2.read(),
        })
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

/// 交易执行结果：描述双方各收到什么
#[derive(Debug, Clone)]
pub struct TradeExecution {
    /// 玩家1 收到的物品（来自玩家2）
    pub items_for_player1: Vec<TradeItem>,
    /// 玩家2 收到的物品（来自玩家1）
    pub items_for_player2: Vec<TradeItem>,
    /// 玩家1 支付的 Zeny（转给玩家2）
    pub zeny_from_player1: u32,
    /// 玩家2 支付的 Zeny（转给玩家1）
    pub zeny_from_player2: u32,
}

/// 交易管理器
#[derive(Debug)]
pub struct TradeManager {
    /// 活跃的交易会话（session_id -> TradeSession）
    sessions: RwLock<HashMap<Uuid, TradeSession>>,
    /// 玩家到交易会话的映射（player_id -> session_id）
    player_sessions: RwLock<HashMap<Uuid, Uuid>>,
}

impl TradeManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            player_sessions: RwLock::new(HashMap::new()),
        }
    }

    /// 发起交易请求，创建新的交易会话
    pub fn request_trade(&self, player1_id: Uuid, player2_id: Uuid) -> Uuid {
        let session = TradeSession::new(player1_id, player2_id);
        let id = session.id;
        self.sessions.write().insert(id, session);
        self.player_sessions.write().insert(player1_id, id);
        self.player_sessions.write().insert(player2_id, id);
        id
    }

    /// 根据玩家ID查找其参与的交易会话
    pub fn find_session_for_player(&self, player_id: Uuid) -> Option<Uuid> {
        self.player_sessions.read().get(&player_id).copied()
    }

    /// 获取交易会话（克隆，仅用于读取状态）
    pub fn get_session(&self, id: Uuid) -> Option<TradeSession> {
        self.sessions.read().get(&id).cloned()
    }

    /// 开始交易（从 Requesting 转为 Trading）
    pub fn start_trade(&self, session_id: Uuid) -> bool {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(&session_id) {
            session.start()
        } else {
            false
        }
    }

    /// 添加物品到交易会话
    pub fn add_item_to_session(
        &self,
        session_id: Uuid,
        player_id: Uuid,
        item: TradeItem,
    ) -> bool {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(&session_id) {
            session.add_item(player_id, item)
        } else {
            false
        }
    }

    /// 设置交易 Zeny
    pub fn set_zeny_in_session(&self, session_id: Uuid, player_id: Uuid, amount: u32) -> bool {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(&session_id) {
            session.set_zeny(player_id, amount)
        } else {
            false
        }
    }

    /// 锁定交易（玩家点击确认），返回是否双方都已锁定
    pub fn lock_trade(&self, session_id: Uuid, player_id: Uuid) -> bool {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(&session_id) {
            if session.lock(player_id) {
                return session.is_fully_locked();
            }
        }
        false
    }

    /// 取消交易
    pub fn cancel_trade(&self, session_id: Uuid) {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(&session_id) {
            session.cancel();
        }
    }

    /// 结束交易，清理所有映射
    pub fn end_trade(&self, id: Uuid) {
        if let Some(session) = self.sessions.write().remove(&id) {
            let mut player_sessions = self.player_sessions.write();
            player_sessions.remove(&session.player1_id);
            player_sessions.remove(&session.player2_id);
        }
    }
}

impl Default for TradeManager {
    fn default() -> Self {
        Self::new()
    }
}
