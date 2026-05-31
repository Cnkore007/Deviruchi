# Deviruchi

高性能 Ragnarok Online 服务端，用 Rust 语言，基于 rAthena 重构而来，简单、高效、稳定、易扩展。

> **v0.0.1** — 首个公开版本

## 目标

Deviruchi 是 rAthena 服务端的 Rust 实现，目标是：

- **零配置运行** — 双击运行，无需导入数据库等
- **语言级安全** — Rust 快速重构
- **微秒级延迟** — 内存 channel 直连通信，公会战多人 boss 战依然流畅
- **无缝升级** — 自动数据库迁移，SQLite 到 MySQL 一行配置
- **配套客户端** — 不仅支持原生客户端，还开发独立的 Devi 客户端（开发中，基于 Bevy 重构 RO 现代化客户端）
- **智能助手** — DeviAgent 服务端运维助手，REPL 交互 + 知识库检索

## 核心特性

| 特性 | 说明 |
|------|------|
| 简单部署 | 单可执行文件包含 Login/Char/Map 全部服务 |
| 开箱即用 | 无需安装 MySQL 数据库或配置连接参数 |
| 统一日志 | 统一的日志及崩溃报告，方便调试 |
| 极低延迟 | 模块间内存 channel 通信，延迟微秒级 |
| 无忧升级 | 自动数据库迁移，支持 SQLite → MySQL |
| 双协议支持 | Legacy TCP（二进制）兼容 rAthena 客户端，Modern WebSocket 服务 Devi 客户端 |

## 架构

```
Deviruchi (服务端)
├── Login Server  : 6900 (Legacy) / 16900 (Modern)
├── Char Server   : 6000 (Legacy) / 16000 (Modern)
└── Map Server    : 6121 (Legacy) / 16121 (Modern)

Devi (客户端)
├── 渲染引擎: Bevy + wgpu
├── 像素增强: Integer Scaling + xBRZ
├── 同步策略: 混合同步（自己角色预测，他人角色插值）
└── 同步频率: 20ms tick

DeviAgent (智能助手)
├── REPL 交互: 命令行实时查询服务端状态
├── 知识库: YAML 文档索引 + 全文检索
└── 通信: TCP 直连服务端
```

## 当前进度

> **89,500+ 行 Rust 代码，288 个源文件，1,286 测试（全部通过），覆盖 54 个服务端模块 + Devi 客户端 + DeviAgent。

### 服务端 — 已完成（52 个模块）

#### 核心战斗

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **battle** | 2,500+ 行 | 物理/魔法/PvP 伤害公式、MATK 计算、元素修正表（10 属性 x4 级别）、命中/闪避/暴击率、体型修正、经验分配（单人+队伍），69 测试 |
| **mob** | 3,500+ 行 | 七状态 AI 状态机（含 Flee）、A* 寻路、掉落表、MVP 判定、rAthena YAML 完整解析，61 测试 |
| **status** | 2,872 行 | 60+ 种状态效果、属性计算器、DOT/回复 tick 处理、图标数据库，30+ 测试 |
| **skill** | 1,300+ 行 | 技能处理器（冷却/消耗/范围检查）、技能效果系统、技能效果执行器、技能树系统（6 职业），42 测试 |
| **heal** | 860 行 | HP/SP 自然回复、食物效果系统 |

#### 地图与玩家

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **map** | 5,387 行 | Player 结构（6 个分组锁）、MapServer（GM/NPC/社交/公会 handler）、频道消息总线、传送/重生/掉落物管理 |
| **item** | 4,775 行 | 物品使用效果、装备系统、背包管理、脚本执行、延迟系统，12 个文件 |
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
| **homunculus** | 1,400+ 行 | 生命体系统：六属性/技能/进化、实时数据库持久化、13 测试 |
| **mercenary** | 1,000+ 行 | 佣兵系统：六属性/技能/忠诚度、合同计时到期自动解散、13 测试 |
#### 新增模块（本次迭代）

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **date** | 300+ 行 | 游戏时间系统，chrono 驱动，DayOfWeek 转换，16 测试 |
| **clan** | 350+ 行 | 公会联盟系统，AllianceType 枚举，成员管理，9 测试 |
| **duel** | 280+ 行 | 决斗系统，冷却机制，参与者管理，12 测试 |
| **mapreg** | 250+ 行 | 全局变量存储，`$`前缀=永久，`@`前缀=临时，11 测试 |
| **npc_chat** | 200+ 行 | NPC 聊天匹配，regex 驱动，12 测试 |
| **buyingstore** | 180+ 行 | 收购商店，8 测试 |
| **searchstore** | 150+ 行 | 全服商店搜索，5 测试 |
| **navi** | 220+ 行 | A* 地图导航，9 测试 |
| **elemental** | 300+ 行 | 精灵召唤，AI 状态机（Follow/Attack/Wander），RwLock 死锁修复，6 测试 |
| **pc_groups** | 250+ 行 | GM 权限组（Player→Super 6级），10 测试 |
| **log** | 200+ 行 | 游戏日志，按类型过滤（Chat/Trade/Battle/GM），8 测试 |
| **path** | 280+ 行 | A* 寻路算法，8方向+对角线代价，10 测试 |
| **unit** | 300+ 行 | 单位移动系统，碰撞验证，12 测试 |
| **intif** | 250+ 行 | 服务器间通信，消息队列，11 测试 |
| **zeny** | 120+ 行 | 金币管理，溢出保护 |
| **agent_api** | 180+ 行 | DeviAgent 服务端 API 接口 |


