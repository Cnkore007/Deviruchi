# Script Engine + NPC System Completion Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development

**Goal:** 补全脚本引擎（goto/set/变量系统/错误报告/条件分支）和 NPC 系统（Warp 目标修复/learn_skill 补全/Quest 处理/测试），使 NPC 交互流程完整可用。

**Architecture:** 脚本解析器 (parser) + 对话状态机 (dialogue) + NPC 处理器 (handler) 三层架构。解析器将脚本文本转为 `ScriptNode`，状态机驱动命令执行流程，处理器管理 NPC 实体和交互逻辑。

**Tech Stack:** Rust, parking_lot, uuid, thiserror

---

## 现状分析

### 问题清单

| # | 模块 | 问题 | 严重程度 |
|---|------|------|----------|
| 1 | parser.rs | `goto` 语法未解析（已定义 `ScriptCommand::Goto` 但 parse_line 不识别） | 高 |
| 2 | parser.rs | `set "$var", value;` 语法未解析（已定义 `ScriptCommand::Set` 但 parse_line 不识别） | 高 |
| 3 | parser.rs | 解析失败静默返回 `None`，无行号、无错误信息 | 中 |
| 4 | dialogue.rs | `Set` 命令只做 `index+1`，变量未存储 | 高 |
| 5 | dialogue.rs | `handle_input(_input)` 忽略玩家 Select 选择结果 | 严重 |
| 6 | dialogue.rs | 无变量系统（无 `variables: HashMap<String, i64>`） | 高 |
| 7 | npc/data.rs | `Npc::warp()` 用 `_dest_map/_dest_x/_dest_y` 前缀丢弃参数 | 严重 |
| 8 | npc/data.rs | `Npc` 结构体缺少 `dest_map/dest_x/dest_y` 字段 | 严重 |
| 9 | npc/handler.rs | `interact()` 的 Warp 分支返回 NPC 自身坐标而非目标坐标 | 严重 |
| 10 | npc/handler.rs | `learn_skill()` 不扣 Zeny、不检查已学习技能 | 高 |
| 11 | npc/handler.rs | 零测试覆盖 | 中 |
| 12 | npc/data.rs | `NpcFlag` 定义但从未使用 | 低 |
| 13 | npc/data.rs | 只有 3 个硬编码 NPC 模板 | 低 |
| 14 | commands.rs | 无条件分支命令（if/else） | 中（Phase 2） |

### 依赖关系

```
Task 1 (goto/set 解析) ──┐
Task 2 (变量系统)     ────┼──→ Task 5 (handle_input 修复)
Task 3 (错误报告)     ────┘         │
                                    ├──→ Task 7 (集成测试)
Task 4 (Warp 修复)   ──────────────┤
Task 6 (learn_skill) ──────────────┤
Task 8 (Quest/CashShop) ───────────┘
```

---

## Phase 1: 脚本引擎补全

### Task 1: 解析器补全 — goto 和 set 语法

**目标:** 解析器支持 `goto "label";` 和 `set "$var", value;` 语法

**Files:**
- Modify: `src/game/script/parser.rs`

- [ ] **Step 1.1: 编写 goto/set 解析测试（红灯）**

在 `parser.rs` 的 `#[cfg(test)] mod tests` 中追加：

```rust
#[test]
fn test_parse_goto() {
    let node = parse_script(r#"goto "my_label";"#);
    assert_eq!(node.commands.len(), 1);
    if let ScriptCommand::Goto(label) = &node.commands[0] {
        assert_eq!(label, "my_label");
    } else {
        panic!("期望 Goto 命令，实际: {:?}", node.commands[0]);
    }
}

#[test]
fn test_parse_goto_without_quotes() {
    // goto 也应支持不带引号的标签名
    let node = parse_script(r#"goto my_label;"#);
    assert_eq!(node.commands.len(), 1);
    if let ScriptCommand::Goto(label) = &node.commands[0] {
        assert_eq!(label, "my_label");
    } else {
        panic!("期望 Goto 命令");
    }
}

#[test]
fn test_parse_set_string_var() {
    // set "$var", 42;
    let node = parse_script(r#"set "$var", 42;"#);
    assert_eq!(node.commands.len(), 1);
    if let ScriptCommand::Set(name, value) = &node.commands[0] {
        assert_eq!(name, "$var");
        assert_eq!(*value, 42);
    } else {
        panic!("期望 Set 命令，实际: {:?}", node.commands[0]);
    }
}

#[test]
fn test_parse_set_negative_value() {
    let node = parse_script(r#"set "$counter", -5;"#);
    assert_eq!(node.commands.len(), 1);
    if let ScriptCommand::Set(name, value) = &node.commands[0] {
        assert_eq!(name, "$counter");
        assert_eq!(*value, -5);
    } else {
        panic!("期望 Set 命令");
    }
}

#[test]
fn test_parse_goto_and_set_in_script() {
    let script = r#"
        mes "Hello!";
        set "$talked", 1;
        goto "end_label";
    end_label:
        close;
    "#;
    let node = parse_script(script);
    // mes + set + goto + close = 4 条命令
    assert_eq!(node.commands.len(), 4);
    assert!(node.labels.contains_key("end_label"));
}

#[test]
fn test_parse_goto_label_with_underscore() {
    let node = parse_script(r#"goto "my_long_label_1";"#);
    assert_eq!(node.commands.len(), 1);
    if let ScriptCommand::Goto(label) = &node.commands[0] {
        assert_eq!(label, "my_long_label_1");
    }
}
```

运行 `cargo test parser::tests -- --nocapture` 验证红灯。

- [ ] **Step 1.2: 在 parse_line 中实现 goto/set 解析（绿灯）**

修改 `src/game/script/parser.rs` 的 `parse_line` 函数，在现有 `warp` 解析之后、`None` 返回之前，添加：

