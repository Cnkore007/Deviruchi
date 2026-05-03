# Devi Client MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 创建可运行的 Devi 客户端 MVP，实现连接服务器、显示角色、方向键移动

**Architecture:** Bevy 0.14+ 作为游戏主循环 + Tokio Runtime 处理 WebSocket 网络，Channel 桥接两者

**Tech Stack:** Bevy, wgpu, tokio, tokio-tungstenite

---

## 文件结构

```
Deviruchi/
├── Cargo.toml                    # 修改：添加 workspace members
├── devi/                         # 新建：客户端 crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               # Bevy App 入口
│       ├── lib.rs                # 模块导出
│       ├── network/
│       │   ├── mod.rs
│       │   ├── client.rs         # WebSocket 客户端
│       │   └── protocol.rs       # 协议定义
│       ├── game/
│       │   ├── mod.rs
│       │   ├── player.rs         # Player 组件
│       │   ├── map.rs            # 地图生成和组件
│       │   └── input.rs          # 输入系统
│       └── render/
│           ├── mod.rs
│           ├── tile.rs           # 瓦片渲染
│           └── camera.rs         # 摄像机跟随
```

---

## Task 1: 项目搭建

**Files:**
- Create: `devi/Cargo.toml`
- Modify: `Cargo.toml`
- Create: `devi/src/main.rs`
- Create: `devi/src/lib.rs`

- [ ] **Step 1: 创建 devi/Cargo.toml**

```toml
[package]
name = "devi"
version = "0.1.0"
edition = "2021"

[dependencies]
bevy = "0.14"
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.21"
futures-util = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"

[profile.dev]
opt-level = 0
debug = true
```

- [ ] **Step 2: 修改 Cargo.toml 添加 workspace members**

```toml
[workspace]
members = ["devi"]
resolver = "2"
```

- [ ] **Step 3: 创建 devi/src/main.rs**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .run();
}
```

- [ ] **Step 4: 创建 devi/src/lib.rs**

```rust
pub mod network;
pub mod game;
pub mod render;
```

- [ ] **Step 5: 验证编译**

Run: `cargo build -p devi`
Expected: SUCCESS (可执行文件会在 target/debug/devi)

- [ ] **Step 6: 提交**

```bash
git add Cargo.toml devi/
git commit -m "feat(devi): scaffold Bevy project structure"
```

---

## Task 2: 基础渲染 - 地图和角色

**Files:**
- Create: `devi/src/game/mod.rs`
- Create: `devi/src/game/map.rs`
- Create: `devi/src/game/player.rs`
- Create: `devi/src/render/mod.rs`
- Create: `devi/src/render/tile.rs`
- Modify: `devi/src/main.rs`

- [ ] **Step 1: 创建 game/mod.rs**

```rust
pub mod map;
pub mod player;
```

- [ ] **Step 2: 创建 game/map.rs - 地图组件和生成**

```rust
use bevy::prelude::*;

#[derive(Component)]
pub struct MapTile {
    pub x: u32,
    pub y: u32,
}

#[derive(Resource)]
pub struct GameMap {
    pub width: u32,
    pub height: u32,
    pub tile_size: f32,
}

impl GameMap {
    pub fn new(width: u32, height: u32, tile_size: f32) -> Self {
        Self { width, height, tile_size }
    }
}

pub fn generate_test_map() -> Vec<MapTile> {
    let mut tiles = Vec::new();
    for y in 0..20 {
        for x in 0..30 {
            tiles.push(MapTile { x, y });
        }
    }
    tiles
}
```

- [ ] **Step 3: 创建 game/player.rs - 玩家组件**

```rust
use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub id: u32,
    pub name: String,
}

#[derive(Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct LocalPlayer;
```

- [ ] **Step 4: 创建 render/mod.rs**

```rust
pub mod tile;
```

- [ ] **Step 5: 创建 render/tile.rs - 瓦片和角色渲染系统**

```rust
use bevy::prelude::*;
use crate::game::map::{GameMap, MapTile, generate_test_map};
use crate::game::player::{Player, Position, LocalPlayer};

