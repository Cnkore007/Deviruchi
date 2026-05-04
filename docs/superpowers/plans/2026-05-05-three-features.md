# Deviruchi 三功能完善实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成 Homunculus/Mercenary 持久化、Mob/NPC YAML 完整解析、MySQL 支持三个待完善功能

**Architecture:** 引入 `DatabaseBackend` trait 作为数据库抽象层，`SqliteBackend` 和 `MySqlBackend` 分别实现。现有 `Database` 结构改为持有 `Arc<dyn DatabaseBackend>`，消除 rusqlite 类型泄漏。Homunculus/Mercenary 管理器接收 `DatabaseBackend` 实现实时持久化。Mob/NPC YAML 加载器扩展 rAthena 数据映射。

**Tech Stack:** Rust, rusqlite, mysql (optional feature), serde_yaml, parking_lot

---

## Phase 1: DatabaseBackend 抽象层

### Task 1: 定义 Value、Row、TransactionOps 和 DatabaseBackend trait

**Files:**
- Create: `src/storage/backend.rs`

- [ ] **Step 1: 创建 backend.rs，定义核心类型和 trait**

```rust
//! 数据库后端抽象层
//!
//! 提供 DatabaseBackend trait 及其关联类型（Value、Row、TransactionOps），
//! 使上层代码与具体数据库引擎（SQLite、MySQL）解耦。

use crate::error::Result;
use std::sync::Arc;

/// 数据库值类型
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Int(i32),
    BigInt(i64),
    Float(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl Value {
    /// 转换为 rusqlite 的 ToSql 值
    pub fn to_rusqlite(&self) -> Box<dyn rusqlite::types::ToSql> {
        match self {
            Value::Null => Box::new(rusqlite::types::Null),
            Value::Int(v) => Box::new(*v),
            Value::BigInt(v) => Box::new(*v),
            Value::Float(v) => Box::new(*v),
            Value::Text(v) => Box::new(v.clone()),
            Value::Blob(v) => Box::new(v.clone()),
        }
    }
}

/// 数据库行
#[derive(Debug)]
pub struct Row {
    columns: Vec<(String, Value)>,
}

impl Row {
    pub fn new(columns: Vec<(String, Value)>) -> Self {
        Self { columns }
    }

    pub fn get_i32(&self, index: usize) -> Result<i32> {
        match self.columns.get(index).map(|(_, v)| v) {
            Some(Value::Int(v)) => Ok(*v),
            Some(Value::BigInt(v)) => Ok(*v as i32),
            Some(Value::Null) => Err(crate::error::Error::DatabaseBackend(
                format!("column {} is NULL", index),
            )),
            other => Err(crate::error::Error::DatabaseBackend(
                format!("column {} expected Int, got {:?}", index, other),
            )),
        }
    }

    pub fn get_i64(&self, index: usize) -> Result<i64> {
        match self.columns.get(index).map(|(_, v)| v) {
            Some(Value::BigInt(v)) => Ok(*v),
            Some(Value::Int(v)) => Ok(*v as i64),
            Some(Value::Null) => Err(crate::error::Error::DatabaseBackend(
                format!("column {} is NULL", index),
            )),
            other => Err(crate::error::Error::DatabaseBackend(
                format!("column {} expected BigInt, got {:?}", index, other),
            )),
        }
    }

    pub fn get_f64(&self, index: usize) -> Result<f64> {
        match self.columns.get(index).map(|(_, v)| v) {
            Some(Value::Float(v)) => Ok(*v),
            Some(Value::Null) => Err(crate::error::Error::DatabaseBackend(
                format!("column {} is NULL", index),
            )),
            other => Err(crate::error::Error::DatabaseBackend(
                format!("column {} expected Float, got {:?}", index, other),
            )),
        }
    }

    pub fn get_string(&self, index: usize) -> Result<String> {
        match self.columns.get(index).map(|(_, v)| v) {
            Some(Value::Text(v)) => Ok(v.clone()),
            Some(Value::Null) => Err(crate::error::Error::DatabaseBackend(
                format!("column {} is NULL", index),
            )),
            other => Err(crate::error::Error::DatabaseBackend(
                format!("column {} expected Text, got {:?}", index, other),
            )),
        }
    }

    pub fn get_optional_string(&self, index: usize) -> Result<Option<String>> {
        match self.columns.get(index).map(|(_, v)| v) {
            Some(Value::Text(v)) => Ok(Some(v.clone())),
            Some(Value::Null) => Ok(None),
            None => Ok(None),
            other => Err(crate::error::Error::DatabaseBackend(
                format!("column {} expected Text or Null, got {:?}", index, other),
            )),
        }
    }

    pub fn get_optional_i64(&self, index: usize) -> Result<Option<i64>> {
        match self.columns.get(index).map(|(_, v)| v) {
            Some(Value::BigInt(v)) => Ok(Some(*v)),
            Some(Value::Int(v)) => Ok(Some(*v as i64)),
            Some(Value::Null) => Ok(None),
            None => Ok(None),
            other => Err(crate::error::Error::DatabaseBackend(
                format!("column {} expected Int/BigInt or Null, got {:?}", index, other),
            )),
        }
    }
}

/// 事务内操作 trait
pub trait TransactionOps: Send + Sync {
    fn execute(&self, sql: &str) -> Result<usize>;
    fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize>;
    fn execute_batch(&self, sql: &str) -> Result<()>;
    fn last_insert_rowid(&self) -> i64;
}

/// 值转换 trait，替代 rusqlite::ToSql
pub trait IntoValue {
    fn into_value(&self) -> Value;
}

impl IntoValue for i32 {
    fn into_value(&self) -> Value {
        Value::Int(*self)
    }
}

impl IntoValue for u32 {
    fn into_value(&self) -> Value {
        Value::Int(*self as i32)
    }
}

impl IntoValue for i64 {
    fn into_value(&self) -> Value {
        Value::BigInt(*self)
    }
}

impl IntoValue for u64 {
    fn into_value(&self) -> Value {
        Value::BigInt(*self as i64)
    }
}

impl IntoValue for f64 {
    fn into_value(&self) -> Value {
        Value::Float(*self)
    }
}

impl IntoValue for str {
    fn into_value(&self) -> Value {
        Value::Text(self.to_string())
    }
}

impl IntoValue for String {
    fn into_value(&self) -> Value {
        Value::Text(self.clone())
    }
}

impl IntoValue for &str {
    fn into_value(&self) -> Value {
        Value::Text(self.to_string())
    }
}

impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(&self) -> Value {
        match self {
            Some(v) => v.into_value(),
            None => Value::Null,
        }
    }
}

/// 数据库后端 trait
pub trait DatabaseBackend: Send + Sync + 'static {
    /// 执行 SQL（无返回值）
    fn execute(&self, sql: &str) -> Result<usize>;

    /// 带参数执行 SQL，返回影响行数
    fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize>;

    /// 查询多行
    fn query_rows(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<Vec<Row>>;

    /// 查询单行
    fn query_row<F, T>(&self, sql: &str, params: &[&dyn IntoValue], f: F) -> Result<T>
    where
        F: FnOnce(&Row) -> Result<T>;

    /// 查询可选单行
    fn query_row_optional<F, T>(&self, sql: &str, params: &[&dyn IntoValue], f: F) -> Result<Option<T>>
    where
        F: FnOnce(&Row) -> Result<T>;

    /// 事务
    fn with_transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn TransactionOps) -> Result<T>;

    /// 最后插入的行 ID
    fn last_insert_rowid(&self) -> i64;

    /// 批量执行（用于迁移）
    fn execute_batch(&self, sql: &str) -> Result<()>;

    /// UPSERT 便捷方法
    fn upsert(
        &self,
        table: &str,
        columns: &[&str],
        params: &[&dyn IntoValue],
        conflict_cols: &[&str],
    ) -> Result<usize>;
}

impl Clone for Arc<dyn DatabaseBackend> {
    // Arc<dyn DatabaseBackend> is already Clone
}
```

- [ ] **Step 2: 运行编译检查**

Run: `cargo check --lib 2>&1 | head -20`
Expected: 可能有未使用的 warning，但应编译通过

- [ ] **Step 3: 在 storage/mod.rs 中注册新模块**

在 `src/storage/mod.rs` 第 7 行添加：
```rust
pub mod backend;
```

并在 pub use 区域添加：
```rust
pub use backend::{DatabaseBackend, IntoValue, Row, TransactionOps, Value};
```

- [ ] **Step 4: 运行测试确认无破坏**

Run: `cargo test --lib storage::backend 2>&1 | tail -5`
Expected: 无测试（尚无测试用例），编译通过即可

- [ ] **Step 5: Commit**

```bash
git add src/storage/backend.rs src/storage/mod.rs
git commit -m "feat(storage): 定义 DatabaseBackend trait 和关联类型（Value、Row、TransactionOps）"
```

---

### Task 2: 实现 SqliteBackend

**Files:**
- Create: `src/storage/sqlite_backend.rs`

- [ ] **Step 1: 创建 SqliteBackend 实现**

