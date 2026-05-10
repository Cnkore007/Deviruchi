# DeviAgent 服务端智能助手实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 创建独立的 DeviAgent 进程，通过 Unix Socket + JSON-RPC 与游戏服务器通信，集成 LLM（OpenAI 兼容格式）提供自然语言管理能力。

**Architecture:** 两个独立组件——游戏服务器端新增 Agent API（Unix Socket 监听器），以及独立的 `devi-agent` 二进制（REPL + LLM + 工具系统）。两者通过 JSON-RPC over Unix Socket 通信。

**Tech Stack:** Rust, tokio, serde_json, reqwest (HTTP), rusqlite, rustyline, parking_lot

---

## 文件结构

### 游戏服务器端（修改现有）

| 文件 | 职责 |
|------|------|
| `src/network/agent_server.rs` | **新建** — Unix Socket 监听器，JSON-RPC 分发 |
| `src/game/agent_api.rs` | **新建** — Agent API 实现，桥接 MapState/Config/Database |
| `src/network/mod.rs` | **修改** — 添加 `pub mod agent_server` |
| `src/core/mod.rs` | **修改** — 在 `run()` 中启动 AgentServer |
| `Cargo.toml` | **修改** — workspace 添加 `devi-agent` 成员 |

### Agent 独立进程（新建 crate）

| 文件 | 职责 |
|------|------|
| `devi-agent/Cargo.toml` | crate 配置和依赖 |
| `devi-agent/src/main.rs` | 入口，初始化 REPL/IPC/LLM |
| `devi-agent/src/repl.rs` | 终端交互（rustyline） |
| `devi-agent/src/llm/mod.rs` | LLM trait 定义 |
| `devi-agent/src/llm/openai.rs` | OpenAI 兼容 API 客户端 |
| `devi-agent/src/llm/prompt.rs` | System prompt + 工具 schema |
| `devi-agent/src/tools/mod.rs` | 工具注册 + 执行调度 |
| `devi-agent/src/tools/config.rs` | 配置读写工具 |
| `devi-agent/src/tools/database.rs` | YAML 数据库编辑工具 |
| `devi-agent/src/tools/player.rs` | 玩家查询/操作工具 |
| `devi-agent/src/tools/script.rs` | 脚本创建/修改工具 |
| `devi-agent/src/tools/server.rs` | 服务器状态工具 |
| `devi-agent/src/tools/log.rs` | 日志搜索工具 |
| `devi-agent/src/ipc/mod.rs` | Unix Socket 客户端 |
| `devi-agent/src/ipc/protocol.rs` | JSON-RPC 协议类型 |
| `devi-agent/src/knowledge.rs` | 知识索引生成 |
| `devi-agent/src/memory.rs` | SQLite 持久化记忆 |

---

## Task 1: IPC 协议类型定义

**Files:**
- Create: `devi-agent/src/ipc/protocol.rs`

- [ ] **Step 1: 创建 devi-agent crate 目录结构**

```bash
mkdir -p devi-agent/src/ipc devi-agent/src/llm devi-agent/src/tools
```

- [ ] **Step 2: 创建 Cargo.toml**

```toml
# devi-agent/Cargo.toml
[package]
name = "devi-agent"
version = "0.1.0"
edition = "2021"
description = "DeviAgent - Deviruchi 服务端智能助手"

[[bin]]
name = "devi-agent"
path = "src/main.rs"

[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json"] }
rusqlite = { version = "0.31", features = ["bundled"] }
rustyline = "14"
toml = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1"
chrono = "0.4"
```

- [ ] **Step 3: 写 IPC 协议类型**

```rust
// devi-agent/src/ipc/protocol.rs
use serde::{Deserialize, Serialize};

/// JSON-RPC 请求（Agent → Server）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// JSON-RPC 响应（Server → Agent）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

/// JSON-RPC 错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

impl RpcResponse {
    pub fn success(id: u64, result: serde_json::Value) -> Self {
        Self { id, result: Some(result), error: None }
    }

    pub fn error(id: u64, code: i32, message: String) -> Self {
        Self { id, result: None, error: Some(RpcError { code, message }) }
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
}
```

- [ ] **Step 4: 创建 lib.rs 和 main.rs 占位**

```rust
// devi-agent/src/main.rs
fn main() {
    println!("DeviAgent v0.1 — Deviruchi 智能助手");
}
```

```rust
// devi-agent/src/lib.rs (不需要，bin crate 直接用 main.rs)
```

- [ ] **Step 5: 添加到 workspace**

修改根目录 `Cargo.toml`，在 `[workspace]` 的 `members` 中添加 `"devi-agent"`。

- [ ] **Step 6: 验证编译**

```bash
cargo check -p devi-agent
```

Expected: 编译通过

- [ ] **Step 7: Commit**

```bash
git add devi-agent/ Cargo.toml
git commit -m "feat(agent): 初始化 devi-agent crate 和 IPC 协议类型"
```

---

## Task 2: 游戏服务器端 Agent API

**Files:**
- Create: `src/network/agent_server.rs`
- Create: `src/game/agent_api.rs`
- Modify: `src/network/mod.rs`
- Modify: `src/core/mod.rs`

- [ ] **Step 1: 创建 AgentApi 实现**

