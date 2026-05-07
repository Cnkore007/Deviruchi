# Devi 客户端设计文档

> 现代化重制 RO 客户端，保持原版 UI 布局，基于 Bevy + 自定义渲染管线。

## 决策记录

| 维度 | 决定 | 原因 |
|------|------|------|
| 定位 | 现代化重制，保持原版 UI 布局 | 还原经典体验，技术现代化 |
| 渲染 | 2D 精灵 + 3D 低多边形地形混合渲染 | RO 核心视觉特征 |
| 协议 | 双协议兼容 Deviruchi (WebSocket) + rAthena (Legacy TCP) | 覆盖面最广 |
| 平台 | 桌面优先 (Win/Mac/Linux) | RO 本身是桌面游戏 |
| 资源 | 同时支持 GRF 原版格式 + 自定义 PNG+JSON 格式 | 灵活度最高 |
| 引擎 | Bevy + 自定义渲染管线 | Rust 全栈，ECS 架构，wgpu 跨平台 |
| 参考 | roBrowser (https://github.com/skybook888/roBrowser) | 已解决格式解析和协议问题 |

## 项目结构

```
devi/
├── Cargo.toml
├── src/
│   ├── main.rs              # 入口，初始化 Bevy App
│   ├── lib.rs               # 模块导出
│   │
│   ├── core/                # 核心基础设施
│   │   ├── config.rs        # 客户端配置（分辨率、服务器地址等）
│   │   ├── state.rs         # 游戏状态机（登录/选角色/游戏中）
│   │   └── tick.rs          # 固定 tick 循环（20ms）
│   │
│   ├── net/                 # 网络层
│   │   ├── mod.rs
│   │   ├── legacy.rs        # rAthena Legacy TCP 协议
│   │   ├── modern.rs        # Deviruchi WebSocket 协议
│   │   ├── codec.rs         # 包编解码（共享）
│   │   └── handler.rs       # 包分发处理
│   │
│   ├── asset/               # 资源系统
│   │   ├── mod.rs
│   │   ├── grf.rs           # GRF 文件解析器
│   │   ├── sprite.rs        # 精灵图集加载（ACT + SPR）
│   │   ├── model.rs         # 3D 模型加载（RSM）
│   │   ├── map.rs           # 地图加载（GAT + GND + RSW）
│   │   ├── texture.rs       # 纹理管理
│   │   └── loader.rs        # 统一资源加载器（GRF/自定义格式自动切换）
│   │
│   ├── render/              # 自定义渲染管线
│   │   ├── mod.rs
│   │   ├── pipeline.rs      # 混合渲染管线注册
│   │   ├── terrain.rs       # 3D 地形渲染（GAT/GND → GPU mesh）
│   │   ├── sprite.rs        # 2D 精灵 billboard 渲染
│   │   ├── effect.rs        # 特效（光照、技能动画）
│   │   ├── camera.rs        # RO 风格 45° 等距相机
│   │   └── ui/              # UI 渲染（保持原版布局）
│   │       ├── mod.rs
│   │       ├── hud.rs       # 血条、SP条、快捷栏
│   │       ├── chat.rs      # 聊天窗口
│   │       ├── inventory.rs # 背包
│   │       └── npc_dialog.rs# NPC 对话框
│   │
│   ├── game/                # 游戏逻辑（ECS）
│   │   ├── mod.rs
│   │   ├── player.rs        # 玩家组件和系统
│   │   ├── mob.rs           # 怪物组件和系统
│   │   ├── npc.rs           # NPC 组件和系统
│   │   ├── movement.rs      # 移动和寻路
│   │   ├── skill.rs         # 技能系统
│   │   └── chat.rs          # 聊天系统
│   │
│   └── protocol/            # 协议定义（可与服务端共享）
│       ├── mod.rs
│       ├── login.rs         # 登录包
│       ├── char.rs          # 选角色包
│       └── map.rs           # 地图包
│
├── assets/                  # Bevy 资源目录
│   └── shaders/             # 自定义 shader
└── tools/                   # 辅助工具
    └── grf_extract.py       # GRF → PNG+JSON 转换
```

## 混合渲染管线

### 渲染层次

```
┌─────────────────────────────────────┐
│  Layer 4: UI（原版布局）              │  ← 2D，屏幕空间
├─────────────────────────────────────┤
│  Layer 3: 特效（技能、光照、粒子）     │  ← 混合
├─────────────────────────────────────┤
│  Layer 2: 2D 精灵（角色、怪物、NPC）  │  ← Billboard，世界空间
├─────────────────────────────────────┤
│  Layer 1: 3D 地形（GAT/GND mesh）    │  ← 3D，世界空间
└─────────────────────────────────────┘
```

### 3D 地形渲染

- **数据来源：** GAT 文件定义地面高度网格，GND 文件定义地面纹理和法线
- **Mesh 生成：** 将 GAT 的高度数据转为三角形网格，每个 cell 2 个三角形
- **纹理：** GND 的 texture atlas，UV 映射到 mesh 上
- **相机：** RO 经典的 45° 等距视角，固定俯仰角，玩家可旋转

### 2D 精灵渲染

- **Billboard 技术：** 精灵始终面向相机，在 3D 世界空间中用 quad + 纹理实现
- **动画：** SPR + ACT 文件定义帧序列和变换（缩放、旋转、位移），按时间插值播放
- **排序：** 按 Y 轴深度排序（painter's algorithm），确保遮挡关系正确
- **光照：** 精灵接收地形光照，支持简单的明暗变化

### 特效系统

- **技能特效：** 基于 ACT 动画的 2D 粒子，叠加在精灵上层
- **地面光照：** 地形 mesh 上叠加光照贴图
- **水反射：** 简单的水面反射（先用平面反射，后期优化）

### 相机控制

- 俯仰角: ~30°
- FOV: ~45°
- 可旋转: 360°
- 可缩放: 近/中/远

## 网络层（双协议）

### 架构

```
游戏逻辑（ECS Systems）
       │
       ▼
┌──────────────────────┐
│   NetworkTransport   │  ← 统一 trait
│   - send(packet)     │
│   - recv() → Packet  │
│   - connected()      │
└──────┬───────┬───────┘
       │       │
  ┌────▼──┐ ┌──▼─────┐
  │Legacy │ │Modern  │
  │ TCP   │ │WebSocket│
  └───────┘ └────────┘
```

### 协议差异

| 维度 | Legacy (rAthena) | Modern (Deviruchi) |
|------|-------------------|---------------------|
| 传输 | TCP 二进制 | WebSocket 二进制 |
| 包头 | 2 字节 packet_id + 长度 | 统一帧格式 |
| 加密 | 可选 XOR | TLS |
| 序列化 | 手动字节序打包 | 与 Legacy 共享编解码 |

`codec.rs` 负责包的编解码，两种协议共享同一个 `Packet` 枚举定义。`handler.rs` 根据 packet_id 分发到对应的 ECS 系统处理。

### 连接流程

1. **配置选择协议** — 用户在配置文件或启动界面选择连接 Deviruchi 还是 rAthena
2. **登录阶段** — 发送登录包，接收服务器列表
3. **选角色阶段** — 查询/创建/删除角色
4. **进入地图** — 切换到地图协议，开始游戏循环

## 资源系统

### 双格式加载策略

请求资源时，先检查自定义格式目录（PNG+JSON），找不到则 fallback 到 GRF 文件解析，两者都找不到则报错。

### 格式解析（参考 roBrowser）

| 格式 | 用途 | 参考路径 |
|------|------|----------|
| `.grf` | 资源打包容器 | `src/VFS/` |
| `.spr` | 精灵图片数据 | `src/Loaders/Sprite.js` |
| `.act` | 精灵动画定义 | `src/Loaders/Action.js` |
| `.gat` | 地面高度网格 | `src/Loaders/GAT.js` |
| `.gnd` | 地面纹理/法线 | `src/Loaders/GND.js` |
| `.rsw` | 场景定义（灯光、模型位置） | `src/Loaders/RSW.js` |
| `.rsm` | 3D 模型 | `src/Loaders/RSM.js` |

GRF 是 zlib 压缩的文件容器，有文件索引表。解析流程：读取 header → 读取文件表 → 按路径查找 → 解压文件条目。

## UI 布局（保持原版）

```
┌─────────────────────────────────────────────┐
│                                             │
│              游戏视口（3D+2D）                │
│                                             │
├────────────────────────────┬────────────────┤
│                            │                │
│       聊天窗口             │   迷你地图      │
│                            │                │
├────────────────────────────┼────────────────┤
│  快捷技能栏（F1-F9）       │   状态栏        │
│  血条/SP条                 │   HP/SP/基础信息 │
└────────────────────────────┴────────────────┘
```

- **固定像素布局** — UI 元素用像素坐标定位，不随分辨率缩放
- **可拖拽窗口** — 背包、装备、技能树等窗口可拖拽
- **字体** — 使用 RO 原版字体或等效像素字体

## 参考资源

- **roBrowser:** https://github.com/skybook888/roBrowser — RO Web 客户端实现，格式解析和协议的最佳参考
- **Deviruchi 服务端:** 当前项目，Rust 实现的 RO 服务端，双协议支持
