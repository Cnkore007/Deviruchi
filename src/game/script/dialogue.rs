use crate::game::script::commands::{ScriptCommand, ScriptNode};
use std::collections::HashMap;
use uuid::Uuid;

/// NPC 对话状态
pub struct NpcDialogueState {
    pub player_id: Uuid,
    pub npc_id: u32,
    pub script: ScriptNode,
    pub current_index: usize,
    pub pending_next: bool,
    /// 脚本变量存储（变量名 -> 值）
    pub variables: HashMap<String, i64>,
    /// 最近一次 Select 调用的玩家选择（1-based，0 表示未选择）
    last_select: usize,
}

impl NpcDialogueState {
    pub fn new(player_id: Uuid, npc_id: u32, script: ScriptNode) -> Self {
        Self {
            player_id,
            npc_id,
            script,
            current_index: 0,
            pending_next: false,
            variables: HashMap::new(),
            last_select: 0,
        }
    }

    /// 获取变量值
    pub fn get_variable(&self, name: &str) -> Option<i64> {
        self.variables.get(name).copied()
    }

    /// 设置变量值
    pub fn set_variable(&mut self, name: String, value: i64) {
        self.variables.insert(name, value);
    }

    /// 处理当前命令，返回需要显示的内容
    pub fn process(&mut self) -> DialogueResponse {
        if self.current_index >= self.script.commands.len() {
            return DialogueResponse::Closed;
        }

        let cmd = &self.script.commands[self.current_index].clone();

        match cmd {
            ScriptCommand::Mes(msg) => {
                self.current_index += 1;
                DialogueResponse::Message(msg.clone())
            }

            ScriptCommand::Next => {
                self.pending_next = true;
                self.current_index += 1;
                DialogueResponse::Pending
            }

            ScriptCommand::Close | ScriptCommand::End => DialogueResponse::Closed,

            ScriptCommand::Select(options) => {
                self.current_index += 1;
                DialogueResponse::Select(options.clone())
            }

            ScriptCommand::Warp(map, x, y) => {
                self.current_index += 1;
                DialogueResponse::Warp {
                    map: map.clone(),
                    x: *x,
                    y: *y,
                }
            }

            ScriptCommand::Goto(label) => {
                if let Some(&idx) = self.script.labels.get(label) {
                    self.current_index = idx;
                } else {
                    // 标签未找到，跳过此命令并记录警告
                    tracing::warn!(
                        "脚本标签 '{}' 未找到，跳过 goto 命令 (NPC: {})",
                        label, self.npc_id
                    );
                    self.current_index += 1;
                }
                DialogueResponse::Continue
            }

            ScriptCommand::Set(var_name, value) => {
                self.variables.insert(var_name.clone(), *value);
                self.current_index += 1;
                DialogueResponse::Continue
            }

            ScriptCommand::GotoIf(var_name, target_value, label) => {
                let current_value = self.variables.get(var_name).copied().unwrap_or(0);
                if current_value == *target_value {
                    if let Some(&idx) = self.script.labels.get(label) {
                        self.current_index = idx;
                    } else {
                        tracing::warn!("条件跳转标签 '{}' 未找到 (NPC: {})", label, self.npc_id);
                        self.current_index += 1;
                    }
                } else {
                    self.current_index += 1;
                }
                DialogueResponse::Continue
            }
        }
    }

    /// 处理玩家输入（如选择菜单）
    /// input: 1-based 选择索引（rAthena 协议发送 1-based 索引）
    pub fn handle_input(&mut self, input: usize) -> DialogueResponse {
        self.pending_next = false;
        self.last_select = input;
        self.process()
    }

    /// 获取最近一次 Select 的选择结果
    pub fn last_select(&self) -> usize {
        self.last_select
    }

    /// 是否仍在对话中
    pub fn is_active(&self) -> bool {
        self.current_index < self.script.commands.len()
    }
}

/// 对话响应
#[derive(Debug, Clone)]
pub enum DialogueResponse {
    /// 显示消息
    Message(String),
    /// 选择菜单
    Select(Vec<String>),
    /// 等待 next
    Pending,
    /// 继续下一条
    Continue,
    /// 传送
    Warp { map: String, x: u16, y: u16 },
    /// 对话关闭
    Closed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::script::parse_script;
    use uuid::Uuid;

    #[test]
    fn test_simple_dialogue() {
        let script = parse_script(
            r#"
            mes "Hello!";
            next;
            mes "How are you?";
            close;
        "#,
        );

        let state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        assert!(state.is_active());
    }