```rust
// src/game/agent_api.rs
use std::sync::Arc;
use serde_json::{json, Value};
use crate::core::config::Config;
use crate::game::map::MapState;

/// Agent API 处理器，桥接游戏状态和 Agent 请求
pub struct AgentApi {
    config_path: String,
    map_state: Arc<MapState>,
    start_time: std::time::Instant,
}

impl AgentApi {
    pub fn new(config_path: String, map_state: Arc<MapState>) -> Self {
        Self {
            config_path,
            map_state,
            start_time: std::time::Instant::now(),
        }
    }

    /// 处理 JSON-RPC 请求，返回结果
    pub fn handle(&self, method: &str, params: &Value) -> Result<Value, String> {
        match method {
            "server.status" => self.server_status(),
            "config.get" => self.config_get(params),
            "config.set" => self.config_set(params),
            "config.reload" => self.config_reload(),
            "player.list" => self.player_list(params),
            "player.info" => self.player_info(params),
            _ => Err(format!("未知方法: {}", method)),
        }
    }

    fn server_status(&self) -> Result<Value, String> {
        let uptime_secs = self.start_time.elapsed().as_secs();
        let player_count = self.map_state.player_count();
        Ok(json!({
            "uptime_seconds": uptime_secs,
            "online_players": player_count,
        }))
    }

    fn config_get(&self, params: &Value) -> Result<Value, String> {
        let section = params.get("section")
            .and_then(|v| v.as_str())
            .ok_or("缺少 section 参数")?;

        let config = Config::load(&self.config_path)
            .map_err(|e| format!("加载配置失败: {}", e))?;

        let value = match section {
            "server" => serde_json::to_value(&config.server),
            "database" => serde_json::to_value(&config.database),
            "network" => serde_json::to_value(&config.network),
            "game" => serde_json::to_value(&config.game),
            "battle" => serde_json::to_value(&config.battle),
            "drop" => serde_json::to_value(&config.drop),
            "exp" => serde_json::to_value(&config.exp),
            "respawn" => serde_json::to_value(&config.respawn),
            "log" | "logging" => serde_json::to_value(&config.logging),
            "skill" => serde_json::to_value(&config.skill),
            "party" => serde_json::to_value(&config.party),
            "storage" => serde_json::to_value(&config.storage),
            "chat" => serde_json::to_value(&config.chat),
            _ => return Err(format!("未知配置节: {}", section)),
        };

        value.map_err(|e| format!("序列化失败: {}", e))
    }

    fn config_set(&self, params: &Value) -> Result<Value, String> {
        let section = params.get("section").and_then(|v| v.as_str()).ok_or("缺少 section")?;
        let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
        let value = params.get("value").ok_or("缺少 value")?;

        // 加载当前配置
        let mut config = Config::load(&self.config_path)
            .map_err(|e| format!("加载配置失败: {}", e))?;

        // 备份原文件
        let backup_path = format!("{}.bak", self.config_path);
        let _ = std::fs::copy(&self.config_path, &backup_path);

        // 修改指定字段
        self.set_config_field(&mut config, section, key, value)?;

        // 保存
        config.save(&self.config_path)
            .map_err(|e| format!("保存配置失败: {}", e))?;

        Ok(json!({"success": true, "message": format!("{}.{} 已更新", section, key)}))
    }

    fn set_config_field(&self, config: &mut Config, section: &str, key: &str, value: &Value) -> Result<(), String> {
        macro_rules! set_field {
            ($config_section:expr) => {
                serde_json::from_value::<serde_json::Value>(
                    serde_json::to_value(&*$config_section).unwrap()
                )
                .and_then(|mut map| {
                    if let Some(obj) = map.as_object_mut() {
                        obj.insert(key.to_string(), value.clone());
                    }
                    serde_json::from_value(map)
                })
                .map(|v| { *$config_section = v; })
                .map_err(|e| format!("更新失败: {}", e))
            };
        }

        match section {
            "battle" => set_field!(config.battle),
            "game" => set_field!(config.game),
            "exp" => set_field!(config.exp),
            "drop" => set_field!(config.drop),
            "network" => set_field!(config.network),
            "server" => set_field!(config.server),
            _ => Err(format!("暂不支持修改 {} 节", section)),
        }
    }

    fn config_reload(&self) -> Result<Value, String> {
        // 热重载需要 HotReloadConfig 支持，这里返回提示
        Ok(json!({"success": true, "message": "配置已保存，重启服务器后生效"}))
    }

    fn player_list(&self, params: &Value) -> Result<Value, String> {
        let map_filter = params.get("map").and_then(|v| v.as_str());

        let players = if let Some(map_name) = map_filter {
            self.map_state.get_players_on_map(map_name)
        } else {
            self.map_state.get_all_players()
        };

        let player_list: Vec<Value> = players.iter().map(|p| {
            let pos = p.pos.read();
            let combat = p.combat.read();
            let level = p.level.read();
            json!({
                "name": p.name,
                "map": p.map_name,
                "x": pos.x,
                "y": pos.y,
                "hp": combat.hp,
                "max_hp": combat.max_hp,
                "sp": combat.sp,
                "max_sp": combat.max_sp,
                "base_level": level.base_level,
                "job_level": level.job_level,
            })
        }).collect();

        Ok(json!({
            "count": player_list.len(),
            "players": player_list,
        }))
    }

    fn player_info(&self, params: &Value) -> Result<Value, String> {
        let name = params.get("name").and_then(|v| v.as_str()).ok_or("缺少 name 参数")?;

        let player = self.map_state.find_player_by_name(name)
            .ok_or(format!("未找到玩家: {}", name))?;

        let pos = player.pos.read();
        let combat = player.combat.read();
        let level = player.level.read();
        let attrs = player.attrs.read();
        let economy = player.economy.read();

        Ok(json!({
            "name": player.name,
            "char_id": player.char_id,
            "account_id": player.account_id,
            "map": player.map_name,
            "x": pos.x, "y": pos.y,
            "hp": combat.hp, "max_hp": combat.max_hp,
            "sp": combat.sp, "max_sp": combat.max_sp,
            "base_level": level.base_level,
            "job_level": level.job_level,
            "base_exp": level.base_exp,
            "job_exp": level.job_exp,
            "status_point": level.status_point,
            "skill_point": level.skill_point,
            "str": attrs.str, "agi": attrs.agi, "vit": attrs.vit,
            "int": attrs.int, "dex": attrs.dex, "luk": attrs.luk,
            "zeny": economy.zeny,
            "job": economy.job,
        }))
    }
}
```

- [ ] **Step 2: 创建 AgentServer（Unix Socket 监听器）**