pub fn setup_map(mut commands: Commands) {
    commands.insert_resource(GameMap::new(30, 20, 32.0));

    let tiles = generate_test_map();
    for tile in tiles {
        commands.spawn((
            MapTile { x: tile.x, y: tile.y },
            Transform::from_xyz(
                tile.x as f32 * 32.0,
                tile.y as f32 * 32.0,
                0.0,
            ),
            Sprite {
                color: Color::rgb(0.3, 0.3, 0.35),
                custom_size: Some(Vec2::new(32.0, 32.0)),
                ..default()
            },
        ));
    }
}

pub fn setup_local_player(mut commands: Commands) {
    commands.spawn((
        Player { id: 0, name: "LocalPlayer".to_string() },
        Position { x: 5.0, y: 5.0 },
        LocalPlayer,
        Transform::from_xyz(5.0 * 32.0, 5.0 * 32.0, 1.0),
        Sprite {
            color: Color::rgb(0.2, 0.8, 0.2), // 绿色
            custom_size: Some(Vec2::new(28.0, 28.0)),
            ..default()
        },
    ));
}
```

- [ ] **Step 6: 修改 main.rs 添加系统**

```rust
use bevy::prelude::*;
use devi::render::tile::{setup_map, setup_local_player};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_map, setup_local_player))
        .run();
}
```

- [ ] **Step 7: 验证渲染**

Run: `cargo run -p devi`
Expected: 窗口打开，显示 30x20 的灰色格子地图，中心有绿色方块（本地玩家）

- [ ] **Step 8: 提交**

```bash
git add devi/
git commit -m "feat(devi): add basic map and player rendering"
```

---

## Task 3: 输入系统 - 键盘移动

**Files:**
- Modify: `devi/src/game/mod.rs`
- Create: `devi/src/game/input.rs`
- Create: `devi/src/render/camera.rs`
- Modify: `devi/src/main.rs`

- [ ] **Step 1: 创建 game/input.rs - 输入处理系统**

```rust
use bevy::prelude::*;
use crate::game::player::{Position, LocalPlayer};

const MOVE_SPEED: f32 = 5.0;

pub fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Position, &mut Transform), With<LocalPlayer>>,
    map: Res<crate::game::map::GameMap>,
) {
    let Ok((mut pos, mut transform)) = query.get_single_mut() else {
        return;
    };

    let mut dx: f32 = 0.0;
    let mut dy: f32 = 0.0;

    if keyboard.pressed(KeyCode::ArrowUp) || keyboard.pressed(KeyCode::KeyW) {
        dy = 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowDown) || keyboard.pressed(KeyCode::KeyS) {
        dy = -1.0;
    }
    if keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA) {
        dx = -1.0;
    }
    if keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD) {
        dx = 1.0;
    }

    if dx != 0.0 || dy != 0.0 {
        let delta = time.delta_secs() * MOVE_SPEED * 32.0;
        pos.x += dx * delta;
        pos.y += dy * delta;

        // 边界检查
        pos.x = pos.x.clamp(0.0, (map.width - 1) as f32 * map.tile_size);
        pos.y = pos.y.clamp(0.0, (map.height - 1) as f32 * map.tile_size);

        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
    }
}
```

- [ ] **Step 2: 创建 render/camera.rs - 摄像机跟随**

```rust
use bevy::prelude::*;
use crate::game::player::{Position, LocalPlayer};