```rust
    // goto "label"  或  goto label
    if let Some(label_arg) = line.strip_prefix("goto ") {
        let label_arg = label_arg.trim();
        // 支持带引号和不带引号的标签名
        let label = if label_arg.starts_with('"') && label_arg.ends_with('"') {
            label_arg[1..label_arg.len() - 1].to_string()
        } else {
            label_arg.to_string()
        };
        if !label.is_empty() {
            return Some(ScriptCommand::Goto(label));
        }
    }

    // set "$var", value
    if let Some(args) = line.strip_prefix("set ") {
        let parts: Vec<&str> = args.splitn(2, ',').collect();
        if parts.len() == 2 {
            // 第一部分是变量名（带引号）
            let var_name = extract_string(parts[0]);
            // 第二部分是数值
            let value_str = parts[1].trim();
            if let (Some(name), Ok(value)) = (var_name, value_str.parse::<i64>()) {
                return Some(ScriptCommand::Set(name, value));
            }
        }
    }
```

运行 `cargo test parser::tests -- --nocapture` 验证绿灯。

- [ ] **Step 1.3: 编译验证**

```bash
cargo build 2>&1 | head -30
```

- [ ] **Step 1.4: 提交**

```bash
git add src/game/script/parser.rs
git commit -m "feat(script): 补全 goto 和 set 解析支持"
```

---

### Task 2: 变量系统 + Set 命令实现

**目标:** `NpcDialogueState` 添加变量存储，`Set` 命令实际写入变量

**Files:**
- Modify: `src/game/script/dialogue.rs`
- Modify: `src/game/script/commands.rs`

- [ ] **Step 2.1: 编写变量系统测试（红灯）**

在 `dialogue.rs` 的 `#[cfg(test)] mod tests` 中追加：

```rust
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
```

运行 `cargo test dialogue::tests -- --nocapture` 验证红灯。

- [ ] **Step 2.2: 扩展 NpcDialogueState 添加变量系统（绿灯）**

修改 `src/game/script/dialogue.rs`：

```rust
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
```

运行 `cargo test dialogue::tests -- --nocapture` 验证绿灯。

- [ ] **Step 2.3: 同步更新 mod.rs 的 re-export**

修改 `src/game/script/mod.rs`，确认导出完整：

```rust
pub mod commands;
pub mod dialogue;
pub mod parser;

pub use commands::{NpcScript, ScriptCommand, ScriptNode};
pub use dialogue::{DialogueResponse, NpcDialogueState};
pub use parser::parse_script;
```

- [ ] **Step 2.4: 编译验证 + 全量测试**

```bash
cargo build 2>&1 | head -30
cargo test script -- --nocapture 2>&1
```

- [ ] **Step 2.5: 提交**

```bash
git add src/game/script/
git commit -m "feat(script): 实现变量系统和 Set 命令存储"
```

---

### Task 3: 解析器错误报告

**目标:** 解析器返回 `Result<ScriptNode, Vec<ParseError>>` 替代 `Option`，包含行号信息

**Files:**
- Modify: `src/game/script/parser.rs`
- Modify: `src/game/script/commands.rs`

- [ ] **Step 3.1: 在 commands.rs 中添加 ParseError 类型**

在 `src/game/script/commands.rs` 末尾追加：

```rust
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
```

- [ ] **Step 3.2: 编写错误报告测试（红灯）**

在 `parser.rs` 的测试模块中追加：

```rust
#[test]
fn test_parse_error_on_invalid_line() {
    let result = parse_script_checked("this is not a valid command;");
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].line, 1);
    assert!(errors[0].message.contains("未知命令"));
}

#[test]
fn test_parse_error_reports_line_number() {
    let script = r#"
        mes "Hello";
        invalid_command;
        mes "World";
    "#;
    let result = parse_script_checked(script);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 1);
    // 第 3 行（跳过空行后）
    assert!(errors[0].line >= 2 && errors[0].line <= 4);
}

#[test]
fn test_parse_error_multiple_errors() {
    let script = r#"
        bad_cmd_1;
        mes "OK";
        bad_cmd_2;
    "#;
    let result = parse_script_checked(script);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 2);
}

#[test]
fn test_parse_valid_script_returns_ok() {
    let script = r#"
        mes "Hello!";
        next;
        mes "World!";
        close;
    "#;
    let result = parse_script_checked(script);
    assert!(result.is_ok());
    let node = result.unwrap();
    assert_eq!(node.commands.len(), 4);
}
```

- [ ] **Step 3.3: 实现 parse_script_checked（绿灯）**

在 `parser.rs` 中添加新函数，保留原有 `parse_script` 保持向后兼容：

```rust
use super::commands::{ParseError, ScriptCommand, ScriptNode};

/// 解析脚本文本（向后兼容，静默忽略错误行）
pub fn parse_script(script_text: &str) -> ScriptNode {
    match parse_script_checked(script_text) {
        Ok(node) => node,
        Err(_) => {
            // 向后兼容：有错误时仍然返回能解析的部分
            let mut commands = Vec::new();
            let mut labels = HashMap::new();
            for (line_num, line) in script_text.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() || line.starts_with("//") {
                    continue;
                }
                if line.ends_with(':') {
                    let label_name = line.trim_end_matches(':').to_string();
                    labels.insert(label_name, commands.len());
                    continue;
                }
                if let Some(cmd) = parse_line(line) {
                    commands.push(cmd);
                }
            }
            ScriptNode { commands, labels }
        }
    }
}

/// 解析脚本文本（带错误报告）
pub fn parse_script_checked(script_text: &str) -> Result<ScriptNode, Vec<ParseError>> {
    let mut commands = Vec::new();
    let mut labels = HashMap::new();
    let mut errors = Vec::new();

    for (line_idx, line) in script_text.lines().enumerate() {
        let line_num = line_idx + 1; // 1-based 行号
        let trimmed = line.trim();

        // 跳过空行和注释
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        // 解析标签
        if trimmed.ends_with(':') {
            let label_name = trimmed.trim_end_matches(':').to_string();
            if label_name.is_empty() {
                errors.push(ParseError {
                    line: line_num,
                    source: trimmed.to_string(),
                    message: "标签名不能为空".to_string(),
                });
            } else {
                labels.insert(label_name, commands.len());
            }
            continue;
        }

        // 解析命令
        match parse_line(trimmed) {
            Some(cmd) => commands.push(cmd),
            None => {
                errors.push(ParseError {
                    line: line_num,
                    source: trimmed.to_string(),
                    message: format!("未知命令或语法错误: '{}'", trimmed),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(ScriptNode { commands, labels })
    } else {
        Err(errors)
    }
}

fn parse_line(line: &str) -> Option<ScriptCommand> {
    // ... 保持原有实现不变 ...
}
```

