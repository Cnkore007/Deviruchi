# DeviAgent — 服务端智能助手设计文档

## 概述

DeviAgent 是 Deviruchi 服务器的独立智能助手进程，通过自然语言（LLM）与管理员交互，提供配置管理、数据库编辑、玩家监控、脚本编写等全功能管理能力。

## 设计目标

1. **独立进程** — 与游戏服务器隔离，服务端崩溃不影响 Agent 运行
2. **自然语言交互** — 通过 LLM（OpenAI 兼容格式）理解模糊指令
3. **全功能覆盖** — 配置、数据库、玩家、脚本、状态、日志六大能力
4. **持久记忆** — 对话历史和学习模式跨会话保存
5. **安全可靠** — 修改前确认、自动备份、破坏性操作二次确认

## 架构

```
devi-agent (独立 Rust 进程)
├── Terminal REPL           ← 用户交互入口
├── LLM Client              ← OpenAI 兼容 API 客户端
├── Tool System             ← 10 个工具（配置/数据库/玩家/脚本/状态/日志）
├── IPC Client              ← Unix Socket + JSON-RPC
├── Knowledge Index         ← 代码库知识（自动生成）
└── Memory Store            ← SQLite 持久化记忆
```

### 通信流

```
用户输入 → REPL → LLM (Function Calling)
                    ↓
              LLM 返回 tool_use
                    ↓
              Agent 执行工具 → IPC (Unix Socket) → 游戏服务器
                    ↓
              工具结果 → LLM → 自然语言回复 → REPL 显示
```

## IPC 协议

Unix Socket + JSON-RPC 风格协议。

### 请求格式

```json
{"id": 1, "method": "player.list", "params": {"map": "prontera"}}
```

### 响应格式

```json
{"id": 1, "result": {"players": [{"name": "TestPlayer", "map": "prontera", "x": 150, "y": 150}]}}
{"id": 2, "error": {"code": -1, "message": "Player not found"}}
```

### 方法清单

| 方法 | 说明 | 参数 |
|------|------|------|
| `server.status` | 服务器运行状态 | 无 |
| `config.get` | 读取配置 | `{section: string}` |
| `config.set` | 修改配置 | `{section: string, key: string, value: string}` |
| `config.reload` | 触发热重载 | 无 |
| `mob.query` | 查询怪物 | `{name?: string, id?: number}` |
| `mob.update` | 修改怪物属性 | `{id: number, field: string, value: string}` |
| `item.query` | 查询物品 | `{name?: string, id?: number}` |
| `item.update` | 修改物品属性 | `{id: number, field: string, value: string}` |
| `skill.query` | 查询技能 | `{name?: string, id?: number}` |
| `drop.query` | 查询掉落表 | `{mob_id?: number, item_id?: number}` |
| `drop.update` | 修改掉落 | `{mob_id: number, item_id: number, rate: number}` |
| `player.list` | 在线玩家列表 | `{map?: string}` |
| `player.info` | 玩家详情 | `{name: string}` |
| `player.warp` | 传送玩家 | `{name: string, map: string, x: number, y: number}` |
| `player.kick` | 踢出玩家 | `{name: string}` |
| `script.create` | 创建/修改 NPC 脚本 | `{npc_name: string, script: string}` |
| `script.validate` | 验证脚本语法 | `{script: string}` |
| `log.search` | 搜索日志 | `{keyword: string, category?: string, since?: string}` |

## LLM 集成

### 工具定义（Function Calling）

Agent 向 LLM 暴露上述 IPC 方法作为工具，LLM 决定调用哪个。工具参数映射到 IPC 方法参数。

### System Prompt 核心

```
你是 DeviAgent，Deviruchi（Ragnarok Online 服务器模拟器）的智能助手。

你熟悉整个服务器代码库，包括：
- 配置系统：TOML 格式，13 个节（server, database, network, game, battle, drop, exp, respawn, log, skill, party, storage, chat）
- 数据库：rAthena 兼容 YAML 文件（item_db, mob_db, skill_db, droptable 等）
- 脚本系统：自定义行式脚本语言（mes, next, close, select, warp, getitem 等命令）
- 网络协议：rAthena 兼容二进制协议 + WebSocket JSON 协议

你的能力：
1. 读取和修改服务器配置，支持热重载
2. 查询和编辑游戏数据库（物品、怪物、技能、掉落表）
3. 查看在线玩家、位置、状态、背包
4. 创建和修改 NPC 脚本、物品脚本
5. 监控服务器运行状态（CPU、内存、Tick 延迟）
6. 搜索服务器日志

规则：
- 修改配置或数据前，先展示当前值和修改后的值，确认后再执行
- 涉及破坏性操作（删除数据、重启服务器）必须二次确认
- 所有修改自动备份原文件
- 用中文回复
```

### LLM 后端

支持 OpenAI 兼容 API 格式，可通过配置切换：
- OpenAI（GPT-4）
- Claude（Anthropic API，OpenAI 兼容模式）
- 本地模型（Ollama，OpenAI 兼容模式）

配置文件：`~/.devi-agent/config.toml`

```toml
[llm]
provider = "openai"  # openai, claude, ollama
api_key = "sk-..."
base_url = "https://api.openai.com/v1"  # 可改为 Ollama 地址
model = "gpt-4"
max_tokens = 4096
```

## 知识索引

Agent 启动时自动从源码生成知识索引，检测到源码变化时自动更新。

### 生成内容

