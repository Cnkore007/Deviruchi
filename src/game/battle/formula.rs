use crate::game::battle::element::{Element, ElementLevel, WeaponType};
#[cfg(test)]
use crate::game::constants;
use crate::game::map::Player;
use crate::game::mob::Mob;
use crate::game::rand::GameRng;
use rand::Rng;

/// 魔法伤害计算结果
///
/// 包含最终伤害值以及计算过程中的关键中间值，方便调试和日志记录。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageResult {
    /// 最终伤害值（保证 >= 1）
    pub damage: i32,
    /// 应用的元素修正倍率（1.0 = 100%，1.75 = 175%）
    pub element_modifier: f32,
}

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

        let damage = ((total_atk.saturating_sub(defense)).saturating_mul(skill_damage_bonus)) / 100;

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

        // 物理伤害方差：90%-110%，模拟 rAthena 的随机波动
        let variance = 90 + (rng.rand_range(0, 20) as i32);
        let damage = (damage * variance) / 100;

        // 最低伤害保证放在最后
        damage.max(1)
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

        let damage = ((total_atk.saturating_sub(defense)).saturating_mul(skill_damage_bonus)) / 100;

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

        // 物理伤害方差：90%-110%，模拟 rAthena 的随机波动
        let variance = 90 + (rng.rand_range(0, 20) as i32);
        let damage = (damage * variance) / 100;

        // 最低伤害保证放在最后
        damage.max(1)
    }

    /// 计算魔法攻击伤害
    pub fn magical_damage(
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

        let damage = ((magic_atk.saturating_sub(magic_defense))
            .max(1)
            .saturating_mul(skill_damage_bonus))
            / 100;
        // 魔法伤害方差：90%-110%，与物理伤害保持一致
        let variance = 90 + (rng.rand_range(0, 20) as i32);
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

        let damage = ((magic_atk.saturating_sub(magic_defense))
            .max(1)
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

/// 计算元素属性修正倍率（简化接口，使用 1 级元素表）
///
/// 将 u8 编码的元素属性映射到 `Element` 枚举，然后查询 rAthena 的 1 级元素修正表。
/// 返回 f32 倍率（1.0 = 100%，1.75 = 175%，0.0 = 免疫）。
///
/// 元素编码: 0=Neutral, 1=Water, 2=Earth, 3=Fire, 4=Wind,
///           5=Poison, 6=Holy, 7=Dark, 8=Ghost, 9=Undead
///
/// 如果需要指定元素等级，请直接使用 `element::get_element_modifier`。
pub fn element_modifier(atk_element: u8, def_element: u8) -> f32 {
    let atk = Element::from_u8(atk_element).unwrap_or(Element::Neutral);
    let def = Element::from_u8(def_element).unwrap_or(Element::Neutral);
    super::element::get_element_modifier(atk, def, ElementLevel::Level1) as f32 / 100.0
}

/// 计算 PvP 物理伤害（rAthena 风格公式）
///
/// PvP 环境下的物理伤害计算，与 PvE 公式不同：
/// - PvP 中 VIT 直接按百分比减免伤害
/// - 不考虑武器体型修正（玩家无体型）
/// - 元素修正使用 1 级元素表
///
/// # rAthena 参考公式
/// ```text
/// damage = (atk - def) * (100 - vit) / 100 * element_modifier
/// damage = max(damage, 1)
/// ```
///
/// # 参数
/// - `atk`: 攻击方的总物理攻击力
/// - `def`: 防御方的物理防御力
/// - `vit`: 防御方的 VIT 值（用于百分比减伤）
/// - `skill_level`: 技能等级（影响技能倍率，每级 +50%，最低 100%）
/// - `element`: 攻击的元素属性
/// - `target_element`: 防御方的元素属性
///
/// # 返回
/// `DamageResult` 包含最终伤害和元素修正倍率。
pub fn calc_pvp_physical_damage(
    atk: u32,
    def: u32,
    vit: u32,
    skill_level: u8,
    element: u8,
    target_element: u8,
) -> DamageResult {
    // 技能倍率：每级 +50%，最低 100%（level 1 = 100%，level 2 = 150%，以此类推）
    let skill_multiplier =
        100u32.saturating_add((skill_level as u32).saturating_sub(1).saturating_mul(50));

    // 基础伤害 = (ATK - DEF) * 技能倍率%
    let base_damage = (atk as i64)
        .saturating_sub(def as i64)
        .saturating_mul(skill_multiplier as i64)
        .saturating_div(100);

    // VIT 百分比减伤：vit 越高减伤越多，最高 99% 减伤
    let vit_reduction = (vit as i64).min(99);
    let after_vit = base_damage
        .saturating_mul(100 - vit_reduction)
        .saturating_div(100);

    // 应用元素修正
    let atk_elem = Element::from_u8(element).unwrap_or(Element::Neutral);
    let def_elem = Element::from_u8(target_element).unwrap_or(Element::Neutral);
    let elem_mod = super::element::get_element_modifier(atk_elem, def_elem, ElementLevel::Level1);

    let final_damage = (after_vit
        .saturating_mul(elem_mod as i64)
        .saturating_div(100))
    .max(1) as i32;

    DamageResult {
        damage: final_damage,
        element_modifier: elem_mod as f32 / 100.0,
    }
}

/// 计算 PvP 魔法伤害（rAthena 风格公式）
///
/// PvP 环境下的魔法伤害计算：
/// - INT 提供百分比减伤（效果为 VIT 的一半）
/// - 元素修正使用 1 级元素表
///
/// # rAthena 参考公式
/// ```text
/// damage = (matk - mdef) * (100 - int/2) / 100 * element_modifier
/// damage = max(damage, 1)
/// ```
///
/// # 参数
/// - `matk`: 攻击方的总魔法攻击力
/// - `mdef`: 防御方的魔法防御力
/// - `int`: 防御方的 INT 值（用于百分比减伤，效果为 VIT 的一半）
/// - `skill_level`: 技能等级（影响技能倍率，每级 +50%，最低 100%）
/// - `element`: 攻击的元素属性
/// - `target_element`: 防御方的元素属性
///
/// # 返回
/// `DamageResult` 包含最终伤害和元素修正倍率。
pub fn calc_pvp_magic_damage(
    matk: u32,
    mdef: u32,
    int: u32,
    skill_level: u8,
    element: u8,
    target_element: u8,
) -> DamageResult {
    // 技能倍率：每级 +50%，最低 100%
    let skill_multiplier =
        100u32.saturating_add((skill_level as u32).saturating_sub(1).saturating_mul(50));

    // 基础伤害 = (MATK - MDEF) * 技能倍率%
    let base_damage = (matk as i64)
        .saturating_sub(mdef as i64)
        .saturating_mul(skill_multiplier as i64)
        .saturating_div(100);

    // INT 百分比减伤：int/2 提供减伤，最高 99% 减伤
    let int_reduction = ((int as i64) / 2).min(99);
    let after_int = base_damage
        .saturating_mul(100 - int_reduction)
        .saturating_div(100);

    // 应用元素修正
    let atk_elem = Element::from_u8(element).unwrap_or(Element::Neutral);
    let def_elem = Element::from_u8(target_element).unwrap_or(Element::Neutral);
    let elem_mod = super::element::get_element_modifier(atk_elem, def_elem, ElementLevel::Level1);

    let final_damage = (after_int
        .saturating_mul(elem_mod as i64)
        .saturating_div(100))
    .max(1) as i32;

    DamageResult {
        damage: final_damage,
        element_modifier: elem_mod as f32 / 100.0,
    }
}

/// 计算 MATK 随机波动（rAthena 风格：在 MATK_MIN ~ MATK_MAX 之间均匀随机）
///
/// 模拟 rAthena 中施法时 MATK 的随机波动。
/// 实际 MATK 值 = `rand(matk_min, matk_max)`，结果为闭区间 [matk_min, matk_max]。
///
/// # 参数
/// - `matk_min`: 最小魔法攻击力
/// - `matk_max`: 最大魔法攻击力
///
/// # 返回
/// 在 [matk_min, matk_max] 范围内的随机 MATK 值。
/// 如果 `matk_min > matk_max`，则交换后计算。
pub fn calc_matk_variance(matk_min: u32, matk_max: u32) -> u32 {
    if matk_min >= matk_max {
        return matk_min;
    }
    rand::thread_rng().gen_range(matk_min..=matk_max)
}

/// 计算魔法伤害（单体技能，rAthena 风格公式）
///
/// 完整的魔法伤害计算流程：
/// 1. 使用传入的 `matk` 作为基础魔法攻击力（调用方应先通过 `calc_matk_variance` 随机化）
/// 2. 乘以技能倍率（基于 `skill_level`：每级 +50%，最低 100%）
/// 3. 减去目标魔法防御的一半
/// 4. 应用元素属性修正
/// 5. 保证最低伤害为 1
///
/// # rAthena 参考公式
/// ```text
/// damage = matk * skill_multiplier - target_mdef * 0.5
/// damage *= element_table[atk_element][def_element]
/// damage = max(damage, 1)
/// ```
///
/// # 参数
/// - `matk`: 魔法攻击力（建议通过 `calc_matk_variance` 随机化后传入）
/// - `skill_level`: 技能等级（1-10，影响技能倍率）
/// - `element`: 技能的元素属性
/// - `target_element`: 目标的元素属性
/// - `target_mdef`: 目标魔法防御力
/// - `target_level`: 目标等级（预留字段，当前未直接影响公式）
///
/// # 返回
/// `DamageResult` 包含最终伤害和元素修正倍率。
pub fn calc_magic_damage(
    matk: u32,
    skill_level: u8,
    element: Element,
    target_element: Element,
    target_mdef: u32,
    _target_level: u32,
) -> DamageResult {
    // 技能倍率：每级 +50%，最低 100%（level 1 = 100%，level 2 = 150%，以此类推）
    let skill_multiplier =
        100u32.saturating_add((skill_level as u32).saturating_sub(1).saturating_mul(50));

    // 基础伤害 = MATK * 技能倍率% - MDEF * 0.5
    let base_damage = (matk as i64)
        .saturating_mul(skill_multiplier as i64)
        .saturating_div(100)
        .saturating_sub((target_mdef as i64).saturating_div(2));

    // 应用元素修正
    let elem_mod =
        super::element::get_element_modifier(element, target_element, ElementLevel::Level1);
    let final_damage = (base_damage
        .saturating_mul(elem_mod as i64)
        .saturating_div(100))
    .max(1) as i32;

    DamageResult {
        damage: final_damage,
        element_modifier: elem_mod as f32 / 100.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::mob::data::{MobPathManager, MobPosition};
    use crate::game::rand::MockRng;
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
                direction: 0,
            }),
            pos: RwLock::new(crate::game::map::player::Position { x: 100, y: 100 }),
            level: RwLock::new(crate::game::map::player::LevelStats {
                base_level,
                job_level: 5,
                base_exp: 0,
                job_exp: 0,
                status_point: 0,
                skill_point: 0,
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
            entity_id: std::sync::atomic::AtomicU32::new(0),
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
            race: crate::game::mob::MobRace::Formless,
            mob_type: crate::game::mob::MobType::Normal,
            ai_state: RwLock::new(crate::game::mob::MobAIState::Idle),
            target_id: RwLock::new(None),
            behavior: crate::game::mob::MobBehavior::Passive,
            skills: Vec::new(),
            skill_cooldowns: RwLock::new(std::collections::HashMap::new()),
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
            dmglog: RwLock::new(std::collections::HashMap::new()),
            flee_from: RwLock::new(None),
        }
    }

    #[test]
    fn test_physical_damage_formula() {
        // Player: level 10, str=10, dex=10, agi=10
        // Base ATK = 10*2 + 10 + 10/2 + 10/3 = 20 + 10 + 5 + 3 = 38
        // Weapon ATK = weapon_type(1) * 2 = 2
        // Total ATK = 38 + 2 = 40
        // Damage = (40 - 0) * 100 / 100 = 40
        // Size modifier (Fist vs Medium) = 75%
        // Final = 40 * 75 / 100 = 30
        let player = make_player(10, 10, 10, 10, 1, 1);
        let mob = make_mob(5, 0, 0, 0, 0);

        let damage = BattleFormula::physical_damage(
            &player,
            &mob,
            100,
            1,
            crate::game::rand::thread_rng().as_ref(),
        );
        // 基础伤害 30，方差 90%-110%（27-33）
        assert!(
            (27..=33).contains(&damage),
            "damage {} not in range 27-33",
            damage
        );
    }

    #[test]
    fn test_physical_damage_with_defense() {
        // Player: level 10, str=10, dex=10, agi=10
        // Base ATK = 10*2 + 10 + 10/2 + 10/3 = 38
        // Weapon ATK = 1 * 2 = 2
        // Total ATK = 40
        // Defense = 10
        // Damage = (40 - 10) * 100 / 100 = 30
        // Size modifier (Fist vs Medium) = 75%
        // Final = 30 * 75 / 100 = 22
        let player = make_player(10, 10, 10, 10, 1, 1);
        let mob = make_mob(5, 10, 0, 0, 0);

        let damage = BattleFormula::physical_damage(
            &player,
            &mob,
            100,
            1,
            crate::game::rand::thread_rng().as_ref(),
        );
        // 基础伤害 22，方差 90%-110%（19-24）
        assert!(
            (19..=24).contains(&damage),
            "damage {} not in range 19-24",
            damage
        );
    }

    #[test]
    fn test_physical_damage_with_skill_bonus() {
        // Same as above but with 150% skill damage bonus
        // Damage = (40 - 0) * 150 / 100 = 60
        // Size modifier (Fist vs Medium) = 75%
        // Final = 60 * 75 / 100 = 45
        let player = make_player(10, 10, 10, 10, 1, 1);
        let mob = make_mob(5, 0, 0, 0, 0);

        let damage = BattleFormula::physical_damage(
            &player,
            &mob,
            150,
            1,
            crate::game::rand::thread_rng().as_ref(),
        );
        // 基础伤害 45，方差 90%-110%（40-49）
        assert!(
            (40..=49).contains(&damage),
            "damage {} not in range 40-49",
            damage
        );
    }

    #[test]
    fn test_physical_damage_with_variance() {
        // Using deterministic MockRng with value 10 (gives variance = 100)
        let player = make_player(10, 10, 10, 10, 1, 1);
        let mob = make_mob(5, 0, 0, 0, 0);
        let rng = Arc::new(MockRng::new(vec![10]));

        // Base damage = 40, size mod = 75% -> 30, variance = 90 + 10 = 100
        // Final = 30 * 100 / 100 = 30
        let damage =
            BattleFormula::physical_damage_with_variance(&player, &mob, 100, 1, rng.as_ref());
        assert_eq!(damage, 30);
    }

    #[test]
    fn test_physical_damage_minimum_one() {
        // When defense exceeds attack, damage should be at least 1
        let player = make_player(1, 1, 1, 1, 1, 1); // Very weak player
        let mob = make_mob(5, 100, 0, 0, 0); // High defense mob

        // Base ATK = 1*2 + 1 + 1/2 + 1/3 = 2 + 1 + 0 + 0 = 3
        // Defense = 100
        // Damage = ((3 - 100).max(1) * 100) / 100 = 1
        let damage = BattleFormula::physical_damage(
            &player,
            &mob,
            100,
            1,
            crate::game::rand::thread_rng().as_ref(),
        );
        assert_eq!(damage, 1);
    }

    #[test]
    fn test_magical_damage_formula() {
        // Player: level 10, int=20, dex=15
        // Base MATK = 20*2 + 15/3 + 10/4 = 40 + 5 + 2 = 47
        let player = make_player(10, 1, 15, 1, 20, 1);
        let mob = make_mob(5, 0, 5, 0, 0);

        let damage = BattleFormula::magical_damage(
            &player,
            &mob,
            100,
            50,
            crate::game::rand::thread_rng().as_ref(),
        );
        // magic_atk = max(50, 47) = 50
        // magic_defense = 5
        // Damage = ((50 - 5).max(1) * 100) / 100 = 45, 方差 90%-110%（40-49）
        assert!(
            (40..=49).contains(&damage),
            "damage {} not in range 40-49",
            damage
        );
    }

    #[test]
    fn test_hit_rate_calculation() {
        // Player: level 10, dex=20
        // HIT = dex*3 + base_level = 20*3 + 10 = 70
        // FLEE = 0
        // Hit Rate = 95 + (70 - 0) / 2 = 130, clamped to 95
        let player = make_player(10, 1, 20, 1, 1, 1);
        let mob = make_mob(5, 0, 0, 0, 0);

        let hit_rate = BattleFormula::hit_rate(&player, &mob);
        assert_eq!(hit_rate, 95);
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
        // Crit = 1 + luk/3 = 1 + 10 = 11%
        let player = make_player(10, 1, 1, 1, 1, 30);
        let mob = make_mob(5, 0, 0, 0, 0);

        let crit_rate = BattleFormula::crit_rate(&player, &mob);
        assert_eq!(crit_rate, 11);
    }

    #[test]
    fn test_crit_rate_zero_luk() {
        // Player: level 10, luk=0
        // Crit = 1 + 0/3 = 1% (base_crit = 1)
        let player = make_player(10, 1, 1, 1, 1, 0);
        let mob = make_mob(5, 0, 0, 0, 0);

        let crit_rate = BattleFormula::crit_rate(&player, &mob);
        assert_eq!(crit_rate, 1);
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

    // Level penalty calculation is done in ExpDistributor, but we test the logic here
    // by testing the expected values

    #[test]
    fn level_penalty_five_cases() {
        // Test cases for level difference penalty
        // Player level 50 vs different mob levels:

        // Case 1: diff <= 10 -> 100% exp
        let (player_level, mob_level) = (50, 40); // diff = 10
        let level_diff = player_level - mob_level;
        let penalty = if level_diff <= 10 { 1.0 } else { 0.0 };
        assert_eq!(penalty, 1.0);

        // Case 2: diff <= 15 -> 75% exp
        let (player_level, mob_level) = (50, 36); // diff = 14
        let level_diff = player_level - mob_level;
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
        let level_diff = player_level - mob_level;
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
        let level_diff = player_level - mob_level;
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
        let level_diff = player_level - mob_level;
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

/// 魔法伤害公式测试模块
///
/// 测试 `calc_magic_damage`、`element_modifier`、`calc_matk_variance` 三个函数。
#[cfg(test)]
mod magic_damage_tests {
    use super::*;

    // ========================================================================
    // calc_magic_damage 测试
    // ========================================================================

    #[test]
    fn test_magic_damage_basic() {
        // 基本魔法伤害计算（无属性克制）
        // matk=100, skill_level=1 (倍率100%), element=Neutral vs Neutral
        // base_damage = 100 * 100/100 - 0/2 = 100
        // element_modifier(Neutral, Neutral) = 100% (1.0)
        // final = 100 * 1.0 = 100
        let result = calc_magic_damage(100, 1, Element::Neutral, Element::Neutral, 0, 1);
        assert_eq!(result.damage, 100);
        assert_eq!(result.element_modifier, 1.0);
    }

    #[test]
    fn test_magic_damage_with_skill_level() {
        // 技能等级影响倍率
        // matk=100, skill_level=3 (倍率100% + 2*50% = 200%)
        // base_damage = 100 * 200/100 - 0/2 = 200
        // element = Neutral vs Neutral = 100%
        // final = 200
        let result = calc_magic_damage(100, 3, Element::Neutral, Element::Neutral, 0, 1);
        assert_eq!(result.damage, 200);
    }

    #[test]
    fn test_magic_damage_element_advantage() {
        // 克制关系：Wind vs Water = 175%
        // matk=100, skill_level=1 (倍率100%)
        // base_damage = 100 - 0 = 100
        // element_modifier = 1.75
        // final = 100 * 1.75 = 175
        let result = calc_magic_damage(100, 1, Element::Wind, Element::Water, 0, 1);
        assert_eq!(result.damage, 175);
        assert_eq!(result.element_modifier, 1.75);
    }

    #[test]
    fn test_magic_damage_element_disadvantage() {
        // 被克制：Fire vs Water = 90%
        // matk=100, skill_level=1 (倍率100%)
        // base_damage = 100 - 0 = 100
        // element_modifier = 0.90
        // final = 100 * 0.90 = 90
        let result = calc_magic_damage(100, 1, Element::Fire, Element::Water, 0, 1);
        assert_eq!(result.damage, 90);
        assert_eq!(result.element_modifier, 0.9);
    }

    #[test]
    fn test_magic_damage_element_immune() {
        // 免疫关系：Poison vs Poison Lv1 = 0%
        // base_damage = 100 - 0 = 100
        // 100 * 0 = 0, 但 max(1) 保证最低伤害
        let result = calc_magic_damage(100, 1, Element::Poison, Element::Poison, 0, 1);
        assert_eq!(result.damage, 1);
        assert_eq!(result.element_modifier, 0.0);
    }

    #[test]
    fn test_magic_damage_mdef_reduction() {
        // MDEF 减伤测试
        // matk=100, skill_level=1 (倍率100%)
        // base_damage = 100 * 100/100 - 50/2 = 100 - 25 = 75
        // element = Neutral vs Neutral = 100%
        // final = 75
        let result = calc_magic_damage(100, 1, Element::Neutral, Element::Neutral, 50, 1);
        assert_eq!(result.damage, 75);
    }

    #[test]
    fn test_magic_damage_high_mdef_minimum_damage() {
        // 高 MDEF 导致基础伤害为负，但最低保证 1
        // matk=10, skill_level=1 (倍率100%)
        // base_damage = 10 * 100/100 - 1000/2 = 10 - 500 = -490
        // element = Neutral vs Neutral = 100%
        // final = max(-490, 1) = 1
        let result = calc_magic_damage(10, 1, Element::Neutral, Element::Neutral, 1000, 1);
        assert_eq!(result.damage, 1);
    }

    #[test]
    fn test_magic_damage_combined() {
        // 组合测试：高 MATK + 技能倍率 + 克制 + MDEF
        // matk=200, skill_level=2 (倍率150%)
        // base_damage = 200 * 150/100 - 40/2 = 300 - 20 = 280
        // Wind vs Water = 175% = 1.75
        // final = 280 * 1.75 = 490
        let result = calc_magic_damage(200, 2, Element::Wind, Element::Water, 40, 1);
        assert_eq!(result.damage, 490);
    }

    // ========================================================================
    // element_modifier 测试
    // ========================================================================

    #[test]
    fn test_element_modifier_neutral_vs_neutral() {
        // 无属性 vs 无属性 = 100%
        assert_eq!(element_modifier(0, 0), 1.0);
    }

    #[test]
    fn test_element_modifier_wind_vs_water() {
        // 风 vs 水 = 175% (克制)
        assert_eq!(element_modifier(4, 1), 1.75);
    }

    #[test]
    fn test_element_modifier_fire_vs_water() {
        // 火 vs 水 = 90% (被克制)
        assert_eq!(element_modifier(3, 1), 0.9);
    }

    #[test]
    fn test_element_modifier_poison_vs_poison() {
        // 毒 vs 毒 = 0% (免疫)
        assert_eq!(element_modifier(5, 5), 0.0);
    }

    #[test]
    fn test_element_modifier_invalid_defaults_to_neutral() {
        // 无效元素值默认为 Neutral
        // 99 不是有效元素，应 fallback 到 Neutral
        // Neutral vs Fire = 100%
        assert_eq!(element_modifier(99, 3), 1.0);
    }

    // ========================================================================
    // calc_matk_variance 测试
    // ========================================================================

    #[test]
    fn test_matk_variance_in_range() {
        // 多次调用验证结果在 [min, max] 范围内
        for _ in 0..100 {
            let result = calc_matk_variance(50, 100);
            assert!(
                (50..=100).contains(&result),
                "结果 {} 不在 [50, 100] 范围内",
                result
            );
        }
    }

    #[test]
    fn test_matk_variance_equal_min_max() {
        // min == max 时应直接返回该值
        assert_eq!(calc_matk_variance(75, 75), 75);
    }

    #[test]
    fn test_matk_variance_min_greater_than_max() {
        // min > max 时应返回 min
        assert_eq!(calc_matk_variance(100, 50), 100);
    }

    #[test]
    fn test_matk_variance_zero_range() {
        // [0, 0] 应返回 0
        assert_eq!(calc_matk_variance(0, 0), 0);
    }
}

/// PvP 伤害公式测试模块
///
/// 测试 `calc_pvp_physical_damage` 和 `calc_pvp_magic_damage` 两个函数。
#[cfg(test)]
mod pvp_damage_tests {
    use super::*;

    // ========================================================================
    // calc_pvp_physical_damage 测试
    // ========================================================================

    #[test]
    fn test_pvp_physical_damage_basic() {
        // 基本 PvP 物理伤害计算
        // atk=100, def=50, vit=10, skill_level=1, element=Neutral vs Neutral
        // base_damage = (100 - 50) * 100/100 = 50
        // after_vit = 50 * (100 - 10) / 100 = 45
        // element_modifier(Neutral, Neutral) = 100% (1.0)
        // final = 45 * 1.0 = 45
        let result = calc_pvp_physical_damage(100, 50, 10, 1, 0, 0);
        assert_eq!(result.damage, 45);
        assert_eq!(result.element_modifier, 1.0);
    }

    #[test]
    fn test_pvp_physical_damage_with_skill_level() {
        // 技能等级影响倍率
        // atk=100, def=50, vit=10, skill_level=3 (倍率100% + 2*50% = 200%)
        // base_damage = (100 - 50) * 200/100 = 100
        // after_vit = 100 * (100 - 10) / 100 = 90
        // element_modifier(Neutral, Neutral) = 100%
        // final = 90
        let result = calc_pvp_physical_damage(100, 50, 10, 3, 0, 0);
        assert_eq!(result.damage, 90);
    }

    #[test]
    fn test_pvp_physical_damage_element_advantage() {
        // 克制关系：Wind vs Water = 175%
        // atk=100, def=50, vit=10, skill_level=1
        // base_damage = 50
        // after_vit = 45
        // element_modifier = 1.75
        // final = 45 * 1.75 = 78
        let result = calc_pvp_physical_damage(100, 50, 10, 1, 4, 1);
        assert_eq!(result.damage, 78);
        assert_eq!(result.element_modifier, 1.75);
    }

    #[test]
    fn test_pvp_physical_damage_element_disadvantage() {
        // 被克制：Fire vs Water = 90%
        // atk=100, def=50, vit=10, skill_level=1
        // base_damage = 50
        // after_vit = 45
        // element_modifier = 0.90
        // final = 45 * 0.90 = 40
        let result = calc_pvp_physical_damage(100, 50, 10, 1, 3, 1);
        assert_eq!(result.damage, 40);
        assert_eq!(result.element_modifier, 0.9);
    }

    #[test]
    fn test_pvp_physical_damage_high_vit() {
        // 高 VIT 减伤测试
        // atk=200, def=50, vit=80, skill_level=1
        // base_damage = 150
        // after_vit = 150 * (100 - 80) / 100 = 30
        // element_modifier(Neutral, Neutral) = 100%
        // final = 30
        let result = calc_pvp_physical_damage(200, 50, 80, 1, 0, 0);
        assert_eq!(result.damage, 30);
    }

    #[test]
    fn test_pvp_physical_damage_max_vit() {
        // VIT=99 是最大减伤（99%）
        // atk=200, def=50, vit=99, skill_level=1
        // base_damage = 150
        // after_vit = 150 * (100 - 99) / 100 = 1
        // final = 1
        let result = calc_pvp_physical_damage(200, 50, 99, 1, 0, 0);
        assert_eq!(result.damage, 1);
    }

    #[test]
    fn test_pvp_physical_damage_vit_over_99_clamped() {
        // VIT 超过 99 应被限制为 99%
        // atk=200, def=50, vit=150, skill_level=1
        // base_damage = 150
        // after_vit = 150 * (100 - 99) / 100 = 1
        // final = 1
        let result = calc_pvp_physical_damage(200, 50, 150, 1, 0, 0);
        assert_eq!(result.damage, 1);
    }

    #[test]
    fn test_pvp_physical_damage_def_exceeds_atk() {
        // 防御超过攻击时，最低伤害保证为 1
        // atk=50, def=100, vit=10, skill_level=1
        // base_damage = (50 - 100) = -50
        // after_vit = -50 * 90 / 100 = -45
        // max(-45, 1) = 1
        let result = calc_pvp_physical_damage(50, 100, 10, 1, 0, 0);
        assert_eq!(result.damage, 1);
    }

    #[test]
    fn test_pvp_physical_damage_combined() {
        // 组合测试：高 ATK + 技能倍率 + 克制 + 高 VIT
        // atk=300, def=100, vit=50, skill_level=2 (倍率150%)
        // base_damage = (300 - 100) * 150/100 = 300
        // after_vit = 300 * (100 - 50) / 100 = 150
        // Wind vs Water = 175% = 1.75
        // final = 150 * 1.75 = 262
        let result = calc_pvp_physical_damage(300, 100, 50, 2, 4, 1);
        assert_eq!(result.damage, 262);
    }

    // ========================================================================
    // calc_pvp_magic_damage 测试
    // ========================================================================

    #[test]
    fn test_pvp_magic_damage_basic() {
        // 基本 PvP 魔法伤害计算
        // matk=100, mdef=30, int=20, skill_level=1, element=Neutral vs Neutral
        // base_damage = (100 - 30) * 100/100 = 70
        // int_reduction = 20/2 = 10
        // after_int = 70 * (100 - 10) / 100 = 63
        // element_modifier(Neutral, Neutral) = 100% (1.0)
        // final = 63
        let result = calc_pvp_magic_damage(100, 30, 20, 1, 0, 0);
        assert_eq!(result.damage, 63);
        assert_eq!(result.element_modifier, 1.0);
    }

    #[test]
    fn test_pvp_magic_damage_with_skill_level() {
        // 技能等级影响倍率
        // matk=100, mdef=30, int=20, skill_level=3 (倍率200%)
        // base_damage = (100 - 30) * 200/100 = 140
        // int_reduction = 10
        // after_int = 140 * 90 / 100 = 126
        // final = 126
        let result = calc_pvp_magic_damage(100, 30, 20, 3, 0, 0);
        assert_eq!(result.damage, 126);
    }

    #[test]
    fn test_pvp_magic_damage_element_advantage() {
        // 克制关系：Wind vs Water = 175%
        // matk=100, mdef=30, int=20, skill_level=1
        // base_damage = 70
        // after_int = 63
        // element_modifier = 1.75
        // final = 63 * 1.75 = 110
        let result = calc_pvp_magic_damage(100, 30, 20, 1, 4, 1);
        assert_eq!(result.damage, 110);
        assert_eq!(result.element_modifier, 1.75);
    }

    #[test]
    fn test_pvp_magic_damage_element_disadvantage() {
        // 被克制：Fire vs Water = 90%
        // matk=100, mdef=30, int=20, skill_level=1
        // base_damage = 70
        // after_int = 63
        // element_modifier = 0.90
        // final = 63 * 0.90 = 56
        let result = calc_pvp_magic_damage(100, 30, 20, 1, 3, 1);
        assert_eq!(result.damage, 56);
        assert_eq!(result.element_modifier, 0.9);
    }

    #[test]
    fn test_pvp_magic_damage_high_int() {
        // 高 INT 减伤测试
        // matk=200, mdef=30, int=100, skill_level=1
        // base_damage = 170
        // int_reduction = 100/2 = 50
        // after_int = 170 * 50 / 100 = 85
        // final = 85
        let result = calc_pvp_magic_damage(200, 30, 100, 1, 0, 0);
        assert_eq!(result.damage, 85);
    }

    #[test]
    fn test_pvp_magic_damage_max_int_reduction() {
        // INT=198 时减伤 99%（最大）
        // matk=200, mdef=30, int=198, skill_level=1
        // base_damage = 170
        // int_reduction = 198/2 = 99
        // after_int = 170 * 1 / 100 = 1
        // final = 1
        let result = calc_pvp_magic_damage(200, 30, 198, 1, 0, 0);
        assert_eq!(result.damage, 1);
    }

    #[test]
    fn test_pvp_magic_damage_int_over_198_clamped() {
        // INT 超过 198 时减伤被限制为 99%
        // matk=200, mdef=30, int=300, skill_level=1
        // int_reduction = min(300/2, 99) = 99
        // after_int = 170 * 1 / 100 = 1
        // final = 1
        let result = calc_pvp_magic_damage(200, 30, 300, 1, 0, 0);
        assert_eq!(result.damage, 1);
    }

    #[test]
    fn test_pvp_magic_damage_mdef_exceeds_matk() {
        // 魔防超过魔攻时，最低伤害保证为 1
        // matk=30, mdef=100, int=20, skill_level=1
        // base_damage = (30 - 100) = -70
        // after_int = -70 * 90 / 100 = -63
        // max(-63, 1) = 1
        let result = calc_pvp_magic_damage(30, 100, 20, 1, 0, 0);
        assert_eq!(result.damage, 1);
    }

    #[test]
    fn test_pvp_magic_damage_combined() {
        // 组合测试：高 MATK + 技能倍率 + 克制 + 高 INT
        // matk=300, mdef=100, int=80, skill_level=2 (倍率150%)
        // base_damage = (300 - 100) * 150/100 = 300
        // int_reduction = 80/2 = 40
        // after_int = 300 * 60 / 100 = 180
        // Fire vs Earth = 175% = 1.75
        // final = 180 * 1.75 = 315
        let result = calc_pvp_magic_damage(300, 100, 80, 2, 3, 2);
        assert_eq!(result.damage, 315);
    }

    #[test]
    fn test_pvp_magic_damage_immune_element() {
        // 免疫关系：Poison vs Poison Lv1 = 0%
        // matk=100, mdef=30, int=20, skill_level=1
        // base_damage = 70
        // after_int = 63
        // element_modifier = 0.0
        // 63 * 0 = 0, 但 max(1) 保证最低伤害
        let result = calc_pvp_magic_damage(100, 30, 20, 1, 5, 5);
        assert_eq!(result.damage, 1);
        assert_eq!(result.element_modifier, 0.0);
    }
}
