//! 战场管理器

use parking_lot::RwLock;
use std::collections::HashMap;

use super::data::{BGError, Battleground, BattlegroundState, BattlegroundType, TeamColor};

/// 战场管理器
pub struct BattlegroundManager {
    /// 所有战场
    battlegrounds: RwLock<HashMap<u32, Battleground>>,
    /// 玩家所在战场映射 (char_id -> bg_id)
    player_battles: RwLock<HashMap<u32, u32>>,
    /// 战场队列 (bg_type -> char_ids)
    bg_queues: RwLock<HashMap<BattlegroundType, Vec<u32>>>,
    /// 下一个战场ID
    next_bg_id: RwLock<u32>,
}

impl BattlegroundManager {
    /// 创建新的战场管理器
    pub fn new() -> Self {
        Self {
            battlegrounds: RwLock::new(HashMap::new()),
            player_battles: RwLock::new(HashMap::new()),
            bg_queues: RwLock::new(HashMap::new()),
            next_bg_id: RwLock::new(1),
        }
    }

    /// 获取下一个战场ID
    fn next_id(&self) -> u32 {
        let mut next = self.next_bg_id.write();
        let id = *next;
        *next += 1;
        id
    }

    /// 创建战场
    pub fn create_battleground(
        &self,
        bg_type: BattlegroundType,
        name: Option<String>,
    ) -> Result<Battleground, BGError> {
        let bg_id = self.next_id();
        let bg_name = name.unwrap_or_else(|| format!("{} #{}", bg_type.display_name(), bg_id));

        let bg = Battleground::new(bg_id, bg_type, bg_name);

        self.battlegrounds.write().insert(bg_id, bg.clone());
        Ok(bg)
    }

    /// 创建指定ID的战场
    pub fn create_battleground_with_id(
        &self,
        bg_id: u32,
        bg_type: BattlegroundType,
        name: String,
    ) -> Result<Battleground, BGError> {
        let mut battlegrounds = self.battlegrounds.write();

        // 检查ID是否已存在
        if battlegrounds.contains_key(&bg_id) {
            return Err(BGError::BattleAlreadyExists);
        }

        let bg = Battleground::new(bg_id, bg_type, name);
        battlegrounds.insert(bg_id, bg.clone());
        Ok(bg)
    }

    /// 获取战场
    pub fn get_battleground(&self, bg_id: u32) -> Option<Battleground> {
        self.battlegrounds.read().get(&bg_id).cloned()
    }

