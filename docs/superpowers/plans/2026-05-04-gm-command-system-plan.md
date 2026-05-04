# GM 命令框架实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development

**Goal:** 实现完整的 @ 命令系统，包括权限检查、命令注册、解析和常用命令

**Architecture:** 集中式命令注册，命令处理器模式

**Tech Stack:** Rust, parking_lot

---

## Task 1: 创建命令模块核心结构

**Files:**
- Create: `src/game/command/mod.rs`
- Create: `src/game/command/atcommand.rs`
- Create: `src/game/command/parser.rs`

- [ ] **Step 1: 创建 mod.rs**

```rust
pub mod atcommand;
pub mod parser;

pub use atcommand::{AtCommandHandler, CommandInfo, CommandResult, CommandHandler};
pub use parser::parse_command;
```

- [ ] **Step 2: 创建 atcommand.rs**

```rust
use std::collections::HashMap;
use parking_lot::RwLock;
use uuid::Uuid;

use crate::game::map::{Player, MapState};

/// 命令处理器类型
pub type CommandHandler = fn(
    player: &Player,
    args: &[String],
    map_state: &MapState,
) -> CommandResult;

/// 命令结果
#[derive(Debug, Clone)]
pub enum CommandResult {
    Success(String),
    Failure(String),
    NoPermission,
}

/// 命令信息
#[derive(Clone)]
pub struct CommandInfo {
    pub name: &'static str,
    pub aliases: Vec<&'static str>,
    pub min_level: u8,
    pub description: &'static str,
    pub usage: &'static str,
    pub handler: CommandHandler,
}

/// @命令处理器
pub struct AtCommandHandler {
    commands: RwLock<HashMap<String, CommandInfo>>,
    permission_levels: HashMap<u8, &'static str>,
}

impl AtCommandHandler {
    pub fn new() -> Self {
        let mut handler = Self {
            commands: RwLock::new(HashMap::new()),
            permission_levels: HashMap::new(),
        };
        handler.init_permission_levels();
        handler
    }

    fn init_permission_levels(&mut self) {
        self.permission_levels.insert(0, "Player");
        self.permission_levels.insert(1, "Support");
        self.permission_levels.insert(10, "GM");
        self.permission_levels.insert(20, "High GM");
        self.permission_levels.insert(50, "Admin");
        self.permission_levels.insert(99, "Super Admin");
    }

    /// 注册命令
    pub fn register(&self, info: CommandInfo) {
        let mut commands = self.commands.write();

        // 注册主名称
        commands.insert(info.name.to_string(), info.clone());

        // 注册别名
        for alias in &info.aliases {
            commands.insert(alias.to_string(), info.clone());
        }
    }

    /// 执行命令
    pub fn execute(&self, player: &Player, input: &str, map_state: &MapState) -> CommandResult {
        let (cmd_name, args) = crate::game::command::parser::parse_command(input);

        let commands = self.commands.read();
        let Some(info) = commands.get(cmd_name) else {
            return CommandResult::Failure(format!("未知命令: @{}", cmd_name));
        };

        // 权限检查
        if !self.check_permission(player, info.min_level) {
            tracing::warn!("Player {} attempted command @{} without permission", player.name, cmd_name);
            return CommandResult::NoPermission;
        }

        // 执行命令
        (info.handler)(player, &args, map_state)
    }

    /// 检查权限
    pub fn check_permission(&self, player: &Player, required_level: u8) -> bool {
        // TODO: 从 player.account_id 查询 group_id
        // 目前简化处理，使用固定等级
        let player_level = self.get_player_level(player);
        player_level >= required_level
    }

    /// 获取玩家权限等级
    fn get_player_level(&self, player: &Player) -> u8 {
        // TODO: 从数据库或缓存获取实际权限等级
        // 目前返回 10 (GM) 以便测试
        10
    }

    /// 获取权限等级名称
    pub fn get_level_name(&self, level: u8) -> &'static str {
        self.permission_levels.get(&level).copied().unwrap_or("Unknown")
    }
}

impl Default for AtCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 3: 创建 parser.rs**

```rust
/// 解析命令字符串
/// 输入: "@warp prontera 100 200"
/// 输出: ("warp", ["prontera", "100", "200"])
pub fn parse_command(input: &str) -> (String, Vec<String>) {
    let input = input.trim();

    // 移除开头的 @
    let input = if let Some(stripped) = input.strip_prefix('@') {
        stripped
    } else {
        return (String::new(), vec![]);
    };

    // 分割命令名和参数
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return (String::new(), vec![]);
    }

    let command = parts[0].to_lowercase();
    let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

    (command, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        let (cmd, args) = parse_command("@warp prontera 100 200");
        assert_eq!(cmd, "warp");
        assert_eq!(args, vec!["prontera", "100", "200"]);
    }

    #[test]
    fn test_parse_command_no_args() {
        let (cmd, args) = parse_command("@heal");
        assert_eq!(cmd, "heal");
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_command_uppercase() {
        let (cmd, args) = parse_command("@WARP prontera");
        assert_eq!(cmd, "warp");
    }

    #[test]
    fn test_parse_command_invalid() {
        let (cmd, args) = parse_command("hello world");
        assert!(cmd.is_empty());
        assert!(args.is_empty());
    }
}
```

- [ ] **Step 4: 编译验证**

Run: `cargo build 2>&1 | head -30`
Expected: 无错误

- [ ] **Step 5: 提交**

```bash
git add src/game/command/
git commit -m "feat(command): add core @command framework"
```

---

## Task 2: 实现常用命令

**Files:**
- Create: `src/game/command/commands/mod.rs`
- Create: `src/game/command/commands/teleport.rs`
- Create: `src/game/command/commands/player.rs`

- [ ] **Step 1: 创建命令模块**

`src/game/command/commands/mod.rs`:
```rust
pub mod teleport;
pub mod player;

