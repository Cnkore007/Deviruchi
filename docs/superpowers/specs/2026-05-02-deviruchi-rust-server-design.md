# Deviruchi Rust 服务端 - 架构设计文档

**日期**: 2026-05-02
**状态**: 已批准
**版本**: 1.0

## 1. 项目概述

### 1.1 项目目标
使用 Rust 语言重写 rAthena 游戏服务端，创建名为 Deviruchi 的高性能、可扩展游戏服务器。

### 1.2 核心需求
1. 单个可执行文件包含全部服务，双击即可运行
2. 规模增长时可切换为分布式部署
3. 内置 SQLite 引擎，无需安装数据库
4. 自动生成中文崩溃报告，精确到函数和行号
5. 编译期杜绝缓冲区溢出和 SQL 注入
6. 支持原生客户端和新客户端（协议兼容 + 增强）

### 1.3 客户端兼容性
- 兼容原生客户端（老协议版本）
- 支持新客户端（新协议版本，增强像素和效果）
- 服务端处理版本协商和协议切换

## 2. 架构设计

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                      Deviruchi Server                        │
│                    (单个可执行文件)                           │
├─────────────────────────────────────────────────────────────┤
│  入口层 (cli)                                               │
│  ├── 单机模式：所有模块在进程内直接调用                         │
│  └── 分布式模式：启动多个进程，通过网络通信                     │
├─────────────────────────────────────────────────────────────┤
│  核心层 (core)                                              │
│  ├── 配置管理 (YAML/TOML)                                    │
│  ├── 日志系统 (中文崩溃报告)                                  │
│  ├── 定时器系统 (min-heap event loop)                        │
│  └── 插件系统 (热加载扩展)                                    │
├─────────────────────────────────────────────────────────────┤
│  网络层 (network)                                           │
│  ├── 协议解析器 (Packet Parser)                              │
│  │   ├── 版本协商                                           │
│  │   ├── 数据包分片/重组                                     │
│  │   └── 加密/混淆支持                                       │
│  └── 网络 I/O (Tokio async)                                 │
├─────────────────────────────────────────────────────────────┤
│  业务层 (game)                                               │
│  ├── Login Server (账户认证、会话管理)                        │
│  ├── Char Server (角色管理、工会、公会)                       │
│  ├── Map Server (游戏逻辑核心)                               │
│  │   ├── 玩家状态机                                         │
│  │   ├── 战斗系统                                           │
│  │   ├── AI 系统                                            │
│  │   └── 地图管理                                           │
│  └── Web Server (API、管理后台)                              │
├─────────────────────────────────────────────────────────────┤
│  数据层 (storage)                                           │
│  ├── SQLite 内嵌引擎                                         │
│  ├── 数据模型 (Player, Item, Guild...)                      │
│  └── 迁移系统                                               │
├─────────────────────────────────────────────────────────────┤
│  脚本层 (script)                                             │
│  ├── rAthena 脚本兼容层                                      │
│  └── Rust 原生事件系统                                       │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 部署模式

**单机模式**（默认）
- 所有服务运行在单一进程中
- 适合 1-5000 同时在线
- 零配置，开箱即用

**分布式模式**
- 多个进程协同工作
- Login/Char/Map Server 可独立扩展
- 适合 5000+ 同时在线

```toml
# 配置示例
[server]
mode = "distributed"
nodes = ["login:6900", "char:6000", "map1:6121", "map2:6122"]
```

## 3. 技术选型

| 组件 | 选型 | 说明 |
|------|------|------|
| 语言 | Rust 2024 Edition | 内存安全、零成本抽象 |
| 运行时 | Tokio | 异步 I/O、高性能网络 |
| 数据库 | SQLite (rusqlite) | 单文件、零配置 |
| 序列化 | rkyv / bincode | 二进制序列化，高性能 |
| 日志 | tracing + custom | 结构化日志 + 中文崩溃报告 |
| 配置 | toml | TOML 配置文件 |
| 宏 | proc_macro | 编译期协议验证 |
| FFI | 无 | 纯 Rust 实现 |

## 4. 核心模块设计

### 4.1 网络层 (network)

**协议解析器**
- 完整协议栈实现
- 版本协商机制
- 数据包分片/重组
- 加密/混淆支持

**网络 I/O**
- 基于 Tokio 异步运行时
- 非阻塞 I/O
- 连接池管理

### 4.2 数据层 (storage)

**SQLite 集成**
- 内嵌数据库引擎
- 参数化查询杜绝 SQL 注入
- 自动建表和数据迁移

