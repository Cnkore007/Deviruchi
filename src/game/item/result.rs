//! 物品使用结果定义

/// 物品使用结果
#[derive(Debug, Clone)]
pub enum ItemUseResult {
    /// 使用成功（带消息）
    Success(String),
    /// 使用失败（带原因）
    Failure(String),
    /// 触发传送
    Teleport { map: String, x: u16, y: u16 },
    /// 触发随机传送
    RandomTeleport { range: u16 },
    /// 触发技能
    SkillUsed { skill_id: u32 },
    /// 学习技能成功
    SkillLearned { skill_id: u32 },
    /// 复活
    Revive { hp_percent: u32 },
    /// 设置存档点
    SavePointSet,
    /// 无权限
    NoPermission,
    /// 冷却中
    CooldownActive { remaining_ms: u64 },
    /// 物品不存在
    ItemNotFound,
    /// 物品不可使用
    CannotUse,
    /// 死亡状态无法使用
    CannotUseWhileDead,
    /// 条件不满足
    RequirementsNotMet(String),
}

impl ItemUseResult {
    /// 是否成功
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            ItemUseResult::Success(_)
                | ItemUseResult::Teleport { .. }
                | ItemUseResult::RandomTeleport { .. }
                | ItemUseResult::SkillUsed { .. }
                | ItemUseResult::SkillLearned { .. }
                | ItemUseResult::Revive { .. }
                | ItemUseResult::SavePointSet
        )
    }

    /// 获取成功消息
    pub fn success_message(&self) -> Option<&str> {
        match self {
            ItemUseResult::Success(msg) => Some(msg),
            _ => None,
        }
    }

    /// 获取失败原因
    pub fn error_message(&self) -> Option<&str> {
        match self {
            ItemUseResult::Failure(msg) => Some(msg),
            ItemUseResult::RequirementsNotMet(msg) => Some(msg),
            _ => None,
        }
    }

    /// 获取消息（成功或失败）
    pub fn message(&self) -> &str {
        match self {
            ItemUseResult::Success(msg) => msg,
            ItemUseResult::Failure(msg) => msg,
            ItemUseResult::NoPermission => "你没有权限使用此物品",
            ItemUseResult::CooldownActive { .. } => "物品冷却中",
            ItemUseResult::ItemNotFound => "物品不存在",
            ItemUseResult::CannotUse => "此物品无法使用",
            ItemUseResult::CannotUseWhileDead => "死亡状态无法使用此物品",
            ItemUseResult::RequirementsNotMet(msg) => msg,
            ItemUseResult::Teleport { .. } => "正在传送...",
            ItemUseResult::RandomTeleport { .. } => "正在随机传送...",
            ItemUseResult::SkillUsed { .. } => "技能已使用",
            ItemUseResult::SkillLearned { .. } => "技能已学习",
            ItemUseResult::Revive { .. } => "已复活",
            ItemUseResult::SavePointSet => "存档点已设置",
        }
    }
}

impl Default for ItemUseResult {
    fn default() -> Self {
        ItemUseResult::Success(String::new())
    }
}

impl std::fmt::Display for ItemUseResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}
