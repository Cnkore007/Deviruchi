//! 任务管理器模块

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use super::data::{
    ObjectiveType, PlayerQuestData, Quest, QuestDatabase, QuestProgress, QuestRewards, QuestType,
};

/// 任务错误类型
#[derive(Debug, Error, Clone)]
pub enum QuestError {
    #[error("Quest not found: {0}")]
    NotFound(u32),

    #[error("Player not found: {0}")]
    PlayerNotFound(u32),

    #[error("Quest already active: {0}")]
    AlreadyActive(u32),

    #[error("Quest not active: {0}")]
    NotActive(u32),

    #[error("Quest already completed: {0}")]
    AlreadyCompleted(u32),

    #[error("Quest not completed: {0}")]
    NotCompleted(u32),

    #[error("Level requirement not met: need level {0}, have {1}")]
    LevelRequirementNotMet(u16, u16),

    #[error("Quest expired: {0}")]
    QuestExpired(u32),

    #[error("Daily quest already completed today: {0}")]
    DailyQuestCompleted(u32),

    #[error("Max active quests reached: {0}")]
    MaxActiveQuests(usize),
}

/// 任务管理器
pub struct QuestManager {
    /// 任务数据库
    database: Arc<QuestDatabase>,
    /// 玩家任务数据 (char_id -> PlayerQuestData)
    player_quests: RwLock<HashMap<u32, PlayerQuestData>>,
    /// 最大同时进行的任务数
    max_active_quests: usize,
}

impl Default for QuestManager {
    fn default() -> Self {
        Self::new()
    }
}

impl QuestManager {
    /// 创建任务管理器
    pub fn new() -> Self {
        let mut database = QuestDatabase::new();
        database.load_default_quests();

        Self {
            database: Arc::new(database),
            player_quests: RwLock::new(HashMap::new()),
            max_active_quests: 30,
        }
    }

    /// 使用自定义数据库创建
    pub fn with_database(database: QuestDatabase) -> Self {
        Self {
            database: Arc::new(database),
            player_quests: RwLock::new(HashMap::new()),
            max_active_quests: 30,
        }
    }

    /// 设置最大活动任务数
    pub fn with_max_quests(mut self, max: usize) -> Self {
        self.max_active_quests = max;
        self
    }

    /// 获取数据库引用
    pub fn database(&self) -> &Arc<QuestDatabase> {
        &self.database
    }

    /// 开始任务
    pub fn start_quest(&self, char_id: u32, quest_id: u32) -> Result<QuestProgress, QuestError> {
        let mut player_quests = self.player_quests.write();
        let player_data = player_quests.entry(char_id).or_default();

        // 检查任务是否存在
        let quest = self
            .database
            .get(quest_id)
            .ok_or(QuestError::NotFound(quest_id))?;

        // 检查是否已激活
        if player_data.has_quest(quest_id) {
            return Err(QuestError::AlreadyActive(quest_id));
        }

        // 检查是否已完成（非每日任务）
        if !self.database.is_daily(quest_id) && player_data.is_completed(quest_id) {
            return Err(QuestError::AlreadyCompleted(quest_id));
        }

        // 检查每日任务是否已完成
        if self.database.is_daily(quest_id) && player_data.is_daily_completed_today(quest_id) {
            return Err(QuestError::DailyQuestCompleted(quest_id));
        }

        // 检查最大任务数
        if player_data.active_quests.len() >= self.max_active_quests {
            return Err(QuestError::MaxActiveQuests(self.max_active_quests));
        }

        // 创建任务进度（克隆目标）
        let objectives = quest.objectives.iter().cloned().collect();
        let progress = QuestProgress::new(quest_id, objectives, quest.time_limit);

        player_data.add_quest(progress.clone());
        Ok(progress)
    }

    /// 开始任务（带等级检查）
    pub fn start_quest_with_level_check(
        &self,
        char_id: u32,
        quest_id: u32,
        player_level: u16,
    ) -> Result<QuestProgress, QuestError> {
        // 检查等级要求
        if let Some(quest) = self.database.get(quest_id)
            && player_level < quest.level_required
        {
            return Err(QuestError::LevelRequirementNotMet(
                quest.level_required,
                player_level,
            ));
        }

        self.start_quest(char_id, quest_id)
    }