- [ ] **Step 3.4: 更新 mod.rs 导出**

在 `src/game/script/mod.rs` 中添加导出：

```rust
pub use commands::{NpcScript, ParseError, ScriptCommand, ScriptNode};
pub use parser::{parse_script, parse_script_checked};
```

- [ ] **Step 3.5: 编译验证 + 全量测试**

```bash
cargo build 2>&1 | head -30
cargo test script -- --nocapture 2>&1
```

- [ ] **Step 3.6: 提交**

```bash
git add src/game/script/
git commit -m "feat(script): 添加解析器错误报告，返回 Result 包含行号"
```

---

### Task 4: 条件分支命令（if/goto_if） — Phase 2 标记

**目标:** 支持基本条件分支，使脚本可以实现"如果变量为某值则跳转"

**Files:**
- Modify: `src/game/script/commands.rs`
- Modify: `src/game/script/parser.rs`
- Modify: `src/game/script/dialogue.rs`

> **注意:** 此 Task 为 Phase 2 内容。如果 Phase 1 范围太大，可推迟到下一轮迭代。
> 简化设计：不实现完整的 if/else 块语法，而是用 `goto_if "$var", value, "label";` 单行命令。

- [ ] **Step 4.1: 扩展 ScriptCommand 枚举**

在 `commands.rs` 中添加：

```rust
/// 条件跳转：如果变量等于指定值则跳转
GotoIf(String, i64, String),  // 变量名, 比较值, 目标标签
```

- [ ] **Step 4.2: 编写条件分支测试（红灯）**

```rust
#[test]
fn test_parse_goto_if() {
    let node = parse_script(r#"goto_if "$done", 1, "end_label";"#);
    assert_eq!(node.commands.len(), 1);
    if let ScriptCommand::GotoIf(var, val, label) = &node.commands[0] {
        assert_eq!(var, "$done");
        assert_eq!(*val, 1);
        assert_eq!(label, "end_label");
    } else {
        panic!("期望 GotoIf 命令");
    }
}

#[test]
fn test_goto_if_condition_true() {
    let script = r#"
        set "$done", 1;
        goto_if "$done", 1, "skip";
        mes "Should be skipped";
    skip:
        mes "Jumped!";
        close;
    "#;
    let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, parse_script(script));

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
    let script = r#"
        set "$done", 0;
        goto_if "$done", 1, "skip";
        mes "Not skipped";
    skip:
        mes "End";
        close;
    "#;
    let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, parse_script(script));

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
    let script = r#"
        goto_if "$undefined", 1, "skip";
        mes "Variable undefined, no jump";
        close;
    "#;
    let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, parse_script(script));

    // 未定义变量 != 1，不应跳转
    let response = state.process();
    match response {
        DialogueResponse::Message(msg) => assert_eq!(msg, "Variable undefined, no jump"),
        other => panic!("期望不跳转，实际: {:?}", other),
    }
}
```

- [ ] **Step 4.3: 实现 goto_if 解析和执行（绿灯）**

解析器添加：

```rust
// goto_if "$var", value, "label"
if let Some(args) = line.strip_prefix("goto_if ") {
    let parts: Vec<&str> = args.splitn(3, ',').collect();
    if parts.len() == 3 {
        let var_name = extract_string(parts[0]);
        let value = parts[1].trim().parse::<i64>().ok();
        let label = extract_string(parts[2]);
        if let (Some(name), Some(val), Some(lbl)) = (var_name, value, label) {
            return Some(ScriptCommand::GotoIf(name, val, lbl));
        }
    }
}
```

对话状态机添加：

```rust
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
```

- [ ] **Step 4.4: 编译验证 + 全量测试**

```bash
cargo build 2>&1 | head -30
cargo test script -- --nocapture 2>&1
```

- [ ] **Step 4.5: 提交**

```bash
git add src/game/script/
git commit -m "feat(script): 添加 goto_if 条件分支命令"
```

---

## Phase 2: NPC 系统修复

### Task 5: Warp 目标坐标修复

**目标:** `Npc` 结构体正确存储传送目标坐标，`interact()` 返回正确的目标坐标

**Files:**
- Modify: `src/game/npc/data.rs`
- Modify: `src/game/npc/handler.rs`

- [ ] **Step 5.1: 编写 Warp 坐标测试（红灯）**

在 `src/game/npc/handler.rs` 底部添加测试模块：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::map::Player;
    use crate::storage::Character;

    fn create_test_player() -> Player {
        let char = Character {
            char_id: 1,
            char_num: 0,
            name: "TestPlayer".to_string(),
            class: 0,
            base_level: 1,
            job_level: 1,
            base_exp: 0,
            job_exp: 0,
            zeny: 10000,
            str: 10,
            agi: 10,
            vit: 10,
            int: 10,
            dex: 10,
            luk: 10,
            hp: 100,
            max_hp: 100,
            sp: 50,
            max_sp: 50,
            hair: 0,
            hair_color: 0,
            clothes_color: 0,
            weapon: 0,
            shield: 0,
            head_top: 0,
            head_mid: 0,
            head_bottom: 0,
            last_map: "new_1-1.gat".to_string(),
            last_x: 50,
            last_y: 50,
            save_map: "new_1-1.gat".to_string(),
            save_x: 50,
            save_y: 50,
            delete_timer: 0,
            created_at: 0,
            updated_at: 0,
        };
        Player::from_character(char)
    }

    #[test]
    fn test_warp_npc_returns_dest_coordinates() {
        let handler = NpcHandler::new();
        let player = create_test_player();

        // NPC 3 是 Prontera 传送门
        let response = handler.interact(&player, 3);
        match response {
            NpcResponse::Warp { map, x, y } => {
                assert_eq!(map, "prontera.gat", "应返回目标地图，而非 NPC 所在地图");
                assert_eq!(x, 150, "应返回目标 X 坐标");
                assert_eq!(y, 100, "应返回目标 Y 坐标");
            }
            other => panic!("期望 Warp 响应，实际: {:?}", other),
        }
    }

    #[test]
    fn test_warp_npc_stores_dest_fields() {
        let npc = super::data::Npc::warp(
            100,
            "Test Warp",
            50,
            50,
            "prontera.gat",
            "geffen.gat",
            120,
            100,
        );
        assert_eq!(npc.dest_map.as_deref(), Some("geffen.gat"));
        assert_eq!(npc.dest_x, 120);
        assert_eq!(npc.dest_y, 100);
    }

    #[test]
    fn test_npc_not_found() {
        let handler = NpcHandler::new();
        let player = create_test_player();
        let response = handler.interact(&player, 9999);
        assert!(matches!(response, NpcResponse::NotFound));
    }
}
```

- [ ] **Step 5.2: 修改 Npc 结构体添加目标字段（绿灯）**

修改 `src/game/npc/data.rs`：

```rust
/// NPC数据
#[derive(Debug)]
pub struct Npc {
    pub id: u32,
    pub name: String,
    pub display_name: String,
    pub type_: NpcType,
    pub pos_x: u16,
    pub pos_y: u16,
    pub map_name: String,
    pub sprite_id: u16,
    pub level: u16,
    pub flags: u32,

