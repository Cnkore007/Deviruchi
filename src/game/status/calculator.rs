//! 状态效果属性计算器

use super::effect::StatusEffect;
use super::player_status::PlayerStatus;
use super::types::StatusChange;

/// 计算后的属性加成
#[derive(Debug, Clone, Default)]
pub struct StatModifiers {
    /// STR 加成
    pub str_bonus: i32,
    /// AGI 加成
    pub agi_bonus: i32,
    /// VIT 加成
    pub vit_bonus: i32,
    /// INT 加成
    pub int_bonus: i32,
    /// DEX 加成
    pub dex_bonus: i32,
    /// LUK 加成
    pub luk_bonus: i32,

    /// ATK 加成（固定值）
    pub atk_flat: i32,
    /// ATK 加成（百分比，100 = 100%）
    pub atk_percent: i32,

    /// MATK 加成（固定值）
    pub matk_flat: i32,
    /// MATK 加成（百分比）
    pub matk_percent: i32,

    /// DEF 加成（固定值）
    pub def_flat: i32,
    /// DEF 加成（百分比）
    pub def_percent: i32,

    /// MDEF 加成（固定值）
    pub mdef_flat: i32,
    /// MDEF 加成（百分比）
    pub mdef_percent: i32,

    /// ASPD 加成（增量）
    pub aspd_bonus: i32,

    /// 移动速度加成（增量）
    pub speed_bonus: i32,

    /// HP 回复加成（百分比）
    pub hp_regen_percent: i32,
    /// SP 回复加成（百分比）
    pub sp_regen_percent: i32,

    /// HP 每秒回复量
    pub hp_regen_per_sec: i32,
    /// SP 每秒回复量
    pub sp_regen_per_sec: i32,

    /// 命中率加成
    pub hit_bonus: i32,
    /// 回避率加成
    pub flee_bonus: i32,
    /// 暴击率加成
    pub crit_bonus: i32,

    /// 全属性加成
    pub all_stats_bonus: i32,

    /// 伤害减免（百分比）
    pub damage_reduction_percent: i32,

    /// 物理伤害反射（百分比）
    pub physical_reflect_percent: i32,
    /// 魔法伤害反射（百分比）
    pub magic_reflect_percent: i32,

    /// 元素抗性
    pub fire_resist: i32,
    pub water_resist: i32,
    pub earth_resist: i32,
    pub wind_resist: i32,
    pub holy_resist: i32,
    pub shadow_resist: i32,
}

/// 状态效果计算器
pub struct StatusCalculator;

impl StatusCalculator {
    /// 计算所有状态效果产生的属性加成
    pub fn calculate_from_status(status: &PlayerStatus) -> StatModifiers {
        let effects = status.get_all_statuses();
        let mut modifiers = StatModifiers::default();

        for effect in effects {
            Self::apply_effect(&effect, &mut modifiers);
        }

        modifiers
    }

    /// 从单个效果计算属性加成
    pub fn calculate_single(effect: &StatusEffect) -> StatModifiers {
        let mut modifiers = StatModifiers::default();
        Self::apply_effect(effect, &mut modifiers);
        modifiers
    }

