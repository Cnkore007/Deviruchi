//! 决斗系统
//!
//! 对应 rAthena 的 `src/map/duel.cpp`，提供 1v1 / 多人决斗功能。

use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

/// 决斗状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuelState {
    /// 等待对方接受
    Pending,
    /// 决斗进行中
    Active,
    /// 决斗已结束
    Finished,
}

/// 决斗数据
///
/// 对应 rAthena 的 `struct duel`。
#[derive(Debug, Clone)]
pub struct Duel {
    /// 决斗 ID
    pub id: usize,
    /// 创建者 ID
    pub creator_id: Uuid,
    /// 成员数（已接受）
    pub members_count: usize,
    /// 邀请数（待接受）
    pub invites_count: usize,
    /// 最大玩家限制（0 = 无限制）
    pub max_players_limit: usize,
    /// 状态
    pub state: DuelState,
    /// 成员列表
    pub members: Vec<Uuid>,
    /// 邀请列表 (player_id -> 邀请者)
    pub invites: HashMap<Uuid, Uuid>,
    /// 创建时间（分钟级时间戳）
    pub created_at: i64,
}

impl Duel {
    /// 创建新决斗
    pub fn new(id: usize, creator_id: Uuid, max_players: usize) -> Self {
        Self {
            id,
            creator_id,
            members_count: 1,
            invites_count: 0,
            max_players_limit: max_players,
            state: DuelState::Active,
            members: vec![creator_id],
            invites: HashMap::new(),
            created_at: Self::current_timestamp(),
        }
    }

    /// 检查是否满员
    pub fn is_full(&self) -> bool {
        if self.max_players_limit == 0 {
            return false;
        }
        self.members_count >= self.max_players_limit
    }

    /// 获取当前时间戳（分钟级）
    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64 / 60)
            .unwrap_or(0)
    }
}

/// 决斗操作结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuelResult {
    /// 操作成功
    Success,
    /// 决斗不存在
    NotFound,
    /// 玩家已在决斗中
    AlreadyInDuel,
    /// 决斗已满员
    Full,
    /// 无待处理的邀请
    NoInvite,
    /// 冷却时间未到
    Cooldown,
    /// 玩家不在决斗中
    NotInDuel,
}

/// 决斗管理器
///
/// 管理所有决斗实例，提供创建、邀请、接受、拒绝、离开等操作。
pub struct DuelManager {
    /// 决斗列表 (duel_id -> Duel)
    duels: RwLock<HashMap<usize, Duel>>,
    /// 下一个决斗 ID
    next_id: RwLock<usize>,
    /// 玩家当前决斗 (player_id -> duel_id)
    player_duels: RwLock<HashMap<Uuid, usize>>,
    /// 玩家待处理邀请 (player_id -> duel_id)
    player_invites: RwLock<HashMap<Uuid, usize>>,
    /// 玩家上次决斗时间 (player_id -> timestamp)
    player_last_duel: RwLock<HashMap<Uuid, i64>>,
    /// 决斗冷却时间（分钟）
    cooldown_minutes: i64,
}

impl DuelManager {
    /// 创建决斗管理器
    pub fn new() -> Self {
        Self {
            duels: RwLock::new(HashMap::new()),
            next_id: RwLock::new(1),
            player_duels: RwLock::new(HashMap::new()),
            player_invites: RwLock::new(HashMap::new()),
            player_last_duel: RwLock::new(HashMap::new()),
            cooldown_minutes: 0, // 0 = 无冷却
        }
    }

    /// 设置冷却时间
    pub fn with_cooldown(mut self, minutes: i64) -> Self {
        self.cooldown_minutes = minutes;
        self
    }

    /// 获取决斗总数
    pub fn total_duels(&self) -> usize {
        self.duels.read().len()
    }

    /// 获取活跃决斗数（成员 > 1）
    pub fn active_duels(&self) -> usize {
        self.duels
            .read()
            .values()
            .filter(|d| d.members_count > 1)
            .count()
    }

    /// 检查玩家是否在决斗中
    pub fn is_in_duel(&self, player_id: &Uuid) -> bool {
        self.player_duels.read().contains_key(player_id)
    }

    /// 检查决斗是否存在
    pub fn duel_exists(&self, duel_id: usize) -> bool {
        self.duels.read().contains_key(&duel_id)
    }

    /// 获取玩家的决斗 ID
    pub fn get_player_duel(&self, player_id: &Uuid) -> Option<usize> {
        self.player_duels.read().get(player_id).copied()
    }

    /// 获取玩家待处理的邀请
    pub fn get_player_invite(&self, player_id: &Uuid) -> Option<usize> {
        self.player_invites.read().get(player_id).copied()
    }