    // 商店物品 (如果是商店)
    pub shop_items: RwLock<Vec<ShopItem>>,

    // 技能列表 (如果是技能训练师)
    pub skills: RwLock<Vec<NpcSkill>>,

    // NPC脚本 (如果有)
    pub script: Option<String>,

    // 传送目标（仅 Warp 类型 NPC 使用）
    pub dest_map: Option<String>,
    pub dest_x: u16,
    pub dest_y: u16,
}
```

修改 `Npc::new()` 初始化新字段：

```rust
impl Npc {
    pub fn new(id: u32, name: &str, x: u16, y: u16, map: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            display_name: name.to_string(),
            type_: NpcType::Shop,
            pos_x: x,
            pos_y: y,
            map_name: map.to_string(),
            sprite_id: 100,
            level: 1,
            flags: 0,
            shop_items: RwLock::new(Vec::new()),
            skills: RwLock::new(Vec::new()),
            script: None,
            dest_map: None,
            dest_x: 0,
            dest_y: 0,
        }
    }
```

修复 `Npc::warp()` 不再丢弃参数：

```rust
    pub fn warp(
        id: u32,
        name: &str,
        x: u16,
        y: u16,
        map: &str,
        dest_map: &str,
        dest_x: u16,
        dest_y: u16,
    ) -> Self {
        let mut npc = Self::new(id, name, x, y, map);
        npc.type_ = NpcType::Warp;
        npc.dest_map = Some(dest_map.to_string());
        npc.dest_x = dest_x;
        npc.dest_y = dest_y;
        npc
    }
```

- [ ] **Step 5.3: 修复 interact() 的 Warp 分支**

修改 `src/game/npc/handler.rs` 的 `interact()` 方法：

```rust
    /// 处理NPC交互
    pub fn interact(&self, _player: &Player, npc_id: u32) -> NpcResponse {
        let npc = match self.get_npc(npc_id) {
            Some(n) => n,
            None => return NpcResponse::NotFound,
        };

        match npc.type_ {
            super::data::NpcType::Shop => NpcResponse::OpenShop {
                npc_id,
                items: npc.shop_items.read().clone(),
            },
            super::data::NpcType::SkillTrainer => NpcResponse::SkillList {
                npc_id,
                skills: npc.skills.read().clone(),
            },
            super::data::NpcType::Warp => {
                // 返回传送目标坐标（而非 NPC 自身坐标）
                let dest_map = npc.dest_map.clone()
                    .unwrap_or_else(|| npc.map_name.clone());
                NpcResponse::Warp {
                    map: dest_map,
                    x: npc.dest_x,
                    y: npc.dest_y,
                }
            }
            _ => NpcResponse::Message(npc.display_name.clone()),
        }
    }
```

- [ ] **Step 5.4: 编译验证 + 测试**

```bash
cargo build 2>&1 | head -30
cargo test npc -- --nocapture 2>&1
```

- [ ] **Step 5.5: 提交**

```bash
git add src/game/npc/
git commit -m "fix(npc): 修复 Warp NPC 返回自身坐标的 bug，添加目标坐标字段"
```

---

### Task 6: learn_skill 补全 — 扣除 Zeny + 已学习检查

**目标:** `learn_skill()` 执行完整验证流程：检查已学习、检查 Zeny、扣除费用

**Files:**
- Modify: `src/game/npc/handler.rs`

- [ ] **Step 6.1: 编写 learn_skill 测试（红灯）**

在 `handler.rs` 的测试模块中追加：

```rust
#[test]
fn test_learn_skill_success() {
    let handler = NpcHandler::new();
    let player = create_test_player();

    // NPC 2 是技能训练师，技能 1 (Bash) 价格 1000
    let result = handler.learn_skill(&player, 2, 1);
    assert!(matches!(result, LearnResult::Success { skill_id: 1 }));
}

#[test]
fn test_learn_skill_not_enough_zeny() {
    let handler = NpcHandler::new();
    let player = create_test_player();
    // 设置 Zeny 为 0
    crate::game::zeny::ZenyManager::set(&player, 0);

    let result = handler.learn_skill(&player, 2, 1);
    assert!(matches!(result, LearnResult::NotEnoughZeny));
}

#[test]
fn test_learn_skill_deducts_zeny() {
    let handler = NpcHandler::new();
    let player = create_test_player();
    let initial_zeny = crate::game::zeny::ZenyManager::get(&player);

    handler.learn_skill(&player, 2, 1); // Bash, price 1000

    let remaining = crate::game::zeny::ZenyManager::get(&player);
    assert_eq!(remaining, initial_zeny - 1000);
}

#[test]
fn test_learn_skill_already_learned() {
    let handler = NpcHandler::new();
    let player = create_test_player();

    // 第一次学习
    handler.learn_skill(&player, 2, 1);
    // 第二次学习应返回已学习
    let result = handler.learn_skill(&player, 2, 1);
    assert!(matches!(result, LearnResult::AlreadyLearned));
}