    /// 应用单个效果到加成结构
    fn apply_effect(effect: &StatusEffect, modifiers: &mut StatModifiers) {
        match effect.id {
            // ==================== 属性提升类 ====================
            StatusChange::IncreaseStr => {
                modifiers.str_bonus += effect.val1;
            }
            StatusChange::IncreaseAgi => {
                modifiers.agi_bonus += effect.val1;
            }
            StatusChange::IncreaseVit => {
                modifiers.vit_bonus += effect.val1;
            }
            StatusChange::IncreaseInt => {
                modifiers.int_bonus += effect.val1;
            }
            StatusChange::IncreaseDex => {
                modifiers.dex_bonus += effect.val1;
            }
            StatusChange::IncreaseLuk => {
                modifiers.luk_bonus += effect.val1;
            }

            // ==================== 攻击加成类 ====================
            StatusChange::PowerUp => {
                // val1: ATK 加成百分比
                modifiers.atk_percent += effect.val1;
            }
            StatusChange::MagicPowerUp => {
                // val1: MATK 加成百分比
                modifiers.matk_percent += effect.val1;
            }
            StatusChange::AtkUp => {
                // val1: 固定 ATK 加成
                modifiers.atk_flat += effect.val1;
            }

            // ==================== 速度类 ====================
            StatusChange::Haste => {
                // val1: ASPD 加成 (通常为正值，如 +50)
                modifiers.aspd_bonus += effect.val1;
                // val1 也可能包含速度加成
                modifiers.speed_bonus += effect.val1;
            }
            StatusChange::AttackSpeedUp => {
                // ASPD 加成
                modifiers.aspd_bonus += effect.val1;
            }
            StatusChange::MaxSpeedUp => {
                // 移动速度加成
                modifiers.speed_bonus += effect.val1;
            }
            StatusChange::Slow => {
                // val1: ASPD 减少
                modifiers.aspd_bonus -= effect.val1;
            }
            StatusChange::SpeedDown => {
                // val1: 移动速度减少
                modifiers.speed_bonus -= effect.val1;
            }

            // ==================== 防御类 ====================
            StatusChange::Shield => {
                // val1: DEF 加成百分比
                modifiers.def_percent += effect.val1;
            }
            StatusChange::DefenseUp => {
                // val1: DEF 加成百分比
                modifiers.def_percent += effect.val1;
            }
            StatusChange::DefUp => {
                // val1: 固定 DEF 加成
                modifiers.def_flat += effect.val1;
            }
            StatusChange::DefenseDown => {
                // val1: DEF 减少百分比
                modifiers.def_percent -= effect.val1;
            }
            StatusChange::MagicDefenseUp => {
                // val1: MDEF 加成百分比
                modifiers.mdef_percent += effect.val1;
            }
            StatusChange::MagicDefenseDown => {
                // val1: MDEF 减少百分比
                modifiers.mdef_percent -= effect.val1;
            }

            // ==================== 祝福与集中 ====================
            StatusChange::Blessing => {
                // val1: 全属性加成
                modifiers.all_stats_bonus += effect.val1;
                // ATK 加成
                modifiers.atk_percent += effect.val1;
            }
            StatusChange::Concentration => {
                // val1: DEX/AGI 加成, val2: HIT 加成
                modifiers.dex_bonus += effect.val1;
                modifiers.hit_bonus += effect.val2;
            }
            StatusChange::SignumCrucis => {
                // val1: 对不死系伤害增加百分比
                // 这需要在战斗系统中处理
            }

            // ==================== 回复类 ====================
            StatusChange::Regen => {
                // val1: HP 回复百分比
                modifiers.hp_regen_percent += effect.val1;
            }
            StatusChange::SpRegen => {
                // val1: SP 回复百分比
                modifiers.sp_regen_percent += effect.val1;
            }
            StatusChange::Soul => {
                // val1: HP/SP 回复加速百分比
                modifiers.hp_regen_percent += effect.val1;
                modifiers.sp_regen_percent += effect.val1;
            }
            StatusChange::Hunger => {
                // val1: HP/SP 回复减少百分比
                modifiers.hp_regen_percent -= effect.val1;
                modifiers.sp_regen_percent -= effect.val1;
            }

            // ==================== 反射类 ====================
            StatusChange::ReflectPhysical => {
                // val1: 物理伤害反射百分比
                modifiers.physical_reflect_percent += effect.val1;
            }
            StatusChange::ReflectMagic => {
                // val1: 魔法伤害反射百分比
                modifiers.magic_reflect_percent += effect.val1;
            }

            // ==================== 命中率/回避率 ====================
            StatusChange::Blind => {
                // val1: 命中率减少
                modifiers.hit_bonus -= effect.val1;
            }
            StatusChange::Chaos => {
                // val1: 命中率减少
                modifiers.hit_bonus -= effect.val1;
            }

            // ==================== 元素抗性 ====================
            StatusChange::FireResist => {
                modifiers.fire_resist += effect.val1;
            }
            StatusChange::WaterResist => {
                modifiers.water_resist += effect.val1;
            }
            StatusChange::EarthResist => {
                modifiers.earth_resist += effect.val1;
            }
            StatusChange::WindResist => {
                modifiers.wind_resist += effect.val1;
            }
            StatusChange::HolyResist => {
                modifiers.holy_resist += effect.val1;
            }
            StatusChange::ShadowResist => {
                modifiers.shadow_resist += effect.val1;
            }

            // ==================== 伤害减免 ====================
            StatusChange::BodyDefDown => {
                // val1: 伤害减免减少
                modifiers.damage_reduction_percent -= effect.val1;
            }

            // ==================== 其他 ====================
            _ => {}
        }
    }