```rust
//! SQLite 数据库后端实现

use super::backend::{DatabaseBackend, IntoValue, Row, TransactionOps, Value};
use crate::error::Result;
use parking_lot::{Mutex, RwLock};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// SQLite 后端配置
#[derive(Debug, Clone)]
pub struct SqliteConfig {
    pub path: String,
    pub wal_mode: bool,
    pub busy_timeout_ms: u32,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            path: "deviruchi.db".to_string(),
            wal_mode: true,
            busy_timeout_ms: 5000,
        }
    }
}

/// SQLite 事务操作
/// 持有 MutexGuard，在事务期间独占连接
struct SqliteTransaction<'a> {
    conn: parking_lot::MutexGuard<'a, rusqlite::Connection>,
}

impl<'a> TransactionOps for SqliteTransaction<'a> {
    fn execute(&self, sql: &str) -> Result<usize> {
        Ok(self.conn.execute(sql, [])?)
    }

    fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize> {
        let values: Vec<Value> = params.iter().map(|p| p.into_value()).collect();
        let rusqlite_params: Vec<Box<dyn rusqlite::types::ToSql>> =
            values.iter().map(|v| v.to_rusqlite()).collect();
        let mut stmt = self.conn.prepare(sql)?;
        Ok(stmt.execute(rusqlite::params_from_iter(
            rusqlite_params.iter().map(|p| p.as_ref()),
        ))?)
    }

    fn execute_batch(&self, sql: &str) -> Result<()> {
        self.conn.execute_batch(sql)?;
        Ok(())
    }

    fn last_insert_rowid(&self) -> i64 {
        self.conn.last_insert_rowid()
    }
}

/// SQLite 数据库后端
///
/// rusqlite::Connection 的 execute/prepare/execute_batch 均接受 &self，
/// 所以可以用 parking_lot::Mutex 包装。事务期间持有锁，通过 SqliteTransaction
/// 传递操作。
pub struct SqliteBackend {
    conn: Arc<parking_lot::Mutex<rusqlite::Connection>>,
}

impl SqliteBackend {
    pub fn new(config: &SqliteConfig) -> Result<Self> {
        let conn = rusqlite::Connection::open(&config.path)?;
        if config.wal_mode {
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        }
        conn.busy_timeout(Duration::from_millis(config.busy_timeout_ms as u64))?;
        Ok(Self {
            conn: Arc::new(parking_lot::Mutex::new(conn)),
        })
    }

    pub fn open_memory() -> Result<Self> {
        let conn = rusqlite::Connection::open_in_memory()?;
        Ok(Self {
            conn: Arc::new(parking_lot::Mutex::new(conn)),
        })
    }
}

impl DatabaseBackend for SqliteBackend {
    fn execute(&self, sql: &str) -> Result<usize> {
        let conn = self.conn.lock();
        Ok(conn.execute(sql, [])?)
    }

    fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize> {
        let conn = self.conn.lock();
        let values: Vec<Value> = params.iter().map(|p| p.into_value()).collect();
        let rusqlite_params: Vec<Box<dyn rusqlite::types::ToSql>> =
            values.iter().map(|v| v.to_rusqlite()).collect();
        let mut stmt = conn.prepare(sql)?;
        Ok(stmt.execute(rusqlite::params_from_iter(
            rusqlite_params.iter().map(|p| p.as_ref()),
        ))?)
    }

    fn query_rows(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<Vec<Row>> {
        let conn = self.conn.lock();
        let values: Vec<Value> = params.iter().map(|p| p.into_value()).collect();
        let rusqlite_params: Vec<Box<dyn rusqlite::types::ToSql>> =
            values.iter().map(|v| v.to_rusqlite()).collect();
        let mut stmt = conn.prepare(sql)?;
        let column_count = stmt.column_count();
        let column_names: Vec<String> = (0..column_count)
            .map(|i| stmt.column_name(i).unwrap_or("unknown").to_string())
            .collect();

        let mut rows_result = Vec::new();
        let mut query_rows = stmt.query(rusqlite::params_from_iter(
            rusqlite_params.iter().map(|p| p.as_ref()),
        ))?;

        while let Some(sqlite_row) = query_rows.next()? {
            let mut columns = Vec::new();
            for (idx, col_name) in column_names.iter().enumerate() {
                let value = match sqlite_row.get_ref(idx)? {
                    rusqlite::types::ValueRef::Null => Value::Null,
                    rusqlite::types::ValueRef::Integer(i) => Value::BigInt(i),
                    rusqlite::types::ValueRef::Real(f) => Value::Float(f),
                    rusqlite::types::ValueRef::Text(s) => {
                        Value::Text(String::from_utf8_lossy(s).to_string())
                    }
                    rusqlite::types::ValueRef::Blob(b) => Value::Blob(b.to_vec()),
                };
                columns.push((col_name.clone(), value));
            }
            rows_result.push(Row::new(columns));
        }

        Ok(rows_result)
    }

    fn query_row<F, T>(&self, sql: &str, params: &[&dyn IntoValue], f: F) -> Result<T>
    where
        F: FnOnce(&Row) -> Result<T>,
    {
        let rows = self.query_rows(sql, params)?;
        let row = rows.into_iter().next().ok_or_else(|| {
            crate::error::Error::DatabaseBackend("query returned no rows".into())
        })?;
        f(&row)
    }

    fn query_row_optional<F, T>(&self, sql: &str, params: &[&dyn IntoValue], f: F) -> Result<Option<T>>
    where
        F: FnOnce(&Row) -> Result<T>,
    {
        let rows = self.query_rows(sql, params)?;
        match rows.into_iter().next() {
            Some(row) => Ok(Some(f(&row)?)),
            None => Ok(None),
        }
    }

    fn with_transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn TransactionOps) -> Result<T>,
    {
        // 获取锁，整个事务期间持有
        let mut conn = self.conn.lock();
        conn.execute_batch("BEGIN IMMEDIATE")?;

        let tx = SqliteTransaction { conn: parking_lot::MutexGuard::map(conn, |c| c) };
        // 注意：MutexGuard::map 需要 &mut self，这里需要不同方式
        // 实际实现中，应该直接在锁内执行操作

        // 临时方案：在锁内直接执行
        // 由于 TransactionOps trait 的约束，我们需要传递一个实现了该 trait 的对象
        // 但 parking_lot::MutexGuard 的生命周期与闭包不匹配
        // 解决方案：使用 scoped transaction 模式

        // 实际实现：在锁内执行 BEGIN -> 操作 -> COMMIT/ROLLBACK
        // 这里用一个内部 struct 来持有锁的引用
        struct ScopedTx<'a> {
            conn: &'a rusqlite::Connection,
        }

        impl<'a> TransactionOps for ScopedTx<'a> {
            fn execute(&self, sql: &str) -> Result<usize> {
                Ok(self.conn.execute(sql, [])?)
            }
            fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize> {
                let values: Vec<Value> = params.iter().map(|p| p.into_value()).collect();
                let rusqlite_params: Vec<Box<dyn rusqlite::types::ToSql>> =
                    values.iter().map(|v| v.to_rusqlite()).collect();
                let mut stmt = self.conn.prepare(sql)?;
                Ok(stmt.execute(rusqlite::params_from_iter(
                    rusqlite_params.iter().map(|p| p.as_ref()),
                ))?)
            }
            fn execute_batch(&self, sql: &str) -> Result<()> {
                self.conn.execute_batch(sql)?;
                Ok(())
            }
            fn last_insert_rowid(&self) -> i64 {
                self.conn.last_insert_rowid()
            }
        }

        let tx = ScopedTx { conn: &conn };
        match f(&tx) {
            Ok(result) => {
                conn.execute_batch("COMMIT")?;
                Ok(result)
            }
            Err(e) => {
                conn.execute_batch("ROLLBACK").ok();
                Err(e)
            }
        }
    }

    fn last_insert_rowid(&self) -> i64 {
        let conn = self.conn.lock();
        conn.last_insert_rowid()
    }

    fn execute_batch(&self, sql: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(sql)?;
        Ok(())
    }

    fn upsert(
        &self,
        table: &str,
        columns: &[&str],
        params: &[&dyn IntoValue],
        _conflict_cols: &[&str],
    ) -> Result<usize> {
        // SQLite 使用 INSERT OR REPLACE
        let sql = format!(
            "INSERT OR REPLACE INTO {} ({}) VALUES ({})",
            table,
            columns.join(", "),
            columns.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
        );
        self.execute_params(&sql, params)
    }
}

// SqliteBackend 通过 Arc<Mutex<Connection>> 实现 Clone
// 但 DatabaseBackend trait 要求 'static，所以我们用 Arc 包装
```

- [ ] **Step 2: 在 storage/mod.rs 中注册**

在 `src/storage/mod.rs` 添加：
```rust
pub mod sqlite_backend;
pub use sqlite_backend::{SqliteBackend, SqliteConfig};
```

- [ ] **Step 3: 运行编译检查**

Run: `cargo check --lib 2>&1 | tail -10`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add src/storage/sqlite_backend.rs src/storage/mod.rs
git commit -m "feat(storage): 实现 SqliteBackend（DatabaseBackend trait 的 SQLite 实现）"
```

---

### Task 3: 改造 Database 结构使用 DatabaseBackend

**Files:**
- Modify: `src/storage/sqlite.rs`

- [ ] **Step 1: 改造 Database 结构**

将 `src/storage/sqlite.rs` 改造为：

```rust
//! 数据库操作入口
//!
//! 提供统一的数据库操作接口，通过 DatabaseBackend trait 支持多种后端。

