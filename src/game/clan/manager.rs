//! 公会联盟管理器
//!
//! 对应 rAthena 的 `clan.cpp`，管理公会数据和成员操作。

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use uuid::Uuid;

use super::data::{AllianceType, Clan, ClanAlliance, ClanMember};

/// 公会操作结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClanResult {
    /// 操作成功
    Success,
    /// 公会不存在
    NotFound,
    /// 公会已满员
    Full,
    /// 成员不存在
    MemberNotFound,
    /// 重复操作
    AlreadyJoined,
    /// 副本中禁止操作
    InstanceBlocked,
    /// 权限不足
    PermissionDenied,
}

/// 公会管理器
///
/// 管理所有公会数据，提供公会查询、成员管理、联盟操作等接口。
pub struct ClanManager {
    /// 公会数据 (clan_id -> Clan)
    clans: RwLock<HashMap<i32, Arc<RwLock<Clan>>>>,
}

impl ClanManager {
    /// 创建空的公会管理器
    pub fn new() -> Self {
        Self {
            clans: RwLock::new(HashMap::new()),
        }
    }

    /// 加载公会数据（从数据库或配置）
    pub fn load_clans(&self, clans: Vec<Clan>) {
        let mut map = self.clans.write();
        for clan in clans {
            map.insert(clan.id, Arc::new(RwLock::new(clan)));
        }
    }

    /// 按 ID 查找公会
    pub fn get_clan(&self, clan_id: i32) -> Option<Arc<RwLock<Clan>>> {
        self.clans.read().get(&clan_id).cloned()
    }

    /// 按名称查找公会
    pub fn find_clan_by_name(&self, name: &str) -> Option<Arc<RwLock<Clan>>> {
        let clans = self.clans.read();
        clans
            .values()
            .find(|c| {
                let clan = c.read();
                clan.name.eq_ignore_ascii_case(name)
            })
            .cloned()
    }

    /// 获取公会总数
    pub fn clan_count(&self) -> usize {
        self.clans.read().len()
    }

    /// 成员加入公会
    pub fn member_join(
        &self,
        clan_id: i32,
        char_id: Uuid,
        account_id: u32,
        name: String,
        block_instance: bool,
    ) -> ClanResult {
        let clans = self.clans.read();
        let clan_arc = match clans.get(&clan_id) {
            Some(c) => c.clone(),
            None => return ClanResult::NotFound,
        };
        drop(clans);

        let mut clan = clan_arc.write();

        // 检查副本限制
        if block_instance && clan.instance_id > 0 {
            return ClanResult::InstanceBlocked;
        }

        // 检查是否已在公会中
        if clan.members.contains_key(&char_id) {
            return ClanResult::AlreadyJoined;
        }

        // 检查满员
        if clan.is_full() {
            return ClanResult::Full;
        }

        let member = ClanMember {
            char_id,
            account_id,
            name,
            online: true,
        };
        clan.add_member(member);
        clan.connect_member += 1;

        ClanResult::Success
    }

    /// 成员离开公会
    pub fn member_leave(
        &self,
        clan_id: i32,
        char_id: Uuid,
        account_id: u32,
        block_instance: bool,
    ) -> ClanResult {
        let clans = self.clans.read();
        let clan_arc = match clans.get(&clan_id) {
            Some(c) => c.clone(),
            None => return ClanResult::NotFound,
        };
        drop(clans);

        let mut clan = clan_arc.write();

        // 检查副本限制
        if block_instance && clan.instance_id > 0 {
            return ClanResult::InstanceBlocked;
        }

        // 验证成员
        match clan.find_member_by_account(account_id) {
            Some(member) if member.char_id == char_id => {}
            _ => return ClanResult::MemberNotFound,
        }

        clan.remove_member(&char_id);
        if clan.connect_member > 0 {
            clan.connect_member -= 1;
        }

        ClanResult::Success
    }

    /// 成员上线
    pub fn member_online(&self, clan_id: i32, char_id: Uuid) {
        if let Some(clan_arc) = self.get_clan(clan_id) {
            clan_arc.write().member_online(&char_id);
        }
    }