```rust
// src/network/agent_server.rs
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use serde_json::Value;
use tracing::{info, error};
use crate::game::agent_api::AgentApi;

pub struct AgentServer {
    socket_path: String,
    api: Arc<AgentApi>,
}

impl AgentServer {
    pub fn new(socket_path: String, api: Arc<AgentApi>) -> Self {
        Self { socket_path, api }
    }

    pub async fn listen(&self) -> anyhow::Result<()> {
        // 清理旧 socket 文件
        let _ = std::fs::remove_file(&self.socket_path);

        let listener = UnixListener::bind(&self.socket_path)?;
        info!("Agent API 监听: {}", self.socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let api = self.api.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, api).await {
                            error!("Agent 连接错误: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Agent accept 错误: {}", e);
                }
            }
        }
    }

    async fn handle_connection(
        stream: tokio::net::UnixStream,
        api: Arc<AgentApi>,
    ) -> anyhow::Result<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break; // 连接关闭
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // 解析 JSON-RPC 请求
            let response = match serde_json::from_str::<Value>(trimmed) {
                Ok(req) => {
                    let id = req.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
                    let params = req.get("params").cloned().unwrap_or(Value::Null);

                    match api.handle(method, &params) {
                        Ok(result) => serde_json::json!({
                            "id": id,
                            "result": result,
                        }),
                        Err(msg) => serde_json::json!({
                            "id": id,
                            "error": {"code": -1, "message": msg},
                        }),
                    }
                }
                Err(e) => serde_json::json!({
                    "id": 0,
                    "error": {"code": -32700, "message": format!("JSON 解析错误: {}", e)},
                }),
            };

            let mut response_str = serde_json::to_string(&response)?;
            response_str.push('\n');
            writer.write_all(response_str.as_bytes()).await?;
        }

        Ok(())
    }
}
```

- [ ] **Step 3: 注册模块导出**

修改 `src/network/mod.rs`，添加：
```rust
pub mod agent_server;
pub use agent_server::AgentServer;
```

修改 `src/game/mod.rs`，添加：
```rust
pub mod agent_api;
pub use agent_api::AgentApi;
```

- [ ] **Step 4: 在 Core::run() 中启动 AgentServer**

在 `src/core/mod.rs` 的 `run()` 方法中，在服务器 spawn 之前添加：

```rust
// 启动 Agent API
let agent_api = Arc::new(AgentApi::new(
    self.cli.config.clone(),
    self.map_state.clone(),
));
let agent_socket = "/tmp/deviruchi.sock".to_string();
let agent_server = AgentServer::new(agent_socket, agent_api);
handles.push(tokio::spawn(async move {
    if let Err(e) = agent_server.listen().await {
        tracing::error!("Agent API 错误: {}", e);
    }
}));
```

- [ ] **Step 5: 验证编译**

```bash
cargo check -p deviruchi
```

Expected: 编译通过

- [ ] **Step 6: Commit**

```bash
git add src/network/agent_server.rs src/game/agent_api.rs src/network/mod.rs src/game/mod.rs src/core/mod.rs
git commit -m "feat(agent): 添加游戏服务器端 Agent API（Unix Socket + JSON-RPC）"
```

---

## Task 3: Agent IPC 客户端

**Files:**
- Create: `devi-agent/src/ipc/mod.rs`

- [ ] **Step 1: 实现 IPC 客户端**

```rust
// devi-agent/src/ipc/mod.rs
pub mod protocol;

use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use protocol::{RpcRequest, RpcResponse};
use anyhow::{Result, anyhow};

pub struct IpcClient {
    socket_path: PathBuf,
    stream: Mutex<Option<(BufReader<tokio::io::ReadHalf<UnixStream>>, tokio::io::WriteHalf<UnixStream>)>>,
    next_id: Mutex<u64>,
}

impl IpcClient {
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
            stream: Mutex::new(None),
            next_id: Mutex::new(1),
        }
    }

    /// 连接到游戏服务器
    pub async fn connect(&self) -> Result<()> {
        let stream = UnixStream::connect(&self.socket_path).await
            .map_err(|e| anyhow!("无法连接到游戏服务器 {}: {}", self.socket_path.display(), e))?;
        let (reader, writer) = stream.into_split();
        let mut guard = self.stream.lock().await;
        *guard = Some((BufReader::new(reader), writer));
        Ok(())
    }

    /// 检查是否已连接
    pub async fn is_connected(&self) -> bool {
        self.stream.lock().await.is_some()
    }

    /// 发送 RPC 请求并等待响应
    pub async fn call(&self, method: &str, params: serde_json::Value) -> Result<RpcResponse> {
        let mut guard = self.stream.lock().await;
        let (reader, writer) = guard.as_mut()
            .ok_or(anyhow!("未连接到游戏服务器"))?;

        // 分配请求 ID
        let mut id_guard = self.next_id.lock().await;
        let id = *id_guard;
        *id_guard += 1;
        drop(id_guard);

        // 发送请求
        let request = RpcRequest { id, method: method.to_string(), params };
        let mut request_str = serde_json::to_string(&request)?;
        request_str.push('\n');
        writer.write_all(request_str.as_bytes()).await?;

        // 读取响应
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        let response: RpcResponse = serde_json::from_str(response_line.trim())?;
        Ok(response)
    }

    /// 断开连接
    pub async fn disconnect(&self) {
        let mut guard = self.stream.lock().await;
        *guard = None;
    }
}
```

- [ ] **Step 2: 验证编译**

```bash
cargo check -p devi-agent
```

Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add devi-agent/src/ipc/
git commit -m "feat(agent): 实现 IPC 客户端（Unix Socket + JSON-RPC）"
```

---

## Task 4: LLM 客户端（OpenAI 兼容）

**Files:**
- Create: `devi-agent/src/llm/mod.rs`
- Create: `devi-agent/src/llm/openai.rs`
- Create: `devi-agent/src/llm/prompt.rs`

- [ ] **Step 1: 定义 LLM trait**

```rust
// devi-agent/src/llm/mod.rs
pub mod openai;
pub mod prompt;

use serde::{Deserialize, Serialize};
use anyhow::Result;

/// LLM 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String, // JSON 字符串
}

