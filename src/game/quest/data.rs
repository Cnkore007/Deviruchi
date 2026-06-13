//! 任务系统数据结构

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// 任务类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum QuestType {
    #[default]
    KillHunt, // 击杀狩猎
    CollectItem, // 收集物品
    Deliver,     // 配送
    Escort,      // 护送
    Talk,        // 对话
    Custom,      // 自定义
}

impl fmt::Display for QuestType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuestType::KillHunt => write!(f, "KillHunt"),
            QuestType::CollectItem => write!(f, "CollectItem"),
            QuestType::Deliver => write!(f, "Deliver"),
            QuestType::Escort => write!(f, "Escort"),
            QuestType::Talk => write!(f, "Talk"),
            QuestType::Custom => write!(f, "Custom"),
        }
    }
}

impl QuestType {
    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "killhunt" | "kill" | "hunt" => Some(Self::KillHunt),
            "collectitem" | "collect" => Some(Self::CollectItem),
            "deliver" | "delivery" => Some(Self::Deliver),
            "escort" => Some(Self::Escort),
            "talk" | "dialogue" => Some(Self::Talk),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

/// 目标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ObjectiveType {
    #[default]
    Kill, // 击杀
    Collect,  // 收集
    Deliver,  // 配送
    EscortTo, // 护送至
    TalkTo,   // 对话
}

impl fmt::Display for ObjectiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjectiveType::Kill => write!(f, "Kill"),
            ObjectiveType::Collect => write!(f, "Collect"),
            ObjectiveType::Deliver => write!(f, "Deliver"),
            ObjectiveType::EscortTo => write!(f, "EscortTo"),
            ObjectiveType::TalkTo => write!(f, "TalkTo"),
        }
    }
}

/// 任务目标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestObjective {
    /// 目标ID
    pub id: u32,
    /// 目标类型
    pub objective_type: ObjectiveType,
    /// 目标ID (怪物ID或物品ID)
    pub target_id: u16,
    /// 目标数量
    pub target_count: u32,
    /// 当前进度
    pub current_count: u32,
    /// 目标描述
    pub description: String,
}

impl QuestObjective {
    pub fn new(
        id: u32,
        objective_type: ObjectiveType,
        target_id: u16,
        target_count: u32,
        description: &str,
    ) -> Self {
        Self {
            id,
            objective_type,
            target_id,
            target_count,
            current_count: 0,
            description: description.to_string(),
        }
    }

    /// 检查目标是否完成
    pub fn is_completed(&self) -> bool {
        self.current_count >= self.target_count
    }

    /// 获取进度百分比
    pub fn progress_percent(&self) -> u8 {
        if self.target_count == 0 {
            return 100;
        }
        ((self.current_count as f64 / self.target_count as f64) * 100.0) as u8
    }

    /// 更新进度
    pub fn update_progress(&mut self, count: u32) {
        self.current_count = self.current_count.saturating_add(count);
        if self.current_count > self.target_count {
            self.current_count = self.target_count;
        }
    }

    /// 直接设置进度
    pub fn set_progress(&mut self, count: u32) {
        self.current_count = count.min(self.target_count);
    }
}

/// 任务奖励
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuestRewards {
    /// 经验值
    pub exp: u64,
    /// 职业经验值
    pub job_exp: u64,
    /// 金币
    pub zeny: u32,
    /// 奖励物品ID
    pub item_id: Option<u16>,
    /// 物品数量
    pub item_count: u16,
}

impl QuestRewards {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_exp(mut self, exp: u64) -> Self {
        self.exp = exp;
        self
    }

    pub fn with_job_exp(mut self, job_exp: u64) -> Self {
        self.job_exp = job_exp;
        self
    }

    pub fn with_zeny(mut self, zeny: u32) -> Self {
        self.zeny = zeny;
        self
    }

    pub fn with_item(mut self, item_id: u16, count: u16) -> Self {
        self.item_id = Some(item_id);
        self.item_count = count;
        self
    }
}

/// 任务数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    /// 任务ID
    pub id: u32,
    /// 任务标题
    pub title: String,
    /// 任务描述
    pub description: String,
    /// 任务类型
    pub quest_type: QuestType,
    /// 目标列表
    pub objectives: Vec<QuestObjective>,
    /// 奖励
    pub rewards: QuestRewards,
    /// 时间限制(秒), None = 无限制
    pub time_limit: Option<u32>,
    /// 最低等级要求
    pub level_required: u16,
    /// 所需职业列表
    pub job_required: Vec<u16>,
}

impl Quest {
    pub fn new(id: u32, title: &str, description: &str, quest_type: QuestType) -> Self {
        Self {
            id,
            title: title.to_string(),
            description: description.to_string(),
            quest_type,
            objectives: Vec::new(),
            rewards: QuestRewards::new(),
            time_limit: None,
            level_required: 1,
            job_required: Vec::new(),
        }
    }