#### 基础设施

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **network** | 815 行 | Legacy TCP + Modern WebSocket 双协议服务器、Session 管理 |
| **protocol** | 2,155 行 | ~80 种包定义（map/guild/teleport/trade/party/char/storage/login） |
| **storage** | 3,500+ 行 | Backend 抽象层（SQLite/MySQL 双后端）、14 张表、仓库同步管理器、UPSERT 持久化、迁移框架（v5）、125+ 测试 |
| **game_loop** | 331 行 | 100ms tick 主循环：掉落物清理→Token 清理→怪物重生→AI 更新→玩家回复 |
| **login** | 339 行 | 登录认证（Argon2）、封禁过期检查、重复登录检测 |
| **char** | 916 行 | 角色列表/创建/选择/删除、名称验证（CJK+特殊字符）、Token 生成 |
| **npc** | 1,200+ 行 | NPC 数据库（YAML 加载+硬编码回退）、5 种类型+Event 触发、商店/技能训练师/Warp/Quest/CashShop |
| **script** | 2,300+ 行 | 脚本解析器（30 个命令）、表达式解析、函数调用栈、循环控制、98 测试 |
| **gat** | 386 行 | .gat 文件解析器（rAthena 格式 v1-5）、MapState 真实碰撞检测 |
| **migration** | 420 行 | 数据库迁移框架（schema_version 版本追踪、事务保护、幂等执行） |

### 客户端 (Devi) — 6,500+ 行，40 个源文件，80+ 测试

| 模块 | 状态 | 说明 |
|------|------|------|
| 核心框架 | ✅ | Bevy ECS + Cargo Workspace、配置系统、游戏状态机、Tick 系统 |
| 网络层 | ✅ | Legacy TCP + WebSocket 双传输层、17 种包类型编解码、会话管理 |
| 登录流程 | ✅ | 用户名/密码 UI、角色选择/创建、地图连接 |
| 渲染引擎 | ✅ | 等距相机、地形网格、精灵动画、Billboard、HUD、聊天窗口 |
| 实体系统 | ✅ | 实体同步（Appear/Disappear/Move）、角色攻击、瓦片地图渲染 |
| 聊天 IME | ✅ | Bevy Ime 事件处理、5 种消息类型、100 条历史缓冲 |
| 像素增强 | 📋 | 规划中 |
| 资源热更新 | 📋 | 规划中 |
| 启动器 | 📋 | 规划中 |

### 智能助手 (DeviAgent) — 1,500+ 行，16 个源文件

| 模块 | 说明 |
|------|------|
| REPL 交互 | 命令行实时查询服务端状态（在线玩家、地图信息等） |
| 知识库 | YAML 文档索引 + 全文检索 |
| TCP 通信 | 直连服务端，支持 Windows 跨平台 |

## 技术栈

| 组件 | 技术 |
|------|------|
| 服务端 | Rust, Tokio, SQLite, MySQL（可选） |
| 客户端 | Rust, Bevy, wgpu |
| 智能助手 | Rust, Tokio, SQLite |
| 协议 | rAthena 二进制包（兼容）+ WebSocket |
| 构建 | Cargo Workspace |

## 快速开始

```bash
# 克隆项目
git clone https://github.com/Cnkore007/Deviruchi.git
cd Deviruchi

# 编译全部
cargo build --release

# 运行服务端
./target/release/deviruchi

# 运行智能助手
./target/release/devi-agent

# 运行客户端
./target/release/devi
```

## License

GPL-3.0