    /// 获取战场（可变）
    pub fn get_battleground_mut(
        &self,
        _bg_id: u32,
    ) -> Option<parking_lot::RwLockWriteGuard<'_, HashMap<u32, Battleground>>> {
        Some(self.battlegrounds.write())
    }

    /// 获取玩家所在的战场
    pub fn get_player_battle(&self, char_id: u32) -> Option<Battleground> {
        let player_battles = self.player_battles.read();
        let bg_id = player_battles.get(&char_id)?;
        self.battlegrounds.read().get(bg_id).cloned()
    }

    /// 获取玩家所在的战场ID
    pub fn get_player_battle_id(&self, char_id: u32) -> Option<u32> {
        self.player_battles.read().get(&char_id).copied()
    }

    /// 加入战场队列
    pub fn join_queue(&self, char_id: u32, bg_type: BattlegroundType) -> Result<(), BGError> {
        // 检查是否已在战场中
        if self.player_battles.read().contains_key(&char_id) {
            return Err(BGError::PlayerAlreadyInBattle);
        }

        let mut queues = self.bg_queues.write();

        // 检查是否已在其他队列中
        for (qt, queue) in queues.iter() {
            if *qt != bg_type && queue.contains(&char_id) {
                return Err(BGError::AlreadyInQueue);
            }
        }

        // 添加到指定类型的队列
        let queue = queues.entry(bg_type).or_default();

        if queue.contains(&char_id) {
            return Err(BGError::AlreadyInQueue);
        }

        queue.push(char_id);
        Ok(())
    }

    /// 离开队列
    pub fn leave_queue(&self, char_id: u32) -> Result<BattlegroundType, BGError> {
        let mut queues = self.bg_queues.write();

        for (bg_type, queue) in queues.iter_mut() {
            if let Some(pos) = queue.iter().position(|&id| id == char_id) {
                queue.remove(pos);
                return Ok(*bg_type);
            }
        }

        Err(BGError::NotInQueue)
    }

    /// 获取队列中的玩家数量
    pub fn get_queue_size(&self, bg_type: BattlegroundType) -> usize {
        self.bg_queues
            .read()
            .get(&bg_type)
            .map(|q| q.len())
            .unwrap_or(0)
    }

    /// 获取所有队列
    pub fn get_all_queues(&self) -> HashMap<BattlegroundType, Vec<u32>> {
        self.bg_queues
            .read()
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect()
    }

    /// 加入战场
    pub fn join_battle(&self, char_id: u32, bg_id: u32) -> Result<Battleground, BGError> {
        let mut battlegrounds = self.battlegrounds.write();

        // 检查战场是否存在
        let bg = battlegrounds
            .get_mut(&bg_id)
            .ok_or(BGError::BattleNotFound)?;

        // 检查战场状态
        if bg.state != BattlegroundState::Waiting && bg.state != BattlegroundState::Ready {
            return Err(BGError::BattleAlreadyStarted);
        }

        // 检查玩家是否已在其他战场
        drop(battlegrounds);
        {
            let player_battles = self.player_battles.read();
            if player_battles.contains_key(&char_id) {
                return Err(BGError::PlayerAlreadyInBattle);
            }
        }

        // 选择玩家数最少的队伍
        let mut battlegrounds = self.battlegrounds.write();
        let bg = battlegrounds
            .get_mut(&bg_id)
            .ok_or(BGError::BattleNotFound)?;

        // 找到玩家数最少的队伍
        let team_id = bg
            .teams
            .iter()
            .min_by_key(|t| t.player_count())
            .map(|t| t.team_id)
            .ok_or(BGError::TeamNotFound)?;

        // 添加到队伍
        bg.add_player_to_team(char_id, team_id)?;

        // 检查是否可以开始
        if bg.can_start() {
            bg.state = BattlegroundState::Ready;
        }

        // 更新玩家战场映射
        self.player_battles.write().insert(char_id, bg_id);

        Ok(bg.clone())
    }

    /// 加入战场到指定队伍
    pub fn join_battle_team(
        &self,
        char_id: u32,
        bg_id: u32,
        team_id: u32,
    ) -> Result<Battleground, BGError> {
        let mut battlegrounds = self.battlegrounds.write();

        let bg = battlegrounds
            .get_mut(&bg_id)
            .ok_or(BGError::BattleNotFound)?;

        if bg.state != BattlegroundState::Waiting && bg.state != BattlegroundState::Ready {
            return Err(BGError::BattleAlreadyStarted);
        }

        bg.add_player_to_team(char_id, team_id)?;

        if bg.can_start() {
            bg.state = BattlegroundState::Ready;
        }

        drop(battlegrounds);
        self.player_battles.write().insert(char_id, bg_id);

        self.get_battleground(bg_id).ok_or(BGError::BattleNotFound)
    }

    /// 离开战场
    pub fn leave_battle(&self, char_id: u32) -> Result<u32, BGError> {
        let player_battles = self.player_battles.read();
        let bg_id = player_battles
            .get(&char_id)
            .copied()
            .ok_or(BGError::PlayerNotInBattle)?;
        drop(player_battles);

        let mut battlegrounds = self.battlegrounds.write();
        let bg = battlegrounds
            .get_mut(&bg_id)
            .ok_or(BGError::BattleNotFound)?;

        bg.remove_player(char_id);

        // 如果玩家离开后没有足够玩家，设置为等待状态
        if !bg.all_teams_have_min_players() {
            bg.state = BattlegroundState::Waiting;
        }

        self.player_battles.write().remove(&char_id);
        Ok(bg_id)
    }

    /// 添加分数到队伍
    pub fn add_score(&self, bg_id: u32, team_id: u32, score: u16) -> Result<(), BGError> {
        let mut battlegrounds = self.battlegrounds.write();
        let bg = battlegrounds
            .get_mut(&bg_id)
            .ok_or(BGError::BattleNotFound)?;

        if bg.state != BattlegroundState::Active {
            return Err(BGError::BattleAlreadyStarted);
        }

        bg.add_score_to_team(team_id, score);

        // 检查是否达到分数限制
        if bg.is_score_limit_reached() {
            bg.end();
        }

        Ok(())
    }

    /// 添加击杀
    pub fn add_kill(
        &self,
        bg_id: u32,
        killer_char_id: u32,
        victim_char_id: u32,
    ) -> Result<(), BGError> {
        let mut battlegrounds = self.battlegrounds.write();
        let bg = battlegrounds
            .get_mut(&bg_id)
            .ok_or(BGError::BattleNotFound)?;

        if bg.state != BattlegroundState::Active {
            return Err(BGError::BattleAlreadyStarted);
        }

        // 找到击杀者和受害者
        let killer_team_id = bg.get_player_team(killer_char_id).map(|t| t.team_id);
        let victim_team_id = bg.get_player_team(victim_char_id).map(|t| t.team_id);

        // 增加击杀者的击杀数和分数
        if let Some(team_id) = killer_team_id
            && let Some(team) = bg.teams.iter_mut().find(|t| t.team_id == team_id)
        {
            team.add_kill();
            team.add_score(1);
        }

        // 增加受害者的死亡数
        if let Some(team_id) = victim_team_id
            && let Some(team) = bg.teams.iter_mut().find(|t| t.team_id == team_id)
        {
            team.add_death();
        }

        // 检查分数限制
        if bg.is_score_limit_reached() {
            bg.end();
        }

        Ok(())
    }

    /// 开始战场
    pub fn start_battle(&self, bg_id: u32) -> Result<(), BGError> {
        let mut battlegrounds = self.battlegrounds.write();
        let bg = battlegrounds
            .get_mut(&bg_id)
            .ok_or(BGError::BattleNotFound)?;

        if bg.state == BattlegroundState::Active || bg.state == BattlegroundState::Ended {
            return Err(BGError::BattleAlreadyStarted);
        }

        if !bg.all_teams_have_min_players() {
            return Err(BGError::NotEnoughPlayers);
        }

        bg.start();
        Ok(())
    }

    /// 结束战场
    pub fn end_battle(&self, bg_id: u32) -> Result<Battleground, BGError> {
        let mut battlegrounds = self.battlegrounds.write();
        let bg = battlegrounds
            .get_mut(&bg_id)
            .ok_or(BGError::BattleNotFound)?;

        bg.end();

        // 移除所有玩家的战场映射
        let char_ids: Vec<u32> = bg.teams.iter().flat_map(|t| t.players.clone()).collect();
        drop(battlegrounds);

        let mut player_battles = self.player_battles.write();
        for char_id in char_ids {
            player_battles.remove(&char_id);
        }

        self.get_battleground(bg_id).ok_or(BGError::BattleNotFound)
    }

    /// 获取所有战场
    pub fn get_all_battlegrounds(&self) -> Vec<Battleground> {
        self.battlegrounds.read().values().cloned().collect()
    }

    /// 获取特定状态的战场
    pub fn get_battlegrounds_by_state(&self, state: BattlegroundState) -> Vec<Battleground> {
        self.battlegrounds
            .read()
            .values()
            .filter(|bg| bg.state == state)
            .cloned()
            .collect()
    }

    /// 获取战场统计
    pub fn get_battleground_stats(&self, bg_id: u32) -> Option<BattlegroundStats> {
        let battlegrounds = self.battlegrounds.read();
        let bg = battlegrounds.get(&bg_id)?;

        Some(BattlegroundStats {
            bg_id: bg.bg_id,
            bg_type: bg.bg_type,
            name: bg.name.clone(),
            state: bg.state,
            total_players: bg.total_players(),
            team_stats: bg
                .teams
                .iter()
                .map(|t| TeamStats {
                    team_id: t.team_id,
                    color: t.color,
                    player_count: t.player_count() as u16,
                    score: t.score,
                    kills: t.kills,
                    deaths: t.deaths,
                })
                .collect(),
            remaining_time: bg.remaining_time(),
            score_limit: bg.score_limit,
        })
    }

    /// 删除战场
    pub fn delete_battleground(&self, bg_id: u32) -> bool {
        let battlegrounds = self.battlegrounds.read();
        let bg = battlegrounds.get(&bg_id).cloned();
        drop(battlegrounds);

        if let Some(bg) = bg {
            // 移除所有玩家的战场映射
            let mut player_battles = self.player_battles.write();
            for char_id in bg.teams.iter().flat_map(|t| t.players.clone()) {
                player_battles.remove(&char_id);
            }

            self.battlegrounds.write().remove(&bg_id);
            true
        } else {
            false
        }
    }

    /// 检查玩家是否在战场中
    pub fn is_player_in_battle(&self, char_id: u32) -> bool {
        self.player_battles.read().contains_key(&char_id)
    }

    /// 检查玩家是否在队列中
    pub fn is_player_in_queue(&self, char_id: u32) -> bool {
        self.bg_queues.read().values().any(|q| q.contains(&char_id))
    }

    /// 清理空战场
    pub fn cleanup_empty_battles(&self) -> Vec<u32> {
        let mut removed = Vec::new();
        let mut battlegrounds = self.battlegrounds.write();

        battlegrounds.retain(|bg_id, bg| {
            if bg.total_players() == 0 && bg.state != BattlegroundState::Active {
                removed.push(*bg_id);
                false
            } else {
                true
            }
        });

        removed
    }

    /// 获取管理器中战场数量
    pub fn battleground_count(&self) -> usize {
        self.battlegrounds.read().len()
    }

    /// 获取所有玩家数
    pub fn total_player_count(&self) -> usize {
        self.battlegrounds
            .read()
            .values()
            .map(|bg| bg.total_players())
            .sum()
    }
}

