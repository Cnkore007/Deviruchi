//! NPC系统

pub mod data;
pub mod handler;
pub mod yaml_loader;

pub use data::{Npc, NpcDatabase, NpcEvent, NpcFlag, NpcType};
pub use handler::NpcHandler;
