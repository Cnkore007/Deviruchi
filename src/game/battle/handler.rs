use crate::game::map::Player;
use crate::game::mob::Mob;
use super::formula::BattleFormula;

/// 战斗处理器
pub struct BattleHandler;

impl BattleHandler {
    pub fn new() -> Self {
        Self
    }

    /// 普通攻击
    pub fn normal_attack(&self, attacker: &Player, defender: &Mob) -> AttackResult {
        let hit_chance = BattleFormula::hit_rate(attacker, defender);
        if !rand_chance(hit_chance) {
            return AttackResult::Miss;
        }

        let crit_chance = BattleFormula::crit_rate(attacker, defender);
        let is_crit = rand_chance(crit_chance);

        let base_damage = BattleFormula::physical_damage(attacker, defender, 100, 1);

        let damage = if is_crit {
            (base_damage * BattleFormula::crit_multiplier()) / 100
        } else {
            base_damage
        };

        let killed = defender.take_damage(damage as u32);

        AttackResult::Hit {
            damage,
            is_crit,
            killed,
        }
    }

    /// 技能攻击
    pub fn skill_attack(
        &self,
        attacker: &Player,
        defender: &Mob,
        skill_damage: i32,
        skill_id: u16,
    ) -> AttackResult {
        let hit_chance = BattleFormula::hit_rate(attacker, defender) + 5;
        if !rand_chance(hit_chance) {
            return AttackResult::Miss;
        }

        let crit_chance = BattleFormula::crit_rate(attacker, defender);
        let is_crit = rand_chance(crit_chance) && skill_id != 25;

        let base_damage = BattleFormula::physical_damage(attacker, defender, skill_damage, 1);

        let damage = if is_crit {
            (base_damage * BattleFormula::crit_multiplier()) / 100
        } else {
            base_damage
        };

        let killed = defender.take_damage(damage as u32);

        AttackResult::Hit {
            damage,
            is_crit,
            killed,
        }
    }

    /// 魔法攻击
    pub fn magic_attack(
        &self,
        attacker: &Player,
        defender: &Mob,
        skill_damage: i32,
    ) -> AttackResult {
        let matk = (*attacker.int.read() as i32) * 2 + (*attacker.dex.read() as i32) / 3;
        let damage = BattleFormula::magical_damage(attacker, defender, skill_damage, matk);

        let killed = defender.take_damage(damage as u32);

        AttackResult::Hit {
            damage,
            is_crit: false,
            killed,
        }
    }
}

impl Default for BattleHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 攻击结果
#[derive(Debug, Clone)]
pub enum AttackResult {
    Miss,
    Hit {
        damage: i32,
        is_crit: bool,
        killed: bool,
    },
    Blocked,
    Immune,
}

fn rand_chance(percent: i32) -> bool {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos as i32 % 100) < percent
}
