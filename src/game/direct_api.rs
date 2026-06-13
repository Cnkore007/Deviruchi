//! DirectApi 适配器
//!
//! 将 AgentApi 包装为 ServerConnector trait 实现，
//! 使 devi-agent 工具层可以直接调用服务器 API，无需 TCP。

use std::sync::Arc;
use serde_json::Value;
use devi_agent::{ServerConnector, RpcResponse};
use super::agent_api::AgentApi;

/// 直连 API 适配器
///
/// 实现 devi-agent 的 ServerConnector trait，
/// 将 JSON-RPC 调用直接转发给 AgentApi。
pub struct DirectApi {
    inner: Arc<AgentApi>,
}

impl DirectApi {
    pub fn new(api: Arc<AgentApi>) -> Self {
        Self { inner: api }
    }
}

#[async_trait::async_trait]
impl ServerConnector for DirectApi {
    async fn call(&self, method: &str, params: Value) -> anyhow::Result<RpcResponse> {
        match self.inner.handle(method, &params) {
            Ok(result) => Ok(RpcResponse::success(0, result)),
            Err(e) => Ok(RpcResponse::error(0, -1, e)),
        }
    }
}
