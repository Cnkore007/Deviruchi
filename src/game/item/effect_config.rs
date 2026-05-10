//! 物品效果配置系统
//!
//! 定义和管理物品效果配置，包括内置物品和自定义物品

use super::effect::ItemEffect;
use super::script::ItemScript;
use crate::game::status::StatusChange;
use std::collections::HashMap;

/// 物品效果类型 - 用于配置系统
#[derive(Debug, Clone)]
pub enum ItemEffectType {
    /// 传送效果
    Teleport { map: String, x: u16, y: u16 },
    /// 随机传送
    RandomTeleport { range: u16 },
    /// 传送到存档点
    SavePoint,
    /// 恢复HP
    HealHp { amount: u32 },
    /// 恢复SP
    HealSp { amount: u32 },
    /// 恢复HP和SP
    HealBoth { hp: u32, sp: u32 },
    /// 百分比恢复
    PercentHeal { hp_percent: u32, sp_percent: u32 },
    /// 学习技能
    LearnSkill { skill_id: u32 },
    /// 触发技能
    UseSkill { skill_id: u32, level: u8 },
    /// 复活
    Revive { hp_percent: u32 },
    /// 设置存档点
    SetSavePoint,
    /// 增益BUFF
    ApplyBuff {
        status: StatusChange,
        duration_secs: u64,
        val1: i32,
        val2: i32,
        val3: i32,
    },
    /// 自定义脚本
    Script { script: String },
}

impl ItemEffectType {
    /// 转换为 ItemEffect
    pub fn to_effect(&self) -> Option<ItemEffect> {
        match self {
            ItemEffectType::Teleport { map, x, y } => Some(ItemEffect::Teleport {
                map: map.clone(),
                x: *x as i32,
                y: *y as i32,
            }),
            ItemEffectType::RandomTeleport { range } => Some(ItemEffect::Endure {
                duration_ms: *range as u64,
                is_invincible: false,
            }),
            ItemEffectType::SavePoint => None, // 特殊处理
            ItemEffectType::HealHp { amount } => Some(ItemEffect::ItemHeal {
                hp: *amount as i32,
                sp: 0,
            }),
            ItemEffectType::HealSp { amount } => Some(ItemEffect::ItemHeal {
                hp: 0,
                sp: *amount as i32,
            }),
            ItemEffectType::HealBoth { hp, sp } => Some(ItemEffect::ItemHeal {
                hp: *hp as i32,
                sp: *sp as i32,
            }),
            ItemEffectType::PercentHeal {
                hp_percent,
                sp_percent,
            } => Some(ItemEffect::PercentHeal {
                hp_percent: *hp_percent as i32,
                sp_percent: *sp_percent as i32,
            }),
            ItemEffectType::LearnSkill { skill_id } => {
                Some(ItemEffect::LearnSkill(*skill_id as u16))
            }
            ItemEffectType::UseSkill { skill_id, level } => Some(ItemEffect::UseSkill {
                skill_id: *skill_id as u16,
                level: *level,
            }),
            ItemEffectType::Revive { hp_percent } => {
                Some(ItemEffect::Resurrection(*hp_percent as u16))
            }
            ItemEffectType::SetSavePoint => None, // 特殊处理
            ItemEffectType::ApplyBuff {
                status,
                duration_secs,
                val1,
                val2,
                val3,
            } => Some(ItemEffect::StatusStart {
                status: *status,
                val1: *val1,
                val2: *val2,
                val3: *val3,
                duration_ms: duration_secs * 1000,
            }),
            ItemEffectType::Script { script } => {
                let parsed = ItemScript::parse(script);
                parsed.execute().into_iter().next()
            }
        }
    }