    #[test]
    fn test_dialogue_process_mes() {
        let script = parse_script(r#"mes "Welcome!";"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        let response = state.process();
        match response {
            DialogueResponse::Message(msg) => assert_eq!(msg, "Welcome!"),
            _ => panic!("Expected Message response"),
        }
    }

    #[test]
    fn test_dialogue_process_warp() {
        let script = parse_script(r#"warp "prontera", 150, 183;"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        let response = state.process();
        match response {
            DialogueResponse::Warp { map, x, y } => {
                assert_eq!(map, "prontera");
                assert_eq!(x, 150);
                assert_eq!(y, 183);
            }
            _ => panic!("Expected Warp response"),
        }
    }

    #[test]
    fn test_dialogue_select() {
        let script = parse_script(r#"select("Yes","No");"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        let response = state.process();
        match response {
            DialogueResponse::Select(options) => {
                assert_eq!(options.len(), 2);
                assert_eq!(options[0], "Yes");
                assert_eq!(options[1], "No");
            }
            _ => panic!("Expected Select response"),
        }
    }

    #[test]
    fn test_dialogue_close() {
        let script = parse_script(
            r#"
            mes "Goodbye!";
            close;
        "#,
        );

        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        // 处理 mes
        let _ = state.process();
        // 处理 close
        let response = state.process();
        match response {
            DialogueResponse::Closed => {}
            _ => panic!("Expected Closed response"),
        }
    }

    #[test]
    fn test_set_stores_variable() {
        let script = parse_script(r#"set "$talked", 1;"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        let response = state.process();
        assert!(matches!(response, DialogueResponse::Continue));
        assert_eq!(state.get_variable("$talked"), Some(1));
    }

    #[test]
    fn test_set_multiple_variables() {
        let script = parse_script(
            r#"
            set "$count", 0;
            set "$name", 100;
            "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        state.process(); // set $count
        state.process(); // set $name
        assert_eq!(state.get_variable("$count"), Some(0));
        assert_eq!(state.get_variable("$name"), Some(100));
    }

    #[test]
    fn test_goto_uses_label() {
        let script = parse_script(
            r#"
            goto "skip";
            mes "This should be skipped";
        skip:
            mes "Arrived at skip";
            close;
            "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        // goto "skip" 应跳转到 skip 标签处（即 "Arrived at skip" 的 mes）
        let response = state.process(); // goto -> skip
        // goto 返回 Continue，状态机继续处理
        assert!(matches!(response, DialogueResponse::Continue));

        // 下一次 process 应该是 mes "Arrived at skip"
        let response = state.process();
        match response {
            DialogueResponse::Message(msg) => assert_eq!(msg, "Arrived at skip"),
            other => panic!("期望 Message(\"Arrived at skip\")，实际: {:?}", other),
        }
    }

    #[test]
    fn test_set_then_goto() {
        let script = parse_script(
            r#"
            set "$done", 1;
            goto "end_part";
            mes "Skipped";
        end_part:
            mes "Done!";
            close;
            "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        state.process(); // set
        assert_eq!(state.get_variable("$done"), Some(1));

        state.process(); // goto -> end_part

        let response = state.process(); // mes "Done!"
        match response {
            DialogueResponse::Message(msg) => assert_eq!(msg, "Done!"),
            other => panic!("期望 Message(\"Done!\"), 实际: {:?}", other),
        }
    }

    #[test]
    fn test_variable_default_none() {
        let script = parse_script(r#"mes "Hi";"#);
        let state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        assert_eq!(state.get_variable("$nonexistent"), None);
    }

    #[test]
    fn test_goto_if_condition_true() {
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, parse_script(
            r#"
            set "$done", 1;
            goto_if "$done", 1, "skip";
            mes "Should be skipped";
        skip:
            mes "Jumped!";
            close;
            "#,
        ));

        state.process(); // set "$done", 1
        state.process(); // goto_if -> 条件为真，跳转到 skip
        let response = state.process(); // mes "Jumped!"
        match response {
            DialogueResponse::Message(msg) => assert_eq!(msg, "Jumped!"),
            other => panic!("期望 Message(\"Jumped!\"), 实际: {:?}", other),
        }
    }

    #[test]
    fn test_goto_if_condition_false() {
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, parse_script(
            r#"
            set "$done", 0;
            goto_if "$done", 1, "skip";
            mes "Not skipped";
        skip:
            mes "End";
            close;
            "#,
        ));

        state.process(); // set "$done", 0
        state.process(); // goto_if -> 条件为假，不跳转
        let response = state.process(); // mes "Not skipped"
        match response {
            DialogueResponse::Message(msg) => assert_eq!(msg, "Not skipped"),
            other => panic!("期望 Message(\"Not skipped\"), 实际: {:?}", other),
        }
    }

    #[test]
    fn test_goto_if_undefined_variable() {
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, parse_script(
            r#"
            goto_if "$undefined", 1, "skip";
            mes "Variable undefined, no jump";
            close;
            "#,
        ));

        // goto_if 条件为假（未定义变量默认 0 != 1），返回 Continue
        let response = state.process();
        assert!(matches!(response, DialogueResponse::Continue));

        // 下一次 process 应该是 mes
        let response = state.process();
        match response {
            DialogueResponse::Message(msg) => assert_eq!(msg, "Variable undefined, no jump"),
            other => panic!("期望不跳转，实际: {:?}", other),
        }
    }
}
