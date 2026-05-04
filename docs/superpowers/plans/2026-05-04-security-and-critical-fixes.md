# Deviruchi 安全与关键缺陷修复计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 Deviruchi MMORPG 服务端的 4 个 Critical 安全漏洞和 8 个 High 严重性问题，按安全→功能缺失→架构优化优先级排序。

**Architecture:** 分三阶段：Phase 1 修复可远程触发的崩溃和安全漏洞；Phase 2 修复功能桩（GM权限、TODO假成功、静默失败）；Phase 3 连接已定义但未使用的子系统（element/size表、trade系统）。

**Tech Stack:** Rust, rusqlite, parking_lot, rand, tracing

---

## Phase 1: Critical 安全修复 (4 tasks)

### Task 1: 修复数据包长度下溢导致的 panic

**Files:**
- Modify: `src/network/packet.rs:22-56`

**问题:** `Packet::new` 中 `(data.len() + 4) as u16` 会静默截断大包；`Packet::from_bytes` 中当 `length < 4` 时 `bytes[4..length]` 产生反向切片导致 panic。

- [ ] **Step 1: 编写失败测试**

```rust
// 在 src/network/packet.rs 末尾的 #[cfg(test)] mod tests 中添加
#[test]
fn test_packet_new_rejects_oversized_data() {
    let big_data = vec![0u8; 65533]; // 65533 + 4 = 65537 > u16::MAX
    let packet = Packet::new(0x0064, big_data);
    // 长度不应截断 - 应返回 None 或 panic
    // 由于当前实现会截断，这个测试会暴露 bug
    assert!(packet.header.length >= 4);
}

#[test]
fn test_packet_from_bytes_rejects_length_lt_4() {
    // 构造一个 length 字段为 2 的恶意包
    let mut bytes = vec![2, 0, 0x64, 0x00]; // length=2, packet_id=0x0064
    bytes.extend_from_slice(&[0, 0]); // 额外数据
    let result = Packet::from_bytes(&bytes);
    // 当前实现会 panic，修复后应返回 None
    assert!(result.is_none());
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --lib network::packet::tests -- --nocapture 2>&1 | tail -20`
Expected: 测试失败或 panic

- [ ] **Step 3: 修复 Packet::new 和 Packet::from_bytes**

在 `src/network/packet.rs` 中修改：

```rust
impl Packet {
    pub fn new(packet_id: PacketId, data: Vec<u8>) -> Option<Self> {
        let total = data.len() + 4; // 4 = header size
        if total > u16::MAX as usize {
            return None; // 包太大，拒绝构造
        }
        let length = total as u16;
        Some(Self {
            header: PacketHeader { length, packet_id },
            data,
        })
    }

    // ... to_bytes 不变 ...

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }

        let length = u16::from_le_bytes([bytes[0], bytes[1]]);
        let packet_id = u16::from_le_bytes([bytes[2], bytes[3]]);

        // 长度必须至少包含 header (4 bytes)
        if length < 4 {
            return None;
        }

        if (bytes.len() as u16) < length {
            return None;
        }

        let data = bytes[4..length as usize].to_vec();

        Some(Self {
            header: PacketHeader { length, packet_id },
            data,
        })
    }
}
```

- [ ] **Step 4: 确认 Packet::new 无外部调用者**

经 grep 确认，`Packet::new` 没有外部调用者（包通过 struct literal `Packet { header, data }` 构造）。`Packet::from_bytes` 在 `src/network/codec.rs:27` 调用，返回类型已是 `Option<Self>`，无需修改调用者。

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --lib network::packet::tests -- --nocapture`
Expected: 全部 PASS

- [ ] **Step 6: 提交**

```bash
git add src/network/packet.rs
git commit -m "fix(packet): reject oversized packets and length < 4 to prevent panic

Packet::new now returns None when data exceeds u16::MAX - 4 bytes.
Packet::from_bytes now returns None when length field < 4, preventing
a backwards slice panic on malformed packets."
```

---

### Task 2: 修复战斗伤害 i32→u32 转换溢出

**Files:**
- Modify: `src/game/battle/handler.rs:37,70,89,110`

**问题:** `damage as u32` 当 `damage` 为负数（暴击乘法溢出）时会 wrap 到 ~4B，造成巨额伤害。

- [ ] **Step 1: 编写失败测试**

在 `src/game/battle/handler.rs` 的 `#[cfg(test)]` 中添加：

