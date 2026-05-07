# Devi 客户端基础架构实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 搭建 Devi 客户端的项目骨架、核心基础设施（配置、状态机、tick 循环），使其能启动并显示一个空白窗口。

**Architecture:** 基于 Bevy ECS 引擎，使用 Rust 实现。项目结构按模块划分：core（基础设施）、net（网络）、asset（资源）、render（渲染）、game（游戏逻辑）、protocol（协议）。

**Tech Stack:** Rust, Bevy 0.15+, wgpu, tokio (异步网络)

---

## 文件结构

| 操作 | 文件路径 | 职责 |
|------|----------|------|
| Create | `devi/Cargo.toml` | 项目依赖配置 |
| Create | `devi/src/main.rs` | 入口，初始化 Bevy App |
| Create | `devi/src/lib.rs` | 模块导出 |
| Create | `devi/src/core/mod.rs` | core 模块声明 |
| Create | `devi/src/core/config.rs` | 客户端配置（分辨率、服务器地址） |
| Create | `devi/src/core/state.rs` | 游戏状态机（登录/选角色/游戏中） |
| Create | `devi/src/core/tick.rs` | 固定 tick 循环（20ms） |
| Create | `devi/src/net/mod.rs` | net 模块声明（占位） |
| Create | `devi/src/asset/mod.rs` | asset 模块声明（占位） |
| Create | `devi/src/render/mod.rs` | render 模块声明（占位） |
| Create | `devi/src/game/mod.rs` | game 模块声明（占位） |
| Create | `devi/src/protocol/mod.rs` | protocol 模块声明（占位） |
| Create | `devi/tests/core_test.rs` | 核心模块集成测试 |

---

### Task 1: 项目脚手架

**Files:**
- Create: `devi/Cargo.toml`
- Create: `devi/src/main.rs`
- Create: `devi/src/lib.rs`
- Create: `devi/src/core/mod.rs`
- Create: `devi/src/net/mod.rs`
- Create: `devi/src/asset/mod.rs`
- Create: `devi/src/render/mod.rs`
- Create: `devi/src/game/mod.rs`
- Create: `devi/src/protocol/mod.rs`

- [ ] **Step 1: 创建 Cargo.toml**

```toml
[package]
name = "devi"
version = "0.1.0"
edition = "2021"
description = "Devi - 现代化 RO 客户端"

[dependencies]
bevy = { version = "0.15", features = ["wayland"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
tracing = "0.1"
tracing-subscriber = "0.3"
async-trait = "0.1"
flate2 = "1"            # GRF zlib 解压
byteorder = "1"         # 字节序处理
image = "0.25"          # 图片处理
tokio-tungstenite = { version = "0.24", features = ["native-tls"] }  # WebSocket
futures-util = "0.3"    # 异步流处理

[dev-dependencies]
bevy = { version = "0.15", features = ["wayland"] }
```

- [ ] **Step 2: 创建模块占位文件**

创建 `devi/src/core/mod.rs`:
```rust
// 核心基础设施模块
pub mod config;
pub mod state;
pub mod tick;
```

创建 `devi/src/net/mod.rs`:
```rust
// 网络模块（后续实现）
```

创建 `devi/src/asset/mod.rs`:
```rust
// 资源系统模块（后续实现）
```

创建 `devi/src/render/mod.rs`:
```rust
// 渲染管线模块（后续实现）
```

创建 `devi/src/game/mod.rs`:
```rust
// 游戏逻辑模块（后续实现）
```

创建 `devi/src/protocol/mod.rs`:
```rust
// 协议定义模块（后续实现）
```

- [ ] **Step 3: 创建 lib.rs**

创建 `devi/src/lib.rs`:
```rust
// Devi 客户端库
pub mod core;
pub mod net;
pub mod asset;
pub mod render;
pub mod game;
pub mod protocol;
```

- [ ] **Step 4: 创建 main.rs**

创建 `devi/src/main.rs`:
```rust
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Devi - Ragnarok Online".to_string(),
                resolution: (1024.0, 768.0).into(),
                ..default()
            }),
            ..default()
        }))
        .run();
}
```

- [ ] **Step 5: 验证编译**

Run: `cd devi && cargo check`
Expected: 编译通过，无错误

- [ ] **Step 6: Commit**

```bash
git add devi/
git commit -m "feat(devi): 初始化项目脚手架，Bevy 空窗口"
```

---

### Task 2: 配置系统

**Files:**
- Create: `devi/src/core/config.rs`
- Create: `devi/tests/core_test.rs`

