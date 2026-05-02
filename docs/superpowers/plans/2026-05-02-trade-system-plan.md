# 交易系统 (Trade System) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现玩家间交易系统，支持物品和 Zeny 的交易，包含请求、确认、锁定、取消等完整流程。

**Architecture:** Trade 系统使用状态机模式管理交易会话。TradeManager 管理所有进行中的交易。每个交易有两个参与者，双方各自确认后交易才生效。交易过程使用乐观锁定：一方锁定后，只能由自己取消；双方锁定后交易立即执行。

**Tech Stack:** Rust + parking_lot::RwLock, tokio::sync::mpsc, uuid::Uuid

---

## File Structure

### New Files

| File | Responsibility |
|------|---------------|
| `src/game/trade/data.rs` | TradeItem, TradeSession 数据结构 |
| `src/game/trade/manager.rs` | TradeManager - 交易管理器 |
| `src/game/trade/mod.rs` | 模块入口 |
| `src/protocol/trade_packets.rs` | 交易数据包结构体 |
| `tests/trade_test.rs` | 交易系统测试 |

### Modified Files

| File | Changes |
|------|---------|
| `src/game/mod.rs` | 添加 trade 模块 |
| `src/protocol/mod.rs` | 添加 trade_packets 子模块 |
| `src/game/map/map_server.rs` | 添加交易数据包处理 |
| `src/network/packet.rs` | 新增交易 packet ID 常量 |

---

## Packet IDs

| ID | 名称 | 方向 | 说明 |
|----|------|------|------|
| 0x00E4 | CZTradeRequest | C→S | 请求交易 |
| 0x00E6 | CZTradeAck | C→S | 接受/拒绝交易 |
| 0x00B0 | CZTradeAddItem | C→S | 添加物品到交易栏 |
| 0x00B1 | CZTradeAddZeny | C→S | 添加 Zeny |
| 0x00EF | CZTradeLock | C→S | 锁定交易 |
| 0x00E5 | ZCTradeRequest | S→C | 交易请求通知 |
| 0x00E7 | ZCTradeAck | S→C | 交易接受确认 |
| 0x00E8 | ZCTradeAddItem | S→C | 对方添加物品通知 |
| 0x00E9 | ZCTradeAddZeny | S→C | 对方添加 Zeny 通知 |
| 0x00EC | ZCTradeLock | S→C | 对方锁定通知 |
| 0x00F0 | ZCTradeCommit | S→C | 交易成功 |
| 0x00F1 | ZCTradeCancel | S→C | 交易取消 |

---

### Task 1: Trade 数据结构定义

**Files:**
- Create: `src/game/trade/data.rs`
- Test: `tests/trade_test.rs`

- [ ] **Step 1: Write the failing test**

Create `tests/trade_test.rs`:

