//! Rhai 脚本公式引擎
//!
//! 提供可配置的战斗和状态公式，允许通过 `.rhai` 脚本覆盖默认计算逻辑。

pub mod battle;
pub mod engine;
pub mod status;

pub use engine::FormulaEngine;