- [ ] **Step 1: 编写配置系统测试**

创建 `devi/tests/core_test.rs`:
```rust
use devi::core::config::ClientConfig;

#[test]
fn test_default_config() {
    let config = ClientConfig::default();
    assert_eq!(config.window_width, 1024);
    assert_eq!(config.window_height, 768);
    assert_eq!(config.server_address, "127.0.0.1");
    assert_eq!(config.protocol, "modern");
}

#[test]
fn test_config_from_yaml() {
    let yaml = r#"
window_width: 1920
window_height: 1080
server_address: "192.168.1.100"
protocol: "legacy"
"#;
    let config: ClientConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.window_width, 1920);
    assert_eq!(config.window_height, 1080);
    assert_eq!(config.server_address, "192.168.1.100");
    assert_eq!(config.protocol, "legacy");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test core_test`
Expected: FAIL，编译错误 `unresolved import devi::core::config`

- [ ] **Step 3: 实现配置系统**

创建 `devi/src/core/config.rs`:
```rust
use serde::Deserialize;

/// 客户端配置
#[derive(Debug, Clone, Deserialize)]
pub struct ClientConfig {
    /// 窗口宽度
    pub window_width: u32,
    /// 窗口高度
    pub window_height: u32,
    /// 服务器地址
    pub server_address: String,
    /// 协议类型："modern" (WebSocket) 或 "legacy" (TCP)
    pub protocol: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            window_width: 1024,
            window_height: 768,
            server_address: "127.0.0.1".to_string(),
            protocol: "modern".to_string(),
        }
    }
}
```

- [ ] **Step 4: 更新 lib.rs 导出**

修改 `devi/src/lib.rs`:
```rust
pub mod core;
pub mod net;
pub mod asset;
pub mod render;
pub mod game;
pub mod protocol;
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cd devi && cargo test --test core_test`
Expected: 2 tests PASS

- [ ] **Step 6: Commit**

```bash
git add devi/src/core/config.rs devi/tests/core_test.rs
git commit -m "feat(devi/core): 实现客户端配置系统，支持 YAML 反序列化"
```

---

### Task 3: 游戏状态机

**Files:**
- Modify: `devi/src/core/state.rs` (create)
- Modify: `devi/tests/core_test.rs`

- [ ] **Step 1: 编写状态机测试**

在 `devi/tests/core_test.rs` 中追加:
```rust
use devi::core::state::GameState;

#[test]
fn test_initial_state_is_login() {
    let state = GameState::default();
    assert_eq!(state, GameState::Login);
}

#[test]
fn test_state_transitions() {
    let mut state = GameState::default();
    assert_eq!(state, GameState::Login);

    state = GameState::CharSelect;
    assert_eq!(state, GameState::CharSelect);

    state = GameState::InGame;
    assert_eq!(state, GameState::InGame);

    state = GameState::Login;
    assert_eq!(state, GameState::Login);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test core_test`
Expected: FAIL，`unresolved import devi::core::state`

- [ ] **Step 3: 实现状态机**

创建 `devi/src/core/state.rs`:
```rust
use bevy::prelude::States;

/// 游戏状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States)]
pub enum GameState {
    /// 登录界面
    #[default]
    Login,
    /// 选角色界面
    CharSelect,
    /// 游戏中
    InGame,
}
```

- [ ] **Step 4: 更新 core/mod.rs**

修改 `devi/src/core/mod.rs`:
```rust
pub mod config;
pub mod state;
pub mod tick;
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cd devi && cargo test --test core_test`
Expected: 所有测试 PASS

- [ ] **Step 6: Commit**

```bash
git add devi/src/core/state.rs devi/tests/core_test.rs
git commit -m "feat(devi/core): 实现游戏状态机（Login/CharSelect/InGame）"
```

---

### Task 4: 固定 Tick 循环

**Files:**
- Create: `devi/src/core/tick.rs`
- Modify: `devi/tests/core_test.rs`
- Modify: `devi/src/main.rs`

- [ ] **Step 1: 编写 tick 循环测试**

在 `devi/tests/core_test.rs` 中追加:
```rust
use devi::core::tick::TickConfig;

#[test]
fn test_tick_config_default() {
    let config = TickConfig::default();
    assert_eq!(config.tick_rate_ms, 20);
    assert!((config.tick_rate_hz - 50.0).abs() < f64::EPSILON);
}

#[test]
fn test_tick_config_custom() {
    let config = TickConfig::new(16);
    assert_eq!(config.tick_rate_ms, 16);
    assert!((config.tick_rate_hz - 62.5).abs() < f64::EPSILON);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test core_test`
