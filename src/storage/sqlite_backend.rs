//! SQLite 数据库后端实现
//!
//! 封装 rusqlite::Connection，通过 parking_lot::Mutex 实现线程安全。
//! 事务期间持有锁，通过 ScopedTx 传递操作。

use super::backend::{DatabaseBackend, IntoValue, Row, TransactionOps, Value};
use crate::error::Result;
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

/// SQLite 数据库后端
///
/// rusqlite::Connection 的 execute/prepare/execute_batch 均接受 &self，
/// 所以可以用 parking_lot::Mutex 包装。事务期间持有锁，通过 ScopedTx
/// 传递操作。
pub struct SqliteBackend {
    conn: Arc<parking_lot::Mutex<Connection>>,
}

// rusqlite::Connection 内部使用 RefCell，不自动满足 Send/Sync。
// 但我们通过 parking_lot::Mutex 保证同一时刻只有一个线程访问连接，
// 所以可以安全地声明 Send + Sync（与现有 Database 结构做法一致）。
unsafe impl Send for SqliteBackend {}
unsafe impl Sync for SqliteBackend {}

impl SqliteBackend {
    /// 根据配置创建 SQLite 后端
    pub fn new(config: &SqliteConfig) -> Result<Self> {
        let conn = Connection::open(&config.path)?;
        if config.wal_mode {
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        }
        conn.busy_timeout(Duration::from_millis(config.busy_timeout_ms as u64))?;
        Ok(Self {
            conn: Arc::new(parking_lot::Mutex::new(conn)),
        })
    }

    /// 创建内存数据库后端（测试用）
    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Ok(Self {
            conn: Arc::new(parking_lot::Mutex::new(conn)),
        })
    }

    /// 根据路径创建 SQLite 后端（使用默认配置）
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = SqliteConfig {
            path: path.as_ref().to_string_lossy().to_string(),
            ..Default::default()
        };
        Self::new(&config)
    }

    /// 将 IntoValue 参数转换为 rusqlite 可用的参数并执行
    fn execute_with_rusqlite_params(
        conn: &Connection,
        sql: &str,
        params: &[&dyn IntoValue],
    ) -> Result<usize> {
        let values: Vec<Value> = params.iter().map(|p| p.into_value()).collect();
        let rusqlite_params: Vec<Box<dyn rusqlite::types::ToSql>> =
            values.iter().map(|v| v.to_rusqlite()).collect();
        let mut stmt = conn.prepare(sql)?;
        Ok(stmt.execute(rusqlite::params_from_iter(
            rusqlite_params.iter().map(|p| p.as_ref()),
        ))?)
    }

    /// 将 IntoValue 参数转换为 rusqlite 可用的参数并查询多行
    fn query_rows_with_rusqlite_params(
        conn: &Connection,
        sql: &str,
        params: &[&dyn IntoValue],
    ) -> Result<Vec<Row>> {
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
}

impl DatabaseBackend for SqliteBackend {
    fn execute(&self, sql: &str) -> Result<usize> {
        let conn = self.conn.lock();
        Ok(conn.execute(sql, [])?)
    }

    fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize> {
        let conn = self.conn.lock();
        Self::execute_with_rusqlite_params(&conn, sql, params)
    }

    fn query_rows(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<Vec<Row>> {
        let conn = self.conn.lock();
        Self::query_rows_with_rusqlite_params(&conn, sql, params)
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
        let conn = self.conn.lock();
        conn.execute_batch("BEGIN IMMEDIATE")?;

        // ScopedTx 通过引用持有连接，实现 TransactionOps
        struct ScopedTx<'a> {
            conn: &'a rusqlite::Connection,
        }

        impl<'a> TransactionOps for ScopedTx<'a> {
            fn execute(&self, sql: &str) -> Result<usize> {
                Ok(self.conn.execute(sql, [])?)
            }

            fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize> {
                SqliteBackend::execute_with_rusqlite_params(self.conn, sql, params)
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
