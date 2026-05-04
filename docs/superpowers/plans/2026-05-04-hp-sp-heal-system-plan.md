# HP/SP 自然回复系统实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现玩家自然 HP/SP 回复系统，支持站立和坐下回复，VIT/INT 属性加成

**Architecture:** 基于 Timer 的定时服务，定时遍历所有在线玩家并应用回复公式

**Tech Stack:** Rust, tokio, parking_lot, 复用现有 Timer 系统

---

## Task 1: Player 添加坐下状态

**Files:**
- Modify: `src/game/map/player.rs`

- [ ] **Step 1: 添加 is_sitting 字段**

在 Player 结构体中添加:
```rust
pub is_sitting: RwLock<bool>,
```

- [ ] **Step 2: 添加 sit/stand 方法**

```rust
pub fn sit(&self) {
    *self.is_sitting.write() = true;
}

pub fn stand(&self) {
    *self.is_sitting.write() = false;
}

pub fn is_sitting(&self) -> bool {
    *self.is_sitting.read()
}
```

- [ ] **Step 3: 修改 new() 初始化**

在 new() 中添加: `is_sitting: RwLock::new(false),`

- [ ] **Step 4: 编译验证**

Run: `cargo build 2>&1 | head -30`
Expected: 无错误

- [ ] **Step 5: 提交**

```bash
git add src/game/map/player.rs
git commit -m "feat(player): add is_sitting state for heal system"
```

---

## Task 2: 创建 HealService

**Files:**
- Create: `src/game/heal/mod.rs`
- Create: `src/game/heal/service.rs`

- [ ] **Step 1: 创建 heal 模块目录**

```bash
mkdir -p src/game/heal
```

- [ ] **Step 2: 创建 mod.rs**

```rust
pub mod service;

pub use service::HealService;
```

- [ ] **Step 3: 创建 service.rs 骨架**

```rust
use std::sync::Arc;
use crate::core::Config;
use crate::game::map::{MapState, Player};

pub struct HealService {
    config: Arc<Config>,
}

impl HealService {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    pub fn start(&self, map_state: Arc<MapState>) {
        // TODO: 启动定时器
    }

    fn process_heal(&self, map_state: &MapState) {
        // TODO: 遍历玩家应用回复
    }

    pub fn calculate_hp_heal(&self, player: &Player, is_sitting: bool) -> u32 {
        // TODO: 计算 HP 回复量
        0
    }

    pub fn calculate_sp_heal(&self, player: &Player, is_sitting: bool) -> u32 {
        // TODO: 计算 SP 回复量
        0
    }
}
```

- [ ] **Step 4: 实现计算公式**

```rust
pub fn calculate_hp_heal(&self, player: &Player, is_sitting: bool) -> u32 {
    let vit = *player.vit.read() as u32;
    let max_hp = *player.max_hp.read();
    let base_heal = 1 + (vit / 2) + (max_hp / 200);

    let rate = if is_sitting {
        self.config.battle.sit_heal_hp_rate
    } else {
        self.config.battle.natural_heal_hp_rate
    };

    let rate_heal = (max_hp * rate) / 100;
    base_heal + rate_heal
}

pub fn calculate_sp_heal(&self, player: &Player, is_sitting: bool) -> u32 {
    let int = *player.int.read() as u32;
    let max_sp = *player.max_sp.read();
    let base_heal = 1 + (int / 2) + (max_sp / 100);

    let rate = if is_sitting {
        self.config.battle.sit_heal_sp_rate
    } else {
        self.config.battle.natural_heal_sp_rate
    };

    let rate_heal = (max_sp * rate) / 100;
    base_heal + rate_heal
}
```

- [ ] **Step 5: 实现 process_heal**

```rust
fn process_heal(&self, map_state: &MapState) {
    let threshold_hp = self.config.battle.natural_heal_threshold_hp;
    let threshold_sp = self.config.battle.natural_heal_threshold_sp;

    let players: Vec<_> = map_state.get_all_players().values().cloned().collect();

    for player in players {
        // 跳过死亡玩家
        if *player.state.read() == PlayerState::Dead {
            continue;
        }

        let is_sitting = player.is_sitting();
        let current_hp = *player.hp.read();
        let max_hp = *player.max_hp.read();
        let current_sp = *player.sp.read();
        let max_sp = *player.max_sp.read();

        // HP 回复
        if current_hp < max_hp {
            let hp_threshold = (max_hp * threshold_hp) / 100;
            if current_hp >= hp_threshold {
                let heal = self.calculate_hp_heal(&player, is_sitting);
                let new_hp = (current_hp + heal).min(max_hp);
                *player.hp.write() = new_hp;
                tracing::debug!("Player {} healed {} HP (sitting: {})", player.name, heal, is_sitting);
            }
        }

        // SP 回复
        if current_sp < max_sp {
            let sp_threshold = (max_sp * threshold_sp) / 100;
            if current_sp >= sp_threshold {
                let heal = self.calculate_sp_heal(&player, is_sitting);
                let new_sp = (current_sp + heal).min(max_sp);
                *player.sp.write() = new_sp;
                tracing::debug!("Player {} healed {} SP (sitting: {})", player.name, heal, is_sitting);
            }
        }
    }
}
```

- [ ] **Step 6: 实现 start() 启动定时器**

```rust
pub fn start(&self, map_state: Arc<MapState>) {
    let interval_ms = self.config.battle.natural_heal_interval_ms;
    let service = Arc::new(self.clone());

    crate::core::timer::Timer::add_interval(
        std::time::Duration::from_millis(interval_ms),
        move || {
            service.process_heal(&map_state);
        }
    );

    tracing::info!("HealService started with interval {}ms", interval_ms);
}
```

- [ ] **Step 7: 添加 Clone derive**

```rust
#[derive(Clone)]
pub struct HealService {
    config: Arc<Config>,
}
```

- [ ] **Step 8: 编译验证**

Run: `cargo build 2>&1 | head -50`
Expected: 无错误

- [ ] **Step 9: 提交**

```bash
git add src/game/heal/
git commit -m "feat(heal): add HealService for natural HP/SP recovery"
```

---

## Task 3: 集成到 Core

**Files:**
- Modify: `src/core/mod.rs`
- Modify: `src/game/mod.rs`

- [ ] **Step 1: 添加 heal 模块到 game/mod.rs**

```rust
pub mod heal;
```

- [ ] **Step 2: 修改 Core 结构体**

添加: `heal_service: Arc<heal::HealService>,`

- [ ] **Step 3: 修改 Core::new()**

```rust
heal_service: Arc::new(heal::HealService::new(Arc::new(config.clone()))),
```

- [ ] **Step 4: 修改 Core::run()**

```rust
// 启动回复服务
self.heal_service.start(self.map_state.clone());
```

- [ ] **Step 5: 编译验证**

Run: `cargo build 2>&1 | head -50`
Expected: 无错误

- [ ] **Step 6: 提交**

```bash
git add src/core/mod.rs src/game/mod.rs
git commit -m "feat(core): integrate HealService into server startup"
```

---

## Task 4: 测试验证

- [ ] **Step 1: 启动服务器**

Run: `cargo run -- 2>&1 | head -30`

Expected: 显示 "HealService started with interval 6000ms"

- [ ] **Step 2: 连接客户端并观察日志**

启动 devi 客户端，查看服务端日志是否有 HP/SP 回复调试信息

- [ ] **Step 3: 验证完成**

确认回复系统正常工作后提交最终代码