    /// 创建决斗
    pub fn create_duel(&self, creator_id: Uuid, max_players: usize) -> Result<usize, DuelResult> {
        // 检查冷却
        if self.cooldown_minutes > 0 {
            let last_duel = self.player_last_duel.read();
            if let Some(&last_time) = last_duel.get(&creator_id) {
                let now = Duel::current_timestamp();
                if now - last_time < self.cooldown_minutes {
                    return Err(DuelResult::Cooldown);
                }
            }
        }

        // 检查是否已在决斗中
        if self.is_in_duel(&creator_id) {
            return Err(DuelResult::AlreadyInDuel);
        }

        let mut next_id = self.next_id.write();
        let duel_id = *next_id;
        *next_id += 1;

        let duel = Duel::new(duel_id, creator_id, max_players);

        self.duels.write().insert(duel_id, duel);
        self.player_duels.write().insert(creator_id, duel_id);

        tracing::info!("Duel {} created by player {:?}", duel_id, creator_id);
        Ok(duel_id)
    }

    /// 邀请玩家加入决斗
    pub fn invite_player(&self, duel_id: usize, inviter_id: Uuid, target_id: Uuid) -> DuelResult {
        let mut duels = self.duels.write();
        let duel = match duels.get_mut(&duel_id) {
            Some(d) => d,
            None => return DuelResult::NotFound,
        };

        // 检查邀请者是否是决斗成员
        if !duel.members.contains(&inviter_id) {
            return DuelResult::NotInDuel;
        }

        // 检查是否满员
        if duel.is_full() {
            return DuelResult::Full;
        }

        // 检查目标是否已在决斗中
        if self.player_duels.read().contains_key(&target_id) {
            return DuelResult::AlreadyInDuel;
        }

        duel.invites.insert(target_id, inviter_id);
        duel.invites_count += 1;

        drop(duels);

        self.player_invites.write().insert(target_id, duel_id);

        tracing::info!(
            "Player {:?} invited {:?} to duel {}",
            inviter_id,
            target_id,
            duel_id
        );
        DuelResult::Success
    }

    /// 接受决斗邀请
    pub fn accept_invite(&self, player_id: Uuid) -> DuelResult {
        let invite_duel_id = match self.player_invites.read().get(&player_id).copied() {
            Some(id) => id,
            None => return DuelResult::NoInvite,
        };

        let mut duels = self.duels.write();
        let duel = match duels.get_mut(&invite_duel_id) {
            Some(d) => d,
            None => return DuelResult::NotFound,
        };

        // 检查满员
        if duel.is_full() {
            return DuelResult::Full;
        }

        duel.members.push(player_id);
        duel.members_count += 1;
        duel.invites.remove(&player_id);
        duel.invites_count = duel.invites_count.saturating_sub(1);

        drop(duels);

        self.player_duels.write().insert(player_id, invite_duel_id);
        self.player_invites.write().remove(&player_id);

        tracing::info!(
            "Player {:?} accepted duel {} invitation",
            player_id,
            invite_duel_id
        );
        DuelResult::Success
    }

    /// 拒绝决斗邀请
    pub fn reject_invite(&self, player_id: Uuid) -> DuelResult {
        let invite_duel_id = match self.player_invites.write().remove(&player_id) {
            Some(id) => id,
            None => return DuelResult::NoInvite,
        };

        let mut duels = self.duels.write();
        if let Some(duel) = duels.get_mut(&invite_duel_id) {
            duel.invites.remove(&player_id);
            duel.invites_count = duel.invites_count.saturating_sub(1);
        }

        tracing::info!(
            "Player {:?} rejected duel {} invitation",
            player_id,
            invite_duel_id
        );
        DuelResult::Success
    }

    /// 离开决斗
    pub fn leave_duel(&self, player_id: Uuid) -> DuelResult {
        let duel_id = match self.player_duels.write().remove(&player_id) {
            Some(id) => id,
            None => return DuelResult::NotInDuel,
        };

        let mut duels = self.duels.write();
        if let Some(duel) = duels.get_mut(&duel_id) {
            duel.members.retain(|&m| m != player_id);
            duel.members_count = duel.members_count.saturating_sub(1);

            // 记录决斗时间
            self.player_last_duel
                .write()
                .insert(player_id, Duel::current_timestamp());

            // 如果决斗无人了，清理
            if duel.members_count == 0 {
                // 清理所有待处理邀请
                let invites: Vec<Uuid> = duel.invites.keys().copied().collect();
                duels.remove(&duel_id);
                drop(duels);

                let mut player_invites = self.player_invites.write();
                for invitee in invites {
                    player_invites.remove(&invitee);
                }

                tracing::info!("Duel {} ended (no members left)", duel_id);
            } else {
                tracing::info!("Player {:?} left duel {}", player_id, duel_id);
            }
        }

        DuelResult::Success
    }

    /// 获取决斗信息
    pub fn get_duel_info(&self, duel_id: usize) -> Option<Duel> {
        self.duels.read().get(&duel_id).cloned()
    }

    /// 清理所有决斗
    pub fn clear(&self) {
        self.duels.write().clear();
        self.player_duels.write().clear();
        self.player_invites.write().clear();
    }
}