use crate::game::command::atcommand::{CommandInfo, CommandHandler};
```

`src/game/command/commands/teleport.rs`:
```rust
use crate::game::map::{Player, MapState};
use crate::game::command::atcommand::CommandResult;

/// @warp <map> [x] [y] - 传送到地图
pub fn cmd_warp(player: &Player, args: &[String], _map_state: &MapState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Failure("用法: @warp <地图> [x] [y]".to_string());
    }

    let map_name = &args[0];
    let x: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100);
    let y: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100);

    player.map_name = map_name.clone();
    player.move_to(x, y);

    CommandResult::Success(format!("传送到 {} ({}, {})", map_name, x, y))
}

/// @goto <player> - 传送到玩家位置
pub fn cmd_goto(player: &Player, args: &[String], map_state: &MapState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Failure("用法: @goto <玩家名>".to_string());
    }

    let target_name = &args[0];
    let target = map_state.find_player_by_name(target_name);

    match target {
        Some(target) => {
            player.map_name = target.map_name.clone();
            let (x, y) = target.get_position();
            player.move_to(x, y);
            CommandResult::Success(format!("传送到 {} 的位置 ({}, {})", target_name, x, y))
        }
        None => CommandResult::Failure(format!("找不到玩家: {}", target_name)),
    }
}
```

`src/game/command/commands/player.rs`:
```rust
use crate::game::map::{Player, MapState};
use crate::game::command::atcommand::CommandResult;

/// @heal - 恢复 HP/SP
pub fn cmd_heal(player: &Player, _args: &[String], _map_state: &MapState) -> CommandResult {
    let max_hp = *player.max_hp.read();
    let max_sp = *player.max_sp.read();

    *player.hp.write() = max_hp;
    *player.sp.write() = max_sp;

    CommandResult::Success(format!("HP 和 SP 已回满"))
}

/// @revive - 复活
pub fn cmd_revive(player: &Player, _args: &[String], _map_state: &MapState) -> CommandResult {
    if player.is_alive() {
        return CommandResult::Failure("你并没有死亡".to_string());
    }

    player.respawn(*player.pos_x.read(), *player.pos_y.read());
    CommandResult::Success("已复活".to_string())
}

/// @level <level> - 设置等级
pub fn cmd_level(player: &Player, args: &[String], _map_state: &MapState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Failure("用法: @level <等级>".to_string());
    }

    let level: u16 = match args[0].parse() {
        Ok(l) => l,
        Err(_) => return CommandResult::Failure("无效的等级".to_string()),
    };

    if level > 99 {
        return CommandResult::Failure("等级不能超过 99".to_string());
    }

    *player.base_level.write() = level;
    CommandResult::Success(format!("等级设置为 {}", level))
}

/// @zeny <amount> - 增加 Zeny
pub fn cmd_zeny(player: &Player, args: &[String], _map_state: &MapState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Failure("用法: @zeny <数量>".to_string());
    }

    let amount: u32 = match args[0].parse() {
        Ok(a) => a,
        Err(_) => return CommandResult::Failure("无效的数量".to_string()),
    };

    player.add_zeny(amount as u64);
    CommandResult::Success(format!("获得 {} Zeny", amount))
}

/// @hp <amount> - 设置 HP
pub fn cmd_hp(player: &Player, args: &[String], _map_state: &MapState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Failure("用法: @hp <数量>".to_string());
    }

    let amount: u32 = match args[0].parse() {
        Ok(a) => a,
        Err(_) => return CommandResult::Failure("无效的数量".to_string()),
    };

    let max_hp = *player.max_hp.read();
    *player.hp.write() = amount.min(max_hp);
    CommandResult::Success(format!("HP 设置为 {}", amount.min(max_hp)))
}