    /// 更新进度
    pub fn update_progress(
        &self,
        char_id: u32,
        objective_type: ObjectiveType,
        target_id: u16,
        count: u32,
    ) {
        let mut player_quests = self.player_quests.write();

        if let Some(player_data) = player_quests.get_mut(&char_id) {
            for progress in player_data.active_quests.values_mut() {
                progress.update_by_target(objective_type, target_id, count);
            }
        }
    }

    /// 更新特定任务的进度
    pub fn update_quest_progress(
        &self,
        char_id: u32,
        quest_id: u32,
        objective_id: u32,
        count: u32,
    ) -> bool {
        let mut player_quests = self.player_quests.write();

        if let Some(player_data) = player_quests.get_mut(&char_id)
            && let Some(progress) = player_data.active_quests.get_mut(&quest_id)
        {
            progress.update_objective(objective_id, count);
            return true;
        }
        false
    }

    /// 完成任务
    pub fn complete_quest(&self, char_id: u32, quest_id: u32) -> Result<QuestRewards, QuestError> {
        let mut player_quests = self.player_quests.write();
        let player_data = player_quests
            .get_mut(&char_id)
            .ok_or(QuestError::PlayerNotFound(char_id))?;

        // 检查任务是否激活
        let progress = player_data
            .active_quests
            .get(&quest_id)
            .ok_or(QuestError::NotActive(quest_id))?;

        // 检查是否过期
        if progress.is_expired() {
            player_data.remove_quest(quest_id);
            return Err(QuestError::QuestExpired(quest_id));
        }

        // 检查是否所有目标都完成
        if !progress.is_completed() {
            return Err(QuestError::NotCompleted(quest_id));
        }

        // 获取奖励
        let quest = self
            .database
            .get(quest_id)
            .ok_or(QuestError::NotFound(quest_id))?;

        // 移除任务并标记完成
        player_data.remove_quest(quest_id);

        // 标记每日任务完成
        if self.database.is_daily(quest_id) {
            player_data.mark_daily_complete(quest_id);
        } else {
            player_data.complete_quest(quest_id);
        }

        Ok(quest.rewards.clone())
    }

    /// 放弃任务
    pub fn abandon_quest(&self, char_id: u32, quest_id: u32) -> Result<(), QuestError> {
        let mut player_quests = self.player_quests.write();
        let player_data = player_quests
            .get_mut(&char_id)
            .ok_or(QuestError::PlayerNotFound(char_id))?;

        if player_data.remove_quest(quest_id).is_none() {
            return Err(QuestError::NotActive(quest_id));
        }

        Ok(())
    }