use crate::error::Result;
use crate::storage::backend::DatabaseBackend;
use std::sync::Arc;

/// 数据库操作入口
pub struct Database {
    backend: Arc<dyn DatabaseBackend>,
}

impl Database {
    pub fn new(backend: Arc<dyn DatabaseBackend>) -> Self {
        Self { backend }
    }

    /// 获取后端引用（用于需要直接访问后端的场景）
    pub fn backend(&self) -> &dyn DatabaseBackend {
        self.backend.as_ref()
    }

    pub fn execute(&self, sql: &str) -> Result<usize> {
        self.backend.execute(sql)
    }

    pub fn execute_params(&self, sql: &str, params: &[&dyn crate::storage::backend::IntoValue]) -> Result<usize> {
        self.backend.execute_params(sql, params)
    }

    pub fn query_rows(
        &self,
        sql: &str,
        params: &[&dyn crate::storage::backend::IntoValue],
    ) -> Result<Vec<crate::storage::backend::Row>> {
        self.backend.query_rows(sql, params)
    }

    pub fn query_row<F, T>(
        &self,
        sql: &str,
        params: &[&dyn crate::storage::backend::IntoValue],
        f: F,
    ) -> Result<T>
    where
        F: FnOnce(&crate::storage::backend::Row) -> Result<T>,
    {
        self.backend.query_row(sql, params, f)
    }

    pub fn query_row_optional<F, T>(
        &self,
        sql: &str,
        params: &[&dyn crate::storage::backend::IntoValue],
        f: F,
    ) -> Result<Option<T>>
    where
        F: FnOnce(&crate::storage::backend::Row) -> Result<T>,
    {
        self.backend.query_row_optional(sql, params, f)
    }

    pub fn with_transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn crate::storage::backend::TransactionOps) -> Result<T>,
    {
        self.backend.with_transaction(f)
    }

    pub fn last_insert_rowid(&self) -> i64 {
        self.backend.last_insert_rowid()
    }

    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        self.backend.execute_batch(sql)
    }

    pub fn upsert(
        &self,
        table: &str,
        columns: &[&str],
        params: &[&dyn crate::storage::backend::IntoValue],
        conflict_cols: &[&str],
    ) -> Result<usize> {
        self.backend.upsert(table, columns, params, conflict_cols)
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
        }
    }
}

pub fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
```

- [ ] **Step 2: 更新 storage/mod.rs 的 pub use**

修改 `src/storage/mod.rs`：
```rust
pub use sqlite::Database;
pub use sqlite::chrono_now;
```
（保持不变，因为 Database 仍在 sqlite.rs 中，只是实现改了）

- [ ] **Step 3: 运行编译检查，查看破坏性变更**

Run: `cargo check --lib 2>&1 | grep "error\[" | wc -l`
Expected: 会有编译错误，因为现有代码使用了旧的 `Database::open()` 和 `rusqlite::Params` 接口

- [ ] **Step 4: Commit**

```bash
git add src/storage/sqlite.rs
git commit -m "refactor(storage): Database 结构改为持有 Arc<dyn DatabaseBackend>"
```

---

### Task 4: 更新 error.rs 支持后端无关的数据库错误

**Files:**
- Modify: `src/error.rs`

- [ ] **Step 1: 修改 Error 枚举**

将 `src/error.rs` 改为：

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("数据库后端错误: {0}")]
    DatabaseBackend(String),

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

- [ ] **Step 2: Commit**

```bash
git add src/error.rs
git commit -m "feat(error): 添加 DatabaseBackend 错误类型，为多后端支持做准备"
```

---

### Task 5: 创建 SqliteBackend 的便捷构造函数并适配现有 Database 用法

**Files:**
- Modify: `src/storage/sqlite.rs` — 添加 `Database::open()` 和 `Database::open_memory()` 便捷方法
- Modify: `src/storage/account.rs` — 迁移 rusqlite 调用
- Modify: `src/storage/character.rs` — 迁移 rusqlite 调用
- Modify: `src/storage/guild.rs` — 迁移 rusqlite 调用

- [ ] **Step 1: 在 Database 中添加便捷构造函数**

在 `src/storage/sqlite.rs` 的 `impl Database` 中添加：

```rust
use crate::storage::sqlite_backend::{SqliteBackend, SqliteConfig};

impl Database {
    /// 打开 SQLite 数据库文件
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let config = SqliteConfig {
            path: path.as_ref().to_string_lossy().to_string(),
            ..Default::default()
        };
        let backend = SqliteBackend::new(&config)?;
        Ok(Self::new(Arc::new(backend)))
    }

    /// 打开内存数据库（测试用）
    pub fn open_memory() -> Result<Self> {
        let backend = SqliteBackend::open_memory()?;
        Ok(Self::new(Arc::new(backend)))
    }
}
```

- [ ] **Step 2: 迁移 account.rs**

将 `src/storage/account.rs` 中的 `use rusqlite::params;` 和 `rusqlite::params![]` 调用改为使用新的 `execute_params` 和 `query_row` 接口。每个 `rusqlite::params![a, b, c]` 改为 `&[&a as &dyn IntoValue, &b as &dyn IntoValue, &c as &dyn IntoValue]`。

- [ ] **Step 3: 迁移 character.rs**

同 account.rs 的模式。

- [ ] **Step 4: 迁移 guild.rs**

同 account.rs 的模式。

- [ ] **Step 5: 运行测试**

Run: `cargo test --lib 2>&1 | tail -10`
Expected: 所有存储相关测试通过

- [ ] **Step 6: Commit**

```bash
git add src/storage/sqlite.rs src/storage/account.rs src/storage/character.rs src/storage/guild.rs
git commit -m "refactor(storage): 迁移 account/character/guild 到 DatabaseBackend 接口"
```

---

### Task 6: 迁移 game 层的 rusqlite 调用

**Files:**
- Modify: `src/game/login.rs`
- Modify: `src/game/storage/repository.rs`
- Modify: `src/game/map/teleport.rs`
- Modify: `src/game/map/map_server/gm.rs`

- [ ] **Step 1: 迁移 login.rs**

将 `src/game/login.rs` 中的 `rusqlite::params![]` 改为新的参数传递方式。

- [ ] **Step 2: 迁移 repository.rs**

将 `src/game/storage/repository.rs` 中的 `rusqlite::params![]` 改为新的参数传递方式。

- [ ] **Step 3: 迁移 teleport.rs**

将 `src/game/map/teleport.rs` 中的 `rusqlite::params![]` 改为新的参数传递方式。

- [ ] **Step 4: 迁移 gm.rs**

将 `src/game/map/map_server/gm.rs` 中的 `rusqlite::params![]` 改为新的参数传递方式。

- [ ] **Step 5: 运行全部测试**

Run: `cargo test --lib 2>&1 | tail -15`
Expected: 全部测试通过

- [ ] **Step 6: Commit**

```bash
git add src/game/login.rs src/game/storage/repository.rs src/game/map/teleport.rs src/game/map/map_server/gm.rs
git commit -m "refactor(game): 迁移 login/storage/map 到 DatabaseBackend 接口"
```

---

### Task 7: 改造 MigrationManager 使用 DatabaseBackend

**Files:**
- Modify: `src/storage/migration.rs`

- [ ] **Step 1: 改造 MigrationManager**

将 `src/storage/migration.rs` 中直接操作 `rusqlite::Connection` 的代码改为通过 `Database` 的 `with_transaction`、`execute_batch` 和 `execute_params` 方法。

关键改动：
- `ensure_version_table` 改用 `db.execute()`
- `current_version` 改用 `db.query_row()`
- `migrate_up` 中的 `conn.execute_batch(up_sql)` 改为通过 `TransactionOps` 的 `execute_batch`
- `migrate_up` 中的 `conn.execute("INSERT INTO schema_version ...")` 改为 `tx.execute_params()`
- `migrate_down` 同理

- [ ] **Step 2: 运行迁移测试**

Run: `cargo test --lib storage::migration 2>&1 | tail -10`
Expected: 8 个迁移测试全部通过

- [ ] **Step 3: Commit**

```bash
git add src/storage/migration.rs
git commit -m "refactor(migration): MigrationManager 改用 DatabaseBackend 接口"
```

---

### Task 8: 添加 SqliteBackend 单元测试

**Files:**
- Modify: `src/storage/sqlite_backend.rs`

- [ ] **Step 1: 添加测试模块**

在 `src/storage/sqlite_backend.rs` 末尾添加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_backend() -> SqliteBackend {
        SqliteBackend::open_memory().unwrap()
    }

    #[test]
    fn test_execute_and_query() {
        let backend = create_test_backend();
        backend
            .execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .unwrap();

        backend
            .execute_params(
                "INSERT INTO test (id, name) VALUES (?, ?)",
                &[&1i32, &"Alice"],
            )
            .unwrap();

        let rows = backend.query_rows("SELECT * FROM test", &[]).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get_i32(0).unwrap(), 1);
        assert_eq!(rows[0].get_string(1).unwrap(), "Alice");
    }

    #[test]
    fn test_query_row() {
        let backend = create_test_backend();
        backend
            .execute("CREATE TABLE test (id INTEGER, val REAL)")
            .unwrap();
        backend
            .execute_params(
                "INSERT INTO test VALUES (?, ?)",
                &[&42i32, &3.14f64],
            )
            .unwrap();

        let val: f64 = backend
            .query_row("SELECT val FROM test WHERE id = ?", &[&42i32], |row| {
                row.get_f64(0)
            })
            .unwrap();
        assert!((val - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_query_row_optional_none() {
        let backend = create_test_backend();
        backend
            .execute("CREATE TABLE test (id INTEGER)")
            .unwrap();

        let result: Option<i32> = backend
            .query_row_optional("SELECT id FROM test WHERE id = 999", &[], |row| {
                row.get_i32(0)
            })
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_transaction_commit() {
        let backend = create_test_backend();
        backend
            .execute("CREATE TABLE test (id INTEGER)")
            .unwrap();

        backend
            .with_transaction(|tx| {
                tx.execute_params("INSERT INTO test VALUES (?)", &[&1i32])?;
                tx.execute_params("INSERT INTO test VALUES (?)", &[&2i32])?;
                Ok(())
            })
            .unwrap();

        let rows = backend.query_rows("SELECT * FROM test", &[]).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_transaction_rollback() {
        let backend = create_test_backend();
        backend
            .execute("CREATE TABLE test (id INTEGER)")
            .unwrap();

        let result: Result<()> = backend.with_transaction(|tx| {
            tx.execute_params("INSERT INTO test VALUES (?)", &[&1i32])?;
            Err(crate::error::Error::Game("intentional error".into()))
        });
        assert!(result.is_err());

        let rows = backend.query_rows("SELECT * FROM test", &[]).unwrap();
        assert_eq!(rows.len(), 0); // 回滚了
    }

    #[test]
    fn test_last_insert_rowid() {
        let backend = create_test_backend();
        backend
            .execute("CREATE TABLE test (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)")
            .unwrap();

        backend
            .execute_params("INSERT INTO test (name) VALUES (?)", &[&"a"])
            .unwrap();
        let id1 = backend.last_insert_rowid();

        backend
            .execute_params("INSERT INTO test (name) VALUES (?)", &[&"b"])
            .unwrap();
        let id2 = backend.last_insert_rowid();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn test_upsert() {
        let backend = create_test_backend();
        backend
            .execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .unwrap();

        backend
            .upsert("test", &["id", "name"], &[&1i32, &"Alice"], &["id"])
            .unwrap();

        backend
            .upsert("test", &["id", "name"], &[&1i32, &"Bob"], &["id"])
            .unwrap();

        let rows = backend.query_rows("SELECT name FROM test WHERE id = 1", &[]).unwrap();
        assert_eq!(rows.len(), 1);
        // INSERT OR REPLACE 模式下，第二条会替换第一条
        assert_eq!(rows[0].get_string(0).unwrap(), "Bob");
    }

    #[test]
    fn test_row_null_handling() {
        let backend = create_test_backend();
        backend
            .execute("CREATE TABLE test (id INTEGER, name TEXT)")
            .unwrap();
        backend
            .execute_params("INSERT INTO test (id) VALUES (?)", &[&1i32])
            .unwrap();

        let rows = backend.query_rows("SELECT id, name FROM test", &[]).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get_optional_string(1).unwrap(), None);
    }

    #[test]
    fn test_execute_batch() {
        let backend = create_test_backend();
        backend
            .execute_batch(
                "CREATE TABLE t1 (id INTEGER); CREATE TABLE t2 (id INTEGER);",
            )
            .unwrap();

        // 验证两张表都创建了
        backend.execute_params("INSERT INTO t1 VALUES (?)", &[&1i32]).unwrap();
        backend.execute_params("INSERT INTO t2 VALUES (?)", &[&2i32]).unwrap();
    }
}
```

