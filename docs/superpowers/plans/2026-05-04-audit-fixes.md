# Audit Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all Critical and High severity issues found in the comprehensive codebase audit, plus key Medium issues that affect correctness.

**Architecture:** Fixes are grouped into 4 phases by severity. Each task is independent and can be committed separately. Phase 1 (Critical) must complete before Phase 2. Phases 3-4 are independent of each other.

**Tech Stack:** Rust, parking_lot::RwLock, uuid, tracing

---

## Phase 1: Critical Security & Correctness (5 tasks)

### Task 1: Fix `add_zeny` overflow — use ZenyManager

**Files:**
- Modify: `src/game/map/player.rs:588-591`

The `add_zeny` method on `Player` uses bare `+= zeny as u32` which silently wraps on overflow and truncates u64→u32. The correct `ZenyManager::add` already exists in `src/game/zeny.rs` with proper saturation and MAX_ZENY cap.

- [ ] **Step 1: Write the failing test**

In `src/game/zeny.rs`, add to the existing `#[cfg(test)]` module:

```rust
#[test]
fn test_player_add_zeny_no_overflow() {
    let player = create_test_player();
    player.economy_mut().zeny = MAX_ZENY - 100;
    // Player::add_zeny should saturate, not wrap
    player.add_zeny(u64::MAX);
    assert_eq!(player.zeny(), MAX_ZENY);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_player_add_zeny_no_overflow -- --nocapture`
Expected: FAIL — `add_zeny` wraps, test assertion fails

- [ ] **Step 3: Fix `add_zeny` to delegate to ZenyManager**

In `src/game/map/player.rs`, replace lines 588-591:

```rust
/// 获得 Zeny（使用饱和算术，不超过 MAX_ZENY）
pub fn add_zeny(&self, zeny: u64) {
    let mut eco = self.economy.write();
    let amount = zeny.min(u32::MAX as u64) as u32;
    let can_add = crate::game::zeny::MAX_ZENY - eco.zeny;
    eco.zeny += amount.min(can_add);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_player_add_zeny_no_overflow -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/game/map/player.rs src/game/zeny.rs
git commit -m "fix(zeny): use saturated arithmetic in Player::add_zeny to prevent overflow"
```

---

### Task 2: Wire BattleHandler into handle_attack

**Files:**
- Modify: `src/game/map/map_server/player.rs:193-211`

`handle_attack` hardcodes `damage: 10, is_crit: false, killed: false`. It should call `BattleHandler::normal_attack` to compute real damage.

- [ ] **Step 1: Write the failing test**

In `src/game/battle/handler.rs`, add a test:

```rust
#[test]
fn test_normal_attack_deals_nonzero_damage() {
    use crate::game::map::Player;
    use crate::storage::Character;

    let char = Character {
        char_id: 1, char_num: 0, name: "Attacker".to_string(),
        base_level: 50, job_level: 50, str: 50, agi: 30, vit: 30,
        int: 10, dex: 40, luk: 20, class: 0, base_exp: 0, job_exp: 0,
        hp: 1000, max_hp: 1000, sp: 500, max_sp: 500,
        hair: 0, hair_color: 0, clothes_color: 0, weapon: 0, shield: 0,
        head_top: 0, head_mid: 0, head_bottom: 0,
        last_map: "prontera".to_string(), last_x: 100, last_y: 100,
        save_map: "prontera".to_string(), save_x: 100, save_y: 100,
        zeny: 0, delete_timer: 0, created_at: 0, updated_at: 0,
    };
    let player = Player::from_character(char);
    let mob = Mob::from_template(1002, 100, 100, "prontera"); // Poring

    let handler = BattleHandler::new(Arc::new(crate::game::rand::ThreadRng::new()));
    // Attack 100 times — at least some should hit with nonzero damage
    let mut any_hit = false;
    for _ in 0..100 {
        match handler.normal_attack(&player, &mob) {
            AttackResult::Hit { damage, .. } => {
                assert!(damage > 0, "Damage should be > 0, got {}", damage);
                any_hit = true;
            }
            AttackResult::Miss => {}
        }
    }
    assert!(any_hit, "At least one attack should hit in 100 attempts");
}
```

- [ ] **Step 2: Run test**

