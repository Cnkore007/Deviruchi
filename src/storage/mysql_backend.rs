//! MySQL 数据库后端实现
//!
//! 仅在启用 `mysql-backend` feature 时编译。
//! 实现与 SqliteBackend 相同的方法接口，通过 Backend enum dispatch 统一调用。
//!
//! SQL 方言差异：
//! - MySQL 使用 `?` 占位符（与 SQLite 相同）
//! - MySQL 使用 `ON DUPLICATE KEY UPDATE` 替代 `INSERT OR REPLACE`
//! - MySQL 不支持 `PRAGMA` 语句

#[cfg(feature = "mysql-backend")]
mod inner {
    use super::super::backend::{IntoValue, Row, TransactionOps, Value};
    use crate::error::{Error, Result};
    use mysql::prelude::Queryable;
    use parking_lot::Mutex;
    use std::sync::atomic::{AtomicI64, Ordering};

    // ============================================================
    // 配置
    // ============================================================

    /// MySQL 后端配置
    #[derive(Debug, Clone)]
    pub struct MySqlConfig {
        /// 数据库主机地址
        pub host: String,
        /// 数据库端口
        pub port: u16,
        /// 数据库用户名
        pub user: String,
        /// 数据库密码
        pub password: String,
        /// 数据库名称
        pub database: String,
    }

    impl Default for MySqlConfig {
        fn default() -> Self {
            Self {
                host: "127.0.0.1".to_string(),
                port: 3306,
                user: "deviruchi".to_string(),
                password: String::new(),
                database: "deviruchi".to_string(),
            }
        }
    }

    // ============================================================
    // 值类型转换
    // ============================================================

    /// 将抽象 Value 转换为 mysql::Value
    fn value_to_mysql(v: &Value) -> mysql::Value {
        match v {
            Value::Null => mysql::Value::NULL,
            Value::Int(i) => mysql::Value::Int(*i as i64),
            Value::BigInt(i) => mysql::Value::Int(*i),
            Value::Float(f) => mysql::Value::Double(*f),
            Value::Text(s) => mysql::Value::Bytes(s.as_bytes().to_vec()),
            Value::Blob(b) => mysql::Value::Bytes(b.clone()),
        }
    }

    /// 将 mysql::Value 转换为抽象 Value
    fn mysql_to_value(v: &mysql::Value) -> Value {
        match v {
            mysql::Value::NULL => Value::Null,
            mysql::Value::Int(i) => Value::BigInt(*i),
            mysql::Value::UInt(u) => Value::BigInt(*u as i64),
            mysql::Value::Float(f) => Value::Float(*f as f64),
            mysql::Value::Double(f) => Value::Float(*f),
            mysql::Value::Bytes(b) => {
                // 尝试解析为 UTF-8 文本，失败则作为 Blob
                match std::str::from_utf8(b) {
                    Ok(s) => Value::Text(s.to_string()),
                    Err(_) => Value::Blob(b.clone()),
                }
            }
            // Date/Time 等特殊类型序列化为文本
            other => Value::Text(format!("{:?}", other)),
        }
    }

    /// 将 IntoValue 参数列表转换为 mysql::Params（位置参数）
    fn params_to_mysql(params: &[&dyn IntoValue]) -> Vec<mysql::Value> {
        params.iter().map(|p| value_to_mysql(&p.into_value())).collect()
    }

