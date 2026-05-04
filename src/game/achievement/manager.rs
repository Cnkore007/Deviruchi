//! 成就管理器模块

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use super::data::{
    Achievement, AchievementCategory, AchievementCondition, AchievementDatabase, AchievementReward,
    PlayerAchievementProgress,
};

/// 成就错误类型
#[derive(Debug, Error, Clone)]
pub enum AchievementError {
    #[error("Achievement not found: {0}")]
    NotFound(u32),

    #[error("Player not found: {0}")]
    PlayerNotFound(u32),

    #[error("Achievement already completed: {0}")]
    AlreadyCompleted(u32),

    #[error("Achievement not completed: {0}")]
    NotCompleted(u32),

    #[error("Reward already claimed: {0}")]
    RewardAlreadyClaimed(u32),

    #[error("Prerequisites not met for achievement: {0}")]
    PrerequisitesNotMet(u32),

    #[error("Progress not found: {0}")]
    ProgressNotFound(u32),
}

/// 成就管理器
pub struct AchievementManager {
    /// 成就数据库
    database: Arc<AchievementDatabase>,
    /// 玩家成就进度 (char_id -> PlayerAchievementProgress)
    player_progress: RwLock<HashMap<u32, PlayerAchievementProgress>>,
}

impl Default for AchievementManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AchievementManager {
    /// 创建成就管理器
    pub fn new() -> Self {
        let mut database = AchievementDatabase::new();
        database.load_default_achievements();

        Self {
            database: Arc::new(database),
            player_progress: RwLock::new(HashMap::new()),
        }
    }

    /// 使用自定义数据库创建
    pub fn with_database(database: AchievementDatabase) -> Self {
        Self {
            database: Arc::new(database),
            player_progress: RwLock::new(HashMap::new()),
        }
    }

    /// 获取数据库引用
    pub fn database(&self) -> &Arc<AchievementDatabase> {
        &self.database
    }

    /// 检查条件并更新进度
    ///
    /// 当玩家完成某个动作时调用此方法检查相关成就进度
    pub fn check_progress(
        &self,
        char_id: u32,
        condition: &AchievementCondition,
        value: u32,
    ) -> Vec<Achievement> {
        let mut completed_achievements = Vec::new();
        let mut progress = self.player_progress.write();

        // 确保玩家进度存在
        let player_data = progress.entry(char_id).or_default();

        for achievement in self.database.all() {
            // 跳过已完成的成就
            if player_data.is_completed(achievement.id) {
                continue;
            }

            // 检查条件是否匹配
            if self.matches_condition(&achievement.condition, condition, value) {
                let current = player_data.get_progress(achievement.id);
                let new_progress = current.saturating_add(1);

                // 检查前置成就是否满足
                if !self.check_prerequisites(player_data, &achievement.pre_achievements) {
                    continue;
                }

                player_data.update_progress(achievement.id, new_progress);

                // 检查是否达成目标
                if new_progress >= achievement.target {
                    player_data.complete(achievement.id);
                    completed_achievements.push(achievement.clone());
                }
            }
        }

        completed_achievements
    }

    /// 直接设置进度值（用于某些需要直接设置而非递增的情况）
    pub fn set_progress(
        &self,
        char_id: u32,
        achievement_id: u32,
        value: u32,
    ) -> Result<Option<Achievement>, AchievementError> {
        let mut progress = self.player_progress.write();
        let player_data = progress.entry(char_id).or_default();

        let achievement = self
            .database
            .get(achievement_id)
            .ok_or(AchievementError::NotFound(achievement_id))?;

        if player_data.is_completed(achievement_id) {
            return Err(AchievementError::AlreadyCompleted(achievement_id));
        }

        player_data.update_progress(achievement_id, value);

        if value >= achievement.target {
            player_data.complete(achievement_id);
            Ok(Some(achievement.clone()))
        } else {
            Ok(None)
        }
    }

    /// 完成成就
    pub fn complete_achievement(
        &self,
        char_id: u32,
        achievement_id: u32,
    ) -> Result<Achievement, AchievementError> {
        let mut progress = self.player_progress.write();
        let player_data = progress.entry(char_id).or_default();

        let achievement = self
            .database
            .get(achievement_id)
            .ok_or(AchievementError::NotFound(achievement_id))?;

        if player_data.is_completed(achievement_id) {
            return Err(AchievementError::AlreadyCompleted(achievement_id));
        }

        // 检查前置成就
        if !self.check_prerequisites(player_data, &achievement.pre_achievements) {
            return Err(AchievementError::PrerequisitesNotMet(achievement_id));
        }

        player_data.complete(achievement_id);
        Ok(achievement.clone())
    }

    /// 领取奖励
    pub fn claim_reward(
        &self,
        char_id: u32,
        achievement_id: u32,
    ) -> Result<AchievementReward, AchievementError> {
        let mut progress = self.player_progress.write();
        let player_data = progress.entry(char_id).or_default();

        let achievement = self
            .database
            .get(achievement_id)
            .ok_or(AchievementError::NotFound(achievement_id))?;

        if !player_data.is_completed(achievement_id) {
            return Err(AchievementError::NotCompleted(achievement_id));
        }

        if player_data.is_reward_claimed(achievement_id) {
            return Err(AchievementError::RewardAlreadyClaimed(achievement_id));
        }

        player_data.claim_reward(achievement_id);
        Ok(achievement.reward.clone())
    }