Expected: FAIL，`unresolved import devi::core::tick`

- [ ] **Step 3: 实现 tick 配置**

创建 `devi/src/core/tick.rs`:
```rust
/// Tick 配置，控制游戏逻辑更新频率
#[derive(Debug, Clone)]
pub struct TickConfig {
    /// 每 tick 毫秒数
    pub tick_rate_ms: u32,
    /// 每秒 tick 数
    pub tick_rate_hz: f64,
}

impl Default for TickConfig {
    fn default() -> Self {
        Self::new(20)
    }
}

impl TickConfig {
    /// 创建自定义 tick 配置
    pub fn new(tick_rate_ms: u32) -> Self {
        Self {
            tick_rate_ms,
            tick_rate_hz: 1000.0 / tick_rate_ms as f64,
        }
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd devi && cargo test --test core_test`
Expected: 所有测试 PASS

- [ ] **Step 5: 集成状态机到 main.rs**

修改 `devi/src/main.rs`:
```rust
use bevy::prelude::*;
use devi::core::state::GameState;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Devi - Ragnarok Online".to_string(),
                resolution: (1024.0, 768.0).into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .add_systems(Startup, setup)
        .add_systems(Update, log_state_changes)
        .run();
}

fn setup(mut commands: Commands) {
    // 初始相机
    commands.spawn(Camera2d::bundle(Transform::default()));
}

fn log_state_changes(state: Res<State<GameState>>) {
    if state.is_changed() {
        tracing::info!("游戏状态切换: {:?}", state.get());
    }
}
```

- [ ] **Step 6: 验证编译和运行**

Run: `cd devi && cargo check`
Expected: 编译通过

- [ ] **Step 7: Commit**

```bash
git add devi/src/core/tick.rs devi/src/main.rs devi/tests/core_test.rs
git commit -m "feat(devi/core): 实现固定 tick 配置，集成状态机到主循环"
```

---

### Task 5: 集成测试完善

**Files:**
- Modify: `devi/tests/core_test.rs`

- [ ] **Step 1: 完善集成测试**

替换 `devi/tests/core_test.rs` 为完整版本:
```rust
use devi::core::config::ClientConfig;
use devi::core::state::GameState;
use devi::core::tick::TickConfig;

// ===== 配置系统测试 =====

#[test]
fn test_default_config() {
    let config = ClientConfig::default();
    assert_eq!(config.window_width, 1024);
    assert_eq!(config.window_height, 768);
    assert_eq!(config.server_address, "127.0.0.1");
    assert_eq!(config.protocol, "modern");
}

#[test]
fn test_config_from_yaml() {
    let yaml = r#"
window_width: 1920
window_height: 1080
server_address: "192.168.1.100"
protocol: "legacy"
"#;
    let config: ClientConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.window_width, 1920);
    assert_eq!(config.window_height, 1080);
    assert_eq!(config.server_address, "192.168.1.100");
    assert_eq!(config.protocol, "legacy");
}

// ===== 状态机测试 =====

#[test]
fn test_initial_state_is_login() {
    let state = GameState::default();
    assert_eq!(state, GameState::Login);
}

#[test]
fn test_state_transitions() {
    let mut state = GameState::default();
    assert_eq!(state, GameState::Login);

    state = GameState::CharSelect;
    assert_eq!(state, GameState::CharSelect);

    state = GameState::InGame;
    assert_eq!(state, GameState::InGame);

    state = GameState::Login;
    assert_eq!(state, GameState::Login);
}

// ===== Tick 配置测试 =====

#[test]
fn test_tick_config_default() {
    let config = TickConfig::default();
    assert_eq!(config.tick_rate_ms, 20);
    assert!((config.tick_rate_hz - 50.0).abs() < f64::EPSILON);
}

#[test]
fn test_tick_config_custom() {
    let config = TickConfig::new(16);
    assert_eq!(config.tick_rate_ms, 16);
    assert!((config.tick_rate_hz - 62.5).abs() < f64::EPSILON);
}
```

- [ ] **Step 2: 运行全部测试**

Run: `cd devi && cargo test --test core_test`
Expected: 6 tests PASS

- [ ] **Step 3: Commit**

```bash
git add devi/tests/core_test.rs
git commit -m "test(devi/core): 完善核心模块集成测试，覆盖配置/状态/tick"
```