    pub fn with_objectives(mut self, objectives: Vec<QuestObjective>) -> Self {
        self.objectives = objectives;
        self
    }

    pub fn with_rewards(mut self, rewards: QuestRewards) -> Self {
        self.rewards = rewards;
        self
    }

    pub fn with_time_limit(mut self, seconds: u32) -> Self {
        self.time_limit = Some(seconds);
        self
    }

    pub fn with_level_required(mut self, level: u16) -> Self {
        self.level_required = level;
        self
    }

    pub fn with_job_required(mut self, jobs: Vec<u16>) -> Self {
        self.job_required = jobs;
        self
    }

    /// 检查所有目标是否完成
    pub fn is_completed(&self) -> bool {
        self.objectives.iter().all(|o| o.is_completed())
    }

    /// 获取整体进度百分比
    pub fn progress_percent(&self) -> u8 {
        if self.objectives.is_empty() {
            return 100;
        }
        let total: u32 = self.objectives.iter().map(|o| o.target_count).sum();
        if total == 0 {
            return 100;
        }
        let current: u32 = self.objectives.iter().map(|o| o.current_count).sum();
        ((current as f64 / total as f64) * 100.0) as u8
    }
}

/// 任务进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestProgress {
    /// 任务ID
    pub quest_id: u32,
    /// 目标列表(克隆)
    pub objectives: Vec<QuestObjective>,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 过期时间
    pub expires_at: Option<DateTime<Utc>>,
}

impl QuestProgress {
    pub fn new(quest_id: u32, objectives: Vec<QuestObjective>, time_limit: Option<u32>) -> Self {
        let started_at = Utc::now();
        let expires_at =
            time_limit.map(|seconds| started_at + chrono::Duration::seconds(seconds as i64));

        Self {
            quest_id,
            objectives,
            started_at,
            expires_at,
        }
    }

    /// 检查是否已过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            Utc::now() > expires
        } else {
            false
        }
    }

    /// 检查所有目标是否完成
    pub fn is_completed(&self) -> bool {
        self.objectives.iter().all(|o| o.is_completed())
    }

    /// 获取剩余时间(秒)
    pub fn time_remaining(&self) -> Option<i64> {
        self.expires_at
            .map(|expires| (expires - Utc::now()).num_seconds().max(0))
    }

    /// 更新目标进度
    pub fn update_objective(&mut self, objective_id: u32, count: u32) {
        if let Some(obj) = self.objectives.iter_mut().find(|o| o.id == objective_id) {
            obj.update_progress(count);
        }
    }

    /// 按类型和目标ID更新进度
    pub fn update_by_target(&mut self, objective_type: ObjectiveType, target_id: u16, count: u32) {
        if let Some(obj) = self.objectives.iter_mut().find(|o| {
            o.objective_type == objective_type && (o.target_id == 0 || o.target_id == target_id)
        }) {
            obj.update_progress(count);
        }
    }
}

/// 玩家任务数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerQuestData {
    /// 进行中的任务 (quest_id -> QuestProgress)
    pub active_quests: HashMap<u32, QuestProgress>,
    /// 已完成任务 (quest_id)
    pub completed_quests: Vec<u32>,
    /// 每日任务及完成时间 (quest_id -> completion_time)
    pub daily_quests: HashMap<u32, DateTime<Utc>>,
}

impl PlayerQuestData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_quest(&self, quest_id: u32) -> bool {
        self.active_quests.contains_key(&quest_id)
    }

    pub fn is_completed(&self, quest_id: u32) -> bool {
        self.completed_quests.contains(&quest_id)
    }

    pub fn is_daily_completed_today(&self, quest_id: u32) -> bool {
        if let Some(completed_at) = self.daily_quests.get(&quest_id) {
            let today = Utc::now().date_naive();
            completed_at.date_naive() == today
        } else {
            false
        }
    }

    pub fn add_quest(&mut self, progress: QuestProgress) {
        self.active_quests.insert(progress.quest_id, progress);
    }

    pub fn remove_quest(&mut self, quest_id: u32) -> Option<QuestProgress> {
        self.active_quests.remove(&quest_id)
    }

    pub fn complete_quest(&mut self, quest_id: u32) -> bool {
        if self.active_quests.remove(&quest_id).is_some() {
            if !self.completed_quests.contains(&quest_id) {
                self.completed_quests.push(quest_id);
            }
            true
        } else {
            false
        }
    }

    pub fn mark_daily_complete(&mut self, quest_id: u32) {
        self.daily_quests.insert(quest_id, Utc::now());
    }
}

/// 任务数据库
#[derive(Debug, Clone, Default)]
pub struct QuestDatabase {
    /// 任务数据 (quest_id -> Quest)
    quests: HashMap<u32, Quest>,
    /// 按类型索引 (quest_type -> Vec<quest_id>)
    by_type: HashMap<QuestType, Vec<u32>>,
    /// 每日任务 (quest_id)
    daily_quests: Vec<u32>,
}

