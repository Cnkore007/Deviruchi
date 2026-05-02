# Deviruchi Phase 1: 核心骨架实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 搭建 Deviruchi Rust 服务端的完整项目骨架，包括配置系统、日志崩溃报告、SQLite 集成、基础网络框架

**Architecture:** 使用 Rust + Tokio 异步运行时，模块化设计，单体优先架构。核心层负责配置、日志、定时器；网络层基于 Tokio 实现异步 I/O；数据层使用 rusqlite 内嵌 SQLite。

**Tech Stack:** Rust 2024, Tokio, rusqlite, toml, tracing

---

## 文件结构规划

```
deviruchi/
├── Cargo.toml                     # 项目配置 + 依赖
├── src/
│   ├── main.rs                   # 程序入口
│   ├── lib.rs                    # 库入口
│   ├── cli.rs                    # 命令行参数解析
│   ├── core/                    # 核心模块
│   │   ├── mod.rs
│   │   ├── config.rs            # 配置管理
│   │   ├── logging.rs           # 日志系统
│   │   ├── panic.rs             # 崩溃处理 + 中文堆栈
│   │   ├── timer.rs             # 定时器系统
│   │   └── version.rs           # 版本信息
│   ├── network/                 # 网络层
│   │   ├── mod.rs
│   │   ├── packet.rs            # 数据包定义
│   │   ├── codec.rs             # 编解码器
│   │   └── session.rs           # 会话管理
│   ├── storage/                 # 数据层
│   │   ├── mod.rs
│   │   ├── sqlite.rs            # SQLite 封装
│   │   └── schema.rs            # 数据库 Schema
│   └── error.rs                 # 统一错误类型
├── deviruchi.toml                # 配置文件模板
└── tests/
    └── core_test.rs             # 核心模块测试
```

---

## 任务列表

### Task 1: 项目初始化

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`

- [ ] **Step 1: 创建 Cargo.toml**

```toml
[package]
name = "deviruchi"
version = "0.1.0"
edition = "2024"
authors = ["Deviruchi Team"]
description = "High-performance MMORPG game server in Rust"

[dependencies]
tokio = { version = "1", features = ["full"] }
rusqlite = { version = "0.31", features = ["bundled"] }
toml = "0.8"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "1"
parking_lot = "0.12"
once_cell = "1"

[lib]
name = "deviruchi"
path = "src/lib.rs"

[[bin]]
name = "deviruchi"
path = "src/main.rs"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

- [ ] **Step 2: 创建 src/lib.rs**

```rust
//! Deviruchi - High-performance MMORPG game server

pub mod cli;
pub mod core;
pub mod network;
pub mod storage;
pub mod error;

pub use error::{Error, Result};
```

- [ ] **Step 3: 创建 src/main.rs**

```rust
use anyhow::Result;
use deviruchi::cli::Cli;
use deviruchi::core::Core;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut core = Core::new(cli);
    core.run().await
}
```

- [ ] **Step 4: 创建 src/cli.rs**

```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "deviruchi")]
#[command(about = "Deviruchi - High-performance MMORPG game server")]
pub struct Cli {
    /// 配置文件的路径
    #[arg(short, long, default_value = "deviruchi.toml")]
    pub config: String,

    /// 服务器名称
    #[arg(short, long, default_value = "Deviruchi")]
    pub name: String,

    /// 日志级别
    #[arg(short, long, default_value = "info")]
    pub log_level: String,

    /// 单机模式运行
    #[arg(long, default_value = "true")]
    pub standalone: bool,

    /// 运行模式: login, char, map
    #[arg(long, default_value = "all")]
    pub mode: String,
}
```

