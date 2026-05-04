use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

use super::data::{Guild, GuildPermission};

/// 公会管理器
pub struct GuildManager {
    guilds: RwLock<HashMap<Uuid, Guild>>,
    player_guild: RwLock<HashMap<Uuid, Uuid>>, // player_id -> guild_id
}

impl GuildManager {
    pub fn new() -> Self {
        Self {
            guilds: RwLock::new(HashMap::new()),
            player_guild: RwLock::new(HashMap::new()),
        }
    }

    /// 创建公会
    pub fn create_guild(&self, name: String, master_name: String) -> Option<Uuid> {
        let guilds = self.guilds.read();
        if guilds.values().any(|g| g.name == name) {
            return None;
        }
        drop(guilds);

        let guild = Guild::new(name, master_name);
        let guild_id = guild.id;

        self.guilds.write().insert(guild_id, guild);
        Some(guild_id)
    }

    /// 解散公会
    pub fn disband_guild(&self, guild_id: &Uuid) -> bool {
        let mut guilds = self.guilds.write();
        if let Some(guild) = guilds.remove(guild_id) {
            let mut player_guild = self.player_guild.write();
            for player_id in guild.members.keys() {
                player_guild.remove(player_id);
            }
            true
        } else {
            false
        }
    }

    /// 获取公会
    pub fn get_guild(&self, guild_id: &Uuid) -> Option<Guild> {
        self.guilds.read().get(guild_id).cloned()
    }

    /// 通过名称获取公会
    pub fn get_guild_by_name(&self, name: &str) -> Option<Guild> {
        self.guilds
            .read()
            .values()
            .find(|g| g.name == name)
            .cloned()
    }

    /// 获取玩家所在公会
    pub fn get_player_guild(&self, player_id: &Uuid) -> Option<Guild> {
        let player_guild = self.player_guild.read();
        let guild_id = player_guild.get(player_id)?;
        self.guilds.read().get(guild_id).cloned()
    }

    /// 获取玩家所属公会ID
    pub fn get_player_guild_id(&self, player_id: &Uuid) -> Option<Uuid> {
        self.player_guild.read().get(player_id).copied()
    }

    /// 玩家加入公会
    pub fn join_guild(&self, guild_id: Uuid, player_id: Uuid, name: String) -> bool {
        let player_guild = self.player_guild.read();
        if player_guild.contains_key(&player_id) {
            return false;
        }
        drop(player_guild);

        let mut guilds = self.guilds.write();
        let Some(guild) = guilds.get_mut(&guild_id) else {
            return false;
        };

        if !guild.add_member(player_id, name) {
            return false;
        }

        drop(guilds);
        self.player_guild.write().insert(player_id, guild_id);
        true
    }

    /// 玩家离开公会
    pub fn leave_guild(&self, player_id: Uuid) -> bool {
        let player_guild = self.player_guild.read();
        let Some(guild_id) = player_guild.get(&player_id).copied() else {
            return false;
        };
        drop(player_guild);

        let mut guilds = self.guilds.write();
        if let Some(guild) = guilds.get_mut(&guild_id) {
            guild.remove_member(&player_id);
        }
        drop(guilds);

        self.player_guild.write().remove(&player_id);
        true
    }

    /// 踢出成员
    pub fn expel_member(&self, guild_id: Uuid, expeller_id: &Uuid, target_id: &Uuid) -> bool {
        let mut guilds = self.guilds.write();
        let Some(guild) = guilds.get_mut(&guild_id) else {
            return false;
        };

        if !guild.has_permission(expeller_id, GuildPermission::Expel) {
            return false;
        }

        guild.remove_member(target_id);
        drop(guilds);

        self.player_guild.write().remove(target_id);
        true
    }

    /// 修改成员职位
    pub fn change_position(
        &self,
        guild_id: Uuid,
        operator_id: &Uuid,
        target_id: &Uuid,
        position_id: u8,
    ) -> bool {
        let mut guilds = self.guilds.write();
        let Some(guild) = guilds.get_mut(&guild_id) else {
            return false;
        };

        if !guild.has_permission(operator_id, GuildPermission::Expel) {
            return false;
        }

        guild.change_position(target_id, position_id)
    }

    /// 直接设置成员职位（无需权限检查，用于初始化会长等）
    pub fn set_member_position_direct(
        &self,
        guild_id: &Uuid,
        player_id: &Uuid,
        position_id: u8,
    ) -> bool {
        let mut guilds = self.guilds.write();
        let Some(guild) = guilds.get_mut(guild_id) else {
            return false;
        };
        guild.change_position(player_id, position_id)
    }

    /// 更新公会公告
    pub fn update_notice(&self, guild_id: &Uuid, notice: String) -> bool {
        let mut guilds = self.guilds.write();
        let Some(guild) = guilds.get_mut(guild_id) else {
            return false;
        };
        guild.set_notice(notice);
        true
    }

    /// 列出所有公会
    pub fn list_guilds(&self) -> Vec<(Uuid, String, String)> {
        self.guilds
            .read()
            .values()
            .map(|g| (g.id, g.name.clone(), g.master_name.clone()))
            .collect()
    }