/// LLM 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// LLM 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// LLM 响应
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

/// LLM 客户端 trait
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, messages: &[ChatMessage], tools: &[ToolDefinition]) -> Result<LlmResponse>;
}
```

- [ ] **Step 2: 添加 async-trait 依赖**

在 `devi-agent/Cargo.toml` 的 `[dependencies]` 中添加：
```toml
async-trait = "0.1"
```

- [ ] **Step 3: 实现 OpenAI 兼容客户端**

```rust
// devi-agent/src/llm/openai.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use super::{ChatMessage, ToolDefinition, LlmResponse, ToolCall, FunctionCall, LlmClient};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4".to_string(),
            max_tokens: 4096,
        }
    }
}

pub struct OpenAiClient {
    config: LlmConfig,
    http: Client,
}

impl OpenAiClient {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            http: Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl LlmClient for OpenAiClient {
    async fn chat(&self, messages: &[ChatMessage], tools: &[ToolDefinition]) -> Result<LlmResponse> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "max_tokens": self.config.max_tokens,
        });

        if !tools.is_empty() {
            body["tools"] = serde_json::to_value(tools)?;
        }

        let resp = self.http.post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("LLM API 错误 {}: {}", status, text));
        }

        let data: serde_json::Value = resp.json().await?;

        let choice = data["choices"][0].clone();
        let message = &choice["message"];

        let content = message["content"].as_str().map(|s| s.to_string());

        let tool_calls: Vec<ToolCall> = if let Some(calls) = message["tool_calls"].as_array() {
            calls.iter().filter_map(|tc| {
                Some(ToolCall {
                    id: tc["id"].as_str()?.to_string(),
                    function: FunctionCall {
                        name: tc["function"]["name"].as_str()?.to_string(),
                        arguments: tc["function"]["arguments"].as_str()?.to_string(),
                    },
                })
            }).collect()
        } else {
            vec![]
        };

        Ok(LlmResponse { content, tool_calls })
    }
}
```

- [ ] **Step 4: 实现 System Prompt 和工具定义**

```rust
// devi-agent/src/llm/prompt.rs
use super::ToolDefinition;

pub fn system_prompt() -> String {
    r#"你是 DeviAgent，Deviruchi（Ragnarok Online 服务器模拟器）的智能助手。

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
- 用中文回复"#.to_string()
}

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: crate::llm::FunctionDefinition {
                name: "server_status".to_string(),
                description: "查看服务器运行状态（运行时间、在线人数、内存等）".to_string(),
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: crate::llm::FunctionDefinition {
                name: "config_get".to_string(),
                description: "读取服务器配置的指定节".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "section": {
                            "type": "string",
                            "description": "配置节名称，如 battle, game, exp, drop, network 等"
                        }
                    },
                    "required": ["section"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: crate::llm::FunctionDefinition {
                name: "config_set".to_string(),
                description: "修改服务器配置并保存".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "section": {"type": "string", "description": "配置节名称"},
                        "key": {"type": "string", "description": "配置键名"},
                        "value": {"description": "新值"}
                    },
                    "required": ["section", "key", "value"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: crate::llm::FunctionDefinition {
                name: "player_list".to_string(),
                description: "列出当前在线玩家及其位置".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "map": {"type": "string", "description": "可选，按地图名过滤"}
                    }
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: crate::llm::FunctionDefinition {
                name: "player_info".to_string(),
                description: "查看指定玩家的详细信息（等级、属性、装备等）".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "玩家名称"}
                    },
                    "required": ["name"]
                }),
            },
        },
    ]
}
```

- [ ] **Step 5: 验证编译**

```bash
cargo check -p devi-agent
```

Expected: 编译通过

- [ ] **Step 6: Commit**

```bash
git add devi-agent/src/llm/
git commit -m "feat(agent): 实现 LLM 客户端（OpenAI 兼容）和工具定义"
```

---

## Task 5: 工具执行系统

**Files:**
- Create: `devi-agent/src/tools/mod.rs`
- Create: `devi-agent/src/tools/config.rs`
- Create: `devi-agent/src/tools/database.rs`
- Create: `devi-agent/src/tools/player.rs`
- Create: `devi-agent/src/tools/script.rs`
- Create: `devi-agent/src/tools/server.rs`
- Create: `devi-agent/src/tools/log.rs`

- [ ] **Step 1: 工具注册和调度**

```rust
// devi-agent/src/tools/mod.rs
pub mod config;
pub mod database;
pub mod player;
pub mod script;
pub mod server;
pub mod log;

use std::sync::Arc;
use anyhow::Result;
use crate::ipc::IpcClient;

/// 工具执行结果
pub struct ToolResult {
    pub success: bool,
    pub output: String,
}

/// 工具 trait
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, args: &serde_json::Value) -> Result<ToolResult>;
}

/// 工具注册表
pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new(ipc: Arc<IpcClient>) -> Self {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(server::ServerTool::new(ipc.clone())),
            Arc::new(config::ConfigTool::new(ipc.clone())),
            Arc::new(player::PlayerTool::new(ipc.clone())),
        ];
        Self { tools }
    }

    pub async fn execute(&self, name: &str, args: &serde_json::Value) -> Result<ToolResult> {
        for tool in &self.tools {
            if tool.name() == name {
                return tool.execute(args).await;
            }
        }
        Ok(ToolResult {
            success: false,
            output: format!("未知工具: {}", name),
        })
    }

    pub fn list_names(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name()).collect()
    }
}
```

- [ ] **Step 2: 实现 server 工具**

```rust
// devi-agent/src/tools/server.rs
use std::sync::Arc;
use anyhow::Result;
use super::{Tool, ToolResult};
use crate::ipc::IpcClient;

pub struct ServerTool {
    ipc: Arc<IpcClient>,
}

impl ServerTool {
    pub fn new(ipc: Arc<IpcClient>) -> Self {
        Self { ipc }
    }
}

#[async_trait::async_trait]
impl Tool for ServerTool {
    fn name(&self) -> &str { "server_status" }

