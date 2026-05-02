use rusqlite::Connection;
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

    pub fn execute_with_params<T: rusqlite::Params>(&self, sql: &str, params: T) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.execute(sql, params)?)
    }

    pub fn query<T, F>(&self, sql: &str, f: F) -> Result<Vec<T>>
    where
        F: FnMut(&rusqlite::Row<'_>) -> Result<T, rusqlite::Error>,
    {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], f)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    pub fn query_row<T, F>(&self, sql: &str, f: F) -> Result<T>
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
