use std::sync::Arc;
use crate::game::map::Player;
use crate::game::mob::Mob;
use crate::game::rand::GameRng;
use super::formula::BattleFormula;

/// 战斗处理器
pub struct BattleHandler {
    rng: std::sync::Mutex<Arc<dyn GameRng>>,
}

impl BattleHandler {
    pub fn new(rng: Arc<dyn GameRng>) -> Self {
        Self { rng: std::sync::Mutex::new(rng) }
    }

    /// 普通攻击
    pub fn normal_attack(&self, attacker: &Player, defender: &Mob) -> AttackResult {
        let hit_chance = BattleFormula::hit_rate(attacker, defender);
        if !self.rand_chance(hit_chance) {
            return AttackResult::Miss;
        }

        let crit_chance = BattleFormula::crit_rate(attacker, defender);
        let is_crit = self.rand_chance(crit_chance);

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
        if !self.rand_chance(hit_chance) {
            return AttackResult::Miss;
        }

        let crit_chance = BattleFormula::crit_rate(attacker, defender);
        let is_crit = self.rand_chance(crit_chance) && skill_id != 25;

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
        let killed = defender.take_damage(damage as u32);

        MobAttackResult::Hit { damage, killed }
    }

    fn rand_chance(&self, percent: i32) -> bool {
        let mut rng = self.rng.lock().unwrap();
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
    Hit {
        damage: i32,
        killed: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rand::GameRng;
    use std::cell::UnsafeCell;

    /// Test-only mock RNG for battle handler tests
    struct MockRng {
        values: Vec<u32>,
        index: UnsafeCell<usize>,
    }

    impl MockRng {
        fn new(values: Vec<u32>) -> Self {
            Self {
                values,
                index: UnsafeCell::new(0),
            }
        }
    }

    // Safety: MockRng is only used in single-threaded test context
    unsafe impl Send for MockRng {}
    unsafe impl Sync for MockRng {}

    impl GameRng for MockRng {
        fn rand_range(&self, min: u32, max: u32) -> u32 {
            let idx = {
                let p = unsafe { &mut *self.index.get() };
                let current = *p;
                *p = current.wrapping_add(1);
                current
            };
            let val = self.values.get(idx % self.values.len()).copied().unwrap_or(min);
            val.min(max).max(min)
        }

        fn rand_bool(&self, _probability: f32) -> bool {
            let idx = {
                let p = unsafe { &mut *self.index.get() };
                let current = *p;
                *p = current.wrapping_add(1);
                current
            };
            let val = self.values.get(idx % self.values.len()).copied().unwrap_or(0);
            val % 2 == 0
        }

        fn rand_bp(&self, _chance: u32) -> u32 {
            let idx = {
                let p = unsafe { &mut *self.index.get() };
                let current = *p;
                *p = current.wrapping_add(1);
                current
            };
            self.values.get(idx % self.values.len()).copied().unwrap_or(0)
        }
    }

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
}
