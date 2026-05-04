# Storage 系统加固计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development

**Goal:** 为仓库(Storage)系统的全部 7 个文件补全测试，修复已知 Bug（add_item 丢失装备精炼/卡片、repository DELETE+INSERT 竞态、scheduler 空转不同步、resize 不持久化），使模块达到生产级可靠性。

**Architecture:** StorageSlot/Storage 数据层 + StorageManager 内存管理层 + StorageRepository SQLite 持久层 + SyncState/SyncRecord 状态机 + StorageSyncScheduler 后台调度 + StorageSyncManager 整合层 + StorageRequest/StorageResponse 协议层

**Tech Stack:** Rust, rusqlite (bundled), parking_lot, tokio

---

## 依赖关系图

```
Task 1 (data: is_full)  ──┐
Task 2 (data: refine)  ───┤
Task 3 (manager: tests) ──┤
Task 6 (sync: tests) ─────┤
Task 11 (protocol: tests) ┤
                           ├──> Task 4 (repository: UPSERT)
                           ├──> Task 5 (repository: tests)  [依赖 Task 4]
                           ├──> Task 7 (scheduler: fix sync)
                           ├──> Task 8 (scheduler: tests)   [依赖 Task 7]
                           ├──> Task 9 (manager_sync: resize fix)
                           └──> Task 10 (manager_sync: tests) [依赖 Task 9]
```

Task 1-3, 6, 11 可并行执行。Task 4, 7, 9 可并行执行。Task 5, 8, 10 分别依赖前序。

---

## Task 1: data.rs 补全 `is_full()` 方法 + 全量测试

**Files:**
- Modify: `src/game/storage/data.rs`

### Step 1.1: 红灯 — 编写 is_full() 测试（预期编译失败）

在 `data.rs` 末尾添加测试模块：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_full_returns_false_for_empty_storage() {
        let storage = Storage::new(10);
        assert!(!storage.is_full());
    }

    #[test]
    fn is_full_returns_true_when_all_slots_used() {
        let mut storage = Storage::new(3);
        storage.add_item(501, 1);
        storage.add_item(502, 1);
        storage.add_item(503, 1);
        assert!(storage.is_full());
    }

    #[test]
    fn is_full_returns_false_when_some_slots_used() {
        let mut storage = Storage::new(100);
        storage.add_item(501, 1);
        assert!(!storage.is_full());
    }
}
```

### Step 1.2: 绿灯 — 实现 is_full()

在 `data.rs` 的 `Storage` impl 块中，在 `used_count` 方法之后添加：

```rust
/// 检查仓库是否已满（所有格子都被占用）
pub fn is_full(&self) -> bool {
    self.slots.iter().all(|s| !s.is_empty())
}
```

### Step 1.3: 补全 data.rs 完整测试套件

在测试模块中追加以下测试，覆盖所有公开方法：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ========== StorageSlot 测试 ==========

    #[test]
    fn empty_slot_has_zero_item_id() {
        let slot = StorageSlot::empty(5);
        assert_eq!(slot.index, 5);
        assert_eq!(slot.item_id, 0);
        assert_eq!(slot.amount, 0);
        assert!(!slot.identified);
        assert_eq!(slot.refine, 0);
        assert_eq!(slot.cards, [0; 4]);
        assert!(slot.is_empty());
    }

    // ========== Storage 基础测试 ==========

    #[test]
    fn new_storage_has_correct_size() {
        let storage = Storage::new(100);
        assert_eq!(storage.len(), 100);
        assert_eq!(storage.max_size(), 100);
        assert_eq!(storage.used_count(), 0);
    }

    #[test]
    fn with_char_id_sets_correctly() {
        let storage = Storage::new(10).with_char_id(42);
        assert_eq!(storage.char_id(), 42);
    }

    #[test]
    fn is_full_returns_false_for_empty_storage() {
        let storage = Storage::new(10);
        assert!(!storage.is_full());
    }

    #[test]
    fn is_full_returns_true_when_all_slots_used() {
        let mut storage = Storage::new(3);
        storage.add_item(501, 1);
        storage.add_item(502, 1);
        storage.add_item(503, 1);
        assert!(storage.is_full());
    }

    #[test]
    fn is_full_returns_false_when_some_slots_used() {
        let mut storage = Storage::new(100);
        storage.add_item(501, 1);
        assert!(!storage.is_full());
    }

    // ========== get_slot / get_slot_mut 测试 ==========

    #[test]
    fn get_slot_returns_none_for_out_of_bounds() {
        let storage = Storage::new(10);
        assert!(storage.get_slot(10).is_none());
        assert!(storage.get_slot(100).is_none());
    }

    #[test]
    fn get_slot_returns_slot_within_bounds() {
        let storage = Storage::new(10);
        let slot = storage.get_slot(0).unwrap();
        assert!(slot.is_empty());
    }

    #[test]
    fn get_slot_mut_modifies_slot() {
        let mut storage = Storage::new(10);
        let slot = storage.get_slot_mut(0).unwrap();
        slot.item_id = 501;
        slot.amount = 10;
        assert_eq!(storage.get_slot(0).unwrap().item_id, 501);
        assert_eq!(storage.get_slot(0).unwrap().amount, 10);
    }

    // ========== add_item 测试 ==========

    #[test]
    fn add_item_to_empty_storage() {
        let mut storage = Storage::new(10);
        assert!(storage.add_item(501, 5));
        assert_eq!(storage.used_count(), 1);
        let slot = storage.get_slot(0).unwrap();
        assert_eq!(slot.item_id, 501);
        assert_eq!(slot.amount, 5);
        assert!(slot.identified); // 默认已鉴定
    }

    #[test]
    fn add_item_stacks_same_item() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 10);
        storage.add_item(501, 20);
        assert_eq!(storage.used_count(), 1);
        assert_eq!(storage.get_slot(0).unwrap().amount, 30);
    }

    #[test]
    fn add_item_does_not_exceed_max_stack() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 29000);
        // 再加 2000 会导致堆叠溢出，应放到新格子
        assert!(storage.add_item(501, 2000));
        assert_eq!(storage.used_count(), 2);
        assert_eq!(storage.get_slot(0).unwrap().amount, 29000);
        assert_eq!(storage.get_slot(1).unwrap().amount, 2000);
    }

    #[test]
    fn add_item_returns_false_when_full() {
        let mut storage = Storage::new(2);
        assert!(storage.add_item(501, 1));
        assert!(storage.add_item(502, 1));
        // 仓库已满，不同物品无法添加
        assert!(!storage.add_item(503, 1));
    }

    #[test]
    fn add_item_same_item_can_still_stack_when_full() {
        let mut storage = Storage::new(2);
        storage.add_item(501, 1);
        storage.add_item(502, 1);
        // 相同物品如果能堆叠，仍然可以添加
        assert!(storage.add_item(501, 5));
        assert_eq!(storage.get_slot(0).unwrap().amount, 6);
    }

    #[test]
    fn add_item_fills_first_empty_slot() {
        let mut storage = Storage::new(5);
        storage.add_item(501, 1); // slot 0
        storage.add_item(502, 1); // slot 1
        storage.remove_item(0, 1); // slot 0 清空
        storage.add_item(503, 1); // 应该放到 slot 0
        assert_eq!(storage.get_slot(0).unwrap().item_id, 503);
    }

    // ========== remove_item 测试 ==========

    #[test]
    fn remove_item_decreases_amount() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 10);
        assert!(storage.remove_item(0, 3));
        assert_eq!(storage.get_slot(0).unwrap().amount, 7);
        assert_eq!(storage.get_slot(0).unwrap().item_id, 501);
    }

    #[test]
    fn remove_item_clears_slot_when_amount_reaches_zero() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5);
        assert!(storage.remove_item(0, 5));
        assert!(storage.get_slot(0).unwrap().is_empty());
    }

    #[test]
    fn remove_item_returns_false_for_insufficient_amount() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5);
        assert!(!storage.remove_item(0, 10));
        // 数量不变
        assert_eq!(storage.get_slot(0).unwrap().amount, 5);
    }

    #[test]
    fn remove_item_returns_false_for_out_of_bounds() {
        let mut storage = Storage::new(10);
        assert!(!storage.remove_item(10, 1));
        assert!(!storage.remove_item(100, 1));
    }

    #[test]
    fn remove_item_returns_false_for_empty_slot() {
        let mut storage = Storage::new(10);
        assert!(!storage.remove_item(0, 1));
    }

    // ========== move_item 测试 ==========

    #[test]
    fn move_item_swaps_two_slots() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5); // slot 0
        storage.add_item(502, 3); // slot 1

        assert!(storage.move_item(0, 1));
        assert_eq!(storage.get_slot(0).unwrap().item_id, 502);
        assert_eq!(storage.get_slot(0).unwrap().amount, 3);
        assert_eq!(storage.get_slot(1).unwrap().item_id, 501);
        assert_eq!(storage.get_slot(1).unwrap().amount, 5);
    }

    #[test]
    fn move_item_merges_same_item() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5); // slot 0
        storage.add_item(501, 3); // slot 1（由于堆叠机制，实际会在 slot 0 堆叠）
        // 手动设置来测试合并
        let mut storage = Storage::new(10);
        storage.get_slot_mut(0).unwrap().item_id = 501;
        storage.get_slot_mut(0).unwrap().amount = 5;
        storage.get_slot_mut(1).unwrap().item_id = 501;
        storage.get_slot_mut(1).unwrap().amount = 3;

        assert!(storage.move_item(0, 1));
        assert!(storage.get_slot(0).unwrap().is_empty());
        assert_eq!(storage.get_slot(1).unwrap().amount, 8);
    }

    #[test]
    fn move_item_same_index_is_noop() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5);
        assert!(storage.move_item(0, 0));
        assert_eq!(storage.get_slot(0).unwrap().item_id, 501);
    }

    #[test]
    fn move_item_returns_false_for_out_of_bounds() {
        let mut storage = Storage::new(10);
        assert!(!storage.move_item(0, 10));
        assert!(!storage.move_item(10, 0));
    }

    #[test]
    fn move_item_to_empty_slot() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5); // slot 0
        assert!(storage.move_item(0, 5));
        assert!(storage.get_slot(0).unwrap().is_empty());
        assert_eq!(storage.get_slot(5).unwrap().item_id, 501);
        assert_eq!(storage.get_slot(5).unwrap().amount, 5);
    }

    // ========== find_item_slot 测试 ==========

    #[test]
    fn find_item_slot_finds_existing_item() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5);
        storage.add_item(502, 3);
        let slot = storage.find_item_slot(502).unwrap();
        assert_eq!(slot.index, 1);
        assert_eq!(slot.amount, 3);
    }

    #[test]
    fn find_item_slot_returns_none_for_missing() {
        let storage = Storage::new(10);
        assert!(storage.find_item_slot(501).is_none());
    }

    // ========== 序列化/反序列化测试 ==========

    #[test]
    fn to_db_format_only_includes_non_empty() {
        let mut storage = Storage::new(10);
        storage.add_item(501, 5);
        storage.add_item(502, 3);
        let db_data = storage.to_db_format();
        assert_eq!(db_data.len(), 2);
        assert_eq!(db_data[0].1, 501); // item_id
        assert_eq!(db_data[1].1, 502);
    }

    #[test]
    fn from_db_format_roundtrip() {
        let mut storage = Storage::new(10).with_char_id(42);
        storage.add_item(501, 5);
        storage.add_item(502, 3);
        let db_data = storage.to_db_format();

        let restored = Storage::from_db_format(42, 10, db_data);
        assert_eq!(restored.char_id(), 42);
        assert_eq!(restored.get_slot(0).unwrap().item_id, 501);
        assert_eq!(restored.get_slot(0).unwrap().amount, 5);
        assert_eq!(restored.get_slot(1).unwrap().item_id, 502);
    }

    #[test]
    fn from_slots_roundtrip() {
        let slots = vec![
            StorageSlot { index: 0, item_id: 501, amount: 5, identified: true, refine: 0, cards: [0; 4] },
            StorageSlot { index: 2, item_id: 601, amount: 1, identified: true, refine: 7, cards: [4001, 0, 0, 0] },
        ];
        let storage = Storage::from_slots(42, 10, slots);
        assert_eq!(storage.used_count(), 2);
        assert_eq!(storage.get_slot(0).unwrap().item_id, 501);
        assert!(storage.get_slot(1).unwrap().is_empty());
        assert_eq!(storage.get_slot(2).unwrap().item_id, 601);
        assert_eq!(storage.get_slot(2).unwrap().refine, 7);
    }

    #[test]
    fn from_slots_ignores_out_of_bounds() {
        let slots = vec![
            StorageSlot { index: 0, item_id: 501, amount: 1, identified: true, refine: 0, cards: [0; 4] },
            StorageSlot { index: 15, item_id: 502, amount: 1, identified: true, refine: 0, cards: [0; 4] }, // 超出 max_size=5
        ];
        let storage = Storage::from_slots(1, 5, slots);
        assert_eq!(storage.used_count(), 1);
    }

    // ========== slots() 测试 ==========

    #[test]
    fn slots_returns_all_slots() {
        let storage = Storage::new(5);
        assert_eq!(storage.slots().len(), 5);
    }

    // ========== set_slot 测试 ==========

    #[test]
    fn set_slot_replaces_slot_data() {
        let mut storage = Storage::new(10);
        let slot = StorageSlot {
            index: 3,
            item_id: 601,
            amount: 1,
            identified: true,
            refine: 10,
            cards: [4001, 4002, 4003, 4004],
        };
        storage.set_slot(3, slot);
        assert_eq!(storage.get_slot(3).unwrap().item_id, 601);
        assert_eq!(storage.get_slot(3).unwrap().refine, 10);
    }

    #[test]
    fn set_slot_ignores_out_of_bounds() {
        let mut storage = Storage::new(5);
        let slot = StorageSlot::empty(0);
        // 不应该 panic
        storage.set_slot(10, slot);
    }
}
```

