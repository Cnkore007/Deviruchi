use crate::game::map::Player;
use crate::game::status::StatusChange;

/// 属性类型
#[derive(Debug, Clone, Copy)]
pub enum StatType {
    Str,
    Agi,
    Vit,
    Int,
    Dex,
    Luk,
    Atk,
    Matk,
    Def,
    Mdef,
    Hit,
    Flee,
    Aspd,
    Hp,
    Sp,
    MaxHp,
    MaxSp,
    Speed,
}

impl StatType {
    /// 从字符串解析属性类型
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "str" => Some(StatType::Str),
            "agi" => Some(StatType::Agi),
            "vit" => Some(StatType::Vit),
            "int" => Some(StatType::Int),
            "dex" => Some(StatType::Dex),
            "luk" => Some(StatType::Luk),
            "atk" => Some(StatType::Atk),
            "matk" => Some(StatType::Matk),
            "def" => Some(StatType::Def),
            "mdef" => Some(StatType::Mdef),
            "hit" => Some(StatType::Hit),
            "flee" => Some(StatType::Flee),
            "aspd" => Some(StatType::Aspd),
            "hp" => Some(StatType::Hp),
            "sp" => Some(StatType::Sp),
            "maxhp" => Some(StatType::MaxHp),
            "maxsp" => Some(StatType::MaxSp),
            "speed" => Some(StatType::Speed),
            _ => None,
        }
    }
}

/// 物品效果类型 - 扩展支持更多rAthena风格的效果
#[derive(Debug, Clone)]
pub enum ItemEffect {
    // ==================== 基础效果 ====================
    /// 恢复HP
    HealHp(i32),
    /// 恢复SP
    HealSp(i32),
    /// 伤害HP
    DamageHp(i32),
    /// 添加金币
    AddZeny(i32),
    /// 学习技能
    LearnSkill(u16),
    /// 打开仓库
    OpenStorage,

    // ==================== 传送/地图效果 ====================
    /// 传送 (warp)
    Teleport { map: String, x: i32, y: i32 },
    /// 传送卷轴效果
    Scroll { map: String, x: i32, y: i32 },

    // ==================== BUFF/状态效果 ====================
    /// 属性BUFF
    Buff {
        stat: StatType,
        value: i16,
        duration_secs: u32,
    },
    /// 开始状态效果 (sc_start)
    StatusStart {
        status: StatusChange,
        val1: i32,
        val2: i32,
        val3: i32,
        duration_ms: u64,
    },
    /// 结束状态效果 (sc_end)
    StatusEnd(StatusChange),

    // ==================== 治愈/恢复效果 ====================
    /// 百分比治愈 (percentheal)
    PercentHeal { hp_percent: i32, sp_percent: i32 },
    /// 固定值治愈 (itemheal)
    ItemHeal { hp: i32, sp: i32 },
    /// 状态治愈 (statusheal - 无上限治愈)
    StatusHeal { hp: i32, sp: i32 },
    /// 完全恢复 (restore)
    Restore,

    // ==================== 状态清除/恢复效果 ====================
    /// 治愈负面状态 (cure)
    Cure(Vec<StatusChange>),
    /// 复活 (resurrection)
    Resurrection(u16), // HP百分比

    // ==================== 物品生成效果 ====================
    /// 制作物品 (produce)
    Produce(i32), // item_id
    /// 随机获得物品 (getitem)
    GetItem { item_id: u16, count: u16, rate: u16 },

    // ==================== 技能效果 ====================
    /// 使用技能 (useskill)
    UseSkill { skill_id: u16, level: u8 },

    // ==================== 装备剥离效果 ====================
    /// 剥离护甲
    StripArmor,
    /// 剥离武器
    StripWeapon,
    /// 剥离饰品
    StripAccessory,

    // ==================== 特殊效果 ====================
    /// 隐身
    Hide,
    /// 忍耐/无敌 (endure)
    Endure {
        duration_ms: u64,
        is_invincible: bool,
    },
    /// 弹药消耗
    ConsumeAmmo,
    /// 禁止野外传送
    NoPortable,

    // ==================== 视觉效果/脚本效果 ====================
    /// 显示过场动画 (cutin)
    Cutin(String),
    /// 伪装/变形 (disguise)
    Disguise(u16), // mob_id
    /// 增加 karma 值
    Karma,
    /// 心灵控制 (未实现)
    MindControl,
}