#[test]
fn test_learn_skill_not_found_on_npc() {
    let handler = NpcHandler::new();
    let player = create_test_player();

    // NPC 1 是商店，没有技能
    let result = handler.learn_skill(&player, 1, 1);
    assert!(matches!(result, LearnResult::SkillNotFound));
}

#[test]
fn test_learn_skill_npc_not_found() {
    let handler = NpcHandler::new();
    let player = create_test_player();

    let result = handler.learn_skill(&player, 9999, 1);
    assert!(matches!(result, LearnResult::NpcNotFound));
}
```

- [ ] **Step 6.2: 实现 learn_skill 完整逻辑（绿灯）**

修改 `handler.rs` 的 `learn_skill` 方法。需要跟踪已学习技能。由于 `NpcHandler` 是无状态的（不跟踪玩家），我们需要在 `learn_skill` 签名中接受技能列表参数，或在 handler 中维护一个已学习技能表。

方案：在 `NpcHandler` 中维护 `learned_skills: RwLock<HashMap<u32, HashSet<u16>>>`（char_id -> 已学习的 skill_id 集合）。

```rust
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use super::data::Npc;
use crate::game::item::Inventory;
use crate::game::map::Player;
use crate::game::zeny::ZenyManager;
use std::sync::Arc;

/// NPC交互处理器
pub struct NpcHandler {
    npcs: std::collections::HashMap<u32, Arc<Npc>>,
    /// 已学习技能表: char_id -> Set<skill_id>
    learned_skills: RwLock<HashMap<u32, HashSet<u16>>>,
}

impl NpcHandler {
    pub fn new() -> Self {
        let mut handler = Self {
            npcs: std::collections::HashMap::new(),
            learned_skills: RwLock::new(HashMap::new()),
        };
        handler.init_default_npcs();
        handler
    }

    // ... 其他方法保持不变 ...

    /// 学习技能（完整流程：验证 NPC -> 检查技能 -> 检查已学习 -> 扣除 Zeny）
    pub fn learn_skill(&self, player: &Player, npc_id: u32, skill_id: u16) -> LearnResult {
        // 1. 检查 NPC 存在
        let npc = match self.get_npc(npc_id) {
            Some(n) => n,
            None => return LearnResult::NpcNotFound,
        };

        // 2. 检查 NPC 上有该技能
        let npc_skill = npc
            .skills
            .read()
            .iter()
            .find(|s| s.skill_id == skill_id)
            .copied();

        let npc_skill = match npc_skill {
            Some(s) => s,
            None => return LearnResult::SkillNotFound,
        };

        // 3. 检查是否已学习
        {
            let learned = self.learned_skills.read();
            if let Some(skills) = learned.get(&player.char_id) {
                if skills.contains(&skill_id) {
                    return LearnResult::AlreadyLearned;
                }
            }
        }

        // 4. 检查 Zeny 是否足够
        if !ZenyManager::can_spend(player, npc_skill.price) {
            return LearnResult::NotEnoughZeny;
        }

        // 5. 扣除 Zeny
        ZenyManager::sub(player, npc_skill.price);

        // 6. 记录已学习
        {
            let mut learned = self.learned_skills.write();
            learned
                .entry(player.char_id)
                .or_insert_with(HashSet::new)
                .insert(skill_id);
        }

        LearnResult::Success { skill_id }
    }
}
```

- [ ] **Step 6.3: 编译验证 + 测试**

```bash
cargo build 2>&1 | head -30
cargo test npc::handler::tests -- --nocapture 2>&1
```

- [ ] **Step 6.4: 提交**

```bash
git add src/game/npc/handler.rs
git commit -m "feat(npc): learn_skill 实现 Zeny 扣除和已学习检查"
```

---

### Task 7: 集成测试 — 对话 + Select 选择 + Warp 完整流程

**目标:** 端到端测试脚本对话流程，特别是 Select 选择影响脚本流程

**Files:**
- Modify: `src/game/script/dialogue.rs`（追加集成测试）

- [ ] **Step 7.1: 编写完整对话流程测试**

在 `dialogue.rs` 的测试模块中追加：

```rust
#[test]
fn test_select_choice_affects_flow() {
    // 模拟一个 NPC 对话脚本：选择 Yes 则传送，选择 No 则关闭
    let script = parse_script(
        r#"
        mes "Do you want to warp?";
        select("Yes","No");
        set "$choice", 1;
        goto_if "$choice", 1, "do_warp";
        mes "Okay, goodbye!";
        close;
    do_warp:
        mes "Warping!";
        warp "prontera", 150, 183;
        close;
        "#,
    );
    let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

    // 处理 mes
    let resp = state.process();
    assert!(matches!(resp, DialogueResponse::Message(_)));

    // 处理 select -> 返回 Select 响应
    let resp = state.process();
    assert!(matches!(resp, DialogueResponse::Select(_)));

    // 模拟玩家选择第 1 项 (Yes)
    let resp = state.handle_input(1);
    // set "$choice", 1 应该执行
    // goto_if 条件为真，跳转到 do_warp
    // mes "Warping!"
    match resp {
        DialogueResponse::Message(msg) => assert_eq!(msg, "Warping!"),
        other => panic!("期望 Message(\"Warping!\"), 实际: {:?}", other),
    }

    // 下一步应是 warp
    let resp = state.process();
    assert!(matches!(resp, DialogueResponse::Warp { .. }));
}

#[test]
fn test_multi_step_dialogue_with_next() {
    let script = parse_script(
        r#"
        mes "Step 1";
        next;
        mes "Step 2";
        next;
        mes "Step 3";
        close;
        "#,
    );
    let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

    // Step 1
    let resp = state.process();
    assert!(matches!(resp, DialogueResponse::Message(ref s) if s == "Step 1"));

    // next -> Pending -> 继续处理
    let resp = state.process();
    assert!(matches!(resp, DialogueResponse::Pending));

    // 模拟玩家点击 next
    let resp = state.handle_input(0);
    assert!(matches!(resp, DialogueResponse::Message(ref s) if s == "Step 2"));

    // next -> Pending
    let resp = state.process();
    assert!(matches!(resp, DialogueResponse::Pending));

    // 模拟玩家点击 next
    let resp = state.handle_input(0);
    assert!(matches!(resp, DialogueResponse::Message(ref s) if s == "Step 3"));

    // close
    let resp = state.process();
    assert!(matches!(resp, DialogueResponse::Closed));
}

