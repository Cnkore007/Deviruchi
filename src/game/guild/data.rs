use std::collections::HashMap;
use uuid::Uuid;

/// 公会职位
#[derive(Debug, Clone)]
pub struct GuildPosition {
    pub id: u8,
    pub name: String,
    pub can_invite: bool,
    pub can_expel: bool,
    pub can_use_storage: bool,
    pub can_use_skill: bool,
}

impl GuildPosition {
    pub fn default(id: u8) -> Self {
        match id {
            0 => Self {
                id,
                name: "Guild Master".to_string(),
                can_invite: true,
                can_expel: true,
                can_use_storage: true,
                can_use_skill: true,
            },
            1 => Self {
                id,
                name: "Vice Master".to_string(),
                can_invite: true,
                can_expel: true,
                can_use_storage: true,
                can_use_skill: true,
            },
            _ => Self {
                id,
                name: format!("Position {}", id),
                can_invite: id <= 2,
                can_expel: id <= 1,
                can_use_storage: id <= 3,
                can_use_skill: id <= 4,
            },
        }
    }
}

/// 公会成员
#[derive(Debug, Clone)]
pub struct GuildMember {
    pub player_id: Uuid,
    pub char_id: u32,
    pub name: String,
    pub position_id: u8,
    pub level: u16,
    pub job: u16,
    pub contribution: u32,
    pub online: bool,
    pub map_name: String,
}

impl GuildMember {
    pub fn new(player_id: Uuid, name: String, position_id: u8) -> Self {
        Self {
            player_id,
            char_id: 0,
            name,
            position_id,
            level: 1,
            job: 0,
            contribution: 0,
            online: true,
            map_name: String::new(),
        }
    }
}

/// 公会信息
#[derive(Debug, Clone)]
pub struct Guild {
    pub id: Uuid,
    pub name: String,
    pub master_name: String,
    pub level: u8,
    pub exp: u64,
    pub max_exp: u64,
    pub member_count: u32,
    pub max_members: u32,
    pub average_level: u16,
    pub notice: String,
    pub emblem_id: u32,
    pub positions: Vec<GuildPosition>,
    pub members: HashMap<Uuid, GuildMember>,
}

impl Guild {
    pub fn new(name: String, master_name: String) -> Self {
        let positions: Vec<_> = (0..5).map(GuildPosition::default).collect();

        Self {
            id: Uuid::new_v4(),
            name,
            master_name,
            level: 1,
            exp: 0,
            max_exp: 1000,
            member_count: 0,
            max_members: 16,
            average_level: 1,
            notice: String::new(),
            emblem_id: 0,
            positions,
            members: HashMap::new(),
        }
    }

    /// 添加成员
    pub fn add_member(&mut self, player_id: Uuid, name: String) -> bool {
        if self.members.len() >= self.max_members as usize {
            return false;
        }

        let member = GuildMember::new(player_id, name, 4);
        self.members.insert(player_id, member);
        self.member_count = self.members.len() as u32;
        self.update_average_level();
        true
    }

    /// 移除成员
    pub fn remove_member(&mut self, player_id: &Uuid) -> bool {
        if self.members.remove(player_id).is_some() {
            self.member_count = self.members.len() as u32;
            self.update_average_level();
            true
        } else {
            false
        }
    }

    /// 获取成员
    pub fn get_member(&self, player_id: &Uuid) -> Option<&GuildMember> {
        self.members.get(player_id)
    }

    /// 获取成员可变引用
    pub fn get_member_mut(&mut self, player_id: &Uuid) -> Option<&mut GuildMember> {
        self.members.get_mut(player_id)
    }

    /// 检查是否为成员
    pub fn is_member(&self, player_id: &Uuid) -> bool {
        self.members.contains_key(player_id)
    }

    /// 更新成员职位
    pub fn change_position(&mut self, player_id: &Uuid, position_id: u8) -> bool {
        if let Some(member) = self.members.get_mut(player_id)
            && (position_id as usize) < self.positions.len()
        {
            member.position_id = position_id;
            return true;
        }
        false
    }

    /// 获取在线成员数量
    pub fn online_count(&self) -> usize {
        self.members.values().filter(|m| m.online).count()
    }

    /// 更新平均等级
    fn update_average_level(&mut self) {
        if self.members.is_empty() {
            self.average_level = 1;
        } else {
            let total: u32 = self.members.values().map(|m| m.level as u32).sum();
            self.average_level = (total / self.members.len() as u32) as u16;
        }
    }

    /// 添加经验
    pub fn add_exp(&mut self, exp: u64) -> bool {
        self.exp += exp;

        if self.exp >= self.max_exp && self.level < 50 {
            self.exp -= self.max_exp;
            self.level += 1;
            self.max_exp = self.calculate_max_exp();
            self.max_members = self.calculate_max_members();
            true
        } else {
            false
        }
    }

    /// 计算升级所需经验
    fn calculate_max_exp(&self) -> u64 {
        1000 * (self.level as u64).pow(2)
    }

    /// 计算最大成员数
    fn calculate_max_members(&self) -> u32 {
        16 + (self.level as u32 - 1) * 2
    }

    /// 设置公告
    pub fn set_notice(&mut self, notice: String) {
        self.notice = notice;
    }

    /// 检查是否有权限
    pub fn has_permission(&self, player_id: &Uuid, permission: GuildPermission) -> bool {
        let Some(member) = self.members.get(player_id) else {
            return false;
        };

        let Some(position) = self.positions.get(member.position_id as usize) else {
            return false;
        };

        match permission {
            GuildPermission::Invite => position.can_invite,
            GuildPermission::Expel => position.can_expel,
            GuildPermission::UseStorage => position.can_use_storage,
            GuildPermission::UseSkill => position.can_use_skill,
        }
    }
}

/// 公会权限
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildPermission {
    Invite,
    Expel,
    UseStorage,
    UseSkill,
}