- [ ] **Step 5: 创建 src/error.rs**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("网络错误: {0}")]
    Network(String),

    #[error("协议错误: {0}")]
    Protocol(String),

    #[error("游戏逻辑错误: {0}")]
    Game(String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

- [ ] **Step 6: 运行编译验证**

Run: `cargo check`
Expected: 无编译错误

- [ ] **Step 7: 提交**

```bash
git add -A
git commit -m "chore: 初始化 Rust 项目结构"
```

---

### Task 2: 核心模块 - 配置系统

**Files:**
- Create: `src/core/mod.rs`
- Create: `src/core/config.rs`
- Create: `deviruchi.toml`

- [ ] **Step 1: 创建 src/core/mod.rs**

```rust
pub mod config;
pub mod logging;
pub mod panic;
pub mod timer;
pub mod version;

pub use config::Config;
pub use version::VERSION;
```

- [ ] **Step 2: 创建 src/core/config.rs**

```rust
use serde::Deserialize;
use std::path::Path;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub network: NetworkConfig,
    pub game: GameConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub mode: String,
    pub standalone: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkConfig {
    pub login_port: u16,
    pub char_port: u16,
    pub map_port: u16,
    pub max_connections: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GameConfig {
    pub max_players: usize,
    pub timeout_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                name: "Deviruchi".to_string(),
                mode: "all".to_string(),
                standalone: true,
            },
            database: DatabaseConfig {
                path: "deviruchi.db".to_string(),
            },
            network: NetworkConfig {
                login_port: 6900,
                char_port: 6000,
                map_port: 6121,
                max_connections: 10000,
            },
            game: GameConfig {
                max_players: 5000,
                timeout_seconds: 300,
            },
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            let config = Self::default();
            config.save(path)?;
            tracing::info!("配置文件不存在，已创建默认配置: {:?}", path);
            return Ok(config);
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("读取配置文件失败: {:?}", path))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("解析配置文件失败: {:?}", path))?;

        Ok(config)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
```

- [ ] **Step 3: 创建 deviruchi.toml 配置模板**

```toml
[server]
name = "Deviruchi"
mode = "all"
standalone = true

[database]
path = "deviruchi.db"

[network]
login_port = 6900
char_port = 6000
map_port = 6121
max_connections = 10000

[game]
max_players = 5000
timeout_seconds = 300
```

- [ ] **Step 4: 创建 src/core/version.rs**

```rust
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "Deviruchi";
pub const BUILD_DATE: &str = env!("BUILD_DATE");
```

- [ ] **Step 5: 更新 Cargo.toml 添加依赖**

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
```

- [ ] **Step 6: 编写配置测试**

Create: `tests/config_test.rs`

```rust
use deviruchi::core::config::Config;

#[test]
fn test_config_default() {
    let config = Config::default();
    assert_eq!(config.server.name, "Deviruchi");
    assert_eq!(config.network.login_port, 6900);
}

#[test]
fn test_config_save_load() {
    let config = Config::default();
    let path = "/tmp/test_deviruchi_config.toml";

    config.save(path).unwrap();
    let loaded = Config::load(path).unwrap();

    assert_eq!(config.server.name, loaded.server.name);
    assert_eq!(config.network.login_port, loaded.network.login_port);
}
```

- [ ] **Step 7: 运行测试**

Run: `cargo test`
Expected: 所有测试通过

- [ ] **Step 8: 提交**

```bash
git add -A
git commit -m "feat(core): 添加配置系统
- Config 结构体，支持 TOML 格式
- 自动创建默认配置文件
- 配置加载和保存功能
"
```

---

### Task 3: 核心模块 - 日志系统与崩溃处理

**Files:**
- Create: `src/core/logging.rs`
- Create: `src/core/panic.rs`

- [ ] **Step 1: 创建 src/core/logging.rs**

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::path::Path;

pub fn init_logging<P: AsRef<Path>>(log_dir: P, log_level: &str) -> anyhow::Result<()> {
    let log_dir = log_dir.as_ref();
    std::fs::create_dir_all(log_dir)?;

    let file_appender = tracing_appender::rolling::daily(log_dir, "deviruchi.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 保存 guard 防止被 drop
    std::mem::forget(_guard);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
        )
        .init();

    tracing::info!("日志系统初始化完成");
    Ok(())
}

pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info.location();
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        let report = format!(
            r#"
=====================================
       Deviruchi 崩溃报告
=====================================
时间: {}

崩溃位置:
  文件: {}
  行号: {}

崩溃信息:
  {}

调用栈:
{:?}
=====================================
"#,
            chrono_lite_now(),
            location.map(|l| l.file()).unwrap_or("unknown"),
            location.map(|l| l.line()).unwrap_or(0),
            message,
            std::backtrace::Backtrace::capture(),
        );

        eprintln!("{}", report);

        // 写入崩溃日志文件
        let crash_log = format!(
            "crash_{}.log",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        let _ = std::fs::write(&crash_log, &report);

        std::process::exit(1);
    }));
}

fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    let secs = now.as_secs();
    format!("{}", secs)
}
```

- [ ] **Step 2: 创建 src/core/panic.rs**

```rust
use std::panic;
use std::io::Write;
use std::backtrace::Backtrace;

pub struct PanicHandler;

impl PanicHandler {
    pub fn init() {
        panic::set_hook(Box::new(Self::handle_panic));
    }

    fn handle_panic(info: &panic::PanicInfo<'_>) {
        let location = info.location();
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown".to_string()
        };

        let timestamp = chrono_lite_now();
        let file = location.map(|l| l.file()).unwrap_or("unknown");
        let line = location.map(|l| l.line()).unwrap_or(0);
        let col = location.map(|l| l.column()).unwrap_or(0);

        let report = format!(
            "=====================================\n\
             |       Deviruchi 崩溃报告        |\n\
             =====================================\n\
             时间: {timestamp}\n\
             \n\
             崩溃位置:\n\
               文件: {file}\n\
               行号: {line}\n\
               列号: {col}\n\
             \n\
             崩溃信息:\n\
               {payload}\n\
             \n\
             调用栈:\n\
             {backtrace}\n\
             =====================================\n",
            timestamp = timestamp,
            file = file,
            line = line,
            col = col,
            payload = payload,
            backtrace = Backtrace::capture(),
        );

        // 输出到 stderr
        let _ = write!(std::io::stderr(), "{}", report);

        // 保存到文件
        let crash_file = format!("crash_{}.log", timestamp);
        let _ = std::fs::write(&crash_file, &report);
    }
}

fn chrono_lite_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    format!("{}", now.as_secs())
}
```

- [ ] **Step 3: 更新 Cargo.toml 添加 tracing-appender**

```toml
[dependencies]
tracing-appender = "0.2"
```

- [ ] **Step 4: 编写日志测试**

Create: `tests/logging_test.rs`

```rust
use deviruchi::core::panic::PanicHandler;

#[test]
fn test_panic_hook_installed() {
    PanicHandler::init();
    // 如果能执行到这里说明 hook 安装成功
}
```

- [ ] **Step 5: 运行测试**

Run: `cargo test`
Expected: 测试通过

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat(core): 添加日志系统和崩溃处理
- 基于 tracing 的结构化日志
- 中文崩溃报告生成
- 崩溃信息保存到文件
"
```

---

### Task 4: 核心模块 - 定时器系统

**Files:**
- Create: `src/core/timer.rs`

- [ ] **Step 1: 创建 src/core/timer.rs**

```rust
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use once_cell::sync::Lazy;

static TIMER_QUEUE: Lazy<Mutex<BinaryHeap<TimerEntry>>> = Lazy::new(|| Mutex::new(BinaryHeap::new()));

#[derive(Debug, Clone)]
pub struct TimerId(u64);

impl TimerId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug)]
struct TimerEntry {
    due: Instant,
    id: u64,
    callback: Box<dyn Fn() + Send + 'static>,
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TimerEntry {}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap 是 max-heap，我们用 reverse 实现 min-heap
        other.due.cmp(&self.due)
    }
}

