//! 状态图标定义
//!
//! 客户端显示的状态效果图标ID映射

use super::types::StatusChange;

/// 状态图标信息
#[derive(Debug, Clone, Copy)]
pub struct StatusIcon {
    /// 图标ID
    pub id: u16,
    /// 图标文件名（通常为 .bmp 或 .png）
    pub filename: &'static str,
    /// 是否为负面效果（红色边框）
    pub is_negative: bool,
    /// 描述
    pub description: &'static str,
}

/// 状态图标数据库
pub struct StatusIcons;

impl StatusIcons {
    /// 根据状态类型获取图标
    pub fn get_icon(status: StatusChange) -> StatusIcon {
        match status {
            // ==================== 移动限制类 ====================
            StatusChange::Stun => StatusIcon {
                id: 1,
                filename: "stun.bmp",
                is_negative: true,
                description: "Stun - Cannot move or attack",
            },
            StatusChange::Freeze => StatusIcon {
                id: 2,
                filename: "freeze.bmp",
                is_negative: true,
                description: "Freeze - Cannot move or attack",
            },
            StatusChange::Sleep => StatusIcon {
                id: 3,
                filename: "sleep.bmp",
                is_negative: true,
                description: "Sleep - Cannot move or attack, wakes on damage",
            },
            StatusChange::Stone => StatusIcon {
                id: 4,
                filename: "stone.bmp",
                is_negative: true,
                description: "Petrify - Cannot move or attack",
            },
            StatusChange::Confusion => StatusIcon {
                id: 5,
                filename: "confusion.bmp",
                is_negative: true,
                description: "Confusion - Move randomly",
            },
            StatusChange::Hide => StatusIcon {
                id: 6,
                filename: "hide.bmp",
                is_negative: false,
                description: "Hide - Invisible to monsters",
            },
            StatusChange::Cloak => StatusIcon {
                id: 7,
                filename: "cloak.bmp",
                is_negative: false,
                description: "Cloak - Disguised",
            },

            // ==================== 攻击限制类 ====================
            StatusChange::Silence => StatusIcon {
                id: 10,
                filename: "silence.bmp",
                is_negative: true,
                description: "Silence - Cannot use skills",
            },
            StatusChange::Curse => StatusIcon {
                id: 11,
                filename: "curse.bmp",
                is_negative: true,
                description: "Curse - Reduced stats, cannot use advanced skills",
            },

            // ==================== 属性提升 ====================
            StatusChange::IncreaseStr => StatusIcon {
                id: 20,
                filename: "increase_str.bmp",
                is_negative: false,
                description: "STR Up",
            },
            StatusChange::IncreaseAgi => StatusIcon {
                id: 21,
                filename: "increase_agi.bmp",
                is_negative: false,
                description: "AGI Up",
            },
            StatusChange::IncreaseVit => StatusIcon {
                id: 22,
                filename: "increase_vit.bmp",
                is_negative: false,
                description: "VIT Up",
            },
            StatusChange::IncreaseInt => StatusIcon {
                id: 23,
                filename: "increase_int.bmp",
                is_negative: false,
                description: "INT Up",
            },
            StatusChange::IncreaseDex => StatusIcon {
                id: 24,
                filename: "increase_dex.bmp",
                is_negative: false,
                description: "DEX Up",
            },
            StatusChange::IncreaseLuk => StatusIcon {
                id: 25,
                filename: "increase_luk.bmp",
                is_negative: false,
                description: "LUK Up",
            },

            // ==================== 速度类 ====================
            StatusChange::Haste => StatusIcon {
                id: 30,
                filename: "haste.bmp",
                is_negative: false,
                description: "Haste - Increased ASPD and move speed",
            },
            StatusChange::AttackSpeedUp => StatusIcon {
                id: 31,
                filename: "aspd_up.bmp",
                is_negative: false,
                description: "Attack Speed Up",
            },
            StatusChange::MaxSpeedUp => StatusIcon {
                id: 32,
                filename: "speed_up.bmp",
                is_negative: false,
                description: "Max Speed Up",
            },
            StatusChange::Slow => StatusIcon {
                id: 33,
                filename: "slow.bmp",
                is_negative: true,
                description: "Slow - Reduced ASPD",
            },
            StatusChange::SpeedDown => StatusIcon {
                id: 34,
                filename: "speed_down.bmp",
                is_negative: true,
                description: "Speed Down - Reduced move speed",
            },

            // ==================== 祝福与集中 ====================
            StatusChange::Blessing => StatusIcon {
                id: 40,
                filename: "blessing.bmp",
                is_negative: false,
                description: "Blessing - All stats up, increased attack",
            },
            StatusChange::Concentration => StatusIcon {
                id: 41,
                filename: "concentration.bmp",
                is_negative: false,
                description: "Concentration - Increased DEX/AGI and HIT",
            },
            StatusChange::SignumCrucis => StatusIcon {
                id: 42,
                filename: "signum_crucis.bmp",
                is_negative: false,
                description: "Signum Crucis - Increased damage to Undead",
            },

            // ==================== 攻击加成 ====================
            StatusChange::PowerUp => StatusIcon {
                id: 50,
                filename: "power_up.bmp",
                is_negative: false,
                description: "Power Up - Increased ATK",
            },
            StatusChange::MagicPowerUp => StatusIcon {
                id: 51,
                filename: "magic_power_up.bmp",
                is_negative: false,
                description: "Magic Power Up - Increased MATK",
            },
            StatusChange::AtkUp => StatusIcon {
                id: 52,
                filename: "atk_up.bmp",
                is_negative: false,
                description: "ATK Up",
            },

            // ==================== 防护 ====================
            StatusChange::Shield => StatusIcon {
                id: 60,
                filename: "shield.bmp",
                is_negative: false,
                description: "Shield - Damage reduction",
            },
            StatusChange::ReflectPhysical => StatusIcon {
                id: 61,
                filename: "reflect_physical.bmp",
                is_negative: false,
                description: "Auto Guard - Reflect physical damage",
            },
            StatusChange::ReflectMagic => StatusIcon {
                id: 62,
                filename: "reflect_magic.bmp",
                is_negative: false,
                description: "Reflect Magic",
            },
            StatusChange::DefenseUp => StatusIcon {
                id: 63,
                filename: "def_up.bmp",
                is_negative: false,
                description: "DEF Up",
            },
            StatusChange::MagicDefenseUp => StatusIcon {
                id: 64,
                filename: "mdef_up.bmp",
                is_negative: false,
                description: "MDEF Up",
            },

            // ==================== 回复 ====================
            StatusChange::Regen => StatusIcon {
                id: 70,
                filename: "regen.bmp",
                is_negative: false,
                description: "Regeneration - HP recovery",
            },
            StatusChange::SpRegen => StatusIcon {
                id: 71,
                filename: "sp_regen.bmp",
                is_negative: false,
                description: "SP Recovery - SP recovery",
            },
            StatusChange::Soul => StatusIcon {
                id: 72,
                filename: "soul.bmp",
                is_negative: false,
                description: "Soul - Accelerated HP/SP recovery",
            },

            // ==================== 无敌/隐身 ====================
            StatusChange::Invincible => StatusIcon {
                id: 80,
                filename: "invincible.bmp",
                is_negative: false,
                description: "Invincible - Immune to damage",
            },
            StatusChange::Invisible => StatusIcon {
                id: 81,
                filename: "invisible.bmp",
                is_negative: false,
                description: "Invisible - Cannot be seen",
            },
            StatusChange::HolyBody => StatusIcon {
                id: 82,
                filename: "holy_body.bmp",
                is_negative: false,
                description: "Holy Body - Immune to status effects",
            },

            // ==================== 持续伤害 ====================
            StatusChange::Poison => StatusIcon {
                id: 100,
                filename: "poison.bmp",
                is_negative: true,
                description: "Poison - Continuous HP damage",
            },
            StatusChange::Bleeding => StatusIcon {
                id: 101,
                filename: "bleeding.bmp",
                is_negative: true,
                description: "Bleeding - Continuous HP damage",
            },
            StatusChange::Hunger => StatusIcon {
                id: 102,
                filename: "hunger.bmp",
                is_negative: true,
                description: "Hunger - HP/SP recovery stopped",
            },

            // ==================== 视觉/感知限制 ====================
            StatusChange::Blind => StatusIcon {
                id: 110,
                filename: "blind.bmp",
                is_negative: true,
                description: "Blind - Reduced HIT",
            },
            StatusChange::Deafness => StatusIcon {
                id: 111,
                filename: "deafness.bmp",
                is_negative: true,
                description: "Deafness - Reduced sight range",
            },
            StatusChange::Chaos => StatusIcon {
                id: 112,
                filename: "chaos.bmp",
                is_negative: true,
                description: "Chaos - Reduced HIT",
            },

            // ==================== 虚弱 ====================
            StatusChange::Weakness => StatusIcon {
                id: 130,
                filename: "weakness.bmp",
                is_negative: true,
                description: "Weakness - Reduced ATK",
            },
            StatusChange::MagicWeakness => StatusIcon {
                id: 131,
                filename: "magic_weakness.bmp",
                is_negative: true,
                description: "Magic Weakness - Reduced MATK",
            },
            StatusChange::DefenseDown => StatusIcon {
                id: 132,
                filename: "def_down.bmp",
                is_negative: true,
                description: "DEF Down",
            },
            StatusChange::MagicDefenseDown => StatusIcon {
                id: 133,
                filename: "mdef_down.bmp",
                is_negative: true,
                description: "MDEF Down",
            },

            // ==================== 通用状态 ====================
            StatusChange::Sit => StatusIcon {
                id: 1000,
                filename: "sit.bmp",
                is_negative: false,
                description: "Sitting",
            },
            StatusChange::Trade => StatusIcon {
                id: 1001,
                filename: "trade.bmp",
                is_negative: false,
                description: "Trading",
            },

            // ==================== 战斗状态 ====================
            StatusChange::Battle => StatusIcon {
                id: 1100,
                filename: "battle.bmp",
                is_negative: false,
                description: "In Combat",
            },

            // ==================== 复活/死亡保护 ====================
            StatusChange::Resurrection => StatusIcon {
                id: 1200,
                filename: "resurrection.bmp",
                is_negative: false,
                description: "Resurrection Ready",
            },
            StatusChange::DeathProtection => StatusIcon {
                id: 1201,
                filename: "death_protection.bmp",
                is_negative: false,
                description: "Death Protection",
            },

            // 默认
            _ => StatusIcon {
                id: status.icon_id(),
                filename: "unknown.bmp",
                is_negative: status.is_negative(),
                description: status.name(),
            },
        }
    }