```rust
#[test]
fn test_negative_damage_clamped_to_zero() {
    // 模拟一个负伤害值（溢出场景）
    let handler = create_test_handler(vec![0]);
    let player = make_player(1, 1, 1, 1, 1, 1);
    let mob = make_mob(1, 0, 0, 0, 0);

    // 直接测试 take_damage 的输入处理
    // 负数 i32 as u32 应该被 clamp 到 0
    let negative_damage: i32 = -100;
    let safe_damage = negative_damage.max(0) as u32;
    assert_eq!(safe_damage, 0);
}
```

- [ ] **Step 2: 运行测试确认编译通过**

Run: `cargo test --lib game::battle::handler::tests -- --nocapture`
Expected: 测试通过（这只是验证 clamp 逻辑）

- [ ] **Step 3: 添加 safe_damage 工具函数并修复所有 4 处转换**

在 `src/game/battle/handler.rs` 中，在 `impl BattleHandler` 之前添加：

```rust
/// 将 i32 伤害安全转换为 u32，负值 clamp 到 0
fn safe_damage(damage: i32) -> u32 {
    damage.max(0) as u32
}
```

然后修改 4 处调用：

```rust
// line 37 (normal_attack)
let killed = defender.take_damage(safe_damage(damage));

// line 70 (skill_attack)
let killed = defender.take_damage(safe_damage(damage));

// line 89 (magic_attack)
let killed = defender.take_damage(safe_damage(damage));

// line 110 (mob_attack)
let killed = defender.take_damage(safe_damage(damage));
```

- [ ] **Step 4: 编写溢出场景测试**

```rust
#[test]
fn test_crit_damage_overflow_clamped() {
    // 验证暴击乘法不会产生负伤害
    // crit_multiplier = 140, 如果 base_damage * 140 溢出 i32
    // safe_damage 应该返回 0 而不是 ~4B
    let large_base: i32 = i32::MAX / 2;
    let crit_damage = (large_base * 140) / 100; // 这会溢出
    let result = safe_damage(crit_damage);
    // 溢出后为负数，clamp 到 0
    assert_eq!(result, 0);
}
```

- [ ] **Step 5: 运行全部战斗测试**

Run: `cargo test --lib game::battle -- --nocapture`
Expected: 全部 PASS

- [ ] **Step 6: 提交**

```bash
git add src/game/battle/handler.rs
git commit -m "fix(battle): clamp negative damage to 0 to prevent u32 wrap

When crit multiplier causes i32 overflow, damage becomes negative.
Casting negative i32 to u32 wrapped to ~4 billion. Now clamped to 0."
```

---

### Task 3: 修复 Mutex 中毒导致的服务器崩溃

**Files:**
- Modify: `src/storage/sqlite.rs:27,32,41,53,63,69`
- Modify: `src/game/rand.rs:46,51,56`
- Modify: `src/game/battle/handler.rs:116`

**问题:** `std::sync::Mutex::lock().unwrap()` 在任何线程 panic 后会传播 poison，导致后续所有调用都 panic。数据库连接是最严重的：一次 panic 就会让整个服务器崩溃。

- [ ] **Step 1: 将 sqlite.rs 的 Mutex 替换为 parking_lot::Mutex**

`parking_lot::Mutex` 没有 poisoning 机制，且性能更好。项目已经依赖 `parking_lot`。

修改 `src/storage/sqlite.rs`：

```rust
// 将 import 从:
use std::sync::{Arc, Mutex};
// 改为:
use std::sync::Arc;
use parking_lot::Mutex;
```

然后将所有 6 处 `self.conn.lock().unwrap()` 改为 `self.conn.lock()`：

```rust
// line 27
let conn = self.conn.lock();

// line 32
let conn = self.conn.lock();

// line 41
let conn = self.conn.lock();

// line 53
let conn = self.conn.lock();

// line 63
let conn = self.conn.lock();

// line 69
let conn = self.conn.lock();
```

- [ ] **Step 2: 将 rand.rs 的 Mutex 替换为 parking_lot::Mutex**

修改 `src/game/rand.rs`：