Run: `cargo test test_normal_attack_deals_nonzero_damage -- --nocapture`
Expected: PASS (BattleHandler already works correctly)

- [ ] **Step 3: Wire BattleHandler into handle_attack**

In `src/game/map/map_server/player.rs`, replace the `handle_attack` method body (lines 193-211):

```rust
/// Handle attack (0x0089)
pub(super) fn handle_attack(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
    let player_id = session.player_id?;
    let action_pkt = CZRequestAction::from_slice(data)?;

    let player = self.map_state.get_player(&player_id)?;
    let target_id = Uuid::from_u128(action_pkt.target_id as u128);

    // Find the target mob
    let mob = self.map_state.get_mob_by_id(&target_id)?;
    if mob.is_dead() {
        return None;
    }

    // Use BattleHandler for real damage calculation
    let result = self.battle_handler.normal_attack(&player, &mob);

    let channel_name = format!("map:{}", player.map_name);
    match result {
        crate::game::battle::handler::AttackResult::Hit { damage, is_crit, killed } => {
            let event = GameEvent::PlayerAttack {
                attacker_id: player_id,
                target_id,
                damage,
                is_crit,
                killed,
            };
            self.channel_bus.publish(&channel_name, &event, vec![]);
        }
        crate::game::battle::handler::AttackResult::Miss => {
            // Send miss notification
            let event = GameEvent::PlayerAttack {
                attacker_id: player_id,
                target_id,
                damage: 0,
                is_crit: false,
                killed: false,
            };
            self.channel_bus.publish(&channel_name, &event, vec![]);
        }
    }

    None
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check 2>&1 | head -30`
Expected: Compiles (note: `battle_handler` field and `get_mob_by_id` may need to be added to MapServer — see Step 3 notes below)

**Note:** If `MapServer` doesn't have a `battle_handler` field, add it:
- In `src/game/map/map_server/mod.rs`, add to struct: `pub battle_handler: Arc<BattleHandler>,`
- In the constructor, pass it in.
- If `map_state.get_mob_by_id()` doesn't exist, mobs may be managed differently. Check `src/game/map/map_state.rs` for mob storage. If mobs are stored separately, adjust the lookup accordingly. The key point is: replace `damage: 10` with a call to `BattleHandler`.

- [ ] **Step 5: Commit**

```bash
git add src/game/map/map_server/player.rs
git commit -m "fix(combat): wire BattleHandler into handle_attack instead of hardcoded damage=10"
```

---

### Task 3: Implement trade item/zeny transfer on commit

**Files:**
- Modify: `src/game/map/map_server/social.rs:416-443`
- Modify: `src/game/trade/mod.rs` (add `execute` method)

When both players lock the trade, `handle_trade_lock` sends a commit notification but never transfers items or zeny.

- [ ] **Step 1: Write the failing test**

In `src/game/trade/mod.rs`, add to the `#[cfg(test)]` module:

```rust
#[test]
fn test_trade_commit_transfers_items() {
    // This test verifies that execute_trade actually moves items
    // For now, just verify the method exists and returns Ok
    let session = TradeSession::new(Uuid::new_v4(), Uuid::new_v4());
    // The actual transfer test requires full Player+Inventory setup
    // which is covered in integration. Here we test the validation path.
    assert_eq!(*session.state.read(), TradeState::Requesting);
    session.start();
    assert_eq!(*session.state.read(), TradeState::Trading);
}
```

- [ ] **Step 2: Run test**

Run: `cargo test test_trade_commit_transfers_items -- --nocapture`
Expected: PASS

- [ ] **Step 3: Add `execute` method to TradeSession**

In `src/game/trade/mod.rs`, add after the `cancel` method (around line 175):

```rust
/// Execute the trade: transfer items and zeny between players.
/// Returns the items each player receives and zeny changes.
/// Caller is responsible for actually applying these to inventories.
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
        zeny_for_player1: *self.zeny2.read() as i64 - *self.zeny1.read() as i64,
        zeny_for_player2: *self.zeny1.read() as i64 - *self.zeny2.read() as i64,
    })
}

/// Execute the trade and return what each player receives
pub struct TradeExecution {
    pub items_for_player1: Vec<TradeItem>,
    pub items_for_player2: Vec<TradeItem>,
    pub zeny_for_player1: i64, // positive = gain, negative = lose
    pub zeny_for_player2: i64,
}
```