```rust
use deviruchi::game::trade::data::*;
use uuid::Uuid;

#[test]
fn test_trade_item_new() {
    let item = TradeItem::new(1, 501, 10, false, 0, [0; 4]);
    assert_eq!(item.index, 1);
    assert_eq!(item.item_id, 501);
    assert_eq!(item.amount, 10);
    assert!(!item.identified);
    assert_eq!(item.refine, 0);
    assert_eq!(item.cards, [0; 4]);
}

#[test]
fn test_trade_session_new() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();
    
    let session = TradeSession::new(player1, player2);
    
    assert_eq!(session.player1_id, player1);
    assert_eq!(session.player2_id, player2);
    assert_eq!(session.state, TradeState::Requesting);
    assert_eq!(session.player1_zeny, 0);
    assert_eq!(session.player2_zeny, 0);
    assert!(!session.player1_locked);
    assert!(!session.player2_locked);
}

#[test]
fn test_trade_session_add_item() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();
    
    let mut session = TradeSession::new(player1, player2);
    session.state = TradeState::Trading;
    
    let item = TradeItem::new(0, 501, 5, true, 0, [0; 4]);
    assert!(session.add_item(player1, item));
    
    assert_eq!(session.player1_items.len(), 1);
    assert_eq!(session.player1_items[0].item_id, 501);
    assert_eq!(session.player1_items[0].amount, 5);
}

#[test]
fn test_trade_session_add_zeny() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();
    
    let mut session = TradeSession::new(player1, player2);
    session.state = TradeState::Trading;
    
    assert!(session.add_zeny(player1, 1000));
    assert_eq!(session.player1_zeny, 1000);
}

#[test]
fn test_trade_session_lock() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();
    
    let mut session = TradeSession::new(player1, player2);
    session.state = TradeState::Trading;
    
    assert!(session.lock(player1));
    assert!(session.player1_locked);
    assert!(!session.player2_locked);
    assert!(!session.is_fully_locked());
    
    assert!(session.lock(player2));
    assert!(session.player2_locked);
    assert!(session.is_fully_locked());
}

#[test]
fn test_trade_session_cancel() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();
    
    let mut session = TradeSession::new(player1, player2);
    session.state = TradeState::Trading;
    
    session.cancel();
    assert_eq!(session.state, TradeState::Cancelled);
}

#[test]
fn test_trade_session_get_partner_items() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();
    
    let mut session = TradeSession::new(player1, player2);
    session.state = TradeState::Trading;
    
    // Player1 添加物品
    let item = TradeItem::new(0, 501, 5, true, 0, [0; 4]);
    session.add_item(player1, item);
    
    // Player1 查看对方物品（应该是 player2 的物品，现在为空）
    let partner_items = session.get_partner_items(player1);
    assert!(partner_items.is_empty());
    
    // Player2 添加物品
    let item2 = TradeItem::new(0, 502, 3, true, 0, [0; 4]);
    session.add_item(player2, item2);
    
    // Player1 查看对方物品
    let partner_items = session.get_partner_items(player1);
    assert_eq!(partner_items.len(), 1);
    assert_eq!(partner_items[0].item_id, 502);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test trade_test 2>&1`
Expected: FAIL — Trade 相关类型未定义

- [ ] **Step 3: Write minimal implementation**

Create `src/game/trade/data.rs`:

```rust
use uuid::Uuid;

/// 交易物品
#[derive(Debug, Clone)]
pub struct TradeItem {
    pub index: u16,      // 背包索引
    pub item_id: u16,    // 物品ID
    pub amount: u16,     // 数量
    pub identified: bool, // 是否已鉴定
    pub refine: u8,      // 精炼等级
    pub cards: [u16; 4], // 卡片槽
}

impl TradeItem {
    pub fn new(index: u16, item_id: u16, amount: u16, identified: bool, refine: u8, cards: [u16; 4]) -> Self {
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
    Requesting,  // 请求中
    Trading,     // 交易中
    Locked,      // 已锁定
    Completed,   // 已完成
    Cancelled,   // 已取消
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test trade_test 2>&1`
Expected: PASS - 7 tests passing

- [ ] **Step 5: Commit**

```bash
git add src/game/trade/data.rs tests/trade_test.rs
git commit -m "feat: add Trade data structures with state machine"
```

---

### Task 2: TradeManager 交易管理器

**Files:**
- Create: `src/game/trade/manager.rs`
- Modify: `src/game/trade/mod.rs`
- Test: `tests/trade_test.rs` (添加新测试)

- [ ] **Step 1: Write the failing test**

在 `tests/trade_test.rs` 添加：