**数据模型**
- Player: 玩家角色数据
- Item: 物品数据
- Guild: 公会数据
- Party: 队伍数据

### 4.3 脚本层 (script)

**混合方案**
- Phase 1: 兼容 rAthena 脚本引擎
- Phase 2: 逐步迁移到 Rust 原生事件系统

## 5. 安全特性

### 5.1 编译期防护
- 使用 `Vec<T>` 而非裸指针
- `rusqlite` 参数化查询杜绝 SQL 注入
- 协议数据包大小边界检查
- 编译期断言验证

### 5.2 运行时防护
- panic 捕获生成中文堆栈
- 连接限流/防 DDoS
- 输入验证层
- 超时机制

## 6. 崩溃报告

### 6.1 中文崩溃报告格式
```
=====================================
       Deviruchi 崩溃报告
=====================================
时间: 2026-05-02 15:30:00
版本: 0.1.0

崩溃位置:
  文件: src/map/battle.rs
  函数: calculate_damage
  行号: 245

调用栈:
  1. calculate_damage (battle.rs:245)
  2. process_skill_attack (skill.rs:892)
  3. handle_packet (handler.rs:1204)
  4. process_packet (network.rs:340)
  5. main (main.rs:50)

环境信息:
  操作系统: Windows 11
  内存使用: 512MB
  在线玩家: 1234
=====================================
```

## 7. 开发阶段

### Phase 1 - 核心骨架（2-3周）
- 项目结构搭建
- 配置系统
- 日志与崩溃报告
- SQLite 集成
- 基础网络框架

### Phase 2 - 协议栈（4-6周）
- 协议解析器
- 登录流程
- 角色数据流
- Map Server 核心

### Phase 3 - 游戏逻辑（6-8周）
- 战斗系统
- NPC 脚本引擎
- 物品/技能/怪物系统
- 地图管理

### Phase 4 - 分布式（3-4周）
- 服务间通信
- 负载均衡
- 状态同步
- 容错处理

## 8. 目录结构

```
deviruchi/
├── src/
│   ├── main.rs              # 入口
│   ├── cli/                 # 命令行参数
│   ├── core/                # 核心模块
│   │   ├── config.rs        # 配置管理
│   │   ├── logging.rs       # 日志系统
│   │   ├── panic.rs         # 崩溃处理
│   │   ├── timer.rs         # 定时器
│   │   └── plugin.rs        # 插件系统
│   ├── network/             # 网络层
│   │   ├── mod.rs
│   │   ├── packet.rs        # 协议解析
│   │   ├── protocol.rs      # 协议定义
│   │   └── session.rs       # 会话管理
│   ├── game/                # 游戏逻辑
│   │   ├── login.rs         # 登录服务
│   │   ├── char.rs          # 角色服务
│   │   ├── map/             # 地图服务
│   │   │   ├── mod.rs
│   │   │   ├── player.rs    # 玩家
│   │   │   ├── battle.rs    # 战斗
│   │   │   ├── skill.rs     # 技能
│   │   │   ├── item.rs      # 物品
│   │   │   ├── mob.rs       # 怪物
│   │   │   ├── npc.rs       # NPC
│   │   │   └── map.rs       # 地图
│   │   └── web.rs           # Web 服务
│   ├── storage/             # 数据层
│   │   ├── mod.rs
│   │   ├── sqlite.rs        # SQLite 封装
│   │   ├── player.rs        # 玩家数据
│   │   ├── item.rs          # 物品数据
│   │   └── guild.rs         # 公会数据
│   └── script/              # 脚本层
│       ├── mod.rs
│       ├── lexer.rs         # 词法分析
│       ├── parser.rs        # 语法分析
│       ├── vm.rs            # 虚拟机
│       └── builtin.rs       # 内置命令
├── Cargo.toml
├── deviruchi.toml           # 配置文件
└── README.md
```

## 9. 数据库 Schema

