//! LocalConnector — 离线模式服务器连接器
//!
//! 当游戏服务器未运行时，直接读取文件和数据库
//! 提供 config、log、script 等工具的离线支持。

use serde_json::{Value, json};
use devi_agent::{ServerConnector, RpcResponse};

/// 离线模式连接器
///
/// 直接读取配置文件、数据库和日志文件，
/// 不依赖运行中的服务器进程。
pub struct LocalConnector {
    config_path: String,
    db_path: String,
    log_dir: String,
}

impl LocalConnector {
    pub fn new(config_path: &str, db_path: &str, log_dir: &str) -> Self {
        Self {
            config_path: config_path.to_string(),
            db_path: db_path.to_string(),
            log_dir: log_dir.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl ServerConnector for LocalConnector {
    async fn call(&self, method: &str, params: Value) -> anyhow::Result<RpcResponse> {
        let result: std::result::Result<RpcResponse, String> = match method {
            "server.status" => {
                Err("服务器未运行，无法查询运行状态".to_string())
            }

            "config.get" => {
                let section = params.get("section")
                    .and_then(|v| v.as_str())
                    .unwrap_or("server");

                let content = match std::fs::read_to_string(&self.config_path) {
                    Ok(c) => c,
                    Err(e) => return Ok(RpcResponse::error(0, -1, format!("读取配置失败: {}", e))),
                };
                let config: toml::Value = match toml::from_str(&content) {
                    Ok(c) => c,
                    Err(e) => return Ok(RpcResponse::error(0, -1, format!("解析配置失败: {}", e))),
                };

                let value = match section {
                    "server" => config.get("server"),
                    "database" => config.get("database"),
                    "network" => config.get("network"),
                    "game" => config.get("game"),
                    "battle" => config.get("battle"),
                    "drop" => config.get("drop"),
                    "exp" => config.get("exp"),
                    "respawn" => config.get("respawn"),
                    "log" | "logging" => config.get("logging"),
                    "skill" => config.get("skill"),
                    "party" => config.get("party"),
                    "storage" => config.get("storage"),
                    "chat" => config.get("chat"),
                    _ => return Ok(RpcResponse::error(0, -1, format!("未知配置节: {}", section))),
                };

                match value {
                    Some(v) => Ok(RpcResponse::success(0, serde_json::to_value(v).unwrap_or_default())),
                    None => Ok(RpcResponse::success(0, json!(null))),
                }
            }

            "config.set" => {
                Err("离线模式不支持修改配置（需要服务器运行以热重载）".to_string())
            }

            "player.list" | "player.info" => {
                Err("服务器未运行，无法查询在线玩家".to_string())
            }

            "database.query" => {
                let table = params.get("table").and_then(|v| v.as_str()).unwrap_or("");
                query_local_db(&self.db_path, table)
            }

            "database.update" => {
                Err("离线模式不支持修改数据库（需要服务器运行以维护缓存一致性）".to_string())
            }

            "log.search" => {
                let keyword = params.get("keyword").and_then(|v| v.as_str()).unwrap_or("");
                let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
                let level = params.get("level").and_then(|v| v.as_str()).unwrap_or("all");
                search_logs(&self.log_dir, keyword, limit, level)
            }

            "log.tail" => {
                let lines = params.get("lines").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                tail_logs(&self.log_dir, lines)
            }

            "script.reload" => {
                Err("离线模式不支持重载脚本（需要服务器运行）".to_string())
            }

            _ => Err(format!("未知方法: {}", method)),
        };

        match result {
            Ok(resp) => Ok(resp),
            Err(e) => Ok(RpcResponse::error(0, -1, e)),
        }
    }
}

fn query_local_db(db_path: &str, table: &str) -> std::result::Result<RpcResponse, String> {
    use rusqlite::Connection;

    let conn = Connection::open(db_path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    // 安全表名白名单
    let allowed = ["accounts", "characters", "guilds", "character_status",
                    "character_inventory", "character_hotkeys", "schema_version"];
    if !allowed.contains(&table) {
        return Ok(RpcResponse::error(0, -1, format!("不允许查询表: {}（允许: {:?}）", table, allowed)));
    }

    let sql = format!("SELECT * FROM {} LIMIT 100", table);
    let mut stmt = conn.prepare(&sql)
        .map_err(|e| format!("准备查询失败: {}", e))?;

    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    let rows = stmt.query_map([], |row| {
        let mut map = serde_json::Map::new();
        for (i, name) in column_names.iter().enumerate() {
            let val: rusqlite::types::Value = row.get(i).unwrap_or(rusqlite::types::Value::Null);
            let json_val = match val {
                rusqlite::types::Value::Null => serde_json::Value::Null,
                rusqlite::types::Value::Integer(n) => json!(n),
                rusqlite::types::Value::Real(f) => json!(f),
                rusqlite::types::Value::Text(s) => json!(s),
                rusqlite::types::Value::Blob(b) => json!(format!("<{} bytes>", b.len())),
            };
            map.insert(name.clone(), json_val);
        }
        Ok(serde_json::Value::Object(map))
    }).map_err(|e| format!("查询执行失败: {}", e))?;

    let mut data = Vec::new();
    for row in rows {
        if let Ok(r) = row {
            data.push(r);
        }
    }

    let count = data.len();
    Ok(RpcResponse::success(0, json!({
        "count": count,
        "data": data,
    })))
}

fn search_logs(log_dir: &str, keyword: &str, limit: usize, level: &str) -> std::result::Result<RpcResponse, String> {
    let dir = std::path::Path::new(log_dir);
    if !dir.exists() {
        return Ok(RpcResponse::error(0, -1, "日志目录不存在".to_string()));
    }

    // 读取最新的日志文件
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("读取日志目录失败: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "log"))
        .collect();

    entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    let mut results = Vec::new();
    for entry in entries.iter().take(5) {
        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for line in content.lines().rev() {
            if !keyword.is_empty() && !line.to_lowercase().contains(&keyword.to_lowercase()) {
                continue;
            }
            if level != "all" && !line.to_lowercase().contains(&level.to_lowercase()) {
                continue;
            }
            results.push(json!({
                "timestamp": entry.file_name().to_string_lossy(),
                "level": "INFO",
                "message": line,
            }));
            if results.len() >= limit {
                break;
            }
        }
        if results.len() >= limit {
            break;
        }
    }

    let count = results.len();
    Ok(RpcResponse::success(0, json!({
        "count": count,
        "logs": results,
    })))
}

fn tail_logs(log_dir: &str, lines: usize) -> std::result::Result<RpcResponse, String> {
    let dir = std::path::Path::new(log_dir);
    if !dir.exists() {
        return Ok(RpcResponse::error(0, -1, "日志目录不存在".to_string()));
    }

    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("读取日志目录失败: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "log"))
        .collect();

    entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    let latest = match entries.first() {
        Some(e) => e.path(),
        None => return Ok(RpcResponse::success(0, json!({"count": 0, "logs": []}))),
    };

    let content = std::fs::read_to_string(&latest)
        .map_err(|e| format!("读取日志失败: {}", e))?;

    let all_lines: Vec<&str> = content.lines().collect();
    let start = all_lines.len().saturating_sub(lines);
    let logs: Vec<Value> = all_lines[start..].iter().map(|line| {
        json!({
            "timestamp": latest.file_name().unwrap_or_default().to_string_lossy(),
            "level": "INFO",
            "message": line,
        })
    }).collect();

    let count = logs.len();
    Ok(RpcResponse::success(0, json!({
        "count": count,
        "logs": logs,
    })))
}
