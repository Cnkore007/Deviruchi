use rusqlite::params;
use crate::storage::Database;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct Account {
    pub account_id: u32,
    pub user_id: String,
    pub password_hash: String,
    pub sex: u8,
    pub email: Option<String>,
    pub group_id: i32,
    pub state: i32,
    pub logcount: i32,
    pub last_login: Option<i64>,
    pub created_at: i64,
}

impl Database {
    pub fn create_account(&self, user_id: &str, password_hash: &str, sex: u8) -> Result<u32> {
        let created_at = chrono_now();
        self.execute_with_params(
            "INSERT INTO accounts (user_id, password_hash, sex, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![user_id, password_hash, sex, created_at],
        )?;
        Ok(self.last_insert_rowid()?)
    }

    pub fn get_account_by_userid(&self, user_id: &str) -> Result<Option<Account>> {
        self.query_row_optional(
            "SELECT account_id, user_id, password_hash, sex, email, group_id,
                    state, logcount, last_login, created_at
             FROM accounts WHERE user_id = ?1",
            params![user_id],
            |row| {
                Ok(Account {
                    account_id: row.get(0)?,
                    user_id: row.get(1)?,
                    password_hash: row.get(2)?,
                    sex: row.get(3)?,
                    email: row.get(4)?,
                    group_id: row.get(5)?,
                    state: row.get(6)?,
                    logcount: row.get(7)?,
                    last_login: row.get(8)?,
                    created_at: row.get(9)?,
                })
            },
        )
    }

    pub fn update_last_login(&self, account_id: u32) -> Result<()> {
        let now = chrono_now();
        self.execute_with_params(
            "UPDATE accounts SET last_login = ?1, logcount = logcount + 1 WHERE account_id = ?2",
            params![now, account_id],
        )?;
        Ok(())
    }

    pub fn get_account_by_id(&self, account_id: u32) -> Result<Option<Account>> {
        self.query_row_optional(
            "SELECT account_id, user_id, password_hash, sex, email, group_id,
                    state, logcount, last_login, created_at
             FROM accounts WHERE account_id = ?1",
            params![account_id],
            |row| {
                Ok(Account {
                    account_id: row.get(0)?,
                    user_id: row.get(1)?,
                    password_hash: row.get(2)?,
                    sex: row.get(3)?,
                    email: row.get(4)?,
                    group_id: row.get(5)?,
                    state: row.get(6)?,
                    logcount: row.get(7)?,
                    last_login: row.get(8)?,
                    created_at: row.get(9)?,
                })
            },
        )
    }
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