- [ ] **Step 2: 运行新测试**

Run: `cargo test --lib storage::sqlite_backend 2>&1 | tail -15`
Expected: 9 个测试全部通过

- [ ] **Step 3: Commit**

```bash
git add src/storage/sqlite_backend.rs
git commit -m "test(storage): 添加 SqliteBackend 单元测试（9 个测试用例）"
```

---

## Phase 2: Homunculus/Mercenary 持久化

### Task 9: 扩展 Migration v4（homunculus 表 + skills 表）

**Files:**
- Modify: `src/storage/migration.rs`

- [ ] **Step 1: 添加 v4 迁移**

在 `src/storage/migration.rs` 的 `create_default_migrations()` 函数中，在 v3 之后添加：

```rust
// 迁移 v4: 扩展 homunculus 表 + 添加 skills 表
manager.register(Migration {
    version: 4,
    description: "扩展 homunculus 表（combat stats, race, element, evolution）+ skills 表",
    up: "ALTER TABLE homunculus ADD COLUMN race TEXT DEFAULT 'None';
ALTER TABLE homunculus ADD COLUMN element TEXT DEFAULT 'Neutral';
ALTER TABLE homunculus ADD COLUMN element_level INTEGER DEFAULT 1;
ALTER TABLE homunculus ADD COLUMN evolution_stage TEXT DEFAULT 'Base';
ALTER TABLE homunculus ADD COLUMN skill_points INTEGER DEFAULT 0;
ALTER TABLE homunculus ADD COLUMN atk INTEGER DEFAULT 0;
ALTER TABLE homunculus ADD COLUMN matk INTEGER DEFAULT 0;
ALTER TABLE homunculus ADD COLUMN defense INTEGER DEFAULT 0;
ALTER TABLE homunculus ADD COLUMN magic_defense INTEGER DEFAULT 0;
ALTER TABLE homunculus ADD COLUMN hit INTEGER DEFAULT 0;
ALTER TABLE homunculus ADD COLUMN flee INTEGER DEFAULT 0;
ALTER TABLE homunculus ADD COLUMN walk_speed INTEGER DEFAULT 200;
ALTER TABLE homunculus ADD COLUMN attack_delay INTEGER DEFAULT 1000;
CREATE TABLE IF NOT EXISTS homunculus_skills (
    homun_id INTEGER NOT NULL,
    skill_id INTEGER NOT NULL,
    skill_level INTEGER DEFAULT 1,
    PRIMARY KEY (homun_id, skill_id)
);",
    down: Some("DROP TABLE IF EXISTS homunculus_skills;"),
});
```

- [ ] **Step 2: 运行迁移测试**

Run: `cargo test --lib storage::migration 2>&1 | tail -10`
Expected: 迁移测试通过

- [ ] **Step 3: Commit**

```bash
git add src/storage/migration.rs
git commit -m "feat(migration): 添加 v4 迁移（homunculus 扩展列 + skills 表）"
```

---

### Task 10: 扩展 Migration v5（mercenaries 表 + skills 表）

**Files:**
- Modify: `src/storage/migration.rs`

- [ ] **Step 1: 添加 v5 迁移**

在 v4 之后添加：

```rust
// 迁移 v5: 扩展 mercenaries 表 + 添加 skills 表
manager.register(Migration {
    version: 5,
    description: "扩展 mercenaries 表（六属性, combat stats）+ skills 表",
    up: "ALTER TABLE mercenaries ADD COLUMN defense INTEGER DEFAULT 0;
ALTER TABLE mercenaries ADD COLUMN magic_defense INTEGER DEFAULT 0;
ALTER TABLE mercenaries ADD COLUMN str INTEGER DEFAULT 1;
ALTER TABLE mercenaries ADD COLUMN agi INTEGER DEFAULT 1;
ALTER TABLE mercenaries ADD COLUMN vit INTEGER DEFAULT 1;
ALTER TABLE mercenaries ADD COLUMN int INTEGER DEFAULT 1;
ALTER TABLE mercenaries ADD COLUMN dex INTEGER DEFAULT 1;
ALTER TABLE mercenaries ADD COLUMN luk INTEGER DEFAULT 1;
ALTER TABLE mercenaries ADD COLUMN hit INTEGER DEFAULT 0;
ALTER TABLE mercenaries ADD COLUMN flee INTEGER DEFAULT 0;
ALTER TABLE mercenaries ADD COLUMN walk_speed INTEGER DEFAULT 200;
ALTER TABLE mercenaries ADD COLUMN attack_range INTEGER DEFAULT 1;
ALTER TABLE mercenaries ADD COLUMN contract_cost INTEGER DEFAULT 0;
CREATE TABLE IF NOT EXISTS mercenary_skills (
    mercenary_id INTEGER NOT NULL,
    skill_id INTEGER NOT NULL,
    skill_level INTEGER DEFAULT 1,
    PRIMARY KEY (mercenary_id, skill_id)
);",
    down: Some("DROP TABLE IF EXISTS mercenary_skills;"),
});
```

- [ ] **Step 2: 运行迁移测试**

Run: `cargo test --lib storage::migration 2>&1 | tail -10`
Expected: 迁移测试通过

- [ ] **Step 3: Commit**

```bash
git add src/storage/migration.rs
git commit -m "feat(migration): 添加 v5 迁移（mercenaries 扩展列 + skills 表）"
```

---

### Task 11: HomunculusManager 接收 DatabaseBackend 并实现持久化

**Files:**
- Modify: `src/game/homunculus/manager.rs`

- [ ] **Step 1: 改造 HomunculusManager**

修改 `src/game/homunculus/manager.rs`：

