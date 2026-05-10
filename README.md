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

> **74,000+ 行** Rust 代码，260+ 个源文件，1,146 单元测试（0 失败），覆盖 48 个游戏子系统 + Devi 客户端，整体完成度 **~99%**。

### 服务端 — 已完成（36 个模块）

#### 核心战斗

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **battle** | 2,500+ 行 | 物理/魔法/PvP 伤害公式、MATK 计算、元素修正表（10属性x4级别）、命中/闪避/暴击率、体型修正、经验分配（单人+队伍），69 测试 |
| **mob** | 3,500+ 行 | 七状态 AI 状态机（含 Flee）、A* 寻路、掉落表、MVP 判定、rAthena YAML 完整解析（Race/Class/Modes/Skills/MvpDrops）、FleeWhenLowHp/Assist/PassiveAssist/RudeAttacked/LongRange 条件，61 测试 |
| **status** | 2,872 行 | 60+ 种状态效果、属性计算器、DOT/回复 tick 处理、图标数据库，30+ 测试 |
| **skill** | 1,300+ 行 | 技能处理器（冷却/消耗/范围检查）、技能效果系统、技能效果执行器（伤害/治疗/buff/范围）、技能树系统（6 职业），42 测试 |
| **heal** | 860 行 | HP/SP 自然回复、食物效果系统 |

#### 地图与玩家

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **map** | 5,387 行 | Player 结构（6 个分组锁）、MapServer（GM/NPC/社交/公会 handler）、频道消息总线、传送/重生/掉落物管理 |
| **item** | 4,775 行 | 物品使用效果、装备系统、背包管理、脚本执行、延迟系统，12 个文件，最庞大的子系统 |
| **trade** | 639 行 | 交易状态机（Requesting→Trading→Completed）、物品/金币转移、重量验证 |

#### 社交与公会

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **party** | 388 行 | 组队创建/加入/离开/解散、经验分配模式 |
| **guild** | 597 行 | 公会创建/邀请/踢出、权限系统、职位管理 |
| **chat** | 1,383 行 | 统一聊天管理、私聊（在线/离线）、频率限制 |
| **command** | 422 行 | @命令框架、GM 权限检查、传送/玩家命令 |

#### 进阶系统

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **pet** | 1,170 行 | 宠物孵化/召唤/喂食/回收、宠物 AI（Follow/Attack/Pickup） |
| **mount** | 667 行 | 坐骑骑乘/下马、等级检查 |
| **quest** | 1,029 行 | 任务接受/更新/完成/放弃、目标追踪 |
| **achievement** | 829 行 | 成就解锁/进度追踪、分类系统 |
| **instance** | 1,615 行 | 副本创建/加入/离开/清理、模板数据库 |
| **card** | 566 行 | 卡片插入/移除、属性计算 |
| **vending** | 1,081 行 | 摆摊开店/关店/购买、商品搜索 |
| **cashshop** | 1,827 行 | 商城购买/点数管理/礼物赠送、Kafra 服务 |
| **battleground** | 1,283 行 | 战场队列/匹配/状态管理 |
| **woe** | 1,052 行 | 攻城战城堡/时间安排/攻击冷却 |
| **auction** | 554 行 | 拍卖上架/竞价/领取 |
| **mail** | 452 行 | 邮件发送/收取/删除/附件 |

#### 基础设施

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **network** | 815 行 | Legacy TCP + Modern WebSocket 双协议服务器、Session 管理 |
| **protocol** | 2,155 行 | ~80 种包定义（map/guild/teleport/trade/party/char/storage/login） |
| **storage** | 3,500+ 行 | Backend 抽象层（SQLite/MySQL 双后端）、14 张表、仓库同步管理器、UPSERT 持久化、迁移框架（v5）、125+ 测试 |
| **game_loop** | 331 行 | 100ms tick 主循环：掉落物清理→Token 清理→怪物重生→AI 更新→玩家回复 |
| **token** | 319 行 | Map Server 连接验证 Token，30 秒过期 TTL |
| **zeny** | 150 行 | 金币管理，溢出保护 |
| **rand** | 176 行 | 可注入 RNG 接口，生产/测试双实现 |

#### 登录与角色

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **login** | 339 行 | 登录认证（Argon2）、封禁过期检查、重复登录检测、8 测试 |
| **char** | 916 行 | 角色列表/创建/选择/删除/取消删除、名称验证（CJK+特殊字符）、Token 生成，11 测试 |

#### NPC 与脚本

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **npc** | 1,200+ 行 | NPC 数据库（YAML 加载+硬编码回退）、5 种类型+Event 触发（OnTouch/OnInit）、商店/技能训练师/Warp/Quest/CashShop、20 测试 |
| **script** | 2,300+ 行 | 脚本解析器（30 个命令：mes/next/close/select/warp/goto/set/goto_if/getitem/delitem/announce/for/callfunc/return 等）、表达式解析、函数调用栈、循环控制、98 测试 |

#### 仓库系统

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **storage** | 3,500+ 行 | 仓库 CRUD、UPSERT 持久化、后台同步调度器（实际执行+超时恢复）、迁移框架（v5）、125+ 测试 |