    async fn execute(&self, _args: &serde_json::Value) -> Result<ToolResult> {
        let resp = self.ipc.call("server.status", serde_json::json!({})).await?;

        if let Some(err) = resp.error {
            return Ok(ToolResult { success: false, output: err.message });
        }

        let result = resp.result.unwrap_or_default();
        let uptime = result["uptime_seconds"].as_u64().unwrap_or(0);
        let players = result["online_players"].as_u64().unwrap_or(0);

        let hours = uptime / 3600;
        let mins = (uptime % 3600) / 60;
        let secs = uptime % 60;

        Ok(ToolResult {
            success: true,
            output: format!(
                "服务器状态:\n  运行时间: {}h {}m {}s\n  在线玩家: {}",
                hours, mins, secs, players
            ),
        })
    }
}
```

- [ ] **Step 3: 实现 config 工具**

```rust
// devi-agent/src/tools/config.rs
use std::sync::Arc;
use anyhow::Result;
use super::{Tool, ToolResult};
use crate::ipc::IpcClient;

pub struct ConfigTool {
    ipc: Arc<IpcClient>,
}

impl ConfigTool {
    pub fn new(ipc: Arc<IpcClient>) -> Self {
        Self { ipc }
    }
}

#[async_trait::async_trait]
impl Tool for ConfigTool {
    fn name(&self) -> &str { "config" }

    async fn execute(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("get");

        match action {
            "get" => {
                let section = args.get("section").and_then(|v| v.as_str()).unwrap_or("battle");
                let resp = self.ipc.call("config.get", serde_json::json!({"section": section})).await?;
                if let Some(err) = resp.error {
                    return Ok(ToolResult { success: false, output: err.message });
                }
                let pretty = serde_json::to_string_pretty(&resp.result.unwrap_or_default())?;
                Ok(ToolResult { success: true, output: format!("[{}]\n{}", section, pretty) })
            }
            "set" => {
                let section = args.get("section").and_then(|v| v.as_str()).unwrap_or("");
                let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
                let value = args.get("value").cloned().unwrap_or(serde_json::Value::Null);
                let resp = self.ipc.call("config.set", serde_json::json!({
                    "section": section, "key": key, "value": value
                })).await?;
                if let Some(err) = resp.error {
                    return Ok(ToolResult { success: false, output: err.message });
                }
                Ok(ToolResult { success: true, output: format!("{}.{} 已更新", section, key) })
            }
            _ => Ok(ToolResult { success: false, output: format!("未知操作: {}", action) }),
        }
    }
}
```

- [ ] **Step 4: 实现 player 工具**

```rust
// devi-agent/src/tools/player.rs
use std::sync::Arc;
use anyhow::Result;
use super::{Tool, ToolResult};
use crate::ipc::IpcClient;

pub struct PlayerTool {
    ipc: Arc<IpcClient>,
}

impl PlayerTool {
    pub fn new(ipc: Arc<IpcClient>) -> Self {
        Self { ipc }
    }
}

#[async_trait::async_trait]
impl Tool for PlayerTool {
    fn name(&self) -> &str { "player" }

    async fn execute(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("list");

        match action {
            "list" => {
                let map = args.get("map").and_then(|v| v.as_str());
                let mut params = serde_json::json!({});
                if let Some(m) = map {
                    params["map"] = serde_json::json!(m);
                }
                let resp = self.ipc.call("player.list", params).await?;
                if let Some(err) = resp.error {
                    return Ok(ToolResult { success: false, output: err.message });
                }
                let result = resp.result.unwrap_or_default();
                let count = result["count"].as_u64().unwrap_or(0);
                let mut output = format!("当前在线 {} 人:\n", count);
                if let Some(players) = result["players"].as_array() {
                    for p in players {
                        output.push_str(&format!(
                            "  {} (Lv.{}) @ {} ({},{}) HP:{}/{}\n",
                            p["name"].as_str().unwrap_or("?"),
                            p["base_level"].as_u64().unwrap_or(0),
                            p["map"].as_str().unwrap_or("?"),
                            p["x"].as_u64().unwrap_or(0),
                            p["y"].as_u64().unwrap_or(0),
                            p["hp"].as_u64().unwrap_or(0),
                            p["max_hp"].as_u64().unwrap_or(0),
                        ));
                    }
                }
                Ok(ToolResult { success: true, output })
            }
            "info" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let resp = self.ipc.call("player.info", serde_json::json!({"name": name})).await?;
                if let Some(err) = resp.error {
                    return Ok(ToolResult { success: false, output: err.message });
                }
                let p = resp.result.unwrap_or_default();
                let output = format!(
                    "{} (ID:{})\n  等级: Base {} / Job {}\n  HP: {}/{} SP: {}/{}\n  位置: {} ({},{})\n  Zeny: {}\n  属性: STR {} AGI {} VIT {} INT {} DEX {} LUK {}",
                    p["name"].as_str().unwrap_or("?"),
                    p["char_id"].as_u64().unwrap_or(0),
                    p["base_level"].as_u64().unwrap_or(0),
                    p["job_level"].as_u64().unwrap_or(0),
                    p["hp"].as_u64().unwrap_or(0), p["max_hp"].as_u64().unwrap_or(0),
                    p["sp"].as_u64().unwrap_or(0), p["max_sp"].as_u64().unwrap_or(0),
                    p["map"].as_str().unwrap_or("?"),
                    p["x"].as_u64().unwrap_or(0), p["y"].as_u64().unwrap_or(0),
                    p["zeny"].as_u64().unwrap_or(0),
                    p["str"].as_u64().unwrap_or(0), p["agi"].as_u64().unwrap_or(0),
                    p["vit"].as_u64().unwrap_or(0), p["int"].as_u64().unwrap_or(0),
                    p["dex"].as_u64().unwrap_or(0), p["luk"].as_u64().unwrap_or(0),
                );
                Ok(ToolResult { success: true, output })
            }
            _ => Ok(ToolResult { success: false, output: format!("未知操作: {}", action) }),
        }
    }
}
```

- [ ] **Step 5: 创建空的 database/script/log 工具占位**

```rust
// devi-agent/src/tools/database.rs
// TODO: 数据库编辑工具（Task 8 实现）

