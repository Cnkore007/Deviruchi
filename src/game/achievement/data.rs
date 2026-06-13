//! 成就系统数据结构

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 成就分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AchievementCategory {
    #[default]
    Battle, // 战斗相关
    Adventure,   // 冒险相关
    Social,      // 社交相关
    Collection,  // 收集相关
    MonsterHunt, // 怪物狩猎
    LevelUp,     // 升级相关
    ItemTrade,   // 物品交易
    Special,     // 特殊成就
}

impl std::fmt::Display for AchievementCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AchievementCategory::Battle => write!(f, "Battle"),
            AchievementCategory::Adventure => write!(f, "Adventure"),
            AchievementCategory::Social => write!(f, "Social"),
            AchievementCategory::Collection => write!(f, "Collection"),
            AchievementCategory::MonsterHunt => write!(f, "MonsterHunt"),
            AchievementCategory::LevelUp => write!(f, "LevelUp"),
            AchievementCategory::ItemTrade => write!(f, "ItemTrade"),
            AchievementCategory::Special => write!(f, "Special"),
        }
    }
}

impl AchievementCategory {
    /// 从字符串解析分类
    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "battle" => Some(Self::Battle),
            "adventure" => Some(Self::Adventure),
            "social" => Some(Self::Social),
            "collection" => Some(Self::Collection),
            "monsterhunt" | "monster_hunt" => Some(Self::MonsterHunt),
            "levelup" | "level_up" => Some(Self::LevelUp),
            "itemtrade" | "item_trade" => Some(Self::ItemTrade),
            "special" => Some(Self::Special),
            _ => None,
        }
    }
}

/// 成就完成条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AchievementCondition {
    /// 击杀特定怪物
    KillMonster(u16), // mob_id
    /// 击杀任意怪物
    KillAnyMonster(u32), // count
    /// 达到指定等级
    ReachLevel(u16),
    /// 收集物品
    CollectItem(u16, u32), // item_id, count
    /// 完成特定任务
    CompleteQuest(u32),
    /// 交易物品
    TradeItems(u32),
    /// 赢得决斗
    WinDuels(u32),
    /// 探索地图
    ExploreMaps(u32),
    /// 自定义条件
    Custom(String),
}

impl Default for AchievementCondition {
    fn default() -> Self {
        Self::KillAnyMonster(0)
    }
}

impl std::fmt::Display for AchievementCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AchievementCondition::KillMonster(id) => write!(f, "KillMonster({})", id),
            AchievementCondition::KillAnyMonster(count) => write!(f, "KillAnyMonster({})", count),
            AchievementCondition::ReachLevel(level) => write!(f, "ReachLevel({})", level),
            AchievementCondition::CollectItem(item_id, count) => {
                write!(f, "CollectItem({}, {})", item_id, count)
            }
            AchievementCondition::CompleteQuest(quest_id) => {
                write!(f, "CompleteQuest({})", quest_id)
            }
            AchievementCondition::TradeItems(count) => write!(f, "TradeItems({})", count),
            AchievementCondition::WinDuels(count) => write!(f, "WinDuels({})", count),
            AchievementCondition::ExploreMaps(count) => write!(f, "ExploreMaps({})", count),
            AchievementCondition::Custom(s) => write!(f, "Custom({})", s),
        }
    }
}

/// 成就奖励
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AchievementReward {
    /// 点卡/现金点数
    pub cash_points: u32,
    /// 奖励物品ID
    pub item_id: Option<u16>,
    /// 物品数量
    pub item_count: u16,
    /// 称号
    pub title: Option<String>,
}

impl AchievementReward {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cash(mut self, cash: u32) -> Self {
        self.cash_points = cash;
        self
    }

    pub fn with_item(mut self, item_id: u16, count: u16) -> Self {
        self.item_id = Some(item_id);
        self.item_count = count;
        self
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }
}

/// 成就数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    /// 成就ID
    pub id: u32,
    /// 成就名称
    pub name: String,
    /// 成就描述
    pub description: String,
    /// 分类
    pub category: AchievementCategory,
    /// 完成条件
    pub condition: AchievementCondition,
    /// 目标数量
    pub target: u32,
    /// 奖励
    pub reward: AchievementReward,
    /// 前置成就列表
    pub pre_achievements: Vec<u32>,
}

impl Achievement {
    pub fn new(id: u32, name: &str, description: &str, category: AchievementCategory) -> Self {
        Self {
            id,
            name: name.to_string(),
            description: description.to_string(),
            category,
            condition: AchievementCondition::KillAnyMonster(0),
            target: 1,
            reward: AchievementReward::new(),
            pre_achievements: Vec::new(),
        }
    }

    pub fn with_condition(mut self, condition: AchievementCondition) -> Self {
        self.condition = condition;
        self
    }

    pub fn with_target(mut self, target: u32) -> Self {
        self.target = target;
        self
    }

    pub fn with_reward(mut self, reward: AchievementReward) -> Self {
        self.reward = reward;
        self
    }

    pub fn with_prereq(mut self, prereq: Vec<u32>) -> Self {
        self.pre_achievements = prereq;
        self
    }
}

/// 玩家成就进度
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerAchievementProgress {
    /// 已完成的成就
    pub completed: Vec<u32>,
    /// 进行中的成就 (achievement_id -> current_count)
    pub in_progress: HashMap<u32, u32>,
    /// 已领取奖励的成就
    pub rewards_claimed: Vec<u32>,
}

impl PlayerAchievementProgress {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_completed(&self, achievement_id: u32) -> bool {
        self.completed.contains(&achievement_id)
    }

