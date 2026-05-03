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
        commands.insert(info.name.to_string(), info.clone());
        for alias in &info.aliases {
            commands.insert(alias.to_string(), info.clone());
        }
    }

    /// 执行命令
    pub fn execute(&self, player: &Player, input: &str, map_state: &MapState) -> CommandResult {
        let (cmd_name, args) = crate::game::command::parser::parse_command(input);

        if cmd_name.is_empty() {
            return CommandResult::Failure("无效命令".to_string());
        }

        let commands = self.commands.read();
        let Some(info) = commands.get(&cmd_name) else {
            return CommandResult::Failure(format!("未知命令: @{}", cmd_name));
        };

        if !self.check_permission(player, info.min_level) {
            tracing::warn!("Player {} attempted command @{} without permission", player.name, cmd_name);
            return CommandResult::NoPermission;
        }

        (info.handler)(player, &args, map_state)
    }

    /// 检查权限
    pub fn check_permission(&self, player: &Player, required_level: u8) -> bool {
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

    /// 获取所有命令列表
    pub fn list_commands(&self, player_level: u8) -> Vec<&'static str> {
        let commands = self.commands.read();
        let mut result: Vec<&'static str> = Vec::new();
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();

        for (name, info) in commands.iter() {
            if info.min_level <= player_level && !seen.contains(name.as_str()) {
                seen.insert(name.as_str());
                result.push(info.name);
            }
        }

        result.sort();
        result
    }
}

impl Default for AtCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}
