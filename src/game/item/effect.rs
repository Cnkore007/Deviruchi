use crate::game::map::Player;

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

/// 物品效果类型
#[derive(Debug, Clone)]
pub enum ItemEffect {
    HealHp(u16),
    HealSp(u16),
    DamageHp(u16),
    Teleport { map: String, x: u16, y: u16 },
    Buff {
        stat: StatType,
        value: i16,
        duration_secs: u32,
    },
    AddZeny(u32),
    LearnSkill(u16),
    OpenStorage,
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
}

impl ItemEffect {
    /// 执行效果
    pub fn apply(&self, player: &Player) -> EffectResult {
        match self {
            ItemEffect::HealHp(amount) => {
                let current = *player.hp.read();
                let max = *player.max_hp.read();
                let new_hp = (current + *amount as u32).min(max);
                *player.hp.write() = new_hp;
                EffectResult::Success
            }
            ItemEffect::HealSp(amount) => {
                let current = *player.sp.read();
                let max = *player.max_sp.read();
                let new_sp = (current + *amount as u32).min(max);
                *player.sp.write() = new_sp;
                EffectResult::Success
            }
            ItemEffect::AddZeny(amount) => {
                crate::game::zeny::ZenyManager::add(player, *amount);
                EffectResult::Success
            }
            ItemEffect::DamageHp(amount) => {
                let current = *player.hp.read();
                let new_hp = current.saturating_sub(*amount as u32);
                *player.hp.write() = new_hp;
                EffectResult::Success
            }
            _ => EffectResult::Failed(EffectError::SystemError),
        }
    }
}

/// 从脚本字符串解析效果
pub fn parse_item_script(script: &str) -> Vec<ItemEffect> {
    let mut effects = Vec::new();
    for line in script.split(';') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "item_heal" => {
                let hp = parts
                    .get(1)
                    .and_then(|s| s.trim_end_matches(',').parse().ok())
                    .unwrap_or(0);
                let sp = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
                if hp > 0 {
                    effects.push(ItemEffect::HealHp(hp));
                }
                if sp > 0 {
                    effects.push(ItemEffect::HealSp(sp));
                }
            }
            "zeny" => {
                let amount = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                effects.push(ItemEffect::AddZeny(amount));
            }
            "damage" => {
                let amount = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                effects.push(ItemEffect::DamageHp(amount));
            }
            _ => {} // 更多指令后续添加
        }
    }
    effects
}
