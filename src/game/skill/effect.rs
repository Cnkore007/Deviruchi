use super::data::{Skill, SkillType};
use crate::game::map::Player;
use crate::game::status::{StatusChange, StatusEffect, StatusSource};

/// 技能效果应用
pub struct SkillEffect;

impl SkillEffect {
    /// 对目标应用技能效果
    pub fn apply(skill: &Skill, caster: &Player, target: &Player, level: u8) -> SkillResult {
        match skill.type_ {
            SkillType::Attack => Self::apply_attack(skill, caster, target, level),
            SkillType::Healing => Self::apply_healing(skill, caster, target, level),
            SkillType::Support => Self::apply_support(skill, caster, target, level),
            SkillType::Debuff => Self::apply_debuff(skill, caster, target, level),
            _ => SkillResult::None,
        }
    }

    fn apply_attack(skill: &Skill, caster: &Player, _target: &Player, level: u8) -> SkillResult {
        // 基于施法者属性计算物理/魔法伤害
        let base_level = caster.base_level() as i32;
        let str = caster.str() as i32;
        let dex = caster.dex() as i32;
        let int = caster.int() as i32;

        // 基础 ATK = base_level*2 + STR + DEX/2
        let base_atk = base_level * 2 + str + dex / 2;

        // 技能倍率：默认 100% + 每级 10%，如有 damage 字段则使用
        let multiplier = if skill.damage > 0 {
            skill.damage + (level as i32 * 10)
        } else {
            100 + (level as i32 * 10)
        };

        // 区分物理/魔法：元素为 Weapon(0) 或有 hit 标记时用物理，否则用魔法
        let raw_damage = if skill.element == 0 {
            // 物理技能
            base_atk * multiplier / 100
        } else {
            // 魔法技能：MATK = INT*2 + DEX
            let matk = int * 2 + dex;
            matk * multiplier / 100
        };

        let damage = raw_damage.max(1);

        SkillResult::Damage {
            damage,
            element: skill.element,
            hit_bonus: skill.hit,
        }
    }

    fn apply_healing(skill: &Skill, caster: &Player, _target: &Player, level: u8) -> SkillResult {
        // 治疗量 = (INT + VIT/2 + base_level) * 技能倍率
        let int = caster.int() as i32;
        let vit = caster.vit() as i32;
        let base_level = caster.base_level() as i32;

        let heal_base = int + vit / 2 + base_level;
        // 倍率：默认 100% + 每级 20%
        let multiplier = if skill.damage > 0 {
            skill.damage + (level as i32 * 20)
        } else {
            100 + (level as i32 * 20)
        };

        let total_heal = (heal_base * multiplier / 100).max(1);

        SkillResult::Heal {
            amount: total_heal as u32,
        }
    }

    fn apply_support(skill: &Skill, _caster: &Player, target: &Player, level: u8) -> SkillResult {
        // 根据技能ID应用对应的增益效果
        let status_change = match skill.id {
            29 => StatusChange::IncreaseAgi, // 加速术
            34 => StatusChange::Blessing,    // 祝福
            _ => StatusChange::Blessing,     // 默认使用祝福作为通用增益
        };

        let duration_ms = skill.skill_time as u64;
        let val1 = level as i32; // 技能等级作为效果值

        let effect = StatusEffect::with_values(
            status_change,
            duration_ms,
            StatusSource::Skill(skill.id),
            val1,
            0,
            0,
        );

        target.add_status(effect);

        tracing::info!(
            "Applied {:?} buff to {} (duration: {}ms, level: {})",
            status_change,
            target.name,
            duration_ms,
            level
        );

        SkillResult::Buff {
            buff_type: skill.id,
            duration: skill.skill_time,
        }
    }

    fn apply_debuff(skill: &Skill, _caster: &Player, target: &Player, level: u8) -> SkillResult {
        // 根据技能 ID 映射到对应的减益状态效果
        let status_change = match skill.id {
            92 => StatusChange::Poison,  // 中毒术
            93 => StatusChange::Silence, // 沉默术
            94 => StatusChange::Stun,    // 眩晕攻击
            95 => StatusChange::Curse,   // 诅咒
            96 => StatusChange::Blind,   // 暗黑
            _ => StatusChange::Weakness, // 默认：虚弱（ATK 降低）
        };

        let duration_ms = skill.skill_time as u64;
        let val1 = level as i32; // 技能等级作为效果强度

        let effect = StatusEffect::with_values(
            status_change,
            duration_ms,
            StatusSource::Skill(skill.id),
            val1,
            0,
            0,
        );

        target.add_status(effect);

        tracing::info!(
            "Applied {:?} debuff to {} (duration: {}ms, level: {})",
            status_change,
            target.name,
            duration_ms,
            level
        );

        SkillResult::Debuff {
            debuff_type: skill.id,
            duration: skill.skill_time,
        }
    }
}

/// 技能效果结果
#[derive(Debug, Clone)]
pub enum SkillResult {
    None,
    Damage {
        damage: i32,
        element: u8,
        hit_bonus: i16,
    },
    Heal {
        amount: u32,
    },
    Buff {
        buff_type: u16,
        duration: u32,
    },
    Debuff {
        debuff_type: u16,
        duration: u32,
    },
}
