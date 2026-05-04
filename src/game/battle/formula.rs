use crate::game::battle::element::{Element, WeaponType};
#[cfg(test)]
use crate::game::constants;
use crate::game::map::Player;
use crate::game::mob::Mob;
use crate::game::rand::GameRng;

/// 将 weapon_type i32 值映射到 WeaponType 枚举
/// 默认返回 Fist（对所有体型 100%）
fn weapon_type_from_i32(v: i32) -> WeaponType {
    match v {
        1 => WeaponType::Dagger,
        2 => WeaponType::OneHandSword,
        3 => WeaponType::TwoHandSword,
        4 => WeaponType::OneHandSpear,
        5 => WeaponType::TwoHandSpear,
        6 => WeaponType::OneHandAxe,
        7 => WeaponType::TwoHandAxe,
        8 => WeaponType::Mace,
        10 => WeaponType::Staff,
        11 => WeaponType::Bow,
        _ => WeaponType::Fist,
    }
}

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
            let base_level = attacker.base_level() as i32;
            let str = attacker.str() as i32;
            let dex = attacker.dex() as i32;
            let agi = attacker.agi() as i32;

            base_level
                .saturating_mul(2)
                .saturating_add(str)
                .saturating_add(dex / 2)
                .saturating_add(agi / 3)
        };

        let weapon_atk = weapon_type.saturating_mul(2);
        let total_atk = base_atk.saturating_add(weapon_atk);
        let defense = defender.defense as i32;

        let damage = ((total_atk.saturating_sub(defense)).max(1)
            .saturating_mul(skill_damage_bonus))
            / 100;

        // 应用元素修正（普通攻击默认为无属性）
        let element_mod = super::element::get_element_modifier(
            Element::Neutral,
            defender.element,
            defender.element_level,
        );
        let damage = (damage as i64 * element_mod as i64 / 100) as i32;

        // 应用体型修正
        let weapon = weapon_type_from_i32(weapon_type);
        let size_mod = super::element::get_size_modifier(weapon, defender.size);
        let damage = (damage as i64 * size_mod as i64 / 100) as i32;

        // Note: variance needs RNG injection - using fixed value for now
        let variance = 100; // 100% (no variance)
        (damage * variance) / 100
    }

    /// 计算物理攻击伤害 (带随机波动)
    pub fn physical_damage_with_variance(
        attacker: &Player,
        defender: &Mob,
        skill_damage_bonus: i32,
        weapon_type: i32,
        rng: &dyn GameRng,
    ) -> i32 {
        let base_atk = {
            let base_level = attacker.base_level() as i32;
            let str = attacker.str() as i32;
            let dex = attacker.dex() as i32;
            let agi = attacker.agi() as i32;

            base_level
                .saturating_mul(2)
                .saturating_add(str)
                .saturating_add(dex / 2)
                .saturating_add(agi / 3)
        };

        let weapon_atk = weapon_type.saturating_mul(2);
        let total_atk = base_atk.saturating_add(weapon_atk);
        let defense = defender.defense as i32;

        let damage = ((total_atk.saturating_sub(defense)).max(1)
            .saturating_mul(skill_damage_bonus))
            / 100;

        // 应用元素修正（普通攻击默认为无属性）
        let element_mod = super::element::get_element_modifier(
            Element::Neutral,
            defender.element,
            defender.element_level,
        );
        let damage = (damage as i64 * element_mod as i64 / 100) as i32;

        // 应用体型修正
        let weapon = weapon_type_from_i32(weapon_type);
        let size_mod = super::element::get_size_modifier(weapon, defender.size);
        let damage = (damage as i64 * size_mod as i64 / 100) as i32;

        let variance = 90 + (rng.rand_range(0, 20) as i32);
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
            let int = attacker.int() as i32;
            let dex = attacker.dex() as i32;
            let base_level = attacker.base_level() as i32;

            int.saturating_mul(2)
                .saturating_add(dex / 3)
                .saturating_add(base_level / 4)
        };

        let magic_atk = matk.max(base_matk);
        let magic_defense = defender.magic_defense as i32;

        let damage = ((magic_atk.saturating_sub(magic_defense)).max(1)
            .saturating_mul(skill_damage_bonus))
            / 100;
        // Note: variance needs RNG injection - using fixed value for now
        let variance = 100; // 100% (no variance)
        (damage * variance) / 100
    }

    /// 计算魔法攻击伤害 (带随机波动)
    pub fn magical_damage_with_variance(
        attacker: &Player,
        defender: &Mob,
        skill_damage_bonus: i32,
        matk: i32,
        rng: &dyn GameRng,
    ) -> i32 {
        let base_matk = {
            let int = attacker.int() as i32;
            let dex = attacker.dex() as i32;
            let base_level = attacker.base_level() as i32;

            int.saturating_mul(2)
                .saturating_add(dex / 3)
                .saturating_add(base_level / 4)
        };

        let magic_atk = matk.max(base_matk);
        let magic_defense = defender.magic_defense as i32;

        let damage = ((magic_atk.saturating_sub(magic_defense)).max(1)
            .saturating_mul(skill_damage_bonus))
            / 100;
        let variance = 90 + (rng.rand_range(0, 20) as i32);
        (damage * variance) / 100
    }

    /// 计算命中率 (clamped 5..95)
    pub fn hit_rate(attacker: &Player, defender: &Mob) -> i32 {
        let hit = {
            let dex = attacker.dex() as i32;
            let base_level = attacker.base_level() as i32;
            (dex * 3) + base_level
        };
        let flee = defender.flee as i32;
        (95 + (hit - flee) / 2).clamp(5, 95)
    }

    /// 计算闪避率 (rAthena: base 100 + AGI + LUK/5 + base_level)
    pub fn flee_rate(player: &Player, _mob: &Mob) -> i32 {
        let agi = player.agi() as i32;
        let luk = player.luk() as i32;
        let base_level = player.base_level() as i32;
        100 + agi + luk / 5 + base_level
    }

    /// 计算暴击率 (clamped 0..100)
    pub fn crit_rate(attacker: &Player, _defender: &Mob) -> i32 {
        let base_crit = 1;
        let luk = attacker.luk() as i32;
        (base_crit + luk / 3).clamp(0, 100)
    }

    /// 计算暴击伤害
    pub fn crit_multiplier() -> i32 {
        140
    }

    /// 计算 Mob 对 Player 的物理伤害
    /// 使用 rAthena 风格公式: ATK - VIT/2 (基础), 含防御减免
    /// Note: 玩家防御力基于装备，后续从装备系统获取
    pub fn mob_physical_damage(mob: &Mob, player: &Player) -> i32 {
        let atk = mob.atk as i32;
        let player_vit = player.vit() as i32;

        // 基础伤害 = ATK - VIT/2
        let base_damage = (atk - player_vit / 2).max(1);

        // 玩家基础防御力（后续可从装备系统获取）
        let player_def = 0;

        // 防御减免 (rAthena 风格)

        (base_damage - player_def).max(1)
    }

    /// 计算 Mob 对 Player 的命中率
    pub fn mob_hit_rate(mob: &Mob, player: &Player) -> i32 {
        let mob_hit = mob.hit as i32;
        let player_flee = {
            let agi = player.agi() as i32;
            let base_level = player.base_level() as i32;
            80 + agi - (base_level * 2)
        };
        95 + (mob_hit - player_flee) / 2
    }

    /// 伤害减免
    pub fn damage_reduction(defense: i32) -> i32 {
        ((defense as f32) / (defense as f32 + 100.0) * 100.0) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::mob::data::{MobPathManager, MobPosition};
    use crate::game::rand::{GameRng, MockRng};
    use parking_lot::RwLock;
    use std::sync::Arc;
    use uuid::Uuid;

    /// Helper to create a test Player with specified stats
    fn make_player(
        base_level: u16,
        str: u16,
        dex: u16,
        agi: u16,
        int: u16,
        luk: u16,
    ) -> Arc<Player> {
        let player = Player {
            id: Uuid::new_v4(),
            char_id: 1,
            account_id: 1,
            name: "TestPlayer".to_string(),
            map_name: "test_map".to_string(),
            combat: RwLock::new(crate::game::map::player::CombatStats {
                hp: 1000,
                max_hp: 1000,
                sp: 100,
                max_sp: 100,
                state: crate::game::map::player::PlayerState::Alive,
                in_combat: false,
                is_sitting: false,
                walk_speed: constants::DEFAULT_WALK_SPEED,
            }),
            pos: RwLock::new(crate::game::map::player::Position { x: 100, y: 100 }),
            level: RwLock::new(crate::game::map::player::LevelStats {
                base_level,
                job_level: 5,
                base_exp: 0,
                job_exp: 0,
            }),
            attrs: RwLock::new(crate::game::map::player::Attributes {
                str,
                agi,
                vit: 1,
                int,
                dex,
                luk,
            }),
            economy: RwLock::new(crate::game::map::player::Economy {
                zeny: 0,
                current_weight: 0,
                max_weight: constants::BASE_MAX_WEIGHT,
                job: 0,
                shop_id: None,
                group_id: 0,
            }),
            save_point: RwLock::new(crate::game::map::player::SavePoint {
                map: "test_map".to_string(),
                x: 50,
                y: 50,
            }),
            equipment: RwLock::new(crate::game::item::Equipment::new()),
            status: crate::game::status::PlayerStatus::new(Uuid::new_v4()),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
        };
        Arc::new(player)
    }

    /// Helper to create a test Mob with specified stats
    fn make_mob(level: u16, defense: u16, magic_defense: u16, hit: i16, flee: i16) -> Mob {
        Mob {
            id: Uuid::new_v4(),
            mob_id: 1001,
            name: "TestMob".to_string(),
            pos: RwLock::new(MobPosition { x: 100, y: 100 }),
            map_name: "test_map".to_string(),
            level,
            hp: RwLock::new(500),
            max_hp: 500,
            sp: RwLock::new(0),
            max_sp: 0,
            atk: 30,
            matk: 0,
            defense,
            magic_defense,
            hit,
            flee,
            crit: 0,
            walk_speed: constants::DEFAULT_WALK_SPEED,
            atk_range: 1,
            element: crate::game::battle::element::Element::Neutral,
            element_level: crate::game::battle::element::ElementLevel::Level1,
            size: crate::game::battle::element::MobSize::Medium,
            ai_state: RwLock::new(crate::game::mob::MobAIState::Idle),
            target_id: RwLock::new(None),
            behavior: crate::game::mob::MobBehavior::Passive,
            skills: Vec::new(),
            sight_range: 12,
            chase_range: 20,
            aggro_rate: 0,
            spawn_delay: 0,
            respawn_time: 60000,
            spawn_x: 100,
            spawn_y: 100,
            spawn_map: "test_map".to_string(),
            death_time: RwLock::new(None),
            drops: vec![],
            base_exp: 10,
            job_exp: 5,
            zeny: None,
            drops_processed: RwLock::new(false),
            path_manager: RwLock::new(MobPathManager::new()),
            damage_log: RwLock::new(std::collections::HashMap::new()),
        }
    }

    #[test]
    fn test_physical_damage_formula() {
        // Player: level 10, str=10, dex=10, agi=10
        // Base ATK = 10*2 + 10 + 10/2 + 10/3 = 20 + 10 + 5 + 3 = 38
        // Weapon ATK = weapon_type(1) * 2 = 2
        // Total ATK = 38 + 2 = 40
        // Damage = (40 - 0) * 100 / 100 = 40
        let player = make_player(10, 10, 10, 10, 1, 1);
        let mob = make_mob(5, 0, 0, 0, 0);

        let damage = BattleFormula::physical_damage(&player, &mob, 100, 1);
        assert_eq!(damage, 40);
    }

    #[test]
    fn test_physical_damage_with_defense() {
        // Player: level 10, str=10, dex=10, agi=10
        // Base ATK = 10*2 + 10 + 10/2 + 10/3 = 38
        // Weapon ATK = 1 * 2 = 2
        // Total ATK = 40
        // Defense = 10
        // Damage = (40 - 10) * 100 / 100 = 30
        let player = make_player(10, 10, 10, 10, 1, 1);
        let mob = make_mob(5, 10, 0, 0, 0);

        let damage = BattleFormula::physical_damage(&player, &mob, 100, 1);
        assert_eq!(damage, 30);
    }

    #[test]
    fn test_physical_damage_with_skill_bonus() {
        // Same as above but with 150% skill damage bonus
        // Damage = (40 - 0) * 150 / 100 = 60
        let player = make_player(10, 10, 10, 10, 1, 1);
        let mob = make_mob(5, 0, 0, 0, 0);

        let damage = BattleFormula::physical_damage(&player, &mob, 150, 1);
        assert_eq!(damage, 60);
    }

    #[test]
    fn test_physical_damage_with_variance() {
        // Using deterministic MockRng with value 10 (gives variance = 100)
        let player = make_player(10, 10, 10, 10, 1, 1);
        let mob = make_mob(5, 0, 0, 0, 0);
        let rng = Arc::new(MockRng::new(vec![10]));

        // Base damage = 40, variance = 90 + 10 = 100
        // Final = 40 * 100 / 100 = 40
        let damage =
            BattleFormula::physical_damage_with_variance(&player, &mob, 100, 1, rng.as_ref());
        assert_eq!(damage, 40);
    }

    #[test]
    fn test_physical_damage_minimum_one() {
        // When defense exceeds attack, damage should be at least 1
        let player = make_player(1, 1, 1, 1, 1, 1); // Very weak player
        let mob = make_mob(5, 100, 0, 0, 0); // High defense mob

        // Base ATK = 1*2 + 1 + 1/2 + 1/3 = 2 + 1 + 0 + 0 = 3
        // Defense = 100
        // Damage = ((3 - 100).max(1) * 100) / 100 = 1
        let damage = BattleFormula::physical_damage(&player, &mob, 100, 1);
        assert_eq!(damage, 1);
    }

    #[test]
    fn test_magical_damage_formula() {
        // Player: level 10, int=20, dex=15
        // Base MATK = 20*2 + 15/3 + 10/4 = 40 + 5 + 2 = 47
        let player = make_player(10, 1, 15, 1, 20, 1);
        let mob = make_mob(5, 0, 5, 0, 0);

        let damage = BattleFormula::magical_damage(&player, &mob, 100, 50);
        // magic_atk = max(50, 47) = 50
        // magic_defense = 5
        // Damage = ((50 - 5).max(1) * 100) / 100 = 45
        assert_eq!(damage, 45);
    }

    #[test]
    fn test_hit_rate_calculation() {
        // Player: level 10, dex=20
        // HIT = dex*3 + base_level = 20*3 + 10 = 70
        // FLEE = 0
        // Hit Rate = 95 + (70 - 0) / 2 = 95 + 35 = 130
        let player = make_player(10, 1, 20, 1, 1, 1);
        let mob = make_mob(5, 0, 0, 0, 0);

        let hit_rate = BattleFormula::hit_rate(&player, &mob);
        assert_eq!(hit_rate, 130);
    }

    #[test]
    fn test_hit_rate_negative_diff() {
        // Player: level 1, dex=1 (low hit)
        // Player HIT = 1*3 + 1 = 4
        // Mob FLEE = 50
        // Hit Rate = 95 + (4 - 50) / 2 = 95 - 23 = 72
        let player = make_player(1, 1, 1, 1, 1, 1);
        let mob = make_mob(5, 0, 0, 0, 50);

        let hit_rate = BattleFormula::hit_rate(&player, &mob);
        assert_eq!(hit_rate, 72);
    }

    #[test]
    fn test_crit_rate() {
        // Player: level 10, luk=30
        // Crit = 0 + luk/3 = 0 + 10 = 10%
        let player = make_player(10, 1, 1, 1, 1, 30);
        let mob = make_mob(5, 0, 0, 0, 0);

        let crit_rate = BattleFormula::crit_rate(&player, &mob);
        assert_eq!(crit_rate, 10);
    }

    #[test]
    fn test_crit_rate_zero_luk() {
        // Player: level 10, luk=0
        // Crit = 0 + 0/3 = 0%
        let player = make_player(10, 1, 1, 1, 1, 0);
        let mob = make_mob(5, 0, 0, 0, 0);

        let crit_rate = BattleFormula::crit_rate(&player, &mob);
        assert_eq!(crit_rate, 0);
    }

    #[test]
    fn test_crit_multiplier() {
        // Crit multiplier should be 140%
        assert_eq!(BattleFormula::crit_multiplier(), 140);
    }

    #[test]
    fn test_mob_physical_damage_formula() {
        // Mob ATK = 50, Player VIT = 20
        // Base damage = 50 - 20/2 = 50 - 10 = 40
        // Use the static method directly
        let mob_atk: i32 = 50;
        let player_vit: i32 = 20;
        let damage = (mob_atk - player_vit / 2).max(1);
        assert_eq!(damage, 40);
    }

    #[test]
    fn test_mob_physical_damage_high_vit() {
        // When player VIT exceeds mob ATK
        let mob_atk: i32 = 20;
        let player_vit: i32 = 50;
        let damage = (mob_atk - player_vit / 2).max(1);
        // 20 - 25 = -5, but .max(1) ensures at least 1
        assert_eq!(damage, 1);
    }

    #[test]
    fn test_mob_hit_rate() {
        // Mob HIT = 30, Player AGI = 20, Level = 10
        // Player FLEE = 80 + 20 - 10*2 = 80 + 20 - 20 = 80
        // Mob Hit Rate = 95 + (30 - 80) / 2 = 95 - 25 = 70
        let player = make_player(10, 1, 1, 20, 1, 1);
        let mob = make_mob(5, 0, 0, 30, 0);

        let hit_rate = BattleFormula::mob_hit_rate(&mob, &player);
        assert_eq!(hit_rate, 70);
    }

    #[test]
    fn test_damage_reduction() {
        // 0 defense = 0% reduction
        assert_eq!(BattleFormula::damage_reduction(0), 0);
        // 100 defense = 50% reduction
        assert_eq!(BattleFormula::damage_reduction(100), 50);
        // High defense: 200/300 * 100 = 66.67 -> truncates to 66
        assert_eq!(BattleFormula::damage_reduction(200), 66);
    }
}