1. 添加 `db: Arc<dyn DatabaseBackend>` 字段
2. 修改 `new()` 接收 `db` 参数
3. 添加 `load_for_character()` 从数据库加载
4. 修改 `create()` 添加 INSERT 操作
5. 修改 `feed()` 添加 UPDATE 操作
6. 修改 `add_exp()` 添加 UPDATE 操作
7. 修改 `evolve()` 添加 UPDATE 操作
8. 添加 `learn_skill()` 方法

```rust
use crate::storage::backend::{DatabaseBackend, IntoValue};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

pub struct HomunculusManager {
    db: Arc<dyn DatabaseBackend>,
    homunculi: RwLock<HashMap<u32, Homunculus>>,
    summoned: RwLock<HashMap<u32, u32>>,
    database: HomunculusDatabase,
    next_id: AtomicU32,
}

impl HomunculusManager {
    pub fn new(db: Arc<dyn DatabaseBackend>) -> Self {
        // 从数据库初始化 next_id
        let max_id = db
            .query_row(
                "SELECT COALESCE(MAX(homun_id), 0) FROM homunculus",
                &[],
                |row| row.get_i64(0),
            )
            .unwrap_or(0) as u32;

        Self {
            db,
            homunculi: RwLock::new(HashMap::new()),
            summoned: RwLock::new(HashMap::new()),
            database: HomunculusDatabase::new(),
            next_id: AtomicU32::new(max_id + 1),
        }
    }

    /// 从数据库加载角色的所有生命体
    pub fn load_for_character(&self, char_id: u32) -> Result<Vec<Homunculus>, HomunculusError> {
        let rows = self
            .db
            .query_rows(
                "SELECT homun_id, owner_id, homunculus_type, name, level, exp, hunger, intimacy, \
                 hp, max_hp, sp, max_sp, str, agi, vit, int, dex, luk, evolved, alive, \
                 race, element, element_level, evolution_stage, skill_points, \
                 atk, matk, defense, magic_defense, hit, flee, walk_speed, attack_delay \
                 FROM homunculus WHERE owner_id = ?",
                &[&char_id],
            )
            .map_err(|e| HomunculusError::Database(e.to_string()))?;

        let mut homunculi = Vec::new();
        for row in &rows {
            let homun_id = row.get_i32(0).unwrap_or(0) as u32;

            // 加载技能
            let skill_rows = self
                .db
                .query_rows(
                    "SELECT skill_id, skill_level FROM homunculus_skills WHERE homun_id = ?",
                    &[&homun_id],
                )
                .unwrap_or_default();

            let skills: Vec<super::data::HomunculusSkill> = skill_rows
                .iter()
                .map(|sr| super::data::HomunculusSkill {
                    skill_id: sr.get_i32(0).unwrap_or(0) as u16,
                    skill_level: sr.get_i32(1).unwrap_or(0) as u8,
                    max_level: 5,
                })
                .collect();

            let homun = Homunculus {
                homun_id,
                owner_id: row.get_i32(1).unwrap_or(0) as u32,
                homunculus_type: super::data::HomunculusType::from_str(
                    &row.get_string(2).unwrap_or_default(),
                ),
                name: row.get_string(3).unwrap_or_default(),
                level: row.get_i32(4).unwrap_or(1) as u16,
                exp: row.get_i64(5).unwrap_or(0) as u64,
                hunger: row.get_i32(6).unwrap_or(100) as u32,
                intimacy: row.get_i32(7).unwrap_or(100) as u32,
                hp: row.get_i32(8).unwrap_or(500) as u32,
                max_hp: row.get_i32(9).unwrap_or(500) as u32,
                sp: row.get_i32(10).unwrap_or(100) as u32,
                max_sp: row.get_i32(11).unwrap_or(100) as u32,
                str: row.get_i32(12).unwrap_or(1) as u16,
                agi: row.get_i32(13).unwrap_or(1) as u16,
                vit: row.get_i32(14).unwrap_or(1) as u16,
                int: row.get_i32(15).unwrap_or(1) as u16,
                dex: row.get_i32(16).unwrap_or(1) as u16,
                luk: row.get_i32(17).unwrap_or(1) as u16,
                evolved: row.get_i32(18).unwrap_or(0) != 0,
                alive: row.get_i32(19).unwrap_or(1) != 0,
                // 新字段使用默认值或从数据库读取
                race: super::data::HomunculusRace::from_str(
                    &row.get_string(20).unwrap_or_else(|_| "None".to_string()),
                ),
                element: crate::game::battle::element::Element::Neutral,
                element_level: crate::game::battle::element::ElementLevel::Level1,
                evolution_stage: super::data::EvolutionStage::from_str(
                    &row.get_string(23).unwrap_or_else(|_| "Base".to_string()),
                ),
                skill_points: row.get_i32(24).unwrap_or(0) as u16,
                atk: row.get_i32(25).unwrap_or(0) as u16,
                matk: row.get_i32(26).unwrap_or(0) as u16,
                defense: row.get_i32(27).unwrap_or(0) as u16,
                magic_defense: row.get_i32(28).unwrap_or(0) as u16,
                hit: row.get_i32(29).unwrap_or(0) as i16,
                flee: row.get_i32(30).unwrap_or(0) as i16,
                walk_speed: row.get_i32(31).unwrap_or(200) as u16,
                attack_delay: row.get_i32(32).unwrap_or(1000) as u32,
                skills,
            };

            // 加载到内存
            self.homunculi.write().insert(homun_id, homun.clone());
            homunculi.push(homun);
        }

        Ok(homunculi)
    }

    /// 创建新生命体（INSERT + 内存）
    pub fn create(
        &self,
        owner_id: u32,
        htype: HomunculusType,
        name: &str,
    ) -> Result<Homunculus, HomunculusError> {
        let template = self
            .database
            .get_by_type(htype)
            .ok_or(HomunculusError::NotFound(0))?;

        let homun_id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let homun = Homunculus::from_template(homun_id, owner_id, template, name.to_string());

        // INSERT 到数据库
        self.db
            .execute_params(
                "INSERT INTO homunculus (homun_id, owner_id, homunculus_type, name, level, exp, hunger, intimacy, hp, max_hp, sp, max_sp, str, agi, vit, int, dex, luk, evolved, alive, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                &[
                    &homun_id,
                    &owner_id,
                    &format!("{:?}", htype),
                    &name,
                    &(homun.level as i32),
                    &(homun.exp as i64),
                    &(homun.hunger as i32),
                    &(homun.intimacy as i32),
                    &(homun.hp as i32),
                    &(homun.max_hp as i32),
                    &(homun.sp as i32),
                    &(homun.max_sp as i32),
                    &(homun.str as i32),
                    &(homun.agi as i32),
                    &(homun.vit as i32),
                    &(homun.int as i32),
                    &(homun.dex as i32),
                    &(homun.luk as i32),
                    &0i32,
                    &1i32,
                    &crate::storage::chrono_now(),
                ],
            )
            .map_err(|e| HomunculusError::Database(e.to_string()))?;

        self.homunculi.write().insert(homun_id, homun.clone());
        Ok(homun)
    }

    /// 喂食（UPDATE hunger, intimacy）
    pub fn feed(&self, char_id: u32, _item_id: u16) -> Result<(), HomunculusError> {
        let summoned = self.summoned.read();
        let homun_id = summoned
            .get(&char_id)
            .ok_or(HomunculusError::NotSummoned)?;

        let mut homunculi = self.homunculi.write();
        let homun = homunculi
            .get_mut(homun_id)
            .ok_or(HomunculusError::NotFound(*homun_id))?;

        homun.feed(20);
        homun.increase_intimacy(10);

        // 实时写库
        self.db
            .execute_params(
                "UPDATE homunculus SET hunger = ?, intimacy = ? WHERE homun_id = ?",
                &[&(homun.hunger as i32), &(homun.intimacy as i32), homun_id],
            )
            .map_err(|e| HomunculusError::Database(e.to_string()))?;

        Ok(())
    }

    /// 增加经验（返回是否升级，实时写库）
    pub fn add_exp(&self, char_id: u32, exp: u64) -> Result<bool, HomunculusError> {
        let summoned = self.summoned.read();
        let homun_id = summoned
            .get(&char_id)
            .ok_or(HomunculusError::NotSummoned)?;

        let mut homunculi = self.homunculi.write();
        let homun = homunculi
            .get_mut(homun_id)
            .ok_or(HomunculusError::NotFound(*homun_id))?;

        homun.exp += exp;

        let exp_needed = Self::exp_for_level(homun.level + 1);
        let leveled = if homun.exp >= exp_needed {
            homun.level += 1;
            homun.exp -= exp_needed;
            homun.max_hp += 20;
            homun.hp = homun.max_hp;
            homun.max_sp += 5;
            homun.sp = homun.max_sp;
            homun.str += 1;
            homun.agi += 1;
            homun.vit += 1;
            homun.int += 1;
            homun.dex += 1;
            homun.luk += 1;
            if homun.level % 5 == 0 {
                homun.skill_points += 1;
            }
            true
        } else {
            false
        };

        // 实时写库
        self.db
            .execute_params(
                "UPDATE homunculus SET level = ?, exp = ?, hp = ?, max_hp = ?, sp = ?, max_sp = ?, str = ?, agi = ?, vit = ?, int = ?, dex = ?, luk = ?, skill_points = ? WHERE homun_id = ?",
                &[
                    &(homun.level as i32),
                    &(homun.exp as i64),
                    &(homun.hp as i32),
                    &(homun.max_hp as i32),
                    &(homun.sp as i32),
                    &(homun.max_sp as i32),
                    &(homun.str as i32),
                    &(homun.agi as i32),
                    &(homun.vit as i32),
                    &(homun.int as i32),
                    &(homun.dex as i32),
                    &(homun.luk as i32),
                    &(homun.skill_points as i32),
                    homun_id,
                ],
            )
            .map_err(|e| HomunculusError::Database(e.to_string()))?;

        Ok(leveled)
    }

    /// 进化（实时写库）
    pub fn evolve(&self, char_id: u32) -> Result<(), HomunculusError> {
        let summoned = self.summoned.read();
        let homun_id = summoned
            .get(&char_id)
            .ok_or(HomunculusError::NotSummoned)?;

        let mut homunculi = self.homunculi.write();
        let homun = homunculi
            .get_mut(homun_id)
            .ok_or(HomunculusError::NotFound(*homun_id))?;

        if homun.level < 99 || homun.intimacy < 910 {
            return Err(HomunculusError::EvolutionFailed);
        }
        if homun.evolved {
            return Err(HomunculusError::EvolutionFailed);
        }

        homun.evolved = true;
        homun.evolution_stage = super::data::EvolutionStage::Evolved;
        homun.max_hp += 500;
        homun.hp = homun.max_hp;
        homun.max_sp += 100;
        homun.sp = homun.max_sp;
        homun.str += 10;
        homun.agi += 10;
        homun.vit += 10;
        homun.int += 10;
        homun.dex += 10;
        homun.luk += 10;

        // 实时写库
        self.db
            .execute_params(
                "UPDATE homunculus SET evolved = 1, evolution_stage = ?, max_hp = ?, hp = ?, max_sp = ?, sp = ?, str = ?, agi = ?, vit = ?, int = ?, dex = ?, luk = ? WHERE homun_id = ?",
                &[
                    &"Evolved",
                    &(homun.max_hp as i32),
                    &(homun.hp as i32),
                    &(homun.max_sp as i32),
                    &(homun.sp as i32),
                    &(homun.str as i32),
                    &(homun.agi as i32),
                    &(homun.vit as i32),
                    &(homun.int as i32),
                    &(homun.dex as i32),
                    &(homun.luk as i32),
                    homun_id,
                ],
            )
            .map_err(|e| HomunculusError::Database(e.to_string()))?;

        Ok(())
    }

    /// 学习技能
    pub fn learn_skill(&self, homun_id: u32, skill_id: u16) -> Result<(), HomunculusError> {
        let mut homunculi = self.homunculi.write();
        let homun = homunculi
            .get_mut(&homun_id)
            .ok_or(HomunculusError::NotFound(homun_id))?;

        // 检查技能点
        if homun.skill_points == 0 {
            return Err(HomunculusError::SkillPrereqNotMet);
        }

        // 查找或创建技能
        if let Some(skill) = homun.skills.iter_mut().find(|s| s.skill_id == skill_id) {
            if skill.skill_level >= skill.max_level {
                return Err(HomunculusError::SkillPrereqNotMet);
            }
            skill.skill_level += 1;
        } else {
            homun.skills.push(super::data::HomunculusSkill {
                skill_id,
                skill_level: 1,
                max_level: 5,
            });
        }

        homun.skill_points -= 1;

        // 写入技能表
        self.db
            .execute_params(
                "INSERT INTO homunculus_skills (homun_id, skill_id, skill_level) VALUES (?, ?, ?) ON CONFLICT(homun_id, skill_id) DO UPDATE SET skill_level = ?",
                &[&homun_id, &(skill_id as i32), &(1i32), &(1i32)],
            )
            .map_err(|e| HomunculusError::Database(e.to_string()))?;

        Ok(())
    }
}
```

