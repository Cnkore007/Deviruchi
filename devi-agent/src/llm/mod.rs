//! LLM 模块
//! 定义 LLM 客户端 trait 和消息类型

pub mod openai;
pub mod prompt;

use serde::{Deserialize, Serialize};
use anyhow::Result;

/// 工具调用请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 调用 ID
    pub id: String,
    /// 函数调用详情
    pub function: FunctionCall,
}

/// 函数调用详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// 函数名
    pub name: String,
    /// 参数（JSON 字符串）
    pub arguments: String,
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 角色：system / user / assistant / tool
    pub role: String,
    /// 文本内容
    pub content: Option<String>,
    /// 工具调用列表（assistant 消息时有值）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// 工具调用 ID（tool 角色消息时必填）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 工具类型（固定 "function"）
    #[serde(rename = "type")]
    pub tool_type: String,
    /// 函数定义
    pub function: FunctionDefinition,
}

/// 函数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// 函数名
    pub name: String,
    /// 函数描述
    pub description: String,
    /// 参数 JSON Schema
    pub parameters: serde_json::Value,
}

/// LLM 响应
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// 文本回复
    pub content: Option<String>,
    /// 工具调用列表
    pub tool_calls: Vec<ToolCall>,
}

/// LLM 客户端 trait
///
/// 支持 OpenAI 兼容 API 的聊天补全。
/// 实现者只需提供 HTTP 请求逻辑。
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    /// 发送聊天消息并获取响应
    async fn chat(&self, messages: &[ChatMessage], tools: &[ToolDefinition]) -> Result<LlmResponse>;
}
