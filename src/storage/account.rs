use crate::error::Result;
use crate::storage::Database;
use crate::storage::chrono_now;
use rusqlite::params;

#[derive(Debug, Clone)]
pub struct Account {
    pub account_id: u32,
    pub user_id: String,
    pub password_hash: String,
    pub sex: u8,
    pub email: Option<String>,
    pub group_id: i32,
    pub state: i32,
    pub unban_time: i64,
    pub expiration_time: i64,
    pub logcount: i32,
    pub last_login: Option<i64>,
    pub created_at: i64,
}

impl Database {
    pub fn create_account(&self, user_id: &str, password: &str, sex: u8) -> Result<u32> {
        let created_at = chrono_now();
        let password_hash = crate::storage::password::hash_password(password)
            .map_err(|e| crate::error::Error::Game(e))?;
        self.execute_with_params(
            "INSERT INTO accounts (user_id, password_hash, sex, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![user_id, password_hash, sex, created_at],
        )?;
        Ok(self.last_insert_rowid()? as u32)
    }

    pub fn get_account_by_userid(&self, user_id: &str) -> Result<Option<Account>> {
        self.query_row_optional(
            "SELECT account_id, user_id, password_hash, sex, email, group_id,
                    state, unban_time, expiration_time, logcount, last_login, created_at
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
                    unban_time: row.get(7)?,
                    expiration_time: row.get(8)?,
                    logcount: row.get(9)?,
                    last_login: row.get(10)?,
                    created_at: row.get(11)?,
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
                    state, unban_time, expiration_time, logcount, last_login, created_at
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
                    unban_time: row.get(7)?,
                    expiration_time: row.get(8)?,
                    logcount: row.get(9)?,
                    last_login: row.get(10)?,
                    created_at: row.get(11)?,
                })
            },
        )
    }

    /// 检查封禁是否已过期，如果已过期则自动解除
    pub fn check_and_clear_ban(&self, account: &mut Account) -> Result<bool> {
        if account.state == 0 {
            return Ok(true); // 未被封禁
        }

        let now = chrono_now();

        // 检查 unban_time：> 0 表示有时间限制的封禁
        if account.unban_time > 0 && now >= account.unban_time {
            // 封禁已过期，自动解除
            self.execute_with_params(
                "UPDATE accounts SET state = 0, unban_time = 0 WHERE account_id = ?1",
                params![account.account_id],
            )?;
            account.state = 0;
            account.unban_time = 0;
            tracing::info!("Account {} ban expired, auto-unbanned", account.account_id);
            return Ok(true);
        }

        // 检查 expiration_time：> 0 表示账号有过期时间
        if account.expiration_time > 0 && now >= account.expiration_time {
            // 账号已过期
            return Ok(false);
        }

        Ok(false) // 仍在封禁中
    }
}
