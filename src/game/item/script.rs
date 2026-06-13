//! 物品脚本系统
//!
//! 支持 rAthena 风格的物品脚本解析和执行

use super::effect::ItemEffect;
use crate::game::status::StatusChange;

/// 物品脚本命令
#[derive(Debug, Clone)]
pub enum ItemScriptCommand {
    /// 治愈 HP/SP
    Heal { hp: i32, sp: i32 },
    /// 百分比治愈
    PercentHeal { hp_percent: i32, sp_percent: i32 },
    /// 状态治愈（无上限）
    StatusHeal { hp: i32, sp: i32 },
    /// 完全恢复
    Restore,
    /// 传送到指定位置
    Teleport { x: i32, y: i32 },
    /// 传送到指定地图
    Warp { map: String, x: i32, y: i32 },
    /// 随机传送
    RandomTeleport { range: i32 },
    /// BUFF效果
    Buff {
        status: StatusChange,
        duration_ms: u64,
        val1: i32,
        val2: i32,
        val3: i32,
    },
    /// 结束状态
    StatusEnd(StatusChange),
    /// 治愈负面状态
    Cure(Vec<StatusChange>),
    /// 复活
    Resurrection { hp_percent: u16 },
    /// 学习技能
    LearnSkill { skill_id: u16 },
    /// 使用技能
    UseSkill { skill_id: u16, level: u8 },
    /// 获得物品
    GetItem { item_id: u16, count: u16 },
    /// 随机获得物品
    GetItem2 { item_id: u16, count: u16, rate: u16 },
    /// 制作物品
    Produce { item_id: i32 },
    /// 添加金币
    Zeny { amount: i32 },
    /// 伤害HP
    Damage { amount: i32 },
    /// 隐身
    Hide,
    /// 忍耐/无敌
    Endure {
        duration_ms: u64,
        is_invincible: bool,
    },
    /// 消耗弹药
    ConsumeAmmo,
    /// 禁止野外传送
    NoPortable,
    /// 显示过场
    Cutin { filename: String },
    /// 伪装
    Disguise { mob_id: u16 },
    /// 剥离装备
    StripEquipment { slot: StripSlot },
    /// 特殊命令
    Special { command: String, args: Vec<String> },
}

/// 剥离装备的槽位
#[derive(Debug, Clone, Copy)]
pub enum StripSlot {
    Armor,
    Weapon,
    Accessory,
}

/// 物品脚本
#[derive(Debug, Clone, Default)]
pub struct ItemScript {
    pub commands: Vec<ItemScriptCommand>,
}

impl ItemScript {
    /// 从脚本字符串解析
    pub fn parse(script: &str) -> Self {
        let commands = parse_script_commands(script);
        Self { commands }
    }

    /// 执行脚本中的所有命令
    pub fn execute(&self) -> Vec<ItemEffect> {
        let mut effects = Vec::new();
        for cmd in &self.commands {
            if let Some(effect) = cmd_to_effect(cmd) {
                effects.push(effect);
            }
        }
        effects
    }