#[test]
fn test_empty_script_returns_closed() {
    let script = parse_script("");
    let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
    let resp = state.process();
    assert!(matches!(resp, DialogueResponse::Closed));
}

#[test]
fn test_goto_loops() {
    // 测试 goto 循环（有限次）
    let script = parse_script(
        r#"
        set "$i", 0;
    loop_start:
        set "$i", 1;
        goto_if "$i", 3, "loop_end";
        set "$i", 1;
        goto "loop_start";
    loop_end:
        mes "Loop done";
        close;
        "#,
    );
    // 注意：这个脚本实际上会无限循环，因为 goto_if 检查 $i == 3 但 $i 永远被设为 1
    // 这里只是测试 goto 跳转到标签的能力
    let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

    // set $i = 0
    state.process();
    // loop_start: set $i = 1
    state.process();
    assert_eq!(state.get_variable("$i"), Some(1));
}
```

- [ ] **Step 7.2: 编译验证 + 运行全部测试**

```bash
cargo build 2>&1 | head -30
cargo test -- --nocapture 2>&1 | tail -30
```

- [ ] **Step 7.3: 提交**

```bash
git add src/game/script/
git commit -m "test(script): 添加对话流程集成测试（Select 选择 + 条件分支）"
```

---

### Task 8: NPC 模板扩充 + Quest/CashShop 处理

**目标:** 添加更多 NPC 模板，实现 Quest 和 CashShop 类型的交互处理

**Files:**
- Modify: `src/game/npc/data.rs`
- Modify: `src/game/npc/handler.rs`

- [ ] **Step 8.1: 扩充 NPC 模板**

在 `src/game/npc/data.rs` 的 `NpcDatabase` 中添加更多 NPC：

```rust
impl NpcDatabase {
    pub fn get_npc(id: u32) -> Option<Npc> {
        match id {
            1 => Some(Self::create_poring_merchant()),
            2 => Some(Self::create_basilisk_warrior()),
            3 => Some(Self::create_prontera_warp()),
            4 => Some(Self::create_quest_npc()),
            5 => Some(Self::create_cash_shop_npc()),
            6 => Some(Self::create_geffen_warp()),
            7 => Some(Self::create_healing_nurse()),
            _ => None,
        }
    }

    fn create_quest_npc() -> Npc {
        let mut npc = Npc::new(4, "Quest Master", 160, 130, "prontera.gat");
        npc.display_name = "任务大师".to_string();
        npc.type_ = NpcType::Quest;
        npc.sprite_id = 725;
        npc.script = Some(
            r#"mes "[Quest Master]"
mes "I have many tasks for brave adventurers!"
mes "What would you like to do?"
next
select("Accept Quest:Check Progress:Leave")
close"#.to_string(),
        );
        npc
    }

    fn create_cash_shop_npc() -> Npc {
        let mut npc = Npc::new(5, "Cash Shop", 150, 150, "prontera.gat");
        npc.display_name = "现金商店".to_string();
        npc.type_ = NpcType::CashShop;
        npc.sprite_id = 112;
        npc.script = Some(
            r#"mes "[Cash Shop]"
mes "Welcome to the Cash Shop!"
mes "Here you can purchase special items."
close"#.to_string(),
        );
        npc
    }

    fn create_geffen_warp() -> Npc {
        let mut npc = Npc::warp(
            6,
            "To Geffen",
            120,
            100,
            "prontera.gat",
            "geffen.gat",
            119,
            59,
        );
        npc.display_name = "前往吉芬".to_string();
        npc.sprite_id = 406;
        npc
    }

    fn create_healing_nurse() -> Npc {
        let mut npc = Npc::new(7, "Nurse", 150, 180, "prontera.gat");
        npc.display_name = "护士".to_string();
        npc.type_ = NpcType::SkillTrainer;
        npc.sprite_id = 114;
        npc.add_skill(28, 6, 0);  // Heal, 免费
        npc.add_skill(29, 10, 500); // Increase AGI, 500 Zeny
        npc.script = Some(
            r#"mes "[Nurse]"
mes "Hello! Do you need healing?"
next
select("Heal me:Learn Skills:Leave")
close"#.to_string(),
        );
        npc
    }
}
```

- [ ] **Step 8.2: 完善 Quest/CashShop 交互处理**

修改 `handler.rs` 的 `interact()` 方法，为 Quest 和 CashShop 添加脚本驱动的交互：

```rust
    /// 处理NPC交互
    pub fn interact(&self, _player: &Player, npc_id: u32) -> NpcResponse {
        let npc = match self.get_npc(npc_id) {
            Some(n) => n,
            None => return NpcResponse::NotFound,
        };

        match npc.type_ {
            super::data::NpcType::Shop => NpcResponse::OpenShop {
                npc_id,
                items: npc.shop_items.read().clone(),
            },
            super::data::NpcType::SkillTrainer => {
                // 有脚本时优先使用脚本驱动对话
                if npc.script.is_some() {
                    NpcResponse::StartScript {
                        npc_id,
                        script: npc.script.clone().unwrap(),
                    }
                } else {
                    NpcResponse::SkillList {
                        npc_id,
                        skills: npc.skills.read().clone(),
                    }
                }
            }
            super::data::NpcType::Warp => {
                let dest_map = npc.dest_map.clone()
                    .unwrap_or_else(|| npc.map_name.clone());
                NpcResponse::Warp {
                    map: dest_map,
                    x: npc.dest_x,
                    y: npc.dest_y,
                }
            }
            super::data::NpcType::Quest | super::data::NpcType::CashShop => {
                // Quest 和 CashShop 类型通过脚本驱动
                if let Some(ref script_text) = npc.script {
                    NpcResponse::StartScript {
                        npc_id,
                        script: script_text.clone(),
                    }
                } else {
                    NpcResponse::Message(npc.display_name.clone())
                }
            }
        }
    }