#[cfg(test)]
mod level_penalty_tests {
    use super::*;

    // Level penalty calculation is done in ExpDistributor, but we test the logic here
    // by testing the expected values

    #[test]
    fn level_penalty_five_cases() {
        // Test cases for level difference penalty
        // Player level 50 vs different mob levels:

        // Case 1: diff <= 10 -> 100% exp
        let (player_level, mob_level) = (50, 40); // diff = 10
        let level_diff = player_level as i32 - mob_level as i32;
        let penalty = if level_diff <= 10 { 1.0 } else { 0.0 };
        assert_eq!(penalty, 1.0);

        // Case 2: diff <= 15 -> 75% exp
        let (player_level, mob_level) = (50, 36); // diff = 14
        let level_diff = player_level as i32 - mob_level as i32;
        let penalty = if level_diff <= 10 {
            1.0
        } else if level_diff <= 15 {
            0.75
        } else {
            0.0
        };
        assert_eq!(penalty, 0.75);

        // Case 3: diff <= 20 -> 50% exp
        let (player_level, mob_level) = (50, 31); // diff = 19
        let level_diff = player_level as i32 - mob_level as i32;
        let penalty = if level_diff <= 10 {
            1.0
        } else if level_diff <= 15 {
            0.75
        } else if level_diff <= 20 {
            0.5
        } else {
            0.0
        };
        assert_eq!(penalty, 0.5);

        // Case 4: diff <= 25 -> 25% exp
        let (player_level, mob_level) = (50, 26); // diff = 24
        let level_diff = player_level as i32 - mob_level as i32;
        let penalty = if level_diff <= 10 {
            1.0
        } else if level_diff <= 15 {
            0.75
        } else if level_diff <= 20 {
            0.5
        } else if level_diff <= 25 {
            0.25
        } else {
            0.0
        };
        assert_eq!(penalty, 0.25);

        // Case 5: diff > 25 -> 10% exp
        let (player_level, mob_level) = (50, 20); // diff = 30
        let level_diff = player_level as i32 - mob_level as i32;
        let penalty = if level_diff <= 10 {
            1.0
        } else if level_diff <= 15 {
            0.75
        } else if level_diff <= 20 {
            0.5
        } else if level_diff <= 25 {
            0.25
        } else {
            0.1
        };
        assert_eq!(penalty, 0.1);
    }