```rust
use std::sync::Arc;
use deviruchi::game::trade::manager::TradeManager;

#[test]
fn test_trade_manager_create_request() {
    let manager = Arc::new(TradeManager::new());
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();
    
    let trade_id = manager.create_request(player1, player2);
    assert!(trade_id.is_some());
    
    // 检查双方都有交易
    assert!(manager.has_active_trade(player1));
    assert!(manager.has_active_trade(player2));
    
    // 获取交易ID
    assert_eq!(manager.get_trade_id(player1), trade_id);
    assert_eq!(manager.get_trade_id(player2), trade_id);
}

#[test]
fn test_trade_manager_accept() {
    let manager = Arc::new(TradeManager::new());
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();
    
    let trade_id = manager.create_request(player1, player2).unwrap();
    
    // 接受交易
    assert!(manager.accept_trade(trade_id, player2));
    
    // 检查状态
    let session = manager.get_session(trade_id).unwrap();
    let session = session.read();
    assert_eq!(session.state, TradeState::Trading);
}

#[test]
fn test_trade_manager_decline() {
    let manager = Arc::new(TradeManager::new());
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();
    
    let trade_id = manager.create_request(player1, player2).unwrap();
    
    // 拒绝交易
    assert!(manager.decline_trade(trade_id, player2));
    
    // 检查状态
    let session = manager.get_session(trade_id).unwrap();
    let session = session.read();
    assert_eq!(session.state, TradeState::Cancelled);
}

#[test]
fn test_trade_manager_cancel() {
    let manager = Arc::new(TradeManager::new());
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();
    
    let trade_id = manager.create_request(player1, player2).unwrap();
    
    // 取消交易
    assert!(manager.cancel_trade(trade_id, player1));
    
    // 检查状态
    let session = manager.get_session(trade_id).unwrap();
    let session = session.read();
    assert_eq!(session.state, TradeState::Cancelled);
}

#[test]
fn test_trade_manager_complete() {
    let manager = Arc::new(TradeManager::new());
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();
    
    let trade_id = manager.create_request(player1, player2).unwrap();
    manager.accept_trade(trade_id, player2);
    
    // 双方锁定
    let session = manager.get_session(trade_id).unwrap();
    {
        let mut s = session.write();
        s.lock(player1);
        s.lock(player2);
    }
    
    // 完成交易
    assert!(manager.complete_trade(trade_id));
    
    // 检查状态
    let session = session.read();
    assert_eq!(session.state, TradeState::Completed);
}

#[test]
fn test_trade_manager_cannot_create_when_busy() {
    let manager = Arc::new(TradeManager::new());
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();
    let player3 = Uuid::new_v4();
    
    // 创建第一个交易
    assert!(manager.create_request(player1, player2).is_some());
    
    // 尝试创建第二个交易（player1 已在交易中）
    assert!(manager.create_request(player1, player3).is_none());
    
    // player3 可以与 player2 交易
    assert!(manager.create_request(player3, player2).is_none()); // player2 也忙
}

#[test]
fn test_trade_manager_remove_completed() {
    let manager = Arc::new(TradeManager::new());
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();
    
    let trade_id = manager.create_request(player1, player2).unwrap();
    
    // 完成交易
    manager.accept_trade(trade_id, player2);
    let session = manager.get_session(trade_id).unwrap();
    {
        let mut s = session.write();
        s.lock(player1);
        s.lock(player2);
    }
    manager.complete_trade(trade_id);
    
    // 移除交易
    manager.remove_trade(&trade_id);
    
    // 双方应该可以再次交易
    assert!(!manager.has_active_trade(player1));
    assert!(!manager.has_active_trade(player2));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test trade_test 2>&1`
Expected: FAIL — TradeManager 未定义

- [ ] **Step 3: Write minimal implementation**

Create `src/game/trade/manager.rs`:

