# Devi Client MVP 设计文档

**日期**: 2026-05-04
**状态**: 已批准
**目标**: 创建极简可运行的 Devi 游戏客户端 MVP

## 概述

Devi 是 Deviruchi MMORPG 的客户端，使用 Bevy 引擎和 Rust 语言实现。MVP 阶段专注于最核心功能：连接服务器、显示玩家角色、实现方向键移动。

## 项目结构

```
Deviruchi/
├── Cargo.toml          # workspace 配置
├── devi/               # 客户端 crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs     # 入口，Bevy App 启动
│       ├── lib.rs      # 模块导出
│       ├── network/    # WebSocket 网络模块
│       │   ├── mod.rs
│       │   ├── client.rs
│       │   └── protocol.rs
│       ├── game/       # 游戏逻辑模块
│       │   ├── mod.rs
│       │   ├── player.rs
│       │   ├── map.rs
│       │   └── input.rs
│       └── render/     # 渲染模块
│           ├── mod.rs
│           ├── tile.rs
│           └── camera.rs
└── src/                # 服务端 crate (现有)
```

## 技术栈

| 组件 | 技术 | 说明 |
|------|------|------|
| 游戏引擎 | Bevy 0.14+ | ECS 架构 |
| 渲染后端 | wgpu (Bevy 内置) | 跨平台 GPU 渲染 |
| 网络 | tokio + tokio-tungstenite | WebSocket 异步客户端 |
| 协议 | JSON (Modern Protocol) | 与服务端 Modern 协议对接 |
| 构建 | Cargo Workspace | monorepo 结构 |

## 架构设计

### 主循环

Bevy App 作为主游戏循环，Tokio Runtime 处理网络：

```
Bevy App (主循环)
    │
    ├── InputSystem      → 读取键盘输入
    ├── NetworkSystem    → 通过 Channel 发送/接收网络消息
    ├── GameLogicSystem  → 处理游戏逻辑
    └── RenderSystem     → 渲染瓦片和角色
                                │
                                ▼
                        Tokio Runtime (后台)
                                │
                                ├── WebSocket Client
                                └── Packet Handler
```

### 组件定义

```rust
// 玩家组件
#[derive(Component)]
struct Player {
    id: u32,
    name: String,
    position: Vec2,
    direction: Direction,
    is_local: bool,
}

// 地图组件
#[derive(Component)]
struct MapTile {
    x: u32,
    y: u32,
    walkable: bool,
}

// 方向枚举
#[derive(Component, Clone)]
enum Direction {
    Up, Down, Left, Right,
}
```

### 地图生成

MVP 使用程序生成的测试地图，无需外部资源文件：

```rust
fn generate_test_map(width: u32, height: u32) -> Vec<MapTile> {
    // 生成 NxM 的矩形格子地图
    // 默认所有格子可通行
}
```

### 角色渲染

- 自己玩家：绿色方块
- 其他玩家：蓝色方块
- 每个方块固定尺寸（如 32x32 像素）

### 摄像机

固定跟随本地玩家：

```rust
fn follow_camera(player: &Player, camera: &mut Camera) {
    camera.translation = Vec3::new(player.position.x, player.position.y, 0.0);
}
```

## 网络协议

### MVP 实现包

| 包类型 | 方向 | 说明 |
|--------|------|------|
| `CHAT` | 客户端→服务端 | 聊天消息 |
| `MAP_ENTER` | 客户端→服务端 | 请求进入地图 |
| `MAP_LOADED` | 客户端→服务端 | 地图加载完成确认 |
| `ACTOR_SPAWN` | 服务端→客户端 | 角色出现在视野 |
| `ACTOR_MOVE` | 服务端→客户端 | 角色移动更新 |
| `ACTOR_DESPAWN` | 服务端→客户端 | 角色离开视野 |

### JSON 协议示例

```json
// MAP_ENTER
{
    "type": "MAP_ENTER",
    "payload": {
        "character_id": 1
    }
}

// ACTOR_SPAWN
{
    "type": "ACTOR_SPAWN",
    "payload": {
        "id": 123,
        "name": "Player1",
        "x": 10,
        "y": 20,
        "direction": "down"
    }
}

// ACTOR_MOVE
{
    "type": "ACTOR_MOVE",
    "payload": {
        "id": 123,
        "x": 15,
        "y": 20
    }
}
```

## 实现阶段

### Phase 1: 项目搭建
- [ ] 创建 `devi/` 目录和 Cargo.toml
- [ ] 配置 Bevy 依赖
- [ ] 添加到 workspace
- [ ] 验证 Bevy App 能启动

### Phase 2: 基础渲染
- [ ] 程序生成测试地图
- [ ] 瓦片渲染系统
- [ ] 摄像机跟随
- [ ] 本地玩家显示

### Phase 3: 输入系统
- [ ] 键盘输入捕获
- [ ] WASD / 方向键支持
- [ ] 玩家移动逻辑
- [ ] 发送移动协议到服务端

### Phase 4: 网络系统
- [ ] Tokio WebSocket 客户端
- [ ] 服务端连接
- [ ] 协议编解码
- [ ] 接收/处理其他玩家更新

### Phase 5: 集成测试
- [ ] 客户端-服务端连接测试
- [ ] 多玩家移动同步
- [ ] 修复问题

## 配置

服务端地址默认配置：

```toml
[network]
server_host = "127.0.0.1"
server_port = 16121
```

## 成功标准

MVP 完成的判定条件：

1. 客户端能启动并显示窗口
2. 能看到程序生成的格子地图
3. 能看到代表自己的绿色方块
4. 按方向键能移动（发送到服务端）
5. 能接收并显示其他玩家
6. 无运行时 panic

## 待后续实现（非 MVP）

- GRF 资源文件加载
- 精灵动画
- 像素增强着色器
- 聊天系统 UI
- 碰撞检测