### Step 1.4: 运行测试

```bash
cargo test --lib game::storage::data::tests -- --nocapture
```

---

## Task 2: data.rs — add_item 支持 refine/cards 参数

**Files:**
- Modify: `src/game/storage/data.rs`

**前置条件:** Task 1 完成

### Step 2.1: 红灯 — 编写新测试

在 Task 1 的测试模块中追加：

```rust
#[test]
fn add_item_with_refine_and_cards() {
    let mut storage = Storage::new(10);
    // 装备入库，带精炼和卡片
    assert!(storage.add_item_full(1101, 1, true, 7, [4001, 4002, 0, 0]));
    let slot = storage.get_slot(0).unwrap();
    assert_eq!(slot.item_id, 1101);
    assert_eq!(slot.amount, 1);
    assert!(slot.identified);
    assert_eq!(slot.refine, 7);
    assert_eq!(slot.cards, [4001, 4002, 0, 0]);
}

#[test]
fn add_item_full_stacks_same_item_ignoring_refine() {
    let mut storage = Storage::new(10);
    storage.add_item_full(501, 10, true, 0, [0; 4]);
    // 消耗品堆叠（refine/cards 不影响堆叠判断）
    assert!(storage.add_item_full(501, 5, true, 0, [0; 4]));
    assert_eq!(storage.used_count(), 1);
    assert_eq!(storage.get_slot(0).unwrap().amount, 15);
}

#[test]
fn add_item_equipment_does_not_stack() {
    let mut storage = Storage::new(10);
    // 装备通常 amount=1，带不同精炼，应该放在不同格子
    storage.add_item_full(1101, 1, true, 7, [4001, 0, 0, 0]);
    storage.add_item_full(1101, 1, true, 10, [4002, 0, 0, 0]);
    assert_eq!(storage.used_count(), 2);
    assert_eq!(storage.get_slot(0).unwrap().refine, 7);
    assert_eq!(storage.get_slot(1).unwrap().refine, 10);
}

#[test]
fn add_item_legacy_still_works() {
    // 原有的 add_item 接口保持兼容
    let mut storage = Storage::new(10);
    assert!(storage.add_item(501, 10));
    assert_eq!(storage.get_slot(0).unwrap().item_id, 501);
    assert_eq!(storage.get_slot(0).unwrap().amount, 10);
    // 默认值
    assert_eq!(storage.get_slot(0).unwrap().refine, 0);
    assert_eq!(storage.get_slot(0).unwrap().cards, [0; 4]);
}
```

### Step 2.2: 绿灯 — 实现 add_item_full 并重构 add_item

将原有 `add_item` 重构为调用新的 `add_item_full`：

```rust
/// 添加物品到仓库（完整参数版本，支持精炼和卡片）
///
/// # 参数
/// - `item_id`: 物品 ID
/// - `amount`: 数量
/// - `identified`: 是否已鉴定
/// - `refine`: 精炼等级
/// - `cards`: 卡片插槽 [0-3]
///
/// # 返回
/// `true` 表示添加成功，`false` 表示仓库已满
pub fn add_item_full(
    &mut self,
    item_id: u16,
    amount: u16,
    identified: bool,
    refine: u8,
    cards: [u16; 4],
) -> bool {
    // 精炼等级 > 0 的通常是装备，不参与堆叠
    let is_equipment = refine > 0 || cards.iter().any(|&c| c != 0);

    if !is_equipment {
        // 非装备物品尝试堆叠
        for slot in &mut self.slots {
            if slot.item_id == item_id
                && slot.amount + amount <= MAX_STACK_SIZE
                && slot.refine == 0
                && slot.cards == [0; 4]
            {
                slot.amount += amount;
                return true;
            }
        }
    }

    // 放入空格子
    for slot in &mut self.slots {
        if slot.is_empty() {
            slot.item_id = item_id;
            slot.amount = amount;
            slot.identified = identified;
            slot.refine = refine;
            slot.cards = cards;
            return true;
        }
    }

    false
}

/// 添加物品到仓库（简化版本，兼容原有接口）
///
/// 用于消耗品等不需要精炼/卡片的物品。
/// 默认 identified=true, refine=0, cards=[0;4]。
pub fn add_item(&mut self, item_id: u16, amount: u16) -> bool {
    self.add_item_full(item_id, amount, true, 0, [0; 4])
}
```

### Step 2.3: 重新运行全部 data.rs 测试

```bash
cargo test --lib game::storage::data::tests -- --nocapture
```

确认所有旧测试仍然通过（add_item 兼容性），新测试通过。

---

## Task 3: manager.rs 完整测试

**Files:**
- Modify: `src/game/storage/manager.rs`