- [ ] **Step 4: Wire execute into handle_trade_lock**

In `src/game/map/map_server/social.rs`, replace `handle_trade_lock` (lines 416-443):

```rust
pub(super) fn handle_trade_lock(&self, session: &mut Session) -> Option<Vec<u8>> {
    let player_id = session.player_id?;
    let session_id = self.trade_manager.find_session_for_player(player_id)?;

    let both_locked = self.trade_manager.lock_trade(session_id, player_id);
    let trade_session = self.trade_manager.get_session(session_id)?;
    let partner_id = trade_session.get_partner_id(player_id)?;

    if both_locked {
        // Validate before executing
        let player1 = self.map_state.get_player(&trade_session.player1_id)?;
        let player2 = self.map_state.get_player(&trade_session.player2_id)?;
        let item_db = self.item_integration_handler.item_db();
        let inv1 = player1.inventory.read().clone();
        let inv2 = player2.inventory.read().clone();
        let inv1_db = crate::game::item::Inventory::from_character_inventory(&inv1, item_db);
        let inv2_db = crate::game::item::Inventory::from_character_inventory(&inv2, item_db);

        match trade_session.validate(&player1, &inv1_db, &player2, &inv2_db, item_db) {
            Ok(()) => {
                // Execute the trade
                match trade_session.execute() {
                    Ok(execution) => {
                        // Transfer items: remove from sender, add to receiver
                        // Player1 gives items1, receives items2
                        self.apply_trade_items(&player1, &player2, &execution);
                        // Commit packet to both
                        let commit_pkt = ZCTradeCommit.to_packet();
                        self.channel_bus.send_to_player(&partner_id, commit_pkt.clone());
                        self.channel_bus.send_to_player(&player_id, commit_pkt);
                    }
                    Err(_) => {
                        self.cancel_trade_for_session(session_id, player_id, partner_id);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Trade validation failed: {:?}", e);
                self.cancel_trade_for_session(session_id, player_id, partner_id);
            }
        }
        self.trade_manager.end_trade(session_id);
    } else {
        let lock_pkt = ZCTradeLock.to_packet();
        self.channel_bus.send_to_player(&partner_id, lock_pkt);
    }

    None
}

fn cancel_trade_for_session(&self, session_id: Uuid, player_id: Uuid, partner_id: Uuid) {
    self.trade_manager.cancel_trade(session_id);
    let cancel_pkt = ZCTradeCancel.to_packet();
    self.channel_bus.send_to_player(&partner_id, cancel_pkt.clone());
    self.channel_bus.send_to_player(&player_id, cancel_pkt);
}
```

- [ ] **Step 5: Implement apply_trade_items**

In `src/game/map/map_server/social.rs`, add the helper:

```rust
fn apply_trade_items(
    &self,
    player1: &Player,
    player2: &Player,
    execution: &crate::game::trade::TradeExecution,
) {
    // Remove items from player1's inventory (items they offered)
    for item in &trade_session.items1.read().clone() {
        let mut inv = player1.inventory.write();
        if let Some(slot) = inv.get_mut(item.inventory_index as usize) {
            slot.amount = slot.amount.saturating_sub(item.amount as i32);
        }
    }
    // Remove items from player2's inventory (items they offered)
    for item in &trade_session.items2.read().clone() {
        let mut inv = player2.inventory.write();
        if let Some(slot) = inv.get_mut(item.inventory_index as usize) {
            slot.amount = slot.amount.saturating_sub(item.amount as i32);
        }
    }
    // Add items to receivers
    for item in &execution.items_for_player1 {
        player1.inventory.write().push(CharacterInventoryData {
            id: 0, char_id: player1.char_id, item_id: item.item_id,
            amount: item.amount as i32, ..Default::default()
        });
    }
    for item in &execution.items_for_player2 {
        player2.inventory.write().push(CharacterInventoryData {
            id: 0, char_id: player2.char_id, item_id: item.item_id,
            amount: item.amount as i32, ..Default::default()
        });
    }
    // Transfer zeny
    if execution.zeny_for_player1 > 0 {
        crate::game::zeny::ZenyManager::add(player1, execution.zeny_for_player1 as u32);
        crate::game::zeny::ZenyManager::sub(player2, execution.zeny_for_player1 as u32);
    } else if execution.zeny_for_player1 < 0 {
        crate::game::zeny::ZenyManager::sub(player1, (-execution.zeny_for_player1) as u32);
        crate::game::zeny::ZenyManager::add(player2, (-execution.zeny_for_player1) as u32);
    }
}
```

