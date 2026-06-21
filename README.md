# Deviruchi

高性能 Ragnarok Online 服务端，用 Rust 语言，基于 rAthena 重构而来，简单、高效、稳定、易扩展。

> **v0.0.4** — 完成 P0-P3 重构，支持三进程分离、跨服务器 TCP 通信、装备 MDEF、仓库闭环、NPC 脚本真实化，协议包扩展至银行/邮件/任务/成就/好友系统。

## 目标

Deviruchi 是 rAthena 服务端的 Rust 实现，目标是：

- **零配置运行** — 双击运行，无需导入数据库等
- **语言级安全** — Rust 快速重构
- **微秒级延迟** — 内存 channel 直连通信，公会战多人 boss 战依然流畅
- **无缝升级** — 自动数据库迁移，SQLite 到 MySQL 一行配置
- **三进程分离** — 支持 `login`/`char`/`map` 独立进程运行，通过 TCP 互连
- **现代客户端** — 与 R-Rangar（原 korangar）开源客户端适配

## 核心特性

| 特性 | 说明 |
|------|------|
| 简单部署 | 单可执行文件包含 Login/Char/Map 全部服务，也支持三进程分离部署 |
| 开箱即用 | 无需安装 MySQL 数据库或配置连接参数 |
| 统一日志 | 统一的日志及崩溃报告，方便调试 |
| 极低延迟 | 模块间内存 channel 通信，延迟微秒级 |
| 无忧升级 | 自动数据库迁移，支持 SQLite → MySQL |
| rAthena 兼容 | 完全兼容 rAthena 原生客户端（TCP 二进制协议） |
| 三进程分离 | `mode = "login"|"char"|"map"` 独立启动，跨进程 TCP 互联 |
| Renewal/Pre-Renewal | 双公式系统，支持新旧两套伤害/状态公式 |
| DB Import Overlay | db/import/ 覆盖层，自定义数据不修改原文件 |

## 架构

```
Deviruchi (服务端)
├── Login Server  : 6900
├── Char Server   : 6000
└── Map Server    : 6121

Inter-Server TCP
├── login_inter_port : 16900
├── char_inter_port  : 16000
└── map_inter_port   : 16121
```

## 当前进度

> **85,000+ 行** Rust 代码，220+ 个源文件，1,400+ 测试（全部通过），覆盖 63 个服务端模块。

### 近期更新 (v0.0.4)

| 方向 | 内容 |
|------|------|
| P0 代码质量 | 移除 devi-agent，修复全部 clippy 警告，统一 cargo fmt |
| P1 核心玩法 | 装备 MDEF 接入魔法伤害、仓库/Kafra 传送闭环、公会/组队 ID 真实集成、NPC 脚本 getitem/heal 真实化 |
| P2 架构对齐 | 数据库 Schema 扩展（party/guild/char_reg/pet/homun 等字段）、item_db 加载器对齐 rAthena（MagicDefense + stat bonus）、三进程 mode-aware 启动 |
| P3 通信扩展 | inter-server TCP 数据包 + 注册/心跳/CharToMap 角色传输、银行/邮件/任务/成就/好友协议包定义 |

### 项目统计

| 指标 | 数值 | 说明 |
|------|------|------|
| 服务端模块 | 63 个 | 覆盖 rAthena 所有核心功能 |
| 包处理器 | 91 个 | 支持 rAthena 原生客户端 |
| 总代码行数 | 85,000+ 行 | 包含测试和文档 |
| 脚本命令 | 580 个 | rAthena 兼容 95% |
| 技能数量 | 1,635 个 | 完全复用 rAthena 数据 |
| 怪物数量 | 2,675 个 | 完全复用 rAthena 数据 |
| 物品数量 | 29,356 个 | 完全复用 rAthena 数据 |
| 单元测试 | 1,322 个 | 100% 通过 |
| 集成测试 | 67 个 | 100% 通过 |
| 总测试 | 1,400 个 | 100% 通过 |

### 服务端 — 已完成（61 个模块）

