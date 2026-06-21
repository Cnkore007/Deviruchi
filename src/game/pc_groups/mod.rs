//! 玩家权限组系统
//!
//! 对应 rAthena 的 `src/map/pc_groups.cpp`，提供 GM 权限组管理。
//!
//! 权限组定义了玩家可以使用的命令和功能，如 @warp、@kick 等。

use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};

/// 权限级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum GroupLevel {
    /// 普通玩家
    Player = 0,
    /// 初级 GM
    Support = 1,
    /// 中级 GM
    Script = 2,
    /// 高级 GM
    Event = 3,
    /// 管理员
    Admin = 4,
    /// 超级管理员
    Super = 5,
}

impl GroupLevel {
    /// 从 u8 转换
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Player),
            1 => Some(Self::Support),
            2 => Some(Self::Script),
            3 => Some(Self::Event),
            4 => Some(Self::Admin),
            5 => Some(Self::Super),
            _ => None,
        }
    }
}

/// 权限组
#[derive(Debug, Clone)]
pub struct PlayerGroup {
    /// 组 ID
    pub id: u8,
    /// 组名
    pub name: String,
    /// 权限级别
    pub level: GroupLevel,
    /// 允许的命令列表
    pub commands: HashSet<String>,
    /// 是否日志记录
    pub log_commands: bool,
}

impl PlayerGroup {
    /// 创建新的权限组
    pub fn new(id: u8, name: String, level: GroupLevel) -> Self {
        Self {
            id,
            name,
            level,
            commands: HashSet::new(),
            log_commands: level >= GroupLevel::Support,
        }
    }

    /// 添加命令权限
    pub fn add_command(&mut self, command: &str) {
        self.commands.insert(command.to_string());
    }

    /// 移除命令权限
    pub fn remove_command(&mut self, command: &str) {
        self.commands.remove(command);
    }

    /// 检查是否有指定命令的权限
    pub fn has_command(&self, command: &str) -> bool {
        self.commands.contains(command)
    }

    /// 检查是否有指定级别的权限
    pub fn has_level(&self, level: GroupLevel) -> bool {
        self.level >= level
    }
}

/// 权限组管理器
pub struct PcGroupManager {
    /// 权限组 (group_id -> PlayerGroup)
    groups: RwLock<HashMap<u8, PlayerGroup>>,
    /// 玩家权限组映射 (account_id -> group_id)
    player_groups: RwLock<HashMap<u32, u8>>,
    /// 默认组 ID
    default_group_id: u8,
}

impl PcGroupManager {
    /// 创建权限组管理器
    pub fn new() -> Self {
        let mut groups = HashMap::new();

        // 创建默认权限组
        groups.insert(
            0,
            PlayerGroup::new(0, "Player".to_string(), GroupLevel::Player),
        );
        groups.insert(
            1,
            PlayerGroup::new(1, "Support".to_string(), GroupLevel::Support),
        );
        groups.insert(
            2,
            PlayerGroup::new(2, "Script".to_string(), GroupLevel::Script),
        );
        groups.insert(
            3,
            PlayerGroup::new(3, "Event".to_string(), GroupLevel::Event),
        );
        groups.insert(
            4,
            PlayerGroup::new(4, "Admin".to_string(), GroupLevel::Admin),
        );
        groups.insert(
            5,
            PlayerGroup::new(5, "Super".to_string(), GroupLevel::Super),
        );

        Self {
            groups: RwLock::new(groups),
            player_groups: RwLock::new(HashMap::new()),
            default_group_id: 0,
        }
    }

    /// 获取权限组
    pub fn get_group(&self, group_id: u8) -> Option<PlayerGroup> {
        self.groups.read().get(&group_id).cloned()
    }

    /// 获取玩家的权限组
    pub fn get_player_group(&self, account_id: u32) -> Option<PlayerGroup> {
        let group_id = self
            .player_groups
            .read()
            .get(&account_id)
            .copied()
            .unwrap_or(self.default_group_id);
        self.get_group(group_id)
    }

    /// 设置玩家的权限组
    pub fn set_player_group(&self, account_id: u32, group_id: u8) -> bool {
        if !self.groups.read().contains_key(&group_id) {
            return false;
        }
        self.player_groups.write().insert(account_id, group_id);
        true
    }

    /// 检查玩家是否有指定命令的权限
    pub fn has_command_permission(&self, account_id: u32, command: &str) -> bool {
        match self.get_player_group(account_id) {
            Some(group) => group.has_command(command),
            None => false,
        }
    }

    /// 检查玩家是否有指定级别的权限
    pub fn has_level_permission(&self, account_id: u32, level: GroupLevel) -> bool {
        match self.get_player_group(account_id) {
            Some(group) => group.has_level(level),
            None => false,
        }
    }

    /// 添加权限组
    pub fn add_group(&self, group: PlayerGroup) {
        self.groups.write().insert(group.id, group);
    }

    /// 移除权限组
    pub fn remove_group(&self, group_id: u8) -> bool {
        if group_id == self.default_group_id {
            return false; // 不能删除默认组
        }
        self.groups.write().remove(&group_id).is_some()
    }

    /// 为权限组添加命令
    pub fn add_group_command(&self, group_id: u8, command: &str) {
        if let Some(group) = self.groups.write().get_mut(&group_id) {
            group.add_command(command);
        }
    }

