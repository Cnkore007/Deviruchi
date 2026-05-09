//! 雇佣兵 (Mercenary) 系统

pub mod data;
pub mod manager;
pub mod yaml_loader;

pub use data::{Mercenary, MercenaryClass, MercenaryData, MercenaryDatabase, MercenarySkill};
pub use manager::{MercenaryError, MercenaryManager};