impl ItemEffect {
    /// 执行效果
    pub fn apply(&self, player: &Player) -> EffectResult {
        match self {
            ItemEffect::HealHp(amount) => {
                let current = player.hp();
                let max = player.max_hp();
                let heal_amount = if *amount < 0 {
                    current.saturating_sub((-amount) as u32)
                } else {
                    (current + *amount as u32).min(max)
                };
                player.combat_mut().hp = heal_amount;
                EffectResult::Success
            }
            ItemEffect::HealSp(amount) => {
                let current = player.sp();
                let max = player.max_sp();
                let heal_amount = if *amount < 0 {
                    current.saturating_sub((-amount) as u32)
                } else {
                    (current + *amount as u32).min(max)
                };
                player.combat_mut().sp = heal_amount;
                EffectResult::Success
            }
            ItemEffect::DamageHp(amount) => {
                let current = player.hp();
                let new_hp = current.saturating_sub(*amount as u32);
                player.combat_mut().hp = new_hp;
                if new_hp == 0 {
                    // 触发死亡逻辑
                }
                EffectResult::Success
            }
            ItemEffect::AddZeny(amount) => {
                crate::game::zeny::ZenyManager::add(player, *amount as u32);
                EffectResult::Success
            }
            ItemEffect::Teleport { map: _, x: _, y: _ } => {
                // 传送逻辑将在 use_handler 中处理
                EffectResult::Success
            }
            ItemEffect::PercentHeal {
                hp_percent,
                sp_percent,
            } => {
                // 基于百分比的治疗
                let max_hp = player.max_hp();
                let max_sp = player.max_sp();
                let current_hp = player.hp();
                let current_sp = player.sp();

                let hp_heal = (max_hp as i32 * hp_percent / 100).max(0) as u32;
                let sp_heal = (max_sp as i32 * sp_percent / 100).max(0) as u32;

                player.combat_mut().hp = (current_hp + hp_heal).min(max_hp);
                player.combat_mut().sp = (current_sp + sp_heal).min(max_sp);
                EffectResult::Success
            }
            ItemEffect::ItemHeal { hp, sp } => {
                // itemheal 有上限限制 (通常是max_hp/max_sp的50%)
                let max_hp = player.max_hp();
                let max_sp = player.max_sp();
                let current_hp = player.hp();
                let current_sp = player.sp();

                let cap_hp = max_hp / 2;
                let cap_sp = max_sp / 2;

                let hp_val = *hp;
                let sp_val = *sp;
                let actual_hp = hp_val.max(-(cap_hp as i32)).min(cap_hp as i32);
                let actual_sp = sp_val.max(-(cap_sp as i32)).min(cap_sp as i32);

                let new_hp = if actual_hp >= 0 {
                    (current_hp + actual_hp as u32).min(max_hp)
                } else {
                    current_hp.saturating_sub((-actual_hp) as u32)
                };

                let new_sp = if actual_sp >= 0 {
                    (current_sp + actual_sp as u32).min(max_sp)
                } else {
                    current_sp.saturating_sub((-actual_sp) as u32)
                };

                player.combat_mut().hp = new_hp;
                player.combat_mut().sp = new_sp;
                EffectResult::Success
            }
            ItemEffect::StatusHeal { hp, sp } => {
                // 无上限治愈
                let current_hp = player.hp();
                let current_sp = player.sp();
                let max_hp = player.max_hp();
                let max_sp = player.max_sp();

                let hp_val = *hp;
                let sp_val = *sp;
                let new_hp = if hp_val >= 0 {
                    (current_hp + (hp_val as u32)).min(max_hp)
                } else {
                    current_hp.saturating_sub((-hp_val) as u32)
                };

                let new_sp = if sp_val >= 0 {
                    (current_sp + (sp_val as u32)).min(max_sp)
                } else {
                    current_sp.saturating_sub((-sp_val) as u32)
                };

                player.combat_mut().hp = new_hp.min(max_hp);
                player.combat_mut().sp = new_sp.min(max_sp);
                EffectResult::Success
            }
            ItemEffect::Restore => {
                // 完全恢复
                player.combat_mut().hp = player.max_hp();
                player.combat_mut().sp = player.max_sp();
                EffectResult::Success
            }
            ItemEffect::StatusEnd(status) => {
                // 移除状态效果
                player.status.remove_status(*status);
                EffectResult::Success
            }
            ItemEffect::Cure(statuses) => {
                // 治愈负面状态
                for status in statuses {
                    player.status.remove_status(*status);
                }
                EffectResult::Success
            }
            ItemEffect::Resurrection(hp_percent) => {
                // 复活效果，设置HP为指定百分比
                if player.hp() == 0 {
                    let max_hp = player.max_hp();
                    let res_hp = max_hp * (*hp_percent as u32) / 100;
                    player.combat_mut().hp = res_hp;
                    // 移除死亡状态
                    player.status.remove_status(StatusChange::Stone);
                }
                EffectResult::Success
            }
            _ => EffectResult::Failed(EffectError::SystemError),
        }
    }