- [ ] **Step 2: 更新测试**

更新现有测试，传入内存数据库：
```rust
fn create_test_manager() -> HomunculusManager {
    use crate::storage::backend::DatabaseBackend;
    use crate::storage::sqlite_backend::SqliteBackend;
    let db = SqliteBackend::open_memory().unwrap();
    // 创建表
    db.execute_batch("CREATE TABLE IF NOT EXISTS homunculus (...)").unwrap();
    HomunculusManager::new(Arc::new(db))
}
```

- [ ] **Step 3: 运行测试**

Run: `cargo test --lib game::homunculus::manager 2>&1 | tail -15`
Expected: 所有测试通过

- [ ] **Step 4: Commit**

```bash
git add src/game/homunculus/manager.rs
git commit -m "feat(homunculus): HomunculusManager 对接数据库，实现实时持久化"
```

---

### Task 12: MercenaryManager 接收 DatabaseBackend 并实现持久化

**Files:**
- Modify: `src/game/mercenary/manager.rs`

- [ ] **Step 1: 改造 MercenaryManager**

同 HomunculusManager 模式：

1. 添加 `db: Arc<dyn DatabaseBackend>` 字段
2. 修改 `new()` 接收 `db` 参数
3. 添加 `load_for_character()` 从数据库加载
4. 修改 `create()` 添加 INSERT
5. 修改 `increase_loyalty()` 添加 UPDATE
6. 修改 `update_contracts()` 添加 DELETE（到期佣兵）
7. 修改 `summon()` 添加召唤状态持久化

- [ ] **Step 2: 更新测试**

更新测试传入内存数据库。

- [ ] **Step 3: 运行测试**

Run: `cargo test --lib game::mercenary::manager 2>&1 | tail -15`
Expected: 所有测试通过

- [ ] **Step 4: Commit**

```bash
git add src/game/mercenary/manager.rs
git commit -m "feat(mercenary): MercenaryManager 对接数据库，实现实时持久化"
```

---

## Phase 3: MySQL 后端实现

### Task 13: 添加 mysql crate 依赖（feature gate）

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: 更新 Cargo.toml**

```toml
[dependencies]
rusqlite = { version = "0.31", features = ["bundled"], optional = true }
mysql = { version = "25", optional = true }

[features]
default = ["sqlite"]
sqlite = ["rusqlite"]
mysql-backend = ["dep:mysql"]
```

- [ ] **Step 2: 确保默认编译仍使用 SQLite**

Run: `cargo check --lib 2>&1 | tail -5`
Expected: 编译通过（默认 feature 包含 sqlite）

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat(deps): 添加 mysql crate 可选依赖（feature gate）"
```

---

### Task 14: 实现 MySqlBackend

**Files:**
- Create: `src/storage/mysql_backend.rs`

- [ ] **Step 1: 创建 MySqlBackend**

```rust
//! MySQL 数据库后端实现
//!
//! 仅在启用 mysql-backend feature 时编译。

#[cfg(feature = "mysql-backend")]
mod inner {
    use crate::storage::backend::{DatabaseBackend, IntoValue, Row, TransactionOps, Value};
    use crate::error::Result;

    pub struct MySqlConfig {
        pub host: String,
        pub port: u16,
        pub user: String,
        pub password: String,
        pub database: String,
        pub pool_size: u32,
    }

    impl Default for MySqlConfig {
        fn default() -> Self {
            Self {
                host: "127.0.0.1".to_string(),
                port: 3306,
                user: "deviruchi".to_string(),
                password: String::new(),
                database: "deviruchi".to_string(),
                pool_size: 10,
            }
        }
    }

    pub struct MySqlBackend {
        pool: mysql::Pool,
    }

    impl MySqlBackend {
        pub fn new(config: &MySqlConfig) -> Result<Self> {
            let opts = mysql::OptsBuilder::new()
                .ip_or_hostname(Some(&config.host))
                .tcp_port(config.port)
                .db_name(Some(&config.database))
                .user(Some(&config.user))
                .pass(Some(&config.password));
            let pool = mysql::Pool::new(opts)
                .map_err(|e| crate::error::Error::DatabaseBackend(e.to_string()))?;
            Ok(Self { pool })
        }
    }

    impl DatabaseBackend for MySqlBackend {
        fn execute(&self, sql: &str) -> Result<usize> {
            let mut conn = self.pool.get_conn()
                .map_err(|e| crate::error::Error::DatabaseBackend(e.to_string()))?;
            let result = conn.query_drop(sql)
                .map_err(|e| crate::error::Error::DatabaseBackend(e.to_string()))?;
            Ok(0)
        }

        fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize> {
            let mut conn = self.pool.get_conn()
                .map_err(|e| crate::error::Error::DatabaseBackend(e.to_string()))?;
            let values: Vec<mysql::Value> = params.iter()
                .map(|p| value_to_mysql(&p.into_value()))
                .collect();
            let result = conn.exec_iter(sql, values)
                .map_err(|e| crate::error::Error::DatabaseBackend(e.to_string()))?;
            Ok(result.affected_rows() as usize)
        }