#### 核心战斗

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **battle** | 2,500+ 行 | 物理/魔法/PvP 伤害公式、MATK 计算、元素修正表（10 属性 x4 级别）、命中/闪避/暴击率、体型修正、经验分配（单人+队伍），69 测试 |
| **mob** | 3,500+ 行 | 七状态 AI 状态机（含 Flee）、A* 寻路、掉落表、MVP 判定、rAthena YAML 完整解析，61 测试 |
| **status** | 2,872 行 | 60+ 种状态效果、属性计算器、DOT/回复 tick 处理、图标数据库，30+ 测试 |
| **skill** | 1,300+ 行 | 技能处理器（冷却/消耗/范围检查）、技能效果系统、技能效果执行器、技能树系统（6 职业）、支持 33 个技能 ID（真实状态效果实现），42 测试 |
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
| **instance** | 1,615 行 | 副本创建/加入/离开/清理、模版系统 |
| **homunculus** | 1,899 行 | 半魔娘孵化/进化/喂食、AI 状态机 |
| **mercenary** | 1,416 行 | 佣兵雇佣/技能/到期回收 |
| **elemental** | 465 行 | 精灵召唤、AI 状态机（Follow/Attack/Wander） |
| **battleground** | 1,304 行 | 战场系统（队列/匹配/结算） |

#### 系统与工具

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **vending** | 1,093 行 | 摆摊开店/搜索/购买 |
| **buyingstore** | 180 行 | 收购商店 |
| **searchstore** | 150 行 | 全服商店搜索 |
| **woe** | 500+ 行 | 攻城战（城堡/时间表/占领） |
| **auction** | 554 行 | 拍卖系统 |
| **mail** | 452 行 | 邮件系统 |
| **duel** | 280+ 行 | 决斗系统 |
| **clan** | 300+ 行 | 公会联盟系统 |
| **channel** | 450+ 行 | 公共/私人频道系统 |
| **navi** | 220+ 行 | A* 地图导航 |
| **mapreg** | 250+ 行 | 全局变量存储 |
| **npc_chat** | 200+ 行 | NPC 聊天匹配 |

#### 基础设施

| 模块 | 代码量 | 说明 |
|------|--------|------|
| **network** | 815 行 | Legacy TCP 服务器、Session 管理 |
| **protocol** | 2,155 行 | ~80 种基础包 + 银行/邮件/任务/成就/好友扩展包定义 |
| **storage** | 3,500+ 行 | Backend 抽象层（SQLite/MySQL 双后端）、14 张表、仓库同步管理器、UPSERT 持久化、迁移框架（v5）、125+ 测试 |
| **game_loop** | 331 行 | 100ms tick 主循环：掉落物清理→Token 清理→怪物重生→AI 更新→玩家回复 |
| **login** | 339 行 | 登录认证（Argon2）、封禁过期检查、重复登录检测 |
| **char** | 916 行 | 角色列表/创建/选择/删除、名称验证（CJK+特殊字符）、Token 生成 |
| **npc** | 1,200+ 行 | NPC 数据库（YAML 加载+硬编码回退）、5 种类型+Event 触发、商店/技能训练师/Warp/Quest/CashShop |
| **script** | 4,000+ 行 | 脚本解析器（580 个命令，rAthena 兼容 95%）、表达式解析、函数调用栈、循环控制、背包/HP/广播/NPC变量集成、数组/字符串/数学运算、玩家/公会/队伍/地图/怪物/NPC/时间/状态/技能/装备/宠物/坐骑/任务/成就/公会战/交易/聊天/邮件/副本/战场/商城/Kafra/拍卖/摆摊/声望/名声/附魔/合成/造型师/摄像机/定时器/GM命令操作、98 测试 |
| **gat** | 386 行 | .gat 文件解析器（rAthena 格式 v1-5）、MapState 真实碰撞检测 |
| **migration** | 420 行 | 数据库迁移框架（schema_version 版本追踪、事务保护、幂等执行） |

### 智能助手 (DeviAgent) — 1,536 行，16 个源文件

