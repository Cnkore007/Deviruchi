use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use super::data::{Guild, GuildPermission};
use crate::storage::Database;
use crate::storage::guild::GuildStorage;

/// 公会管理器
///
/// 支持两种模式：
/// - 纯内存模式（`new()`）：用于测试，数据不持久化
/// - 持久化模式（`with_db()`）：数据同步写入数据库，启动时自动加载
pub struct GuildManager {
    guilds: RwLock<HashMap<Uuid, Guild>>,
    player_guild: RwLock<HashMap<Uuid, Uuid>>, // player_id -> guild_id
    /// 可选的持久化存储层，None 表示纯内存模式
    storage: Option<Arc<GuildStorage>>,
}

impl GuildManager {
    /// 创建纯内存模式的公会管理器（用于测试）
    pub fn new() -> Self {
        Self {
            guilds: RwLock::new(HashMap::new()),
            player_guild: RwLock::new(HashMap::new()),
            storage: None,
        }
    }

    /// 创建带数据库持久化的公会管理器
    ///
    /// 初始化时会从数据库加载所有已保存的公会数据到内存。
    /// 如果加载失败，会记录警告日志并以空数据启动。
    pub fn with_db(db: Arc<Database>) -> Self {
        let storage = Arc::new(GuildStorage::new(db));
        let manager = Self {
            guilds: RwLock::new(HashMap::new()),
            player_guild: RwLock::new(HashMap::new()),
            storage: Some(storage.clone()),
        };

        // 从数据库加载所有公会
        match storage.load_all_guilds() {
            Ok(guilds) => {
                let mut guild_map = manager.guilds.write();
                let mut player_map = manager.player_guild.write();
                for guild in guilds {
                    // 建立 player_id -> guild_id 映射
                    for player_id in guild.members.keys() {
                        player_map.insert(*player_id, guild.id);
                    }
                    tracing::info!(
                        "已加载公会: {} (成员数: {})",
                        guild.name,
                        guild.members.len()
                    );
                    guild_map.insert(guild.id, guild);
                }
                tracing::info!("公会数据加载完成，共 {} 个公会", guild_map.len());
            }
            Err(e) => {
                tracing::warn!("加载公会数据失败，以空数据启动: {}", e);
            }
        }

        manager
    }

    /// 创建公会
    ///
    /// 创建成功后自动持久化到数据库（如果配置了存储层）。
    /// 持久化失败不影响创建结果，仅记录警告日志。
    pub fn create_guild(&self, name: String, master_name: String) -> Option<Uuid> {
        let guilds = self.guilds.read();
        if guilds.values().any(|g| g.name == name) {
            return None;
        }
        drop(guilds);

        let guild = Guild::new(name, master_name);
        let guild_id = guild.id;

        self.guilds.write().insert(guild_id, guild);

        // 持久化到数据库
        if let Some(ref storage) = self.storage {
            let guilds = self.guilds.read();
            if let Some(guild) = guilds.get(&guild_id)
                && let Err(e) = storage.save_guild(guild)
            {
                tracing::warn!("持久化创建公会失败: {}", e);
            }
        }

        Some(guild_id)
    }

    /// 解散公会
    ///
    /// 解散后自动从数据库删除（如果配置了存储层）。
    pub fn disband_guild(&self, guild_id: &Uuid) -> bool {
        let mut guilds = self.guilds.write();
        if let Some(guild) = guilds.remove(guild_id) {
            let mut player_guild = self.player_guild.write();
            for player_id in guild.members.keys() {
                player_guild.remove(player_id);
            }
            drop(guilds);
            drop(player_guild);

            // 从数据库删除
            if let Some(ref storage) = self.storage
                && let Err(e) = storage.delete_guild(guild_id)
            {
                tracing::warn!("持久化删除公会失败: {}", e);
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
    ///
    /// 加入成功后自动持久化到数据库（如果配置了存储层）。
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

        // 持久化到数据库
        if let Some(ref storage) = self.storage {
            let guilds = self.guilds.read();
            if let Some(guild) = guilds.get(&guild_id)
                && let Err(e) = storage.save_guild(guild)
            {
                tracing::warn!("持久化加入公会失败: {}", e);
            }
        }

        true
    }

    /// 玩家离开公会
    ///
    /// 离开后自动持久化到数据库（如果配置了存储层）。
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

        // 持久化到数据库
        if let Some(ref storage) = self.storage {
            let guilds = self.guilds.read();
            if let Some(guild) = guilds.get(&guild_id)
                && let Err(e) = storage.save_guild(guild)
            {
                tracing::warn!("持久化离开公会失败: {}", e);
            }
        }

        true
    }

    /// 踢出成员
    ///
    /// 踢出后自动持久化到数据库（如果配置了存储层）。
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

        // 持久化到数据库
        if let Some(ref storage) = self.storage {
            let guilds = self.guilds.read();
            if let Some(guild) = guilds.get(&guild_id)
                && let Err(e) = storage.save_guild(guild)
            {
                tracing::warn!("持久化踢出成员失败: {}", e);
            }
        }

        true
    }

