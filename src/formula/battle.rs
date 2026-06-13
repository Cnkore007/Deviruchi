//! 战斗公式脚本辅助函数

use super::engine::FormulaEngine;
use crate::game::battle::formula::BattleFormula;
use crate::game::map::Player;
use crate::game::mob::Mob;
use std::sync::Arc;

pub struct ScriptedBattleFormula {
    engine: Option<Arc<FormulaEngine>>,
}

impl ScriptedBattleFormula {
    pub fn new(engine: Option<Arc<FormulaEngine>>) -> Self {
        Self { engine }
    }

    pub fn hit_rate(&self, attacker: &Player, defender: &Mob) -> i32 {
        if let Some(ref engine) = self.engine
            && let Some(rate) = engine.call_battle_fn(
                "hit_rate",
                vec![
                    rhai::Dynamic::from(attacker.base_level() as i64),
                    rhai::Dynamic::from(attacker.dex() as i64),
                    rhai::Dynamic::from(attacker.luk() as i64),
                    rhai::Dynamic::from(defender.flee as i64),
                ],
            ).and_then(|v| v.as_int().ok())
        {
            return rate as i32;
        }
        BattleFormula::hit_rate(attacker, defender)
    }
}
