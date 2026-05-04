//! 状态效果系统
//!
//! 管理玩家和NPC的所有状态效果（Buff/Debuff）
//!
//! # 主要模块
//!
//! - [`types`] - 状态效果类型定义
//! - [`effect`] - 状态效果实例
//! - [`player_status`] - 玩家状态管理器
//! - [`calculator`] - 属性计算器
//! - [`icons`] - 图标定义
//! - [`tick`] - 周期处理

pub mod calculator;
pub mod effect;
pub mod icons;
pub mod player_status;
pub mod tick;
pub mod types;

// 导出主要类型
pub use calculator::{ReflectType, StatModifiers, StatusCalculator};
pub use effect::{StackingRule, StatusEffect, StatusSource};
pub use icons::{StatusEffectInfo, StatusIcon, StatusIcons};
pub use player_status::PlayerStatus;
pub use tick::{StatusTickConfig, StatusTickProcessor, StatusTickResult, StatusTickService};
pub use types::{StatusCategory, StatusChange};

/// 状态效果系统常量
pub mod consts {
    /// 最大状态效果数量
    pub const MAX_STATUS_EFFECTS: usize = 32;

    /// 最大DOT伤害（每次Tick）
    pub const MAX_DOT_DAMAGE: u32 = 9999;

    /// 最大属性加成
    pub const MAX_STAT_BONUS: i32 = 100;

    /// 无敌状态最大持续时间（毫秒）
    pub const MAX_INVINCIBLE_DURATION_MS: u64 = 300000; // 5分钟

    /// 隐身状态最大持续时间（毫秒）
    pub const MAX_STEALTH_DURATION_MS: u64 = 600000; // 10分钟
}