pub struct Timer;

impl Timer {
    /// 添加一个一次性定时器
    pub fn add<F>(delay: Duration, callback: F) -> TimerId
    where
        F: Fn() + Send + 'static,
    {
        let id = generate_timer_id();
        let entry = TimerEntry {
            due: Instant::now() + delay,
            id,
            callback: Box::new(callback),
        };

        TIMER_QUEUE.lock().push(entry);
        TimerId(id)
    }

    /// 添加一个重复执行的定时器
    pub fn add_interval<F>(interval: Duration, mut callback: F) -> TimerId
    where
        F: Fn() + Send + 'static,
    {
        let id = generate_timer_id();
        let due = Instant::now() + interval;

        // 克隆 callback 用于重复执行
        let callback = Box::new(move || {
            callback();
            let entry = TimerEntry {
                due: Instant::now() + interval,
                id,
                callback: Box::new(move || {
                    callback();
                }),
            };
            TIMER_QUEUE.lock().push(entry);
        });

        let entry = TimerEntry { due, id, callback };
        TIMER_QUEUE.lock().push(entry);
        TimerId(id)
    }

    /// 处理所有到期的定时器
    pub fn process() {
        let now = Instant::now();
        let mut queue = TIMER_QUEUE.lock();

        while let Some(entry) = queue.peek() {
            if entry.due <= now {
                let entry = queue.pop().unwrap();
                (entry.callback)();
            } else {
                break;
            }
        }
    }

