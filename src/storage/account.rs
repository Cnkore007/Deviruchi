use crate::error::Result;
use crate::storage::Database;
use crate::storage::chrono_now;

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
        let password_hash =
            crate::storage::password::hash_password(password).map_err(crate::error::Error::Game)?;
        self.execute_params(
            "INSERT INTO accounts (user_id, password_hash, sex, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            &[
                &user_id as &dyn crate::storage::backend::IntoValue,
                &password_hash as &dyn crate::storage::backend::IntoValue,
                &(sex as i32) as &dyn crate::storage::backend::IntoValue,
                &created_at as &dyn crate::storage::backend::IntoValue,
            ],
        )?;
        Ok(self.last_insert_rowid() as u32)
    }

    pub fn get_account_by_userid(&self, user_id: &str) -> Result<Option<Account>> {
        self.query_row_optional(
            "SELECT account_id, user_id, password_hash, sex, email, group_id,
                    state, unban_time, expiration_time, logcount, last_login, created_at
             FROM accounts WHERE user_id = ?1",
            &[&user_id as &dyn crate::storage::backend::IntoValue],
            |row| {
                Ok(Account {
                    account_id: row.get_i32(0)? as u32,
                    user_id: row.get_string(1)?,
                    password_hash: row.get_string(2)?,
                    sex: row.get_i32(3)? as u8,
                    email: row.get_optional_string(4)?,
                    group_id: row.get_i32(5)?,
                    state: row.get_i32(6)?,
                    unban_time: row.get_i64(7)?,
                    expiration_time: row.get_i64(8)?,
                    logcount: row.get_i32(9)?,
                    last_login: row.get_optional_i64(10)?,
                    created_at: row.get_i64(11)?,
                })
            },
        )
    }

    pub fn update_last_login(&self, account_id: u32) -> Result<()> {
        let now = chrono_now();
        self.execute_params(
            "UPDATE accounts SET last_login = ?1, logcount = logcount + 1 WHERE account_id = ?2",
            &[
                &now as &dyn crate::storage::backend::IntoValue,
                &(account_id as i32) as &dyn crate::storage::backend::IntoValue,
            ],
        )?;
        Ok(())
    }

    pub fn get_account_by_id(&self, account_id: u32) -> Result<Option<Account>> {
        self.query_row_optional(
            "SELECT account_id, user_id, password_hash, sex, email, group_id,
                    state, unban_time, expiration_time, logcount, last_login, created_at
             FROM accounts WHERE account_id = ?1",
            &[&(account_id as i32) as &dyn crate::storage::backend::IntoValue],
            |row| {
                Ok(Account {
                    account_id: row.get_i32(0)? as u32,
                    user_id: row.get_string(1)?,
                    password_hash: row.get_string(2)?,
                    sex: row.get_i32(3)? as u8,
                    email: row.get_optional_string(4)?,
                    group_id: row.get_i32(5)?,
                    state: row.get_i32(6)?,
                    unban_time: row.get_i64(7)?,
                    expiration_time: row.get_i64(8)?,
                    logcount: row.get_i32(9)?,
                    last_login: row.get_optional_i64(10)?,
                    created_at: row.get_i64(11)?,
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
            self.execute_params(
                "UPDATE accounts SET state = 0, unban_time = 0 WHERE account_id = ?1",
                &[&(account.account_id as i32) as &dyn crate::storage::backend::IntoValue],
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
