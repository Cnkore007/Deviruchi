//! 怪物系统

pub mod data;
pub mod spawn;
pub mod ai;

pub use data::{Mob, MobAIState, MobType};
pub use spawn::MobSpawnManager;
pub use ai::MobAI;
