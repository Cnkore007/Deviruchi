//! 宠物系统模块

pub mod ai;
pub mod data;
pub mod manager;

pub use ai::{PetAI, PetAIManager, PetAIState};
pub use data::{Pet, PetData, PetDatabase};
pub use manager::{PetError, PetManager};