    /// 获取效果的简要描述
    pub fn description(&self) -> String {
        match self {
            ItemEffect::HealHp(n) => format!("恢复HP: {}", n),
            ItemEffect::HealSp(n) => format!("恢复SP: {}", n),
            ItemEffect::DamageHp(n) => format!("伤害HP: {}", n),
            ItemEffect::AddZeny(n) => format!("获得金币: {}", n),
            ItemEffect::LearnSkill(id) => format!("学习技能: {}", id),
            ItemEffect::OpenStorage => "打开仓库".to_string(),
            ItemEffect::Teleport { map, x, y } => format!("传送到 {} ({},{})", map, x, y),
            ItemEffect::Scroll { map, x, y } => format!("卷轴传送到 {} ({},{})", map, x, y),
            ItemEffect::Buff {
                stat,
                value,
                duration_secs,
            } => {
                format!("{} {} 持续 {} 秒", stat_name(stat), value, duration_secs)
            }
            ItemEffect::StatusStart {
                status,
                val1,
                duration_ms,
                ..
            } => {
                format!(
                    "状态效果 {} (val1:{}) {}ms",
                    status.name(),
                    val1,
                    duration_ms
                )
            }
            ItemEffect::StatusEnd(status) => format!("移除状态: {}", status.name()),
            ItemEffect::PercentHeal {
                hp_percent,
                sp_percent,
            } => {
                format!("百分比治愈 HP:{}% SP:{}%", hp_percent, sp_percent)
            }
            ItemEffect::ItemHeal { hp, sp } => format!("治愈 HP:{} SP:{}", hp, sp),
            ItemEffect::StatusHeal { hp, sp } => format!("状态治愈 HP:{} SP:{}", hp, sp),
            ItemEffect::Restore => "完全恢复".to_string(),
            ItemEffect::Cure(statuses) => {
                format!(
                    "治愈状态: {:?}",
                    statuses.iter().map(|s| s.name()).collect::<Vec<_>>()
                )
            }
            ItemEffect::Resurrection(pct) => format!("复活 (HP {}%)", pct),
            ItemEffect::Produce(item_id) => format!("制作物品: {}", item_id),
            ItemEffect::GetItem {
                item_id,
                count,
                rate,
            } => {
                format!("随机获得物品 {}x{} ({}%几率)", item_id, count, rate)
            }
            ItemEffect::UseSkill { skill_id, level } => {
                format!("使用技能 {} (Lv{})", skill_id, level)
            }
            ItemEffect::StripArmor => "剥离护甲".to_string(),
            ItemEffect::StripWeapon => "剥离武器".to_string(),
            ItemEffect::StripAccessory => "剥离饰品".to_string(),
            ItemEffect::Hide => "隐身".to_string(),
            ItemEffect::Endure {
                duration_ms,
                is_invincible,
            } => {
                format!("忍耐 {}ms (无敌:{})", duration_ms, is_invincible)
            }
            ItemEffect::ConsumeAmmo => "消耗弹药".to_string(),
            ItemEffect::NoPortable => "禁止野外传送".to_string(),
            ItemEffect::Cutin(name) => format!("显示图像: {}", name),
            ItemEffect::Disguise(mob_id) => format!("变形为: {}", mob_id),
            ItemEffect::Karma => "增加Karma".to_string(),
            ItemEffect::MindControl => "心灵控制".to_string(),
        }
    }
}

fn stat_name(stat: &StatType) -> &'static str {
    match stat {
        StatType::Str => "STR",
        StatType::Agi => "AGI",
        StatType::Vit => "VIT",
        StatType::Int => "INT",
        StatType::Dex => "DEX",
        StatType::Luk => "LUK",
        StatType::Atk => "ATK",
        StatType::Matk => "MATK",
        StatType::Def => "DEF",
        StatType::Mdef => "MDEF",
        StatType::Hit => "HIT",
        StatType::Flee => "FLEE",
        StatType::Aspd => "ASPD",
        StatType::Hp => "HP",
        StatType::Sp => "SP",
        StatType::MaxHp => "MaxHP",
        StatType::MaxSp => "MaxSP",
        StatType::Speed => "速度",
    }
}

/// 效果执行结果
#[derive(Debug, Clone)]
pub enum EffectResult {
    Success,
    Failed(EffectError),
    PartialSuccess { msg: String },
}

/// 效果错误
#[derive(Debug, Clone, Copy)]
pub enum EffectError {
    InvalidTarget,
    CooldownNotReady,
    SkillAlreadyLearned,
    CannotUseHere,
    SystemError,
    CannotUseInThisState,
    InsufficientHp,
    InsufficientSp,
    InvalidItem,
    InventoryFull,
    NoAmmo,
    EquipmentNotFound,
}