```rust
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use super::data::{TradeSession, TradeState};

/// 交易管理器
pub struct TradeManager {
    trades: RwLock<HashMap<Uuid, Arc<RwLock<TradeSession>>>>,
    player_trade: RwLock<HashMap<Uuid, Uuid>>, // player_id -> trade_id
}

impl TradeManager {
    pub fn new() -> Self {
        Self {
            trades: RwLock::new(HashMap::new()),
            player_trade: RwLock::new(HashMap::new()),
        }
    }

    /// 创建交易请求
    pub fn create_request(&self, requester: Uuid, target: Uuid) -> Option<Uuid> {
        // 检查双方是否已有交易
        let player_trade = self.player_trade.read();
        if player_trade.contains_key(&requester) || player_trade.contains_key(&target) {
            return None;
        }
        drop(player_trade);

        let session = TradeSession::new(requester, target);
        let trade_id = session.id;
        
        let mut trades = self.trades.write();
        trades.insert(trade_id, Arc::new(RwLock::new(session)));
        
        // 记录玩家交易关系
        let mut player_trade = self.player_trade.write();
        player_trade.insert(requester, trade_id);
        player_trade.insert(target, trade_id);
        
        Some(trade_id)
    }

    /// 获取交易会话
    pub fn get_session(&self, trade_id: Uuid) -> Option<Arc<RwLock<TradeSession>>> {
        let trades = self.trades.read();
        trades.get(&trade_id).cloned()
    }

    /// 获取玩家的交易ID
    pub fn get_trade_id(&self, player_id: Uuid) -> Option<Uuid> {
        let player_trade = self.player_trade.read();
        player_trade.get(&player_id).copied()
    }

    /// 检查玩家是否有活跃交易
    pub fn has_active_trade(&self, player_id: Uuid) -> bool {
        let player_trade = self.player_trade.read();
        player_trade.contains_key(&player_id)
    }

    /// 接受交易
    pub fn accept_trade(&self, trade_id: Uuid, player_id: Uuid) -> bool {
        let Some(session) = self.get_session(trade_id) else {
            return false;
        };

        let mut session = session.write();
        
        // 检查是否是目标玩家
        if session.player2_id != player_id {
            return false;
        }

        session.start()
    }

    /// 拒绝交易
    pub fn decline_trade(&self, trade_id: Uuid, player_id: Uuid) -> bool {
        let Some(session) = self.get_session(trade_id) else {
            return false;
        };

        let mut session = session.write();
        
        // 检查是否是目标玩家
        if session.player2_id != player_id {
            return false;
        }

        session.cancel();
        true
    }

    /// 取消交易
    pub fn cancel_trade(&self, trade_id: Uuid, player_id: Uuid) -> bool {
        let Some(session) = self.get_session(trade_id) else {
            return false;
        };

        let mut session = session.write();
        
        // 检查是否是参与者
        if !session.is_participant(player_id) {
            return false;
        }

        // 只有发起者可以取消，或者任何一方在锁定前取消
        if session.state == TradeState::Requesting || session.state == TradeState::Trading {
            if !session.is_locked(player_id) || session.player1_id == player_id {
                session.cancel();
                return true;
            }
        }

        false
    }

    /// 完成交易
    pub fn complete_trade(&self, trade_id: Uuid) -> bool {
        let Some(session) = self.get_session(trade_id) else {
            return false;
        };

        let mut session = session.write();
        session.complete()
    }

    /// 移除交易
    pub fn remove_trade(&self, trade_id: &Uuid) {
        let mut trades = self.trades.write();
        let Some(session) = trades.remove(trade_id) else {
            return;
        };

        // 移除玩家交易关系
        let session = session.read();
        let mut player_trade = self.player_trade.write();
        player_trade.remove(&session.player1_id);
        player_trade.remove(&session.player2_id);
    }

    /// 获取交易数量
    pub fn trade_count(&self) -> usize {
        let trades = self.trades.read();
        trades.len()
    }

    /// 清理已完成的交易
    pub fn cleanup_completed(&self) -> usize {
        let mut trades = self.trades.write();
        let to_remove: Vec<_> = trades
            .iter()
            .filter(|(_, s)| {
                let s = s.read();
                s.state == TradeState::Completed || s.state == TradeState::Cancelled
            })
            .map(|(id, _)| *id)
            .collect();

        for trade_id in &to_remove {
            if let Some(session) = trades.remove(trade_id) {
                let session = session.read();
                let mut player_trade = self.player_trade.write();
                player_trade.remove(&session.player1_id);
                player_trade.remove(&session.player2_id);
            }
        }

        to_remove.len()
    }
}

impl Default for TradeManager {
    fn default() -> Self {
        Self::new()
    }
}
```

Create `src/game/trade/mod.rs`：

```rust
//! 交易系统

pub mod data;
pub mod manager;

pub use data::{TradeItem, TradeSession, TradeState};
pub use manager::TradeManager;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test trade_test 2>&1`
Expected: PASS - 13 tests passing

- [ ] **Step 5: Commit**

```bash
git add src/game/trade/
git commit -m "feat: add TradeManager for managing trade sessions"
```

---

### Task 3: 交易数据包结构体

**Files:**
- Create: `src/protocol/trade_packets.rs`
- Modify: `src/protocol/mod.rs`

- [ ] **Step 1: Write the implementation**

Create `src/protocol/trade_packets.rs`：

