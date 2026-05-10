//! 服务器状态查询工具
//! 查询游戏服务器运行时间和在线人数

use std::sync::Arc;
use anyhow::Result;
use super::{Tool, ToolResult};
use crate::ipc::IpcClient;

/// 服务器状态查询工具
pub struct ServerTool {
    ipc: Arc<IpcClient>,
}

impl ServerTool {
    pub fn new(ipc: Arc<IpcClient>) -> Self {
        Self { ipc }
    }
}

#[async_trait::async_trait]
impl Tool for ServerTool {
    fn name(&self) -> &str { "server_status" }

    async fn execute(&self, _args: &serde_json::Value) -> Result<ToolResult> {
        let resp = self.ipc.call("server.status", serde_json::json!({})).await?;

        if let Some(err) = resp.error {
            return Ok(ToolResult { success: false, output: err.message });
        }

        let result = resp.result.unwrap_or_default();
        let uptime = result["uptime_seconds"].as_u64().unwrap_or(0);
        let players = result["online_players"].as_u64().unwrap_or(0);

        let hours = uptime / 3600;
        let mins = (uptime % 3600) / 60;
        let secs = uptime % 60;

        Ok(ToolResult {
            success: true,
            output: format!(
                "服务器状态:\n  运行时间: {}h {}m {}s\n  在线玩家: {}",
                hours, mins, secs, players
            ),
        })
    }
}
