//! 数据库操作入口
//!
//! 提供统一的数据库操作接口，通过 Backend enum 支持多种后端。
//! 消除 rusqlite 类型泄漏，上层代码仅依赖 IntoValue/Row/TransactionOps。

use crate::error::Result;
use crate::storage::backend::{Backend, IntoValue, Row, TransactionOps};
use crate::storage::sqlite_backend::{SqliteBackend, SqliteConfig};
use std::sync::Arc;

/// 数据库操作入口
///
/// 内部持有 Arc<Backend>，Backend 为 enum dispatch，
/// 避免 dyn 兼容性问题，同时保持零成本抽象。
pub struct Database {
    backend: Arc<Backend>,
}

impl Database {
    /// 通过指定后端创建数据库实例
    pub fn new(backend: Backend) -> Self {
        Self {
            backend: Arc::new(backend),
        }
    }

    /// 获取后端引用
    pub fn backend(&self) -> &Backend {
        self.backend.as_ref()
    }

    /// 打开 SQLite 数据库文件
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let config = SqliteConfig {
            path: path.as_ref().to_string_lossy().to_string(),
            ..Default::default()
        };
        let backend = SqliteBackend::new(&config)?;
        Ok(Self::new(Backend::Sqlite(backend)))
    }

    /// 打开内存数据库（测试用）
    pub fn open_memory() -> Result<Self> {
        let backend = SqliteBackend::open_memory()?;
        Ok(Self::new(Backend::Sqlite(backend)))
    }

    /// 执行无参数 SQL
    pub fn execute(&self, sql: &str) -> Result<usize> {
        self.backend.execute(sql)
    }

    /// 带参数执行 SQL，返回影响行数
    pub fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize> {
        self.backend.execute_params(sql, params)
    }

    /// 查询多行
    pub fn query_rows(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<Vec<Row>> {
        self.backend.query_rows(sql, params)
    }

    /// 查询单行
    pub fn query_row<F, T>(&self, sql: &str, params: &[&dyn IntoValue], f: F) -> Result<T>
    where
        F: FnOnce(&Row) -> Result<T>,
    {
        let rows = self.backend.query_rows(sql, params)?;
        let row = rows
            .into_iter()
            .next()
            .ok_or_else(|| crate::error::Error::DatabaseBackend("query returned no rows".into()))?;
        f(&row)
    }

    /// 查询可选单行
    pub fn query_row_optional<F, T>(
        &self,
        sql: &str,
        params: &[&dyn IntoValue],
        f: F,
    ) -> Result<Option<T>>
    where
        F: FnOnce(&Row) -> Result<T>,
    {
        let rows = self.backend.query_rows(sql, params)?;
        match rows.into_iter().next() {
            Some(row) => Ok(Some(f(&row)?)),
            None => Ok(None),
        }
    }

    /// 在事务中执行操作，成功提交，失败回滚
    pub fn with_transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn TransactionOps) -> Result<T>,
    {
        self.backend.with_transaction(f)
    }

    /// 最后插入的行 ID
    pub fn last_insert_rowid(&self) -> i64 {
        self.backend.last_insert_rowid()
    }

    /// 批量执行（用于迁移、schema 初始化等）
    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        self.backend.execute_batch(sql)
    }

    /// UPSERT 便捷方法
    pub fn upsert(
        &self,
        table: &str,
        columns: &[&str],
        params: &[&dyn IntoValue],
        conflict_cols: &[&str],
    ) -> Result<usize> {
        self.backend.upsert(table, columns, params, conflict_cols)
    }

    /// 备份数据库到指定路径（通过 VACUUM INTO 实现在线热备份）
    pub fn backup(&self, dest_path: &str) -> Result<()> {
        let sql = format!("VACUUM INTO '{}'", dest_path.replace('\'', "\'\'"));
        self.backend.execute_batch(&sql)?;
        tracing::info!("数据库备份完成: {}", dest_path);
        Ok(())
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