pub fn follow_camera(
    player_query: Query<&Position, With<LocalPlayer>>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    let Ok(pos) = player_query.get_single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.get_single_mut() else {
        return;
    };

    camera_transform.translation.x = pos.x;
    camera_transform.translation.y = pos.y;
}
```

- [ ] **Step 3: 修改 main.rs 添加系统和摄像机**

```rust
use bevy::prelude::*;
use devi::game::map::GameMap;
use devi::render::tile::{setup_map, setup_local_player};
use devi::render::camera::follow_camera;
use devi::game::input::handle_input;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(GameMap::new(30, 20, 32.0))
        .add_systems(Startup, (setup_map, setup_local_player))
        .add_systems(Update, (handle_input, follow_camera))
        .run();
}
```

- [ ] **Step 4: 验证移动**

Run: `cargo run -p devi`
Expected: 按方向键/WASD 玩家移动，摄像机跟随

- [ ] **Step 5: 提交**

```bash
git add devi/
git commit -m "feat(devi): add keyboard input and camera follow"
```

---

## Task 4: 网络系统 - WebSocket 客户端

**Files:**
- Create: `devi/src/network/mod.rs`
- Create: `devi/src/network/client.rs`
- Create: `devi/src/network/protocol.rs`
- Modify: `devi/src/main.rs`

- [ ] **Step 1: 创建 network/protocol.rs - 协议定义**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    #[serde(rename = "type")]
    pub packet_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnPayload {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovePayload {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapEnterPayload {
    pub character_id: u32,
}

impl Packet {
    pub fn map_enter(character_id: u32) -> Self {
        Self {
            packet_type: "MAP_ENTER".to_string(),
            payload: serde_json::to_value(MapEnterPayload { character_id }).unwrap(),
        }
    }
}
```

- [ ] **Step 2: 创建 network/client.rs - WebSocket 客户端**

```rust
use anyhow::Result;
use bevy::async_compat::Compat;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct NetworkClient {
    sender: mpsc::Sender<String>,
}

impl NetworkClient {
    pub async fn connect(url: &str) -> Result<Self> {
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        let (tx, mut rx) = mpsc::channel::<String>(100);

        // 发送任务
        let send_handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                write.send(Message::Text(msg)).await.ok();
            }
        });

        // 接收任务 (这里只是消费，不处理)
        let _recv_handle = tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                if let Ok(Message::Text(text)) = msg {
                    tracing::debug!("Received: {}", text);
                }
            }
        });

        Ok(Self { sender: tx })
    }

    pub async fn send(&self, packet: &str) -> Result<()> {
        self.sender.send(packet.to_string()).await?;
        Ok(())
    }
}
```

- [ ] **Step 3: 创建 network/mod.rs**

```rust
pub mod client;
pub mod protocol;

pub use client::NetworkClient;
pub use protocol::Packet;
```

- [ ] **Step 4: 修改 main.rs 添加网络初始化**

```rust
use bevy::prelude::*;
use devi::game::map::GameMap;
use devi::render::tile::{setup_map, setup_local_player};
use devi::render::camera::follow_camera;
use devi::game::input::handle_input;
use devi::network::{NetworkClient, Packet};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Resource)]
struct NetworkResource {
    client: Arc<Mutex<Option<NetworkClient>>>,
}

fn main() {
    App::new()
        .insert_resource(GameMap::new(30, 20, 32.0))
        .insert_resource(NetworkResource {
            client: Arc::new(Mutex::new(None)),
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_map, setup_local_player))
        .add_systems(Update, (handle_input, follow_camera))
        .run();
}
```

- [ ] **Step 5: 验证编译**

Run: `cargo build -p devi`
Expected: SUCCESS

- [ ] **Step 6: 提交**

```bash
git add devi/
git commit -m "feat(devi): add WebSocket network client"
```

---

## Task 5: 网络集成 - 发送移动协议

**Files:**
- Modify: `devi/src/game/input.rs`
- Modify: `devi/src/main.rs`

- [ ] **Step 1: 修改 input.rs 发送移动协议**

