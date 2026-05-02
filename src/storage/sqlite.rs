use rusqlite::{Connection, OptionalExtension, Row};
use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::error::Result;

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

    pub fn execute_with_params<T: rusqlite::Params>(&self, sql: &str, params: T) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.execute(sql, params)?)
    }

    pub fn query<T, P, F>(&self, sql: &str, params: P, mut f: F) -> Result<Vec<T>>
    where
        P: rusqlite::Params,
        F: FnMut(&Row<'_>) -> std::result::Result<T, rusqlite::Error>,
    {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params, |row| f(row))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    pub fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<T>
    where
        P: rusqlite::Params,
        F: FnOnce(&Row<'_>) -> std::result::Result<T, rusqlite::Error>,
    {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        stmt.query_row(params, f).map_err(|e| e.into())
    }

    pub fn query_row_optional<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<Option<T>>
    where
        P: rusqlite::Params,
        F: FnOnce(&Row<'_>) -> std::result::Result<T, rusqlite::Error>,
    {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        stmt.query_row(params, f)
            .optional()
            .map_err(|e| e.into())
    }

    pub fn last_insert_rowid(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.last_insert_rowid() as u64)
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