    /// 为权限组移除命令
    pub fn remove_group_command(&self, group_id: u8, command: &str) {
        if let Some(group) = self.groups.write().get_mut(&group_id) {
            group.remove_command(command);
        }
    }

    /// 设置默认 GM 命令
    pub fn setup_default_commands(&self) {
        let mut groups = self.groups.write();

        // Support 组命令
        if let Some(group) = groups.get_mut(&1) {
            group.add_command("@alive");
            group.add_command("@die");
            group.add_command("@kick");
            group.add_command("@kickall");
            group.add_command("@kill");
            group.add_command("@recall");
            group.add_command("@ban");
            group.add_command("@unban");
        }

        // Script 组命令
        if let Some(group) = groups.get_mut(&2) {
            group.add_command("@warp");
            group.add_command("@go");
            group.add_command("@jump");
            group.add_command("@hide");
            group.add_command("@option");
            group.add_command("@str");
            group.add_command("@agi");
            group.add_command("@vit");
            group.add_command("@int");
            group.add_command("@dex");
            group.add_command("@luk");
        }

        // Event 组命令
        if let Some(group) = groups.get_mut(&3) {
            group.add_command("@spawn");
            group.add_command("@monster");
            group.add_command("@item");
            group.add_command("@item2");
            group.add_command("@speed");
            group.add_command("@effect");
        }

        // Admin 组命令
        if let Some(group) = groups.get_mut(&4) {
            group.add_command("@reload");
            group.add_command("@reloadscript");
            group.add_command("@reloaditemdb");
            group.add_command("@reloadmobdb");
        }

        // Super 组命令
        if let Some(group) = groups.get_mut(&5) {
            group.add_command("@shutdown");
            group.add_command("@maintenance");
            group.add_command("@adjgmlvl");
        }
    }

    /// 获取组总数
    pub fn group_count(&self) -> usize {
        self.groups.read().len()
    }

    /// 清理所有
    pub fn clear(&self) {
        self.player_groups.write().clear();
    }
}

impl Default for PcGroupManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_level_ordering() {
        assert!(GroupLevel::Player < GroupLevel::Support);
        assert!(GroupLevel::Admin < GroupLevel::Super);
    }

    #[test]
    fn test_group_level_from_u8() {
        assert_eq!(GroupLevel::from_u8(0), Some(GroupLevel::Player));
        assert_eq!(GroupLevel::from_u8(5), Some(GroupLevel::Super));
        assert_eq!(GroupLevel::from_u8(6), None);
    }

    #[test]
    fn test_player_group_command() {
        let mut group = PlayerGroup::new(1, "Test".to_string(), GroupLevel::Support);

        group.add_command("@warp");
        assert!(group.has_command("@warp"));
        assert!(!group.has_command("@kick"));

        group.remove_command("@warp");
        assert!(!group.has_command("@warp"));
    }

    #[test]
    fn test_player_group_level() {
        let group = PlayerGroup::new(1, "Test".to_string(), GroupLevel::Support);

        assert!(group.has_level(GroupLevel::Player));
        assert!(group.has_level(GroupLevel::Support));
        assert!(!group.has_level(GroupLevel::Script));
    }

    #[test]
    fn test_pc_group_manager_default() {
        let manager = PcGroupManager::new();
        assert_eq!(manager.group_count(), 6);
    }

    #[test]
    fn test_pc_group_manager_player_group() {
        let manager = PcGroupManager::new();

        // 默认是 Player 组
        let group = manager.get_player_group(1001).unwrap();
        assert_eq!(group.level, GroupLevel::Player);

        // 设置为 Admin 组
        manager.set_player_group(1001, 4);
        let group = manager.get_player_group(1001).unwrap();
        assert_eq!(group.level, GroupLevel::Admin);
    }

    #[test]
    fn test_pc_group_manager_command_permission() {
        let manager = PcGroupManager::new();
        manager.setup_default_commands();

        // 普通玩家没有 @warp 权限
        assert!(!manager.has_command_permission(1001, "@warp"));

        // 设置为 Script 组后有权限
        manager.set_player_group(1001, 2);
        assert!(manager.has_command_permission(1001, "@warp"));
    }

    #[test]
    fn test_pc_group_manager_level_permission() {
        let manager = PcGroupManager::new();

        // 普通玩家没有 Support 级别权限
        assert!(!manager.has_level_permission(1001, GroupLevel::Support));

        // 设置为 Support 组后有权限
        manager.set_player_group(1001, 1);
        assert!(manager.has_level_permission(1001, GroupLevel::Support));
    }

    #[test]
    fn test_pc_group_manager_custom_group() {
        let manager = PcGroupManager::new();

        let mut custom = PlayerGroup::new(10, "Custom".to_string(), GroupLevel::Script);
        custom.add_command("@custom");
        manager.add_group(custom);

        assert!(manager.get_group(10).is_some());
        manager.set_player_group(1001, 10);
        assert!(manager.has_command_permission(1001, "@custom"));
    }

    #[test]
    fn test_pc_group_manager_cannot_remove_default() {
        let manager = PcGroupManager::new();
        assert!(!manager.remove_group(0)); // 不能删除默认组
    }
}