    /// 获取活动任务列表
    pub fn get_active_quests(&self, char_id: u32) -> Vec<Quest> {
        let player_quests = self.player_quests.read();

        if let Some(player_data) = player_quests.get(&char_id) {
            player_data
                .active_quests
                .values()
                .filter(|p| !p.is_expired())
                .filter_map(|p| self.database.get(p.quest_id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 获取活动任务的完整进度信息
    pub fn get_active_quests_with_progress(&self, char_id: u32) -> Vec<(Quest, QuestProgress)> {
        let player_quests = self.player_quests.read();

        if let Some(player_data) = player_quests.get(&char_id) {
            player_data
                .active_quests
                .values()
                .filter(|p| !p.is_expired())
                .filter_map(|p| {
                    self.database
                        .get(p.quest_id)
                        .map(|q| (q.clone(), p.clone()))
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 获取已完成任务列表
    pub fn get_completed_quests(&self, char_id: u32) -> Vec<Quest> {
        let player_quests = self.player_quests.read();

        if let Some(player_data) = player_quests.get(&char_id) {
            player_data
                .completed_quests
                .iter()
                .filter_map(|id| self.database.get(*id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 获取可接任务列表
    pub fn get_available_quests(&self, char_id: u32) -> Vec<Quest> {
        let player_quests = self.player_quests.read();
        let player_data = player_quests.get(&char_id);

        self.database
            .all()
            .into_iter()
            .filter(|quest| {
                // 过滤已激活的任务
                if let Some(p) = player_data
                    && p.has_quest(quest.id)
                {
                    return false;
                }

                // 过滤已完成的任务（非每日任务）
                if let Some(p) = player_data {
                    if !self.database.is_daily(quest.id) && p.is_completed(quest.id) {
                        return false;
                    }
                    // 过滤每日任务（今日已完成）
                    if self.database.is_daily(quest.id) && p.is_daily_completed_today(quest.id) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect()
    }

    /// 获取特定类型的可用任务
    pub fn get_available_quests_by_type(&self, char_id: u32, quest_type: QuestType) -> Vec<Quest> {
        self.get_available_quests(char_id)
            .into_iter()
            .filter(|q| q.quest_type == quest_type)
            .collect()
    }

    /// 获取每日任务
    pub fn get_daily_quests(&self, _char_id: u32) -> Vec<Quest> {
        self.database
            .get_daily_quests()
            .into_iter()
            .filter(|q| q.time_limit.is_some())
            .cloned()
            .collect()
    }

    /// 获取玩家任务数据
    pub fn get_player_data(&self, char_id: u32) -> Option<PlayerQuestData> {
        self.player_quests.read().get(&char_id).cloned()
    }

    /// 检查任务进度（返回是否全部完成）
    pub fn check_quest_completion(&self, char_id: u32, quest_id: u32) -> bool {
        let player_quests = self.player_quests.read();

        if let Some(player_data) = player_quests.get(&char_id)
            && let Some(progress) = player_data.active_quests.get(&quest_id)
        {
            return progress.is_completed();
        }
        false
    }

    /// 获取任务进度信息
    pub fn get_quest_progress(&self, char_id: u32, quest_id: u32) -> Option<QuestProgress> {
        self.player_quests
            .read()
            .get(&char_id)
            .and_then(|p| p.active_quests.get(&quest_id).cloned())
    }

    /// 清理过期任务
    pub fn cleanup_expired_quests(&self) -> usize {
        let mut count = 0;
        let mut player_quests = self.player_quests.write();

        for player_data in player_quests.values_mut() {
            let expired: Vec<u32> = player_data
                .active_quests
                .values()
                .filter(|p| p.is_expired())
                .map(|p| p.quest_id)
                .collect();

            for quest_id in expired {
                player_data.active_quests.remove(&quest_id);
                count += 1;
            }
        }

        count
    }

    /// 获取任务详情
    pub fn get_quest(&self, quest_id: u32) -> Option<Quest> {
        self.database.get(quest_id).cloned()
    }

    /// 获取玩家的过期任务
    pub fn get_expired_quests(&self, char_id: u32) -> Vec<Quest> {
        let player_quests = self.player_quests.read();

        if let Some(player_data) = player_quests.get(&char_id) {
            player_data
                .active_quests
                .values()
                .filter(|p| p.is_expired())
                .filter_map(|p| self.database.get(p.quest_id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_quest() {
        let manager = QuestManager::new();

        // 开始任务
        let result = manager.start_quest(1, 1001);
        assert!(result.is_ok());

        // 检查任务是否在活动列表中
        let active = manager.get_active_quests(1);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, 1001);
    }

    #[test]
    fn test_update_progress() {
        let manager = QuestManager::new();

        // 开始任务
        manager.start_quest(1, 1001).unwrap();

        // 更新进度
        manager.update_progress(1, ObjectiveType::Kill, 1001, 10);

        // 检查是否完成
        assert!(manager.check_quest_completion(1, 1001));
    }

    #[test]
    fn test_complete_quest() {
        let manager = QuestManager::new();

        // 开始任务并完成
        manager.start_quest(1, 1001).unwrap();
        manager.update_progress(1, ObjectiveType::Kill, 1001, 10);

        // 完成任务
        let rewards = manager.complete_quest(1, 1001);
        assert!(rewards.is_ok());
        assert_eq!(rewards.unwrap().exp, 100);
    }

    #[test]
    fn test_abandon_quest() {
        let manager = QuestManager::new();

        // 开始任务
        manager.start_quest(1, 1001).unwrap();

        // 放弃任务
        let result = manager.abandon_quest(1, 1001);
        assert!(result.is_ok());

        // 检查任务是否不在活动列表中
        let active = manager.get_active_quests(1);
        assert!(active.is_empty());
    }

    #[test]
    fn test_daily_quest_reset() {
        let manager = QuestManager::new();

        // 开始每日任务
        manager.start_quest(1, 2001).unwrap();
        manager.update_progress(1, ObjectiveType::Kill, 0, 50);
        manager.complete_quest(1, 2001).unwrap();

        // 再次尝试开始（应该失败）
        let result = manager.start_quest(1, 2001);
        assert!(matches!(result, Err(QuestError::DailyQuestCompleted(_))));
    }
}
