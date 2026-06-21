//! 公会联盟/家族数据结构
//!
//! 对应 rAthena `src/common/mmo.hpp` 中的 `struct clan` 和 `struct clan_alliance`。

use std::collections::HashMap;
use uuid::Uuid;

/// 公会联盟关系类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllianceType {
    /// 同盟
    Ally = 0,
    /// 敌对
    Opposition = 1,
}

/// 公会联盟关系
#[derive(Debug, Clone)]
pub struct ClanAlliance {
    /// 关联公会 ID
    pub clan_id: i32,
    /// 关系类型
    pub alliance_type: AllianceType,
    /// 关联公会名称
    pub name: String,
}

/// 公会成员信息
#[derive(Debug, Clone)]
pub struct ClanMember {
    /// 角色 ID
    pub char_id: Uuid,
    /// 账号 ID
    pub account_id: u32,
    /// 角色名
    pub name: String,
    /// 是否在线
    pub online: bool,
}

/// 公会数据
///
/// 对应 rAthena 的 `struct clan`，表示一个公会联盟/家族。
#[derive(Debug, Clone)]
pub struct Clan {
    /// 公会 ID
    pub id: i32,
    /// 公会名称
    pub name: String,
    /// 会长名称
    pub master: String,
    /// 公会地图
    pub map: String,
    /// 最大成员数
    pub max_member: usize,
    /// 在线成员数
    pub connect_member: usize,
    /// 成员列表（char_id -> ClanMember）
    pub members: HashMap<Uuid, ClanMember>,
    /// 联盟关系列表
    pub alliances: Vec<ClanAlliance>,
    /// 关联副本 ID（0 表示无）
    pub instance_id: u16,
}

impl Clan {
    /// 创建新公会
    pub fn new(id: i32, name: String, master: String, map: String, max_member: usize) -> Self {
        Self {
            id,
            name,
            master,
            map,
            max_member,
            connect_member: 0,
            members: HashMap::new(),
            alliances: Vec::new(),
            instance_id: 0,
        }
    }

    /// 获取在线成员数
    pub fn online_count(&self) -> usize {
        self.members.values().filter(|m| m.online).count()
    }

    /// 检查是否已满员
    pub fn is_full(&self) -> bool {
        self.members.len() >= self.max_member
    }

    /// 获取一个可用的在线成员
    pub fn get_available_member(&self) -> Option<&ClanMember> {
        self.members.values().find(|m| m.online)
    }

    /// 按 account_id 查找成员索引
    pub fn find_member_by_account(&self, account_id: u32) -> Option<&ClanMember> {
        self.members.values().find(|m| m.account_id == account_id)
    }

    /// 添加成员
    pub fn add_member(&mut self, member: ClanMember) -> bool {
        if self.is_full() {
            return false;
        }
        self.members.insert(member.char_id, member);
        true
    }

    /// 移除成员
    pub fn remove_member(&mut self, char_id: &Uuid) -> Option<ClanMember> {
        self.members.remove(char_id)
    }

    /// 成员上线
    pub fn member_online(&mut self, char_id: &Uuid) {
        if let Some(member) = self.members.get_mut(char_id)
            && !member.online
        {
            member.online = true;
            self.connect_member += 1;
        }
    }

    /// 成员下线
    pub fn member_offline(&mut self, char_id: &Uuid) {
        if let Some(member) = self.members.get_mut(char_id)
            && member.online
        {
            member.online = false;
            self.connect_member -= 1;
        }
    }

    /// 获取同盟/敌对数量
    pub fn alliance_count(&self, alliance_type: AllianceType) -> usize {
        self.alliances
            .iter()
            .filter(|a| a.alliance_type == alliance_type)
            .count()
    }

    /// 添加联盟关系
    pub fn add_alliance(&mut self, alliance: ClanAlliance) {
        // 避免重复
        if !self.alliances.iter().any(|a| a.clan_id == alliance.clan_id) {
            self.alliances.push(alliance);
        }
    }

    /// 移除联盟关系
    pub fn remove_alliance(&mut self, clan_id: i32) {
        self.alliances.retain(|a| a.clan_id != clan_id);
    }
}
