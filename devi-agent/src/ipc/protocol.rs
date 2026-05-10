//! IPC 协议类型定义
//! Agent 与游戏服务器之间通过 JSON-RPC 风格协议通信

use serde::{Deserialize, Serialize};

/// JSON-RPC 请求（Agent -> Server）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    /// 请求 ID，用于匹配响应
    pub id: u64,
    /// 调用的方法名
    pub method: String,
    /// 方法参数（JSON 值）
    #[serde(default)]
    pub params: serde_json::Value,
}

/// JSON-RPC 响应（Server -> Agent）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    /// 对应请求的 ID
    pub id: u64,
    /// 成功时的返回值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// 失败时的错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

/// JSON-RPC 错误对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    /// 错误码
    pub code: i32,
    /// 错误描述信息
    pub message: String,
}

impl RpcResponse {
    /// 创建成功响应
    pub fn success(id: u64, result: serde_json::Value) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
        }
    }

    /// 创建错误响应
    pub fn error(id: u64, code: i32, message: String) -> Self {
        Self {
            id,
            result: None,
            error: Some(RpcError { code, message }),
        }
    }

    /// 是否成功（无错误）
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
}