```sql
-- 账户表
CREATE TABLE accounts (
    account_id INTEGER PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    sex INTEGER NOT NULL,
    email TEXT,
    group_id INTEGER DEFAULT 0,
    state INTEGER DEFAULT 0,
    unban_time INTEGER DEFAULT 0,
    expiration_time INTEGER DEFAULT 0,
    logcount INTEGER DEFAULT 0,
    last_login INTEGER,
    created_at INTEGER NOT NULL
);

-- 角色表
CREATE TABLE characters (
    char_id INTEGER PRIMARY KEY,
    account_id INTEGER NOT NULL,
    char_num INTEGER NOT NULL,
    name TEXT NOT NULL,
    -- 基础属性
    class INTEGER DEFAULT 0,
    base_level INTEGER DEFAULT 1,
    job_level INTEGER DEFAULT 1,
    base_exp INTEGER DEFAULT 0,
    job_exp INTEGER DEFAULT 0,
    zeny INTEGER DEFAULT 0,
    -- 属性点
    str INTEGER DEFAULT 1,
    agi INTEGER DEFAULT 1,
    vit INTEGER DEFAULT 1,
    int INTEGER DEFAULT 1,
    dex INTEGER DEFAULT 1,
    luk INTEGER DEFAULT 1,
    -- 外观
    hair INTEGER DEFAULT 1,
    hair_color INTEGER DEFAULT 0,
    clothes_color INTEGER DEFAULT 0,
    body INTEGER DEFAULT 0,
    weapon INTEGER DEFAULT 0,
    shield INTEGER DEFAULT 0,
    head_top INTEGER DEFAULT 0,
    head_mid INTEGER DEFAULT 0,
    head_bottom INTEGER DEFAULT 0,
    -- 位置
    last_map TEXT,
    last_x INTEGER,
    last_y INTEGER,
    save_map TEXT,
    save_x INTEGER,
    save_y INTEGER,
    -- 状态
    hp INTEGER DEFAULT 1,
    max_hp INTEGER DEFAULT 1,
    sp INTEGER DEFAULT 1,
    max_sp INTEGER DEFAULT 1,
    option INTEGER DEFAULT 0,
    manner INTEGER DEFAULT 0,
    status_point INTEGER DEFAULT 0,
    skill_point INTEGER DEFAULT 0,
    -- 时间戳
    delete_timer INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (account_id) REFERENCES accounts(account_id)
);

-- 背包物品表
CREATE TABLE inventory (
    id INTEGER PRIMARY KEY,
    char_id INTEGER NOT NULL,
    nameid INTEGER NOT NULL,
    amount INTEGER NOT NULL,
    equipped INTEGER DEFAULT 0,
    identify INTEGER DEFAULT 1,
    refine INTEGER DEFAULT 0,
    attribute INTEGER DEFAULT 0,
    card0 INTEGER DEFAULT 0,
    card1 INTEGER DEFAULT 0,
    card2 INTEGER DEFAULT 0,
    card3 INTEGER DEFAULT 0,
    FOREIGN KEY (char_id) REFERENCES characters(char_id)
);

-- 技能表
CREATE TABLE skills (
    id INTEGER PRIMARY KEY,
    char_id INTEGER NOT NULL,
    id INTEGER NOT NULL,
    lv INTEGER NOT NULL,
    flag INTEGER DEFAULT 0,
    FOREIGN KEY (char_id) REFERENCES characters(char_id)
);

-- 公会表
CREATE TABLE guilds (
    guild_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    master INTEGER NOT NULL,
    guild_lv INTEGER DEFAULT 1,
    exp INTEGER DEFAULT 0,
    emblem_data BLOB,
    created_at INTEGER NOT NULL
);

-- 公会成员表
CREATE TABLE guild_members (
    guild_id INTEGER NOT NULL,
    char_id INTEGER NOT NULL,
    position INTEGER DEFAULT 0,
    PRIMARY KEY (guild_id, char_id),
    FOREIGN KEY (guild_id) REFERENCES guilds(guild_id),
    FOREIGN KEY (char_id) REFERENCES characters(char_id)
);
```

## 10. 协议版本协商

```
Client → Server: 发送版本信息包
Server: 验证版本兼容性
Server → Client: 返回协商结果

支持版本:
- 0x0164 - 经典版 (原生兼容)
- 0x0172 - 2016+
- 0x0198 - 2018+
- 0x0207 - 2020+ (新客户端)
```

## 11. 实现优先级

1. **P0 - 必须**: 配置系统、日志系统、崩溃报告、SQLite 集成
2. **P0 - 必须**: 网络框架、协议解析器
3. **P0 - 必须**: Login Server 完整流程
4. **P1 - 重要**: Char Server 完整流程
5. **P1 - 重要**: Map Server 核心（玩家状态、地图）
6. **P1 - 重要**: 战斗系统基础
7. **P2 - 次要**: NPC 脚本引擎
8. **P2 - 次要**: 高级功能（工会、队伍等）
9. **P3 - 可选**: 分布式部署