#### 进阶系统（新增）

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **homunculus** | 1,400+ 行 | 生命体系统：数据结构（六属性/技能/进化）、经验/升级/进化/喂食、实时数据库持久化、13 测试 |
| **mercenary** | 1,000+ 行 | 佣兵系统：数据结构（六属性/技能/忠诚度）、合同计时到期自动解散+数据库删除、5 种模板、13 测试 |
| **skill/executor** | 300+ 行 | 技能效果执行器：伤害/治疗/buff/范围技能完整执行，17 测试 |
| **skill/skill_tree** | 250+ 行 | 技能树系统：6 个一转职业技能树定义、前置技能检查、YAML 加载，15 测试 |
| **job** | 400+ 行 | 职业转职系统：20 种 RO 职业枚举、转职条件校验、@jobchange GM 命令，32 测试 |
| **battle/formula** | +600 行 | 魔法伤害公式（MATK+元素修正）、PvP 物理/魔法伤害公式，34 测试 |

#### 地图基础设施（新增）

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **gat** | 386 行 | .gat 文件解析器（rAthena 格式 v1-5）、MapState 真实碰撞检测、6 测试 |
| **migration** | 420 行 | 数据库迁移框架（schema_version 版本追踪、事务保护、幂等执行） |

### 客户端 (Devi) — 6,500+ 行，42 个源文件，80+ 测试

#### 核心框架

| 模块 | 状态 | 说明 |
|------|------|------|
| 项目结构 | ✅ 已完成 | Bevy ECS + Cargo Workspace，模块化架构 |
| 配置系统 | ✅ 已完成 | ClientConfig，服务器地址/窗口/协议配置 |
| 游戏状态机 | ✅ 已完成 | Login → CharSelect → InGame 状态流转 |
| Tick 系统 | ✅ 已完成 | TickConfig 游戏循环定时 |

#### 网络与协议

| 模块 | 状态 | 说明 |
|------|------|------|
| 网络层 | ✅ 已完成 | Legacy TCP + WebSocket 双传输层，异步 I/O + Bevy 同步桥接 |
| 协议编解码 | ✅ 已完成 | 17 种包类型（Login/Char/Map），rAthena 二进制格式 |
| 会话管理 | ✅ 已完成 | NetworkManager，tokio mpsc + 独立线程，支持多实例（登录+地图） |

#### 游戏流程

| 模块 | 状态 | 说明 |
|------|------|------|
| 登录流程 | ✅ 已完成 | 用户名/密码 UI，连接登录服务器，凭据传递 |
| 角色选择 | ✅ 已完成 | 角色卡片列表，进入游戏按钮 |
| 角色创建 | ✅ 已完成 | 名称输入 + 六属性分配（30 点） + 确认/返回 |
| 地图连接 | ✅ 已完成 | MapEnterRequest 发送，初始位置接收，实体事件处理 |

#### 渲染引擎

| 模块 | 状态 | 说明 |
|------|------|------|
| 等距相机 | ✅ 已完成 | RO 风格 45° 等距视角，平移/缩放控制 |
| 地形网格 | ✅ 已完成 | .gat 数据 → 3D 网格构建器 |
| 精灵动画 | ✅ 已完成 | SPR/ACT 解析，帧动画播放 |
| Billboard | ✅ 已完成 | 精灵始终面向相机 |
| HUD 界面 | ✅ 已完成 | 原版 RO 布局（HP/SP/状态栏） |
| 聊天窗口 | ✅ 已完成 | 聊天消息显示 UI |

#### 实体系统

| 模块 | 状态 | 说明 |
|------|------|------|
| 玩家组件 | ✅ 已完成 | Player/Mob ECS 组件定义 |
| 移动系统 | ✅ 已完成 | 玩家移动逻辑 |
| 实体同步 | 🔧 进行中 | EntityAppear/Disappear/Move → 场景实体（TODO） |

#### 实体与战斗

| 模块 | 状态 | 说明 |
|------|------|------|
| 实体同步 | ✅ 已完成 | EntityAppear/Disappear/Move → Bevy ECS 实体管理，entity_map 映射表，坐标/速度转换 |
| 角色攻击 | ✅ 已完成 | AttackRequest/AttackNotify 协议，攻击动画状态机（Prepare→Strike→Recovery），浮动伤害数字 |
| 瓦片地图渲染 | ✅ 已完成 | GND 文件解析器，GRF 纹理加载，按纹理分组的 Bevy Mesh 渲染 |

#### 聊天与 UI

| 模块 | 状态 | 说明 |
|------|------|------|
| 中文聊天 IME | ✅ 已完成 | Bevy Ime 事件处理（Preedit/Commit），5 种消息类型，100 条历史缓冲 |

#### 待开发

| 模块 | 状态 |
|------|------|
| 像素增强着色器 | 📋 规划中 |
| 资源热更新 | 📋 规划中 |
| 启动器 | 📋 规划中 |

## 技术栈

| 组件 | 技术 |
|------|------|
| 服务端 | Rust, Tokio, SQLite, MySQL（可选） |
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
