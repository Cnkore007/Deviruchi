//! DeviAgent — Deviruchi 服务端智能助手
//!
//! 独立进程，通过 Unix Socket 与游戏服务器通信，
//! 集成 LLM 提供自然语言管理能力。

mod ipc;
mod llm;
mod memory;
mod tools;
mod repl;
mod knowledge;

use std::sync::Arc;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(false)
        .init();

    print_banner();

    // 知识索引初始化
    // 在启动时检查并生成 LLM 可用的参考文档
    let source_dir = std::env::current_dir().unwrap_or_default();
    let knowledge_dir = home_dir().join(".devi-agent").join("knowledge");
    let knowledge = knowledge::KnowledgeIndex::new(knowledge_dir, source_dir);
    if knowledge.needs_update() {
        println!("正在生成知识索引...");
        if let Err(e) = knowledge.generate() {
            println!("⚠ 知识索引生成失败: {}", e);
        } else {
            println!("✓ 知识索引已更新");
        }
    }

    // 初始化 IPC 客户端
    let ipc = Arc::new(ipc::IpcClient::new("/tmp/deviruchi.sock"));

    // 尝试连接游戏服务器
    match ipc.connect().await {
        Ok(_) => println!("✓ 已连接到游戏服务器"),
        Err(e) => println!("⚠ 无法连接: {}（使用 /connect 重连）", e),
    }

    // 初始化工具注册表
    let tools = Arc::new(tools::ToolRegistry::new(ipc.clone()));

    // 初始化持久化记忆存储
    let memory_path = home_dir().join(".devi-agent").join("memory.db");
    let memory = match memory::MemoryStore::new(&memory_path) {
        Ok(m) => {
            println!("✓ 记忆存储已初始化");
            Some(Arc::new(m))
        }
        Err(e) => {
            println!("⚠ 记忆存储初始化失败: {}", e);
            None
        }
    };

    // 初始化 LLM 客户端
    let llm_config = load_llm_config();
    let llm_client: Arc<dyn llm::LlmClient> = Arc::new(
        llm::openai::OpenAiClient::new(llm_config)
    );

    // 初始化 REPL
    let mut repl = repl::Repl::new()?;

    // 对话历史（以 system prompt 开始）
    let mut messages: Vec<llm::ChatMessage> = vec![
        llm::ChatMessage {
            role: "system".to_string(),
            content: Some(llm::prompt::system_prompt()),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    println!("\n输入自然语言与 AI 对话，/help 查看命令\n");

    // 主循环
    loop {
        let input = match repl.read_line()? {
            Some(line) => line,
            None => break, // Ctrl+C / Ctrl+D
        };

        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }

        // 斜杠命令处理
        if trimmed.starts_with('/') {
            match handle_slash_command(trimmed, &ipc, &tools).await {
                SlashResult::Quit => break,
                SlashResult::Output(msg) => println!("{}", msg),
                SlashResult::Continue => continue,
            }
            continue;
        }

        // 自然语言 → LLM 处理
        messages.push(llm::ChatMessage {
            role: "user".to_string(),
            content: Some(trimmed.to_string()),
            tool_calls: None,
            tool_call_id: None,
        });

        // LLM 对话循环（可能包含多轮工具调用）
        loop {
            let tool_defs = llm::prompt::tool_definitions();
            let response = match llm_client.chat(&messages, &tool_defs).await {
                Ok(r) => r,
                Err(e) => {
                    println!("LLM 错误: {}", e);
                    break;
                }
            };

            // 显示 LLM 文本回复
            if let Some(ref content) = response.content {
                if !content.is_empty() {
                    println!("\n{}\n", content);
                }
            }

            // 无工具调用 → 结束本轮对话
            if response.tool_calls.is_empty() {
                messages.push(llm::ChatMessage {
                    role: "assistant".to_string(),
                    content: response.content,
                    tool_calls: None,
                    tool_call_id: None,
                });
                break;
            }

            // 记录 assistant 消息（含 tool_calls）
            messages.push(llm::ChatMessage {
                role: "assistant".to_string(),
                content: response.content.clone(),
                tool_calls: Some(response.tool_calls.clone()),
                tool_call_id: None,
            });

            // 执行每个工具调用
            for tc in &response.tool_calls {
                let args: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(serde_json::Value::Null);

                println!("  ⚙ {}", tc.function.name);

                let result = tools.execute(&tc.function.name, &args).await
                    .unwrap_or(tools::ToolResult {
                        success: false,
                        output: "工具执行异常".to_string(),
                    });

                println!("  → {}\n", result.output);

                // 记录工具结果
                messages.push(llm::ChatMessage {
                    role: "tool".to_string(),
                    content: Some(result.output),
                    tool_calls: None,
                    tool_call_id: Some(tc.id.clone()),
                });
            }
        }
    }

    repl.save_history();
    println!("再见！");
    Ok(())
}

/// 打印启动横幅
fn print_banner() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  DeviAgent v0.1 — Deviruchi 智能助手                     ║");
    println!("║  连接状态: 检测中...                                      ║");
    println!("╚══════════════════════════════════════════════════════════╝");
}

/// 斜杠命令处理结果
enum SlashResult {
    Quit,
    Output(String),
    Continue,
}

/// 处理斜杠命令
async fn handle_slash_command(
    input: &str,
    ipc: &Arc<ipc::IpcClient>,
    tools: &Arc<tools::ToolRegistry>,
) -> SlashResult {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    match parts[0] {
        "/quit" | "/exit" | "/q" => SlashResult::Quit,

        "/help" => SlashResult::Output(
            "可用命令:\n\
             \x20 /help     — 显示帮助\n\
             \x20 /connect  — 连接游戏服务器\n\
             \x20 /status   — 服务器状态\n\
             \x20 /players  — 在线玩家\n\
             \x20 /quit     — 退出\n\n\
             直接输入自然语言与 AI 对话".to_string()
        ),

        "/connect" => {
            match ipc.connect().await {
                Ok(_) => SlashResult::Output("✓ 已连接到游戏服务器".to_string()),
                Err(e) => SlashResult::Output(format!("✗ 连接失败: {}", e)),
            }
        }

        "/status" => {
            match tools.execute("server_status", &serde_json::json!({})).await {
                Ok(r) => SlashResult::Output(r.output),
                Err(e) => SlashResult::Output(format!("错误: {}", e)),
            }
        }

        "/players" => {
            match tools.execute("player", &serde_json::json!({"action": "list"})).await {
                Ok(r) => SlashResult::Output(r.output),
                Err(e) => SlashResult::Output(format!("错误: {}", e)),
            }
        }

        _ => SlashResult::Output(format!("未知命令: {}（输入 /help 查看帮助）", parts[0])),
    }
}

/// 加载 LLM 配置
/// 从 ~/.devi-agent/config.toml 读取，不存在则使用默认值
fn load_llm_config() -> llm::openai::LlmConfig {
    let config_path = config_file_path();
    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str::<llm::openai::LlmConfig>(&content) {
                return config;
            }
        }
    }
    llm::openai::LlmConfig::default()
}

/// 配置文件路径: ~/.devi-agent/config.toml
fn config_file_path() -> std::path::PathBuf {
    home_dir().join(".devi-agent").join("config.toml")
}

/// 获取用户主目录
///
/// 从 HOME 环境变量获取，如果未设置则返回空路径。
/// 这是一个简化的实现，仅支持 Unix 系统。
fn home_dir() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_default()
}
