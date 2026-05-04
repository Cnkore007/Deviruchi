# GM 命令框架设计

## 概述

实现完整的 @ 命令系统，包括权限检查、命令注册、解析和常用命令实现。

## 设计目标

1. **权限等级系统** - 0-99 等级，不同等级可用不同命令
2. **命令注册机制** - 集中式命令注册，支持动态添加
3. **命令解析器** - 解析 @command [args] 格式
4. **常用命令** - @warp, @item, @heal, @goto, @summon, @reload 等

## 权限等级

| Level | 名称 | 说明 |
|-------|------|------|
| 0 | Player | 普通玩家（无命令权限）|
| 1 | Support | 支援人员 |
| 10 | GM | 普通 GM |
| 20 | High GM | 高级 GM |
| 50 | Admin | 管理员 |
| 99 | Super Admin | 超级管理员 |

## 命令注册机制

```rust
/// 命令信息
pub struct CommandInfo {
    pub name: &'static str,
    pub aliases: Vec<&'static str>,
    pub min_level: u8,
    pub description: &'static str,
    pub usage: &'static str,
    pub handler: CommandHandler,
}

/// 命令处理器类型
type CommandHandler = fn(
    player: &Player,
    args: &[&str],
    map_state: &MapState,
    handler: &AtCommandHandler,
) -> CommandResult;

/// 命令结果
pub enum CommandResult {
    Success(String),
    Failure(String),
    NoPermission,
}
```

## 命令表

| 命令 | 等级 | 说明 |
|------|------|------|
| @warp <map> [x] [y] | 10 | 传送到地图 |
| @goto <player> | 10 | 传送到玩家位置 |
| @summon <mob_name> | 10 | 召唤怪物 |
| @item <item_id> [count] | 50 | 生成物品 |
| @heal | 10 | 恢复 HP/SP |
| @revive | 10 | 复活玩家 |
| @hide | 20 | 隐身 |
| @speed <1-100> | 10 | 设置移动速度 |
| @spawn <mob_id> [count] | 10 | 生成怪物 |
| @kill <player> | 50 | 杀死玩家 |
| @level <1-99> | 50 | 设置等级 |
| @zeny <amount> | 50 | 增加 Zeny |
| @reload | 99 | 重载配置 |
| @shutdown | 99 | 关闭服务器 |
| @broadcast <msg> | 20 | 服务器广播 |

## 架构设计

```
src/game/
  └── command/
      ├── mod.rs          # 模块导出
      ├── atcommand.rs     # @命令处理器
      ├── handler.rs       # 命令执行器
      ├── parser.rs        # 命令解析
      ├── permissions.rs   # 权限检查
      └── commands/        # 具体命令实现
          ├── mod.rs
          ├── teleport.rs   # @warp, @goto
          ├── player.rs    # @heal, @revive, @level
          ├── item.rs      # @item
          ├── mob.rs       # @summon, @spawn
          └── admin.rs     # @reload, @shutdown, @broadcast
```

## 核心组件

### AtCommandHandler

```rust
pub struct AtCommandHandler {
    commands: RwLock<HashMap<String, CommandInfo>>,
    permission_levels: HashMap<u8, &'static str>,
}

impl AtCommandHandler {
    pub fn new() -> Self;
    pub fn register(&self, info: CommandInfo);
    pub fn register_all(&self);
    pub fn execute(&self, player: &Player, input: &str, map_state: &MapState) -> CommandResult;
    pub fn check_permission(&self, player: &Player, command: &str) -> bool;
}
```

### 解析流程

1. 收到玩家输入 `@warp prontera 100 200`
2. 解析命令名 `warp` 和参数 `["prontera", "100", "200"]`
3. 查找命令 `warp` 的 CommandInfo
4. 检查玩家权限等级是否 >= 命令所需等级
5. 调用命令处理器
6. 返回结果

## 集成点

1. **聊天系统集成** - 聊天消息以 `@` 开头时调用命令处理器
2. **PacketHandler** - 处理客户端发送的命令数据包
3. **ModernServer** - WebSocket 命令支持

## 实现步骤

1. 创建 command 模块目录
2. 实现 AtCommandHandler 核心结构
3. 实现命令注册机制
4. 实现常用命令
5. 集成到聊天系统
6. 添加测试
