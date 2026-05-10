//! OpenAI 兼容 API 客户端
//! 支持 OpenAI、Claude（兼容模式）、Ollama 等

use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use super::{ChatMessage, ToolDefinition, LlmResponse, ToolCall, FunctionCall, LlmClient};

/// LLM 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// API Key
    pub api_key: String,
    /// API 基础 URL
    pub base_url: String,
    /// 模型名称
    pub model: String,
    /// 最大输出 token 数
    pub max_tokens: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4".to_string(),
            max_tokens: 4096,
        }
    }
}

/// OpenAI 兼容 API 客户端
pub struct OpenAiClient {
    config: LlmConfig,
    http: Client,
}

impl OpenAiClient {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            http: Client::new(),
        }
    }
}

/// API 响应结构
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ApiToolCall>>,
}

#[derive(Debug, Deserialize)]
struct ApiToolCall {
    id: String,
    function: ApiFunctionCall,
}

#[derive(Debug, Deserialize)]
struct ApiFunctionCall {
    name: String,
    arguments: String,
}

#[async_trait::async_trait]
impl LlmClient for OpenAiClient {
    async fn chat(&self, messages: &[ChatMessage], tools: &[ToolDefinition]) -> Result<LlmResponse> {
        let url = format!("{}/chat/completions", self.config.base_url);

        // 构造请求体
        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "max_tokens": self.config.max_tokens,
        });

        // 添加工具定义（如果有）
        if !tools.is_empty() {
            body["tools"] = serde_json::to_value(tools)?;
        }

        // 发送请求
        let resp = self.http.post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        // 检查 HTTP 状态
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("LLM API 错误 ({}): {}", status, text));
        }

        // 解析响应
        let data: ChatCompletionResponse = resp.json().await?;

        let choice = data.choices.into_iter().next()
            .ok_or(anyhow!("LLM 响应中没有 choices"))?;

        // 提取工具调用
        let tool_calls: Vec<ToolCall> = choice.message.tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| ToolCall {
                id: tc.id,
                function: FunctionCall {
                    name: tc.function.name,
                    arguments: tc.function.arguments,
                },
            })
            .collect();

        Ok(LlmResponse {
            content: choice.message.content,
            tool_calls,
        })
    }
}