    /// 获取效果描述
    pub fn description(&self) -> String {
        match self {
            ItemEffectType::Teleport { map, x, y } => {
                format!("传送到 {} ({}, {})", map, x, y)
            }
            ItemEffectType::RandomTeleport { range } => {
                format!("随机传送到 {} 范围内", range)
            }
            ItemEffectType::SavePoint => "传送到存档点".to_string(),
            ItemEffectType::HealHp { amount } => format!("恢复 HP: {}", amount),
            ItemEffectType::HealSp { amount } => format!("恢复 SP: {}", amount),
            ItemEffectType::HealBoth { hp, sp } => {
                format!("恢复 HP: {}, SP: {}", hp, sp)
            }
            ItemEffectType::PercentHeal {
                hp_percent,
                sp_percent,
            } => {
                format!("百分比恢复 HP: {}%, SP: {}%", hp_percent, sp_percent)
            }
            ItemEffectType::LearnSkill { skill_id } => {
                format!("学习技能 ID: {}", skill_id)
            }
            ItemEffectType::UseSkill { skill_id, level } => {
                format!("使用技能 {} Lv{}", skill_id, level)
            }
            ItemEffectType::Revive { hp_percent } => format!("复活 HP {}%", hp_percent),
            ItemEffectType::SetSavePoint => "设置存档点".to_string(),
            ItemEffectType::ApplyBuff {
                status,
                duration_secs,
                ..
            } => {
                format!("获得 {} 状态 {} 秒", status.name(), duration_secs)
            }
            ItemEffectType::Script { .. } => "执行脚本".to_string(),
        }
    }
}

/// 物品使用需求
#[derive(Debug, Clone, Default)]
pub struct ItemRequirements {
    /// 最低等级要求
    pub min_level: Option<u16>,
    /// 允许的职业列表
    pub job_ids: Option<Vec<u16>>,
    /// 必须存活
    pub must_be_alive: bool,
    /// 必须死亡
    pub must_be_dead: bool,
    /// 不能在坐下状态使用
    pub no_sitting: bool,
    /// 不能在战斗状态使用
    pub no_battle: bool,
    /// 必须在特定地图
    pub required_map: Option<String>,
}

impl ItemRequirements {
    /// 检查需求是否满足
    pub fn check(&self, player: &crate::game::map::Player) -> Option<String> {
        // 检查等级
        if let Some(min_level) = self.min_level
            && player.base_level() < min_level
        {
            return Some(format!("需要等级 {}", min_level));
        }

        // 检查职业
        if let Some(required_jobs) = &self.job_ids {
            let player_job = player.job();
            if !required_jobs.contains(&player_job) {
                return Some("此物品需要特定职业才能使用".to_string());
            }
        }

        // 检查存活状态
        if self.must_be_alive && player.hp() == 0 {
            return Some("死亡状态无法使用".to_string());
        }

        // 检查死亡状态
        if self.must_be_dead && player.hp() != 0 {
            return Some("必须在死亡状态使用".to_string());
        }

        // 检查坐下状态
        if self.no_sitting && player.status.has_status(StatusChange::Sit) {
            return Some("坐下状态无法使用".to_string());
        }

        // 检查战斗状态限制
        if self.no_battle && player.is_in_combat() {
            return Some("战斗状态无法使用此物品".to_string());
        }

        // 检查地图
        if let Some(ref map) = self.required_map
            && &player.map_name != map
        {
            return Some(format!("需要在 {} 地图使用", map));
        }

        None
    }
}

/// 物品效果配置
#[derive(Debug, Clone)]
pub struct ItemEffectConfig {
    /// 物品ID
    pub item_id: u16,
    /// 物品名称
    pub name: &'static str,
    /// 效果类型
    pub effect_type: ItemEffectType,
    /// 使用需求
    pub requirements: ItemRequirements,
    /// 冷却时间（毫秒）
    pub cooldown_ms: u64,
    /// 使用消息
    pub use_message: &'static str,
}

impl ItemEffectConfig {
    /// 创建新配置
    pub fn new(item_id: u16, name: &'static str, effect_type: ItemEffectType) -> Self {
        Self {
            item_id,
            name,
            effect_type,
            requirements: ItemRequirements::default(),
            cooldown_ms: 0,
            use_message: "使用成功",
        }
    }

    /// 设置冷却时间
    pub fn with_cooldown(mut self, cooldown_ms: u64) -> Self {
        self.cooldown_ms = cooldown_ms;
        self
    }

    /// 设置需求
    pub fn with_requirements(mut self, requirements: ItemRequirements) -> Self {
        self.requirements = requirements;
        self
    }

    /// 设置使用消息
    pub fn with_message(mut self, msg: &'static str) -> Self {
        self.use_message = msg;
        self
    }
}

/// 物品效果数据库
#[derive(Debug, Clone)]
pub struct ItemEffectDatabase {
    effects: HashMap<u16, ItemEffectConfig>,
}