    /// 检查脚本是否为空
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// 将脚本命令转换为物品效果
fn cmd_to_effect(cmd: &ItemScriptCommand) -> Option<ItemEffect> {
    match cmd {
        ItemScriptCommand::Heal { hp, sp } => Some(ItemEffect::ItemHeal { hp: *hp, sp: *sp }),
        ItemScriptCommand::PercentHeal {
            hp_percent,
            sp_percent,
        } => Some(ItemEffect::PercentHeal {
            hp_percent: *hp_percent,
            sp_percent: *sp_percent,
        }),
        ItemScriptCommand::StatusHeal { hp, sp } => {
            Some(ItemEffect::StatusHeal { hp: *hp, sp: *sp })
        }
        ItemScriptCommand::Restore => Some(ItemEffect::Restore),
        ItemScriptCommand::Teleport { x, y } => Some(ItemEffect::Teleport {
            map: "this".to_string(),
            x: *x,
            y: *y,
        }),
        ItemScriptCommand::Warp { map, x, y } => Some(ItemEffect::Teleport {
            map: map.clone(),
            x: *x,
            y: *y,
        }),
        ItemScriptCommand::RandomTeleport { range } => {
            // 随机传送使用特殊的 ItemEffect 变体
            Some(ItemEffect::Endure {
                duration_ms: *range as u64,
                is_invincible: false,
            })
        }
        ItemScriptCommand::Buff {
            status,
            duration_ms,
            val1,
            val2,
            val3,
        } => Some(ItemEffect::StatusStart {
            status: *status,
            val1: *val1,
            val2: *val2,
            val3: *val3,
            duration_ms: *duration_ms,
        }),
        ItemScriptCommand::StatusEnd(status) => Some(ItemEffect::StatusEnd(*status)),
        ItemScriptCommand::Cure(statuses) => Some(ItemEffect::Cure(statuses.clone())),
        ItemScriptCommand::Resurrection { hp_percent } => {
            Some(ItemEffect::Resurrection(*hp_percent))
        }
        ItemScriptCommand::LearnSkill { skill_id } => Some(ItemEffect::LearnSkill(*skill_id)),
        ItemScriptCommand::UseSkill { skill_id, level } => Some(ItemEffect::UseSkill {
            skill_id: *skill_id,
            level: *level,
        }),
        ItemScriptCommand::GetItem { item_id, count } => Some(ItemEffect::GetItem {
            item_id: *item_id,
            count: *count,
            rate: 10000,
        }),
        ItemScriptCommand::GetItem2 {
            item_id,
            count,
            rate,
        } => Some(ItemEffect::GetItem {
            item_id: *item_id,
            count: *count,
            rate: *rate,
        }),
        ItemScriptCommand::Produce { item_id } => Some(ItemEffect::Produce(*item_id)),
        ItemScriptCommand::Zeny { amount } => Some(ItemEffect::AddZeny(*amount)),
        ItemScriptCommand::Damage { amount } => Some(ItemEffect::DamageHp(*amount)),
        ItemScriptCommand::Hide => Some(ItemEffect::Hide),
        ItemScriptCommand::Endure {
            duration_ms,
            is_invincible,
        } => Some(ItemEffect::Endure {
            duration_ms: *duration_ms,
            is_invincible: *is_invincible,
        }),
        ItemScriptCommand::ConsumeAmmo => Some(ItemEffect::ConsumeAmmo),
        ItemScriptCommand::NoPortable => Some(ItemEffect::NoPortable),
        ItemScriptCommand::Cutin { filename } => Some(ItemEffect::Cutin(filename.clone())),
        ItemScriptCommand::Disguise { mob_id } => Some(ItemEffect::Disguise(*mob_id)),
        ItemScriptCommand::StripEquipment { slot } => match slot {
            StripSlot::Armor => Some(ItemEffect::StripArmor),
            StripSlot::Weapon => Some(ItemEffect::StripWeapon),
            StripSlot::Accessory => Some(ItemEffect::StripAccessory),
        },
        ItemScriptCommand::Special { .. } => None,
    }
}

/// 解析脚本字符串为命令列表
fn parse_script_commands(script: &str) -> Vec<ItemScriptCommand> {
    let mut commands = Vec::new();

    // 处理多个语句（用分号分隔）
    for line in script.split(';') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // 移除注释
        let line = line.split('/').next().unwrap_or(line).trim();
        if line.is_empty() {
            continue;
        }

        // 解析命令
        if let Some(cmd) = parse_single_command(line) {
            commands.push(cmd);
        }
    }

    commands
}

