//! Agent 服务器
//!
//! 监听 Unix Socket，处理来自 Agent 进程的 JSON-RPC 请求。
//! 每个连接独立处理，逐行读取 JSON-RPC 请求并返回响应。

use std::sync::Arc;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tracing::{error, info};

use crate::game::agent_api::AgentApi;

/// Agent API 服务器
///
/// 在 Unix Socket 上监听，接受 Agent 进程的连接，
/// 解析 JSON-RPC 请求并分发到 AgentApi 处理。
pub struct AgentServer {
    /// Unix Socket 文件路径
    socket_path: String,
    /// API 处理器
    api: Arc<AgentApi>,
}

impl AgentServer {
    /// 创建 AgentServer 实例
    ///
    /// - `socket_path`: Unix Socket 文件路径（如 `/tmp/deviruchi.sock`）
    /// - `api`: AgentApi 处理器引用
    pub fn new(socket_path: String, api: Arc<AgentApi>) -> Self {
        Self { socket_path, api }
    }

    /// 启动监听
    ///
    /// 清理可能残留的旧 socket 文件后绑定监听。
    /// 每个新连接独立 spawn 一个异步任务处理。
    pub async fn listen(&self) -> anyhow::Result<()> {
        // 清理可能残留的旧 socket 文件
        let _ = std::fs::remove_file(&self.socket_path);

        let listener = UnixListener::bind(&self.socket_path)?;
        info!("Agent API 监听: {}", self.socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let api = self.api.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, api).await {
                            error!("Agent 连接错误: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Agent accept 错误: {}", e);
                }
            }
        }
    }

    /// 处理单个 Agent 连接
    ///
    /// 协议格式：每行一个 JSON-RPC 请求（以 `\n` 分隔）。
    ///
    /// 请求格式:
    /// ```json
    /// {"id": 1, "method": "server.status", "params": {}}
    /// ```
    ///
    /// 成功响应:
    /// ```json
    /// {"id": 1, "result": {...}}
    /// ```
    ///
    /// 错误响应:
    /// ```json
    /// {"id": 1, "error": {"code": -1, "message": "..."}}
    /// ```
    ///
    /// 连接断开时自动清理。
    async fn handle_connection(
        stream: tokio::net::UnixStream,
        api: Arc<AgentApi>,
    ) -> anyhow::Result<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        info!("Agent 已连接");

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                info!("Agent 已断开");
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // 解析 JSON-RPC 请求并分发处理
            let response = match serde_json::from_str::<Value>(trimmed) {
                Ok(req) => {
                    let id = req.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
                    let params = req.get("params").cloned().unwrap_or(Value::Null);

                    match api.handle(method, &params) {
                        Ok(result) => serde_json::json!({
                            "id": id,
                            "result": result,
                        }),
                        Err(msg) => serde_json::json!({
                            "id": id,
                            "error": {"code": -1, "message": msg},
                        }),
                    }
                }
                Err(e) => serde_json::json!({
                    "id": 0,
                    "error": {"code": -32700, "message": format!("JSON 解析错误: {}", e)},
                }),
            };

            // 写回响应（每行一个 JSON）
            let mut response_str = serde_json::to_string(&response)?;
            response_str.push('\n');
            writer.write_all(response_str.as_bytes()).await?;
        }

        Ok(())
    }
}
