//! 数据库后端抽象层
//!
//! 提供 DatabaseBackend trait 及其关联类型（Value、Row、TransactionOps），
//! 使上层代码与具体数据库引擎（SQLite、MySQL）解耦。

use crate::error::Result;

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
/// 线程安全由 DatabaseBackend 的 with_transaction 方法保证（事务期间持有锁）。
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
///
/// 统一的数据库操作接口，支持 SQLite、MySQL 等多种后端。
pub trait DatabaseBackend: Send + Sync + 'static {
    /// 执行 SQL（无参数），返回影响行数
    fn execute(&self, sql: &str) -> Result<usize>;

    /// 带参数执行 SQL，返回影响行数
    fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize>;

    /// 查询多行
    fn query_rows(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<Vec<Row>>;

    /// 查询单行（无结果返回错误）
    fn query_row<F, T>(&self, sql: &str, params: &[&dyn IntoValue], f: F) -> Result<T>
    where
        F: FnOnce(&Row) -> Result<T>;

    /// 查询可选单行（无结果返回 None）
    fn query_row_optional<F, T>(&self, sql: &str, params: &[&dyn IntoValue], f: F) -> Result<Option<T>>
    where
        F: FnOnce(&Row) -> Result<T>;

    /// 在事务中执行操作，成功提交，失败回滚
    fn with_transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn TransactionOps) -> Result<T>;

    /// 最后插入的行 ID
    fn last_insert_rowid(&self) -> i64;

    /// 批量执行（用于迁移、schema 初始化等）
    fn execute_batch(&self, sql: &str) -> Result<()>;

    /// UPSERT 便捷方法
    ///
    /// SQLite 使用 INSERT OR REPLACE，MySQL 使用 ON DUPLICATE KEY UPDATE。
    fn upsert(
        &self,
        table: &str,
        columns: &[&str],
        params: &[&dyn IntoValue],
        conflict_cols: &[&str],
    ) -> Result<usize>;
}