    pub fn is_reward_claimed(&self, achievement_id: u32) -> bool {
        self.rewards_claimed.contains(&achievement_id)
    }

    pub fn get_progress(&self, achievement_id: u32) -> u32 {
        self.in_progress.get(&achievement_id).copied().unwrap_or(0)
    }

    pub fn update_progress(&mut self, achievement_id: u32, count: u32) {
        self.in_progress.insert(achievement_id, count);
    }

    pub fn complete(&mut self, achievement_id: u32) {
        if !self.completed.contains(&achievement_id) {
            self.completed.push(achievement_id);
        }
        self.in_progress.remove(&achievement_id);
    }

    pub fn claim_reward(&mut self, achievement_id: u32) {
        if !self.rewards_claimed.contains(&achievement_id) {
            self.rewards_claimed.push(achievement_id);
        }
    }
}

/// 成就数据库
#[derive(Debug, Clone, Default)]
pub struct AchievementDatabase {
    /// 成就数据 (achievement_id -> Achievement)
    achievements: HashMap<u32, Achievement>,
    /// 按分类索引 (category -> Vec<achievement_id>)
    by_category: HashMap<AchievementCategory, Vec<u32>>,
}

impl AchievementDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加成就
    pub fn add(&mut self, achievement: Achievement) {
        let id = achievement.id;
        self.achievements.insert(id, achievement.clone());

        self.by_category
            .entry(achievement.category)
            .or_default()
            .push(id);
    }

    /// 获取成就
    pub fn get(&self, id: u32) -> Option<&Achievement> {
        self.achievements.get(&id)
    }

    /// 获取所有成就
    pub fn all(&self) -> Vec<&Achievement> {
        self.achievements.values().collect()
    }

    /// 按分类获取成就
    pub fn get_by_category(&self, category: AchievementCategory) -> Vec<&Achievement> {
        self.by_category
            .get(&category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.achievements.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取前置成就
    pub fn get_prerequisites(&self, achievement_id: u32) -> Vec<u32> {
        self.achievements
            .get(&achievement_id)
            .map(|a| a.pre_achievements.clone())
            .unwrap_or_default()
    }

    /// 加载默认成就数据
    pub fn load_default_achievements(&mut self) {
        // 战斗成就
        self.add(
            Achievement::new(
                1,
                "First Blood",
                "Kill your first monster",
                AchievementCategory::Battle,
            )
            .with_condition(AchievementCondition::KillAnyMonster(1))
            .with_target(1)
            .with_reward(AchievementReward::new().with_cash(100)),
        );

        self.add(
            Achievement::new(
                2,
                "Monster Slayer",
                "Kill 100 monsters",
                AchievementCategory::MonsterHunt,
            )
            .with_condition(AchievementCondition::KillAnyMonster(100))
            .with_target(100)
            .with_reward(AchievementReward::new().with_cash(500).with_item(501, 10)),
        );

        self.add(
            Achievement::new(
                3,
                "Monster Hunter",
                "Kill 1000 monsters",
                AchievementCategory::MonsterHunt,
            )
            .with_condition(AchievementCondition::KillAnyMonster(1000))
            .with_target(1000)
            .with_reward(
                AchievementReward::new()
                    .with_cash(2000)
                    .with_title("Monster Hunter"),
            ),
        );

        // 升级成就
        self.add(
            Achievement::new(
                10,
                "Level 10",
                "Reach level 10",
                AchievementCategory::LevelUp,
            )
            .with_condition(AchievementCondition::ReachLevel(10))
            .with_target(10)
            .with_reward(AchievementReward::new().with_cash(200)),
        );

        self.add(
            Achievement::new(
                11,
                "Level 50",
                "Reach level 50",
                AchievementCategory::LevelUp,
            )
            .with_condition(AchievementCondition::ReachLevel(50))
            .with_target(50)
            .with_reward(AchievementReward::new().with_cash(1000).with_item(501, 50)),
        );

        self.add(
            Achievement::new(
                12,
                "Level 99",
                "Reach level 99",
                AchievementCategory::LevelUp,
            )
            .with_condition(AchievementCondition::ReachLevel(99))
            .with_target(99)
            .with_reward(
                AchievementReward::new()
                    .with_cash(10000)
                    .with_title("Legend"),
            ),
        );

        // 收集成就
        self.add(
            Achievement::new(
                20,
                "Collector",
                "Collect 100 items",
                AchievementCategory::Collection,
            )
            .with_condition(AchievementCondition::CollectItem(0, 100))
            .with_target(100)
            .with_reward(AchievementReward::new().with_cash(300)),
        );

        // 社交成就
        self.add(
            Achievement::new(30, "Friendly", "Win 10 duels", AchievementCategory::Social)
                .with_condition(AchievementCondition::WinDuels(10))
                .with_target(10)
                .with_reward(AchievementReward::new().with_cash(500)),
        );

        // 探索成就
        self.add(
            Achievement::new(
                40,
                "Explorer",
                "Explore 10 different maps",
                AchievementCategory::Adventure,
            )
            .with_condition(AchievementCondition::ExploreMaps(10))
            .with_target(10)
            .with_reward(AchievementReward::new().with_cash(300).with_item(502, 5)),
        );

        // 特殊成就
        self.add(
            Achievement::new(
                100,
                "First Steps",
                "Complete your first quest",
                AchievementCategory::Special,
            )
            .with_condition(AchievementCondition::CompleteQuest(0))
            .with_target(1)
            .with_reward(AchievementReward::new().with_cash(50)),
        );
    }
}
