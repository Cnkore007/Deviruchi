//! 怪物系统

pub mod ai;
pub mod data;
pub mod droptable;
pub mod pathfinder;
pub mod spawn;
pub mod yaml_loader;

pub use ai::MobAI;
pub use data::{Mob, MobAIState, MobBehavior, MobDatabase, MobPosition, MobRace, MobSkill, MobType};
pub use droptable::{DropResolver, DropTableEntry, DropTableLoader, MVPResolver, MobDropTable};
pub use spawn::MobSpawnManager;
