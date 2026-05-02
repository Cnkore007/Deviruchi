use crate::game::map::Player;
use crate::game::mob::Mob;

/// rAthena 风格战斗公式
pub struct BattleFormula;

impl BattleFormula {
    /// 计算物理攻击伤害
    pub fn physical_damage(
        attacker: &Player,
        defender: &Mob,
        skill_damage_bonus: i32,
        weapon_type: i32,
    ) -> i32 {
        let base_atk = {
            let base_level = *attacker.base_level.read() as i32;
            let str = *attacker.str.read() as i32;
            let dex = *attacker.dex.read() as i32;
            let agi = *attacker.agi.read() as i32;

            (base_level * 2 + str + dex / 2 + agi / 3) as i32
        };

        let weapon_atk = weapon_type * 2;
        let total_atk = base_atk + weapon_atk;
        let defense = defender.defense as i32;

        let damage = ((total_atk - defense).max(1) * skill_damage_bonus) / 100;
        let variance = 90 + (rand_range(0, 20) as i32);
        (damage * variance) / 100
    }

    /// 计算魔法攻击伤害
    pub fn magical_damage(
        attacker: &Player,
        defender: &Mob,
        skill_damage_bonus: i32,
        matk: i32,
    ) -> i32 {
        let base_matk = {
            let int = *attacker.int.read() as i32;
            let dex = *attacker.dex.read() as i32;
            let base_level = *attacker.base_level.read() as i32;

            int * 2 + dex / 3 + base_level / 4
        };

        let magic_atk = matk.max(base_matk);
        let magic_defense = defender.magic_defense as i32;

        let damage = ((magic_atk - magic_defense).max(1) * skill_damage_bonus) / 100;
        let variance = 90 + (rand_range(0, 20) as i32);
        (damage * variance) / 100
    }

    /// 计算命中率
    pub fn hit_rate(attacker: &Player, defender: &Mob) -> i32 {
        let hit = {
            let dex = *attacker.dex.read() as i32;
            let base_level = *attacker.base_level.read() as i32;
            (dex * 3) + base_level
        };
        let flee = defender.flee as i32;
        95 + (hit - flee) / 2
    }

    /// 计算闪避率
    pub fn flee_rate(player: &Player, mob: &Mob) -> i32 {
        let agi = *player.agi.read() as i32;
        let base_level = *player.base_level.read() as i32;
        80 + agi - (base_level * 2)
    }

    /// 计算暴击率
    pub fn crit_rate(attacker: &Player, defender: &Mob) -> i32 {
        let base_crit = 0;
        let luk = *attacker.luk.read() as i32;
        base_crit + luk / 3
    }

    /// 计算暴击伤害
    pub fn crit_multiplier() -> i32 {
        140
    }

    /// 伤害减免
    pub fn damage_reduction(defense: i32) -> i32 {
        ((defense as f32) / (defense as f32 + 100.0) * 100.0) as i32
    }
}

fn rand_range(min: i32, max: i32) -> i32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let range = max - min + 1;
    min + ((nanos as i32) % range)
}