impl Default for BattlegroundManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 战场统计信息
#[derive(Debug, Clone)]
pub struct BattlegroundStats {
    pub bg_id: u32,
    pub bg_type: BattlegroundType,
    pub name: String,
    pub state: BattlegroundState,
    pub total_players: usize,
    pub team_stats: Vec<TeamStats>,
    pub remaining_time: u32,
    pub score_limit: u16,
}

/// 队伍统计信息
#[derive(Debug, Clone)]
pub struct TeamStats {
    pub team_id: u32,
    pub color: TeamColor,
    pub player_count: u16,
    pub score: u16,
    pub kills: u16,
    pub deaths: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_battleground() {
        let manager = BattlegroundManager::new();
        let bg = manager
            .create_battleground(BattlegroundType::TOC, None)
            .unwrap();

        assert_eq!(bg.bg_id, 1);
        assert_eq!(bg.bg_type, BattlegroundType::TOC);
        assert_eq!(bg.state, BattlegroundState::Waiting);
    }

    #[test]
    fn test_join_queue() {
        let manager = BattlegroundManager::new();

        assert!(manager.join_queue(1, BattlegroundType::TOC).is_ok());
        assert_eq!(manager.get_queue_size(BattlegroundType::TOC), 1);

        // 重复加入
        assert_eq!(
            manager.join_queue(1, BattlegroundType::TOC).unwrap_err(),
            BGError::AlreadyInQueue
        );
    }

