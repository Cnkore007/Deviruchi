use uuid::Uuid;
use crate::game::script::commands::{ScriptCommand, ScriptNode};

/// NPC 对话状态
pub struct NpcDialogueState {
    pub player_id: Uuid,
    pub npc_id: u32,
    pub script: ScriptNode,
    pub current_index: usize,
    pub pending_next: bool,
}

impl NpcDialogueState {
    pub fn new(player_id: Uuid, npc_id: u32, script: ScriptNode) -> Self {
        Self {
            player_id,
            npc_id,
            script,
            current_index: 0,
            pending_next: false,
        }
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

            ScriptCommand::Close | ScriptCommand::End => {
                DialogueResponse::Closed
            }

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
                    self.current_index += 1;
                }
                DialogueResponse::Continue
            }

            ScriptCommand::Set(_, _) => {
                self.current_index += 1;
                DialogueResponse::Continue
            }
        }
    }

    /// 处理玩家输入（如选择菜单）
    pub fn handle_input(&mut self, input: usize) -> DialogueResponse {
        self.pending_next = false;
        self.process()
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
