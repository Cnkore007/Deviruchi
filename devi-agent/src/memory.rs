//! SQLite 持久化记忆存储
//!
//! 记录对话历史、工具调用和学习到的模式。
//! 数据库默认存储在 ~/.devi-agent/memory.db

use std::path::PathBuf;
use rusqlite::{Connection, params};
use anyhow::Result;
use chrono::Utc;

/// 持久化记忆存储
///
/// 使用 SQLite 存储对话历史、工具调用记录和学习到的模式。
/// 支持按关键词搜索历史记录。
pub struct MemoryStore {
    /// SQLite 数据库连接
    conn: Connection,
}

impl MemoryStore {
    /// 创建或打开记忆数据库
    ///
    /// 如果数据库文件不存在会自动创建，包括父目录。
    /// 自动初始化所需的表结构。
    pub fn new(path: &PathBuf) -> Result<Self> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        // 创建表（如不存在）
        conn.execute_batch("
            -- 对话历史表：记录用户和助手的消息
            CREATE TABLE IF NOT EXISTS conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL
            );

            -- 工具调用表：记录每次工具调用的详情和结果
            CREATE TABLE IF NOT EXISTS tool_calls (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                params TEXT NOT NULL,
                result TEXT,
                success INTEGER
            );

            -- 学习记录表：存储从对话中学到的模式和偏好
            CREATE TABLE IF NOT EXISTS learnings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                category TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL
            );
        ")?;

        Ok(Self { conn })
    }

    /// 记录一条对话消息
    ///
    /// # 参数
    /// - `role`: 消息角色（"user" 或 "assistant"）
    /// - `content`: 消息内容
    pub fn save_conversation(&self, role: &str, content: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO conversations (timestamp, role, content) VALUES (?1, ?2, ?3)",
            params![Utc::now().to_rfc3339(), role, content],
        )?;
        Ok(())
    }

    /// 记录一次工具调用
    ///
    /// # 参数
    /// - `tool_name`: 工具名称
    /// - `params_str`: 工具参数（JSON 字符串）
    /// - `result`: 执行结果
    /// - `success`: 是否成功
    pub fn save_tool_call(&self, tool_name: &str, params_str: &str, result: &str, success: bool) -> Result<()> {
        self.conn.execute(
            "INSERT INTO tool_calls (timestamp, tool_name, params, result, success) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![Utc::now().to_rfc3339(), tool_name, params_str, result, success as i32],
        )?;
        Ok(())
    }

    /// 获取最近的对话历史
    ///
    /// # 参数
    /// - `limit`: 返回的最大记录数
    ///
    /// # 返回
    /// 返回 (timestamp, role, content) 元组列表，按时间倒序
    pub fn recent_conversations(&self, limit: usize) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT timestamp, role, content FROM conversations ORDER BY id DESC LIMIT ?1"
        )?;

        let rows: Vec<(String, String, String)> = stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// 搜索对话历史
    ///
    /// 在消息内容中搜索包含关键词的记录。
    ///
    /// # 参数
    /// - `keyword`: 搜索关键词
    ///
    /// # 返回
    /// 返回最多 20 条匹配的记录
    pub fn search(&self, keyword: &str) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT timestamp, role, content FROM conversations WHERE content LIKE ?1 ORDER BY id DESC LIMIT 20"
        )?;

        let pattern = format!("%{}%", keyword);
        let rows: Vec<(String, String, String)> = stmt.query_map(params![pattern], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// 记录学习到的模式/偏好
    ///
    /// # 参数
    /// - `category`: 分类（如 "preference", "pattern", "skill"）
    /// - `key`: 键名
    /// - `value`: 值内容
    pub fn save_learning(&self, category: &str, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO learnings (timestamp, category, key, value) VALUES (?1, ?2, ?3, ?4)",
            params![Utc::now().to_rfc3339(), category, key, value],
        )?;
        Ok(())
    }

    /// 获取所有学习记录
    ///
    /// # 返回
    /// 返回 (timestamp, category, key, value) 元组列表，按时间倒序
    pub fn get_learnings(&self) -> Result<Vec<(String, String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT timestamp, category, key, value FROM learnings ORDER BY id DESC"
        )?;

        let rows: Vec<(String, String, String, String)> = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(rows)
    }
}