/// 解析单条命令
fn parse_single_command(line: &str) -> Option<ItemScriptCommand> {
    // 提取命令名和参数
    let parts: Vec<&str> = line.split(',').collect();
    let first_part = parts.first()?.trim();

    // 进一步分割第一个部分（可能有空格分隔的命令和第一个参数）
    let cmd_parts: Vec<&str> = first_part.split_whitespace().collect();
    let cmd = cmd_parts.first()?.to_lowercase();

    // 根据命令解析
    match cmd.as_str() {
        // ==================== 治愈命令 ====================
        "itemheal" => {
            let hp = parse_int(parts.get(1).unwrap_or(&"0"));
            let sp = parse_int(parts.get(2).unwrap_or(&"0"));
            Some(ItemScriptCommand::Heal { hp, sp })
        }
        "percentheal" => {
            let hp = parse_int(parts.get(1).unwrap_or(&"0"));
            let sp = parse_int(parts.get(2).unwrap_or(&"0"));
            Some(ItemScriptCommand::PercentHeal {
                hp_percent: hp,
                sp_percent: sp,
            })
        }
        "statusheal" => {
            let hp = parse_int(parts.get(1).unwrap_or(&"0"));
            let sp = parse_int(parts.get(2).unwrap_or(&"0"));
            Some(ItemScriptCommand::StatusHeal { hp, sp })
        }
        "restore" | "heal" => Some(ItemScriptCommand::Restore),

        // ==================== 传送命令 ====================
        "warp" => {
            let map = parts.get(1).unwrap_or(&"this").trim().to_string();
            let x = parse_int(parts.get(2).unwrap_or(&"0"));
            let y = parse_int(parts.get(3).unwrap_or(&"0"));
            if map == "this" {
                Some(ItemScriptCommand::Teleport { x, y })
            } else {
                Some(ItemScriptCommand::Warp { map, x, y })
            }
        }
        "scroll" => {
            let x = parse_int(parts.get(1).unwrap_or(&"0"));
            let y = parse_int(parts.get(2).unwrap_or(&"0"));
            Some(ItemScriptCommand::Teleport { x, y })
        }
        "randomwarp" | "rndwarp" => {
            let range = parse_int(parts.get(1).unwrap_or(&"300"));
            Some(ItemScriptCommand::RandomTeleport { range })
        }

        // ==================== 状态效果命令 ====================
        "sc_start" | "sc_start2" | "sc_start4" => {
            let status = parse_status(parts.get(1).unwrap_or(&"0"))?;
            let duration_ms = parse_int(parts.get(2).unwrap_or(&"0")) as u64 * 1000;
            let val1 = parse_int(parts.get(3).unwrap_or(&"0"));
            let val2 = parse_int(parts.get(4).unwrap_or(&"0"));
            let val3 = parse_int(parts.get(5).unwrap_or(&"0"));
            Some(ItemScriptCommand::Buff {
                status,
                duration_ms,
                val1,
                val2,
                val3,
            })
        }
        "sc_end" => {
            let status = parse_status(parts.get(1).unwrap_or(&"0"))?;
            Some(ItemScriptCommand::StatusEnd(status))
        }
        "cure" => {
            let mut statuses = Vec::new();
            for part in parts.iter().skip(1) {
                if let Some(status) = parse_status(part) {
                    statuses.push(status);
                }
            }
            Some(ItemScriptCommand::Cure(statuses))
        }

        // ==================== 复活命令 ====================
        "resurrection" | "raise" => {
            let hp_percent = parse_int(parts.get(1).unwrap_or(&"50")).clamp(1, 100) as u16;
            Some(ItemScriptCommand::Resurrection { hp_percent })
        }

        // ==================== 物品生成命令 ====================
        "produce" => {
            let item_id = parse_int(parts.get(1).unwrap_or(&"0"));
            Some(ItemScriptCommand::Produce { item_id })
        }
        "getitem" => {
            let item_id = parse_int(parts.get(1).unwrap_or(&"0")) as u16;
            let count = parse_int(parts.get(2).unwrap_or(&"1")).max(1) as u16;
            Some(ItemScriptCommand::GetItem { item_id, count })
        }
        "getitem2" => {
            let item_id = parse_int(parts.get(1).unwrap_or(&"0")) as u16;
            let count = parse_int(parts.get(2).unwrap_or(&"1")).max(1) as u16;
            let rate = parse_int(parts.get(3).unwrap_or(&"10000"))
                .clamp(1, 10000) as u16;
            Some(ItemScriptCommand::GetItem2 {
                item_id,
                count,
                rate,
            })
        }

        // ==================== 技能命令 ====================
        "useskill" | "skill" => {
            let skill_id = parse_int(parts.get(1).unwrap_or(&"0")) as u16;
            let level = parse_int(parts.get(2).unwrap_or(&"1")).clamp(1, 255) as u8;
            Some(ItemScriptCommand::UseSkill { skill_id, level })
        }
        "learningskill" => {
            let skill_id = parse_int(parts.get(1).unwrap_or(&"0")) as u16;
            Some(ItemScriptCommand::LearnSkill { skill_id })
        }

        // ==================== 装备剥离命令 ====================
        "striparmor" | "strip_armor" => Some(ItemScriptCommand::StripEquipment {
            slot: StripSlot::Armor,
        }),
        "stripweapon" | "strip_weapon" => Some(ItemScriptCommand::StripEquipment {
            slot: StripSlot::Weapon,
        }),
        "stripaccessory" | "strip_accessory" => Some(ItemScriptCommand::StripEquipment {
            slot: StripSlot::Accessory,
        }),

        // ==================== 特殊效果命令 ====================
        "hide" => Some(ItemScriptCommand::Hide),
        "endure" => {
            let duration_ms = parse_int(parts.get(1).unwrap_or(&"0")) as u64 * 1000;
            let is_invincible = parse_int(parts.get(2).unwrap_or(&"0")) != 0;
            Some(ItemScriptCommand::Endure {
                duration_ms,
                is_invincible,
            })
        }
        "consumeammo" | "ammo" => Some(ItemScriptCommand::ConsumeAmmo),
        "incave" | "noportable" => Some(ItemScriptCommand::NoPortable),

        // ==================== 视觉效果命令 ====================
        "cutin" => {
            let name = parts.get(1).unwrap_or(&"").trim().to_string();
            Some(ItemScriptCommand::Cutin { filename: name })
        }
        "disguise" => {
            let mob_id = parse_int(parts.get(1).unwrap_or(&"0")) as u16;
            Some(ItemScriptCommand::Disguise { mob_id })
        }

        // ==================== 金币命令 ====================
        "zeny" => {
            let amount = parse_int(parts.get(1).unwrap_or(&"0"));
            Some(ItemScriptCommand::Zeny { amount })
        }

        // ==================== 伤害命令 ====================
        "damage" => {
            let amount = parse_int(parts.get(1).unwrap_or(&"0"));
            Some(ItemScriptCommand::Damage { amount })
        }

        _ => None,
    }
}