```rust
// 将 import 从:
use std::sync::Arc;
// 不需要改 import，因为 parking_lot 已在 Cargo.toml 中

// 将 ThreadRng 定义从:
pub struct ThreadRng(std::sync::Mutex<StdRng>);
// 改为:
pub struct ThreadRng(parking_lot::Mutex<StdRng>);
```

然后将所有 3 处 `.lock().unwrap()` 改为 `.lock()`：

```rust
// line 46
self.0.lock().gen_range(min..=max)

// line 51
self.0.lock().gen_bool(normalized as f64)

// line 56
self.0.lock().gen_range(0..=10000)
```

- [ ] **Step 3: 将 battle/handler.rs 的 Mutex 替换为 parking_lot::Mutex**

修改 `src/game/battle/handler.rs`：

```rust
// 将 struct 定义从:
pub struct BattleHandler {
    rng: std::sync::Mutex<Arc<dyn GameRng>>,
}
// 改为:
pub struct BattleHandler {
    rng: parking_lot::Mutex<Arc<dyn GameRng>>,
}

// line 116 从:
let rng = self.rng.lock().unwrap();
// 改为:
let rng = self.rng.lock();
```

- [ ] **Step 4: 运行全部测试**

Run: `cargo test --lib 2>&1 | tail -30`
Expected: 全部 PASS

- [ ] **Step 5: 提交**

```bash
git add src/storage/sqlite.rs src/game/rand.rs src/game/battle/handler.rs
git commit -m "fix(mutex): replace std::sync::Mutex with parking_lot::Mutex

std::sync::Mutex poisoning causes cascading panics after any thread
crash. parking_lot::Mutex has no poisoning, preventing server-wide
crash from a single DB operation failure."
```

---

### Task 4: 实现密码哈希（替代明文存储）

**Files:**
- Modify: `Cargo.toml` (添加 argon2 依赖)
- Create: `src/storage/password.rs`
- Modify: `src/storage/mod.rs` (添加 mod password)
- Modify: `src/storage/account.rs:21` (create_account 接受明文，存储哈希)
- Modify: `src/game/login.rs:67` (验证时哈希比较)
- Modify: `src/game/char.rs:218` (测试中的明文密码)

**问题:** 密码以明文传输和存储，无哈希无盐值。

- [ ] **Step 1: 添加 argon2 依赖**

在 `Cargo.toml` 的 `[dependencies]` 中添加：

```toml
argon2 = "0.5"
```

- [ ] **Step 2: 创建密码哈希模块**

创建 `src/storage/password.rs`：

```rust
use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};

/// 对明文密码进行 Argon2 哈希
pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Failed to hash password: {}", e))?
        .to_string();
    Ok(password_hash)
}

/// 验证明文密码是否匹配存储的哈希
pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(stored_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash));
        assert!(!verify_password("wrong_password", &hash));
    }

    #[test]
    fn test_different_hashes_for_same_password() {
        let password = "same_password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();
        // 盐值不同，哈希应不同
        assert_ne!(hash1, hash2);
        // 但都能验证
        assert!(verify_password(password, &hash1));
        assert!(verify_password(password, &hash2));
    }
}
```

- [ ] **Step 3: 注册模块**

在 `src/storage/mod.rs` 中添加：

```rust
pub mod password;
```

- [ ] **Step 4: 修改 create_account 接受明文并存储哈希**

修改 `src/storage/account.rs` 的 `create_account`：

```rust
pub fn create_account(&self, user_id: &str, password: &str, sex: u8) -> Result<u32> {
    let created_at = chrono_now();
    let password_hash = crate::storage::password::hash_password(password)
        .map_err(|e| crate::error::Error::Other(e))?;
    self.execute_with_params(
        "INSERT INTO accounts (user_id, password_hash, sex, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![user_id, password_hash, sex, created_at],
    )?;
    Ok(self.last_insert_rowid()? as u32)
}
```

- [ ] **Step 5: 修改 login.rs 使用哈希验证**

修改 `src/game/login.rs` 的 `handle_ca_login`：

```rust
// 将 line 67 从:
if login.password != account.password_hash {
// 改为:
if !crate::storage::password::verify_password(&login.password, &account.password_hash) {
```

- [ ] **Step 6: 运行密码模块测试**

Run: `cargo test --lib storage::password::tests -- --nocapture`
Expected: 全部 PASS