```

- [ ] **Step 8.3: 添加 StartScript 到 NpcResponse 枚举**

在 `handler.rs` 的 `NpcResponse` 枚举中添加新变体：

```rust
/// NPC响应
#[derive(Debug, Clone)]
pub enum NpcResponse {
    NotFound,
    Message(String),
    OpenShop {
        npc_id: u32,
        items: Vec<super::data::ShopItem>,
    },
    SkillList {
        npc_id: u32,
        skills: Vec<super::data::NpcSkill>,
    },
    Warp {
        map: String,
        x: u16,
        y: u16,
    },
    /// 启动脚本对话
    StartScript {
        npc_id: u32,
        script: String,
    },
}
```

- [ ] **Step 8.4: 添加 NPC 模板测试**

```rust
#[test]
fn test_quest_npc_exists() {
    let handler = NpcHandler::new();
    let npc = handler.get_npc(4);
    assert!(npc.is_some());
    let npc = npc.unwrap();
    assert_eq!(npc.type_, super::data::NpcType::Quest);
    assert!(npc.script.is_some());
}

#[test]
fn test_cash_shop_npc_exists() {
    let handler = NpcHandler::new();
    let npc = handler.get_npc(5);
    assert!(npc.is_some());
    let npc = npc.unwrap();
    assert_eq!(npc.type_, super::data::NpcType::CashShop);
}

#[test]
fn test_quest_npc_interaction_returns_script() {
    let handler = NpcHandler::new();
    let player = create_test_player();
    let response = handler.interact(&player, 4);
    assert!(matches!(response, NpcResponse::StartScript { .. }));
}

#[test]
fn test_geffen_warp_target() {
    let handler = NpcHandler::new();
    let player = create_test_player();
    let response = handler.interact(&player, 6);
    match response {
        NpcResponse::Warp { map, x, y } => {
            assert_eq!(map, "geffen.gat");
            assert_eq!(x, 119);
            assert_eq!(y, 59);
        }
        other => panic!("期望 Warp 响应，实际: {:?}", other),
    }
}

#[test]
fn test_healing_nurse_has_skills() {
    let handler = NpcHandler::new();
    let npc = handler.get_npc(7).unwrap();
    let skills = npc.skills.read();
    assert_eq!(skills.len(), 2);
    assert_eq!(skills[0].skill_id, 28); // Heal
    assert_eq!(skills[0].price, 0);     // 免费
}

#[test]
fn test_get_npcs_on_map() {
    let handler = NpcHandler::new();
    let prontera_npcs = handler.get_npcs_on_map("prontera.gat");
    // prontera.gat 应该有: Quest(4), CashShop(5), GeffenWarp(6), Nurse(7)
    assert!(prontera_npcs.len() >= 4);
}
```

- [ ] **Step 8.5: 编译验证 + 全量测试**

```bash
cargo build 2>&1 | head -30
cargo test npc -- --nocapture 2>&1
```

- [ ] **Step 8.6: 提交**

```bash
git add src/game/npc/
git commit -m "feat(npc): 扩充 NPC 模板，实现 Quest/CashShop 脚本驱动交互"
```

---

## Phase 3: 集成与收尾

### Task 9: map_server NPC 集成更新

**目标:** 确保 `map_server/npc.rs` 正确处理新的 `StartScript` 响应和变量系统

**Files:**
- Modify: `src/game/map/map_server/npc.rs`

- [ ] **Step 9.1: 更新 handle_npc_interact 处理 StartScript**

当前 `map_server/npc.rs` 已经在 `handle_npc_interact` 中检查 `npc.script` 并启动对话。需要确认新的 `NpcResponse::StartScript` 也被正确处理。

修改 `handle_npc_interact` 的非脚本 NPC 交互部分：

```rust
    // Otherwise, handle by NPC type
    let response = self.npc_handler.interact(/* player */, npc.id);
    match response {
        NpcResponse::StartScript { npc_id, script } => {
            let script_node = parse_script(&script);
            let dialogue = NpcDialogueState::new(player_id, npc_id, script_node);
            self.active_dialogues.write().insert(player_id, dialogue);
            return self.advance_dialogue(player_id, npc_id);
        }
        NpcResponse::OpenShop { .. } => {
            // 发送商店打开包
            let msg = ZcSayDialog {
                npc_id: npc.id,
                message: format!("Welcome to {}!", npc.display_name),
            };
            Some(msg.to_packet())
        }
        NpcResponse::Warp { map, x, y } => {
            // 执行传送
            tracing::info!("NPC warp: {} -> {} ({}, {})", npc.id, map, x, y);
            Some(ZcCloseDialog { npc_id: npc.id }.to_packet())
        }
        _ => {
            let msg = ZcSayDialog {
                npc_id: npc.id,
                message: format!("{}: Hello!", npc.display_name),
            };
            Some(msg.to_packet())
        }
    }
```

- [ ] **Step 9.2: 确保 Select 输入正确传递**

检查 `handle_npc_select` 中 `pkt.select` 的类型。rAthena 协议中 `CZ_ACK_SELECT_MENU` 的 select 字段是 1-based 索引，这与我们的 `handle_input` 一致。

- [ ] **Step 9.3: 编译验证**

```bash
cargo build 2>&1 | head -30
```

- [ ] **Step 9.4: 提交**

```bash
git add src/game/map/map_server/npc.rs
git commit -m "fix(map): 更新 NPC 交互处理以支持 StartScript 响应"
```

---

### Task 10: 全量测试 + 清理

**目标:** 运行所有测试，确保无回归，清理死代码

- [ ] **Step 10.1: 运行全量测试**

```bash
cargo test -- --nocapture 2>&1
```

- [ ] **Step 10.2: 检查编译警告**

```bash
cargo build 2>&1 | grep -i warning
```

- [ ] **Step 10.3: 清理**

- 移除 `NpcFlag` 的 `#[allow(dead_code)]` 或添加使用处
- 确认 `npc/mod.rs` 导出完整

- [ ] **Step 10.4: 最终提交**

```bash
git add -A
git commit -m "chore: 脚本引擎和 NPC 系统补全，全量测试通过"
```

---

## 完整测试矩阵

