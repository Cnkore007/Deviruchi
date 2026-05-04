# Deviruchi

高性能 Ragnarok Online 服务端，用 Rust 语言，基于rAthena重构而来，简单、高效、稳定、易扩展。

## 目标

Deviruchi 是 rAthena 服务端的 Rust 实现，目标是：

- **零配置运行** — 双击运行，无需导入数据库等
- **语言级安全** — Rust快速重构
- **微秒级延迟** — 内存channel直连通信，公会战多人boss战依然流畅
- **无缝升级** — 自动数据库迁移，SQLite 到 MySQL 一行配置
- **配套客户端** — 不仅支持原生客户端，还开发独立的 Devi 客户端（开发中，基于Bevy重构Ro现代化客户端）

## 核心特性

| 特性 | 说明 |
|------|------|
| 简单部署| 单可执行文件包含 Login/Char/Map 全部服务 |
| 开箱即用 | 无需安装Mysql数据库或配置连接参数 |
| 统一Log | 统一的日志及崩溃报告，方便调试。 |
| 极低延迟 | 模块间内存 channel 通信，延迟微秒级 |
| 无忧升级 | 自动数据库迁移，支持 SQLite → MySQL |
| 双协议支持 | Legacy TCP（二进制）兼容 rAthena 客户端，Modern WebSocket 服务 Devi 客户端 |

## 架构

```
Deviruchi
├── Login Server  : 6900 (Legacy) / 16900 (Modern)
├── Char Server   : 6000 (Legacy) / 16000 (Modern)
└── Map Server    : 6121 (Legacy) / 16121 (Modern)

Devi (客户端)
├── 渲染引擎: Bevy + wgpu
├── 像素增强: Integer Scaling + xBRZ
├── 同步策略: 混合同步（自己角色预测，他人角色插值）
└── 同步频率: 20ms tick

Python 工具
├── 资源转换: GRF → PNG 图集 + JSON 配置
└── 地图导出: .gat → JSON + PNG
```

## 当前进度

### 服务端 (Deviruchi)

| 模块 | 状态 |
|------|------|
| TCP 网络层 + Session 生命周期 | ✅ |
| Login/Char/Map 三服务器 | ✅ |
| ~80 种包定义，36 个 MapServer 处理器 | ✅ |
| SQLite 持久化（8 张表） | ✅ |
| Mob AI（6 状态 FSM）+ A* 寻路 | ✅ |
| 战斗公式 + 经验分配 | ✅ |
| 组队/公会/交易/仓库/传送 | ✅ |
| 掉落物管理 + 死亡/重生 | ✅ |
| 死亡/重生 + 经验惩罚 | ✅ |
| HP/SP 回复 | 🔧 开发中 |
| NPC 对话引擎 | 📋 规划中 |
| WebSocket/Modern Protocol | 📋 规划中 |
| 地图 .gat 文件加载 | 📋 规划中 |
| 碰撞检测 | 📋 规划中 |
| 自动数据库迁移 | 📋 规划中 |

### 客户端 (Devi)

| 模块 | 状态 |
|------|------|
| 项目结构搭建 | 📋 规划中 |
| 瓦片地图渲染 | 📋 规划中 |
| 精灵动画系统 | 📋 规划中 |
| 角色移动 + 攻击 | 📋 规划中 |
| 中文聊天 (IME) | 📋 规划中 |
| UI/HUD 系统 | 📋 规划中 |
| 像素增强着色器 | 📋 规划中 |
| 资源热更新 | 📋 规划中 |
| 启动器 | 📋 规划中 |

## 技术栈

| 组件 | 技术 |
|------|------|
| 服务端 | Rust, Tokio, SQLite |
| 客户端 | Rust, Bevy, wgpu |
| 协议 | rAthena 二进制包（兼容）+ WebSocket |
| 构建 | Cargo Workspace |
| 协议参考 | rAthena (C++) |

## 快速开始

```bash
# 克隆项目
git clone https://github.com/Cnkore007/Deviruchi.git
cd Deviruchi

# 编译
cargo build --release

# 运行（单机模式）
cargo run --release
```

## 文档

- [CONTEXT.md](CONTEXT.md) — 项目术语表
- [docs/adr/](docs/adr/) — 架构决策记录
- [docs/agents/](docs/agents/) — Agent 工作流配置

## License

GPL-3.0