impl ItemEffectDatabase {
    /// 创建新的物品效果数据库
    pub fn new() -> Self {
        let mut db = Self {
            effects: HashMap::new(),
        };
        db.load_default_effects();
        db
    }

    /// 获取物品效果配置
    pub fn get(&self, item_id: u16) -> Option<&ItemEffectConfig> {
        self.effects.get(&item_id)
    }

    /// 注册自定义物品效果
    pub fn register(&mut self, config: ItemEffectConfig) {
        self.effects.insert(config.item_id, config);
    }

    /// 加载内置物品效果配置
    pub fn load_default_effects(&mut self) {
        // ==================== 传送卷轴 ====================

        // 蝴蝶翅膀 - 传送到存档点
        self.register(
            ItemEffectConfig::new(602, "Butterfly Wing", ItemEffectType::SavePoint)
                .with_cooldown(10000) // 10秒冷却
                .with_message("已传送到存档点"),
        );

        // 飞翔之翼 - 随机传送
        self.register(
            ItemEffectConfig::new(
                601,
                "Fly Wing",
                ItemEffectType::RandomTeleport { range: 300 },
            )
            .with_cooldown(5000) // 5秒冷却
            .with_message("随机传送中..."),
        );

        // ==================== 药水类 ====================

        // 红色药水
        self.register(
            ItemEffectConfig::new(604, "Red Potion", ItemEffectType::HealHp { amount: 50 })
                .with_message("HP +50"),
        );

        // 橙色药水
        self.register(
            ItemEffectConfig::new(605, "Orange Potion", ItemEffectType::HealHp { amount: 100 })
                .with_message("HP +100"),
        );

        // 黄色药水
        self.register(
            ItemEffectConfig::new(606, "Yellow Potion", ItemEffectType::HealHp { amount: 150 })
                .with_message("HP +150"),
        );

        // 白色药水
        self.register(
            ItemEffectConfig::new(607, "White Potion", ItemEffectType::HealHp { amount: 200 })
                .with_message("HP +200"),
        );

        // 蓝色药水
        self.register(
            ItemEffectConfig::new(608, "Blue Potion", ItemEffectType::HealSp { amount: 30 })
                .with_message("SP +30"),
        );

        // 绿色药水
        self.register(
            ItemEffectConfig::new(
                609,
                "Green Potion",
                ItemEffectType::HealBoth { hp: 50, sp: 10 },
            )
            .with_message("HP +50, SP +10"),
        );

        // ==================== 增益药水 ====================

        // 觉醒药水 - 敏捷提升
        self.register(
            ItemEffectConfig::new(
                610,
                "Awakening Potion",
                ItemEffectType::ApplyBuff {
                    status: StatusChange::IncreaseAgi,
                    duration_secs: 300,
                    val1: 10,
                    val2: 0,
                    val3: 0,
                },
            )
            .with_message("AGI +10 (5分钟)"),
        );

        // 祝福药水 - 幸运提升
        self.register(
            ItemEffectConfig::new(
                611,
                "Blessing Potion",
                ItemEffectType::ApplyBuff {
                    status: StatusChange::IncreaseLuk,
                    duration_secs: 300,
                    val1: 10,
                    val2: 0,
                    val3: 0,
                },
            )
            .with_message("LUK +10 (5分钟)"),
        );

        // ==================== 复活道具 ====================

        // 阿鲁纳
        self.register(
            ItemEffectConfig::new(1202, "Anodyne", ItemEffectType::Revive { hp_percent: 50 })
                .with_message("已复活"),
        );

        // 神圣之复活 (使用不同的物品ID)
        self.register(
            ItemEffectConfig::new(
                1223,
                "Seed of Life",
                ItemEffectType::Revive { hp_percent: 100 },
            )
            .with_message("完全复活"),
        );

        // ==================== 食物类 ====================

        // 苹果
        self.register(
            ItemEffectConfig::new(1219, "Apple", ItemEffectType::HealHp { amount: 30 })
                .with_message("HP +30"),
        );

        // 香蕉
        self.register(
            ItemEffectConfig::new(1220, "Banana", ItemEffectType::HealHp { amount: 20 })
                .with_message("HP +20"),
        );

        // 葡萄
        self.register(
            ItemEffectConfig::new(1221, "Grape", ItemEffectType::HealBoth { hp: 40, sp: 5 })
                .with_message("HP +40, SP +5"),
        );

        // 胡萝卜
        self.register(
            ItemEffectConfig::new(
                1222,
                "Carrot",
                ItemEffectType::PercentHeal {
                    hp_percent: 10,
                    sp_percent: 5,
                },
            )
            .with_message("HP +10%, SP +5%"),
        );

        // ==================== 技能学习 ====================

        // 老旧蓝色盒子 - 学习火球术
        self.register(
            ItemEffectConfig::new(
                603,
                "Old Blue Box",
                ItemEffectType::LearnSkill { skill_id: 1 },
            )
            .with_message("学会了火球术！"),
        );

        // ==================== 其他常用物品 ====================

        // 玉 (注意：1202已用于Anodyne复活，这里使用1221)
        self.register(
            ItemEffectConfig::new(1221, "Jade", ItemEffectType::HealBoth { hp: 100, sp: 50 })
                .with_message("HP +100, SP +50"),
        );

        // 蓝宝石
        self.register(
            ItemEffectConfig::new(1203, "Blue Gemstone", ItemEffectType::HealSp { amount: 50 })
                .with_message("SP +50"),
        );

        // 红宝石
        self.register(
            ItemEffectConfig::new(1204, "Red Gemstone", ItemEffectType::HealHp { amount: 100 })
                .with_message("HP +100"),
        );

        // 蜂蜜
        self.register(
            ItemEffectConfig::new(
                1205,
                "Honey",
                ItemEffectType::PercentHeal {
                    hp_percent: 30,
                    sp_percent: 20,
                },
            )
            .with_message("HP +30%, SP +20%"),
        );
    }
}