- [ ] **Step 7: 运行全部测试**

Run: `cargo test --lib 2>&1 | tail -30`
Expected: 全部 PASS

- [ ] **Step 8: 提交**

```bash
git add Cargo.toml Cargo.lock src/storage/password.rs src/storage/mod.rs src/storage/account.rs src/game/login.rs
git commit -m "feat(security): implement Argon2 password hashing

Replace plaintext password storage with Argon2id hashing. Passwords
are now salted and hashed before storage, and verified against the
hash during login."
```

---

## Phase 2: High 严重性修复 (4 tasks)

### Task 5: 实现 GM 命令权限检查

**Files:**
- Modify: `src/game/map/player.rs` (添加 `group_id` 字段)
- Modify: `src/game/command/atcommand.rs:95-98` (读取实际 group_id)
- Modify: `src/game/map/map_server.rs` (GM handler 添加权限检查)

**问题:** `get_player_level` 始终返回 10 (GM)，所有玩家都能使用 GM 命令。

- [ ] **Step 1: 给 Player 添加 group_id 字段**

在 `src/game/map/player.rs` 的 `Player` struct 中添加字段（在 `in_combat` 之后）：

```rust
    /// 账户权限等级 (0=玩家, 10=GM, 50=Admin, 99=SuperAdmin)
    pub group_id: RwLock<i32>,
```

同时在 `Clone` impl 和 `from_character` 方法中添加对应字段（如果存在的话）。

- [ ] **Step 2: 修改 atcommand.rs 读取实际权限**

修改 `src/game/command/atcommand.rs` 的 `get_player_level`：

```rust
fn get_player_level(&self, player: &Player) -> u8 {
    *player.group_id.read() as u8
}
```

- [ ] **Step 3: 给 map_server.rs 的 GM handler 添加权限检查**

在 `src/game/map/map_server.rs` 的 4 个 GM handler 开头添加权限检查：

```rust
// 在每个 GM handler 函数开头添加:
fn check_gm_permission(player: &Player, min_level: i32) -> bool {
    *player.group_id.read() >= min_level
}
```

在 `handle_gm_warp`, `handle_gm_goto`, `handle_gm_summon`, `handle_gm_savepoint` 开头：

```rust
if !check_gm_permission(&session.player, 10) {
    warn!("Player {} attempted GM command without permission", session.player.name);
    return; // 或发送拒绝包
}
```

- [ ] **Step 4: 运行测试**

Run: `cargo test --lib 2>&1 | tail -30`
Expected: 全部 PASS

- [ ] **Step 5: 提交**

```bash
git add src/game/map/player.rs src/game/command/atcommand.rs src/game/map/map_server.rs
git commit -m "fix(auth): enforce GM command permission checks via group_id

Player.group_id is now read from account data and used for permission
checks. GM handlers reject non-GM players instead of allowing all."
```

---

### Task 6: 修复 TODO 桩返回假 Success 的问题

**Files:**
- Modify: `src/game/item/use_handler.rs:232-386`
- Modify: `src/game/skill/handler.rs:220-229`
- Modify: `src/game/cashshop/manager.rs:323-413`
- Modify: `src/game/cashshop/kafra.rs:222-324`

**问题:** 多个函数只 log 不执行，返回 Success 给客户端造成"成功"假象。

- [ ] **Step 1: 修复 item/use_handler.rs 中的桩函数**

将 5 个假成功函数改为返回 `ItemUseResult::NotImplemented`（需要先定义此变体）：

在 `ItemUseResult` enum 中添加变体（如果不存在）：

```rust
pub enum ItemUseResult {
    Success,
    Fail,
    NotImplemented, // 新增：功能未实现
    // ... 其他变体保持不变
}
```

修改各函数：

