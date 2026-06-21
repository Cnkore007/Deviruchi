use super::formula::BattleFormula;
use crate::game::map::Player;
use crate::game::mob::Mob;
use crate::game::rand::GameRng;
use std::sync::Arc;

/// 将 i32 伤害安全转换为 u32，负值 clamp 到 0
fn safe_damage(damage: i32) -> u32 {
    damage.max(0) as u32
}

/// 战斗处理器
pub struct BattleHandler {
    rng: parking_lot::Mutex<Arc<dyn GameRng>>,
}

impl BattleHandler {
    pub fn new(rng: Arc<dyn GameRng>) -> Self {
        Self {
            rng: parking_lot::Mutex::new(rng),
        }
    }

    /// 普通攻击
    pub fn normal_attack(&self, attacker: &Player, defender: &Mob) -> AttackResult {
        let hit_chance = BattleFormula::hit_rate(attacker, defender);
        if !self.rand_chance(hit_chance) {
            return AttackResult::Miss;
        }

        let crit_chance = BattleFormula::crit_rate(attacker, defender);
        let is_crit = self.rand_chance(crit_chance);

        let rng_guard = self.rng.lock();
        let base_damage = BattleFormula::physical_damage(attacker, defender, 100, 1, &**rng_guard);

        let damage = if is_crit {
            // 使用 i64 中间值防止高基础伤害 * 140 溢出 i32
            ((base_damage as i64 * BattleFormula::crit_multiplier() as i64) / 100) as i32
        } else {
            base_damage
        };

        let killed = defender.take_damage(safe_damage(damage));

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
        if !self.rand_chance(hit_chance) {
            return AttackResult::Miss;
        }

        let crit_chance = BattleFormula::crit_rate(attacker, defender);
        let is_crit = self.rand_chance(crit_chance) && skill_id != 25;

        let rng_guard = self.rng.lock();
        let base_damage =
            BattleFormula::physical_damage(attacker, defender, skill_damage, 1, &**rng_guard);

        let damage = if is_crit {
            ((base_damage as i64 * BattleFormula::crit_multiplier() as i64) / 100) as i32
        } else {
            base_damage
        };

        let killed = defender.take_damage(safe_damage(damage));

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
        let matk = (attacker.int() as i32) * 2 + (attacker.dex() as i32) / 3;
        let rng_guard = self.rng.lock();
        let damage =
            BattleFormula::magical_damage(attacker, defender, skill_damage, matk, &**rng_guard);

        let killed = defender.take_damage(safe_damage(damage));

        AttackResult::Hit {
            damage,
            is_crit: false,
            killed,
        }
    }

    /// Mob 对 Player 的普通攻击
    pub fn mob_attack(&self, attacker: &Mob, defender: &Player) -> MobAttackResult {
        // 检查命中率
        let hit_chance = BattleFormula::mob_hit_rate(attacker, defender);
        if !self.rand_chance(hit_chance) {
            return MobAttackResult::Miss;
        }

        // 计算伤害
        let damage = BattleFormula::mob_physical_damage(attacker, defender);

        // 应用伤害
        let killed = defender.take_damage(safe_damage(damage));

        MobAttackResult::Hit { damage, killed }
    }

    fn rand_chance(&self, percent: i32) -> bool {
        let rng = self.rng.lock();
        let rand_val = rng.rand_range(0, 99);
        (rand_val as i32) < percent
    }
}

impl Default for BattleHandler {
    fn default() -> Self {
        Self::new(crate::game::rand::thread_rng())
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

/// Mob 攻击结果
#[derive(Debug, Clone)]
pub enum MobAttackResult {
    Miss,
    Hit { damage: i32, killed: bool },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rand::{GameRng, MockRng};

    fn create_test_handler(values: Vec<u32>) -> BattleHandler {
        BattleHandler::new(Arc::new(MockRng::new(values)))
    }

    #[test]
    fn test_rand_chance_always_true_when_100() {
        let handler = create_test_handler(vec![99, 50, 0]);
        assert!(handler.rand_chance(100));
        assert!(handler.rand_chance(100));
        assert!(handler.rand_chance(100));
    }

    #[test]
    fn test_rand_chance_always_false_when_0() {
        let handler = create_test_handler(vec![0, 50, 99]);
        assert!(!handler.rand_chance(0));
        assert!(!handler.rand_chance(0));
        assert!(!handler.rand_chance(0));
    }

    #[test]
    fn test_rand_chance_threshold() {
        // Mock returns 49, which is < 50
        let handler = create_test_handler(vec![49]);
        assert!(handler.rand_chance(50));
        assert!(!handler.rand_chance(49));

        // Mock returns 50, which is >= 50
        let handler = create_test_handler(vec![50]);
        assert!(!handler.rand_chance(50));
    }

    #[test]
    fn test_negative_damage_clamped_to_zero() {
        let negative_damage: i32 = -100;
        let safe = safe_damage(negative_damage);
        assert_eq!(safe, 0);
    }

    #[test]
    fn test_positive_damage_preserved() {
        let damage: i32 = 500;
        let safe = safe_damage(damage);
        assert_eq!(safe, 500);
    }

    #[test]
    fn test_zero_damage_preserved() {
        let safe = safe_damage(0);
        assert_eq!(safe, 0);
    }

    #[test]
    fn test_crit_damage_overflow_clamped() {
        // Simulate overflow: large_base * 140 overflows i32
        let large_base: i32 = i32::MAX / 2;
        let crit_damage = large_base.wrapping_mul(140) / 100; // overflows to negative
        let result = safe_damage(crit_damage);
        assert_eq!(result, 0);
    }
}