    /// 修改成员职位
    ///
    /// 变更后自动持久化到数据库（如果配置了存储层）。
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

        let result = guild.change_position(target_id, position_id);
        drop(guilds);

        // 持久化到数据库
        if result && let Some(ref storage) = self.storage {
            let guilds = self.guilds.read();
            if let Some(guild) = guilds.get(&guild_id)
                && let Err(e) = storage.save_guild(guild)
            {
                tracing::warn!("持久化变更职位失败: {}", e);
            }
        }

        result
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
    ///
    /// 更新后自动持久化到数据库（如果配置了存储层）。
    pub fn update_notice(&self, guild_id: &Uuid, notice: String) -> bool {
        let mut guilds = self.guilds.write();
        let Some(guild) = guilds.get_mut(guild_id) else {
            return false;
        };
        guild.set_notice(notice);
        drop(guilds);

        // 持久化到数据库
        if let Some(ref storage) = self.storage {
            let guilds = self.guilds.read();
            if let Some(guild) = guilds.get(guild_id)
                && let Err(e) = storage.save_guild(guild)
            {
                tracing::warn!("持久化更新公告失败: {}", e);
            }
        }

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
    ///
    /// 注意：此方法调用频繁，不在每次变更时持久化。
    /// 在线状态的持久化由 `save_all()` 定期执行，避免频繁数据库写入。
    pub fn set_member_online(&self, guild_id: &Uuid, player_id: &Uuid, online: bool) {
        let mut guilds = self.guilds.write();
        if let Some(guild) = guilds.get_mut(guild_id)
            && let Some(member) = guild.get_member_mut(player_id)
        {
            member.online = online;
        }
    }

    /// 定期保存所有公会到数据库
    ///
    /// 用于定时任务调用，将内存中的所有公会数据批量持久化。
    /// 如果未配置存储层，此方法为空操作。
    pub fn save_all(&self) {
        let storage = match self.storage {
            Some(ref s) => s.clone(),
            None => return,
        };

        let guilds = self.guilds.read();
        let mut saved = 0;
        let mut failed = 0;

        for guild in guilds.values() {
            match storage.save_guild(guild) {
                Ok(_) => saved += 1,
                Err(e) => {
                    failed += 1;
                    tracing::warn!("定期保存公会 {} 失败: {}", guild.name, e);
                }
            }
        }

        if failed > 0 {
            tracing::warn!("公会定期保存完成: 成功 {}, 失败 {}", saved, failed);
        } else if saved > 0 {
            tracing::debug!("公会定期保存完成: 共 {} 个公会", saved);
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

    /// 创建带内存数据库的 GuildManager（用于持久化测试）
    fn create_db_manager() -> GuildManager {
        let db = Arc::new(Database::open_memory().expect("创建内存数据库失败"));
        crate::storage::schema::init_schema(&db).expect("初始化 schema 失败");
        GuildManager::with_db(db)
    }

    #[test]
    fn test_with_db_create_guild_persists() {
        let manager = create_db_manager();

        // 创建公会
        let guild_id = manager
            .create_guild("PersistentGuild".to_string(), "Master".to_string())
            .unwrap();

        // 验证内存中有公会
        let guild = manager.get_guild(&guild_id).unwrap();
        assert_eq!(guild.name, "PersistentGuild");

        // 获取数据库引用，验证数据已持久化
        let storage = manager.storage.as_ref().unwrap();
        let loaded = storage.load_guild(&guild_id).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().name, "PersistentGuild");
    }

    #[test]
    fn test_with_db_disband_guild_removes_from_db() {
        let manager = create_db_manager();

        let guild_id = manager
            .create_guild("TempGuild".to_string(), "Master".to_string())
            .unwrap();

        // 解散公会
        assert!(manager.disband_guild(&guild_id));

        // 验证从数据库中删除
        let storage = manager.storage.as_ref().unwrap();
        let loaded = storage.load_guild(&guild_id).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_with_db_join_guild_persists() {
        let manager = create_db_manager();

        let guild_id = manager
            .create_guild("TestGuild".to_string(), "Master".to_string())
            .unwrap();
        let player_id = Uuid::new_v4();

        // 加入公会
        assert!(manager.join_guild(guild_id, player_id, "NewMember".to_string()));

        // 验证数据库中有成员
        let storage = manager.storage.as_ref().unwrap();
        let loaded = storage.load_guild(&guild_id).unwrap().unwrap();
        assert!(loaded.members.contains_key(&player_id));
    }

    #[test]
    fn test_with_db_leave_guild_persists() {
        let manager = create_db_manager();

        let guild_id = manager
            .create_guild("TestGuild".to_string(), "Master".to_string())
            .unwrap();
        let player_id = Uuid::new_v4();

        manager.join_guild(guild_id, player_id, "Member".to_string());
        assert!(manager.leave_guild(player_id));

        // 验证数据库中成员已移除
        let storage = manager.storage.as_ref().unwrap();
        let loaded = storage.load_guild(&guild_id).unwrap().unwrap();
        assert!(!loaded.members.contains_key(&player_id));
    }

    #[test]
    fn test_with_db_expel_member_persists() {
        let manager = create_db_manager();

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

        // 验证数据库中被踢成员已移除
        let storage = manager.storage.as_ref().unwrap();
        let loaded = storage.load_guild(&guild_id).unwrap().unwrap();
        assert!(!loaded.members.contains_key(&member_id));
    }

    #[test]
    fn test_with_db_update_notice_persists() {
        let manager = create_db_manager();

        let guild_id = manager
            .create_guild("TestGuild".to_string(), "Master".to_string())
            .unwrap();

        assert!(manager.update_notice(&guild_id, "新公告内容".to_string()));

        // 验证数据库中公告已更新
        let storage = manager.storage.as_ref().unwrap();
        let loaded = storage.load_guild(&guild_id).unwrap().unwrap();
        assert_eq!(loaded.notice, "新公告内容");
    }

    #[test]
    fn test_with_db_load_on_init() {
        // 第一个管理器：创建公会和成员
        let db = Arc::new(Database::open_memory().expect("创建内存数据库失败"));
        crate::storage::schema::init_schema(&db).expect("初始化 schema 失败");

        let manager1 = GuildManager::with_db(db.clone());
        let guild_id = manager1
            .create_guild("LoadedGuild".to_string(), "OriginalMaster".to_string())
            .unwrap();
        let player_id = Uuid::new_v4();
        manager1.join_guild(guild_id, player_id, "OriginalMember".to_string());

        // 第二个管理器：使用同一个数据库，应自动加载已有公会
        let manager2 = GuildManager::with_db(db);

        assert_eq!(manager2.guild_count(), 1);
        let guild = manager2.get_guild(&guild_id).unwrap();
        assert_eq!(guild.name, "LoadedGuild");
        assert_eq!(guild.master_name, "OriginalMaster");
        assert!(guild.members.contains_key(&player_id));

        // 验证 player_guild 映射也被正确恢复
        assert_eq!(manager2.get_player_guild_id(&player_id), Some(guild_id));
    }

    #[test]
    fn test_save_all() {
        let manager = create_db_manager();

        // 创建多个公会（已经通过 create_guild 持久化了）
        manager
            .create_guild("Guild1".to_string(), "M1".to_string())
            .unwrap();
        manager
            .create_guild("Guild2".to_string(), "M2".to_string())
            .unwrap();

        // save_all 不应报错
        manager.save_all();

        // 验证所有公会仍然在数据库中
        let storage = manager.storage.as_ref().unwrap();
        let all_guilds = storage.load_all_guilds().unwrap();
        assert_eq!(all_guilds.len(), 2);
    }

    #[test]
    fn test_save_all_noop_without_storage() {
        // 纯内存模式下 save_all 应为空操作，不报错
        let manager = GuildManager::new();
        manager.create_guild("MemGuild".to_string(), "Master".to_string());
        manager.save_all(); // 不应 panic
    }

    #[test]
    fn test_new_is_pure_memory() {
        // 验证纯内存模式不依赖数据库
        let manager = GuildManager::new();
        assert!(manager.storage.is_none());

        let guild_id = manager
            .create_guild("MemGuild".to_string(), "Master".to_string())
            .unwrap();
        assert!(manager.get_guild(&guild_id).is_some());
    }
}