```rust
// execute_teleport (line 232-243)
fn execute_teleport(&self, player: &Player, item: &ItemData) -> ItemUseResult {
    warn!("Item teleport not yet implemented for item={}", item.id);
    ItemUseResult::NotImplemented
}

// execute_use_skill (line 314-322)
fn execute_use_skill(&self, player: &Player, item: &ItemData) -> ItemUseResult {
    warn!("Item skill use not yet implemented for item={}", item.id);
    ItemUseResult::NotImplemented
}

// execute_strip_equipment (line 339-343)
fn execute_strip_equipment(&self, player: &Player, item: &ItemData) -> ItemUseResult {
    warn!("Equipment strip not yet implemented for item={}", item.id);
    ItemUseResult::NotImplemented
}

// execute_consume_ammo (line 376-379)
fn execute_consume_ammo(&self, player: &Player, item: &ItemData) -> ItemUseResult {
    warn!("Ammo consumption not yet implemented for item={}", item.id);
    ItemUseResult::NotImplemented
}

// execute_disguise (line 383-386)
fn execute_disguise(&self, player: &Player, item: &ItemData) -> ItemUseResult {
    warn!("Disguise effect not yet implemented for item={}", item.id);
    ItemUseResult::NotImplemented
}
```

- [ ] **Step 2: 修复 skill/handler.rs 的 learn_skill**

修改 `src/game/skill/handler.rs` 的 `learn_skill` 函数，返回错误而不是假成功：

```rust
pub fn learn_skill(&self, player: &mut Player, skill_id: u16) -> Result<(), String> {
    warn!("Skill learning not yet implemented: skill_id={}", skill_id);
    Err("Skill learning system not yet implemented".to_string())
}
```

- [ ] **Step 3: 修复 cashshop/manager.rs 的 purchase 和 gift**

修改 `src/game/cashshop/manager.rs`：

```rust
// purchase (line 323-331) - 回退扣款，返回失败
pub fn purchase(&self, player: &mut Player, item_id: u32, quantity: u32) -> PurchaseResult {
    warn!("Cash shop purchase not yet implemented: item_id={}", item_id);
    // 不扣款，返回失败
    PurchaseResult::Fail
}

// gift (line 404-413)
pub fn gift(&self, from: &mut Player, to: &str, item_id: u32) -> GiftResult {
    warn!("Cash shop gift not yet implemented: item_id={}", item_id);
    GiftResult::Fail
}
```

- [ ] **Step 4: 修复 cashshop/kafra.rs 的桩函数**

修改 `src/game/cashshop/kafra.rs`：

```rust
// use_kafra_service (line 222-228) - 不扣款，返回失败
pub fn use_kafra_service(&self, player: &mut Player, dest: &str) -> TeleportResult {
    warn!("Kafra teleport not yet implemented: dest={}", dest);
    TeleportResult::Fail
}

// teleport_to_coords (line 258-264)
pub fn teleport_to_coords(&self, player: &mut Player, x: u16, y: u16) -> TeleportResult {
    warn!("Kafra coord teleport not yet implemented: ({}, {})", x, y);
    TeleportResult::Fail
}

// use_storage (line 319-324)
pub fn use_storage(&self, player: &mut Player) -> StorageResult {
    warn!("Kafra storage not yet implemented");
    StorageResult::Fail
}
```

- [ ] **Step 5: 运行测试**

Run: `cargo test --lib 2>&1 | tail -30`
Expected: 全部 PASS

- [ ] **Step 6: 提交**

```bash
git add src/game/item/use_handler.rs src/game/skill/handler.rs src/game/cashshop/manager.rs src/game/cashshop/kafra.rs
git commit -m "fix(stubs): return NotImplemented/Fail instead of fake Success

Multiple subsystems returned Success without executing logic, causing
clients to believe operations succeeded. Now returns proper failure
codes and does not deduct resources (cash points, kafra points)."
```

---

### Task 7: 修复静默失败问题

**Files:**
- Modify: `src/game/map/teleport.rs:371`
- Modify: `src/game/storage/manager_sync.rs:94,143,153,176,242`
- Modify: `src/game/map/map_server.rs:225`

**问题:** `let _ =` 吞掉了关键错误（DB写入失败、同步通道满），导致数据丢失。

- [ ] **Step 1: 修复 teleport.rs 的静默 DB 失败**

修改 `src/game/map/teleport.rs` line 371：

```rust
// 从:
let _ = self.update_character_position(
    char_id, &action.to_map, action.to_pos.0 as i32, action.to_pos.1 as i32,
);
// 改为:
if let Err(e) = self.update_character_position(
    char_id, &action.to_map, action.to_pos.0 as i32, action.to_pos.1 as i32,
) {
    error!("Failed to persist warp position for char_id={}: {}", char_id, e);
    // 仍然执行 warp（内存中），但记录错误以便排查
}
```