// devi-agent/src/tools/script.rs
// TODO: 脚本编写工具（Task 8 实现）

// devi-agent/src/tools/log.rs
// TODO: 日志搜索工具（Task 8 实现）
```

- [ ] **Step 6: 验证编译**

```bash
cargo check -p devi-agent
```

Expected: 编译通过

- [ ] **Step 7: Commit**

```bash
git add devi-agent/src/tools/
git commit -m "feat(agent): 实现工具执行系统（server/config/player 工具）"
```

---

## Task 6: REPL 和 Agent 主循环

**Files:**
- Create: `devi-agent/src/main.rs`
- Create: `devi-agent/src/repl.rs`

- [ ] **Step 1: 实现 REPL**

```rust
// devi-agent/src/repl.rs
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result as RustyResult};
use tracing::info;

pub struct Repl {
    editor: DefaultEditor,
    prompt: String,
}

impl Repl {
    pub fn new() -> RustyResult<Self> {
        let mut editor = DefaultEditor::new()?;
        let history_path = dirs().map(|d| d.join("history.txt"));
        if let Some(ref path) = history_path {
            let _ = editor.load_history(path);
        }
        Ok(Self {
            editor,
            prompt: "DeviAgent> ".to_string(),
        })
    }

    /// 读取一行用户输入
    pub fn read_line(&mut self) -> Result<Option<String>, ReadlineError> {
        match self.editor.readline(&self.prompt) {
            Ok(line) => {
                self.editor.add_history_entry(&line)?;
                Ok(Some(line))
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// 保存历史记录
    pub fn save_history(&self) {
        if let Some(path) = dirs().map(|d| d.join("history.txt")) {
            let _ = self.editor.save_history(&path);
        }
    }
}

fn dirs() -> Option<std::path::PathBuf> {
    std::env::var("HOME").ok().map(|h| {
        let mut path = std::path::PathBuf::from(h);
        path.push(".devi-agent");
        std::fs::create_dir_all(&path).ok();
        path
    })
}
```

- [ ] **Step 2: 实现 Agent 主循环**

```rust
// devi-agent/src/main.rs
mod ipc;
mod llm;
mod tools;
mod repl;

use std::sync::Arc;
use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  DeviAgent v0.1 — Deviruchi 智能助手                     ║");
    println!("║  输入 /help 查看命令，直接输入自然语言对话                 ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    // 初始化 IPC 客户端
    let ipc = Arc::new(ipc::IpcClient::new("/tmp/deviruchi.sock"));

    // 尝试连接游戏服务器
    match ipc.connect().await {
        Ok(_) => println!("✓ 已连接到游戏服务器"),
        Err(e) => println!("⚠ 无法连接到游戏服务器: {}（稍后可使用 /connect 重连）", e),
    }

    // 初始化工具注册表
    let tools = Arc::new(tools::ToolRegistry::new(ipc.clone()));

    // 初始化 LLM 客户端
    let llm_config = load_llm_config();
    let llm_client: Arc<dyn llm::LlmClient> = Arc::new(llm::openai::OpenAiClient::new(llm_config));

    // 初始化 REPL
    let mut repl = repl::Repl::new()?;

    // 加载对话历史
    let mut messages: Vec<llm::ChatMessage> = vec![
        llm::ChatMessage {
            role: "system".to_string(),
            content: Some(llm::prompt::system_prompt()),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    println!("\n开始对话（输入 /quit 退出）:\n");

    loop {
        let input = match repl.read_line()? {
            Some(line) => line,
            None => break, // Ctrl+C / Ctrl+D
        };

        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }

        // 处理斜杠命令
        if trimmed.starts_with('/') {
            match handle_slash_command(trimmed, &ipc, &tools).await {
                SlashResult::Quit => break,
                SlashResult::Output(msg) => println!("{}", msg),
                SlashResult::Continue => continue,
            }
            continue;
        }

        // 发送给 LLM
        messages.push(llm::ChatMessage {
            role: "user".to_string(),
            content: Some(trimmed.to_string()),
            tool_calls: None,
            tool_call_id: None,
        });

        // LLM 对话循环（处理工具调用）
        loop {
            let tool_defs = llm::prompt::tool_definitions();
            let response = match llm_client.chat(&messages, &tool_defs).await {
                Ok(r) => r,
                Err(e) => {
                    println!("LLM 错误: {}", e);
                    break;
                }
            };

            // 如果有文本回复，显示
            if let Some(ref content) = response.content {
                if !content.is_empty() {
                    println!("\n{}\n", content);
                }
            }

            // 如果没有工具调用，结束循环
            if response.tool_calls.is_empty() {
                // 记录 assistant 消息
                messages.push(llm::ChatMessage {
                    role: "assistant".to_string(),
                    content: response.content,
                    tool_calls: None,
                    tool_call_id: None,
                });
                break;
            }

            // 记录 assistant 消息（带 tool_calls）
            messages.push(llm::ChatMessage {
                role: "assistant".to_string(),
                content: response.content.clone(),
                tool_calls: Some(response.tool_calls.clone()),
                tool_call_id: None,
            });

            // 执行工具调用
            for tc in &response.tool_calls {
                let args: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(serde_json::Value::Null);

                println!("  ⚙ 调用工具: {}({})", tc.function.name, tc.function.arguments);

                let result = tools.execute(&tc.function.name, &args).await
                    .unwrap_or(tools::ToolResult { success: false, output: "工具执行错误".to_string() });

                println!("  → {}", result.output);

                // 记录工具结果
                messages.push(llm::ChatMessage {
                    role: "tool".to_string(),
                    content: Some(result.output),
                    tool_calls: None,
                    tool_call_id: Some(tc.id.clone()),
                });
            }
        }
    }

    repl.save_history();
    println!("再见！");
    Ok(())
}

enum SlashResult {
    Quit,
    Output(String),
    Continue,
}

async fn handle_slash_command(
    input: &str,
    ipc: &Arc<ipc::IpcClient>,
    tools: &Arc<tools::ToolRegistry>,
) -> SlashResult {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    match parts[0] {
        "/quit" | "/exit" | "/q" => SlashResult::Quit,
        "/help" => SlashResult::Output(
            "可用命令:\n  /help     — 显示帮助\n  /connect  — 连接游戏服务器\n  /status   — 服务器状态\n  /players  — 在线玩家\n  /quit     — 退出\n\n直接输入自然语言与 AI 对话".to_string()
        ),
        "/connect" => {
            match ipc.connect().await {
                Ok(_) => SlashResult::Output("✓ 已连接到游戏服务器".to_string()),
                Err(e) => SlashResult::Output(format!("✗ 连接失败: {}", e)),
            }
        }
        "/status" => {
            let result = tools.execute("server_status", &serde_json::json!({})).await;
            match result {
                Ok(r) => SlashResult::Output(r.output),
                Err(e) => SlashResult::Output(format!("错误: {}", e)),
            }
        }
        "/players" => {
            let result = tools.execute("player", &serde_json::json!({"action": "list"})).await;
            match result {
                Ok(r) => SlashResult::Output(r.output),
                Err(e) => SlashResult::Output(format!("错误: {}", e)),
            }
        }
        _ => SlashResult::Output(format!("未知命令: {}（输入 /help 查看帮助）", parts[0])),
    }
}

fn load_llm_config() -> llm::openai::LlmConfig {
    let config_path = dirs_config_path();
    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str::<llm::openai::LlmConfig>(&content) {
                return config;
            }
        }
    }
    // 返回默认配置
    llm::openai::LlmConfig::default()
}

fn dirs_config_path() -> std::path::PathBuf {
    let mut path = std::env::var("HOME").map(std::path::PathBuf::from).unwrap_or_default();
    path.push(".devi-agent");
    path.push("config.toml");
    path
}
```

- [ ] **Step 3: 验证编译**

```bash
cargo check -p devi-agent
```

Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add devi-agent/src/main.rs devi-agent/src/repl.rs
git commit -m "feat(agent): 实现 REPL 和 Agent 主循环"
```

---

## Task 7: 知识索引生成

**Files:**
- Create: `devi-agent/src/knowledge.rs`

- [ ] **Step 1: 实现知识索引生成器**

```rust
// devi-agent/src/knowledge.rs
use std::path::{Path, PathBuf};
use anyhow::Result;
use tracing::info;

/// 知识索引生成器
pub struct KnowledgeIndex {
    output_dir: PathBuf,
    source_dir: PathBuf,
}

impl KnowledgeIndex {
    pub fn new(output_dir: PathBuf, source_dir: PathBuf) -> Self {
        Self { output_dir, source_dir }
    }

    /// 生成知识索引
    pub fn generate(&self) -> Result<()> {
        std::fs::create_dir_all(&self.output_dir)?;
        std::fs::create_dir_all(self.output_dir.join("schemas"))?;

        self.generate_codebase_map()?;
        self.generate_config_schema()?;
        self.generate_script_reference()?;

        info!("知识索引已生成到: {}", self.output_dir.display());
        Ok(())
    }

    /// 检查是否需要更新
    pub fn needs_update(&self) -> bool {
        let index_file = self.output_dir.join("codebase.md");
        if !index_file.exists() {
            return true;
        }
        // 简单检查：如果 src/ 中有任何文件比索引新，则需要更新
        let index_time = std::fs::metadata(&index_file)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        self.source_dir.join("lib.rs").metadata()
            .and_then(|m| m.modified())
            .map(|t| t > index_time)
            .unwrap_or(true)
    }

    fn generate_codebase_map(&self) -> Result<()> {
        let mut content = String::from("# Deviruchi 代码库结构\n\n");
        content.push_str("## 服务器架构\n\n");
        content.push_str("- `src/core/` — 核心系统（配置、日志、定时器）\n");
        content.push_str("- `src/game/` — 游戏逻辑（40 个子模块）\n");
        content.push_str("- `src/network/` — 网络层（TCP + WebSocket）\n");
        content.push_str("- `src/protocol/` — 协议编解码\n");
        content.push_str("- `src/storage/` — 数据库存储\n");
        content.push_str("\n## 关键类型\n\n");
        content.push_str("- `Core` — 服务主结构，拥有所有子系统\n");
        content.push_str("- `MapState` — 玩家状态管理（RwLock<HashMap>）\n");
        content.push_str("- `Config` — TOML 配置（13 个节）\n");
        content.push_str("- `Player` — 玩家数据（6 个分组锁）\n");
        content.push_str("- `MobTemplate` — 怪物模板\n");
        content.push_str("- `Item` — 物品数据\n");

        std::fs::write(self.output_dir.join("codebase.md"), content)?;
        Ok(())
    }

    fn generate_config_schema(&self) -> Result<()> {
        let content = r#"# 配置 Schema (config/server.toml)

## [server]
- name: String — 服务器名称

## [database]
- backend: String — "sqlite" 或 "mysql"
- path: String — 数据库文件路径

## [network]
- login_port: u16 — 登录服务器端口 (默认 6900)
- char_port: u16 — 角色服务器端口 (默认 6000)
- map_port: u16 — 地图服务器端口 (默认 6121)
- modern_port: u16 — WebSocket 端口 (默认 16121)
- max_connections: usize — 最大连接数

## [game]
- max_players: usize — 最大玩家数
- max_level: u16 — 最大等级
- base_level_cap: u16 — 基础等级上限
- job_level_cap: u16 — 职业等级上限
- autosave_interval_seconds: u64 — 自动保存间隔

## [battle]
- base_exp_rate: u32 — 基础经验倍率
- job_exp_rate: u32 — 职业经验倍率
- zeny_rate: u32 — Zeny 倍率
- item_drop_rate: u32 — 物品掉落倍率

## [drop]
- item_drop_rate: u32 — 掉落倍率

## [exp]
- base_exp_rate: u32 — 经验倍率

## [respawn]
- delay: u64 — 重生延迟
"#;

        std::fs::write(self.output_dir.join("schemas").join("server.toml.md"), content)?;
        Ok(())
    }

    fn generate_script_reference(&self) -> Result<()> {
        let content = r#"# NPC 脚本命令参考

## 对话命令
- `mes "文本"` — 显示对话文本
- `next` — 等待玩家点击
- `close` — 关闭对话
- `select "选项1","选项2"` — 显示选择菜单

## 传送命令
- `warp "地图名",x,y` — 传送玩家

## 物品命令
- `getitem 物品ID,数量` — 给予物品
- `delitem 物品ID,数量` — 删除物品
- `countitem 物品ID,"结果变量"` — 计算物品数量

## 流程控制
- `goto "标签"` — 跳转到标签
- `goto_if "变量",值,"标签"` — 条件跳转
- `set "变量",值` — 设置变量

## 状态命令
- `heal HP,SP` — 恢复 HP/SP
- `announce "消息",flag` — 发送公告
"#;

        std::fs::write(self.output_dir.join("schemas").join("script.md"), content)?;
        Ok(())
    }
}
```

- [ ] **Step 2: 在 main.rs 中集成知识索引**

在 `main()` 中 REPL 之前添加：

```rust
// 知识索引
let source_dir = std::env::current_dir().unwrap_or_default();
let knowledge_dir = dirs().map(|d| d.join("knowledge")).unwrap_or_default();
let knowledge = knowledge::KnowledgeIndex::new(knowledge_dir, source_dir);
if knowledge.needs_update() {
    println!("正在生成知识索引...");
    if let Err(e) = knowledge.generate() {
        println!("⚠ 知识索引生成失败: {}", e);
    } else {
        println!("✓ 知识索引已更新");
    }
}
```

- [ ] **Step 3: 验证编译**

```bash
cargo check -p devi-agent
```

Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add devi-agent/src/knowledge.rs devi-agent/src/main.rs
git commit -m "feat(agent): 实现知识索引自动生成"
```

---

## Task 8: SQLite 持久化记忆

**Files:**
- Create: `devi-agent/src/memory.rs`

- [ ] **Step 1: 实现记忆存储**

```rust
// devi-agent/src/memory.rs
use std::path::PathBuf;
use rusqlite::{Connection, params};
use anyhow::Result;
use chrono::Utc;

pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    pub fn new(path: &PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;

        // 创建表
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tool_calls (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                params TEXT NOT NULL,
                result TEXT,
                success INTEGER
            );
            CREATE TABLE IF NOT EXISTS learnings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                category TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL
            );
        ")?;

        Ok(Self { conn })
    }