/// 解析整数（支持正负）
fn parse_int(s: &str) -> i32 {
    s.trim().replace("%", "").parse().unwrap_or(0)
}

/// 解析状态效果ID/名称
fn parse_status(s: &str) -> Option<StatusChange> {
    let s = s.trim().to_lowercase();

    // 尝试直接解析为数字ID
    if let Ok(id) = s.parse::<u32>() {
        return Some(StatusChange::from(id));
    }

    // 尝试解析为名称
    match s.as_str() {
        // 增益状态
        "strmode" | "increase_str" | "strup" | "str" => Some(StatusChange::IncreaseStr),
        "agistyle" | "increase_agi" | "agiup" | "agi" => Some(StatusChange::IncreaseAgi),
        "vitmode" | "increase_vit" | "vitup" | "vit" => Some(StatusChange::IncreaseVit),
        "intmode" | "increase_int" | "intup" | "int" => Some(StatusChange::IncreaseInt),
        "dexmode" | "increase_dex" | "dexup" | "dex" => Some(StatusChange::IncreaseDex),
        "lukmode" | "increase_luk" | "ukup" | "luk" => Some(StatusChange::IncreaseLuk),
        "haste" | "speedup" | "speed_up" => Some(StatusChange::Haste),
        "blessing" | "bless" => Some(StatusChange::Blessing),
        "concentration" | "concentrate" => Some(StatusChange::Concentration),
        "powerup" | "atkup" | "atk_up" => Some(StatusChange::PowerUp),
        "magicpowerup" | "matkup" | "matk_up" => Some(StatusChange::MagicPowerUp),
        "shield" | "defense" => Some(StatusChange::Shield),
        "regen" | "regeneration" => Some(StatusChange::Regen),
        "spregen" | "sp_regen" => Some(StatusChange::SpRegen),
        "invincible" | "invincibility" => Some(StatusChange::Invincible),
        "holybody" | "holy_body" => Some(StatusChange::HolyBody),

        // 减益状态
        "stun" => Some(StatusChange::Stun),
        "freeze" | "frozen" => Some(StatusChange::Freeze),
        "sleep" => Some(StatusChange::Sleep),
        "stone" | "petrify" => Some(StatusChange::Stone),
        "silence" | "mute" => Some(StatusChange::Silence),
        "curse" => Some(StatusChange::Curse),
        "poison" => Some(StatusChange::Poison),
        "bleeding" | "bleed" => Some(StatusChange::Bleeding),
        "blind" => Some(StatusChange::Blind),
        "slow" => Some(StatusChange::Slow),

        // 特殊状态
        "hide" | "cloak" => Some(StatusChange::Hide),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heal_script() {
        let script = ItemScript::parse("itemheal,50,20");
        assert_eq!(script.commands.len(), 1);
        assert!(matches!(
            &script.commands[0],
            ItemScriptCommand::Heal { hp: 50, sp: 20 }
        ));
    }

    #[test]
    fn test_parse_percent_heal() {
        let script = ItemScript::parse("percentheal,50,30");
        assert_eq!(script.commands.len(), 1);
        assert!(matches!(
            &script.commands[0],
            ItemScriptCommand::PercentHeal {
                hp_percent: 50,
                sp_percent: 30
            }
        ));
    }

    #[test]
    fn test_parse_warp() {
        let script = ItemScript::parse("warp,new_1-1,100,200");
        assert_eq!(script.commands.len(), 1);
        assert!(matches!(
            &script.commands[0],
            ItemScriptCommand::Warp {
                map,
                x: 100,
                y: 200
            } if map == "new_1-1"
        ));
    }

    #[test]
    fn test_parse_sc_start() {
        let script = ItemScript::parse("sc_start,30,60000,10");
        assert_eq!(script.commands.len(), 1);
        assert!(matches!(
            &script.commands[0],
            ItemScriptCommand::Buff {
                status: StatusChange::Haste,
                duration_ms: 60000000,
                val1: 10,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_resurrection() {
        let script = ItemScript::parse("resurrection,50");
        assert_eq!(script.commands.len(), 1);
        assert!(matches!(
            &script.commands[0],
            ItemScriptCommand::Resurrection { hp_percent: 50 }
        ));
    }

    #[test]
    fn test_parse_multi_command() {
        let script = ItemScript::parse("itemheal,50,20;percentheal,100,0;");
        assert_eq!(script.commands.len(), 2);
    }

    #[test]
    fn test_parse_with_comments() {
        let script = ItemScript::parse("itemheal,50,20 /恢复HP;percentheal,100,0");
        assert_eq!(script.commands.len(), 2);
    }

    #[test]
    fn test_parse_random_teleport() {
        let script = ItemScript::parse("randomwarp,300");
        assert_eq!(script.commands.len(), 1);
        assert!(matches!(
            &script.commands[0],
            ItemScriptCommand::RandomTeleport { range: 300 }
        ));
    }

    #[test]
    fn test_parse_useskill() {
        let script = ItemScript::parse("useskill,28,1");
        assert_eq!(script.commands.len(), 1);
        assert!(matches!(
            &script.commands[0],
            ItemScriptCommand::UseSkill {
                skill_id: 28,
                level: 1
            }
        ));
    }

    #[test]
    fn test_script_execute() {
        let script = ItemScript::parse("itemheal,100,50;");
        let effects = script.execute();
        assert!(!effects.is_empty());
    }
}