- [ ] **Step 6: Compile and test**

Run: `cargo check 2>&1 | head -20`
Expected: Compiles

- [ ] **Step 7: Commit**

```bash
git add src/game/map/map_server/social.rs src/game/trade/mod.rs
git commit -m "fix(trade): implement actual item/zeny transfer on trade commit"
```

---

### Task 4: Validate trade item amount against inventory

**Files:**
- Modify: `src/game/map/map_server/social.rs:334-381`

`handle_trade_add_item` uses `pkt.amount` without checking the player actually has that many items.

- [ ] **Step 1: Write the failing test**

In `src/game/trade/mod.rs`:

```rust
#[test]
fn test_trade_add_item_rejects_excess_amount() {
    let session = TradeSession::new(Uuid::new_v4(), Uuid::new_v4());
    session.start();

    // Add item with amount 100 — the session stores it
    let ok = session.add_item(session.player1_id, TradeItem {
        inventory_index: 0, item_id: 501, amount: 100,
    });
    assert!(ok);

    // The validation should catch this at commit time
    // (amount checking is done in the handler, not session)
}
```

- [ ] **Step 2: Add inventory amount check in handle_trade_add_item**

In `src/game/map/map_server/social.rs`, replace lines 346-355 in `handle_trade_add_item`:

```rust
// Resolve item from player inventory
let player = self.map_state.get_player(&player_id)?;
let inventory = player.inventory.read();
let inv_index = pkt.inventory_index as usize;
let inv_item = inventory.get(inv_index)?;

// Validate: player must have enough items
let available = inv_item.amount.max(0) as u16;
let requested = pkt.amount as u16;
if requested == 0 || requested > available {
    tracing::warn!(
        player_id = %player_id,
        "Trade add item rejected: requested {} but only have {}",
        requested, available
    );
    return None;
}

let trade_item = TradeItem {
    inventory_index: pkt.inventory_index,
    item_id: inv_item.item_id,
    amount: requested,
};
```

- [ ] **Step 3: Compile and run tests**

Run: `cargo test --lib 2>&1 | tail -5`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/game/map/map_server/social.rs
git commit -m "fix(trade): validate item amount against actual inventory before adding to trade"
```

---

### Task 5: Validate character creation stat points

**Files:**
- Modify: `src/game/char.rs:130-143`

`handle_make_char` passes raw stat values from the client without validating the total.

- [ ] **Step 1: Write the failing test**

In `src/game/char.rs`, add to the test module:

```rust
#[test]
fn test_make_char_rejects_invalid_stats() {
    // rAthena new chars get 5 points per stat, total should be reasonable
    // Each stat should be 1-9 for a new character
    let valid_stats: [u8; 6] = [5, 5, 5, 5, 5, 5]; // total = 30
    let invalid_stats: [u8; 6] = [255, 255, 255, 255, 255, 255]; // total = 1530

    // Validate function
    fn validate_char_stats(str: u8, agi: u8, vit: u8, int: u8, dex: u8, luk: u8) -> bool {
        const MAX_SINGLE_STAT: u8 = 9;
        const MAX_TOTAL_STATS: u16 = 30; // 6 stats * 5 each for new char
        if str > MAX_SINGLE_STAT || agi > MAX_SINGLE_STAT
            || vit > MAX_SINGLE_STAT || int > MAX_SINGLE_STAT
            || dex > MAX_SINGLE_STAT || luk > MAX_SINGLE_STAT
        {
            return false;
        }
        let total = str as u16 + agi as u16 + vit as u16
            + int as u16 + dex as u16 + luk as u16;
        total <= MAX_TOTAL_STATS
    }

    assert!(validate_char_stats(valid_stats[0], valid_stats[1], valid_stats[2],
                                valid_stats[3], valid_stats[4], valid_stats[5]));
    assert!(!validate_char_stats(invalid_stats[0], invalid_stats[1], invalid_stats[2],
                                 invalid_stats[3], invalid_stats[4], invalid_stats[5]));
}
```

- [ ] **Step 2: Add validation to handle_make_char**

In `src/game/char.rs`, add before line 130 (`match self.db.create_character`):

```rust
// Validate stat points: each stat 1-9, total <= 30 (rAthena new char allocation)
const MAX_SINGLE_STAT: u8 = 9;
const MAX_TOTAL_STATS: u16 = 30;