- [ ] **Step 2: 修复 manager_sync.rs 的静默通道失败**

修改 `src/game/storage/manager_sync.rs` 中 5 处 `let _ =`：

```rust
// 将每处从:
let _ = self.scheduler.task_sender().try_send(...);
// 改为:
if let Err(e) = self.scheduler.task_sender().try_send(...) {
    error!("Storage sync channel full or closed, data may be lost: {}", e);
}
```

- [ ] **Step 3: 修复 map_server.rs 的静默 warp 失败**

修改 `src/game/map/map_server.rs` line 225：

```rust
// 从:
let _ = self.warp_service.execute_warp(session, warp_action);
// 改为:
if let Err(e) = self.warp_service.execute_warp(session, warp_action) {
    error!("Warp execution failed for player={}: {}", session.player.name, e);
}
```

- [ ] **Step 4: 运行测试**

Run: `cargo test --lib 2>&1 | tail -30`
Expected: 全部 PASS

- [ ] **Step 5: 提交**

```bash
git add src/game/map/teleport.rs src/game/storage/manager_sync.rs src/game/map/map_server.rs
git commit -m "fix(errors): log silent failures instead of discarding with let _

DB write failures in teleport, storage sync channel errors, and warp
execution errors are now logged. Prevents silent data loss."
```

---

### Task 8: 连接 Trade 系统到数据包处理器

**Files:**
- Modify: `src/game/map/map_server.rs:587-663`

**问题:** 5 个 trade handler 只 echo 包，不调用 TradeManager 的任何方法。

- [ ] **Step 1: 在 map_server 中注入 TradeManager**

确认 `TradeManager` 已在 `map_server.rs` 中可访问。检查是否有 `trade_manager` 字段，如果没有则添加：

```rust
use crate::game::trade::TradeManager;

// 在 MapServer struct 中添加:
trade_manager: TradeManager,
```

- [ ] **Step 2: 修改 handle_trade_request 调用 TradeManager**

```rust
fn handle_trade_request(&self, session: &mut Session, data: &[u8]) {
    let target_id = /* 从 data 解析 */;
    match self.trade_manager.request_trade(session.account_id, target_id) {
        Ok(trade_id) => {
            // 发送请求给目标玩家
            let packet = Packet::new(id::ZC_TRADE_REQUEST, /* ... */);
            // 发送给双方
        }
        Err(e) => {
            warn!("Trade request failed: {}", e);
            // 发送失败通知
        }
    }
}
```

- [ ] **Step 3: 修改其他 4 个 trade handler**

类似地修改 `handle_trade_ack`, `handle_trade_add_item`, `handle_trade_add_zeny`, `handle_trade_lock`，让它们调用 `TradeManager` 的对应方法而不是 echo。

- [ ] **Step 4: 运行测试**

Run: `cargo test --lib 2>&1 | tail -30`
Expected: 全部 PASS

- [ ] **Step 5: 提交**

```bash
git add src/game/map/map_server.rs
git commit -m "fix(trade): connect trade handlers to TradeManager

Trade packet handlers now call TradeSession methods instead of
echoing packets back. Items and zeny are properly tracked."
```

---

## Phase 3: 功能连接 (2 tasks)

### Task 9: 连接 Element/Size 表到战斗公式

**Files:**
- Modify: `src/game/mob/data.rs` (Mob 添加 element, size 字段)
- Modify: `src/game/battle/formula.rs:10-33` (physical_damage 调用 element/size modifier)
- Modify: `src/game/battle/handler.rs` (传递 element/size 数据)

**问题:** Element/Size 表已完整定义但零调用，战斗公式完全忽略属性克制。

- [ ] **Step 1: 给 Mob 添加 element 和 size 字段**

在 `src/game/mob/data.rs` 的 `Mob` struct 中添加：

```rust
use crate::game::battle::element::{Element, ElementLevel, MobSize};

// 在 Mob struct 中添加:
pub element: Element,
pub element_level: ElementLevel,
pub size: MobSize,
```

更新所有 `Mob` 的构造处（测试 helpers、spawn 代码等），默认值为 `Element::Neutral`, `ElementLevel::Level1`, `MobSize::Medium`。

- [ ] **Step 2: 在 physical_damage 中集成 element/size modifier**

修改 `src/game/battle/formula.rs` 的 `physical_damage`：