    /// 将 mysql_common::Row 转换为抽象 Row
    fn mysql_row_to_row(mysql_row: &mysql::Row) -> Row {
        let columns: Vec<(String, Value)> = mysql_row
            .columns_ref()
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let name = col.name_str().to_string();
                let value = mysql_row
                    .as_ref(i)
                    .map(mysql_to_value)
                    .unwrap_or(Value::Null);
                (name, value)
            })
            .collect();
        Row::new(columns)
    }

    // ============================================================
    // 事务支持
    // ============================================================

    /// MySQL 事务操作
    ///
    /// 通过 Mutex 包装连接，在事务期间提供 &self 接口。
    /// mysql::Conn 的方法需要 &mut self，通过 Mutex 的 DerefMut 实现。
    struct MySqlScopedTx<'a> {
        conn: &'a Mutex<mysql::PooledConn>,
        last_insert_id: &'a AtomicI64,
    }

    impl<'a> TransactionOps for MySqlScopedTx<'a> {
        fn execute(&self, sql: &str) -> Result<usize> {
            let mut conn = self.conn.lock();
            let mut result = conn
                .query_iter(sql)
                .map_err(|e| Error::DatabaseBackend(e.to_string()))?;
            let affected = result.affected_rows() as usize;
            if let Some(id) = result.last_insert_id() {
                self.last_insert_id.store(id as i64, Ordering::SeqCst);
            }
            Ok(affected)
        }

        fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize> {
            let mut conn = self.conn.lock();
            let values = params_to_mysql(params);
            let mut result = conn
                .exec_iter(sql, values)
                .map_err(|e| Error::DatabaseBackend(e.to_string()))?;
            let affected = result.affected_rows() as usize;
            if let Some(id) = result.last_insert_id() {
                self.last_insert_id.store(id as i64, Ordering::SeqCst);
            }
            Ok(affected)
        }

        fn execute_batch(&self, sql: &str) -> Result<()> {
            let mut conn = self.conn.lock();
            for stmt in sql.split(';').filter(|s| !s.trim().is_empty()) {
                conn.query_drop(stmt.trim())
                    .map_err(|e| Error::DatabaseBackend(e.to_string()))?;
            }
            Ok(())
        }

        fn last_insert_rowid(&self) -> i64 {
            self.last_insert_id.load(Ordering::SeqCst)
        }
    }

    // ============================================================
    // MySqlBackend 主结构
    // ============================================================

    /// MySQL 数据库后端
    ///
    /// 使用 mysql crate 的连接池（Pool）管理数据库连接。
    /// 每次操作从池中获取连接，操作完成后归还。
    /// 通过 AtomicI64 跟踪最后插入的行 ID。
    pub struct MySqlBackend {
        pool: mysql::Pool,
        last_insert_id: AtomicI64,
    }

    // mysql::Pool 内部使用 Arc，天然满足 Send + Sync。
    // AtomicI64 也是 Send + Sync 的。
    unsafe impl Send for MySqlBackend {}
    unsafe impl Sync for MySqlBackend {}

    impl MySqlBackend {
        /// 根据配置创建 MySQL 后端
        ///
        /// 建立连接池但不立即建立连接，首次操作时才会真正连接。
        pub fn new(config: &MySqlConfig) -> Result<Self> {
            let opts = mysql::OptsBuilder::new()
                .ip_or_hostname(Some(config.host.clone()))
                .tcp_port(config.port)
                .db_name(Some(config.database.clone()))
                .user(Some(config.user.clone()))
                .pass(Some(config.password.clone()));

            let pool = mysql::Pool::new(opts)
                .map_err(|e| Error::DatabaseBackend(format!("创建 MySQL 连接池失败: {}", e)))?;

            Ok(Self {
                pool,
                last_insert_id: AtomicI64::new(0),
            })
        }

        /// 从连接池获取一个连接的便捷方法
        fn get_conn(&self) -> Result<mysql::PooledConn> {
            self.pool
                .get_conn()
                .map_err(|e| Error::DatabaseBackend(format!("获取 MySQL 连接失败: {}", e)))
        }

        // ==================== 数据库操作方法 ====================

        /// 执行无参数 SQL
        pub fn execute(&self, sql: &str) -> Result<usize> {
            let mut conn = self.get_conn()?;
            let mut result = conn
                .query_iter(sql)
                .map_err(|e| Error::DatabaseBackend(e.to_string()))?;
            let affected = result.affected_rows() as usize;
            if let Some(id) = result.last_insert_id() {
                self.last_insert_id.store(id as i64, Ordering::SeqCst);
            }
            Ok(affected)
        }

        /// 带参数执行 SQL，返回影响行数
        pub fn execute_params(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<usize> {
            let mut conn = self.get_conn()?;
            let values = params_to_mysql(params);
            let mut result = conn
                .exec_iter(sql, values)
                .map_err(|e| Error::DatabaseBackend(e.to_string()))?;
            let affected = result.affected_rows() as usize;
            if let Some(id) = result.last_insert_id() {
                self.last_insert_id.store(id as i64, Ordering::SeqCst);
            }
            Ok(affected)
        }

        /// 查询多行
        pub fn query_rows(&self, sql: &str, params: &[&dyn IntoValue]) -> Result<Vec<Row>> {
            let mut conn = self.get_conn()?;
            let values = params_to_mysql(params);

            let mut result = conn
                .exec_iter(sql, values)
                .map_err(|e| Error::DatabaseBackend(e.to_string()))?;

            let mut rows = Vec::new();
            // 使用 iter() 获取结果集迭代器
            if let Some(result_set) = result.iter() {
                for row_result in result_set {
                    let mysql_row = row_result
                        .map_err(|e| Error::DatabaseBackend(e.to_string()))?;
                    rows.push(mysql_row_to_row(&mysql_row));
                }
            }

            Ok(rows)
        }

        /// 最后插入的行 ID
        ///
        /// 返回最近一次 INSERT 操作生成的自增 ID。
        /// 由于使用连接池，通过内部 AtomicI64 跟踪。
        pub fn last_insert_rowid(&self) -> i64 {
            self.last_insert_id.load(Ordering::SeqCst)
        }

        /// 批量执行（用于迁移、schema 初始化等）
        ///
        /// 按分号分割 SQL 语句，逐条执行。
        /// MySQL 默认不支持多语句执行（需要 CLIENT_MULTI_STATEMENTS），因此手动分割。
        pub fn execute_batch(&self, sql: &str) -> Result<()> {
            let mut conn = self.get_conn()?;
            for stmt in sql.split(';').filter(|s| !s.trim().is_empty()) {
                conn.query_drop(stmt.trim())
                    .map_err(|e| Error::DatabaseBackend(e.to_string()))?;
            }
            Ok(())
        }

        /// UPSERT 便捷方法
        ///
        /// MySQL 使用 `INSERT ... ON DUPLICATE KEY UPDATE` 语法。
        /// conflict_cols 在 MySQL 中不需要显式指定（依赖唯一索引/主键），
        /// 但保留参数签名以与 SqliteBackend 一致。
        pub fn upsert(
            &self,
            table: &str,
            columns: &[&str],
            params: &[&dyn IntoValue],
            _conflict_cols: &[&str],
        ) -> Result<usize> {
            // 构建 UPDATE 子句：col1=VALUES(col1), col2=VALUES(col2), ...
            let update_cols: Vec<String> = columns
                .iter()
                .map(|c| format!("{}=VALUES({})", c, c))
                .collect();

            let sql = format!(
                "INSERT INTO {} ({}) VALUES ({}) ON DUPLICATE KEY UPDATE {}",
                table,
                columns.join(", "),
                columns.iter().map(|_| "?").collect::<Vec<_>>().join(", "),
                update_cols.join(", ")
            );
            self.execute_params(&sql, params)
        }

        /// 在事务中执行操作，成功提交，失败回滚
        ///
        /// 实现方式：
        /// 1. 从连接池获取一个专用连接
        /// 2. 开启事务（START TRANSACTION）
        /// 3. 将连接包装在 Mutex 中，通过 MySqlScopedTx 提供 TransactionOps 接口
        /// 4. 执行用户闭包
        /// 5. 成功则 COMMIT，失败则 ROLLBACK
        pub fn with_transaction<F, T>(&self, f: F) -> Result<T>
        where
            F: FnOnce(&dyn TransactionOps) -> Result<T>,
        {
            let mut conn = self.get_conn()?;

            // 开启事务
            conn.query_drop("START TRANSACTION")
                .map_err(|e| Error::DatabaseBackend(format!("开启事务失败: {}", e)))?;

            // 将连接包装在 Mutex 中，以便在闭包中通过 &self 使用
            let conn_mutex = Mutex::new(conn);
            let last_insert_id = AtomicI64::new(0);

            let tx = MySqlScopedTx {
                conn: &conn_mutex,
                last_insert_id: &last_insert_id,
            };

            match f(&tx) {
                Ok(result) => {
                    let mut conn = conn_mutex.lock();
                    conn.query_drop("COMMIT")
                        .map_err(|e| Error::DatabaseBackend(format!("提交事务失败: {}", e)))?;
                    // 将事务内的 last_insert_id 同步到后端
                    self.last_insert_id
                        .store(last_insert_id.load(Ordering::SeqCst), Ordering::SeqCst);
                    Ok(result)
                }
                Err(e) => {
                    let mut conn = conn_mutex.lock();
                    conn.query_drop("ROLLBACK").ok();
                    Err(e)
                }
            }
        }
    }
}

#[cfg(feature = "mysql-backend")]
pub use inner::{MySqlBackend, MySqlConfig};