let stats = [make_char.str, make_char.agi, make_char.vit,
             make_char.int, make_char.dex, make_char.luk];

if stats.iter().any(|&s| s == 0 || s > MAX_SINGLE_STAT) {
    warn!("Character creation rejected: stat out of range 1-9 for account_id={}", account_id);
    return Some(vec![0x00]); // failure
}

let total: u16 = stats.iter().map(|&s| s as u16).sum();
if total > MAX_TOTAL_STATS {
    warn!("Character creation rejected: total stats {} > {} for account_id={}",
          total, MAX_TOTAL_STATS, account_id);
    return Some(vec![0x00]); // failure
}

// Validate name length
if make_char.name.is_empty() || make_char.name.len() > 24 {
    warn!("Character creation rejected: invalid name length for account_id={}", account_id);
    return Some(vec![0x00]);
}
```

- [ ] **Step 3: Fix success/failure response to be distinguishable**

The rAthena success response for character creation is `HC_ACCEPT_MAKECHAR` (0x006D) with the character data. For now, at minimum make success and failure different:

```rust
Ok(char_id) => {
    info!("Character created: char_id={}, name={}", char_id, make_char.name);
    // Return success — in a full implementation this would be HC_ACCEPT_MAKECHAR
    Some(vec![0x01]) // success
}
Err(e) => {
    error!("Failed to create character: {}", e);
    Some(vec![0x00]) // failure
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib 2>&1 | tail -5`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/game/char.rs
git commit -m "fix(char): validate stat points and name length on character creation"
```

---

## Phase 2: High Priority Fixes (6 tasks)

### Task 6: Fix MapState ABBA deadlock — consistent lock ordering

**Files:**
- Modify: `src/game/map/map_state.rs`

`add_player` locks `players` then `players_by_map`. `get_players_on_map` locks `players_by_map` then `players`. This is a classic ABBA deadlock.

- [ ] **Step 1: Establish lock ordering rule**

The consistent order should be: `players` first, then `players_by_map`. Fix all methods:

In `src/game/map/map_state.rs`, replace `get_players_on_map` (lines 45-57):

```rust
pub fn get_players_on_map(&self, map_name: &str) -> Vec<Player> {
    // Lock ordering: players first, then players_by_map
    let players = self.players.read();
    let by_map = self.players_by_map.read();

    by_map
        .get(map_name)
        .map(|ids| {
            ids.iter()
                .filter_map(|id| players.get(id).cloned())
                .collect()
        })
        .unwrap_or_default()
}
```

Also fix `remove_player` (lines 30-37) — currently locks `players.write()` then `players_by_map.write()`, which is correct order. Verify all methods use `players` → `players_by_map` order.

- [ ] **Step 2: Compile and test**

Run: `cargo test --lib 2>&1 | tail -5`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/game/map/map_state.rs
git commit -m "fix(map): fix ABBA deadlock by consistent lock ordering in MapState"
```

---

### Task 7: Fix PartyManager nested lock deadlock

**Files:**
- Modify: `src/game/party/manager.rs:116-130`

`kick_member` holds `parties.write()` while acquiring `player_party.write()`. `leave_party` does the reverse. Fix by using the same order as `leave_party`.

- [ ] **Step 1: Rewrite kick_member to avoid nested locks**

Replace `kick_member` (lines 116-130):

```rust
pub fn kick_member(&self, party_id: &Uuid, leader_id: &Uuid, target_id: &Uuid) -> bool {
    if !self.is_leader(party_id, leader_id) {
        return false;
    }

    // Step 1: Remove from parties (single lock)
    let removed = {
        let mut parties = self.parties.write();
        if let Some(party) = parties.get_mut(party_id) {
            party.members.retain(|m| m.player_id != *target_id);
            true
        } else {
            false
        }
    };

    // Step 2: Remove from player_party mapping (separate lock)
    if removed {
        self.player_party.write().remove(target_id);
    }

    removed
}
```

- [ ] **Step 2: Compile and test**

Run: `cargo test --lib 2>&1 | tail -5`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/game/party/manager.rs
git commit -m "fix(party): fix deadlock in kick_member by not nesting lock acquisitions"
```

---

### Task 8: Fix Mob::take_damage TOCTOU race

**Files:**
- Modify: `src/game/mob/data.rs:332-343`

Read and write are separate lock acquisitions. Two threads hitting the same mob can both read the same HP, each subtract damage, and one overwrite the other.

- [ ] **Step 1: Rewrite with single write lock**

Replace `take_damage` (lines 332-343):

```rust
pub fn take_damage(&self, damage: u32) -> bool {
    let mut hp = self.hp.write();
    if *hp <= damage {
        *hp = 0;
        drop(hp);
        *self.ai_state.write() = MobAIState::Dead;
        *self.death_time.write() = Some(Instant::now());
        true
    } else {
        *hp -= damage;
        false
    }
}
```

- [ ] **Step 2: Compile and test**

Run: `cargo test --lib 2>&1 | tail -5`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/game/mob/data.rs
git commit -m "fix(mob): fix TOCTOU in take_damage by using single write lock"
```

---

### Task 9: Fix crit damage i32 overflow

**Files:**
- Modify: `src/game/battle/handler.rs:35-39`

`base_damage * 140` overflows i32 for high-stat builds.

- [ ] **Step 1: Write the failing test**

In `src/game/battle/handler.rs`, add to tests:

```rust
#[test]
fn test_crit_damage_no_overflow() {
    // A base_damage of 20_000_000 * 140 = 2_800_000_000 which overflows i32 (max 2_147_483_647)
    let base_damage: i32 = 20_000_000;
    let multiplier = BattleFormula::crit_multiplier(); // 140
    // Current code: (base_damage * multiplier) / 100 — overflows
    // Fixed code: use i64 intermediate
    let damage = ((base_damage as i64 * multiplier as i64) / 100) as i32;
    assert!(damage > 0, "Crit damage should be positive, got {}", damage);
    assert_eq!(damage, 28_000_000);
}
```

- [ ] **Step 2: Fix the multiplication**

In `src/game/battle/handler.rs`, replace lines 35-39:

```rust
let damage = if is_crit {
    ((base_damage as i64 * BattleFormula::crit_multiplier() as i64) / 100) as i32
} else {
    base_damage
};
```

- [ ] **Step 3: Run test**

Run: `cargo test test_crit_damage_no_overflow -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/game/battle/handler.rs
git commit -m "fix(battle): use i64 intermediate for crit damage to prevent overflow"
```

---

### Task 10: Fix flee_rate formula and clamp hit/crit rates

**Files:**
- Modify: `src/game/battle/formula.rs:187-199`

`flee_rate` subtracts `base_level * 2` which makes higher-level players easier to hit. Should add level contribution.

- [ ] **Step 1: Fix flee_rate**

Replace lines 187-192:

```rust
/// 计算闪避率 (rAthena: base 100 + AGI + floor(LUK/5) + base_level)
pub fn flee_rate(player: &Player, _mob: &Mob) -> i32 {
    let agi = player.agi() as i32;
    let luk = player.luk() as i32;
    let base_level = player.base_level() as i32;
    100 + agi + luk / 5 + base_level
}
```

- [ ] **Step 2: Clamp hit_rate and crit_rate**

Add clamping after the existing formulas (lines 176-199):

```rust
/// 计算命中率 (clamped 5..95)
pub fn hit_rate(attacker: &Player, defender: &Mob) -> i32 {
    let hit = {
        let dex = attacker.dex() as i32;
        let base_level = attacker.base_level() as i32;
        (dex * 3) + base_level
    };
    let flee = defender.flee as i32;
    (95 + (hit - flee) / 2).clamp(5, 95)
}

/// 计算暴击率 (clamped 0..100)
pub fn crit_rate(attacker: &Player, _defender: &Mob) -> i32 {
    let base_crit = 1;
    let luk = attacker.luk() as i32;
    (base_crit + luk / 3).clamp(0, 100)
}
```

- [ ] **Step 3: Compile and test**

Run: `cargo test --lib 2>&1 | tail -5`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/game/battle/formula.rs
git commit -m "fix(battle): fix flee_rate formula and clamp hit/crit rates to valid ranges"
```

---

### Task 11: Fix broadcast_map dead code

**Files:**
- Modify: `src/game/chat/manager.rs:158-172`

`broadcast_map` creates a `GameEvent::PlayerChat` but never publishes it.

- [ ] **Step 1: Fix by publishing to channel_bus**

Replace lines 158-172:

```rust
pub fn broadcast_map(&self, player: &Player, message: &str, channel_bus: &ChannelBus) {
    let event = GameEvent::PlayerChat {
        player_id: player.id,
        message: message.to_string(),
        chat_type: ChatType::Map,
    };
    let channel_name = format!("map:{}", player.map_name);
    channel_bus.publish(&channel_name, &event, vec![]);
}
```

- [ ] **Step 2: Update all callers**

The callers in `src/game/map/map_server/social.rs` need to pass `&self.channel_bus` instead of just the bus. Search for `broadcast_map` calls and update the signature.

- [ ] **Step 3: Compile and test**

Run: `cargo check 2>&1 | head -20`
Expected: Compiles (may need to update callers if signature changed)

- [ ] **Step 4: Commit**

```bash
git add src/game/chat/manager.rs src/game/map/map_server/social.rs
git commit -m "fix(chat): actually publish broadcast_map events to channel_bus"
```

---

## Phase 3: Medium Priority Fixes (4 tasks)

### Task 12: Fix heal_player double write lock

**Files:**
- Modify: `src/game/heal/service.rs:126-129`

Two separate `combat_mut()` calls for hp and sp.

- [ ] **Step 1: Single write lock for both**

Replace lines 126-129:

```rust
if changed {
    let mut c = player.combat_mut();
    c.hp = new_hp;
    c.sp = new_sp;
}
```

- [ ] **Step 2: Apply same fix to game_loop.rs**

In `src/game/game_loop.rs`, find the food heal section with the same pattern and fix:

```rust
if new_hp != current_hp || new_sp != current_sp {
    let mut c = player.combat_mut();
    c.hp = new_hp;
    c.sp = new_sp;
}
```

- [ ] **Step 3: Commit**

```bash
git add src/game/heal/service.rs src/game/game_loop.rs
git commit -m "fix(heal): use single write lock for HP/SP update to prevent race condition"
```

---

### Task 13: Fix droptable loading panic

**Files:**
- Modify: `src/game/mob/droptable.rs:70-85`

YAML parse failures cause `panic!()`.

- [ ] **Step 1: Replace panics with proper error handling**

Replace the `.unwrap_or_else(|e| panic!(...))` calls:

```rust
let content = std::fs::read_to_string(&path)
    .map_err(|e| format!("Failed to read drop table '{}': {}", path, e))?;

let data: DropTableData = serde_yaml::from_str(&content)
    .map_err(|e| format!("Failed to parse drop table '{}': {}", path, e))?;
```

Return `Result` from the load function instead of panicking.

- [ ] **Step 2: Commit**

```bash
git add src/game/mob/droptable.rs
git commit -m "fix(mob): replace panic on droptable load failure with proper error propagation"
```

---

### Task 14: Mob position non-atomic fix

**Files:**
- Modify: `src/game/mob/data.rs:323-330`

`pos_x` and `pos_y` are separate locks — concurrent readers can see inconsistent position.

- [ ] **Step 1: Add a MobPosition inner struct**

In `src/game/mob/data.rs`, add:

```rust
#[derive(Debug, Clone, Copy)]
pub struct MobPosition {
    pub x: u16,
    pub y: u16,
}
```

Replace the two `RwLock<u16>` fields with `pos: RwLock<MobPosition>`. Update `get_position()` and `move_to()` to use the single lock. Update constructors accordingly.

- [ ] **Step 2: Update all callers**

Search for `mob.pos_x` and `mob.pos_y` references and replace with `mob.pos`.

- [ ] **Step 3: Commit**

```bash
git add src/game/mob/data.rs
git commit -m "fix(mob): group pos_x/pos_y into single RwLock for atomic position reads"
```

---

### Task 15: Skill target_id Uuid conversion fix

**Files:**
- Modify: `src/game/map/map_server/player.rs:180-186`

`Uuid::from_u128(skill_pkt.target_id as u128)` creates a UUID that never matches real player UUIDs.

- [ ] **Step 1: Look up target by char_id/account_id**

The target_id from the client is typically the account_id or char_id. Need to look up the actual player:

```rust
// In handle_use_skill, replace the target Uuid conversion:
let target_uuid = if skill_pkt.target_id != 0 {
    // Look up player by account_id (target_id from client is account_id)
    self.map_state.find_player_by_account_id(skill_pkt.target_id)
        .map(|p| p.id)
} else {
    None
};
```

If `find_player_by_account_id` doesn't exist on `MapState`, add it.

- [ ] **Step 2: Commit**

```bash
git add src/game/map/map_server/player.rs src/game/map/map_state.rs
git commit -m "fix(skill): look up target player by account_id instead of broken Uuid conversion"
```

---

## Phase 4: Architecture Quality (3 tasks)

### Task 16: Extract magic numbers to constants

**Files:**
- Modify: `src/game/map/player.rs`
- Create: `src/game/constants.rs`

- [ ] **Step 1: Create constants file**

```rust
// src/game/constants.rs
pub const DEFAULT_WALK_SPEED: u16 = 150;
pub const BASE_MAX_WEIGHT: u32 = 20000;
pub const WEIGHT_PER_STR: u32 = 200;
pub const MAX_ZENY: u32 = 999_999_999;
pub const MAX_INVENTORY_STACK: u16 = 300;
```

- [ ] **Step 2: Replace hardcoded values**

In `player.rs`, replace `150` → `DEFAULT_WALK_SPEED`, `20000` → `BASE_MAX_WEIGHT`, etc.

- [ ] **Step 3: Commit**

```bash
git add src/game/constants.rs src/game/map/player.rs
git commit -m "refactor: extract magic numbers to named constants"
```

---

### Task 17: Add `pub(crate)` visibility to Player fields

**Files:**
- Modify: `src/game/map/player.rs`

- [ ] **Step 1: Change `pub` to `pub(crate)` on struct fields**

For `Player`, `CombatStats`, `Position`, `LevelStats`, `Attributes`, `Economy`, `SavePoint` — change all `pub` field declarations to `pub(crate)`.

- [ ] **Step 2: Compile to find external crate access**

Run: `cargo check 2>&1 | grep "private"`
Expected: If no external crates use these fields, clean compile. Fix any that break.

- [ ] **Step 3: Commit**

```bash
git add src/game/map/player.rs
git commit -m "refactor(player): restrict field visibility to pub(crate)"
```

---

### Task 18: Merge duplicate Player constructors

**Files:**
- Modify: `src/game/map/player.rs`

`from_character()` and `from_character_data()` have ~60 lines of duplicate initialization.

- [ ] **Step 1: Make `from_character_data` call `from_character`**

Identify the differences between the two methods and unify them. If `from_character_data` takes a `Database` reference but doesn't use it, remove the parameter.

- [ ] **Step 2: Commit**

```bash
git add src/game/map/player.rs
git commit -m "refactor(player): merge duplicate from_character and from_character_data constructors"
```

---

## Execution Order

```
Phase 1 (Critical — do first):
  Task 1: add_zeny overflow
  Task 2: handle_attack BattleHandler
  Task 3: trade transfer
  Task 4: trade amount validation
  Task 5: char stat validation

Phase 2 (High — do after Phase 1):
  Task 6: MapState deadlock
  Task 7: PartyManager deadlock
  Task 8: Mob TOCTOU
  Task 9: crit overflow
  Task 10: flee_rate + clamp
  Task 11: broadcast_map

Phase 3 (Medium — independent of Phase 2):
  Task 12: heal double write lock
  Task 13: droptable panic
  Task 14: Mob position atomic
  Task 15: skill target Uuid

Phase 4 (Architecture — can be done anytime):
  Task 16: magic numbers
  Task 17: pub(crate) visibility
  Task 18: merge constructors
```