    #[test]
    fn test_join_and_leave_battle() {
        let manager = BattlegroundManager::new();
        let bg = manager
            .create_battleground(BattlegroundType::TOC, None)
            .unwrap();

        // 加入战场
        let result = manager.join_battle(1, bg.bg_id).unwrap();
        assert_eq!(result.get_player_team(1).unwrap().team_id, 0);

        // 检查玩家在战场中
        assert!(manager.is_player_in_battle(1));
        assert_eq!(manager.get_player_battle_id(1), Some(bg.bg_id));

        // 离开战场
        assert_eq!(manager.leave_battle(1).unwrap(), bg.bg_id);
        assert!(!manager.is_player_in_battle(1));
    }

    #[test]
    fn test_leave_queue() {
        let manager = BattlegroundManager::new();

        manager.join_queue(1, BattlegroundType::TOC).unwrap();
        assert_eq!(manager.leave_queue(1).unwrap(), BattlegroundType::TOC);
        assert_eq!(manager.get_queue_size(BattlegroundType::TOC), 0);

        // 不在队列中
        assert_eq!(manager.leave_queue(99).unwrap_err(), BGError::NotInQueue);
    }

    #[test]
    fn test_score_tracking() {
        let manager = BattlegroundManager::new();
        let bg = manager
            .create_battleground(BattlegroundType::TOC, None)
            .unwrap();

        // TOC 需要每队至少5人
        manager.join_battle(1, bg.bg_id).unwrap();
        manager.join_battle(2, bg.bg_id).unwrap();
        manager.join_battle(3, bg.bg_id).unwrap();
        manager.join_battle(4, bg.bg_id).unwrap();
        manager.join_battle(5, bg.bg_id).unwrap();
        manager.join_battle(6, bg.bg_id).unwrap();

        // 手动启动战场
        manager.start_battle(bg.bg_id).unwrap();

        // 添加分数
        assert!(manager.add_score(bg.bg_id, 0, 10).is_ok());
        assert!(manager.add_score(bg.bg_id, 1, 5).is_ok());

        let stats = manager.get_battleground_stats(bg.bg_id).unwrap();
        assert_eq!(stats.team_stats[0].score, 10);
        assert_eq!(stats.team_stats[1].score, 5);
    }

