//! 技能系统

pub mod data;
pub mod effect;
pub mod handler;

pub use data::{Skill, SkillType, SkillTarget};
pub use handler::SkillHandler;