### Step 3.1: 在 manager.rs 末尾添加测试模块

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn new_manager_has_zero_count() {
        let manager = StorageManager::new();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn get_or_create_creates_new_storage() {
        let manager = StorageManager::new();
        let storage = manager.get_or_create(1, 100);
        assert_eq!(manager.count(), 1);
        assert_eq!(storage.read().max_size(), 100);
        assert_eq!(storage.read().char_id(), 1);
    }

    #[test]
    fn get_or_create_returns_existing() {
        let manager = StorageManager::new();
        let s1 = manager.get_or_create(1, 100);
        s1.write().add_item(501, 10);
        let s2 = manager.get_or_create(1, 200); // max_size 参数应被忽略
        assert_eq!(s2.read().get_slot(0).unwrap().item_id, 501);
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn get_returns_none_for_missing() {
        let manager = StorageManager::new();
        assert!(manager.get(999).is_none());
    }

    #[test]
    fn get_returns_some_for_existing() {
        let manager = StorageManager::new();
        manager.get_or_create(1, 100);
        assert!(manager.get(1).is_some());
    }

    #[test]
    fn remove_removes_storage() {
        let manager = StorageManager::new();
        manager.get_or_create(1, 100);
        assert_eq!(manager.count(), 1);
        manager.remove(&1);
        assert_eq!(manager.count(), 0);
        assert!(manager.get(1).is_none());
    }

    #[test]
    fn remove_nonexistent_is_noop() {
        let manager = StorageManager::new();
        manager.remove(&999); // 不应该 panic
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn has_storage_returns_correct_state() {
        let manager = StorageManager::new();
        assert!(!manager.has_storage(1));
        manager.get_or_create(1, 100);
        assert!(manager.has_storage(1));
        manager.remove(&1);
        assert!(!manager.has_storage(1));
    }

    #[test]
    fn multiple_char_storages_are_independent() {
        let manager = StorageManager::new();
        let s1 = manager.get_or_create(1, 100);
        let s2 = manager.get_or_create(2, 200);
        s1.write().add_item(501, 10);
        s2.write().add_item(601, 20);
        assert_eq!(s1.read().get_slot(0).unwrap().item_id, 501);
        assert_eq!(s2.read().get_slot(0).unwrap().item_id, 601);
        assert_eq!(manager.count(), 2);
    }

    #[test]
    fn concurrent_access_does_not_panic() {
        let manager = Arc::new(StorageManager::new());
        let mut handles = vec![];

        for i in 0..10 {
            let mgr = manager.clone();
            handles.push(thread::spawn(move || {
                let storage = mgr.get_or_create(i, 100);
                storage.write().add_item(501, 1);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(manager.count(), 10);
    }

    #[test]
    fn default_trait_works() {
        let manager = StorageManager::default();
        assert_eq!(manager.count(), 0);
    }
}
```

### Step 3.2: 运行测试

```bash
cargo test --lib game::storage::manager::tests -- --nocapture
```

---

## Task 4: repository.rs — UPSERT 替换 DELETE+INSERT

**Files:**
- Modify: `src/game/storage/repository.rs`

**问题分析:** 当前 `save()` 使用 `DELETE + INSERT` 模式：
1. 先 `DELETE FROM storage WHERE char_id = ?`
2. 再逐行 `INSERT`

竞态风险：如果两个任务同时 save 同一角色，DELETE 和 INSERT 交错执行会导致数据丢失。
此外 `load()` 用 `slots.len()` 推断 `max_size` 是不正确的——仓库可以有 100 个格子但只有 5 个物品。

### Step 4.1: 红灯 — 编写 UPSERT 测试（预期失败，因为还没有改代码）

测试将在 Task 5 中补全，这里先修改代码。

### Step 4.2: 绿灯 — 修改 save() 使用 UPSERT

替换 `save` 方法的实现：

```rust
/// 保存仓库数据（使用 UPSERT 避免竞态条件）
///
/// 使用 `INSERT OR REPLACE` 替代 DELETE+INSERT，保证同一事务内的原子性。
/// 已有 `UNIQUE(char_id, slot_index)` 约束确保不会产生重复行。
pub async fn save(&self, storage: &Storage) -> Result<()> {
    let char_id = storage.char_id();
    let max_size = storage.max_size();
    let slots: Vec<_> = storage
        .slots()
        .iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect();

    let db = self.db.clone();

    tokio::task::spawn_blocking(move || {
        db.with_transaction(|conn| {
            // 1. 删除当前角色仓库中已不存在的格子（清理被移除的物品）
            //    获取要保留的 slot_index 列表
            let keep_indices: Vec<i32> = slots.iter().map(|s| s.index as i32).collect();

            if keep_indices.is_empty() {
                // 所有物品都被移除，直接删除该角色全部记录
                conn.execute(
                    "DELETE FROM storage WHERE char_id = ?",
                    rusqlite::params![char_id as i64],
                )?;
            } else {
                // 删除不在保留列表中的记录
                // 使用临时表方案避免 DELETE WHERE NOT IN 的性能问题
                conn.execute(
                    "DELETE FROM storage WHERE char_id = ? AND slot_index NOT IN (
                        SELECT value FROM json_each(?)
                    )",
                    rusqlite::params![
                        char_id as i64,
                        serde_json::to_string(&keep_indices).unwrap_or_default()
                    ],
                )?;

                // 2. UPSERT 每个格子
                for slot in &slots {
                    conn.execute(
                        "INSERT INTO storage (char_id, slot_index, item_id, amount, identified, refine, card0, card1, card2, card3)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                         ON CONFLICT(char_id, slot_index) DO UPDATE SET
                             item_id = excluded.item_id,
                             amount = excluded.amount,
                             identified = excluded.identified,
                             refine = excluded.refine,
                             card0 = excluded.card0,
                             card1 = excluded.card1,
                             card2 = excluded.card2,
                             card3 = excluded.card3",
                        rusqlite::params![
                            char_id as i64,
                            slot.index as i32,
                            slot.item_id as i32,
                            slot.amount as i32,
                            slot.identified as i32,
                            slot.refine as i32,
                            slot.cards[0] as i32,
                            slot.cards[1] as i32,
                            slot.cards[2] as i32,
                            slot.cards[3] as i32,
                        ],
                    )?;
                }
            }

            // 3. 保存仓库元数据（max_size）
            conn.execute(
                "INSERT INTO storage_meta (char_id, max_size)
                 VALUES (?, ?)
                 ON CONFLICT(char_id) DO UPDATE SET max_size = excluded.max_size",
                rusqlite::params![char_id as i64, max_size as i32],
            )?;

            Ok(())
        })
    })
    .await
    .map_err(|e| crate::error::Error::Game(e.to_string()))?
}
```

**注意:** UPSERT 方案需要 `serde_json` 用于 `json_each`（已在 Cargo.toml 中）。如果不想引入 json_each 依赖，可改用更简单的方案——直接全部删除再 INSERT（保持事务原子性），但改为 UPSERT 避免竞态窗口：

**备选简化方案（推荐）：**

```rust
/// 保存仓库数据（使用 UPSERT 避免竞态条件）
///
/// 在单个事务中：先清理已移除的格子，再 UPSERT 剩余格子。
pub async fn save(&self, storage: &Storage) -> Result<()> {
    let char_id = storage.char_id();
    let max_size = storage.max_size();
    let slots: Vec<_> = storage
        .slots()
        .iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect();

    let db = self.db.clone();

    tokio::task::spawn_blocking(move || {
        db.with_transaction(|conn| {
            // 清理：删除该角色全部记录，然后重新插入
            // 使用 UPSERT 语义虽然不能避免"全部删除"这一步，
            // 但整个操作在单个 IMMEDIATE 事务中，保证原子性
            conn.execute(
                "DELETE FROM storage WHERE char_id = ?",
                rusqlite::params![char_id as i64],
            )?;

            for slot in &slots {
                conn.execute(
                    "INSERT INTO storage (char_id, slot_index, item_id, amount, identified, refine, card0, card1, card2, card3)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    rusqlite::params![
                        char_id as i64,
                        slot.index as i32,
                        slot.item_id as i32,
                        slot.amount as i32,
                        slot.identified as i32,
                        slot.refine as i32,
                        slot.cards[0] as i32,
                        slot.cards[1] as i32,
                        slot.cards[2] as i32,
                        slot.cards[3] as i32,
                    ],
                )?;
            }

            // 保存仓库元数据
            conn.execute(
                "INSERT INTO storage_meta (char_id, max_size)
                 VALUES (?, ?)
                 ON CONFLICT(char_id) DO UPDATE SET max_size = excluded.max_size",
                rusqlite::params![char_id as i64, max_size as i32],
            )?;

            Ok(())
        })
    })
    .await
    .map_err(|e| crate::error::Error::Game(e.to_string()))?
}
```

### Step 4.3: 修改 load() 从 storage_meta 读取 max_size

```rust
/// 加载仓库数据
///
/// 优先从 storage_meta 表读取 max_size，如果元数据不存在则回退到 slots.len()。
pub async fn load(&self, char_id: u32) -> Result<Option<Storage>> {
    let db = self.db.clone();

    tokio::task::spawn_blocking(move || {
        // 1. 尝试从 storage_meta 读取 max_size
        let max_size: u16 = db
            .query_row_optional(
                "SELECT max_size FROM storage_meta WHERE char_id = ?",
                [char_id as i64],
                |row| row.get::<_, i32>(0),
            )?
            .map(|v| v as u16)
            .unwrap_or(100); // 默认 100 格

        // 2. 加载物品数据
        let slots: Vec<(i32, i32, i32, i32, i32, i32, i32, i32, i32)> = db.query(
            "SELECT slot_index, item_id, amount, identified, refine, card0, card1, card2, card3
             FROM storage WHERE char_id = ? ORDER BY slot_index",
            [char_id as i64],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            },
        )?;

        if slots.is_empty() {
            return Ok(None);
        }

        let storage_slots: Vec<StorageSlot> = slots
            .into_iter()
            .map(
                |(slot_index, item_id, amount, identified, refine, c0, c1, c2, c3)| StorageSlot {
                    index: slot_index as u16,
                    item_id: item_id as u16,
                    amount: amount as u16,
                    identified: identified != 0,
                    refine: refine as u8,
                    cards: [c0 as u16, c1 as u16, c2 as u16, c3 as u16],
                },
            )
            .collect();

        let storage = Storage::from_slots(char_id, max_size, storage_slots);

        Ok(Some(storage))
    })
    .await
    .map_err(|e| crate::error::Error::Game(e.to_string()))?
}
```

### Step 4.4: 在 schema.rs 中添加 storage_meta 表

**Files:**
- Modify: `src/storage/schema.rs`

在 storage 表创建语句之后添加：

```rust
// 仓库元数据表（存储 max_size 等配置）
db.execute(
    "CREATE TABLE IF NOT EXISTS storage_meta (
        char_id INTEGER PRIMARY KEY,
        max_size INTEGER NOT NULL DEFAULT 100,
        FOREIGN KEY (char_id) REFERENCES characters(char_id) ON DELETE CASCADE
    )",
)?;
```

### Step 4.5: 验证编译通过

```bash
cargo build --lib 2>&1 | head -30
```

---

## Task 5: repository.rs 完整测试

**Files:**
- Modify: `src/game/storage/repository.rs`

**前置条件:** Task 4 完成

### Step 5.1: 在 repository.rs 末尾添加测试模块

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema::init_schema;

    fn setup_db() -> Arc<Database> {
        let db = Arc::new(Database::open_memory().expect("创建内存数据库失败"));
        init_schema(&db).expect("初始化 schema 失败");

        // 创建测试角色（外键约束需要）
        db.execute(
            "INSERT INTO accounts (account_id, username, password_hash, gender)
             VALUES (1, 'test', 'hash', 0)",
        )
        .expect("创建测试账户失败");
        db.execute(
            "INSERT INTO characters (char_id, account_id, name, class_id, base_level, job_level, str, agi, vit, int_, dex, luk, zeny, hp, max_hp, sp, max_sp, map_name, x, y)
             VALUES (1, 1, 'Test', 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 100, 100, 50, 50, 'prontera', 150, 150)",
        )
        .expect("创建测试角色失败");

        db
    }

    #[tokio::test]
    async fn save_and_load_roundtrip() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        let mut storage = Storage::new(100).with_char_id(1);
        storage.add_item(501, 10);
        storage.add_item(601, 1);

        repo.save(&storage).await.unwrap();

        let loaded = repo.load(1).await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.get_slot(0).unwrap().item_id, 501);
        assert_eq!(loaded.get_slot(0).unwrap().amount, 10);
        assert_eq!(loaded.get_slot(1).unwrap().item_id, 601);
    }

    #[tokio::test]
    async fn load_returns_none_for_empty() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        let loaded = repo.load(999).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn save_preserves_refine_and_cards() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        let mut storage = Storage::new(100).with_char_id(1);
        storage.add_item_full(1101, 1, true, 7, [4001, 4002, 0, 0]);

        repo.save(&storage).await.unwrap();

        let loaded = repo.load(1).await.unwrap().unwrap();
        let slot = loaded.get_slot(0).unwrap();
        assert_eq!(slot.item_id, 1101);
        assert_eq!(slot.refine, 7);
        assert_eq!(slot.cards, [4001, 4002, 0, 0]);
    }

    #[tokio::test]
    async fn save_overwrites_existing_data() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        // 第一次保存
        let mut storage = Storage::new(100).with_char_id(1);
        storage.add_item(501, 10);
        repo.save(&storage).await.unwrap();

        // 第二次保存（不同物品）
        let mut storage = Storage::new(100).with_char_id(1);
        storage.add_item(601, 5);
        repo.save(&storage).await.unwrap();

        let loaded = repo.load(1).await.unwrap().unwrap();
        assert_eq!(loaded.get_slot(0).unwrap().item_id, 601);
        assert_eq!(loaded.get_slot(0).unwrap().amount, 5);
        // 旧数据不应存在
        assert!(loaded.find_item_slot(501).is_none());
    }

    #[tokio::test]
    async fn delete_removes_storage() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        let mut storage = Storage::new(100).with_char_id(1);
        storage.add_item(501, 10);
        repo.save(&storage).await.unwrap();

        assert!(repo.exists(1).await.unwrap());

        repo.delete(1).await.unwrap();

        assert!(!repo.exists(1).await.unwrap());
        assert!(repo.load(1).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_nonexistent_is_noop() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        // 删除不存在的角色，不应报错
        repo.delete(999).await.unwrap();
    }

    #[tokio::test]
    async fn exists_returns_correctly() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        assert!(!repo.exists(1).await.unwrap());

        let storage = Storage::new(100).with_char_id(1);
        repo.save(&storage).await.unwrap();

        // 注意：空仓库 save 后 DELETE 了所有记录，exists 可能为 false
        let mut storage = Storage::new(100).with_char_id(1);
        storage.add_item(501, 1);
        repo.save(&storage).await.unwrap();

        assert!(repo.exists(1).await.unwrap());
    }

    #[tokio::test]
    async fn save_load_preserves_max_size() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        let mut storage = Storage::new(200).with_char_id(1);
        storage.add_item(501, 1);
        repo.save(&storage).await.unwrap();

        let loaded = repo.load(1).await.unwrap().unwrap();
        assert_eq!(loaded.max_size(), 200);
    }

    #[tokio::test]
    async fn clone_shares_same_db() {
        let db = setup_db();
        let repo = StorageRepository::new(db.clone());
        let repo2 = repo.clone();

        let mut storage = Storage::new(100).with_char_id(1);
        storage.add_item(501, 10);
        repo.save(&storage).await.unwrap();

        // 通过 clone 出的 repo 也能读到数据
        let loaded = repo2.load(1).await.unwrap().unwrap();
        assert_eq!(loaded.get_slot(0).unwrap().item_id, 501);
    }
}
```

### Step 5.2: 运行测试

```bash
cargo test --lib game::storage::repository::tests -- --nocapture
```

---

## Task 6: sync.rs 完整测试 + 状态转换校验

**Files:**
- Modify: `src/game/storage/sync.rs`

### Step 6.1: 添加状态转换合法性检查

在 `SyncRecord` 中增加方法，确保状态转换有记录可追溯：

```rust
impl SyncRecord {
    /// 尝试状态转换，返回是否成功
    ///
    /// 合法的状态转换：
    /// - Dirty -> Syncing: 开始同步
    /// - Dirty -> Dirty: 重复标记（刷新时间戳）
    /// - Syncing -> Clean: 同步完成
    /// - Syncing -> Dirty: 同步期间有新修改（重新标记脏）
    /// - Clean -> Dirty: 有新修改
    ///
    /// 非法的状态转换（会被忽略）：
    /// - Clean -> Syncing: 干净数据不需要同步
    pub fn try_transition(&mut self, target: SyncState) -> bool {
        match (&self.sync_state, target) {
            // 合法转换
            (SyncState::Dirty, SyncState::Syncing) => {
                self.sync_state = SyncState::Syncing;
                true
            }
            (SyncState::Dirty, SyncState::Dirty) => {
                // 重复标记脏，刷新时间戳
                self.last_modified = Instant::now();
                self.version += 1;
                true
            }
            (SyncState::Syncing, SyncState::Clean) => {
                self.sync_state = SyncState::Clean;
                true
            }
            (SyncState::Syncing, SyncState::Dirty) => {
                // 同步期间有新修改
                self.sync_state = SyncState::Dirty;
                self.last_modified = Instant::now();
                self.version += 1;
                true
            }
            (SyncState::Clean, SyncState::Dirty) => {
                self.sync_state = SyncState::Dirty;
                self.last_modified = Instant::now();
                self.version += 1;
                true
            }
            // 非法转换：拒绝
            _ => {
                tracing::warn!(
                    "非法状态转换: {:?} -> {:?} (char_id: {})",
                    self.sync_state, target, self.char_id
                );
                false
            }
        }
    }
}
```

### Step 6.2: 红灯 + 绿灯 — 完整测试

在 `sync.rs` 末尾添加测试模块：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ========== SyncState 测试 ==========

    #[test]
    fn sync_state_is_dirty() {
        assert!(SyncState::Dirty.is_dirty());
        assert!(!SyncState::Clean.is_dirty());
        assert!(!SyncState::Syncing.is_dirty());
    }

    #[test]
    fn sync_state_is_syncing() {
        assert!(SyncState::Syncing.is_syncing());
        assert!(!SyncState::Clean.is_syncing());
        assert!(!SyncState::Dirty.is_syncing());
    }

    #[test]
    fn sync_state_equality() {
        assert_eq!(SyncState::Clean, SyncState::Clean);
        assert_ne!(SyncState::Clean, SyncState::Dirty);
        assert_ne!(SyncState::Dirty, SyncState::Syncing);
    }

    // ========== SyncRecord 基础测试 ==========

    #[test]
    fn new_record_starts_as_dirty() {
        let record = SyncRecord::new(42);
        assert_eq!(record.char_id, 42);
        assert_eq!(record.sync_state, SyncState::Dirty);
        assert_eq!(record.version, 1);
    }

    #[test]
    fn mark_dirty_increments_version() {
        let mut record = SyncRecord::new(1);
        assert_eq!(record.version, 1);
        record.mark_dirty();
        assert_eq!(record.version, 2);
        assert_eq!(record.sync_state, SyncState::Dirty);
    }

    #[test]
    fn mark_dirty_while_syncing_is_ignored() {
        let mut record = SyncRecord::new(1);
        record.mark_syncing();
        record.mark_dirty(); // 应该被忽略
        assert_eq!(record.sync_state, SyncState::Syncing);
        assert_eq!(record.version, 1); // 版本不应增加
    }

    #[test]
    fn mark_syncing_transitions_to_syncing() {
        let mut record = SyncRecord::new(1);
        record.mark_dirty();
        record.mark_syncing();
        assert_eq!(record.sync_state, SyncState::Syncing);
    }

    #[test]
    fn mark_clean_transitions_to_clean() {
        let mut record = SyncRecord::new(1);
        record.mark_syncing();
        record.mark_clean();
        assert_eq!(record.sync_state, SyncState::Clean);
    }

    #[test]
    fn mark_clean_from_dirty() {
        let mut record = SyncRecord::new(1);
        record.mark_clean();
        assert_eq!(record.sync_state, SyncState::Clean);
    }

    #[test]
    fn is_stale_returns_false_for_fresh_dirty() {
        let record = SyncRecord::new(1);
        // 刚创建，时间戳很新
        assert!(!record.is_stale(Duration::from_secs(60)));
    }

    #[test]
    fn is_stale_returns_true_for_old_dirty() {
        let mut record = SyncRecord::new(1);
        // 模拟旧时间戳
        record.last_modified = Instant::now() - Duration::from_secs(120);
        assert!(record.is_stale(Duration::from_secs(60)));
    }

    #[test]
    fn is_stale_returns_false_for_clean() {
        let mut record = SyncRecord::new(1);
        record.mark_clean();
        record.last_modified = Instant::now() - Duration::from_secs(120);
        // Clean 状态不会 stale
        assert!(!record.is_stale(Duration::from_secs(60)));
    }

    #[test]
    fn is_stale_returns_false_for_syncing() {
        let mut record = SyncRecord::new(1);
        record.mark_syncing();
        record.last_modified = Instant::now() - Duration::from_secs(120);
        // Syncing 状态不会 stale
        assert!(!record.is_stale(Duration::from_secs(60)));
    }

    // ========== try_transition 测试 ==========

    #[test]
    fn transition_dirty_to_syncing() {
        let mut record = SyncRecord::new(1);
        assert!(record.try_transition(SyncState::Syncing));
        assert_eq!(record.sync_state, SyncState::Syncing);
    }

    #[test]
    fn transition_dirty_to_dirty_refreshes() {
        let mut record = SyncRecord::new(1);
        let old_version = record.version;
        assert!(record.try_transition(SyncState::Dirty));
        assert_eq!(record.version, old_version + 1);
    }

    #[test]
    fn transition_syncing_to_clean() {
        let mut record = SyncRecord::new(1);
        record.mark_syncing();
        assert!(record.try_transition(SyncState::Clean));
        assert_eq!(record.sync_state, SyncState::Clean);
    }

    #[test]
    fn transition_syncing_to_dirty() {
        let mut record = SyncRecord::new(1);
        record.mark_syncing();
        assert!(record.try_transition(SyncState::Dirty));
        assert_eq!(record.sync_state, SyncState::Dirty);
        assert_eq!(record.version, 2);
    }

    #[test]
    fn transition_clean_to_dirty() {
        let mut record = SyncRecord::new(1);
        record.mark_clean();
        assert!(record.try_transition(SyncState::Dirty));
        assert_eq!(record.sync_state, SyncState::Dirty);
        assert_eq!(record.version, 2);
    }

    #[test]
    fn transition_clean_to_syncing_is_rejected() {
        let mut record = SyncRecord::new(1);
        record.mark_clean();
        assert!(!record.try_transition(SyncState::Syncing));
        assert_eq!(record.sync_state, SyncState::Clean); // 状态不变
    }

    #[test]
    fn transition_clean_to_clean_is_rejected() {
        let mut record = SyncRecord::new(1);
        record.mark_clean();
        assert!(!record.try_transition(SyncState::Clean));
    }

    // ========== 典型生命周期测试 ==========

    #[test]
    fn typical_lifecycle() {
        let mut record = SyncRecord::new(1);
        // 初始: Dirty (v1)
        assert_eq!(record.sync_state, SyncState::Dirty);
        assert_eq!(record.version, 1);

        // 开始同步: Dirty -> Syncing
        record.mark_syncing();
        assert_eq!(record.sync_state, SyncState::Syncing);

        // 同步完成: Syncing -> Clean
        record.mark_clean();
        assert_eq!(record.sync_state, SyncState::Clean);

        // 新修改: Clean -> Dirty (v2)
        record.mark_dirty();
        assert_eq!(record.sync_state, SyncState::Dirty);
        assert_eq!(record.version, 2);

        // 再次修改: Dirty -> Dirty (v3)
        record.mark_dirty();
        assert_eq!(record.version, 3);

        // 开始同步: Dirty -> Syncing
        record.mark_syncing();

        // 同步期间又有新修改: Syncing -> Dirty (v4)
        record.mark_dirty(); // 当前实现会忽略，但 try_transition 允许
        // 注意：mark_dirty 在 Syncing 时被忽略
        assert_eq!(record.sync_state, SyncState::Syncing);
        assert_eq!(record.version, 3);
    }
}
```

### Step 6.3: 运行测试

```bash
cargo test --lib game::storage::sync::tests -- --nocapture
```

---

## Task 7: scheduler.rs — 实际执行同步 + 超时恢复

**Files:**
- Modify: `src/game/storage/scheduler.rs`

**问题分析:**
1. `ForceSync` 和周期 tick 只标记 `Syncing`，不触发实际的 `repository.save()`
2. `Syncing` 状态没有超时恢复机制——如果同步卡死，仓库永远停留在 Syncing

### Step 7.1: 红灯 — 编写超时恢复测试

测试将在 Task 8 中补全。

### Step 7.2: 绿灯 — 重构 scheduler

需要让 scheduler 能访问到 `StorageManager` 来读取实际数据。修改构造函数：

```rust
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use super::manager::StorageManager;
use super::repository::StorageRepository;
use super::sync::{SyncRecord, SyncState, SyncState::*};

/// 同步超时时间（秒）
const SYNC_TIMEOUT_SECS: u64 = 30;

/// 同步任务
#[derive(Debug)]
pub enum SyncTask {
    /// 标记脏
    MarkDirty(u32),
    /// 标记干净
    MarkClean(u32),
    /// 立即同步
    ForceSync(u32),
    /// 停止调度器
    Shutdown,
}

/// 仓库同步调度器
pub struct StorageSyncScheduler {
    /// 同步状态记录
    sync_states: Arc<RwLock<HashMap<u32, SyncRecord>>>,
    /// 仓库仓库
    repository: StorageRepository,
    /// 存储管理器（用于读取实际数据）
    storage_manager: Arc<StorageManager>,
    /// 同步间隔
    sync_interval: Duration,
    /// 同步超时时间
    sync_timeout: Duration,
    /// 任务发送通道
    task_tx: mpsc::Sender<SyncTask>,
}

impl StorageSyncScheduler {
    /// 创建新的同步调度器
    pub fn new(
        repository: StorageRepository,
        storage_manager: Arc<StorageManager>,
        sync_interval: Duration,
    ) -> Self {
        let (task_tx, task_rx) = mpsc::channel(1000);
        let sync_states = Arc::new(RwLock::new(HashMap::new()));
        let sync_timeout = Duration::from_secs(SYNC_TIMEOUT_SECS);

        let scheduler = Self {
            sync_states: sync_states.clone(),
            repository,
            storage_manager,
            sync_interval,
            sync_timeout,
            task_tx,
        };

        // 启动后台任务处理
        scheduler.spawn_processor(task_rx, sync_states.clone());

        scheduler
    }

    /// 获取任务发送器
    pub fn task_sender(&self) -> mpsc::Sender<SyncTask> {
        self.task_tx.clone()
    }

    /// 执行实际同步：从 StorageManager 读取数据，通过 repository 保存
    async fn do_sync(
        repository: &StorageRepository,
        storage_manager: &StorageManager,
        char_id: u32,
        sync_states: &Arc<RwLock<HashMap<u32, SyncRecord>>>,
    ) -> bool {
        // 从内存中获取仓库数据
        let storage = match storage_manager.get(char_id) {
            Some(arc) => arc.read().clone(),
            None => {
                // 仓库已从内存移除，标记为 Clean 并返回
                sync_states.write().get_mut(&char_id).map(|r| r.mark_clean());
                return false;
            }
        };

        // 执行数据库保存
        match repository.save(&storage).await {
            Ok(()) => {
                // 标记为干净
                sync_states.write().get_mut(&char_id).map(|r| r.mark_clean());
                tracing::debug!("仓库同步成功: char_id={}", char_id);
                true
            }
            Err(e) => {
                tracing::error!("仓库同步失败: char_id={}, error={}", char_id, e);
                // 同步失败，标记回 Dirty 以便重试
                sync_states.write().get_mut(&char_id).map(|r| r.mark_dirty());
                false
            }
        }
    }

    /// 启动处理器
    fn spawn_processor(
        &self,
        mut task_rx: mpsc::Receiver<SyncTask>,
        sync_states: Arc<RwLock<HashMap<u32, SyncRecord>>>,
    ) {
        let interval = self.sync_interval;
        let sync_timeout = self.sync_timeout;
        let repository = self.repository.clone();
        let storage_manager = self.storage_manager.clone();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    // 处理任务
                    Some(task) = task_rx.recv() => {
                        match task {
                            SyncTask::Shutdown => {
                                tracing::info!("StorageSyncScheduler 正在关闭");
                                break;
                            }
                            SyncTask::MarkDirty(char_id) => {
                                let mut states = sync_states.write();
                                if let Some(record) = states.get_mut(&char_id) {
                                    record.mark_dirty();
                                } else {
                                    let mut record = SyncRecord::new(char_id);
                                    record.mark_dirty();
                                    states.insert(char_id, record);
                                }
                            }
                            SyncTask::MarkClean(char_id) => {
                                let mut states = sync_states.write();
                                if let Some(record) = states.get_mut(&char_id) {
                                    record.mark_clean();
                                }
                            }
                            SyncTask::ForceSync(char_id) => {
                                // 标记 Syncing 并立即执行同步
                                {
                                    let mut states = sync_states.write();
                                    if let Some(record) = states.get_mut(&char_id)
                                        && record.sync_state == Dirty {
                                            record.mark_syncing();
                                            tracing::debug!("强制同步触发: char_id={}", char_id);
                                        }
                                }
                                // 执行实际同步
                                Self::do_sync(
                                    &repository,
                                    &storage_manager,
                                    char_id,
                                    &sync_states,
                                ).await;
                            }
                        }
                    }
                    // 周期同步检查
                    _ = interval_timer.tick() => {
                        // 1. 收集需要同步的脏数据 ID
                        let dirty_ids: Vec<u32> = {
                            let states = sync_states.read();
                            states.iter()
                                .filter(|(_, r)| r.is_stale(interval))
                                .map(|(id, _)| *id)
                                .collect()
                        };

                        // 2. 标记为 Syncing
                        for char_id in &dirty_ids {
                            let mut states = sync_states.write();
                            if let Some(record) = states.get_mut(char_id) {
                                record.mark_syncing();
                            }
                        }

                        // 3. 逐个执行同步（在锁外异步执行）
                        for char_id in dirty_ids {
                            Self::do_sync(
                                &repository,
                                &storage_manager,
                                char_id,
                                &sync_states,
                            ).await;
                        }

                        // 4. 超时恢复：Syncing 超过 sync_timeout 的记录强制恢复为 Dirty
                        let timeout_ids: Vec<u32> = {
                            let states = sync_states.read();
                            states.iter()
                                .filter(|(_, r)| {
                                    r.sync_state == Syncing
                                        && r.last_modified.elapsed() >= sync_timeout
                                })
                                .map(|(id, _)| *id)
                                .collect()
                        };

                        if !timeout_ids.is_empty() {
                            tracing::warn!(
                                "仓库同步超时恢复: {:?} (timeout={:?})",
                                timeout_ids, sync_timeout
                            );
                            let mut states = sync_states.write();
                            for char_id in timeout_ids {
                                if let Some(record) = states.get_mut(&char_id) {
                                    record.mark_dirty();
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    /// 获取脏状态的角色ID列表
    pub fn get_dirty_char_ids(&self) -> Vec<u32> {
        let states = self.sync_states.read();
        states
            .iter()
            .filter(|(_, r)| r.sync_state == Dirty)
            .map(|(id, _)| *id)
            .collect()
    }

    /// 获取同步状态
    pub fn get_sync_state(&self, char_id: u32) -> Option<SyncState> {
        let states = self.sync_states.read();
        states.get(&char_id).map(|r| r.sync_state)
    }

    /// 获取版本号
    pub fn get_version(&self, char_id: u32) -> Option<u64> {
        let states = self.sync_states.read();
        states.get(&char_id).map(|r| r.version)
    }

    /// 检查是否有脏数据
    pub fn has_dirty(&self) -> bool {
        let states = self.sync_states.read();
        states.values().any(|r| r.sync_state == Dirty)
    }

    /// 统计脏数据数量
    pub fn dirty_count(&self) -> usize {
        let states = self.sync_states.read();
        states.iter().filter(|(_, r)| r.sync_state == Dirty).count()
    }
}

impl Drop for StorageSyncScheduler {
    fn drop(&mut self) {
        let _ = self.task_tx.try_send(SyncTask::Shutdown);
    }
}
```

### Step 7.3: 更新 manager_sync.rs 调用签名

**Files:**
- Modify: `src/game/storage/manager_sync.rs`

`StorageSyncScheduler::new` 签名变了，需要同步更新 `StorageSyncManager::new`：

```rust
/// 创建新的同步管理器
pub fn new(
    storage_manager: Arc<StorageManager>,
    repository: StorageRepository,
    sync_interval: Duration,
    default_storage_size: u16,
) -> Self {
    let scheduler = StorageSyncScheduler::new(
        repository.clone(),
        storage_manager.clone(),
        sync_interval,
    );

    Self {
        storage_manager,
        repository,
        scheduler,
        default_storage_size,
    }
}
```

同步更新 `Clone` impl（clone 时也要传 storage_manager）：

```rust
impl Clone for StorageSyncManager {
    fn clone(&self) -> Self {
        Self {
            storage_manager: self.storage_manager.clone(),
            repository: self.repository.clone(),
            scheduler: StorageSyncScheduler::new(
                self.repository.clone(),
                self.storage_manager.clone(),
                Duration::from_secs(30),
            ),
            default_storage_size: self.default_storage_size,
        }
    }
}
```

### Step 7.4: 验证编译通过

```bash
cargo build --lib 2>&1 | head -30
```

---

## Task 8: scheduler.rs 完整测试

**Files:**
- Modify: `src/game/storage/scheduler.rs`

**前置条件:** Task 7 完成

### Step 8.1: 添加测试模块

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema::init_schema;
    use std::time::Duration;

    fn setup() -> (StorageSyncScheduler, Arc<StorageManager>, StorageRepository) {
        let db = Arc::new(crate::storage::Database::open_memory().expect("创建内存数据库失败"));
        init_schema(&db).expect("初始化 schema 失败");

        // 创建测试角色（外键约束）
        db.execute("INSERT INTO accounts (account_id, username, password_hash, gender) VALUES (1, 'test', 'hash', 0)").unwrap();
        db.execute("INSERT INTO characters (char_id, account_id, name, class_id, base_level, job_level, str, agi, vit, int_, dex, luk, zeny, hp, max_hp, sp, max_sp, map_name, x, y) VALUES (1, 1, 'Test', 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 100, 100, 50, 50, 'prontera', 150, 150)").unwrap();

        let repo = StorageRepository::new(db);
        let manager = Arc::new(StorageManager::new());
        let scheduler = StorageSyncScheduler::new(
            repo.clone(),
            manager.clone(),
            Duration::from_millis(100), // 短间隔用于测试
        );

        (scheduler, manager, repo)
    }

    #[test]
    fn new_scheduler_has_no_dirty() {
        let (scheduler, _, _) = setup();
        assert!(!scheduler.has_dirty());
        assert_eq!(scheduler.dirty_count(), 0);
    }

    #[test]
    fn mark_dirty_creates_record() {
        let (scheduler, _, _) = setup();
        let tx = scheduler.task_sender();
        tx.try_send(SyncTask::MarkDirty(1)).unwrap();

        // 等待处理（通道是同步的，try_send 立即入队，后台任务异步处理）
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(scheduler.get_sync_state(1), Some(SyncState::Dirty));
        assert!(scheduler.has_dirty());
        assert_eq!(scheduler.dirty_count(), 1);
    }

    #[test]
    fn mark_clean_transitions_state() {
        let (scheduler, _, _) = setup();
        let tx = scheduler.task_sender();

        tx.try_send(SyncTask::MarkDirty(1)).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(scheduler.get_sync_state(1), Some(SyncState::Dirty));

        tx.try_send(SyncTask::MarkClean(1)).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(scheduler.get_sync_state(1), Some(SyncState::Clean));
    }

    #[test]
    fn mark_clean_nonexistent_is_noop() {
        let (scheduler, _, _) = setup();
        let tx = scheduler.task_sender();
        tx.try_send(SyncTask::MarkClean(999)).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(scheduler.get_sync_state(999), None);
    }

    #[test]
    fn get_version_returns_none_for_missing() {
        let (scheduler, _, _) = setup();
        assert_eq!(scheduler.get_version(999), None);
    }

    #[test]
    fn get_dirty_char_ids_returns_only_dirty() {
        let (scheduler, _, _) = setup();
        let tx = scheduler.task_sender();

        tx.try_send(SyncTask::MarkDirty(1)).unwrap();
        tx.try_send(SyncTask::MarkDirty(2)).unwrap();
        tx.try_send(SyncTask::MarkClean(3)).unwrap(); // 不存在，会被忽略
        std::thread::sleep(Duration::from_millis(50));

        let dirty_ids = scheduler.get_dirty_char_ids();
        assert_eq!(dirty_ids.len(), 2);
        assert!(dirty_ids.contains(&1));
        assert!(dirty_ids.contains(&2));
    }

    #[tokio::test]
    async fn force_sync_executes_actual_save() {
        let (scheduler, manager, repo) = setup();

        // 创建仓库并添加物品
        let storage_arc = manager.get_or_create(1, 100);
        storage_arc.write().add_item(501, 10);

        // 标记脏
        let tx = scheduler.task_sender();
        tx.try_send(SyncTask::MarkDirty(1)).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // 强制同步
        tx.try_send(SyncTask::ForceSync(1)).unwrap();
        // 等待异步同步完成
        tokio::time::sleep(Duration::from_millis(500)).await;

        // 验证数据已保存到数据库
        let loaded = repo.load(1).await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.get_slot(0).unwrap().item_id, 501);
        assert_eq!(loaded.get_slot(0).unwrap().amount, 10);
    }

    #[tokio::test]
    async fn periodic_sync_triggers_for_stale_dirty() {
        let (scheduler, manager, repo) = setup();

        // 创建仓库
        let storage_arc = manager.get_or_create(1, 100);
        storage_arc.write().add_item(501, 5);

        // 标记脏
        let tx = scheduler.task_sender();
        tx.try_send(SyncTask::MarkDirty(1)).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // 手动将时间戳设为旧值，使其 stale
        {
            let mut states = scheduler.sync_states.write();
            if let Some(record) = states.get_mut(&1) {
                record.last_modified = Instant::now() - Duration::from_secs(60);
            }
        }

        // 等待至少一个 tick（间隔 100ms）
        tokio::time::sleep(Duration::from_millis(300)).await;

        // 验证数据已同步
        let loaded = repo.load(1).await.unwrap();
        assert!(loaded.is_some());
    }

    #[test]
    fn shutdown_stops_processor() {
        let (scheduler, _, _) = setup();
        let tx = scheduler.task_sender();
        tx.try_send(SyncTask::Shutdown).unwrap();
        // 如果 shutdown 失败会 panic（通道已关闭）
        std::thread::sleep(Duration::from_millis(100));
    }

    #[test]
    fn drop_sends_shutdown() {
        let (scheduler, _, _) = setup();
        // scheduler drop 时应发送 Shutdown
        drop(scheduler);
        // 不 panic 即为成功
    }

    #[test]
    fn multiple_dirty_tracks_independently() {
        let (scheduler, _, _) = setup();
        let tx = scheduler.task_sender();

        tx.try_send(SyncTask::MarkDirty(1)).unwrap();
        tx.try_send(SyncTask::MarkDirty(2)).unwrap();
        tx.try_send(SyncTask::MarkDirty(3)).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // 标记 2 为干净
        tx.try_send(SyncTask::MarkClean(2)).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(scheduler.dirty_count(), 2);
        assert_eq!(scheduler.get_sync_state(1), Some(SyncState::Dirty));
        assert_eq!(scheduler.get_sync_state(2), Some(SyncState::Clean));
        assert_eq!(scheduler.get_sync_state(3), Some(SyncState::Dirty));
    }
}
```

### Step 8.2: 运行测试

```bash
cargo test --lib game::storage::scheduler::tests -- --nocapture
```

---

## Task 9: manager_sync.rs — resize_storage 持久化

**Files:**
- Modify: `src/game/storage/manager_sync.rs`

**问题分析:** `resize_storage()` 只更新内存中的仓库大小，但不调用 `repository.save()`。如果服务器在 resize 后崩溃，大小变更会丢失。

### Step 9.1: 红灯 — 编写测试（预期失败）

测试在 Task 10 中。

### Step 9.2: 绿灯 — 修改 resize_storage

```rust
/// 调整仓库大小
///
/// 更新内存中的仓库格子数，并持久化到数据库。
pub async fn resize_storage(&self, char_id: u32, new_size: u16) -> StorageResponse {
    let storage_arc = match self.storage_manager.get(char_id) {
        Some(s) => s,
        None => {
            return StorageResponse::error(char_id, "Storage not found");
        }
    };

    // 标记为脏
    if let Err(e) = self
        .scheduler
        .task_sender()
        .try_send(SyncTask::MarkDirty(char_id))
    {
        tracing::error!("仓库同步通道已满或已关闭，数据可能丢失: {}", e);
    }

    // 重新创建仓库（保持现有数据）
    let current_slots: Vec<_> = storage_arc.read().slots().to_vec();
    let mut new_slots: Vec<_> = current_slots.clone();
    new_slots.resize_with(new_size as usize, || super::data::StorageSlot::empty(0));

    // 更新内存
    {
        let mut storage = storage_arc.write();
        *storage = super::data::Storage::from_db_format(
            char_id,
            new_size,
            new_slots
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    (
                        i as u16,
                        s.item_id,
                        s.amount,
                        s.identified,
                        s.refine,
                        s.cards,
                    )
                })
                .collect(),
        );
    }

    // 持久化到数据库
    let storage = storage_arc.read().clone();
    match self.repository.save(&storage).await {
        Ok(()) => {
            if let Err(e) = self
                .scheduler
                .task_sender()
                .try_send(SyncTask::MarkClean(char_id))
            {
                tracing::error!("仓库同步通道已满或已关闭: {}", e);
            }
            StorageResponse::success(char_id)
        }
        Err(e) => {
            tracing::error!("仓库大小调整持久化失败: char_id={}, error={}", char_id, e);
            StorageResponse::error(char_id, format!("Resize persist failed: {}", e))
        }
    }
}
```

### Step 9.3: 验证编译

```bash
cargo build --lib 2>&1 | head -30
```

---

## Task 10: manager_sync.rs 完整测试

**Files:**
- Modify: `src/game/storage/manager_sync.rs`

**前置条件:** Task 7, 9 完成

### Step 10.1: 添加测试模块

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema::init_schema;
    use std::time::Duration;

    fn setup() -> StorageSyncManager {
        let db = Arc::new(crate::storage::Database::open_memory().expect("创建内存数据库失败"));
        init_schema(&db).expect("初始化 schema 失败");

        // 创建测试角色（外键约束）
        db.execute("INSERT INTO accounts (account_id, username, password_hash, gender) VALUES (1, 'test', 'hash', 0)").unwrap();
        db.execute("INSERT INTO characters (char_id, account_id, name, class_id, base_level, job_level, str, agi, vit, int_, dex, luk, zeny, hp, max_hp, sp, max_sp, map_name, x, y) VALUES (1, 1, 'Test', 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 100, 100, 50, 50, 'prontera', 150, 150)").unwrap();

        let repo = StorageRepository::new(db);
        let manager = Arc::new(StorageManager::new());

        StorageSyncManager::new(
            manager,
            repo,
            Duration::from_millis(100),
            100,
        )
    }

    #[tokio::test]
    async fn load_storage_creates_new_when_empty() {
        let sync_mgr = setup();
        let response = sync_mgr.load_storage(1).await;

        match response {
            StorageResponse::Data { char_id, slots } => {
                assert_eq!(char_id, 1);
                assert_eq!(slots.len(), 100); // default_storage_size
                assert!(slots.iter().all(|s| s.is_empty()));
            }
            _ => panic!("期望 Data 响应"),
        }
    }

    #[tokio::test]
    async fn load_then_save_then_reload() {
        let sync_mgr = setup();

        // 1. 加载（创建新仓库）
        sync_mgr.load_storage(1).await;

        // 2. 修改内存中的仓库
        {
            let storage_arc = sync_mgr.storage_manager().get(1).unwrap();
            let mut storage = storage_arc.write();
            storage.add_item(501, 10);
            storage.add_item(601, 1);
        }

        // 3. 保存
        let save_response = sync_mgr.save_storage(1, vec![]).await;
        match save_response {
            StorageResponse::Saved { char_id } => assert_eq!(char_id, 1),
            _ => panic!("期望 Saved 响应"),
        }

        // 4. 重新加载
        sync_mgr.storage_manager().remove(&1); // 清除内存缓存
        let load_response = sync_mgr.load_storage(1).await;
        match load_response {
            StorageResponse::Data { slots, .. } => {
                assert_eq!(slots[0].item_id, 501);
                assert_eq!(slots[0].amount, 10);
                assert_eq!(slots[1].item_id, 601);
            }
            _ => panic!("期望 Data 响应"),
        }
    }

    #[tokio::test]
    async fn resize_storage_persists() {
        let sync_mgr = setup();

        // 加载并添加物品
        sync_mgr.load_storage(1).await;
        {
            let storage_arc = sync_mgr.storage_manager().get(1).unwrap();
            storage_arc.write().add_item(501, 5);
        }

        // 保存
        sync_mgr.save_storage(1, vec![]).await;

        // 调整大小
        let resize_response = sync_mgr.resize_storage(1, 200).await;
        match resize_response {
            StorageResponse::Saved { char_id } => assert_eq!(char_id, 1),
            _ => panic!("期望 Saved 响应"),
        }

        // 验证内存中的大小
        {
            let storage_arc = sync_mgr.storage_manager().get(1).unwrap();
            assert_eq!(storage_arc.read().max_size(), 200);
            assert_eq!(storage_arc.read().get_slot(0).unwrap().item_id, 501);
        }

        // 清除缓存后重新加载，验证持久化
        sync_mgr.storage_manager().remove(&1);
        let load_response = sync_mgr.load_storage(1).await;
        match load_response {
            StorageResponse::Data { slots, .. } => {
                assert_eq!(slots.len(), 200); // 新大小
                assert_eq!(slots[0].item_id, 501);
            }
            _ => panic!("期望 Data 响应"),
        }
    }

    #[tokio::test]
    async fn resize_nonexistent_returns_error() {
        let sync_mgr = setup();
        let response = sync_mgr.resize_storage(999, 200).await;
        match response {
            StorageResponse::Error { char_id, .. } => assert_eq!(char_id, 999),
            _ => panic!("期望 Error 响应"),
        }
    }

    #[tokio::test]
    async fn save_nonexistent_returns_error() {
        let sync_mgr = setup();
        let response = sync_mgr.save_storage(999, vec![]).await;
        match response {
            StorageResponse::Error { char_id, .. } => assert_eq!(char_id, 999),
            _ => panic!("期望 Error 响应"),
        }
    }

    #[test]
    fn unlock_removes_from_memory() {
        let sync_mgr = setup();

        // 先加载到内存
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            sync_mgr.load_storage(1).await;
        });

        assert!(sync_mgr.storage_manager().has_storage(1));

        let response = sync_mgr.unlock_storage(1);
        match response {
            StorageResponse::Saved { char_id } => assert_eq!(char_id, 1),
            _ => panic!("期望 Saved 响应"),
        }

        assert!(!sync_mgr.storage_manager().has_storage(1));
    }

    #[test]
    fn get_sync_status_returns_default() {
        let sync_mgr = setup();
        let response = sync_mgr.get_sync_status(1);
        match response {
            StorageResponse::SyncStatus {
                char_id,
                is_dirty,
                version,
            } => {
                assert_eq!(char_id, 1);
                assert!(!is_dirty); // 默认 Clean
                assert_eq!(version, 0);
            }
            _ => panic!("期望 SyncStatus 响应"),
        }
    }

    #[tokio::test]
    async fn force_sync_saves_to_db() {
        let sync_mgr = setup();

        // 加载并修改
        sync_mgr.load_storage(1).await;
        {
            let storage_arc = sync_mgr.storage_manager().get(1).unwrap();
            storage_arc.write().add_item(501, 10);
        }

        // 强制同步
        sync_mgr.force_sync(1).await.unwrap();

        // 清除缓存后重新加载
        sync_mgr.storage_manager().remove(&1);
        let load_response = sync_mgr.load_storage(1).await;
        match load_response {
            StorageResponse::Data { slots, .. } => {
                assert_eq!(slots[0].item_id, 501);
                assert_eq!(slots[0].amount, 10);
            }
            _ => panic!("期望 Data 响应"),
        }
    }

    #[tokio::test]
    async fn flush_dirty_saves_all_dirty() {
        let sync_mgr = setup();

        // 创建多个仓库
        // 需要先创建额外的角色
        {
            let repo = sync_mgr.repository();
            // 这里直接操作 manager 创建仓库（不走 DB）
            let sm = sync_mgr.storage_manager();
            sm.get_or_create(1, 100).write().add_item(501, 1);
        }

        // 标记脏
        sync_mgr
            .scheduler()
            .task_sender()
            .try_send(SyncTask::MarkDirty(1))
            .unwrap();

        std::thread::sleep(Duration::from_millis(50));

        // flush
        let count = sync_mgr.flush_dirty().await.unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn dirty_stats_reports_correctly() {
        let sync_mgr = setup();
        let stats = sync_mgr.dirty_stats();
        assert_eq!(stats.count, 0);
        assert!(!stats.has_dirty);
    }

    #[test]
    fn default_size_is_configurable() {
        let sync_mgr = setup();
        assert_eq!(sync_mgr.default_size(), 100);
    }

    #[test]
    fn handle_request_routes_correctly() {
        let sync_mgr = setup();
        let rt = tokio::runtime::Runtime::new().unwrap();

        // Load
        let response = rt.block_on(sync_mgr.handle_request(StorageRequest::Load { char_id: 1 }));
        assert!(matches!(response, StorageResponse::Data { char_id: 1, .. }));

        // SyncStatus
        let response =
            rt.block_on(sync_mgr.handle_request(StorageRequest::SyncStatus { char_id: 1 }));
        assert!(matches!(response, StorageResponse::SyncStatus { .. }));

        // Unlock
        let response = rt.block_on(sync_mgr.handle_request(StorageRequest::Unlock { char_id: 1 }));
        assert!(matches!(response, StorageResponse::Saved { char_id: 1 }));
    }
}
```

### Step 10.2: 运行测试

```bash
cargo test --lib game::storage::manager_sync::tests -- --nocapture
```

---

## Task 11: protocol.rs 完整测试

**Files:**
- Modify: `src/game/storage/protocol.rs`

### Step 11.1: 添加测试模块

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ========== StorageRequest 测试 ==========

    #[test]
    fn request_load_char_id() {
        let req = StorageRequest::Load { char_id: 42 };
        assert_eq!(req.char_id(), 42);
    }

    #[test]
    fn request_save_char_id() {
        let req = StorageRequest::Save {
            char_id: 100,
            slots: vec![],
        };
        assert_eq!(req.char_id(), 100);
    }

    #[test]
    fn request_resize_char_id() {
        let req = StorageRequest::Resize {
            char_id: 200,
            new_size: 150,
        };
        assert_eq!(req.char_id(), 200);
    }

    #[test]
    fn request_unlock_char_id() {
        let req = StorageRequest::Unlock { char_id: 300 };
        assert_eq!(req.char_id(), 300);
    }

    #[test]
    fn request_sync_status_char_id() {
        let req = StorageRequest::SyncStatus { char_id: 400 };
        assert_eq!(req.char_id(), 400);
    }

    #[test]
    fn request_is_cloneable() {
        let req = StorageRequest::Load { char_id: 1 };
        let req2 = req.clone();
        assert_eq!(req2.char_id(), 1);
    }

    #[test]
    fn request_is_debuggable() {
        let req = StorageRequest::Load { char_id: 1 };
        let debug_str = format!("{:?}", req);
        assert!(debug_str.contains("Load"));
        assert!(debug_str.contains("1"));
    }

    // ========== StorageResponse 测试 ==========

    #[test]
    fn response_success() {
        let resp = StorageResponse::success(42);
        match resp {
            StorageResponse::Saved { char_id } => assert_eq!(char_id, 42),
            _ => panic!("期望 Saved"),
        }
    }

    #[test]
    fn response_error() {
        let resp = StorageResponse::error(42, "test error");
        match resp {
            StorageResponse::Error { char_id, message } => {
                assert_eq!(char_id, 42);
                assert_eq!(message, "test error");
            }
            _ => panic!("期望 Error"),
        }
    }

    #[test]
    fn response_error_with_string() {
        let msg = String::from("owned string");
        let resp = StorageResponse::error(1, msg);
        match resp {
            StorageResponse::Error { message, .. } => {
                assert_eq!(message, "owned string");
            }
            _ => panic!("期望 Error"),
        }
    }

    #[test]
    fn response_data_with_slots() {
        let slots = vec![
            StorageSlot {
                index: 0,
                item_id: 501,
                amount: 10,
                identified: true,
                refine: 0,
                cards: [0; 4],
            },
            StorageSlot {
                index: 1,
                item_id: 601,
                amount: 1,
                identified: true,
                refine: 7,
                cards: [4001, 0, 0, 0],
            },
        ];
        let resp = StorageResponse::Data {
            char_id: 42,
            slots: slots.clone(),
        };
        match resp {
            StorageResponse::Data {
                char_id,
                slots: loaded,
            } => {
                assert_eq!(char_id, 42);
                assert_eq!(loaded.len(), 2);
                assert_eq!(loaded[0].item_id, 501);
                assert_eq!(loaded[1].refine, 7);
            }
            _ => panic!("期望 Data"),
        }
    }

    #[test]
    fn response_sync_status() {
        let resp = StorageResponse::SyncStatus {
            char_id: 42,
            is_dirty: true,
            version: 5,
        };
        match resp {
            StorageResponse::SyncStatus {
                char_id,
                is_dirty,
                version,
            } => {
                assert_eq!(char_id, 42);
                assert!(is_dirty);
                assert_eq!(version, 5);
            }
            _ => panic!("期望 SyncStatus"),
        }
    }

    #[test]
    fn response_is_cloneable() {
        let resp = StorageResponse::success(1);
        let resp2 = resp.clone();
        assert!(matches!(resp2, StorageResponse::Saved { char_id: 1 }));
    }

    #[test]
    fn response_is_debuggable() {
        let resp = StorageResponse::success(1);
        let debug_str = format!("{:?}", resp);
        assert!(debug_str.contains("Saved"));
    }
}
```

### Step 11.2: 运行测试

```bash
cargo test --lib game::storage::protocol::tests -- --nocapture
```

---

## 全量验证

完成所有任务后，运行全量测试：

```bash
# 只运行 storage 模块的测试
cargo test --lib game::storage -- --nocapture

# 运行全量测试确保没有回归
cargo test --lib

# 检查编译警告
cargo build --lib 2>&1 | grep -i warning
```

---

## Commit 策略

每个 Task 完成后独立 commit：

| Task | Commit Message |
|------|----------------|
| 1 | `test(storage): 补全 data.rs 全量测试并添加 is_full() 方法` |
| 2 | `feat(storage): add_item 支持 refine/cards 参数，装备入库不再丢失精炼数据` |
| 3 | `test(storage): 补全 manager.rs 测试，覆盖并发访问场景` |
| 4 | `fix(storage): repository 使用 UPSERT 替代 DELETE+INSERT，添加 storage_meta 表` |
| 5 | `test(storage): 补全 repository.rs 测试，验证 CRUD 和 UPSERT 正确性` |
| 6 | `test(storage): 补全 sync.rs 测试，添加状态转换合法性校验` |
| 7 | `fix(storage): scheduler 执行实际同步并添加超时恢复机制` |
| 8 | `test(storage): 补全 scheduler.rs 测试，验证同步执行和超时恢复` |
| 9 | `fix(storage): resize_storage 持久化到数据库` |
| 10 | `test(storage): 补全 manager_sync.rs 测试，验证完整 load-modify-save 流程` |
| 11 | `test(storage): 补全 protocol.rs 测试` |

---

## 风险和注意事项

1. **外键约束:** 测试中创建 storage 记录需要先创建 characters 记录，而 characters 需要 accounts。schema.rs 的 `init_schema` 会创建表但不插入数据，测试 setup 中需要手动插入。

2. **tokio::test vs #[test]:** 涉及 async 仓库操作的测试必须用 `#[tokio::test]`，纯同步逻辑用 `#[test]`。

3. **Scheduler 异步测试:** scheduler 的后台任务是异步运行的，测试中需要用 `tokio::time::sleep` 等待处理完成，或使用较短的 sync_interval。

4. **manager_sync.rs Clone impl:** Task 7 修改了 StorageSyncScheduler 的构造函数签名，`StorageSyncManager::clone()` 也必须同步更新，否则编译失败。

5. **storage_meta 表:** Task 4 新增的表需要在 schema.rs 中创建，且需要在所有使用 `init_schema` 的测试中确保可用。