    /// 应用加成到基础属性
    pub fn apply_to_stats(
        base_str: u16,
        base_agi: u16,
        base_vit: u16,
        base_int: u16,
        base_dex: u16,
        base_luk: u16,
        modifiers: &StatModifiers,
    ) -> (u16, u16, u16, u16, u16, u16) {
        let str = (base_str as i32 + modifiers.str_bonus + modifiers.all_stats_bonus).max(1) as u16;
        let agi = (base_agi as i32 + modifiers.agi_bonus + modifiers.all_stats_bonus).max(1) as u16;
        let vit = (base_vit as i32 + modifiers.vit_bonus + modifiers.all_stats_bonus).max(1) as u16;
        let int = (base_int as i32 + modifiers.int_bonus + modifiers.all_stats_bonus).max(1) as u16;
        let dex = (base_dex as i32 + modifiers.dex_bonus + modifiers.all_stats_bonus).max(1) as u16;
        let luk = (base_luk as i32 + modifiers.luk_bonus + modifiers.all_stats_bonus).max(1) as u16;

        (str, agi, vit, int, dex, luk)
    }

    /// 计算实际 ASPD
    /// ASPD = 基础 ASPD + 状态加成
    pub fn calculate_aspd(base_aspd: i32, modifiers: &StatModifiers) -> i32 {
        (base_aspd + modifiers.aspd_bonus).max(0)
    }

    /// 计算实际移动速度
    pub fn calculate_speed(base_speed: i32, modifiers: &StatModifiers) -> i32 {
        (base_speed + modifiers.speed_bonus).max(1)
    }

    /// 计算 HP 回复量
    pub fn calculate_hp_regen(base_regen: i32, modifiers: &StatModifiers) -> i32 {
        let percent = 100 + modifiers.hp_regen_percent;
        (base_regen * percent) / 100 + modifiers.hp_regen_per_sec
    }

    /// 计算 SP 回复量
    pub fn calculate_sp_regen(base_regen: i32, modifiers: &StatModifiers) -> i32 {
        let percent = 100 + modifiers.sp_regen_percent;
        (base_regen * percent) / 100 + modifiers.sp_regen_per_sec
    }

    /// 计算最终 ATK
    pub fn calculate_atk(base_atk: i32, modifiers: &StatModifiers) -> i32 {
        let after_flat = base_atk + modifiers.atk_flat;
        let after_percent = (after_flat * (100 + modifiers.atk_percent)) / 100;
        after_percent.max(1)
    }

    /// 计算最终 DEF
    pub fn calculate_def(base_def: i32, modifiers: &StatModifiers) -> i32 {
        let after_flat = base_def + modifiers.def_flat;
        let after_percent = (after_flat * (100 + modifiers.def_percent)) / 100;
        after_percent.max(0)
    }

    /// 计算最终 MDEF
    pub fn calculate_mdef(base_mdef: i32, modifiers: &StatModifiers) -> i32 {
        let after_flat = base_mdef + modifiers.mdef_flat;
        let after_percent = (after_flat * (100 + modifiers.mdef_percent)) / 100;
        after_percent.max(0)
    }

    /// 获取反射伤害比例
    pub fn get_reflect_percent(effect_type: ReflectType, modifiers: &StatModifiers) -> i32 {
        match effect_type {
            ReflectType::Physical => modifiers.physical_reflect_percent,
            ReflectType::Magic => modifiers.magic_reflect_percent,
        }
    }

    /// 计算最终伤害减免
    pub fn calculate_damage_reduction(base_reduction: i32, modifiers: &StatModifiers) -> i32 {
        (base_reduction + modifiers.damage_reduction_percent)
            .max(0)
            .min(100)
    }
}