```rust
use crate::protocol::packet_builder::PacketBuilder;

// ========== Client -> Server ==========

/// 请求交易 (0x00E4)
pub struct CZTradeRequest {
    pub target_account_id: u32,
}

impl CZTradeRequest {
    pub fn from_packet(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        Some(Self {
            target_account_id: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
        })
    }
}

/// 接受/拒绝交易 (0x00E6)
pub struct CZTradeAck {
    pub accept: bool,
}

impl CZTradeAck {
    pub fn from_packet(data: &[u8]) -> Option<Self> {
        if data.len() < 1 {
            return None;
        }
        Some(Self {
            accept: data[0] != 0,
        })
    }
}

/// 添加物品到交易 (0x00B0)
pub struct CZTradeAddItem {
    pub inventory_index: u16,
    pub amount: u32,
}

impl CZTradeAddItem {
    pub fn from_packet(data: &[u8]) -> Option<Self> {
        if data.len() < 6 {
            return None;
        }
        Some(Self {
            inventory_index: u16::from_le_bytes([data[0], data[1]]),
            amount: u32::from_le_bytes([data[2], data[3], data[4], data[5]]),
        })
    }
}

/// 添加 Zeny (0x00B1)
pub struct CZTradeAddZeny {
    pub amount: u32,
}

impl CZTradeAddZeny {
    pub fn from_packet(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        Some(Self {
            amount: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
        })
    }
}

/// 锁定交易 (0x00EF)
pub struct CZTradeLock;

impl CZTradeLock {
    pub fn from_packet(_data: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

// ========== Server -> Client ==========

/// 交易请求通知 (0x00E5)
pub struct ZCTradeRequest {
    pub requester_id: u32,
    pub requester_name: String,
}

impl ZCTradeRequest {
    pub fn to_packet(&self) -> Vec<u8> {
        let mut builder = PacketBuilder::new(0x00E5);
        builder.write_u32(self.requester_id);
        builder.write_string(&self.requester_name, 24);
        builder.build()
    }
}

/// 交易接受确认 (0x00E7)
pub struct ZCTradeAck {
    pub accept: bool,
}

impl ZCTradeAck {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x00E7)
            .write_u8(if self.accept { 1 } else { 0 })
            .build()
    }
}

/// 对方添加物品通知 (0x00E8)
pub struct ZCTradeAddItem {
    pub amount: u32,
    pub item_id: u16,
    pub identified: bool,
    pub damaged: bool,
    pub refine: u8,
    pub cards: [u16; 4],
}

impl ZCTradeAddItem {
    pub fn to_packet(&self) -> Vec<u8> {
        let mut builder = PacketBuilder::new(0x00E8);
        builder.write_u32(self.amount);
        builder.write_u16(self.item_id);
        builder.write_u8(if self.identified { 1 } else { 0 });
        builder.write_u8(if self.damaged { 1 } else { 0 });
        builder.write_u8(self.refine);
        for card in &self.cards {
            builder.write_u16(*card);
        }
        builder.build()
    }
}

/// 对方添加 Zeny 通知 (0x00E9)
pub struct ZCTradeAddZeny {
    pub amount: u32,
}

impl ZCTradeAddZeny {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x00E9)
            .write_u32(self.amount)
            .build()
    }
}

/// 对方锁定通知 (0x00EC)
pub struct ZCTradeLock;

impl ZCTradeLock {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x00EC).build()
    }
}

/// 交易成功 (0x00F0)
pub struct ZCTradeCommit;

impl ZCTradeCommit {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x00F0).build()
    }
}

/// 交易取消 (0x00F1)
pub struct ZCTradeCancel {
    pub reason: u8, // 0 = 对方取消, 1 = 拒绝, 2 = 其他原因
}

impl ZCTradeCancel {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x00F1)
            .write_u8(self.reason)
            .build()
    }
}
```

修改 `src/protocol/mod.rs`：

```rust
pub mod char_packets;
pub mod login_packets;
pub mod map_packets;
pub mod packet_builder;
pub mod party_packets;
pub mod storage_packets;
pub mod guild_packets;
pub mod trade_packets;  // 添加这一行
```

- [ ] **Step 2: Run test to verify it compiles**

Run: `cargo check 2>&1`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/protocol/trade_packets.rs src/protocol/mod.rs
git commit -m "feat: add trade packet structures"
```

---

### Task 4-7: MapServer 集成、Core 模块集成、全量编译验证

（与前面计划类似，省略详细步骤）

---

## Self-Review

**1. Spec coverage:**
- ✅ Trade 数据结构（状态机）
- ✅ TradeManager 管理器
- ✅ 数据包结构
- ⚠️ MapServer 处理（待实现）
- ⚠️ 实际物品/Zeny 转移（需 Inventory/Player 支持）

**2. 与 rathena 对比：**
- ✅ 基础交易流程
- ✅ 锁定机制
- ✅ 双方确认
- ⚠️ 交易税（可选功能）
- ⚠️ 交易日志（安全功能）

---

Plan complete and saved to `docs/superpowers/plans/2026-05-02-trade-system-plan.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
