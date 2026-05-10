//! 配置管理工具
//! 读取和修改服务器配置

use std::sync::Arc;
use anyhow::Result;
use super::{Tool, ToolResult};
use crate::ipc::IpcClient;

/// 配置管理工具
/// 支持读取和修改服务器配置
pub struct ConfigTool {
    ipc: Arc<IpcClient>,
}

impl ConfigTool {
    pub fn new(ipc: Arc<IpcClient>) -> Self {
        Self { ipc }
    }
}

#[async_trait::async_trait]
impl Tool for ConfigTool {
    fn name(&self) -> &str { "config" }

    async fn execute(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("get");

        match action {
            "get" => {
                let section = args.get("section")
                    .and_then(|v| v.as_str())
                    .unwrap_or("battle");

                let resp = self.ipc.call("config.get", serde_json::json!({
                    "section": section
                })).await?;

                if let Some(err) = resp.error {
                    return Ok(ToolResult { success: false, output: err.message });
                }

                let pretty = serde_json::to_string_pretty(&resp.result.unwrap_or_default())?;
                Ok(ToolResult {
                    success: true,
                    output: format!("[{}]\n{}", section, pretty),
                })
            }
            "set" => {
                let section = args.get("section").and_then(|v| v.as_str()).unwrap_or("");
                let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
                let value = args.get("value").cloned().unwrap_or(serde_json::Value::Null);

                let resp = self.ipc.call("config.set", serde_json::json!({
                    "section": section,
                    "key": key,
                    "value": value,
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
                output: format!("未知配置操作: {}（支持 get/set）", action),
            }),
        }
    }
}
