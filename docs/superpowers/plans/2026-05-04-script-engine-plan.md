# 脚本引擎基础实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development

**Goal:** 实现 NPC 脚本引擎基础框架，支持基本对话命令

**Architecture:** 脚本解析器 + 对话状态机

**Tech Stack:** Rust

---

## Task 1: 创建脚本模块基础

**Files:**
- Create: `src/game/script/mod.rs`
- Create: `src/game/script/commands.rs`
- Create: `src/game/script/parser.rs`

- [ ] **Step 1: 创建 commands.rs**

```rust
use uuid::Uuid;
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
```

- [ ] **Step 2: 创建 parser.rs**

```rust
use super::commands::{ScriptCommand, ScriptNode};

/// 解析脚本文本
pub fn parse_script(script_text: &str) -> ScriptNode {
    let mut commands = Vec::new();
    let mut labels = HashMap::new();

    for line in script_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        // 解析标签
        if line.ends_with(':') {
            let label_name = line.trim_end_matches(':').to_string();
            labels.insert(label_name, commands.len());
            continue;
        }

        // 解析命令
        if let Some(cmd) = parse_line(line) {
            commands.push(cmd);
        }
    }

    ScriptNode { commands, labels }
}

fn parse_line(line: &str) -> Option<ScriptCommand> {
    let line = line.trim();

    // mes "消息"
    if let Some(msg) = line.strip_prefix("mes ") {
        let msg = extract_string(msg)?;
        return Some(ScriptCommand::Mes(msg));
    }

    // next
    if line == "next" {
        return Some(ScriptCommand::Next);
    }

    // close
    if line == "close" {
        return Some(ScriptCommand::Close);
    }

    // end
    if line == "end" {
        return Some(ScriptCommand::End);
    }

    // select("选项1","选项2",...)
    if let Some(opts) = line.strip_prefix("select(") {
        if let Some(opts) = opts.strip_suffix(")") {
            let options: Vec<String> = opts
                .split(',')
                .filter_map(|s| extract_string(s.trim()))
                .collect();
            if !options.is_empty() {
                return Some(ScriptCommand::Select(options));
            }
        }
    }

    // warp "地图", x, y
    if let Some(args) = line.strip_prefix("warp ") {
        let parts: Vec<&str> = args.split(',').collect();
        if parts.len() >= 3 {
            let map = extract_string(parts[0])?;
            let x: u16 = parts[1].trim().parse().ok()?;
            let y: u16 = parts[2].trim().parse().ok()?;
            return Some(ScriptCommand::Warp(map, x, y));
        }
    }

    None
}

fn extract_string(s: &str) -> Option<String> {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') {
        Some(s[1..s.len()-1].to_string())
    } else if s.starts_with('[') && s.ends_with(']') {
        // [NPC Name] 格式
        Some(s[1..s.len()-1].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mes() {
        let node = parse_script(r#"mes "Hello, world!";"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Mes(msg) = &node.commands[0] {
            assert_eq!(msg, "Hello, world!");
        } else {
            panic!("Expected Mes command");
        }
    }

    #[test]
    fn test_parse_select() {
        let node = parse_script(r#"select("Option 1","Option 2");"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Select(opts) = &node.commands[0] {
            assert_eq!(opts.len(), 2);
        }
    }

    #[test]
    fn test_parse_warp() {
        let node = parse_script(r#"warp "prontera", 100, 200;"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Warp(map, x, y) = &node.commands[0] {
            assert_eq!(map, "prontera");
            assert_eq!(*x, 100);
            assert_eq!(*y, 200);
        }
    }
}
```

- [ ] **Step 3: 创建 mod.rs**

```rust
pub mod commands;
pub mod parser;

pub use commands::{ScriptCommand, ScriptNode, NpcScript};
pub use parser::parse_script;
```

- [ ] **Step 4: 编译验证**

Run: `cargo build 2>&1 | head -30`

- [ ] **Step 5: 提交**

```bash
git add src/game/script/
git commit -m "feat(script): add basic script parser"
```

---

## Task 2: 实现对话状态机

**Files:**
- Create: `src/game/script/dialogue.rs`

- [ ] **Step 1: 创建 dialogue.rs**

```rust
use std::collections::HashMap;
use uuid::Uuid;
use crate::game::script::commands::{ScriptCommand, ScriptNode};

/// NPC 对话状态
pub struct NpcDialogueState {
    pub player_id: Uuid,
    pub npc_id: u32,
    pub script: ScriptNode,
    pub current_index: usize,
    pub pending_next: bool,  // 等待 next 后继续
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

        // 继续处理下一条命令
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
```

- [ ] **Step 2: 更新 mod.rs**

```rust
pub mod commands;
pub mod parser;
pub mod dialogue;

pub use commands::{ScriptCommand, ScriptNode, NpcScript};
pub use parser::parse_script;
pub use dialogue::{NpcDialogueState, DialogueResponse};
```

- [ ] **Step 3: 编译验证**

Run: `cargo build 2>&1 | head -30`

- [ ] **Step 4: 提交**

```bash
git add src/game/script/
git commit -m "feat(script): add dialogue state machine"
```

---

## Task 3: 集成到 NPC 系统

**Files:**
- Modify: `src/game/npc/`
- Modify: `src/game/mod.rs`

- [ ] **Step 1: 更新 game/mod.rs**

添加：
```rust
pub mod script;
pub use script::{parse_script, ScriptCommand, ScriptNode, NpcDialogueState, DialogueResponse};
```

- [ ] **Step 2: 编译验证**

Run: `cargo build 2>&1 | head -30`

- [ ] **Step 3: 提交**

```bash
git add src/game/
git commit -m "feat(script): integrate script engine with NPC system"
```

---

## Task 4: 添加测试

- [ ] **Step 1: 添加综合测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_dialogue() {
        let script = parse_script(r#"
            mes "Hello!";
            next;
            mes "How are you?";
            close;
        "#);

        let state = NpcDialogueState::new(
            Uuid::new_v4(),
            1,
            script,
        );

        assert!(state.is_active());
    }

    #[test]
    fn test_warp_command() {
        let script = parse_script(r#"warp "prontera", 150, 183;"#);
        assert_eq!(script.commands.len(), 1);
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test script 2>&1`

- [ ] **Step 3: 提交**

```bash
git add src/game/script/
git commit -m "test(script): add script engine tests"
```
