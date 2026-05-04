# 脚本引擎基础设计

## 概述

实现 NPC 脚本引擎的基础框架，支持基本对话命令。

## MVP 范围

由于完整脚本引擎复杂，MVP 实现：
1. **脚本解析器** - 解析基本命令
2. **对话状态机** - 管理 NPC 对话流程
3. **简单对话 NPC** - 示例实现

## 目标命令

MVP 支持的命令：
- `mes` - 显示消息
- `next` - 下一行
- `close` - 关闭对话
- `select` - 菜单选择

## 设计方案

### 1. 脚本命令枚举

```rust
pub enum ScriptCommand {
    Mes(String),           // 显示消息
    Next,                 // 下一行
    Close,                // 关闭对话
    Select(Vec<String>),   // 选择菜单
    Warp(String, u16, u16), // 传送
    Goto(String),         // 跳转标签
    End,                  // 结束
}
```

### 2. 脚本节点

```rust
pub struct ScriptNode {
    pub commands: Vec<ScriptCommand>,
    pub labels: HashMap<String, usize>,  // 标签 -> 命令索引
}

pub struct NpcScript {
    pub npc_id: u32,
    pub nodes: Vec<ScriptNode>,
}
```

### 3. NPC 对话状态

```rust
pub struct NpcDialogueState {
    pub player_id: Uuid,
    pub npc_id: u32,
    pub current_node: usize,
    pub current_step: usize,
    pub variables: HashMap<String, i64>,
}
```

### 4. 脚本解析

```rust
pub fn parse_script(script_text: &str) -> ScriptNode {
    // 解析脚本文本，生成命令列表
}
```

## 示例 NPC 脚本

```
prontera,150,180,5  script  Welcome  123,{
    mes "欢迎来到 Deviruchi!";
    mes "我可以帮助你开始冒险。";
    next;
    switch (select("前往新手村:查看商店:再见")) {
    case 1:
        mes "好的，我带你去新手村。";
        warp "new_01", 100, 100;
        close;
    case 2:
        mes "商店在城的另一边。";
        close;
    case 3:
        mes "再见，祝你好运!";
        close;
    }
}
```

## 文件结构

```
src/game/script/
  ├── mod.rs         # 模块导出
  ├── parser.rs      # 脚本解析
  ├── commands.rs    # 命令定义
  ├── runtime.rs     # 运行时
  └── dialogue.rs     # 对话管理
```

## 实现步骤

1. 创建 script 模块
2. 实现命令枚举和解析
3. 实现对话状态管理
4. 集成到 NpcHandler
5. 添加示例 NPC
