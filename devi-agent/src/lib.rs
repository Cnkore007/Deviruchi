//! DeviAgent 库
//!
//! 提供 Agent 工具系统、服务器通信抽象和 LLM 集成。
//! 可被主服务器直接引入使用，也可作为独立二进制运行。

pub mod ipc;
pub mod tools;
pub mod memory;
pub mod knowledge;
#[cfg(feature = "llm")]
pub mod llm;

pub use ipc::protocol::{RpcRequest, RpcResponse, RpcError};
pub use tools::{Tool, ToolResult, ToolRegistry};

use anyhow::Result;
use serde_json::Value;

/// 服务器通信抽象
///
/// 抽象 Agent 与游戏服务器之间的通信方式：
/// - 独立二进制：通过 TCP JSON-RPC（IpcClient）
/// - 服务器内嵌：直接调用 AgentApi（DirectApi）
/// - CLI 离线：直接读取文件/数据库（LocalConnector）
#[async_trait::async_trait]
pub trait ServerConnector: Send + Sync {
    async fn call(&self, method: &str, params: Value) -> Result<RpcResponse>;
}

/// Agent 交互式 REPL 运行器
///
/// 提供斜杠命令和工具执行的交互式循环。
/// LLM 自然语言模式需要 `llm` feature。
pub struct AgentRunner {
    tools: std::sync::Arc<ToolRegistry>,
}

impl AgentRunner {
    pub fn new(connector: std::sync::Arc<dyn ServerConnector>) -> Self {
        Self {
            tools: std::sync::Arc::new(ToolRegistry::new(connector)),
        }
    }

    /// 运行交互式 REPL
    ///
    /// 从 stdin 读取输入，支持斜杠命令和工具调用。
    /// 返回 Ok(()) 表示用户退出。
    pub async fn run(&self) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};

        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║  DeviAgent — Deviruchi 智能助手                          ║");
        println!("║  输入 /help 查看命令，/quit 退出                         ║");
        println!("╚══════════════════════════════════════════════════════════╝");
        println!();

        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        loop {
            print!("DeviAgent> ");
            use std::io::Write;
            std::io::stdout().flush()?;

            let input = match lines.next_line().await? {
                Some(line) => line,
                None => break,
            };

            let trimmed = input.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with('/') {
                match self.handle_slash_command(trimmed).await {
                    SlashResult::Quit => break,
                    SlashResult::Output(msg) => println!("{}\n", msg),
                    SlashResult::Continue => {}
                }
            } else {
                // 直接作为工具名 + JSON 参数调用
                println!("提示: 输入 /help 查看可用命令\n");
            }
        }

        println!("再见！");
        Ok(())
    }

    async fn handle_slash_command(&self, input: &str) -> SlashResult {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        match parts[0] {
            "/quit" | "/exit" | "/q" => SlashResult::Quit,

            "/help" => SlashResult::Output(
                "可用命令:\n\
                 \x20 /help              — 显示帮助\n\
                 \x20 /status            — 服务器状态\n\
                 \x20 /players           — 在线玩家\n\
                 \x20 /config <section>  — 读取配置\n\
                 \x20 /tool <name> <json>— 执行工具\n\
                 \x20 /quit              — 退出\n\n\
                 工具列表: server_status, config, player, database, log, script".to_string()
            ),

            "/status" => {
                match self.tools.execute("server_status", &serde_json::json!({})).await {
                    Ok(r) => SlashResult::Output(r.output),
                    Err(e) => SlashResult::Output(format!("错误: {}", e)),
                }
            }

            "/players" => {
                match self.tools.execute("player", &serde_json::json!({"action": "list"})).await {
                    Ok(r) => SlashResult::Output(r.output),
                    Err(e) => SlashResult::Output(format!("错误: {}", e)),
                }
            }

            "/config" => {
                let section = if parts.len() > 1 { parts[1].trim() } else { "server" };
                match self.tools.execute("config", &serde_json::json!({"action": "get", "section": section})).await {
                    Ok(r) => SlashResult::Output(r.output),
                    Err(e) => SlashResult::Output(format!("错误: {}", e)),
                }
            }

            "/tool" => {
                if parts.len() < 2 {
                    return SlashResult::Output("用法: /tool <name> <json_params>\n示例: /tool player {\"action\":\"list\"}".to_string());
                }
                let tool_input = parts[1].trim();
                let (name, args) = match tool_input.find(' ') {
                    Some(pos) => {
                        let n = &tool_input[..pos];
                        let a = &tool_input[pos + 1..];
                        (n, serde_json::from_str::<serde_json::Value>(a).unwrap_or(serde_json::json!({})))
                    }
                    None => (tool_input, serde_json::json!({})),
                };
                match self.tools.execute(name, &args).await {
                    Ok(r) => SlashResult::Output(r.output),
                    Err(e) => SlashResult::Output(format!("错误: {}", e)),
                }
            }

            _ => SlashResult::Output(format!("未知命令: {}（输入 /help 查看帮助）", parts[0])),
        }
    }
}

enum SlashResult {
    Quit,
    Output(String),
    Continue,
}