impl Default for ItemEffectDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_effect() {
        let db = ItemEffectDatabase::new();

        // 测试蝴蝶翅膀
        let effect = db.get(602);
        assert!(effect.is_some());
        assert!(matches!(
            effect.unwrap().effect_type,
            ItemEffectType::SavePoint
        ));

        // 测试红色药水
        let effect = db.get(604);
        assert!(effect.is_some());
        assert!(matches!(
            effect.unwrap().effect_type,
            ItemEffectType::HealHp { amount: 50 }
        ));
    }

    #[test]
    fn test_register_custom_effect() {
        let mut db = ItemEffectDatabase::new();

        db.register(
            ItemEffectConfig::new(9999, "Custom Item", ItemEffectType::HealHp { amount: 999 })
                .with_message("自定义物品"),
        );

        let effect = db.get(9999);
        assert!(effect.is_some());
        assert_eq!(effect.unwrap().name, "Custom Item");
    }

    #[test]
    fn test_effect_type_description() {
        let effect_type = ItemEffectType::Teleport {
            map: "new_1-1".to_string(),
            x: 100,
            y: 200,
        };
        assert!(effect_type.description().contains("new_1-1"));
    }

    #[test]
    fn test_requirements_check() {
        use crate::game::map::Player;

        // 创建测试角色
        let char_data = crate::storage::Character {
            char_id: 1,
            char_num: 0,
            name: "TestPlayer".to_string(),
            class: 0,
            base_level: 10,
            job_level: 1,
            base_exp: 0,
            job_exp: 0,
            hp: 1000,
            max_hp: 1000,
            sp: 500,
            max_sp: 500,
            str: 10,
            agi: 10,
            vit: 10,
            int: 10,
            dex: 10,
            luk: 10,
            zeny: 1000,
            hair: 0,
            hair_color: 0,
            clothes_color: 0,
            weapon: 0,
            shield: 0,
            head_top: 0,
            head_mid: 0,
            head_bottom: 0,
            last_map: "new_1-1.gat".to_string(),
            last_x: 53,
            last_y: 111,
            save_map: "new_1-1.gat".to_string(),
            save_x: 53,
            save_y: 111,
            delete_timer: 0,
            status_point: 0,
            skill_point: 0,
            created_at: 0,
            updated_at: 0,
        };

        let player = Player::from_character(char_data);

        // 测试等级要求
        let requirements = ItemRequirements {
            min_level: Some(5),
            ..Default::default()
        };
        assert!(requirements.check(&player).is_none());

        let requirements = ItemRequirements {
            min_level: Some(20),
            ..Default::default()
        };
        assert!(requirements.check(&player).is_some());
    }
}