    /// 记录对话
    pub fn save_conversation(&self, role: &str, content: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO conversations (timestamp, role, content) VALUES (?1, ?2, ?3)",
            params![Utc::now().to_rfc3339(), role, content],
        )?;
        Ok(())
    }

    /// 记录工具调用
    pub fn save_tool_call(&self, tool_name: &str, params: &str, result: &str, success: bool) -> Result<()> {
        self.conn.execute(
            "INSERT INTO tool_calls (timestamp, tool_name, params, result, success) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![Utc::now().to_rfc3339(), tool_name, params, result, success as i32],
        )?;
        Ok(())
    }

    /// 获取最近对话历史
    pub fn recent_conversations(&self, limit: usize) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT timestamp, role, content FROM conversations ORDER BY id DESC LIMIT ?1"
        )?;
        let rows: Vec<(String, String, String)> = stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// 搜索历史记录
    pub fn search(&self, keyword: &str) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT timestamp, role, content FROM conversations WHERE content LIKE ?1 ORDER BY id DESC LIMIT 20"
        )?;
        let pattern = format!("%{}%", keyword);
        let rows: Vec<(String, String, String)> = stmt.query_map(params![pattern], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// 记录学习到的模式
    pub fn save_learning(&self, category: &str, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO learnings (timestamp, category, key, value) VALUES (?1, ?2, ?3, ?4)",
            params![Utc::now().to_rfc3339(), category, key, value],
        )?;
        Ok(())
    }
}
```

- [ ] **Step 2: 验证编译**

```bash
cargo check -p devi-agent
```

Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add devi-agent/src/memory.rs
git commit -m "feat(agent): 实现 SQLite 持久化记忆存储"
```

