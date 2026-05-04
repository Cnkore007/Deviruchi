use uuid::Uuid;

/// 交易物品
#[derive(Debug, Clone)]
pub struct TradeItem {
    pub index: u16,       // 背包索引
    pub item_id: u16,     // 物品ID
    pub amount: u16,      // 数量
    pub identified: bool, // 是否已鉴定
    pub refine: u8,       // 精炼等级
    pub cards: [u16; 4],  // 卡片槽
}

impl TradeItem {
    pub fn new(
        index: u16,
        item_id: u16,
        amount: u16,
        identified: bool,
        refine: u8,
        cards: [u16; 4],
    ) -> Self {
        Self {
            index,
            item_id,
            amount,
            identified,
            refine,
            cards,
        }
    }
}

/// 交易状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeState {
    Requesting, // 请求中
    Trading,    // 交易中
    Locked,     // 已锁定
    Completed,  // 已完成
    Cancelled,  // 已取消
}

/// 交易会话
#[derive(Debug)]
pub struct TradeSession {
    pub id: Uuid,
    pub player1_id: Uuid,
    pub player2_id: Uuid,
    pub state: TradeState,

    // 玩家1的交易内容
    pub player1_items: Vec<TradeItem>,
    pub player1_zeny: u32,
    pub player1_locked: bool,

    // 玩家2的交易内容
    pub player2_items: Vec<TradeItem>,
    pub player2_zeny: u32,
    pub player2_locked: bool,
}

impl TradeSession {
    pub fn new(player1_id: Uuid, player2_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            player1_id,
            player2_id,
            state: TradeState::Requesting,
            player1_items: Vec::new(),
            player1_zeny: 0,
            player1_locked: false,
            player2_items: Vec::new(),
            player2_zeny: 0,
            player2_locked: false,
        }
    }

    /// 开始交易
    pub fn start(&mut self) -> bool {
        if self.state == TradeState::Requesting {
            self.state = TradeState::Trading;
            true
        } else {
            false
        }
    }

    /// 检查是否为交易参与者
    pub fn is_participant(&self, player_id: Uuid) -> bool {
        self.player1_id == player_id || self.player2_id == player_id
    }

    /// 添加物品
    pub fn add_item(&mut self, player_id: Uuid, item: TradeItem) -> bool {
        if self.state != TradeState::Trading {
            return false;
        }

        // 检查该玩家是否已锁定
        if self.is_locked(player_id) {
            return false;
        }

        if player_id == self.player1_id {
            self.player1_items.push(item);
            true
        } else if player_id == self.player2_id {
            self.player2_items.push(item);
            true
        } else {
            false
        }
    }

    /// 添加 Zeny
    pub fn add_zeny(&mut self, player_id: Uuid, amount: u32) -> bool {
        if self.state != TradeState::Trading {
            return false;
        }

        // 检查该玩家是否已锁定
        if self.is_locked(player_id) {
            return false;
        }

        if player_id == self.player1_id {
            self.player1_zeny = amount;
            true
        } else if player_id == self.player2_id {
            self.player2_zeny = amount;
            true
        } else {
            false
        }
    }

    /// 锁定交易
    pub fn lock(&mut self, player_id: Uuid) -> bool {
        if self.state != TradeState::Trading {
            return false;
        }

        if player_id == self.player1_id {
            self.player1_locked = true;
            true
        } else if player_id == self.player2_id {
            self.player2_locked = true;
            true
        } else {
            false
        }
    }

    /// 取消锁定
    pub fn unlock(&mut self, player_id: Uuid) -> bool {
        if self.state != TradeState::Trading {
            return false;
        }

        if player_id == self.player1_id && self.player1_locked {
            self.player1_locked = false;
            true
        } else if player_id == self.player2_id && self.player2_locked {
            self.player2_locked = false;
            true
        } else {
            false
        }
    }

    /// 检查是否已锁定
    pub fn is_locked(&self, player_id: Uuid) -> bool {
        if player_id == self.player1_id {
            self.player1_locked
        } else if player_id == self.player2_id {
            self.player2_locked
        } else {
            false
        }
    }

    /// 检查是否双方都锁定
    pub fn is_fully_locked(&self) -> bool {
        self.player1_locked && self.player2_locked
    }

    /// 获取对方ID
    pub fn get_partner_id(&self, player_id: Uuid) -> Option<Uuid> {
        if player_id == self.player1_id {
            Some(self.player2_id)
        } else if player_id == self.player2_id {
            Some(self.player1_id)
        } else {
            None
        }
    }

    /// 获取对方物品
    pub fn get_partner_items(&self, player_id: Uuid) -> &[TradeItem] {
        if player_id == self.player1_id {
            &self.player2_items
        } else {
            &self.player1_items
        }
    }

    /// 获取对方 Zeny
    pub fn get_partner_zeny(&self, player_id: Uuid) -> u32 {
        if player_id == self.player1_id {
            self.player2_zeny
        } else {
            self.player1_zeny
        }
    }

    /// 获取自己的物品
    pub fn get_my_items(&self, player_id: Uuid) -> &[TradeItem] {
        if player_id == self.player1_id {
            &self.player1_items
        } else {
            &self.player2_items
        }
    }

    /// 获取自己的 Zeny
    pub fn get_my_zeny(&self, player_id: Uuid) -> u32 {
        if player_id == self.player1_id {
            self.player1_zeny
        } else {
            self.player2_zeny
        }
    }

    /// 完成交易
    pub fn complete(&mut self) -> bool {
        if self.is_fully_locked() && self.state == TradeState::Trading {
            self.state = TradeState::Completed;
            true
        } else {
            false
        }
    }

    /// 取消交易
    pub fn cancel(&mut self) {
        self.state = TradeState::Cancelled;
    }
}
