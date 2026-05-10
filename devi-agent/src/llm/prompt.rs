//! System Prompt 和工具定义
//! 定义 Agent 的身份、能力和行为规则

use super::ToolDefinition;
use super::FunctionDefinition;

/// 系统提示词
pub fn system_prompt() -> String {
    r#"你是 DeviAgent，Deviruchi（Ragnarok Online 服务器模拟器）的智能助手。

你熟悉整个服务器代码库，包括：
- 配置系统：TOML 格式，13 个节（server, database, network, game, battle, drop, exp, respawn, log, skill, party, storage, chat）
- 数据库：rAthena 兼容 YAML 文件（item_db, mob_db, skill_db, droptable 等）
- 脚本系统：自定义行式脚本语言（mes, next, close, select, warp, getitem 等命令）
- 网络协议：rAthena 兼容二进制协议 + WebSocket JSON 协议

你的能力：
1. 读取和修改服务器配置，支持热重载
2. 查询和编辑游戏数据库（物品、怪物、技能、掉落表）
3. 查看在线玩家、位置、状态、背包
4. 创建和修改 NPC 脚本、物品脚本
5. 监控服务器运行状态（CPU、内存、Tick 延迟）
6. 搜索服务器日志

规则：
- 修改配置或数据前，先展示当前值和修改后的值，确认后再执行
- 涉及破坏性操作（删除数据、重启服务器）必须二次确认
- 所有修改自动备份原文件
- 用中文回复"#.to_string()
}

/// 工具定义列表
pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "server_status".to_string(),
                description: "查看服务器运行状态（运行时间、在线人数等）".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "config_get".to_string(),
                description: "读取服务器配置的指定节".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "section": {
                            "type": "string",
                            "description": "配置节名称，如 battle, game, exp, drop, network, server, skill, party, storage, chat, logging"
                        }
                    },
                    "required": ["section"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "config_set".to_string(),
                description: "修改服务器配置并保存（会自动备份原文件）".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "section": {
                            "type": "string",
                            "description": "配置节名称"
                        },
                        "key": {
                            "type": "string",
                            "description": "配置键名"
                        },
                        "value": {
                            "description": "新值（字符串、数字或布尔）"
                        }
                    },
                    "required": ["section", "key", "value"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "player_list".to_string(),
                description: "列出当前在线玩家及其位置".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "map": {
                            "type": "string",
                            "description": "可选，按地图名过滤"
                        }
                    },
                    "required": []
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "player_info".to_string(),
                description: "查看指定玩家的详细信息（等级、属性、装备等）".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "玩家名称"
                        }
                    },
                    "required": ["name"]
                }),
            },
        },
    ]
}
