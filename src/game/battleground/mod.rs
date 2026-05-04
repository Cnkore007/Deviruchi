//! 战场系统模块
//!
//! 提供战场（Battleground）和城战（WoE）系统的核心功能。

pub mod data;
pub mod manager;

pub use data::{
    BGError, Battleground, BattlegroundConfig, BattlegroundState, BattlegroundTeam,
    BattlegroundType, RespawnType, TeamColor,
};
pub use manager::{BattlegroundManager, BattlegroundStats, TeamStats};

/// 战场模块的错误类型
pub type Result<T> = std::result::Result<T, BGError>;