    /// 获取所有图标（用于批量处理）
    pub fn get_all_icons() -> Vec<StatusIcon> {
        vec![
            // 可以列出所有预定义图标
            // 或者通过迭代所有 StatusChange 变体生成
        ]
    }

    /// 获取负面效果图标数量
    pub fn negative_icon_count() -> usize {
        // 统计所有负面效果
        let negatives = [
            StatusChange::Stun,
            StatusChange::Freeze,
            StatusChange::Sleep,
            StatusChange::Stone,
            StatusChange::Confusion,
            StatusChange::Silence,
            StatusChange::Curse,
            StatusChange::Slow,
            StatusChange::SpeedDown,
            StatusChange::Poison,
            StatusChange::Bleeding,
            StatusChange::Hunger,
            StatusChange::Blind,
            StatusChange::Deafness,
            StatusChange::Chaos,
            StatusChange::Weakness,
            StatusChange::MagicWeakness,
            StatusChange::DefenseDown,
            StatusChange::MagicDefenseDown,
        ];
        negatives.len()
    }
}

/// 用于网络协议的状态效果信息
#[derive(Debug, Clone)]
pub struct StatusEffectInfo {
    /// 状态类型
    pub status: StatusChange,
    /// 图标ID
    pub icon_id: u16,
    /// 剩余时间（秒）
    pub remaining_secs: u32,
    /// 是否为负面效果
    pub is_negative: bool,
}

