//! 状态公式脚本辅助函数

use super::engine::FormulaEngine;
use std::sync::Arc;

pub struct ScriptedStatusFormula {
    engine: Option<Arc<FormulaEngine>>,
}

impl ScriptedStatusFormula {
    pub fn new(engine: Option<Arc<FormulaEngine>>) -> Self {
        Self { engine }
    }

    pub fn stat_point_cost(&self, current: i64, is_renewal: bool) -> i64 {
        if let Some(ref engine) = self.engine {
            let fn_name = if is_renewal { "stat_point_cost_re" } else { "stat_point_cost_pre" };
            if let Some(cost) = engine.call_status_fn(
                fn_name,
                vec![rhai::Dynamic::from(current)],
            ).and_then(|v| v.as_int().ok()) {
                return cost;
            }
        }
        if is_renewal {
            if current < 100 { 2 + (current - 1) / 10 } else { 16 + 4 * ((current - 100) / 5) }
        } else {
            (current + 9) / 10 + 1
        }
    }
}