---

## Task 9: 集成测试 — 端到端验证

**Files:**
- Test: `devi-agent/tests/integration.rs`

- [ ] **Step 1: 验证整体编译**

```bash
cargo build -p devi-agent
```

Expected: 编译成功，生成 `target/debug/devi-agent` 二进制

- [ ] **Step 2: 验证游戏服务器编译**

```bash
cargo build -p deviruchi
```

Expected: 编译成功

- [ ] **Step 3: 运行游戏服务器并测试 Agent 连接**

```bash
# 终端 1: 启动游戏服务器
cargo run -p deviruchi -- --mode all &

# 等待服务器启动
sleep 2

# 终端 2: 启动 Agent
cargo run -p devi-agent

# 在 Agent 中测试:
# /connect
# /status
# /players
# 帮我看看服务器状态
```

Expected: Agent 能连接到服务器，执行命令并返回结果

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(agent): DeviAgent 完整功能实现 — REPL + LLM + IPC + 知识索引 + 持久记忆"
```

---

## 后续扩展（Task 10+）

以下功能在当前计划之后作为后续迭代：

- **Task 10**: 数据库编辑工具（mob/item/skill/drop 的查询和修改）
- **Task 11**: 脚本编写工具（NPC 脚本创建/验证）
- **Task 12**: 日志搜索工具（按关键词/类别搜索服务器日志）
- **Task 13**: 玩家操作工具（warp/kick 等需要认证的操作）
- **Task 14**: 知识索引自动更新（文件变化检测）
- **Task 15**: Agent 记忆集成到 LLM 上下文
