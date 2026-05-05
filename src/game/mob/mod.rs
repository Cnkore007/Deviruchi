//! 怪物系统

pub mod ai;
pub mod data;
pub mod droptable;
pub mod pathfinder;
pub mod spawn;
pub mod yaml_loader;

pub use ai::MobAI;
pub use data::{Mob, MobAIState, MobBehavior, MobBehaviorFlags, MobDatabase, MobPosition, MobRace, MobSkill, MobSkillCondition, MobSkillTarget, MobType};
pub use droptable::{DropResolver, DropTableEntry, DropTableLoader, MVPResolver, MobDropTable};
pub use spawn::MobSpawnManager;