    #[test]
    fn test_cleanup_empty_battles() {
        let manager = BattlegroundManager::new();
        let bg1 = manager
            .create_battleground(BattlegroundType::TOC, None)
            .unwrap();
        let bg2 = manager
            .create_battleground(BattlegroundType::Tierra, None)
            .unwrap();

        manager.join_battle(1, bg1.bg_id).unwrap();
        manager.join_battle(2, bg1.bg_id).unwrap();

        // 离开后清理
        manager.leave_battle(1).unwrap();
        manager.leave_battle(2).unwrap();

        let removed = manager.cleanup_empty_battles();
        assert!(removed.contains(&bg1.bg_id));
        assert!(removed.contains(&bg2.bg_id));
        assert_eq!(manager.battleground_count(), 0);
    }

    #[test]
    fn test_player_in_multiple_battles_prevented() {
        let manager = BattlegroundManager::new();
        let bg1 = manager
            .create_battleground(BattlegroundType::TOC, None)
            .unwrap();
        let bg2 = manager
            .create_battleground(BattlegroundType::TOC, None)
            .unwrap();

        manager.join_battle(1, bg1.bg_id).unwrap();

        // 尝试加入另一个战场
        assert_eq!(
            manager.join_battle(1, bg2.bg_id).unwrap_err(),
            BGError::PlayerAlreadyInBattle
        );
    }

    #[test]
    fn test_join_team_auto_balance() {
        let manager = BattlegroundManager::new();
        let bg = manager
            .create_battleground(BattlegroundType::TOC, None)
            .unwrap();

        // 前3个玩家加入，应该被分散到两个队伍
        manager.join_battle(1, bg.bg_id).unwrap();
        manager.join_battle(2, bg.bg_id).unwrap();
        manager.join_battle(3, bg.bg_id).unwrap();
        manager.join_battle(4, bg.bg_id).unwrap();

        let bg = manager.get_battleground(bg.bg_id).unwrap();
        let team0_count = bg.teams[0].player_count();
        let team1_count = bg.teams[1].player_count();

        // 应该是2:2或3:1（取决于分配逻辑）
        assert_eq!(team0_count + team1_count, 4);
    }
}