```
~/.devi-agent/knowledge/
├── codebase.md          # 模块结构、关键文件路径
├── schemas/
│   ├── server.toml.md   # 配置 schema + 每个字段说明
│   ├── item_db.md       # 物品数据库字段说明
│   ├── mob_db.md        # 怪物数据库字段说明
│   ├── skill_db.md      # 技能数据库字段说明
│   ├── droptable.md     # 掉落表字段说明
│   └── script.md        # 脚本语言命令参考
└── examples/
    ├── npc_shop.yml     # NPC 脚本示例
    └── mob_custom.yml   # 自定义怪物示例
```

### 生成逻辑

1. 扫描 `src/` 目录结构，生成模块树
2. 解析 `Config` 结构体字段，生成配置 schema
3. 解析 YAML loader 中的字段映射，生成数据库 schema
4. 解析 `ScriptCommand` 枚举，生成脚本命令参考
5. 从 `db/` 目录提取示例数据

## 持久记忆

使用 SQLite 存储，路径：`~/.devi-agent/memory.db`

### 表结构

```sql
-- 对话历史
CREATE TABLE conversations (
    id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL,
    role TEXT NOT NULL,  -- 'user' 或 'assistant'
    content TEXT NOT NULL
);

-- 工具调用记录
CREATE TABLE tool_calls (
    id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    params TEXT NOT NULL,  -- JSON
    result TEXT,           -- JSON
    success BOOLEAN
);

-- 学习到的模式/偏好
CREATE TABLE learnings (
    id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL,
    category TEXT NOT NULL,  -- 'preference', 'pattern', 'correction'
    key TEXT NOT NULL,
    value TEXT NOT NULL
);
```

### 记忆使用

- 每次对话开始时，加载最近 20 条对话历史作为上下文
- LLM 可通过 `memory_search` 工具搜索历史记录
- Agent 自动记录用户的修改偏好（如"这个用户总是先备份再修改"）

## 容灾设计

1. **心跳检测** — Agent 每 5 秒向游戏服务器发送心跳
2. **断连提示** — 检测到服务器断开时，在 REPL 显示警告
3. **不自动重启** — 只提示，不尝试重启游戏服务器
4. **状态快照** — 服务器断开前的最后状态保存在内存中，Agent 仍可查看
5. **自动重连** — 服务器恢复后自动重连

## REPL 体验

```
$ devi-agent

╔══════════════════════════════════════════════════════════╗
║  DeviAgent v0.1 — Deviruchi 智能助手                     ║
║  连接状态: ✓ 已连接 (unix:///tmp/deviruchi.sock)         ║
║  在线玩家: 23 | 运行时间: 3h 22m                         ║
║  输入 /help 查看命令，直接输入自然语言对话                 ║
╚══════════════════════════════════════════════════════════╝

DeviAgent> 帮我把波利的血量改成 500

我来帮你修改 Poring 的 HP。先查看当前值：

  Poring (ID: 1002)
  - 当前 HP: 520
  - 修改后: 500

确认修改？[Y/n] Y

✓ 已修改 mob_db.yml，Poring HP: 520 → 500
✓ 已触发热重载，新数据将在下次 spawn 生效

DeviAgent> /status

服务器状态:
  运行时间: 3h 22m 15s
  CPU: 12.3% | 内存: 340 MB
  Tick 延迟: 98ms (avg) / 120ms (max)
  在线玩家: 23 | 活跃怪物: 1,247
  最近错误: 0 (最近 1 小时)
```

## 安全设计

1. **权限控制** — Agent 连接需要认证 token
2. **修改确认** — 所有数据修改先展示 diff，确认后执行
3. **自动备份** — 修改前自动备份原文件（`.bak` 后缀）
4. **破坏性操作二次确认** — 删除数据、批量修改等需要输入 `yes` 确认
5. **操作日志** — 所有工具调用记录到 SQLite，可审计

## 新增文件清单

### 游戏服务器端

| 文件 | 说明 |
|------|------|
| `src/network/agent_server.rs` | Unix Socket 监听 + JSON-RPC 处理 |
| `src/game/agent_api.rs` | Agent API 实现，桥接各子系统 |

### Agent 独立进程

| 文件 | 说明 |
|------|------|
| `devi-agent/Cargo.toml` | 独立 crate |
| `devi-agent/src/main.rs` | 入口 |
| `devi-agent/src/repl.rs` | 终端 REPL |
| `devi-agent/src/llm/mod.rs` | LLM trait 抽象 |
| `devi-agent/src/llm/openai.rs` | OpenAI 兼容客户端 |
| `devi-agent/src/llm/prompt.rs` | System prompt + 工具定义 |
| `devi-agent/src/tools/mod.rs` | 工具注册 + 执行调度 |
| `devi-agent/src/tools/config.rs` | 配置读写工具 |
| `devi-agent/src/tools/database.rs` | YAML 数据库编辑工具 |
| `devi-agent/src/tools/player.rs` | 玩家查询/操作工具 |
| `devi-agent/src/tools/script.rs` | 脚本创建/修改工具 |
| `devi-agent/src/tools/server.rs` | 服务器状态/控制工具 |
| `devi-agent/src/tools/log.rs` | 日志搜索工具 |
| `devi-agent/src/ipc/mod.rs` | Unix Socket 客户端 |
| `devi-agent/src/ipc/protocol.rs` | JSON-RPC 协议定义 |
| `devi-agent/src/knowledge.rs` | 知识索引生成 |
| `devi-agent/src/memory.rs` | SQLite 持久化记忆 |

## 依赖

### 游戏服务器端新增

- `tokio` — 已有，用于 Unix Socket
- `serde_json` — 已有，用于 JSON-RPC

### Agent 独立进程

- `tokio` — 异步运行时
- `serde` / `serde_json` — JSON 序列化
- `reqwest` — HTTP 客户端（调用 LLM API）
- `rusqlite` — SQLite 存储
- `rustyline` — 终端 REPL（历史、补全）
- `toml` — 配置文件
- `tracing` — 日志