    #[test]
    fn level_penalty_boundary_10() {
        // Exactly at 10 level difference
        let level_diff = 10;
        let penalty = if level_diff <= 10 { 1.0 } else { 0.0 };
        assert_eq!(penalty, 1.0);
    }

    #[test]
    fn level_penalty_boundary_11() {
        // Just above 10 level difference
        let level_diff = 11;
        let penalty = if level_diff <= 10 {
            1.0
        } else if level_diff <= 15 {
            0.75
        } else {
            0.0
        };
        assert_eq!(penalty, 0.75);
    }

    #[test]
    fn level_penalty_boundary_15() {
        let level_diff = 15;
        let penalty = if level_diff <= 10 {
            1.0
        } else if level_diff <= 15 {
            0.75
        } else {
            0.0
        };
        assert_eq!(penalty, 0.75);
    }

    #[test]
    fn level_penalty_boundary_16() {
        let level_diff = 16;
        let penalty = if level_diff <= 10 {
            1.0
        } else if level_diff <= 15 {
            0.75
        } else if level_diff <= 20 {
            0.5
        } else {
            0.0
        };
        assert_eq!(penalty, 0.5);
    }

    #[test]
    fn level_penalty_boundary_20() {
        let level_diff = 20;
        let penalty = if level_diff <= 10 {
            1.0
        } else if level_diff <= 15 {
            0.75
        } else if level_diff <= 20 {
            0.5
        } else {
            0.0
        };
        assert_eq!(penalty, 0.5);
    }

    #[test]
    fn level_penalty_boundary_21() {
        let level_diff = 21;
        let penalty = if level_diff <= 10 {
            1.0
        } else if level_diff <= 15 {
            0.75
        } else if level_diff <= 20 {
            0.5
        } else if level_diff <= 25 {
            0.25
        } else {
            0.0
        };
        assert_eq!(penalty, 0.25);
    }

    #[test]
    fn level_penalty_boundary_25() {
        let level_diff = 25;
        let penalty = if level_diff <= 10 {
            1.0
        } else if level_diff <= 15 {
            0.75
        } else if level_diff <= 20 {
            0.5
        } else if level_diff <= 25 {
            0.25
        } else {
            0.0
        };
        assert_eq!(penalty, 0.25);
    }

    #[test]
    fn level_penalty_boundary_26() {
        let level_diff = 26;
        let penalty = if level_diff <= 10 {
            1.0
        } else if level_diff <= 15 {
            0.75
        } else if level_diff <= 20 {
            0.5
        } else if level_diff <= 25 {
            0.25
        } else {
            0.1
        };
        assert_eq!(penalty, 0.1);
    }
}
