//! WoE (War of Emperium) 城战系统模块
//!
//! 提供 War of Emperium 城战系统的核心功能，包括：
//! - 城堡管理
//! - WoE 时间安排
//! - 城堡攻击/防守
//! - 城堡占领

pub mod data;
pub mod manager;

pub use data::{
    Castle, CastleAttacker, CastleStatus, DEFAULT_CASTLES, DayOfWeek, DefaultCastle, WoEError,
    WoESchedule, WoEState,
};
pub use manager::WoEManager;

/// WoE 模块的错误类型
pub type Result<T> = std::result::Result<T, WoEError>;