impl StatusEffectInfo {
    /// 从 StatusEffect 创建
    pub fn from_effect(effect: &super::effect::StatusEffect) -> Self {
        let icon = StatusIcons::get_icon(effect.id);
        Self {
            status: effect.id,
            icon_id: icon.id,
            remaining_secs: (effect.remaining_ms() / 1000) as u32,
            is_negative: icon.is_negative,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_icon() {
        let icon = StatusIcons::get_icon(StatusChange::Stun);
        assert_eq!(icon.id, 1);
        assert!(icon.is_negative);
        assert_eq!(icon.filename, "stun.bmp");
    }

    #[test]
    fn test_blessing_icon() {
        let icon = StatusIcons::get_icon(StatusChange::Blessing);
        assert_eq!(icon.id, 40);
        assert!(!icon.is_negative);
    }

    #[test]
    fn test_poison_icon() {
        let icon = StatusIcons::get_icon(StatusChange::Poison);
        assert_eq!(icon.id, 100);
        assert!(icon.is_negative);
    }

    #[test]
    fn test_status_effect_info() {
        let effect = super::super::effect::StatusEffect::with_values(
            StatusChange::Blessing,
            5000,
            super::super::effect::StatusSource::Skill(1),
            10,
            0,
            0,
        );

        let info = StatusEffectInfo::from_effect(&effect);
        assert_eq!(info.status, StatusChange::Blessing);
        assert_eq!(info.icon_id, 40);
        assert!(!info.is_negative);
        assert_eq!(info.remaining_secs, 5);
    }

    #[test]
    fn test_negative_icon_count() {
        let count = StatusIcons::negative_icon_count();
        assert!(count > 0);
    }
}
