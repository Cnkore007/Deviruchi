use crate::error::Result;
use rusqlite::{Connection, OptionalExtension, Row};
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;

/// SQLite 数据库封装
///
/// 使用 RwLock 实现读写分离：WAL 模式下多个读操作可并发执行，
/// 写操作（execute/transaction）获取独占锁。
pub struct Database {
    conn: Arc<RwLock<Connection>>,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        Ok(Self {
            conn: Arc::new(RwLock::new(conn)),
        })
    }

    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Ok(Self {
            conn: Arc::new(RwLock::new(conn)),
        })
    }

    /// 执行写操作（获取独占锁）
    pub fn execute(&self, sql: &str) -> Result<usize> {
        let conn = self.conn.write();
        Ok(conn.execute(sql, [])?)
    }

    /// 执行带参数的写操作（获取独占锁）
    pub fn execute_with_params<T: rusqlite::Params>(&self, sql: &str, params: T) -> Result<usize> {
        let conn = self.conn.write();
        Ok(conn.execute(sql, params)?)
    }

    /// 执行查询（获取共享读锁，允许并发读）
    pub fn query<T, P, F>(&self, sql: &str, params: P, mut f: F) -> Result<Vec<T>>
    where
        P: rusqlite::Params,
        F: FnMut(&Row<'_>) -> std::result::Result<T, rusqlite::Error>,
    {
        let conn = self.conn.read();
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params, |row| f(row))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    /// 查询单行（获取共享读锁）
    pub fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<T>
    where
        P: rusqlite::Params,
        F: FnOnce(&Row<'_>) -> std::result::Result<T, rusqlite::Error>,
    {
        let conn = self.conn.read();
        let mut stmt = conn.prepare(sql)?;
        stmt.query_row(params, f).map_err(|e| e.into())
    }

    /// 查询可选单行（获取共享读锁）
    pub fn query_row_optional<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<Option<T>>
    where
        P: rusqlite::Params,
        F: FnOnce(&Row<'_>) -> std::result::Result<T, rusqlite::Error>,
    {
        let conn = self.conn.read();
        let mut stmt = conn.prepare(sql)?;
        stmt.query_row(params, f).optional().map_err(|e| e.into())
    }

    /// 获取最后插入的行 ID（获取共享读锁）
    pub fn last_insert_rowid(&self) -> Result<u64> {
        let conn = self.conn.read();
        Ok(conn.last_insert_rowid() as u64)
    }

    /// 在事务中执行一组操作，成功时自动提交，失败时自动回滚
    pub fn with_transaction<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Connection) -> Result<R>,
    {
        let conn = self.conn.write();
        conn.execute_batch("BEGIN IMMEDIATE")?;
        match f(&conn) {
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

pub fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