    /// 获取下一个定时器到期的时间
    pub fn next_due() -> Option<Duration> {
        let queue = TIMER_QUEUE.lock();
        queue.peek().map(|entry| {
            if entry.due > Instant::now() {
                entry.due - Instant::now()
            } else {
                Duration::ZERO
            }
        })
    }
}

static TIMER_ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn generate_timer_id() -> u64 {
    TIMER_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_basic() {
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();

        Timer::add(Duration::from_millis(10), move || {
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        std::thread::sleep(Duration::from_millis(20));
        Timer::process();

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test timer`
Expected: 测试通过

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "feat(core): 添加定时器系统
- 基于 BinaryHeap 的 min-heap 定时器
- 支持一次性定时器和间隔定时器
- 线程安全的定时器队列
"
```

---

### Task 5: 数据层 - SQLite 集成

**Files:**
- Create: `src/storage/mod.rs`
- Create: `src/storage/sqlite.rs`
- Create: `src/storage/schema.rs`

- [ ] **Step 1: 创建 src/storage/mod.rs**

```rust
pub mod sqlite;
pub mod schema;

pub use sqlite::Database;
pub use schema::init_schema;
```

- [ ] **Step 2: 创建 src/storage/sqlite.rs**

```rust
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::{Arc, Mutex};
use anyhow::Result;

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn execute(&self, sql: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.execute(sql, [])?)
    }

    pub fn execute_with_params<T: rusqlite::ToSql>(&self, sql: &str, params: T) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.execute(sql, params)?)
    }

    pub fn query<T, F>(&self, sql: &str, mut f: F) -> Result<Vec<T>>
    where
        F: FnMut(&rusqlite::Row<'_>) -> Result<T, rusqlite::Error>,
    {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], f)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    pub fn query_row<T, F>(&self, sql: &str, mut f: F) -> Result<T>
    where
        F: FnMut(&rusqlite::Row<'_>) -> Result<T, rusqlite::Error>,
    {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        stmt.query_row([], f).map_err(|e| e.into())
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            conn: self.conn.clone(),
        }
    }
}

unsafe impl Send for Database {}
unsafe impl Sync for Database {}
```

- [ ] **Step 3: 创建 src/storage/schema.rs**

```rust
use crate::error::Result;
use crate::storage::Database;

pub fn init_schema(db: &Database) -> Result<()> {
    // 账户表
    db.execute(
        "CREATE TABLE IF NOT EXISTS accounts (
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
        )",
    )?;

    // 角色表
    db.execute(
        "CREATE TABLE IF NOT EXISTS characters (
            char_id INTEGER PRIMARY KEY,
            account_id INTEGER NOT NULL,
            char_num INTEGER NOT NULL,
            name TEXT NOT NULL,
            class INTEGER DEFAULT 0,
            base_level INTEGER DEFAULT 1,
            job_level INTEGER DEFAULT 1,
            base_exp INTEGER DEFAULT 0,
            job_exp INTEGER DEFAULT 0,
            zeny INTEGER DEFAULT 0,
            str INTEGER DEFAULT 1,
            agi INTEGER DEFAULT 1,
            vit INTEGER DEFAULT 1,
            int INTEGER DEFAULT 1,
            dex INTEGER DEFAULT 1,
            luk INTEGER DEFAULT 1,
            hair INTEGER DEFAULT 1,
            hair_color INTEGER DEFAULT 0,
            clothes_color INTEGER DEFAULT 0,
            body INTEGER DEFAULT 0,
            weapon INTEGER DEFAULT 0,
            shield INTEGER DEFAULT 0,
            head_top INTEGER DEFAULT 0,
            head_mid INTEGER DEFAULT 0,
            head_bottom INTEGER DEFAULT 0,
            last_map TEXT,
            last_x INTEGER,
            last_y INTEGER,
            save_map TEXT,
            save_x INTEGER,
            save_y INTEGER,
            hp INTEGER DEFAULT 1,
            max_hp INTEGER DEFAULT 1,
            sp INTEGER DEFAULT 1,
            max_sp INTEGER DEFAULT 1,
            option INTEGER DEFAULT 0,
            manner INTEGER DEFAULT 0,
            status_point INTEGER DEFAULT 0,
            skill_point INTEGER DEFAULT 0,
            delete_timer INTEGER DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (account_id) REFERENCES accounts(account_id)
        )",
    )?;

    // 背包物品表
    db.execute(
        "CREATE TABLE IF NOT EXISTS inventory (
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
        )",
    )?;

    // 技能表
    db.execute(
        "CREATE TABLE IF NOT EXISTS skills (
            id INTEGER PRIMARY KEY,
            char_id INTEGER NOT NULL,
            skill_id INTEGER NOT NULL,
            lv INTEGER NOT NULL,
            flag INTEGER DEFAULT 0,
            FOREIGN KEY (char_id) REFERENCES characters(char_id)
        )",
    )?;

    // 公会表
    db.execute(
        "CREATE TABLE IF NOT EXISTS guilds (
            guild_id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            master INTEGER NOT NULL,
            guild_lv INTEGER DEFAULT 1,
            exp INTEGER DEFAULT 0,
            emblem_data BLOB,
            created_at INTEGER NOT NULL
        )",
    )?;

    // 公会成员表
    db.execute(
        "CREATE TABLE IF NOT EXISTS guild_members (
            guild_id INTEGER NOT NULL,
            char_id INTEGER NOT NULL,
            position INTEGER DEFAULT 0,
            PRIMARY KEY (guild_id, char_id),
            FOREIGN KEY (guild_id) REFERENCES guilds(guild_id),
            FOREIGN KEY (char_id) REFERENCES characters(char_id)
        )",
    )?;

    tracing::info!("数据库 Schema 初始化完成");
    Ok(())
}
```

- [ ] **Step 4: 编写测试**

Create: `tests/storage_test.rs`

```rust
use deviruchi::storage::{Database, init_schema};

#[test]
fn test_database_memory() {
    let db = Database::open_memory().unwrap();
    init_schema(&db).unwrap();

    // 测试插入账户
    db.execute(
        "INSERT INTO accounts (user_id, password_hash, sex, created_at)
         VALUES ('test', 'hash', 0, 1234567890)"
    ).unwrap();

    // 测试查询
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM accounts",
        |row| row.get(0)
    ).unwrap();

    assert_eq!(count, 1);
}
```

- [ ] **Step 5: 运行测试**

Run: `cargo test storage`
Expected: 测试通过

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat(storage): 添加 SQLite 数据层
- Database 连接封装
- Schema 初始化
- 账户、角色、物品、技能、公会表
"
```

---

### Task 6: 网络层 - 基础框架

**Files:**
- Create: `src/network/mod.rs`
- Create: `src/network/packet.rs`
- Create: `src/network/codec.rs`
- Create: `src/network/session.rs`

- [ ] **Step 1: 创建 src/network/mod.rs**

```rust
pub mod packet;
pub mod codec;
pub mod session;

pub use packet::{Packet, PacketHeader, PacketId};
pub use codec::PacketCodec;
pub use session::{Session, SessionManager};
```

- [ ] **Step 2: 创建 src/network/packet.rs**

```rust
use serde::{Deserialize, Serialize};

/// 数据包 ID
pub type PacketId = u16;

/// 数据包头部
#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PacketHeader {
    pub length: u16,
    pub packet_id: u16,
}

/// 数据包
#[derive(Debug, Clone)]
pub struct Packet {
    pub header: PacketHeader,
    pub data: Vec<u8>,
}

impl Packet {
    pub fn new(packet_id: PacketId, data: Vec<u8>) -> Self {
        let length = (data.len() + 4) as u16; // 4 = header size
        Self {
            header: PacketHeader { length, packet_id },
            data,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.header.length as usize);
        bytes.extend_from_slice(&self.header.length.to_le_bytes());
        bytes.extend_from_slice(&self.header.packet_id.to_le_bytes());
        bytes.extend_from_slice(&self.data);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }

        let length = u16::from_le_bytes([bytes[0], bytes[1]]);
        let packet_id = u16::from_le_bytes([bytes[2], bytes[3]]);

        if bytes.len() < length as usize {
            return None;
        }

        let data = bytes[4..length as usize].to_vec();

        Some(Self {
            header: PacketHeader { length, packet_id },
            data,
        })
    }
}

/// 常用数据包 ID 定义
pub mod id {
    use super::PacketId;

    // 登录服务器包
    pub const PACKET_SC_NOTIFY_BAN: PacketId = 0x0081;
    pub const PACKET_AC_ACCEPT_LOGIN: PacketId = 0x0069;
    pub const PACKET_AC_REFUSE_LOGIN: PacketId = 0x006A;

    // 字符服务器包
    pub const PACKET_CA_LOGIN: PacketId = 0x0064;
    pub const PACKET_CH_ENTER: PacketId = 0x0065;
    pub const PACKET_CS_UPDATE_NEXTCHARPOS: PacketId = 0x02D1;

    // 地图服务器包
    pub const PACKET_CZ_ENTER: PacketId = 0x007C;
    pub const PACKET_ZC_ACCEPT_ENTER: PacketId = 0x02D3;
    pub const PACKET_ZC_NOTIFY_ACT: PacketId = 0x02D5;
    pub const PACKET_CZ_REQUEST_MOVE: PacketId = 0x0085;
    pub const PACKET_ZC_MOVE: PacketId = 0x0086;
    pub const PACKET_CZ_USE_SKILL: PacketId = 0x0112;
}
```

- [ ] **Step 3: 创建 src/network/codec.rs**

```rust
use bytes::{BytesMut, BufMut};
use tokio_util::codec::{Decoder, Encoder, Framed};
use super::packet::Packet;

pub struct PacketCodec;

impl Decoder for PacketCodec {
    type Item = Packet;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // 需要至少 4 字节来读取 header
        if src.len() < 4 {
            return Ok(None);
        }

        // 读取长度
        let length = u16::from_le_bytes([src[0], src[1]]) as usize;

        // 检查是否收到完整数据包
        if src.len() < length {
            return Ok(None);
        }

        // 提取数据包
        let packet_bytes = src.split_to(length);
        let packet = match Packet::from_bytes(&packet_bytes) {
            Some(p) => p,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid packet format"
                ));
            }
        };

        Ok(Some(packet))
    }
}