/// @sp <amount> - 设置 SP
pub fn cmd_sp(player: &Player, args: &[String], _map_state: &MapState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Failure("用法: @sp <数量>".to_string());
    }

    let amount: u32 = match args[0].parse() {
        Ok(a) => a,
        Err(_) => return CommandResult::Failure("无效的数量".to_string()),
    };

    let max_sp = *player.max_sp.read();
    *player.sp.write() = amount.min(max_sp);
    CommandResult::Success(format!("SP 设置为 {}", amount.min(max_sp)))
}
```

- [ ] **Step 2: 注册命令到 AtCommandHandler**

修改 `src/game/command/atcommand.rs`，添加 `register_default_commands()` 方法：

```rust
/// 注册默认命令
pub fn register_default_commands(&self) {
    use crate::game::command::commands;

    // 传送命令
    self.register(CommandInfo {
        name: "warp",
        aliases: vec!["mapmove", "rura"],
        min_level: 10,
        description: "传送到指定地图",
        usage: "@warp <地图> [x] [y]",
        handler: commands::teleport::cmd_warp,
    });

    self.register(CommandInfo {
        name: "goto",
        aliases: vec!["jumpto", "warpto"],
        min_level: 10,
        description: "传送到玩家位置",
        usage: "@goto <玩家名>",
        handler: commands::teleport::cmd_goto,
    });

    // 玩家命令
    self.register(CommandInfo {
        name: "heal",
        aliases: vec![],
        min_level: 10,
        description: "恢复 HP 和 SP",
        usage: "@heal",
        handler: commands::player::cmd_heal,
    });

    self.register(CommandInfo {
        name: "revive",
        aliases: vec!["respawn"],
        min_level: 10,
        description: "复活",
        usage: "@revive",
        handler: commands::player::cmd_revive,
    });

    self.register(CommandInfo {
        name: "level",
        aliases: vec!["lv"],
        min_level: 50,
        description: "设置等级",
        usage: "@level <1-99>",
        handler: commands::player::cmd_level,
    });

    self.register(CommandInfo {
        name: "zeny",
        aliases: vec!["gold"],
        min_level: 50,
        description: "增加 Zeny",
        usage: "@zeny <数量>",
        handler: commands::player::cmd_zeny,
    });

    self.register(CommandInfo {
        name: "hp",
        aliases: vec![],
        min_level: 10,
        description: "设置 HP",
        usage: "@hp <数量>",
        handler: commands::player::cmd_hp,
    });

    self.register(CommandInfo {
        name: "sp",
        aliases: vec![],
        min_level: 10,
        description: "设置 SP",
        usage: "@sp <数量>",
        handler: commands::player::cmd_sp,
    });
}
```

- [ ] **Step 3: 修改 mod.rs 添加 commands 模块**

```rust
pub mod atcommand;
pub mod parser;
pub mod commands;

pub use atcommand::{AtCommandHandler, CommandInfo, CommandResult, CommandHandler};
pub use parser::parse_command;
```

- [ ] **Step 4: 编译验证**

Run: `cargo build 2>&1 | head -50`
Expected: 无错误

- [ ] **Step 5: 提交**

```bash
git add src/game/command/
git commit -m "feat(command): implement basic @commands (warp, goto, heal, level, zeny)"
```

---

## Task 3: 集成到聊天系统

**Files:**
- Modify: `src/game/map/channel_bus.rs` 或相关聊天处理文件

- [ ] **Step 1: 查找聊天处理入口**

探索聊天消息如何处理，找到消息路由位置。

- [ ] **Step 2: 添加命令检测和处理**

在聊天消息处理中添加：
```rust
// 如果消息以 @ 开头，则作为命令处理
if message.starts_with('@') {
    let result = at_command_handler.execute(player, message, map_state);
    match result {
        CommandResult::Success(msg) => {
            // 发送成功消息给玩家
        }
        CommandResult::Failure(msg) => {
            // 发送错误消息给玩家
        }
        CommandResult::NoPermission => {
            // 发送权限不足消息
        }
    }
    return;
}
```

- [ ] **Step 3: 编译验证**

Run: `cargo build 2>&1 | head -30`

- [ ] **Step 4: 提交**

```bash
git add src/game/
git commit -m "feat(command): integrate @command handler into chat system"
```

---

## Task 4: 添加测试

**Files:**
- Modify: `src/game/command/atcommand.rs` 或 `parser.rs`

- [ ] **Step 1: 添加命令执行测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parsing() {
        let (cmd, args) = parse_command("@warp prontera 100 200");
        assert_eq!(cmd, "warp");
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn test_atcommand_register() {
        let handler = AtCommandHandler::new();
        handler.register(CommandInfo {
            name: "test",
            aliases: vec!["t"],
            min_level: 10,
            description: "Test command",
            usage: "@test",
            handler: |_, _, _| CommandResult::Success("OK".to_string()),
        });

        assert!(handler.commands.read().contains_key("test"));
        assert!(handler.commands.read().contains_key("t"));
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test command 2>&1`

- [ ] **Step 3: 提交**

```bash
git add src/game/command/
git commit -m "test(command): add @command system tests"
```