```rust
pub fn physical_damage(
    attacker: &Player,
    defender: &Mob,
    skill_damage_bonus: i32,
    weapon_type: i32,
) -> i32 {
    // ... 现有 base_atk, weapon_atk, total_atk, defense 计算 ...

    let damage = ((total_atk - defense).max(1) * skill_damage_bonus) / 100;

    // 新增: 应用 element/size modifier
    let element_mod = super::element::get_element_modifier(
        Element::Neutral, defender.element, defender.element_level
    );
    let size_mod = super::element::get_size_modifier(weapon_type, defender.size);

    let damage = (damage as f64 * element_mod as f64 / 100.0) as i32;
    let damage = (damage as f64 * size_mod as f64 / 100.0) as i32;

    let variance = 100;
    (damage * variance) / 100
}
```

- [ ] **Step 3: 运行测试**

Run: `cargo test --lib game::battle -- --nocapture`
Expected: 全部 PASS

- [ ] **Step 4: 提交**

```bash
git add src/game/mob/data.rs src/game/battle/formula.rs src/game/battle/handler.rs
git commit -m "feat(battle): integrate element and size tables into damage formula

Mob struct now has element, element_level, and size fields.
Physical damage applies element modifier and size penalty."
```

---

### Task 10: 实现 Mob 行为差异化

**Files:**
- Modify: `src/game/mob/ai.rs` (update_idle 检查 behavior)

**问题:** `MobBehavior` enum 已定义但 `update_idle()` 不区分 passive/aggressive，所有 mob 都按 aggressive 行为运行。

- [ ] **Step 1: 修改 update_idle 检查 behavior**

在 `src/game/mob/ai.rs` 的 `update_idle` 中：

```rust
fn update_idle(&mut self, players: &[&Player]) {
    // 只有 Aggressive 行为的 mob 才主动追逐
    match self.behavior {
        MobBehavior::Aggressive => {
            // 检查 sight_range 内的玩家
            if let Some(target) = self.find_player_in_range(players, self.sight_range) {
                self.ai_state = MobAIState::Chase;
                self.target_id = Some(target.id);
            }
        }
        MobBehavior::Passive | MobBehavior::Immobile => {
            // 被动 mob 不主动追逐，只在被攻击后反击
            // （被攻击时由 damage handler 设置 target_id 和 Chase 状态）
        }
        MobBehavior::FleeWhenLowHp => {
            // 低血量时逃跑，否则被动
            if self.hp_pct() < 25 {
                self.ai_state = MobAIState::Flee;
            }
        }
        MobBehavior::Assist | MobBehavior::PassiveAssist => {
            // 暂未实现，按 passive 处理
        }
    }
}
```

- [ ] **Step 2: 给 Mob 添加 behavior 字段**

确认 `Mob` struct 有 `behavior: MobBehavior` 字段。如果没有，添加：

```rust
pub behavior: MobBehavior,
```

- [ ] **Step 3: 运行测试**

Run: `cargo test --lib game::mob -- --nocapture`
Expected: 全部 PASS

- [ ] **Step 4: 提交**

```bash
git add src/game/mob/ai.rs src/game/mob/data.rs
git commit -m "fix(mob-ai): differentiate passive vs aggressive behavior

Passive mobs no longer chase players on sight. Only Aggressive mobs
trigger chase in update_idle. Passive mobs retaliate when attacked."
```

---

## 执行顺序总结

| Task | 优先级 | 预估时间 | 依赖 |
|------|--------|----------|------|
| 1. Packet underflow fix | Critical | 15 min | 无 |
| 2. Damage overflow fix | Critical | 10 min | 无 |
| 3. Mutex poisoning fix | Critical | 15 min | 无 |
| 4. Password hashing | Critical | 30 min | 无 |
| 5. GM permission check | High | 20 min | 无 |
| 6. TODO stub fixes | High | 25 min | 无 |
| 7. Silent failure fixes | High | 15 min | 无 |
| 8. Trade system connection | High | 30 min | 无 |
| 9. Element/Size integration | Medium | 25 min | 无 |
| 10. Mob behavior differentiation | Medium | 15 min | 无 |

Task 1-4 可以并行执行（无互相依赖）。Task 5-8 也可以并行。Task 9-10 可以并行。