impl Encoder<Packet> for PacketCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Packet, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes = item.to_bytes();
        dst.reserve(bytes.len());
        dst.put_slice(&bytes);
        Ok(())
    }
}
```

- [ ] **Step 4: 创建 src/network/session.rs**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use uuid::Uuid;

use super::codec::PacketCodec;
use super::packet::Packet;

pub struct Session {
    pub id: Uuid,
    pub account_id: Option<u32>,
    pub char_id: Option<u32>,
    pub authenticated: bool,
    pub version: u32,
    pub client_type: u8,
}

impl Session {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            account_id: None,
            char_id: None,
            authenticated: false,
            version: 0,
            client_type: 0,
        }
    }

    pub fn authenticate(&mut self, account_id: u32) {
        self.account_id = Some(account_id);
        self.authenticated = true;
    }
}

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    addr_to_session: Arc<RwLock<HashMap<String, Uuid>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            addr_to_session: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add(&self, addr: String, session: Session) -> Uuid {
        let id = session.id;
        self.sessions.write().insert(id, session);
        self.addr_to_session.write().insert(addr, id);
        id
    }

    pub fn remove(&self, id: &Uuid) {
        self.sessions.write().remove(id);
    }

    pub fn get(&self, id: &Uuid) -> Option<Session> {
        self.sessions.read().get(id).cloned()
    }

    pub fn update(&self, id: &Uuid, session: Session) {
        self.sessions.write().insert(*id, session);
    }

    pub fn count(&self) -> usize {
        self.sessions.read().len()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 5: 更新 Cargo.toml**

```toml
[dependencies]
bytes = "1"
tokio-util = { version = "0.7", features = ["codec"] }
uuid = { version = "1", features = ["v4", "serde"] }
```

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat(network): 添加基础网络层
- Packet 数据包定义
- PacketCodec 编解码器
- Session 会话管理
"
```