impl std::fmt::Display for EffectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EffectError::InvalidTarget => write!(f, "无效目标"),
            EffectError::CooldownNotReady => write!(f, "冷却中"),
            EffectError::SkillAlreadyLearned => write!(f, "已学会该技能"),
            EffectError::CannotUseHere => write!(f, "无法在此处使用"),
            EffectError::SystemError => write!(f, "系统错误"),
            EffectError::CannotUseInThisState => write!(f, "当前状态无法使用"),
            EffectError::InsufficientHp => write!(f, "HP不足"),
            EffectError::InsufficientSp => write!(f, "SP不足"),
            EffectError::InvalidItem => write!(f, "无效物品"),
            EffectError::InventoryFull => write!(f, "背包已满"),
            EffectError::NoAmmo => write!(f, "没有弹药"),
            EffectError::EquipmentNotFound => write!(f, "未找到装备"),
        }
    }
}

/// 物品使用结果
#[derive(Debug, Clone)]
pub enum ItemUseResult {
    /// 使用成功
    Success,
    /// 冷却中
    Cooldown { remaining_ms: u64 },
    /// 无效目标
    InvalidTarget,
    /// 当前状态无法使用
    CannotUseInThisState,
    /// 使用失败（带原因）
    Failed(String),
}

impl ItemUseResult {
    /// 转换为布尔值（是否成功）
    pub fn is_success(&self) -> bool {
        matches!(self, ItemUseResult::Success)
    }

    /// 获取失败原因
    pub fn error_message(&self) -> Option<&str> {
        match self {
            ItemUseResult::Failed(msg) => Some(msg),
            _ => None,
        }
    }
}

/// 从脚本字符串解析效果 (rAthena风格)
pub fn parse_item_script(script: &str) -> Vec<ItemEffect> {
    let mut effects = Vec::new();

    // 处理多个语句（用分号分隔）
    for line in script.split(';') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // 解析命令
        if let Some(effect) = parse_command(line) {
            effects.push(effect);
        }
    }

    effects
}

