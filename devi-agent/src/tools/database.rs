//! 数据库编辑工具
//! 查询和修改游戏数据库

use std::sync::Arc;
use anyhow::Result;
use super::{Tool, ToolResult};
use crate::ipc::IpcClient;

/// 数据库编辑工具
pub struct DatabaseTool {
    ipc: Arc<IpcClient>,
}

impl DatabaseTool {
    pub fn new(ipc: Arc<IpcClient>) -> Self {
        Self { ipc }
    }
}

#[async_trait::async_trait]
impl Tool for DatabaseTool {
    fn name(&self) -> &str { "database" }

    async fn execute(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("query");

        match action {
            "query" => {
                let table = args.get("table").and_then(|v| v.as_str()).unwrap_or("");
                let filter = args.get("filter").cloned().unwrap_or(serde_json::json!({}));

                let resp = self.ipc.call("database.query", serde_json::json!({
                    "table": table,
                    "filter": filter,
                })).await?;

                if let Some(err) = resp.error {
                    return Ok(ToolResult { success: false, output: err.message });
                }

                let result = resp.result.unwrap_or_default();
                let count = result["count"].as_u64().unwrap_or(0);
                let pretty = serde_json::to_string_pretty(&result["data"])?;

                Ok(ToolResult {
                    success: true,
                    output: format!("查询 {} 表，返回 {} 条记录:\n{}", table, count, pretty),
                })
            }
            "update" => {
                let table = args.get("table").and_then(|v| v.as_str()).unwrap_or("");
                let id = args.get("id").cloned().unwrap_or(serde_json::json!(0));
                let data = args.get("data").cloned().unwrap_or(serde_json::json!({}));

                let resp = self.ipc.call("database.update", serde_json::json!({
                    "table": table,
                    "id": id,
                    "data": data,
                })).await?;

                if let Some(err) = resp.error {
                    return Ok(ToolResult { success: false, output: err.message });
                }

                let result = resp.result.unwrap_or_default();
                Ok(ToolResult {
                    success: result["success"].as_bool().unwrap_or(false),
                    output: result["message"].as_str().unwrap_or("已更新").to_string(),
                })
            }
            _ => Ok(ToolResult {
                success: false,
                output: format!("未知数据库操作: {}（支持 query/update）", action),
            }),
        }
    }
}