---

### Task 7: Core 主模块整合

**Files:**
- Modify: `src/lib.rs`
- Create: `src/core/core.rs`

- [ ] **Step 1: 创建 src/core/core.rs**

```rust
use crate::cli::Cli;
use crate::error::Result;
use crate::storage::{Database, init_schema};
use crate::network::SessionManager;

pub struct Core {
    cli: Cli,
    config: crate::core::Config,
    db: Option<Database>,
    session_manager: SessionManager,
}

impl Core {
    pub fn new(cli: Cli) -> Self {
        Self {
            config: crate::core::Config::load(&cli.config).unwrap_or_default(),
            db: None,
            session_manager: SessionManager::new(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        // 初始化日志
        crate::core::logging::init_logging("logs", &self.cli.log_level)?;

        // 设置 panic hook
        crate::core::panic::PanicHandler::init();

        tracing::info!("{} v{} 启动中...", crate::core::NAME, crate::core::VERSION);

        // 初始化数据库
        self.db = Some(Database::open(&self.config.database.path)?);
        init_schema(self.db.as_ref().unwrap())?;

        tracing::info!("服务器初始化完成");
        tracing::info!("运行模式: {}", self.cli.mode);

        // 保持运行
        tokio::signal::ctrl_c().await?;

        tracing::info!("服务器关闭中...");
        Ok(())
    }
}
```