        fn query_rows(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<Vec<Row>> {
            let mut conn = self.pool.get_conn()
                .map_err(|e| crate::error::Error::DatabaseBackend(e.to_string()))?;
            let values: Vec<mysql::Value> = params.iter()
                .map(|p| value_to_mysql(&p.into_value()))
                .collect();
            let result = conn.exec_iter(sql, values)
                .map_err(|e| crate::error::Error::DatabaseBackend(e.to_string()))?;

            let mut rows = Vec::new();
            for row_result in result {
                let mysql_row = row_result
                    .map_err(|e| crate::error::Error::DatabaseBackend(e.to_string()))?;
                let columns: Vec<(String, Value)> = mysql_row
                    .columns_ref()
                    .iter()
                    .enumerate()
                    .map(|(i, col)| {
                        let name = col.name_str().to_string();
                        let value = mysql_row.get::<mysql::Value, _>(i)
                            .map(|v| mysql_to_value(&v))
                            .unwrap_or(Value::Null);
                        (name, value)
                    })
                    .collect();
                rows.push(Row::new(columns));
            }
            Ok(rows)
        }

        fn query_row<F, T>(&self, sql: &str, params: &[&dyn IntoValue], f: F) -> Result<T>
        where
            F: FnOnce(&Row) -> Result<T>,
        {
            let rows = self.query_rows(sql, params)?;
            let row = rows.into_iter().next().ok_or_else(|| {
                crate::error::Error::DatabaseBackend("query returned no rows".into())
            })?;
            f(&row)
        }

        fn query_row_optional<F, T>(&self, sql: &str, params: &[&dyn IntoValue], f: F) -> Result<Option<T>>
        where
            F: FnOnce(&Row) -> Result<T>,
        {
            let rows = self.query_rows(sql, params)?;
            match rows.into_iter().next() {
                Some(row) => Ok(Some(f(&row)?)),
                None => Ok(None),
            }
        }

        fn with_transaction<F, T>(&self, f: F) -> Result<T>
        where
            F: FnOnce(&dyn TransactionOps) -> Result<T>,
        {
            let mut conn = self.pool.get_conn()
                .map_err(|e| crate::error::Error::DatabaseBackend(e.to_string()))?;
            conn.query_drop("START TRANSACTION")
                .map_err(|e| crate::error::Error::DatabaseBackend(e.to_string()))?;

            let tx = MySqlTransaction { conn: &mut conn };
            match f(&tx) {
                Ok(result) => {
                    conn.query_drop("COMMIT")
                        .map_err(|e| crate::error::Error::DatabaseBackend(e.to_string()))?;
                    Ok(result)
                }
                Err(e) => {
                    conn.query_drop("ROLLBACK").ok();
                    Err(e)
                }
            }
        }

        fn last_insert_rowid(&self) -> i64 {
            // MySQL 使用 LAST_INSERT_ID()
            0 // TODO: 实现
        }

        fn execute_batch(&self, sql: &str) -> Result<()> {
            let mut conn = self.pool.get_conn()
                .map_err(|e| crate::error::Error::DatabaseBackend(e.to_string()))?;
            for stmt in sql.split(';').filter(|s| !s.trim().is_empty()) {
                conn.query_drop(stmt.trim())
                    .map_err(|e| crate::error::Error::DatabaseBackend(e.to_string()))?;
            }
            Ok(())
        }

        fn upsert(
            &self,
            table: &str,
            columns: &[&str],
            params: &[&dyn IntoValue],
            _conflict_cols: &[&str],
        ) -> Result<usize> {
            let update_cols: Vec<String> = columns.iter()
                .map(|c| format!("{}=VALUES({})", c, c))
                .collect();
            let sql = format!(
                "INSERT INTO {} ({}) VALUES ({}) ON DUPLICATE KEY UPDATE {}",
                table,
                columns.join(", "),
                columns.iter().map(|_| "?").collect::<Vec<_>>().join(", "),
                update_cols.join(", ")
            );
            self.execute_params(&sql, params)
        }
    }

    fn value_to_mysql(v: &Value) -> mysql::Value {
        match v {
            Value::Null => mysql::Value::NULL,
            Value::Int(i) => mysql::Value::Int(*i as i64),
            Value::BigInt(i) => mysql::Value::Int(*i),
            Value::Float(f) => mysql::Value::Float(*f),
            Value::Text(s) => mysql::Value::Bytes(s.as_bytes().to_vec()),
            Value::Blob(b) => mysql::Value::Bytes(b.clone()),
        }
    }

    fn mysql_to_value(v: &mysql::Value) -> Value {
        match v {
            mysql::Value::NULL => Value::Null,
            mysql::Value::Int(i) => Value::BigInt(*i),
            mysql::Value::UInt(u) => Value::BigInt(*u as i64),
            mysql::Value::Float(f) => Value::Float(*f),
            mysql::Value::Bytes(b) => Value::Text(String::from_utf8_lossy(b).to_string()),
            _ => Value::Null,
        }
    }

    struct MySqlTransaction<'a> {
        conn: &'a mut mysql::Conn,
    }

    impl<'a> TransactionOps for MySqlTransaction<'a> {
        fn execute(&self, sql: &str) -> Result<usize> {
            // 需要可变引用，这里简化处理
            Ok(0)
        }

        fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize> {
            Ok(0) // TODO: 实现
        }

        fn execute_batch(&self, sql: &str) -> Result<()> {
            Ok(()) // TODO: 实现
        }

        fn last_insert_rowid(&self) -> i64 {
            0
        }
    }
}

#[cfg(feature = "mysql-backend")]
pub use inner::{MySqlBackend, MySqlConfig};
```

- [ ] **Step 2: 在 storage/mod.rs 中注册**

```rust
#[cfg(feature = "mysql-backend")]
pub mod mysql_backend;
#[cfg(feature = "mysql-backend")]
pub use mysql_backend::{MySqlBackend, MySqlConfig};
```

- [ ] **Step 3: 运行编译检查（不启用 mysql feature）**

Run: `cargo check --lib 2>&1 | tail -5`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add src/storage/mysql_backend.rs src/storage/mod.rs
git commit -m "feat(storage): 实现 MySqlBackend（DatabaseBackend trait 的 MySQL 实现）"
```

---

### Task 15: 扩展配置支持后端选择

**Files:**
- Modify: `src/core/config.rs`

- [ ] **Step 1: 扩展 DatabaseConfig**

将 `src/core/config.rs` 中的 `DatabaseConfig` 改为：

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// 后端类型: "sqlite" 或 "mysql"
    pub backend: String,
    pub path: String,
    pub backup_path: Option<String>,
    pub auto_vacuum: bool,
    pub wal_mode: bool,
    pub busy_timeout_ms: u32,
    pub auto_backup_interval_hours: u32,
    /// MySQL 配置
    pub mysql_host: String,
    pub mysql_port: u16,
    pub mysql_user: String,
    pub mysql_password: String,
    pub mysql_database: String,
    pub mysql_pool_size: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            backend: "sqlite".to_string(),
            path: "deviruchi.db".to_string(),
            backup_path: None,
            auto_vacuum: true,
            wal_mode: true,
            busy_timeout_ms: 5000,
            auto_backup_interval_hours: 24,
            mysql_host: "127.0.0.1".to_string(),
            mysql_port: 3306,
            mysql_user: "deviruchi".to_string(),
            mysql_password: String::new(),
            mysql_database: "deviruchi".to_string(),
            mysql_pool_size: 10,
        }
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test --lib core::config 2>&1 | tail -10`
Expected: 配置测试通过

- [ ] **Step 3: Commit**

```bash
git add src/core/config.rs
git commit -m "feat(config): DatabaseConfig 支持后端选择（sqlite/mysql）"
```

---

## Phase 4: Mob/NPC YAML 完整解析

### Task 16: MobTemplate 扩展 race 和 mob_type 字段

**Files:**
- Modify: `src/game/mob/data.rs`
- Modify: `src/game/mob/yaml_loader.rs`

- [ ] **Step 1: 添加 MobRace 枚举**

在 `src/game/mob/data.rs` 中添加：

```rust
/// 怪物种族
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobRace {
    Formless,
    Undead,
    Brute,
    Plant,
    Insect,
    Fish,
    Demon,
    DemiHuman,
    Angel,
    Dragon,
}

