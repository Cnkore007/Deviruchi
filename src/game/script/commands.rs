use std::collections::HashMap;

/// 脚本命令
#[derive(Debug, Clone)]
pub enum ScriptCommand {
    /// 显示消息
    Mes(String),
    /// 下一行
    Next,
    /// 关闭对话
    Close,
    /// 结束脚本
    End,
    /// 选择菜单
    Select(Vec<String>),
    /// 传送
    Warp(String, u16, u16),
    /// 跳转标签
    Goto(String),
    /// 设置变量
    Set(String, i64),
    /// 条件跳转：如果变量等于指定值则跳转
    GotoIf(String, i64, String),  // 变量名, 比较值, 目标标签
}

/// 脚本节点
#[derive(Debug, Clone)]
pub struct ScriptNode {
    pub commands: Vec<ScriptCommand>,
    pub labels: HashMap<String, usize>,
}

/// NPC 脚本
#[derive(Debug, Clone)]
pub struct NpcScript {
    pub npc_id: u32,
    pub script: ScriptNode,
}

/// 解析错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// 行号（1-based）
    pub line: usize,
    /// 原始行内容
    pub source: String,
    /// 错误描述
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "第 {} 行解析错误: {} (原文: '{}')", self.line, self.message, self.source)
    }
}

impl std::error::Error for ParseError {}