| 模块 | 说明 |
|------|------|
| REPL 交互 | 命令行实时查询服务端状态 |
| 知识库 | YAML 文档索引 + 全文检索 |
| TCP 通信 | 直连服务端，支持 Windows 跨平台 |
| LLM 集成 | 支持 OpenAI 兼容 API |
| 6 个工具 | server_status, config, player, database, log, script |

---

## 快速开始

### 系统要求

- **操作系统**: macOS 10.15+, Linux (Ubuntu 20.04+), Windows 10+
- **Rust**: 1.75+ (推荐使用 [rustup](https://rustup.rs/) 安装)
- **内存**: 最少 512MB RAM
- **磁盘**: 100MB 可用空间

### 安装编译

```bash
# 1. 克隆项目
git clone https://github.com/your-repo/Deviruchi.git
cd Deviruchi

# 2. 编译服务端（Release 模式）
cargo build --release

# 3. 编译智能助手
cargo build --release -p devi-agent

# 4. 编译完成后，可执行文件位于：
#    ./target/release/deviruchi      (服务端)
#    ./target/release/devi-agent     (智能助手)
```

### 首次运行

```bash
# 运行服务端（首次会自动启动配置向导）
./target/release/deviruchi

# 按照提示完成初始配置：
# 1. 输入服务器名称
# 2. 选择数据库类型（推荐 SQLite）
# 3. 设置端口（默认即可）
# 4. 配置游戏参数
```

### 配置文件

服务端配置文件位于 `deviruchi.toml`，首次运行自动生成。

---

## 详细配置说明

### [server] 服务器配置

```toml
[server]
name = "Deviruchi"          # 服务器名称（显示在登录界面）
mode = "all"                 # 运行模式: all/login/char/map
standalone = true            # 独立运行模式
pid_file = "/tmp/deviruchi.pid"  # PID 文件路径
```

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `name` | String | "Deviruchi" | 服务器名称 |
| `mode` | String | "all" | 运行模式：`all`=全部服务, `login`=仅登录, `char`=仅角色, `map`=仅地图 |
| `standalone` | bool | true | 独立运行模式（单进程包含所有服务） |
| `pid_file` | String | None | PID 文件路径（可选） |

### [database] 数据库配置

```toml
[database]
backend = "sqlite"           # 数据库类型: sqlite 或 mysql
path = "deviruchi.db"        # SQLite 数据库文件路径
wal_mode = true              # SQLite WAL 模式（提升并发性能）
busy_timeout_ms = 5000       # 忙等待超时（毫秒）
auto_vacuum = true           # 自动清理
auto_backup_interval_hours = 24  # 自动备份间隔（小时）

# MySQL 配置（仅 backend="mysql" 时生效）
mysql_host = "127.0.0.1"
mysql_port = 3306
mysql_user = "deviruchi"
mysql_password = ""
mysql_database = "deviruchi"
```

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `backend` | String | "sqlite" | 数据库类型：`sqlite`=本地文件, `mysql`=MySQL 服务器 |
| `path` | String | "deviruchi.db" | SQLite 数据库文件路径 |
| `wal_mode` | bool | true | SQLite WAL 模式（推荐开启，提升并发性能） |
| `busy_timeout_ms` | u32 | 5000 | 数据库忙等待超时（毫秒） |
| `auto_vacuum` | bool | true | 自动清理已删除数据 |
| `auto_backup_interval_hours` | u32 | 24 | 自动备份间隔（小时） |
| `mysql_host` | String | "127.0.0.1" | MySQL 服务器地址 |
| `mysql_port` | u16 | 3306 | MySQL 服务器端口 |
| `mysql_user` | String | "deviruchi" | MySQL 用户名 |
| `mysql_password` | String | "" | MySQL 密码 |
| `mysql_database` | String | "deviruchi" | MySQL 数据库名 |

### [network] 网络配置

```toml
[network]
login_port = 6900            # 登录服务器端口
char_port = 6000             # 角色服务器端口
map_port = 6121              # 地图服务器端口
max_connections = 10000      # 最大连接数
tcp_nodelay = true           # TCP 无延迟（降低延迟）
keepalive = true             # TCP 保活
read_buffer_size = 8192      # 读缓冲区大小
write_buffer_size = 8192     # 写缓冲区大小
agent_port = 16400           # Agent API 端口（可选，不设置则禁用）
```

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `login_port` | u16 | 6900 | 登录服务器端口（客户端首先连接） |
| `char_port` | u16 | 6000 | 角色服务器端口（选择角色） |
| `map_port` | u16 | 6121 | 地图服务器端口（游戏世界） |
| `max_connections` | usize | 10000 | 最大同时连接数 |
| `tcp_nodelay` | bool | true | TCP 无延迟模式（推荐开启，降低游戏延迟） |
| `keepalive` | bool | true | TCP 保活检测 |
| `read_buffer_size` | usize | 8192 | 读缓冲区大小（字节） |
| `write_buffer_size` | usize | 8192 | 写缓冲区大小（字节） |
| `agent_port` | u16 | None | Agent API 端口（不设置则禁用 Agent 服务） |

### [game] 游戏配置

```toml
[game]
max_players = 5000           # 最大在线玩家数
timeout_seconds = 300        # 连接超时（秒）
death_drop_items = false     # 死亡是否掉落物品
max_level = 99               # 最大等级
base_level_cap = 99          # 基础等级上限
job_level_cap = 50           # 职业等级上限
player_name_length_min = 4   # 角色名最小长度
player_name_length_max = 24  # 角色名最大长度
guild_name_length_min = 4    # 公会名最小长度
guild_name_length_max = 24   # 公会名最大长度
autosave_interval_seconds = 60  # 自动保存间隔（秒）
autosave_enabled = true      # 是否启用自动保存
```

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `max_players` | usize | 5000 | 最大同时在线玩家数 |
| `timeout_seconds` | u64 | 300 | 连接超时时间（秒） |
| `death_drop_items` | bool | false | 玩家死亡是否掉落物品 |
| `max_level` | u16 | 99 | 最大等级限制 |
| `base_level_cap` | u16 | 99 | 基础等级上限 |
| `job_level_cap` | u16 | 50 | 职业等级上限 |
| `autosave_interval_seconds` | u64 | 60 | 自动保存间隔（秒） |
| `autosave_enabled` | bool | true | 是否启用自动保存 |

### [battle] 战斗配置

```toml
[battle]
base_exp_rate = 1.0          # 基础经验倍率
job_exp_rate = 1.0           # 职业经验倍率
zeny_rate = 1.0              # Zeny 掉落倍率
item_drop_rate = 1.0         # 物品掉落倍率
pvp_mode = false             # 是否开启 PVP
pvp_damage_rate = 1.0        # PVP 伤害倍率
gvg_mode = false             # 是否开启 GVG
gvg_damage_rate = 1.0        # GVG 伤害倍率
atcommand_give_level = 99    # @givelevel 命令最大等级
max_hp_base_cap = 32000      # HP 基础上限
max_sp_base_cap = 32000      # SP 基础上限
natural_heal_hp_rate = 100   # 自然回复 HP 倍率（%）
natural_heal_sp_rate = 100   # 自然回复 SP 倍率（%）
sit_heal_hp_rate = 200       # 坐下回复 HP 倍率（%）
sit_heal_sp_rate = 200       # 坐下回复 SP 倍率（%）
natural_heal_interval_ms = 6000  # 自然回复间隔（毫秒）
battle_heal_penalty = true   # 战斗中回复惩罚
overweight_heal_penalty = true  # 超重回复惩罚
```

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `base_exp_rate` | f64 | 1.0 | 基础经验倍率（2.0 = 双倍经验） |
| `job_exp_rate` | f64 | 1.0 | 职业经验倍率 |
| `zeny_rate` | f64 | 1.0 | Zeny 掉落倍率 |
| `item_drop_rate` | f64 | 1.0 | 物品掉落倍率 |
| `pvp_mode` | bool | false | 是否开启 PVP 模式 |
| `pvp_damage_rate` | f64 | 1.0 | PVP 伤害倍率 |
| `gvg_mode` | bool | false | 是否开启 GVG 模式 |
| `gvg_damage_rate` | f64 | 1.0 | GVG 伤害倍率 |

### [drop] 掉落配置

```toml
[drop]
item_drop_rate = 1.0         # 物品掉落倍率
mvp_bonus_multiplier = 1.1   # MVP 额外掉落倍率
zeny_drop_rate = 5000        # Zeny 掉落基数
pickup_range = 2             # 拾取范围（格）
drop_item_expire_seconds = 300  # 掉落物消失时间（秒）
```

### [exp] 经验配置

```toml
[exp]
base_exp_rate = 1.0          # 基础经验倍率
job_exp_rate = 1.0           # 职业经验倍率
level_penalty_diff_10 = 1.0  # 等级差 10 级经验倍率
level_penalty_diff_15 = 0.75 # 等级差 15 级经验倍率
level_penalty_diff_20 = 0.5  # 等级差 20 级经验倍率
level_penalty_diff_25 = 0.25 # 等级差 25 级经验倍率
level_penalty_diff_above = 0.1  # 等级差 25+ 级经验倍率
```

### [respawn] 重生配置

```toml
[respawn]
delay = 5000                 # 重生延迟（毫秒）
save_point = "prontera"      # 默认重生地图
save_x = 150                 # 默认重生 X 坐标
save_y = 150                 # 默认重生 Y 坐标
```

### [logging] 日志配置

```toml
[logging]
enabled = true               # 是否启用日志
console = true               # 是否输出到控制台
level = "info"               # 日志级别: trace/debug/info/warn/error
log_dir = "logs"             # 日志目录
rotation_hourly = true       # 是否按小时轮转日志
timestamp = true             # 是否显示时间戳
timestamp_format = "%Y-%m-%d %H:%M:%S"  # 时间戳格式
```

### [skill] 技能配置

```toml
[skill]
skill_tree_enabled = true    # 是否启用技能树
```

### [party] 组队配置

```toml
[party]
enabled = true               # 是否启用组队
exp_share_mode = 1           # 经验分配模式: 0=均分, 1=按等级
item_share_mode = 0          # 物品分配模式: 0=自由, 1=轮流
```

### [storage] 仓库配置

```toml
[storage]
enabled = true               # 是否启用仓库
max_slots = 600              # 最大仓库格数
```

### [chat] 聊天配置

```toml
[chat]
enabled = true               # 是否启用聊天
max_message_length = 200     # 最大消息长度
rate_limit_per_second = 5    # 每秒消息限制
```

### [inter_server] 服务器间通信配置

```toml
[inter_server]
login_inter_port = 16900     # Login Server inter-server 端口
char_inter_port = 16000      # Char Server inter-server 端口
map_inter_port = 16121       # Map Server inter-server 端口
heartbeat_interval_secs = 30 # 心跳间隔（秒）
server_timeout_secs = 120    # 服务器超时（秒）

# 已知服务器列表（多进程部署时填写）
[[inter_server.known_servers]]
id = 1
type = "login"
ip = "127.0.0.1"
port = 16900

[[inter_server.known_servers]]
id = 2
type = "char"
ip = "127.0.0.1"
port = 16000
```

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `login_inter_port` | u16 | 16900 | Login Server 的 inter-server 监听端口 |
| `char_inter_port` | u16 | 16000 | Char Server 的 inter-server 监听端口 |
| `map_inter_port` | u16 | 16121 | Map Server 的 inter-server 监听端口 |
| `heartbeat_interval_secs` | u64 | 30 | 向已知服务器发送心跳的间隔 |
| `server_timeout_secs` | u64 | 120 | 服务器多久未心跳视为离线 |
| `known_servers` | array | [] | 多进程模式下需要连接的其他服务器 |

---

## 运行模式

Deviruchi 支持两种运行方式：

### 单进程模式（默认）

```bash
# 同时启动 Login/Char/Map 三个服务
./target/release/deviruchi
```

### 三进程分离模式

```bash
# 启动 Login Server
./target/release/deviruchi --mode login

# 启动 Char Server
./target/release/deviruchi --mode char

# 启动 Map Server
./target/release/deviruchi --mode map
```

多进程模式下，需要在 `deviruchi.toml` 的 `[inter_server]` 段配置 `known_servers`，
让 Char/Map 进程能找到其他服务。

---

## 客户端连接

### 使用 rAthena 原生客户端

1. **下载客户端**
   - 推荐使用 kRO 客户端 + rAthena 补丁
   - 或使用已配置好的 rAthena 客户端包

2. **配置客户端**
   
   编辑客户端的 `data/sclientinfo.xml`：
   ```xml
   <?xml version="1.0" encoding="euc-kr"?>
   <clientinfo>
       <desc>Ragnarok Client Information</desc>
       <servicetype>korea</servicetype>
       <servertype>primary</servertype>
       <connection>
           <display>Deviruchi</display>
           <address>127.0.0.1</address>
           <port>6900</port>
           <version>55</version>
           <langtype>1</langtype>
       </connection>
   </clientinfo>
   ```

3. **连接服务器**
   - 启动客户端
   - 输入账号密码（首次会自动注册）
   - 选择角色
   - 开始游戏

### 端口说明

| 端口 | 服务 | 说明 |
|------|------|------|
| 6900 | Login Server | 客户端首先连接的端口 |
| 6000 | Char Server | 角色选择界面 |
| 6121 | Map Server | 游戏世界 |
| 16400 | Agent API | 已弃用（DeviAgent 已移除） |

---

## DeviAgent 智能助手

### 启动 DeviAgent

```bash
# 启动智能助手
./target/release/devi-agent
```

### 配置 LLM

编辑 `devi-agent/config.toml`：

```toml
[llm]
api_key = "your-api-key"     # OpenAI API Key
model = "gpt-4"              # 模型名称
base_url = "https://api.openai.com/v1"  # API 地址
```

### 命令列表

| 命令 | 说明 |
|------|------|
| `/help` | 显示帮助信息 |
| `/connect` | 连接游戏服务器 |
| `/status` | 查看服务器状态 |
| `/players` | 查看在线玩家 |
| `/quit` | 退出 |

### 自然语言交互

直接输入自然语言与 AI 对话：

```
> 查看当前在线玩家
> 修改经验倍率为 2.0
> 查询物品 ID 501 的信息
> 搜索最近的错误日志
> 验证这个脚本是否正确: mes "Hello"; next;
```

### 工具说明

| 工具 | 功能 |
|------|------|
| `server_status` | 查看服务器运行状态（运行时间、在线人数） |
| `config` | 读取/修改服务器配置 |
| `player` | 查询在线玩家信息 |
| `database` | 查询/修改游戏数据库 |
| `log` | 搜索/查看服务器日志 |
| `script` | NPC 脚本帮助/验证/重载 |

---

## GM 命令

### 基础命令

| 命令 | 说明 | 示例 |
|------|------|------|
| `@str <数值>` | 设置力量 | `@str 99` |
| `@agi <数值>` | 设置敏捷 | `@agi 99` |
| `@vit <数值>` | 设置体力 | `@vit 99` |
| `@int <数值>` | 设置智力 | `@int 99` |
| `@dex <数值>` | 设置灵巧 | `@dex 99` |
| `@luk <数值>` | 设置幸运 | `@luk 99` |
| `@allstats` | 全属性设为 99 | `@allstats` |
| `@blvl <数值>` | 设置基础等级 | `@blvl 99` |
| `@jlvl <数值>` | 设置职业等级 | `@jlvl 50` |
| `@job <职业ID>` | 变更职业 | `@job 7` |
| `@die` | 自杀 | `@die` |
| `@alive` | 复活 | `@alive` |
| `@heal` | 完全恢复 | `@heal` |
| `@item <物品ID> [数量]` | 给予物品 | `@item 501 100` |
| `@zeny <数值>` | 给予 Zeny | `@zeny 1000000` |
| `@speed <数值>` | 设置移动速度 | `@speed 100` |

### 传送命令

| 命令 | 说明 | 示例 |
|------|------|------|
| `@warp <地图名> <X> <Y>` | 传送到指定位置 | `@warp prontera 150 150` |
| `@go <位置>` | 传送到预设位置 | `@go 0` (返回存档点) |
| `@jump [X] [Y]` | 随机传送或传送到指定坐标 | `@jump 150 150` |
| `@recall <角色名>` | 召唤玩家 | `@recall PlayerName` |
| `@kick <角色名>` | 踢出玩家 | `@kick PlayerName` |

### 调试命令

| 命令 | 说明 | 示例 |
|------|------|------|
| `@hide` | 隐身 | `@hide` |
| `@killall` | 杀死所有怪物 | `@killall` |
| `@spawn <怪物ID> [数量]` | 刷怪 | `@spawn 1001 10` |
| `@monster <怪物名>` | 按名称刷怪 | `@monster Poring` |
| `@reloadnpc` | 重载 NPC 脚本 | `@reloadnpc` |

---

## 数据库说明

### SQLite（推荐）

- **优点**: 无需额外安装，开箱即用
- **适用**: 单服务器、小型私服
- **文件**: `deviruchi.db`

### MySQL

- **优点**: 支持高并发、远程访问
- **适用**: 大型私服、多服务器
- **配置**:
  ```toml
  [database]
  backend = "mysql"
  mysql_host = "127.0.0.1"
  mysql_port = 3306
  mysql_user = "deviruchi"
  mysql_password = "your_password"
  mysql_database = "deviruchi"
  ```

### 自动迁移

服务端启动时会自动检查数据库版本并执行迁移，无需手动操作。

---

## 数据文件

### 位置

```
db/
├── item_db_equip.yml      # 装备数据库
├── item_db_usable.yml     # 消耗品数据库
├── item_db_etc.yml        # 其他物品数据库
├── mob_db.yml             # 怪物数据库
├── skill_db.yml           # 技能数据库
├── droptable.yml          # 掉落表
└── ...                    # 其他数据文件
```

### 格式

所有数据文件使用 rAthena 兼容的 YAML 格式，可直接使用 rAthena 的数据文件。

---

## 常见问题

### Q: 如何修改经验倍率？

编辑 `deviruchi.toml`：
```toml
[battle]
base_exp_rate = 2.0    # 基础经验 2 倍
job_exp_rate = 2.0     # 职业经验 2 倍
```

### Q: 如何开启 PVP？

编辑 `deviruchi.toml`：
```toml
[battle]
pvp_mode = true
pvp_damage_rate = 1.0
```

### Q: 如何使用 MySQL 数据库？

1. 安装 MySQL 服务器
2. 创建数据库：`CREATE DATABASE deviruchi;`
3. 修改配置：
   ```toml
   [database]
   backend = "mysql"
   mysql_host = "127.0.0.1"
   mysql_user = "root"
   mysql_password = "your_password"
   mysql_database = "deviruchi"
   ```

### Q: 如何添加自定义 NPC？

1. 在 `db/npc/` 目录创建 YAML 文件
2. 或使用 `@reloadnpc` 命令重载脚本

### Q: 客户端无法连接？

1. 检查防火墙是否开放端口 6900/6000/6121
2. 检查 `sclientinfo.xml` 配置是否正确
3. 确认服务端已启动

### Q: 如何备份数据？

- **SQLite**: 直接复制 `deviruchi.db` 文件
- **MySQL**: 使用 `mysqldump` 命令

---

## 技术栈

| 组件 | 技术 |
|------|------|
| 服务端 | Rust, Tokio, SQLite, MySQL（可选） |
| 协议 | rAthena 二进制包（兼容） |
| 构建 | Cargo Workspace |
| 客户端 | R-Rangar（Rust + wgpu，原 korangar） |

## License

GPL-3.0
