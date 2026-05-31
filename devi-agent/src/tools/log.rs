//! 日志搜索工具
//! 搜索和过滤游戏服务器日志

use std::sync::Arc;
use anyhow::Result;
use super::{Tool, ToolResult};
use crate::ipc::IpcClient;

/// 日志搜索工具
pub struct LogTool {
    ipc: Arc<IpcClient>,
}

impl LogTool {
    pub fn new(ipc: Arc<IpcClient>) -> Self {
        Self { ipc }
    }
}

#[async_trait::async_trait]
impl Tool for LogTool {
    fn name(&self) -> &str { "log" }

    async fn execute(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("search");

        match action {
            "search" => {
                let keyword = args.get("keyword").and_then(|v| v.as_str()).unwrap_or("");
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50);
                let level = args.get("level").and_then(|v| v.as_str()).unwrap_or("all");

                let resp = self.ipc.call("log.search", serde_json::json!({
                    "keyword": keyword,
                    "limit": limit,
                    "level": level,
                })).await?;

                if let Some(err) = resp.error {
                    return Ok(ToolResult { success: false, output: err.message });
                }

                let result = resp.result.unwrap_or_default();
                let count = result["count"].as_u64().unwrap_or(0);
                let mut output = format!("找到 {} 条日志:\n", count);

                if let Some(logs) = result["logs"].as_array() {
                    for log in logs.iter().take(20) {
                        output.push_str(&format!(
                            "[{}] {} {}\n",
                            log["timestamp"].as_str().unwrap_or("?"),
                            log["level"].as_str().unwrap_or("?"),
                            log["message"].as_str().unwrap_or("?"),
                        ));
                    }
                    if logs.len() > 20 {
                        output.push_str(&format!("... 还有 {} 条\n", logs.len() - 20));
                    }
                }

                Ok(ToolResult { success: true, output })
            }
            "tail" => {
                let lines = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(20);

                let resp = self.ipc.call("log.tail", serde_json::json!({
                    "lines": lines,
                })).await?;

                if let Some(err) = resp.error {
                    return Ok(ToolResult { success: false, output: err.message });
                }

                let result = resp.result.unwrap_or_default();
                let mut output = format!("最近 {} 条日志:\n", lines);

                if let Some(logs) = result["logs"].as_array() {
                    for log in logs {
                        output.push_str(&format!(
                            "[{}] {} {}\n",
                            log["timestamp"].as_str().unwrap_or("?"),
                            log["level"].as_str().unwrap_or("?"),
                            log["message"].as_str().unwrap_or("?"),
                        ));
                    }
                }

                Ok(ToolResult { success: true, output })
            }
            _ => Ok(ToolResult {
                success: false,
                output: format!("未知日志操作: {}（支持 search/tail）", action),
            }),
        }
    }
}