```rust
use bevy::prelude::*;
use crate::game::player::{Position, LocalPlayer};
use crate::game::map::GameMap;
use crate::network::Packet;

const MOVE_SPEED: f32 = 5.0;

pub fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Position, &mut Transform), With<LocalPlayer>>,
    map: Res<GameMap>,
    network: Res<crate::main::NetworkResource>,
) {
    let Ok((mut pos, mut transform)) = query.get_single_mut() else {
        return;
    };

    let mut dx: f32 = 0.0;
    let mut dy: f32 = 0.0;

    if keyboard.pressed(KeyCode::ArrowUp) || keyboard.pressed(KeyCode::KeyW) {
        dy = 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowDown) || keyboard.pressed(KeyCode::KeyS) {
        dy = -1.0;
    }
    if keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA) {
        dx = -1.0;
    }
    if keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD) {
        dx = 1.0;
    }

    if dx != 0.0 || dy != 0.0 {
        let delta = time.delta_secs() * MOVE_SPEED * 32.0;
        let old_x = pos.x;
        let old_y = pos.y;

        pos.x += dx * delta;
        pos.y += dy * delta;

        pos.x = pos.x.clamp(0.0, (map.width - 1) as f32 * map.tile_size);
        pos.y = pos.y.clamp(0.0, (map.height - 1) as f32 * map.tile_size);

        transform.translation.x = pos.x;
        transform.translation.y = pos.y;

        // 发送移动协议
        if (old_x != pos.x || old_y != pos.y) {
            let packet = Packet {
                packet_type: "MOVE".to_string(),
                payload: serde_json::json!({
                    "x": pos.x,
                    "y": pos.y
                }),
            };
            if let Ok(json) = serde_json::to_string(&packet) {
                let client = network.client.clone();
                tokio::spawn(async move {
                    if let Some(c) = client.lock().await.as_ref() {
                        c.send(&json).await.ok();
                    }
                });
            }
        }
    }
}
```

- [ ] **Step 2: 验证编译**

Run: `cargo build -p devi`
Expected: SUCCESS

- [ ] **Step 3: 提交**

```bash
git add devi/
git commit -m "feat(devi): send MOVE protocol on player movement"
```

---

## Task 6: 集成测试

**Files:**
- Modify: `devi/src/main.rs`

- [ ] **Step 1: 添加服务端连接初始化**

```rust
use bevy::prelude::*;
use devi::game::map::GameMap;
use devi::render::tile::{setup_map, setup_local_player};
use devi::render::camera::follow_camera;
use devi::game::input::handle_input;
use devi::network::{NetworkClient, Packet};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Resource)]
struct NetworkResource {
    client: Arc<Mutex<Option<NetworkClient>>>,
}

async fn init_network(network: Arc<Mutex<Option<NetworkClient>>>) {
    let url = "ws://127.0.0.1:16121";
    match NetworkClient::connect(url).await {
        Ok(client) => {
            *network.lock().await = Some(client);
            tracing::info!("Connected to server");
        }
        Err(e) => {
            tracing::warn!("Failed to connect to server: {}", e);
        }
    }
}

fn main() {
    let network = Arc::new(Mutex::new(None));

    // 异步初始化网络
    std::thread::spawn({
        let network_clone = network.clone();
        move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                init_network(network_clone).await;
            });
        }
    });

    App::new()
        .insert_resource(GameMap::new(30, 20, 32.0))
        .insert_resource(NetworkResource { client: network })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_map, setup_local_player))
        .add_systems(Update, (handle_input, follow_camera))
        .run();
}
```

- [ ] **Step 2: 最终验证**

Run: `cargo build -p devi --release`
Expected: SUCCESS

- [ ] **Step 3: 提交**

```bash
git add devi/
git commit -m "feat(devi): complete MVP with network integration"
```

---

## 自检清单

- [ ] Spec 覆盖检查：连接服务器 ✅，显示角色 ✅，方向键移动 ✅
- [ ] 占位符检查：无 TBD/TODO
- [ ] 类型一致性：所有 Task 间类型一致
- [ ] 成功标准：6 个判定条件都有对应实现

---

## 执行方式

Plan 完成并保存到 `docs/superpowers/plans/2026-05-04-devi-client-mvp-plan.md`。

**两种执行方式：**

**1. Subagent-Driven (推荐)** - 每个 Task 用独立 subagent 执行，任务间 review

**2. Inline Execution** - 当前 session 内批处理执行

选择哪种方式？
