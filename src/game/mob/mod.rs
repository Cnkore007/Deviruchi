//! 怪物系统

pub mod data;
pub mod spawn;
pub mod ai;
pub mod pathfinder;
pub mod droptable;

pub use data::{Mob, MobAIState, MobType, MobDatabase};
pub use spawn::MobSpawnManager;
pub use ai::MobAI;
pub use droptable::{DropTableLoader, DropResolver, DropTableEntry, MobDropTable, MVPResolver};