    /// 获取玩家进度
    pub fn get_progress(&self, char_id: u32) -> PlayerAchievementProgress {
        self.player_progress
            .read()
            .get(&char_id)
            .cloned()
            .unwrap_or_default()
    }

    /// 检查成就是否已完成
    pub fn is_completed(&self, char_id: u32, achievement_id: u32) -> bool {
        self.player_progress
            .read()
            .get(&char_id)
            .map(|p| p.is_completed(achievement_id))
            .unwrap_or(false)
    }

    /// 检查奖励是否已领取
    pub fn is_reward_claimed(&self, char_id: u32, achievement_id: u32) -> bool {
        self.player_progress
            .read()
            .get(&char_id)
            .map(|p| p.is_reward_claimed(achievement_id))
            .unwrap_or(false)
    }

    /// 获取玩家已完成的成就列表
    pub fn get_completed_achievements(&self, char_id: u32) -> Vec<u32> {
        self.player_progress
            .read()
            .get(&char_id)
            .map(|p| p.completed.clone())
            .unwrap_or_default()
    }

    /// 获取玩家可完成的成就列表
    pub fn get_available_achievements(&self, char_id: u32) -> Vec<Achievement> {
        let progress = self.player_progress.read();
        let player_data = progress.get(&char_id);

        self.database
            .all()
            .into_iter()
            .filter(|a| {
                // 未完成
                if let Some(p) = player_data {
                    if p.is_completed(a.id) {
                        return false;
                    }
                    // 检查前置成就
                    if !self.check_prerequisites(p, &a.pre_achievements) {
                        return false;
                    }
                } else {
                    // 检查前置成就
                    if !a.pre_achievements.is_empty() {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    /// 按分类获取玩家成就进度
    pub fn get_progress_by_category(
        &self,
        char_id: u32,
        category: AchievementCategory,
    ) -> Vec<(Achievement, u32, bool)> {
        let progress = self.player_progress.read();
        let player_data = progress.get(&char_id);

        self.database
            .get_by_category(category)
            .into_iter()
            .map(|a| {
                let (current, completed) = if let Some(p) = player_data {
                    (p.get_progress(a.id), p.is_completed(a.id))
                } else {
                    (0, false)
                };
                (a.clone(), current, completed)
            })
            .collect()
    }

    /// 检查条件是否匹配
    fn matches_condition(
        &self,
        achievement_cond: &AchievementCondition,
        event_cond: &AchievementCondition,
        _value: u32,
    ) -> bool {
        match (achievement_cond, event_cond) {
            (AchievementCondition::KillAnyMonster(_), AchievementCondition::KillAnyMonster(_)) => {
                true
            }
            (
                AchievementCondition::KillMonster(mob_id),
                AchievementCondition::KillMonster(event_mob_id),
            ) if mob_id == event_mob_id => true,
            (AchievementCondition::ReachLevel(_), AchievementCondition::ReachLevel(_)) => true,
            (
                AchievementCondition::CollectItem(item_id, _),
                AchievementCondition::CollectItem(event_item_id, _),
            ) if *item_id == 0 || *item_id == *event_item_id => true,
            (AchievementCondition::CompleteQuest(_), AchievementCondition::CompleteQuest(_)) => {
                true
            }
            (AchievementCondition::WinDuels(_), AchievementCondition::WinDuels(_)) => true,
            (AchievementCondition::ExploreMaps(_), AchievementCondition::ExploreMaps(_)) => true,
            (AchievementCondition::TradeItems(_), AchievementCondition::TradeItems(_)) => true,
            (AchievementCondition::Custom(_), AchievementCondition::Custom(event)) => {
                achievement_cond.to_string().contains(event)
            }
            _ => false,
        }
    }

    /// 检查前置成就是否满足
    fn check_prerequisites(
        &self,
        player_data: &PlayerAchievementProgress,
        prerequisites: &[u32],
    ) -> bool {
        prerequisites
            .iter()
            .all(|&prereq_id| player_data.is_completed(prereq_id))
    }

    /// 获取成就详情
    pub fn get_achievement(&self, achievement_id: u32) -> Option<Achievement> {
        self.database.get(achievement_id).cloned()
    }

    /// 添加自定义成就
    pub fn add_achievement(&self, _achievement: Achievement) {
        // 注意: 由于Arc限制，需要通过内部机制添加
        // 这里提供一种延迟更新的机制
        tracing::warn!("Adding custom achievements requires manager recreation");
    }

    /// 移除玩家进度（用于测试或重置）
    #[cfg(test)]
    pub fn remove_player_progress(&self, char_id: u32) {
        self.player_progress.write().remove(&char_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_achievement_completion() {
        let manager = AchievementManager::new();

        // 测试击杀怪物成就
        let completed = manager.check_progress(1, &AchievementCondition::KillAnyMonster(0), 1);

        // 应该完成"First Blood"成就 (id=1)
        assert!(completed.iter().any(|a| a.id == 1));
    }

    #[test]
    fn test_progress_tracking() {
        let manager = AchievementManager::new();

        // 初始进度为空
        let progress = manager.get_progress(999);
        assert!(progress.completed.is_empty());

        // 更新进度
        manager.set_progress(999, 1, 1).ok();
        assert!(manager.is_completed(999, 1));
    }

    #[test]
    fn test_reward_claiming() {
        let manager = AchievementManager::new();

        // 先完成成就
        manager.complete_achievement(1, 1).unwrap();

        // 领取奖励
        let reward = manager.claim_reward(1, 1).unwrap();
        assert_eq!(reward.cash_points, 100);
        assert!(manager.is_reward_claimed(1, 1));
    }
}
