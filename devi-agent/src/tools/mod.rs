//! 工具执行系统
//! 注册和调度 Agent 工具，每个工具对应一个游戏服务器 API 调用

pub mod config;
pub mod database;
pub mod log;
pub mod player;
pub mod script;
pub mod server;

use std::sync::Arc;
use anyhow::Result;
use crate::ipc::IpcClient;

/// 工具执行结果
pub struct ToolResult {
    /// 是否成功
    pub success: bool,
    /// 输出文本（展示给 LLM 或用户）
    pub output: String,
}

/// 工具 trait
///
/// 每个工具封装一个或多个 IPC 调用，
/// 将游戏服务器的 JSON 响应转换为可读文本。
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// 工具名称（与 LLM 工具定义中的 function.name 对应）
    fn name(&self) -> &str;
    /// 执行工具调用
    async fn execute(&self, args: &serde_json::Value) -> Result<ToolResult>;
}

/// 工具注册表
///
/// 管理所有可用工具，根据名称分发执行。
pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// 创建工具注册表，注册所有内置工具
    pub fn new(ipc: Arc<IpcClient>) -> Self {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(server::ServerTool::new(ipc.clone())),
            Arc::new(config::ConfigTool::new(ipc.clone())),
            Arc::new(player::PlayerTool::new(ipc.clone())),
        ];
        Self { tools }
    }

    /// 根据名称执行工具
    pub async fn execute(&self, name: &str, args: &serde_json::Value) -> Result<ToolResult> {
        for tool in &self.tools {
            if tool.name() == name {
                return tool.execute(args).await;
            }
        }
        Ok(ToolResult {
            success: false,
            output: format!("未知工具: {}", name),
        })
    }

    /// 列出所有注册的工具名称
    pub fn list_names(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name()).collect()
    }
}