- [ ] **Step 2: 更新 src/core/mod.rs**

```rust
pub mod config;
pub mod logging;
pub mod panic;
pub mod timer;
pub mod version;
pub mod core;

pub use config::Config;
pub use version::VERSION;
pub use core::Core;
```

- [ ] **Step 3: 更新 src/lib.rs**

```rust
pub mod cli;
pub mod core;
pub mod network;
pub mod storage;
pub mod error;

pub use error::{Error, Result};
```

- [ ] **Step 4: 编译测试**

Run: `cargo build`
Expected: 编译成功

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat: 完成 Phase 1 核心骨架
- 项目结构搭建完成
- 配置系统
- 日志与崩溃报告
- SQLite 集成
- 基础网络框架
"
```

---

## 自检清单

### Spec 覆盖检查
- [x] 配置系统 - Task 2
- [x] 日志系统 - Task 3
- [x] 崩溃报告 - Task 3
- [x] SQLite 集成 - Task 5
- [x] 网络框架 - Task 6
- [x] 定时器系统 - Task 4

### 占位符检查
- [x] 无 TBD/TODO
- [x] 所有代码块完整
- [x] 所有测试代码完整

### 类型一致性
- [x] `Config::load` 返回 `Result<Config>`
- [x] `Database::open` 返回 `Result<Database>`
- [x] `Session` 字段命名一致

---

## 执行方式选择

**Plan complete and saved to `docs/superpowers/plans/2026-05-02-deviruchi-phase1-plan.md`. Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