    /// 成员下线
    pub fn member_offline(&self, clan_id: i32, char_id: Uuid) {
        if let Some(clan_arc) = self.get_clan(clan_id) {
            clan_arc.write().member_offline(&char_id);
        }
    }

    /// 广播公会消息
    pub fn broadcast_message(&self, clan_id: i32, sender_account_id: u32, message: &str) -> bool {
        let clans = self.clans.read();
        match clans.get(&clan_id) {
            Some(clan_arc) => {
                let clan = clan_arc.read();
                // 验证发送者是公会成员
                if clan.find_member_by_account(sender_account_id).is_some() {
                    // 实际发送需集成网络层，这里仅验证逻辑
                    tracing::info!(
                        "[Clan {}] {}: {}",
                        clan.name,
                        sender_account_id,
                        message
                    );
                    true
                } else {
                    false
                }
            }
            None => false,
        }
    }

    /// 添加联盟关系
    pub fn add_alliance(
        &self,
        clan_id: i32,
        target_clan_id: i32,
        alliance_type: AllianceType,
    ) -> ClanResult {
        let clans = self.clans.read();

        // 验证两个公会都存在
        let target_name = match clans.get(&target_clan_id) {
            Some(c) => c.read().name.clone(),
            None => return ClanResult::NotFound,
        };

        match clans.get(&clan_id) {
            Some(clan_arc) => {
                let mut clan = clan_arc.write();
                clan.add_alliance(ClanAlliance {
                    clan_id: target_clan_id,
                    alliance_type,
                    name: target_name,
                });
                ClanResult::Success
            }
            None => ClanResult::NotFound,
        }
    }

    /// 移除联盟关系
    pub fn remove_alliance(&self, clan_id: i32, target_clan_id: i32) -> ClanResult {
        let clans = self.clans.read();
        match clans.get(&clan_id) {
            Some(clan_arc) => {
                clan_arc.write().remove_alliance(target_clan_id);
                ClanResult::Success
            }
            None => ClanResult::NotFound,
        }
    }

    /// 获取同盟/敌对数量
    pub fn alliance_count(&self, clan_id: i32, alliance_type: AllianceType) -> usize {
        let clans = self.clans.read();
        match clans.get(&clan_id) {
            Some(clan_arc) => clan_arc.read().alliance_count(alliance_type),
            None => 0,
        }
    }
}