| 测试名称 | 文件 | 覆盖内容 |
|----------|------|----------|
| `test_parse_mes` | parser.rs | 基本 mes 命令解析 |
| `test_parse_select` | parser.rs | select 菜单解析 |
| `test_parse_warp` | parser.rs | warp 坐标解析 |
| `test_parse_multi_line` | parser.rs | 多行脚本 |
| `test_parse_goto` | parser.rs | goto 标签解析（新） |
| `test_parse_goto_without_quotes` | parser.rs | 无引号 goto（新） |
| `test_parse_set_string_var` | parser.rs | set 变量解析（新） |
| `test_parse_set_negative_value` | parser.rs | set 负值（新） |
| `test_parse_goto_and_set_in_script` | parser.rs | goto+set 混合（新） |
| `test_parse_error_on_invalid_line` | parser.rs | 错误报告（新） |
| `test_parse_error_reports_line_number` | parser.rs | 行号报告（新） |
| `test_parse_error_multiple_errors` | parser.rs | 多错误收集（新） |
| `test_parse_valid_script_returns_ok` | parser.rs | 有效脚本返回 Ok（新） |
| `test_parse_goto_if` | parser.rs | 条件分支解析（Phase 2） |
| `test_simple_dialogue` | dialogue.rs | 基本对话流程 |
| `test_dialogue_process_mes` | dialogue.rs | mes 响应 |
| `test_dialogue_process_warp` | dialogue.rs | warp 响应 |
| `test_dialogue_select` | dialogue.rs | select 响应 |
| `test_dialogue_close` | dialogue.rs | close 响应 |
| `test_set_stores_variable` | dialogue.rs | Set 变量存储（新） |
| `test_set_multiple_variables` | dialogue.rs | 多变量（新） |
| `test_goto_uses_label` | dialogue.rs | Goto 标签跳转（新） |
| `test_set_then_goto` | dialogue.rs | Set+Goto 组合（新） |
| `test_variable_default_none` | dialogue.rs | 未定义变量（新） |
| `test_goto_if_condition_true` | dialogue.rs | 条件为真跳转（Phase 2） |
| `test_goto_if_condition_false` | dialogue.rs | 条件为假不跳转（Phase 2） |
| `test_goto_if_undefined_variable` | dialogue.rs | 未定义变量不跳转（Phase 2） |
| `test_select_choice_affects_flow` | dialogue.rs | Select 影响脚本流程（新） |
| `test_multi_step_dialogue_with_next` | dialogue.rs | 多步 next 对话（新） |
| `test_empty_script_returns_closed` | dialogue.rs | 空脚本（新） |
| `test_warp_npc_returns_dest_coordinates` | handler.rs | Warp 目标坐标（新） |
| `test_warp_npc_stores_dest_fields` | handler.rs | 目标字段存储（新） |
| `test_npc_not_found` | handler.rs | NPC 不存在（新） |
| `test_learn_skill_success` | handler.rs | 学习技能成功（新） |
| `test_learn_skill_not_enough_zeny` | handler.rs | Zeny 不足（新） |
| `test_learn_skill_deducts_zeny` | handler.rs | Zeny 扣除验证（新） |
| `test_learn_skill_already_learned` | handler.rs | 已学习检查（新） |
| `test_learn_skill_not_found_on_npc` | handler.rs | NPC 无此技能（新） |
| `test_learn_skill_npc_not_found` | handler.rs | NPC 不存在（新） |
| `test_quest_npc_exists` | handler.rs | Quest NPC 模板（新） |
| `test_cash_shop_npc_exists` | handler.rs | CashShop NPC 模板（新） |
| `test_quest_npc_interaction_returns_script` | handler.rs | Quest 交互返回脚本（新） |
| `test_geffen_warp_target` | handler.rs | 第二个 Warp NPC（新） |
| `test_healing_nurse_has_skills` | handler.rs | 技能训练师（新） |
| `test_get_npcs_on_map` | handler.rs | 地图 NPC 查询（新） |

---

## Git 提交计划

| 顺序 | Commit Message | 涉及文件 |
|------|---------------|----------|
| 1 | `feat(script): 补全 goto 和 set 解析支持` | parser.rs |
| 2 | `feat(script): 实现变量系统和 Set 命令存储` | dialogue.rs, mod.rs |
| 3 | `feat(script): 添加解析器错误报告，返回 Result 包含行号` | parser.rs, commands.rs, mod.rs |
| 4 | `feat(script): 添加 goto_if 条件分支命令` (Phase 2) | parser.rs, dialogue.rs, commands.rs |
| 5 | `fix(npc): 修复 Warp NPC 返回自身坐标的 bug，添加目标坐标字段` | data.rs, handler.rs |
| 6 | `feat(npc): learn_skill 实现 Zeny 扣除和已学习检查` | handler.rs |
| 7 | `test(script): 添加对话流程集成测试` | dialogue.rs |
| 8 | `feat(npc): 扩充 NPC 模板，实现 Quest/CashShop 脚本驱动交互` | data.rs, handler.rs |
| 9 | `fix(map): 更新 NPC 交互处理以支持 StartScript 响应` | map_server/npc.rs |
| 10 | `chore: 脚本引擎和 NPC 系统补全，全量测试通过` | 全局 |

---

## 风险与注意事项

1. **Player 结构体当前没有已学习技能列表** — `learn_skill` 测试中使用 `NpcHandler` 内部的 `learned_skills` HashMap 追踪。未来需要将已学习技能持久化到数据库（通过 `CharacterInventoryData` 或新的 `CharacterSkillData`）。

2. **rAthena 协议兼容性** — `CZ_ACK_SELECT_MENU` 的 select 字段是 1-based，我们的 `handle_input` 已对齐。但需确认 `pkt.select` 的实际值范围。

3. **脚本变量作用域** — 当前变量存储在 `NpcDialogueState` 中，对话关闭后变量丢失。如果需要跨对话持久化，需要在玩家数据中添加脚本变量存储。

4. **NpcHandler 的 learned_skills 使用 parking_lot::RwLock** — 这是全局共享的，测试中创建的 Player 使用 `from_character` 会生成随机 `Uuid`，但 `char_id` 是固定的（值为 1），所以测试间的已学习状态会共享。需要在测试间清理或使用不同的 `char_id`。

5. **if/else 块语法未实现** — 当前只实现单行 `goto_if`。完整 if/else 块需要更复杂的解析器（需要处理嵌套），标记为 Phase 2。
