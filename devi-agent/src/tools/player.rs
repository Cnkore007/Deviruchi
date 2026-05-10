//! 玩家查询工具
//! 查询在线玩家列表和玩家详情

use std::sync::Arc;
use anyhow::Result;
use super::{Tool, ToolResult};
use crate::ipc::IpcClient;

/// 玩家查询工具
pub struct PlayerTool {
    ipc: Arc<IpcClient>,
}

impl PlayerTool {
    pub fn new(ipc: Arc<IpcClient>) -> Self {
        Self { ipc }
    }
}

#[async_trait::async_trait]
impl Tool for PlayerTool {
    fn name(&self) -> &str { "player" }

    async fn execute(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("list");

        match action {
            "list" => {
                let mut params = serde_json::json!({});
                if let Some(map) = args.get("map").and_then(|v| v.as_str()) {
                    params["map"] = serde_json::json!(map);
                }

                let resp = self.ipc.call("player.list", params).await?;

                if let Some(err) = resp.error {
                    return Ok(ToolResult { success: false, output: err.message });
                }

                let result = resp.result.unwrap_or_default();
                let count = result["count"].as_u64().unwrap_or(0);
                let mut output = format!("当前在线 {} 人:\n", count);

                if let Some(players) = result["players"].as_array() {
                    for p in players {
                        output.push_str(&format!(
                            "  {} (Lv.{}) @ {} ({},{}) HP:{}/{}\n",
                            p["name"].as_str().unwrap_or("?"),
                            p["base_level"].as_u64().unwrap_or(0),
                            p["map"].as_str().unwrap_or("?"),
                            p["x"].as_u64().unwrap_or(0),
                            p["y"].as_u64().unwrap_or(0),
                            p["hp"].as_u64().unwrap_or(0),
                            p["max_hp"].as_u64().unwrap_or(0),
                        ));
                    }
                }

                Ok(ToolResult { success: true, output })
            }
            "info" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");

                let resp = self.ipc.call("player.info", serde_json::json!({
                    "name": name
                })).await?;

                if let Some(err) = resp.error {
                    return Ok(ToolResult { success: false, output: err.message });
                }

                let p = resp.result.unwrap_or_default();
                let output = format!(
                    "{} (ID:{})\n  等级: Base {} / Job {}\n  HP: {}/{} SP: {}/{}\n  位置: {} ({},{})\n  Zeny: {}\n  属性: STR {} AGI {} VIT {} INT {} DEX {} LUK {}",
                    p["name"].as_str().unwrap_or("?"),
                    p["char_id"].as_u64().unwrap_or(0),
                    p["base_level"].as_u64().unwrap_or(0),
                    p["job_level"].as_u64().unwrap_or(0),
                    p["hp"].as_u64().unwrap_or(0), p["max_hp"].as_u64().unwrap_or(0),
                    p["sp"].as_u64().unwrap_or(0), p["max_sp"].as_u64().unwrap_or(0),
                    p["map"].as_str().unwrap_or("?"),
                    p["x"].as_u64().unwrap_or(0), p["y"].as_u64().unwrap_or(0),
                    p["zeny"].as_u64().unwrap_or(0),
                    p["str"].as_u64().unwrap_or(0), p["agi"].as_u64().unwrap_or(0),
                    p["vit"].as_u64().unwrap_or(0), p["int"].as_u64().unwrap_or(0),
                    p["dex"].as_u64().unwrap_or(0), p["luk"].as_u64().unwrap_or(0),
                );

                Ok(ToolResult { success: true, output })
            }
            _ => Ok(ToolResult {
                success: false,
                output: format!("未知玩家操作: {}（支持 list/info）", action),
            }),
        }
    }
}
