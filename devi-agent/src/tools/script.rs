//! 脚本编写工具
//! 帮助用户编写和测试 NPC 脚本

use std::sync::Arc;
use anyhow::Result;
use super::{Tool, ToolResult};
use crate::ipc::IpcClient;

/// 脚本编写工具
pub struct ScriptTool {
    ipc: Arc<IpcClient>,
}

impl ScriptTool {
    pub fn new(ipc: Arc<IpcClient>) -> Self {
        Self { ipc }
    }
}

#[async_trait::async_trait]
impl Tool for ScriptTool {
    fn name(&self) -> &str { "script" }

    async fn execute(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("help");

        match action {
            "help" => {
                Ok(ToolResult {
                    success: true,
                    output: "NPC 脚本命令参考:\n\
                        \n\
                        对话命令:\n\
                        \x20 mes \"文本\"; — 显示对话框\n\
                        \x20 next; — 显示下一步按钮\n\
                        \x20 close; — 关闭对话框\n\
                        \x20 select \"选项1:选项2\"; — 显示选择菜单\n\
                        \n\
                        物品命令:\n\
                        \x20 getitem <item_id>, <amount>; — 给予物品\n\
                        \x20 countitem(<item_id>) — 查询物品数量\n\
                        \x20 checkweight(<item_id>, <amount>) — 检查负重\n\
                        \n\
                        玩家命令:\n\
                        \x20 getcharid(<type>) — 获取角色ID\n\
                        \x20 readparam(<param_id>) — 读取属性\n\
                        \x20 heal <hp>, <sp>; — 恢复HP/SP\n\
                        \x20 announce \"消息\", <flag>; — 全服公告\n\
                        \n\
                        流程控制:\n\
                        \x20 if (条件) { ... } else { ... }\n\
                        \x20 set <变量>, <值>;\n\
                        \x20 callfunc \"函数名\";".to_string(),
                })
            }
            "validate" => {
                let script = args.get("script").and_then(|v| v.as_str()).unwrap_or("");

                // 简单的语法检查
                let errors = validate_script(script);

                if errors.is_empty() {
                    Ok(ToolResult {
                        success: true,
                        output: "✓ 脚本语法检查通过".to_string(),
                    })
                } else {
                    Ok(ToolResult {
                        success: false,
                        output: format!("✗ 发现 {} 个错误:\n{}", errors.len(), errors.join("\n")),
                    })
                }
            }
            "reload" => {
                let npc_id = args.get("npc_id").and_then(|v| v.as_u64()).unwrap_or(0);

                let resp = self.ipc.call("script.reload", serde_json::json!({
                    "npc_id": npc_id,
                })).await?;

                if let Some(err) = resp.error {
                    return Ok(ToolResult { success: false, output: err.message });
                }

                let result = resp.result.unwrap_or_default();
                Ok(ToolResult {
                    success: result["success"].as_bool().unwrap_or(false),
                    output: result["message"].as_str().unwrap_or("已重载").to_string(),
                })
            }
            _ => Ok(ToolResult {
                success: false,
                output: format!("未知脚本操作: {}（支持 help/validate/reload）", action),
            }),
        }
    }
}

/// 简单的脚本语法检查
fn validate_script(script: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let mut brace_count = 0;
    let mut paren_count = 0;

    for (line_num, line) in script.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        // 检查括号匹配
        for ch in trimmed.chars() {
            match ch {
                '{' => brace_count += 1,
                '}' => brace_count -= 1,
                '(' => paren_count += 1,
                ')' => paren_count -= 1,
                _ => {}
            }
        }

        // 检查语句结尾
        if !trimmed.ends_with(';') && !trimmed.ends_with('{') && !trimmed.ends_with('}')
            && !trimmed.ends_with(':') && !trimmed.starts_with("if") && !trimmed.starts_with("else")
            && !trimmed.starts_with("//") {
            errors.push(format!("行 {}: 语句缺少分号", line_num + 1));
        }
    }

    if brace_count != 0 {
        errors.push(format!("大括号不匹配: 多出 {} 个{}", 
            if brace_count > 0 { brace_count } else { -brace_count },
            if brace_count > 0 { "{" } else { "}" }
        ));
    }

    if paren_count != 0 {
        errors.push(format!("小括号不匹配: 多出 {} 个{}", 
            if paren_count > 0 { paren_count } else { -paren_count },
            if paren_count > 0 { "(" } else { ")" }
        ));
    }

    errors
}