    /// 获取公会数量
    pub fn guild_count(&self) -> usize {
        self.guilds.read().len()
    }

    /// 更新成员在线状态
    pub fn set_member_online(&self, guild_id: &Uuid, player_id: &Uuid, online: bool) {
        let mut guilds = self.guilds.write();
        if let Some(guild) = guilds.get_mut(guild_id)
            && let Some(member) = guild.get_member_mut(player_id)
        {
            member.online = online;
        }
    }
}

impl Default for GuildManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_guild() {
        let manager = GuildManager::new();
        let guild_id = manager.create_guild("TestGuild".to_string(), "Master".to_string());
        assert!(guild_id.is_some());

        let guild = manager.get_guild(&guild_id.unwrap());
        assert!(guild.is_some());
        assert_eq!(guild.unwrap().name, "TestGuild");
    }

    #[test]
    fn test_create_duplicate_guild_name() {
        let manager = GuildManager::new();
        assert!(
            manager
                .create_guild("SameName".to_string(), "M1".to_string())
                .is_some()
        );
        assert!(
            manager
                .create_guild("SameName".to_string(), "M2".to_string())
                .is_none()
        );
    }

    #[test]
    fn test_disband_guild() {
        let manager = GuildManager::new();
        let guild_id = manager
            .create_guild("TestGuild".to_string(), "Master".to_string())
            .unwrap();
        assert!(manager.disband_guild(&guild_id));
        assert!(manager.get_guild(&guild_id).is_none());
    }

    #[test]
    fn test_join_and_leave_guild() {
        let manager = GuildManager::new();
        let guild_id = manager
            .create_guild("TestGuild".to_string(), "Master".to_string())
            .unwrap();
        let player_id = Uuid::new_v4();

        assert!(manager.join_guild(guild_id, player_id, "Member".to_string()));
        assert!(manager.get_player_guild(&player_id).is_some());

        assert!(manager.leave_guild(player_id));
        assert!(manager.get_player_guild(&player_id).is_none());
    }

    #[test]
    fn test_expel_member() {
        let manager = GuildManager::new();
        let guild_id = manager
            .create_guild("TestGuild".to_string(), "Master".to_string())
            .unwrap();
        let master_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();

        manager.join_guild(guild_id, master_id, "Master".to_string());
        // 手动设置 master 为职位 0 (会长)
        {
            let mut guilds = manager.guilds.write();
            if let Some(guild) = guilds.get_mut(&guild_id) {
                guild.change_position(&master_id, 0);
            }
        }

        manager.join_guild(guild_id, member_id, "Member".to_string());

        assert!(manager.expel_member(guild_id, &master_id, &member_id));
        assert!(manager.get_player_guild(&member_id).is_none());
    }

    #[test]
    fn test_list_guilds() {
        let manager = GuildManager::new();
        manager.create_guild("Guild1".to_string(), "M1".to_string());
        manager.create_guild("Guild2".to_string(), "M2".to_string());
        manager.create_guild("Guild3".to_string(), "M3".to_string());

        assert_eq!(manager.list_guilds().len(), 3);
    }

    #[test]
    fn test_guild_count() {
        let manager = GuildManager::new();
        assert_eq!(manager.guild_count(), 0);
        manager.create_guild("G1".to_string(), "M1".to_string());
        assert_eq!(manager.guild_count(), 1);
    }

    #[test]
    fn test_get_player_guild_id() {
        let manager = GuildManager::new();
        let guild_id = manager
            .create_guild("TestGuild".to_string(), "Master".to_string())
            .unwrap();
        let player_id = Uuid::new_v4();

        manager.join_guild(guild_id, player_id, "Member".to_string());
        assert_eq!(manager.get_player_guild_id(&player_id), Some(guild_id));
    }

    #[test]
    fn test_set_member_online() {
        let manager = GuildManager::new();
        let guild_id = manager
            .create_guild("TestGuild".to_string(), "Master".to_string())
            .unwrap();
        let player_id = Uuid::new_v4();

        manager.join_guild(guild_id, player_id, "Member".to_string());

        manager.set_member_online(&guild_id, &player_id, false);
        let guild = manager.get_guild(&guild_id).unwrap();
        let member = guild.get_member(&player_id).unwrap();
        assert!(!member.online);
    }

    #[test]
    fn test_guild_is_full() {
        let mut guild = Guild::new("SmallGuild".to_string(), "Master".to_string());
        guild.max_members = 2;

        assert!(guild.add_member(Uuid::new_v4(), "M1".to_string()));
        assert!(guild.add_member(Uuid::new_v4(), "M2".to_string()));
        assert!(!guild.add_member(Uuid::new_v4(), "M3".to_string()));
    }

    #[test]
    fn test_guild_add_exp_and_level_up() {
        let mut guild = Guild::new("TestGuild".to_string(), "Master".to_string());
        assert_eq!(guild.level, 1);

        let leveled = guild.add_exp(1000);
        assert!(leveled);
        assert_eq!(guild.level, 2);
        assert_eq!(guild.max_exp, 4000); // 1000 * 2^2
    }
}