impl Default for ClanManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_clan(id: i32, name: &str) -> Clan {
        Clan::new(
            id,
            name.to_string(),
            "Master".to_string(),
            "prontera".to_string(),
            5,
        )
    }

    #[test]
    fn test_clan_new() {
        let clan = create_test_clan(1, "TestClan");
        assert_eq!(clan.id, 1);
        assert_eq!(clan.name, "TestClan");
        assert_eq!(clan.max_member, 5);
        assert!(clan.members.is_empty());
    }

    #[test]
    fn test_clan_add_remove_member() {
        let mut clan = create_test_clan(1, "TestClan");
        let char_id = Uuid::new_v4();

        assert!(clan.add_member(ClanMember {
            char_id,
            account_id: 1001,
            name: "Player1".to_string(),
            online: true,
        }));
        assert_eq!(clan.members.len(), 1);

        let removed = clan.remove_member(&char_id);
        assert!(removed.is_some());
        assert_eq!(clan.members.len(), 0);
    }

    #[test]
    fn test_clan_full() {
        let mut clan = create_test_clan(1, "SmallClan");
        clan.max_member = 2;

        assert!(clan.add_member(ClanMember {
            char_id: Uuid::new_v4(),
            account_id: 1001,
            name: "P1".to_string(),
            online: true,
        }));
        assert!(clan.add_member(ClanMember {
            char_id: Uuid::new_v4(),
            account_id: 1002,
            name: "P2".to_string(),
            online: true,
        }));
        assert!(clan.is_full());
        assert!(!clan.add_member(ClanMember {
            char_id: Uuid::new_v4(),
            account_id: 1003,
            name: "P3".to_string(),
            online: true,
        }));
    }

    #[test]
    fn test_clan_online_offline() {
        let mut clan = create_test_clan(1, "TestClan");
        let char_id = Uuid::new_v4();

        clan.add_member(ClanMember {
            char_id,
            account_id: 1001,
            name: "Player1".to_string(),
            online: false,
        });
        assert_eq!(clan.connect_member, 0);

        clan.member_online(&char_id);
        assert_eq!(clan.connect_member, 1);

        clan.member_offline(&char_id);
        assert_eq!(clan.connect_member, 0);
    }

    #[test]
    fn test_clan_alliance() {
        let mut clan = create_test_clan(1, "TestClan");

        clan.add_alliance(ClanAlliance {
            clan_id: 2,
            alliance_type: AllianceType::Ally,
            name: "AllyClan".to_string(),
        });
        assert_eq!(clan.alliance_count(AllianceType::Ally), 1);

        clan.add_alliance(ClanAlliance {
            clan_id: 3,
            alliance_type: AllianceType::Opposition,
            name: "EnemyClan".to_string(),
        });
        assert_eq!(clan.alliance_count(AllianceType::Opposition), 1);

        // 重复添加应被忽略
        clan.add_alliance(ClanAlliance {
            clan_id: 2,
            alliance_type: AllianceType::Ally,
            name: "AllyClan".to_string(),
        });
        assert_eq!(clan.alliance_count(AllianceType::Ally), 1);

        clan.remove_alliance(2);
        assert_eq!(clan.alliance_count(AllianceType::Ally), 0);
    }

    #[test]
    fn test_clan_manager_load_and_find() {
        let manager = ClanManager::new();
        manager.load_clans(vec![
            create_test_clan(1, "Alpha"),
            create_test_clan(2, "Beta"),
        ]);

        assert_eq!(manager.clan_count(), 2);
        assert!(manager.get_clan(1).is_some());
        assert!(manager.get_clan(3).is_none());
        assert!(manager.find_clan_by_name("alpha").is_some());
        assert!(manager.find_clan_by_name("Gamma").is_none());
    }

    #[test]
    fn test_clan_manager_member_join_leave() {
        let manager = ClanManager::new();
        manager.load_clans(vec![create_test_clan(1, "TestClan")]);

        let char_id = Uuid::new_v4();

        // 加入
        let result = manager.member_join(1, char_id, 1001, "Player1".to_string(), false);
        assert_eq!(result, ClanResult::Success);

        // 重复加入
        let result = manager.member_join(1, char_id, 1001, "Player1".to_string(), false);
        assert_eq!(result, ClanResult::AlreadyJoined);

        // 不存在的公会
        let result = manager.member_join(99, Uuid::new_v4(), 1002, "P2".to_string(), false);
        assert_eq!(result, ClanResult::NotFound);

        // 离开
        let result = manager.member_leave(1, char_id, 1001, false);
        assert_eq!(result, ClanResult::Success);

        // 成员不存在
        let result = manager.member_leave(1, char_id, 1001, false);
        assert_eq!(result, ClanResult::MemberNotFound);
    }

    #[test]
    fn test_clan_manager_instance_block() {
        let mut clan = create_test_clan(1, "TestClan");
        clan.instance_id = 42;
        let manager = ClanManager::new();
        manager.load_clans(vec![clan]);

        let char_id = Uuid::new_v4();
        let result = manager.member_join(1, char_id, 1001, "Player1".to_string(), true);
        assert_eq!(result, ClanResult::InstanceBlocked);
    }

    #[test]
    fn test_clan_manager_alliance() {
        let manager = ClanManager::new();
        manager.load_clans(vec![
            create_test_clan(1, "Alpha"),
            create_test_clan(2, "Beta"),
        ]);

        let result = manager.add_alliance(1, 2, AllianceType::Ally);
        assert_eq!(result, ClanResult::Success);
        assert_eq!(manager.alliance_count(1, AllianceType::Ally), 1);

        let result = manager.remove_alliance(1, 2);
        assert_eq!(result, ClanResult::Success);
        assert_eq!(manager.alliance_count(1, AllianceType::Ally), 0);
    }
}