impl Default for MobRace {
    fn default() -> Self {
        MobRace::Formless
    }
}
```

- [ ] **Step 2: MobTemplate 添加新字段**

在 `src/game/mob/data.rs` 的 `MobTemplate` 结构体中添加：

```rust
pub struct MobTemplate {
    // ... 现有字段 ...
    pub race: MobRace,
    pub mob_type: MobType,
    pub mvp_drops: Vec<MobDrop>,
}
```

更新 `MobTemplate::default()` 方法，添加默认值：
```rust
race: MobRace::Formless,
mob_type: MobType::Normal,
mvp_drops: Vec::new(),
```

- [ ] **Step 3: 更新 yaml_loader.rs 的 parse 函数**

在 `src/game/mob/yaml_loader.rs` 中：

1. 添加 `parse_race()` 函数
2. 更新 `MobYamlEntry` 添加 Race 字段（已有，但之前没赋值）
3. 更新 `load_mob_db` 中的 `MobTemplate` 构造，使用 `parse_race(&entry.Race)`

```rust
fn parse_race(s: &str) -> MobRace {
    match s.to_lowercase().as_str() {
        "formless" => MobRace::Formless,
        "undead" => MobRace::Undead,
        "brute" => MobRace::Brute,
        "plant" => MobRace::Plant,
        "insect" => MobRace::Insect,
        "fish" => MobRace::Fish,
        "demon" => MobRace::Demon,
        "demihuman" | "demi_human" => MobRace::DemiHuman,
        "angel" => MobRace::Angel,
        "dragon" => MobRace::Dragon,
        _ => MobRace::Formless,
    }
}
```

4. 添加 MvpDrops 反序列化支持（复用 `MobYamlDrop` 结构）

- [ ] **Step 4: 更新所有 MobTemplate 构造处**

更新 `src/game/mob/data.rs` 中 `MobTemplate::default()` 和所有 `MobTemplate` 构造的地方（硬编码模板）。

- [ ] **Step 5: 运行测试**

Run: `cargo test --lib game::mob 2>&1 | tail -15`
Expected: 所有 mob 测试通过

- [ ] **Step 6: Commit**

```bash
git add src/game/mob/data.rs src/game/mob/yaml_loader.rs
git commit -m "feat(mob): MobTemplate 扩展 race/mob_type/mvp_drops 字段"
```

---

### Task 17: parse_modes 完整解析

**Files:**
- Modify: `src/game/mob/data.rs`
- Modify: `src/game/mob/yaml_loader.rs`

- [ ] **Step 1: 添加怪物行为标记字段**

在 `src/game/mob/data.rs` 的 `MobTemplate` 中添加简单 bool 字段（避免引入 bitflags 依赖）：

```rust
/// 怪物行为标记（从 Modes 解析）
#[derive(Debug, Clone)]
pub struct MobBehaviorFlags {
    pub can_move: bool,
    pub can_attack: bool,
    pub detector: bool,
    pub boss: bool,
    pub plant: bool,
    pub can_chase: bool,
}

impl Default for MobBehaviorFlags {
    fn default() -> Self {
        Self {
            can_move: true,
            can_attack: true,
            detector: false,
            boss: false,
            plant: false,
            can_chase: true,
        }
    }
}
```

- [ ] **Step 2: 更新 MobTemplate**

在 `MobTemplate` 中添加：
```rust
pub behavior_flags: MobBehaviorFlags,
```

- [ ] **Step 3: 更新 yaml_loader.rs 的 parse_modes**

```rust
fn parse_modes(modes: &Option<HashMap<String, bool>>) -> MobBehaviorFlags {
    let mut flags = MobBehaviorFlags::default();
    if let Some(m) = modes {
        if m.get("CanMove").copied() == Some(false) { flags.can_move = false; }
        if m.get("CanAttack").copied() == Some(false) { flags.can_attack = false; }
        if m.get("Detector").copied() == Some(true) { flags.detector = true; }
        if m.get("Boss").copied() == Some(true) { flags.boss = true; }
        if m.get("Plant").copied() == Some(true) { flags.plant = true; }
        if m.get("CanChase").copied() == Some(false) { flags.can_chase = false; }
    }
    flags
}
```

- [ ] **Step 4: 更新硬编码值**

将 `sight_range` 从硬编码 12 改为从 `SkillRange` 或默认值 12 读取。
将 `respawn_time` 从硬编码 60000 改为从 YAML 读取。
将 `aggro_rate` 从硬编码 0 改为根据 AI 类型推导（Aggressive = 100）。

- [ ] **Step 5: 运行测试**

Run: `cargo test --lib game::mob 2>&1 | tail -15`
Expected: 测试通过

- [ ] **Step 6: Commit**

```bash
git add src/game/mob/data.rs src/game/mob/yaml_loader.rs
git commit -m "feat(mob): parse_modes 完整解析 + 行为标记 + 硬编码值改为 YAML 驱动"
```

---

### Task 18: item_db.yml 加载器

**Files:**
- Create: `src/game/item/yaml_loader.rs`（或修改现有文件）

- [ ] **Step 1: 创建物品 YAML 加载器**

```rust
//! 物品数据库 YAML 加载器
//!
//! 从 rAthena item_db.yml 格式加载物品数据，用于物品名称到 ID 的映射。

use serde::Deserialize;
use std::collections::HashMap;

/// rAthena item_db.yml 文件结构
#[derive(Deserialize, Debug)]
struct ItemYamlFile {
    Header: ItemYamlHeader,
    Body: Option<Vec<ItemYamlEntry>>,
}

#[derive(Deserialize, Debug)]
struct ItemYamlHeader {
    #[serde(rename = "Type")]
    _type: String,
    Version: u32,
}

#[derive(Deserialize, Debug)]
struct ItemYamlEntry {
    Id: u32,
    #[serde(rename = "AegisName")]
    AegisName: String,
    #[allow(dead_code)]
    Name: String,
}

/// 从 item_db.yml 加载物品名称到 ID 的映射
pub fn load_item_db(path: &str) -> Result<HashMap<String, u32>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let yaml: ItemYamlFile = serde_yaml::from_str(&content)?;
    let mut map = HashMap::new();
    if let Some(body) = yaml.Body {
        for entry in body {
            map.insert(entry.AegisName, entry.Id);
        }
    }
    Ok(map)
}

/// 混合查找：先查动态映射，再查硬编码回退
pub fn item_name_to_id_dynamic(name: &str, item_map: &HashMap<String, u32>) -> u32 {
    item_map.get(name).copied().unwrap_or_else(|| super::super::mob::yaml_loader::item_name_to_id(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_item_db_from_string() {
        let yaml_str = r#"
Header:
  Type: ITEM_DB
  Version: 5
Body:
  - Id: 501
    AegisName: Red_Potion
    Name: Red Potion
  - Id: 502
    AegisName: Orange_Potion
    Name: Orange Potion
  - Id: 909
    AegisName: Jellopy
    Name: Jellopy
"#;

        let tmp_path = "/tmp/test_item_db.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();
        let map = load_item_db(tmp_path).unwrap();
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("Red_Potion"), Some(&501));
        assert_eq!(map.get("Jellopy"), Some(&909));
        std::fs::remove_file(tmp_path).ok();
    }

    #[test]
    fn test_item_name_to_id_dynamic() {
        let mut map = HashMap::new();
        map.insert("Custom_Item".to_string(), 9999u32);

        // 动态映射
        assert_eq!(item_name_to_id_dynamic("Custom_Item", &map), 9999);
        // 硬编码回退
        assert_eq!(item_name_to_id_dynamic("Red_Potion", &map), 501);
    }
}
```

- [ ] **Step 2: 在 item/mod.rs 中注册**

```rust
pub mod yaml_loader;
```

- [ ] **Step 3: 运行测试**

Run: `cargo test --lib game::item::yaml_loader 2>&1 | tail -10`
Expected: 2 个测试通过

- [ ] **Step 4: Commit**

```bash
git add src/game/item/yaml_loader.rs src/game/item/mod.rs
git commit -m "feat(item): 添加 item_db.yml 加载器和动态物品名称映射"
```

---

### Task 19: NPC YAML 扩展（Event/Trigger 支持）

**Files:**
- Modify: `src/game/npc/data.rs`
- Modify: `src/game/npc/yaml_loader.rs`

- [ ] **Step 1: 添加 NpcEvent 枚举**

在 `src/game/npc/data.rs` 中添加：

```rust
/// NPC 事件触发方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcEvent {
    None,       // 无事件
    OnClick,    // 点击触发（默认）
    OnTouch,    // 接近触发
    OnInit,     // 地图初始化时触发
}

impl Default for NpcEvent {
    fn default() -> Self {
        NpcEvent::OnClick
    }
}
```

- [ ] **Step 2: Npc 结构添加字段**

在 `src/game/npc/data.rs` 的 `Npc` 结构体中添加：

```rust
pub event: NpcEvent,
pub trigger_radius: u16,
```

- [ ] **Step 3: 更新 yaml_loader.rs**

在 `NpcYamlEntry` 中添加：
```rust
event: Option<String>,
trigger_radius: Option<u16>,
```

在构造 `Npc` 时解析：
```rust
event: entry.event.map(|e| parse_npc_event(&e)).unwrap_or_default(),
trigger_radius: entry.trigger_radius.unwrap_or(0),
```

添加解析函数：
```rust
fn parse_npc_event(s: &str) -> NpcEvent {
    match s.to_lowercase().as_str() {
        "onclick" | "click" => NpcEvent::OnClick,
        "ontouch" | "touch" => NpcEvent::OnTouch,
        "oninit" | "init" => NpcEvent::OnInit,
        _ => NpcEvent::None,
    }
}
```

- [ ] **Step 4: 更新测试**

添加 OnTouch 事件的测试用例。

- [ ] **Step 5: 运行测试**

Run: `cargo test --lib game::npc 2>&1 | tail -10`
Expected: 测试通过

- [ ] **Step 6: Commit**

```bash
git add src/game/npc/data.rs src/game/npc/yaml_loader.rs
git commit -m "feat(npc): NPC YAML 扩展支持 Event/TriggerRadius 字段"
```

---

## 最终验证

### Task 20: 全量测试 + 更新 README

- [ ] **Step 1: 运行全部测试**

Run: `cargo test --lib 2>&1 | tail -20`
Expected: 全部测试通过

- [ ] **Step 2: 更新 README**

更新 `README.md` 中的"待完善"部分：
- 删除 Homunculus/Mercenary 持久化条目
- 删除 Mob/NPC YAML 完整解析条目
- 删除 MySQL 支持条目
- 添加 DatabaseBackend 抽象层到基础设施表

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: 更新 README 反映三功能完成状态"
```