impl Default for DuelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duel_create() {
        let manager = DuelManager::new();
        let player = Uuid::new_v4();

        let duel_id = manager.create_duel(player, 2).unwrap();
        assert!(duel_id > 0);
        assert_eq!(manager.total_duels(), 1);
        assert!(manager.is_in_duel(&player));
    }

    #[test]
    fn test_duel_create_already_in_duel() {
        let manager = DuelManager::new();
        let player = Uuid::new_v4();

        manager.create_duel(player, 2).unwrap();
        assert_eq!(
            manager.create_duel(player, 2),
            Err(DuelResult::AlreadyInDuel)
        );
    }

    #[test]
    fn test_duel_invite_and_accept() {
        let manager = DuelManager::new();
        let creator = Uuid::new_v4();
        let invitee = Uuid::new_v4();

        let duel_id = manager.create_duel(creator, 2).unwrap();

        // 邀请
        let result = manager.invite_player(duel_id, creator, invitee);
        assert_eq!(result, DuelResult::Success);
        assert!(manager.get_player_invite(&invitee).is_some());

        // 接受
        let result = manager.accept_invite(invitee);
        assert_eq!(result, DuelResult::Success);
        assert!(manager.is_in_duel(&invitee));
        assert!(manager.get_player_invite(&invitee).is_none());
    }

    #[test]
    fn test_duel_invite_and_reject() {
        let manager = DuelManager::new();
        let creator = Uuid::new_v4();
        let invitee = Uuid::new_v4();

        let duel_id = manager.create_duel(creator, 2).unwrap();

        manager.invite_player(duel_id, creator, invitee);
        let result = manager.reject_invite(invitee);
        assert_eq!(result, DuelResult::Success);
        assert!(!manager.is_in_duel(&invitee));
    }

    #[test]
    fn test_duel_full() {
        let manager = DuelManager::new();
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let p3 = Uuid::new_v4();

        let duel_id = manager.create_duel(p1, 2).unwrap();

        manager.invite_player(duel_id, p1, p2);
        manager.accept_invite(p2);

        // 已满，邀请应该失败
        let result = manager.invite_player(duel_id, p1, p3);
        assert_eq!(result, DuelResult::Full);
    }

    #[test]
    fn test_duel_leave() {
        let manager = DuelManager::new();
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();

        let duel_id = manager.create_duel(p1, 0).unwrap();
        manager.invite_player(duel_id, p1, p2);
        manager.accept_invite(p2);

        assert_eq!(manager.active_duels(), 1);

        // p2 离开
        let result = manager.leave_duel(p2);
        assert_eq!(result, DuelResult::Success);
        assert!(!manager.is_in_duel(&p2));

        // p1 离开（决斗结束）
        let result = manager.leave_duel(p1);
        assert_eq!(result, DuelResult::Success);
        assert_eq!(manager.total_duels(), 0);
    }

    #[test]
    fn test_duel_leave_not_in_duel() {
        let manager = DuelManager::new();
        let player = Uuid::new_v4();

        assert_eq!(manager.leave_duel(player), DuelResult::NotInDuel);
    }

    #[test]
    fn test_duel_no_invite() {
        let manager = DuelManager::new();
        let player = Uuid::new_v4();

        assert_eq!(manager.accept_invite(player), DuelResult::NoInvite);
        assert_eq!(manager.reject_invite(player), DuelResult::NoInvite);
    }

    #[test]
    fn test_duel_not_found() {
        let manager = DuelManager::new();
        let player = Uuid::new_v4();

        assert_eq!(
            manager.invite_player(999, player, Uuid::new_v4()),
            DuelResult::NotFound
        );
    }

    #[test]
    fn test_duel_cooldown() {
        let manager = DuelManager::new().with_cooldown(5);
        let player = Uuid::new_v4();

        // 第一次创建成功
        let _duel_id = manager.create_duel(player, 0).unwrap();
        manager.leave_duel(player);

        // 冷却期内应失败
        assert_eq!(manager.create_duel(player, 0), Err(DuelResult::Cooldown));
    }

    #[test]
    fn test_duel_info() {
        let manager = DuelManager::new();
        let player = Uuid::new_v4();

        let duel_id = manager.create_duel(player, 2).unwrap();
        let info = manager.get_duel_info(duel_id).unwrap();

        assert_eq!(info.id, duel_id);
        assert_eq!(info.creator_id, player);
        assert_eq!(info.members_count, 1);
        assert_eq!(info.max_players_limit, 2);
    }

    #[test]
    fn test_duel_clear() {
        let manager = DuelManager::new();
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();

        manager.create_duel(p1, 0).unwrap();
        manager.create_duel(p2, 0).unwrap();

        assert_eq!(manager.total_duels(), 2);

        manager.clear();
        assert_eq!(manager.total_duels(), 0);
    }
}
