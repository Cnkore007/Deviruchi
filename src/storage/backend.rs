//! 数据库后端抽象层
//!
//! 提供 Backend enum 和相关类型（Value、Row、TransactionOps），
//! 使上层代码与具体数据库引擎（SQLite、MySQL）解耦。

use crate::error::Result;
#[cfg(feature = "mysql-backend")]
use crate::storage::mysql_backend::MySqlBackend;
use crate::storage::sqlite_backend::SqliteBackend;

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
///
/// 以列名+值对的有序列表存储一行数据，
/// 通过列索引（usize）访问各列值。
#[derive(Debug)]
pub struct Row {
    columns: Vec<(String, Value)>,
}

impl Row {
    pub fn new(columns: Vec<(String, Value)>) -> Self {
        Self { columns }
    }

    /// 获取指定列的 i32 值
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

    /// 获取指定列的 i64 值
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

    /// 获取指定列的 f64 值
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

    /// 获取指定列的 String 值
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

    /// 获取指定列的可选 String 值（NULL 返回 None）
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

    /// 获取指定列的可选 i64 值（NULL 返回 None）
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
///
/// 在 with_transaction 闭包中使用，提供 execute/execute_params/execute_batch/last_insert_rowid。
/// 线程安全由 Backend 的 with_transaction 方法保证（事务期间持有锁）。
pub trait TransactionOps {
    fn execute(&self, sql: &str) -> Result<usize>;
    fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize>;
    fn execute_batch(&self, sql: &str) -> Result<()>;
    fn last_insert_rowid(&self) -> i64;
}

/// 值转换 trait，替代 rusqlite::ToSql
///
/// 为常用类型实现此 trait，使上层代码无需依赖 rusqlite 类型。
pub trait IntoValue {
    fn to_value(&self) -> Value;
}

impl IntoValue for i32 {
    fn to_value(&self) -> Value {
        Value::Int(*self)
    }
}

impl IntoValue for u32 {
    fn to_value(&self) -> Value {
        Value::Int(*self as i32)
    }
}

impl IntoValue for i64 {
    fn to_value(&self) -> Value {
        Value::BigInt(*self)
    }
}

impl IntoValue for u64 {
    fn to_value(&self) -> Value {
        Value::BigInt(*self as i64)
    }
}

impl IntoValue for f64 {
    fn to_value(&self) -> Value {
        Value::Float(*self)
    }
}

impl IntoValue for str {
    fn to_value(&self) -> Value {
        Value::Text(self.to_string())
    }
}

impl IntoValue for String {
    fn to_value(&self) -> Value {
        Value::Text(self.clone())
    }
}

impl IntoValue for &str {
    fn to_value(&self) -> Value {
        Value::Text(self.to_string())
    }
}

impl<T: IntoValue> IntoValue for Option<T> {
    fn to_value(&self) -> Value {
        match self {
            Some(v) => v.to_value(),
            None => Value::Null,
        }
    }
}

/// 数据库后端枚举
///
/// 使用 enum dispatch 代替 dyn trait，避免 dyn 兼容性问题。
/// 每个变体对应一种数据库引擎实现。
pub enum Backend {
    Sqlite(SqliteBackend),
    /// MySQL 后端，仅在启用 `mysql-backend` feature 时可用
    #[cfg(feature = "mysql-backend")]
    MySql(MySqlBackend),
}

impl Backend {
    /// 执行无参数 SQL
    pub fn execute(&self, sql: &str) -> Result<usize> {
        match self {
            Backend::Sqlite(b) => b.execute(sql),
            #[cfg(feature = "mysql-backend")]
            Backend::MySql(b) => b.execute(sql),
        }
    }

    /// 带参数执行 SQL，返回影响行数
    pub fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize> {
        match self {
            Backend::Sqlite(b) => b.execute_params(sql, params),
            #[cfg(feature = "mysql-backend")]
            Backend::MySql(b) => b.execute_params(sql, params),
        }
    }

    /// 查询多行
    pub fn query_rows(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<Vec<Row>> {
        match self {
            Backend::Sqlite(b) => b.query_rows(sql, params),
            #[cfg(feature = "mysql-backend")]
            Backend::MySql(b) => b.query_rows(sql, params),
        }
    }

    /// 最后插入的行 ID
    pub fn last_insert_rowid(&self) -> i64 {
        match self {
            Backend::Sqlite(b) => b.last_insert_rowid(),
            #[cfg(feature = "mysql-backend")]
            Backend::MySql(b) => b.last_insert_rowid(),
        }
    }

    /// 批量执行（用于迁移、schema 初始化等）
    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        match self {
            Backend::Sqlite(b) => b.execute_batch(sql),
            #[cfg(feature = "mysql-backend")]
            Backend::MySql(b) => b.execute_batch(sql),
        }
    }

    /// UPSERT 便捷方法
    pub fn upsert(
        &self,
        table: &str,
        columns: &[&str],
        params: &[&dyn IntoValue],
        conflict_cols: &[&str],
    ) -> Result<usize> {
        match self {
            Backend::Sqlite(b) => b.upsert(table, columns, params, conflict_cols),
            #[cfg(feature = "mysql-backend")]
            Backend::MySql(b) => b.upsert(table, columns, params, conflict_cols),
        }
    }

    /// 在事务中执行操作，成功提交，失败回滚
    pub fn with_transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn TransactionOps) -> Result<T>,
    {
        match self {
            Backend::Sqlite(b) => b.with_transaction(f),
            #[cfg(feature = "mysql-backend")]
            Backend::MySql(b) => b.with_transaction(f),
        }
    }
}