/// 解析单条命令
fn parse_command(line: &str) -> Option<ItemEffect> {
    // 移除注释
    let line = line.split('/').next().unwrap_or(line).trim();
    if line.is_empty() {
        return None;
    }

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
            // itemheal <hp>,<sp>
            let hp = parse_int(parts.get(1).unwrap_or(&"0"));
            let sp = parse_int(parts.get(2).unwrap_or(&"0"));
            Some(ItemEffect::ItemHeal { hp, sp })
        }
        "percentheal" => {
            // percentheal <hp>,<sp>
            let hp = parse_int(parts.get(1).unwrap_or(&"0"));
            let sp = parse_int(parts.get(2).unwrap_or(&"0"));
            Some(ItemEffect::PercentHeal {
                hp_percent: hp,
                sp_percent: sp,
            })
        }
        "statusheal" => {
            // statusheal <hp>,<sp> - 无上限治愈
            let hp = parse_int(parts.get(1).unwrap_or(&"0"));
            let sp = parse_int(parts.get(2).unwrap_or(&"0"));
            Some(ItemEffect::StatusHeal { hp, sp })
        }
        "restore" => {
            // restore <hp>,<sp> - 完全恢复
            Some(ItemEffect::Restore)
        }
        "heal" => {
            // heal <hp>,<sp> - 同itemheal
            let hp = parse_int(parts.get(1).unwrap_or(&"0"));
            let sp = parse_int(parts.get(2).unwrap_or(&"0"));
            Some(ItemEffect::ItemHeal { hp, sp })
        }

        // ==================== 传送命令 ====================
        "warp" => {
            // warp <map>,<x>,<y>
            let map = parts.get(1).unwrap_or(&"this").trim().to_string();
            let x = parse_int(parts.get(2).unwrap_or(&"0"));
            let y = parse_int(parts.get(3).unwrap_or(&"0"));
            Some(ItemEffect::Teleport { map, x, y })
        }
        "scroll" => {
            // scroll <map>,<x>,<y>
            let map = parts.get(1).unwrap_or(&"this").trim().to_string();
            let x = parse_int(parts.get(2).unwrap_or(&"0"));
            let y = parse_int(parts.get(3).unwrap_or(&"0"));
            Some(ItemEffect::Scroll { map, x, y })
        }

        // ==================== 状态效果命令 ====================
        "sc_start" => {
            // sc_start <effect>,<duration>,<val1>
            // sc_start2 <effect>,<duration>,<val1>,<val2>
            // sc_start4 <effect>,<duration>,<val1>,<val2>,<val3>
            let status = parse_status(parts.get(1).unwrap_or(&"0"))?;
            let duration_ms = parse_int(parts.get(2).unwrap_or(&"0")) as u64 * 1000;
            let val1 = parse_int(parts.get(3).unwrap_or(&"0"));
            let val2 = parse_int(parts.get(4).unwrap_or(&"0"));
            let val3 = parse_int(parts.get(5).unwrap_or(&"0"));
            Some(ItemEffect::StatusStart {
                status,
                val1,
                val2,
                val3,
                duration_ms,
            })
        }
        "sc_end" => {
            // sc_end <effect>
            let status = parse_status(parts.get(1).unwrap_or(&"0"))?;
            Some(ItemEffect::StatusEnd(status))
        }
        "sc_start2" => {
            // sc_start2 <effect>,<duration>,<val1>,<val2>
            let status = parse_status(parts.get(1).unwrap_or(&"0"))?;
            let duration_ms = parse_int(parts.get(2).unwrap_or(&"0")) as u64 * 1000;
            let val1 = parse_int(parts.get(3).unwrap_or(&"0"));
            let val2 = parse_int(parts.get(4).unwrap_or(&"0"));
            Some(ItemEffect::StatusStart {
                status,
                val1,
                val2,
                val3: 0,
                duration_ms,
            })
        }
        "sc_start4" => {
            // sc_start4 <effect>,<duration>,<val1>,<val2>,<val3>
            let status = parse_status(parts.get(1).unwrap_or(&"0"))?;
            let duration_ms = parse_int(parts.get(2).unwrap_or(&"0")) as u64 * 1000;
            let val1 = parse_int(parts.get(3).unwrap_or(&"0"));
            let val2 = parse_int(parts.get(4).unwrap_or(&"0"));
            let val3 = parse_int(parts.get(5).unwrap_or(&"0"));
            Some(ItemEffect::StatusStart {
                status,
                val1,
                val2,
                val3,
                duration_ms,
            })
        }

        // ==================== 治愈状态效果 ====================
        "cure" => {
            // cure <status1>,<status2>,... - 治愈负面状态
            let mut statuses = Vec::new();
            for i in 1..parts.len() {
                if let Some(status) = parse_status(parts[i]) {
                    statuses.push(status);
                }
            }
            Some(ItemEffect::Cure(statuses))
        }

        // ==================== 复活命令 ====================
        " resurrection" | "resurrection" | "raise" => {
            // resurrection <hp_percent>
            let hp_percent = parse_int(parts.get(1).unwrap_or(&"0")).max(1).min(100) as u16;
            Some(ItemEffect::Resurrection(hp_percent))
        }

        // ==================== 物品生成命令 ====================
        "produce" => {
            // produce <item_id>
            let item_id = parse_int(parts.get(1).unwrap_or(&"0"));
            Some(ItemEffect::Produce(item_id))
        }
        "getitem" => {
            // getitem <item_id>,<count>
            let item_id = parse_int(parts.get(1).unwrap_or(&"0")) as u16;
            let count = parse_int(parts.get(2).unwrap_or(&"1")).max(1) as u16;
            let rate = 10000; // 100%
            Some(ItemEffect::GetItem {
                item_id,
                count,
                rate,
            })
        }
        "getitem2" => {
            // getitem2 <item_id>,<count>,<rate>
            let item_id = parse_int(parts.get(1).unwrap_or(&"0")) as u16;
            let count = parse_int(parts.get(2).unwrap_or(&"1")).max(1) as u16;
            let rate = parse_int(parts.get(3).unwrap_or(&"10000"))
                .max(1)
                .min(10000) as u16;
            Some(ItemEffect::GetItem {
                item_id,
                count,
                rate,
            })
        }

        // ==================== 技能命令 ====================
        "useskill" | "skill" => {
            // useskill <skill_id>,<level>
            let skill_id = parse_int(parts.get(1).unwrap_or(&"0")) as u16;
            let level = parse_int(parts.get(2).unwrap_or(&"1")).max(1).min(255) as u8;
            Some(ItemEffect::UseSkill { skill_id, level })
        }

        // ==================== 装备剥离命令 ====================
        "striparmor" | "strip armor" => Some(ItemEffect::StripArmor),
        "stripweapon" | "strip weapon" => Some(ItemEffect::StripWeapon),
        "stripaccessory" | "strip accessory" => Some(ItemEffect::StripAccessory),

        // ==================== 特殊效果命令 ====================
        "hide" => Some(ItemEffect::Hide),
        "endure" | "bonus2" if parts.len() > 2 => {
            // endure <duration>,<is_invincible>
            // bonus2 不完全是 endure，但可以部分兼容
            let duration_ms = parse_int(parts.get(1).unwrap_or(&"0")) as u64 * 1000;
            let is_invincible = parse_int(parts.get(2).unwrap_or(&"0")) != 0;
            Some(ItemEffect::Endure {
                duration_ms,
                is_invincible,
            })
        }
        "endure" => {
            // endure <duration>
            let duration_ms = parse_int(parts.get(1).unwrap_or(&"0")) as u64 * 1000;
            Some(ItemEffect::Endure {
                duration_ms,
                is_invincible: false,
            })
        }
        "consumeammo" | "ammo" => Some(ItemEffect::ConsumeAmmo),

        // ==================== 视觉效果命令 ====================
        "cutin" => {
            // cutin <filename>
            let name = parts.get(1).unwrap_or(&"").trim().to_string();
            Some(ItemEffect::Cutin(name))
        }
        "disguise" => {
            // disguise <mob_id>
            let mob_id = parse_int(parts.get(1).unwrap_or(&"0")) as u16;
            Some(ItemEffect::Disguise(mob_id))
        }

        // ==================== 其他命令 ====================
        "zeny" => {
            // zeny <amount>
            let amount = parse_int(parts.get(1).unwrap_or(&"0"));
            Some(ItemEffect::AddZeny(amount))
        }
        "damage" => {
            // damage <amount>
            let amount = parse_int(parts.get(1).unwrap_or(&"0"));
            Some(ItemEffect::DamageHp(amount))
        }
        "learningskill" => {
            // learning_skill <skill_id>
            let skill_id = parse_int(parts.get(1).unwrap_or(&"0")) as u16;
            Some(ItemEffect::LearnSkill(skill_id))
        }
        "opencart" => {
            // opencart - 打开购物车
            None // 暂不支持
        }
        "openstorage" => Some(ItemEffect::OpenStorage),
        "incave" => {
            // 禁止野外传送
            Some(ItemEffect::NoPortable)
        }
        "karma" => {
            // 增加karma值
            Some(ItemEffect::Karma)
        }
        "mind_control" => {
            // 心灵控制（未实现）
            Some(ItemEffect::MindControl)
        }

        // ==================== 兼容旧格式 ====================
        "item_heal" => {
            // item_heal <hp> <sp>
            let hp = parse_int(cmd_parts.get(1).unwrap_or(&"0"));
            let sp = parse_int(parts.get(2).unwrap_or(&"0"));
            Some(ItemEffect::ItemHeal { hp, sp })
        }

        _ => {
            // 未知命令，返回None
            None
        }
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
        return status_from_id(id);
    }

    // 尝试解析为名称
    match s.as_str() {
        // 增益状态
        "strmode" | "increase_str" | "strup" | "str" => Some(StatusChange::IncreaseStr),
        "agistyle" | "increase_agi" | "agiup" | "agi" => Some(StatusChange::IncreaseAgi),
        "vitmode" | "increase_vit" | "vitup" | "vit" => Some(StatusChange::IncreaseVit),
        "intmode" | "increase_int" | "intup" | "int" => Some(StatusChange::IncreaseInt),
        "dexmode" | "increase_dex" | "dexup" | "dex" => Some(StatusChange::IncreaseDex),
        "lukmode" | "increase_luk" | "lukup" | "luk" => Some(StatusChange::IncreaseLuk),
        "haste" | "speedup" | "speed_up" => Some(StatusChange::Haste),
        "blessing" | "bless" | "blessing_mode" => Some(StatusChange::Blessing),
        "concentration" | "concentrate" => Some(StatusChange::Concentration),
        "powerup" | "atkup" | "atk_up" => Some(StatusChange::PowerUp),
        "magicpowerup" | "matkup" | "matk_up" => Some(StatusChange::MagicPowerUp),
        "shield" | "defense" => Some(StatusChange::Shield),
        "regen" | "regeneration" | "hp_regen" => Some(StatusChange::Regen),
        "spregen" | "sp_regen" => Some(StatusChange::SpRegen),
        "invincible" | "invincibility" => Some(StatusChange::Invincible),
        "holybody" | "holy_body" => Some(StatusChange::HolyBody),
        "attack_up" => Some(StatusChange::AtkUp),
        "defup" | "def_up" => Some(StatusChange::DefUp),
        "defup2" | "def_up2" => Some(StatusChange::DefenseUp),

        // 减益状态
        "stun" => Some(StatusChange::Stun),
        "freeze" | "frozen" => Some(StatusChange::Freeze),
        "sleep" => Some(StatusChange::Sleep),
        "stone" | "petrify" => Some(StatusChange::Stone),
        "silence" | "mute" => Some(StatusChange::Silence),
        "curse" => Some(StatusChange::Curse),
        "poison" => Some(StatusChange::Poison),
        "bleeding" | "bleed" => Some(StatusChange::Bleeding),
        "hunger" => Some(StatusChange::Hunger),
        "blind" => Some(StatusChange::Blind),
        "deafness" | "deaf" => Some(StatusChange::Deafness),
        "chaos" => Some(StatusChange::Chaos),
        "slow" => Some(StatusChange::Slow),
        "speeddown" | "speed_down" => Some(StatusChange::SpeedDown),
        "weakness" | "weak" => Some(StatusChange::Weakness),
        "magicweakness" | "magic_weak" => Some(StatusChange::MagicWeakness),
        "defensedown" | "def_down" => Some(StatusChange::DefenseDown),
        "magicdefensedown" | "mdef_down" => Some(StatusChange::MagicDefenseDown),

        // 特殊状态
        "hide" | "cloack" | "cloak" => Some(StatusChange::Hide),
        "confusion" | "confuse" => Some(StatusChange::Confusion),
        "invisible" => Some(StatusChange::Invisible),

        // 元素属性
        "fire" | "fireproperty" => Some(StatusChange::FireProperty),
        "water" | "waterproperty" => Some(StatusChange::WaterProperty),
        "earth" | "earthproperty" => Some(StatusChange::EarthProperty),
        "wind" | "windproperty" => Some(StatusChange::WindProperty),
        "holy" | "holyproperty" => Some(StatusChange::HolyProperty),
        "shadow" | "shadowproperty" => Some(StatusChange::ShadowProperty),
        "ghost" | "ghostproperty" => Some(StatusChange::GhostProperty),
        "poison_property" => Some(StatusChange::PoisonProperty),

        // 元素抗性
        "fireresist" | "fire_resist" => Some(StatusChange::FireResist),
        "waterresist" | "water_resist" => Some(StatusChange::WaterResist),
        "earthresist" | "earth_resist" => Some(StatusChange::EarthResist),
        "windresist" | "wind_resist" => Some(StatusChange::WindResist),
        "holyresist" | "holy_resist" => Some(StatusChange::HolyResist),
        "shadowresist" | "shadow_resist" => Some(StatusChange::ShadowResist),

        _ => None,
    }
}