/// 反射类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectType {
    Physical,
    Magic,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_to_stats() {
        let modifiers = StatModifiers {
            str_bonus: 10,
            agi_bonus: 5,
            vit_bonus: 3,
            int_bonus: 0,
            dex_bonus: 7,
            luk_bonus: 2,
            all_stats_bonus: 0,
            ..Default::default()
        };

        let (str, agi, vit, int, dex, luk) =
            StatusCalculator::apply_to_stats(10, 10, 10, 10, 10, 10, &modifiers);

        assert_eq!(str, 20);
        assert_eq!(agi, 15);
        assert_eq!(vit, 13);
        assert_eq!(int, 10);
        assert_eq!(dex, 17);
        assert_eq!(luk, 12);
    }

    #[test]
    fn test_blessing_all_stats() {
        let modifiers = StatModifiers {
            all_stats_bonus: 5,
            atk_percent: 10,
            ..Default::default()
        };

        let (str, agi, vit, int, dex, luk) =
            StatusCalculator::apply_to_stats(10, 10, 10, 10, 10, 10, &modifiers);

        // 所有属性都应该增加5
        assert_eq!(str, 15);
        assert_eq!(agi, 15);
        assert_eq!(vit, 15);
        assert_eq!(int, 15);
        assert_eq!(dex, 15);
        assert_eq!(luk, 15);
    }

    #[test]
    fn test_calculate_aspd() {
        let modifiers = StatModifiers {
            aspd_bonus: 50,
            ..Default::default()
        };

        let aspd = StatusCalculator::calculate_aspd(100, &modifiers);
        assert_eq!(aspd, 150);
    }

    #[test]
    fn test_calculate_atk() {
        let modifiers = StatModifiers {
            atk_flat: 10,
            atk_percent: 20,
            ..Default::default()
        };

        let atk = StatusCalculator::calculate_atk(100, &modifiers);
        // (100 + 10) * 1.2 = 132
        assert_eq!(atk, 132);
    }

    #[test]
    fn test_calculate_def() {
        let modifiers = StatModifiers {
            def_flat: 5,
            def_percent: 50,
            ..Default::default()
        };

        let def = StatusCalculator::calculate_def(100, &modifiers);
        // (100 + 5) * 1.5 = 157
        assert_eq!(def, 157);
    }

    #[test]
    fn test_hp_regen_calculation() {
        let modifiers = StatModifiers {
            hp_regen_percent: 50,
            hp_regen_per_sec: 10,
            ..Default::default()
        };

        let regen = StatusCalculator::calculate_hp_regen(100, &modifiers);
        // 100 * 1.5 + 10 = 160
        assert_eq!(regen, 160);
    }

    #[test]
    fn test_calculate_damage_reduction() {
        let modifiers = StatModifiers {
            damage_reduction_percent: 30,
            ..Default::default()
        };

        let reduction = StatusCalculator::calculate_damage_reduction(10, &modifiers);
        assert_eq!(reduction, 40);
    }

    #[test]
    fn test_calculate_damage_reduction_cap() {
        let modifiers = StatModifiers {
            damage_reduction_percent: 50,
            ..Default::default()
        };

        // 不应超过100%
        let reduction = StatusCalculator::calculate_damage_reduction(60, &modifiers);
        assert_eq!(reduction, 100);
    }

    #[test]
    fn test_minimum_stat_value() {
        let modifiers = StatModifiers {
            str_bonus: -100,
            ..Default::default()
        };

        let (str, _, _, _, _, _) =
            StatusCalculator::apply_to_stats(10, 10, 10, 10, 10, 10, &modifiers);

        // 最小值应该是 1
        assert_eq!(str, 1);
    }

    #[test]
    fn test_single_effect_calculation() {
        let effect = StatusEffect::with_values(
            StatusChange::IncreaseStr,
            5000,
            super::super::effect::StatusSource::Skill(1),
            10,
            0,
            0,
        );

        let modifiers = StatusCalculator::calculate_single(&effect);
        assert_eq!(modifiers.str_bonus, 10);
    }
}
