use super::data::{Skill, SkillType};
use crate::game::map::Player;

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

    fn apply_attack(skill: &Skill, _caster: &Player, _target: &Player, level: u8) -> SkillResult {
        // 计算伤害 (简化版，实际需要引用战斗公式)
        let base_damage = skill.damage * level as i32 / 10;
        SkillResult::Damage {
            damage: base_damage,
            element: skill.element,
            hit_bonus: skill.hit,
        }
    }

    fn apply_healing(skill: &Skill, caster: &Player, _target: &Player, level: u8) -> SkillResult {
        let heal_amount = skill.damage * level as i32 / 10;
        let matk = *caster.int.read() * 2 + *caster.dex.read();
        let total_heal = (heal_amount * matk as i32 / 100).max(1);

        SkillResult::Heal {
            amount: total_heal as u32,
        }
    }

    fn apply_support(skill: &Skill, _caster: &Player, _target: &Player, _level: u8) -> SkillResult {
        // 辅助技能效果
        SkillResult::Buff {
            buff_type: skill.id,
            duration: skill.skill_time,
        }
    }

    fn apply_debuff(skill: &Skill, _caster: &Player, _target: &Player, _level: u8) -> SkillResult {
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
