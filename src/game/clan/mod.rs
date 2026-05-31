//! 公会联盟/家族系统
//!
//! 对应 rAthena 的 `src/map/clan.cpp`，提供公会联盟管理功能。

pub mod data;
pub mod manager;

pub use data::{AllianceType, Clan, ClanAlliance, ClanMember};
pub use manager::{ClanManager, ClanResult};