impl QuestDatabase {
    pub fn new() -> Self {
        let mut db = Self::default();

        // 尝试从 YAML 加载
        let yaml_paths = ["db/quest_db.yml"];
        for path in &yaml_paths {
            if std::path::Path::new(path).exists() {
                match super::yaml_loader::load_quest_db(path) {
                    Ok(quests) if !quests.is_empty() => {
                        let count = quests.len();
                        for (_, quest) in quests {
                            db.add(quest);
                        }
                        tracing::info!("从 {} 加载了 {} 个任务", path, count);
                        return db;
                    }
                    Ok(_) => {
                        tracing::warn!("{} 解析结果为空", path);
                    }
                    Err(e) => {
                        tracing::warn!("加载 {} 失败: {}", path, e);
                    }
                }
            }
        }

        // 回退到硬编码
        tracing::info!("使用硬编码任务数据");
        db.load_default_quests();
        db
    }

    /// 添加任务
    pub fn add(&mut self, quest: Quest) {
        let id = quest.id;
        self.quests.insert(id, quest.clone());

        self.by_type.entry(quest.quest_type).or_default().push(id);

        // 如果任务有时间限制，标记为每日任务
        if quest.time_limit.is_some() {
            self.daily_quests.push(id);
        }
    }

    /// 获取任务
    pub fn get(&self, id: u32) -> Option<&Quest> {
        self.quests.get(&id)
    }

    /// 获取所有任务
    pub fn all(&self) -> Vec<&Quest> {
        self.quests.values().collect()
    }

    /// 按类型获取任务
    pub fn get_by_type(&self, quest_type: QuestType) -> Vec<&Quest> {
        self.by_type
            .get(&quest_type)
            .map(|ids| ids.iter().filter_map(|id| self.quests.get(id)).collect())
            .unwrap_or_default()
    }

    /// 获取每日任务
    pub fn get_daily_quests(&self) -> Vec<&Quest> {
        self.daily_quests
            .iter()
            .filter_map(|id| self.quests.get(id))
            .collect()
    }

    /// 检查是否为每日任务
    pub fn is_daily(&self, quest_id: u32) -> bool {
        self.daily_quests.contains(&quest_id)
    }

    /// 加载默认任务
    pub fn load_default_quests(&mut self) {
        // 新手任务 - 击杀史汀
        self.add(
            Quest::new(1001, "First Hunt", "Defeat 10 Shrooms", QuestType::KillHunt)
                .with_objectives(vec![QuestObjective::new(
                    1,
                    ObjectiveType::Kill,
                    1001,
                    10,
                    "Defeat Shrooms",
                )])
                .with_rewards(QuestRewards::new().with_exp(100).with_zeny(500))
                .with_level_required(1),
        );

        // 收集任务
        self.add(
            Quest::new(1002, "Gatherer", "Collect 20 Fluff", QuestType::CollectItem)
                .with_objectives(vec![QuestObjective::new(
                    1,
                    ObjectiveType::Collect,
                    501,
                    20,
                    "Collect Fluff",
                )])
                .with_rewards(QuestRewards::new().with_exp(200).with_item(502, 5))
                .with_level_required(5),
        );

        // 对话任务
        self.add(
            Quest::new(
                1003,
                "Talk to Elder",
                "Speak with the village elder",
                QuestType::Talk,
            )
            .with_objectives(vec![QuestObjective::new(
                1,
                ObjectiveType::TalkTo,
                100,
                1,
                "Talk to Elder Thomas",
            )])
            .with_rewards(QuestRewards::new().with_exp(50).with_zeny(100))
            .with_level_required(1),
        );

        // 每日任务 - 狩猎
        self.add(
            Quest::new(
                2001,
                "Daily Hunt",
                "Defeat 50 monsters",
                QuestType::KillHunt,
            )
            .with_objectives(vec![QuestObjective::new(
                1,
                ObjectiveType::Kill,
                0,
                50,
                "Defeat any monsters",
            )])
            .with_rewards(QuestRewards::new().with_exp(500).with_item(501, 10))
            .with_time_limit(86400) // 24小时
            .with_level_required(10),
        );

        // 每日任务 - 收集
        self.add(
            Quest::new(
                2002,
                "Daily Gathering",
                "Collect 30 materials",
                QuestType::CollectItem,
            )
            .with_objectives(vec![QuestObjective::new(
                1,
                ObjectiveType::Collect,
                0,
                30,
                "Collect any materials",
            )])
            .with_rewards(QuestRewards::new().with_exp(300).with_zeny(1000))
            .with_time_limit(86400)
            .with_level_required(5),
        );
    }
}