/// 从ID获取状态效果
fn status_from_id(id: u32) -> Option<StatusChange> {
    use StatusChange::*;
    match id {
        0 => Some(Sit),
        1 => Some(Trade),
        2 => Some(Stun),
        3 => Some(Freeze),
        4 => Some(Sleep),
        5 => Some(Stone),
        6 => Some(Confusion),
        7 => Some(Hide),
        8 => Some(Cloak),
        9 => Some(Silence),
        10 => Some(Curse),
        20 => Some(IncreaseStr),
        21 => Some(IncreaseAgi),
        22 => Some(IncreaseVit),
        23 => Some(IncreaseInt),
        24 => Some(IncreaseDex),
        25 => Some(IncreaseLuk),
        30 => Some(Haste),
        31 => Some(AttackSpeedUp),
        32 => Some(MaxSpeedUp),
        40 => Some(Blessing),
        41 => Some(Concentration),
        42 => Some(SignumCrucis),
        50 => Some(PowerUp),
        51 => Some(MagicPowerUp),
        60 => Some(Shield),
        61 => Some(ReflectPhysical),
        62 => Some(ReflectMagic),
        70 => Some(Regen),
        71 => Some(SpRegen),
        72 => Some(Soul),
        80 => Some(Invincible),
        81 => Some(Invisible),
        82 => Some(HolyBody),
        100 => Some(Poison),
        101 => Some(Bleeding),
        102 => Some(Hunger),
        110 => Some(Blind),
        111 => Some(Deafness),
        112 => Some(Chaos),
        120 => Some(Slow),
        121 => Some(SpeedDown),
        130 => Some(Weakness),
        131 => Some(MagicWeakness),
        132 => Some(DefenseDown),
        133 => Some(MagicDefenseDown),
        150 => Some(FireProperty),
        151 => Some(WaterProperty),
        152 => Some(EarthProperty),
        153 => Some(WindProperty),
        154 => Some(HolyProperty),
        155 => Some(ShadowProperty),
        156 => Some(GhostProperty),
        157 => Some(PoisonProperty),
        160 => Some(BodyDefDown),
        170 => Some(SoulStrike),
        180 => Some(Battle),
        181 => Some(Alert),
        182 => Some(Perception),
        200 => Some(FireResist),
        201 => Some(WaterResist),
        202 => Some(EarthResist),
        203 => Some(WindResist),
        204 => Some(HolyResist),
        205 => Some(ShadowResist),
        210 => Some(DefenseUp),
        211 => Some(MagicDefenseUp),
        220 => Some(CriticalDamage),
        221 => Some(ChaseWalk),
        230 => Some(Resurrection),
        231 => Some(DeathProtection),
        240 => Some(AtkUp),
        241 => Some(DefUp),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_item_heal() {
        let effects = parse_item_script("itemheal,50,20");
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            ItemEffect::ItemHeal { hp, sp } => {
                assert_eq!(*hp, 50);
                assert_eq!(*sp, 20);
            }
            _ => panic!("Expected ItemHeal"),
        }
    }

    #[test]
    fn test_parse_percent_heal() {
        let effects = parse_item_script("percentheal,50,30");
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            ItemEffect::PercentHeal {
                hp_percent,
                sp_percent,
            } => {
                assert_eq!(*hp_percent, 50);
                assert_eq!(*sp_percent, 30);
            }
            _ => panic!("Expected PercentHeal"),
        }
    }

    #[test]
    fn test_parse_warp() {
        let effects = parse_item_script("warp,new_1-1,100,200");
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            ItemEffect::Teleport { map, x, y } => {
                assert_eq!(map, "new_1-1");
                assert_eq!(*x, 100);
                assert_eq!(*y, 200);
            }
            _ => panic!("Expected Teleport"),
        }
    }

    #[test]
    fn test_parse_sc_start() {
        // Status ID 30 = Haste
        let effects = parse_item_script("sc_start,30,60000,10");
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            ItemEffect::StatusStart {
                status,
                val1,
                duration_ms,
                ..
            } => {
                assert_eq!(*status, StatusChange::Haste);
                assert_eq!(*val1, 10);
                assert_eq!(*duration_ms, 60000000);
            }
            _ => panic!("Expected StatusStart"),
        }
    }

    #[test]
    fn test_parse_multi_command() {
        let script = "itemheal,50,20;percentheal,100,0;";
        let effects = parse_item_script(script);
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn test_parse_getitem() {
        let effects = parse_item_script("getitem,501,10");
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            ItemEffect::GetItem {
                item_id,
                count,
                rate,
            } => {
                assert_eq!(*item_id, 501);
                assert_eq!(*count, 10);
                assert_eq!(*rate, 10000);
            }
            _ => panic!("Expected GetItem"),
        }
    }

    #[test]
    fn test_parse_useskill() {
        let effects = parse_item_script("useskill,28,1");
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            ItemEffect::UseSkill { skill_id, level } => {
                assert_eq!(*skill_id, 28);
                assert_eq!(*level, 1);
            }
            _ => panic!("Expected UseSkill"),
        }
    }

    #[test]
    fn test_parse_endure() {
        let effects = parse_item_script("endure,1000,1");
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            ItemEffect::Endure {
                duration_ms,
                is_invincible,
            } => {
                assert_eq!(*duration_ms, 1000000);
                assert!(*is_invincible);
            }
            _ => panic!("Expected Endure"),
        }
    }

    #[test]
    fn test_parse_resurrection() {
        let effects = parse_item_script("resurrection,50");
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            ItemEffect::Resurrection(hp_percent) => {
                assert_eq!(*hp_percent, 50);
            }
            _ => panic!("Expected Resurrection"),
        }
    }

    #[test]
    fn test_parse_cure() {
        let effects = parse_item_script("cure,100,101");
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            ItemEffect::Cure(statuses) => {
                assert!(statuses.contains(&StatusChange::Poison));
                assert!(statuses.contains(&StatusChange::Bleeding));
            }
            _ => panic!("Expected Cure"),
        }
    }

    #[test]
    fn test_parse_with_comments() {
        let script = "itemheal,50,20 /恢复HP和SP;percentheal,100,0";
        let effects = parse_item_script(script);
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn test_parse_unknown_command() {
        let effects = parse_item_script("unknown_command,arg1,arg2");
        assert!(effects.is_empty());
    }

    #[test]
    fn test_parse_produce() {
        let effects = parse_item_script("produce,1755");
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            ItemEffect::Produce(item_id) => {
                assert_eq!(*item_id, 1755);
            }
            _ => panic!("Expected Produce"),
        }
    }

    #[test]
    fn test_parse_disguise() {
        let effects = parse_item_script("disguise,1002");
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            ItemEffect::Disguise(mob_id) => {
                assert_eq!(*mob_id, 1002);
            }
            _ => panic!("Expected Disguise"),
        }
    }

    #[test]
    fn test_parse_restore() {
        let effects = parse_item_script("restore");
        assert_eq!(effects.len(), 1);
        assert!(matches!(&effects[0], ItemEffect::Restore));
    }

    #[test]
    fn test_effect_description() {
        let effect = ItemEffect::ItemHeal { hp: 100, sp: 50 };
        let desc = effect.description();
        assert!(desc.contains("100"));
        assert!(desc.contains("50"));
    }
}
