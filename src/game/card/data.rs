use serde::{Deserialize, Serialize};

/// 卡片效果类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CardEffect {
    /// 增加属性 (stat, value)
    AddStat { stat: CardStat, value: i32 },
    /// 增加伤害对某种族 (race, percent%)
    IncreaseDamage { race: MonsterRace, percent: i32 },
    /// 减少伤害来自某种族
    ReduceDamage { race: MonsterRace, percent: i32 },
    /// 增加伤害对某元素
    IncreaseElementDamage { element: u8, percent: i32 },
    /// 减少伤害来自某元素
    ReduceElementDamage { element: u8, percent: i32 },
    /// 增加攻击力百分比
    IncreaseAtkPercent(i32),
    /// 增加魔法攻击力百分比
    IncreaseMatkPercent(i32),
    /// 增加最大HP百分比
    IncreaseMaxHpPercent(i32),
    /// 增加最大SP百分比
    IncreaseMaxSpPercent(i32),
    /// 增加命中
    AddHit(i32),
    /// 增加回避
    AddFlee(i32),
    /// 增加暴击率
    AddCrit(i32),
    /// 增加攻速百分比
    IncreaseAspdPercent(i32),
    /// 无视防御百分比
    IgnoreDefPercent(i32),
    /// 无视魔法防御百分比
    IgnoreMdefPercent(i32),
    /// 技能伤害增加 (skill_id, percent%)
    IncreaseSkillDamage { skill_id: u16, percent: i32 },
    /// 技能冷却减少 (skill_id, percent%)
    ReduceSkillCooldown { skill_id: u16, percent: i32 },
    /// 物理攻击时概率触发技能 (skill_id, chance_percent, level)
    AutoSpellOnAttack { skill_id: u16, chance: i32, level: u8 },
    /// 被攻击时概率触发技能
    AutoSpellOnHit { skill_id: u16, chance: i32, level: u8 },
    /// 使武器附魔 (element)
    EnchantWeapon { element: u8 },
    /// 使护甲附魔
    EnchantArmor { element: u8 },
    /// 防止武器被卸除
    PreventStripWeapon,
    /// 防止护甲被卸除
    PreventStripArmor,
    /// 防止头盔被卸除
    PreventStripHelm,
    /// 免疫指定状态
    ImmuneStatus { status_id: u16 },
    /// 获得指定技能 (skill_id, level)
    GrantSkill { skill_id: u16, level: u8 },
    /// 经验获取增加百分比
    IncreaseExpPercent(i32),
    /// 物品掉落率增加百分比
    IncreaseDropRatePercent(i32),
    /// 套装效果标记 (set_id)
    SetBonus { set_id: u16 },
}

/// 卡片属性类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardStat {
    Str,
    Agi,
    Vit,
    Int,
    Dex,
    Luk,
    Atk,
    Matk,
    Hit,
    Flee,
    Crit,
    Aspd,
    MaxHp,
    MaxSp,
}

/// 怪物种族
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MonsterRace {
    Formless,
    Undead,
    Brute,
    Plant,
    Insect,
    Fish,
    Demon,
    DemiHuman,
    Angel,
    Dragon,
    Player,
    Boss,
    NonBoss,
    All,
}

/// 卡片数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardData {
    pub card_id: u32,
    pub name: String,
    pub description: String,
    /// 该卡片可插入的装备位置
    pub equip_slots: Vec<EquipSlotForCard>,
    /// 卡片效果列表
    pub effects: Vec<CardEffect>,
    /// 是否为 MVP 卡片
    pub is_mvp: bool,
    /// 是否为 mini-boss 卡片
    pub is_mini_boss: bool,
}

/// 卡片可插入的装备槽位
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquipSlotForCard {
    HeadTop,
    HeadMid,
    HeadLow,
    Armor,
    Weapon,
    Shield,
    Garment,
    Shoes,
    Accessory1,
    Accessory2,
    All,
}

/// 已插入卡片的装备槽
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardSlot {
    pub slot_index: usize,
    pub card_id: u32,
    pub card_name: String,
}

/// 卡片数据库
#[derive(Debug, Clone, Default)]
pub struct CardDatabase {
    cards: Vec<CardData>,
}

impl CardDatabase {
    pub fn new() -> Self {
        Self { cards: Vec::new() }
    }

    pub fn register(&mut self, card: CardData) {
        self.cards.push(card);
    }

    pub fn get_card(&self, card_id: u32) -> Option<&CardData> {
        self.cards.iter().find(|c| c.card_id == card_id)
    }

    pub fn card_count(&self) -> usize {
        self.cards.len()
    }

    /// 注册内置的默认卡片数据
    pub fn register_default_cards(&mut self) {
        // Poring Card: LUK +2, Flee +1
        self.register(CardData {
            card_id: 4001,
            name: "Poring Card".to_string(),
            description: "LUK +2, Perfect Dodge +1".to_string(),
            equip_slots: vec![EquipSlotForCard::Armor],
            effects: vec![
                CardEffect::AddStat {
                    stat: CardStat::Luk,
                    value: 2,
                },
                CardEffect::AddFlee(1),
            ],
            is_mvp: false,
            is_mini_boss: false,
        });

        // Fabre Card: VIT +1, MaxHP +100
        self.register(CardData {
            card_id: 4002,
            name: "Fabre Card".to_string(),
            description: "VIT +1, MaxHP +100".to_string(),
            equip_slots: vec![EquipSlotForCard::Weapon],
            effects: vec![
                CardEffect::AddStat {
                    stat: CardStat::Vit,
                    value: 1,
                },
                CardEffect::AddStat {
                    stat: CardStat::MaxHp,
                    value: 100,
                },
            ],
            is_mvp: false,
            is_mini_boss: false,
        });

        // Drops Card: DEX +1, HIT +3
        self.register(CardData {
            card_id: 4004,
            name: "Drops Card".to_string(),
            description: "DEX +1, HIT +3".to_string(),
            equip_slots: vec![EquipSlotForCard::Weapon],
            effects: vec![
                CardEffect::AddStat {
                    stat: CardStat::Dex,
                    value: 1,
                },
                CardEffect::AddHit(3),
            ],
            is_mvp: false,
            is_mini_boss: false,
        });

        // Lunatic Card: LUK +2, Crit +2
        self.register(CardData {
            card_id: 4005,
            name: "Lunatic Card".to_string(),
            description: "LUK +2, CRIT +2".to_string(),
            equip_slots: vec![EquipSlotForCard::Weapon],
            effects: vec![
                CardEffect::AddStat {
                    stat: CardStat::Luk,
                    value: 2,
                },
                CardEffect::AddCrit(2),
            ],
            is_mvp: false,
            is_mini_boss: false,
        });

        // Hydra Card: Increase damage to DemiHuman race by 20%
        self.register(CardData {
            card_id: 4035,
            name: "Hydra Card".to_string(),
            description: "Increase damage to DemiHuman race by 20%".to_string(),
            equip_slots: vec![EquipSlotForCard::Weapon],
            effects: vec![CardEffect::IncreaseDamage {
                race: MonsterRace::DemiHuman,
                percent: 20,
            }],
            is_mvp: false,
            is_mini_boss: false,
        });

        // Thara Frog Card: Reduce damage from DemiHuman race by 30%
        self.register(CardData {
            card_id: 4058,
            name: "Thara Frog Card".to_string(),
            description: "Reduce damage from DemiHuman race by 30%".to_string(),
            equip_slots: vec![EquipSlotForCard::Shield],
            effects: vec![CardEffect::ReduceDamage {
                race: MonsterRace::DemiHuman,
                percent: 30,
            }],
            is_mvp: false,
            is_mini_boss: false,
        });

        // Phen Card: Grant Endure skill Lv.1, prevent casting interruption
        self.register(CardData {
            card_id: 4077,
            name: "Phen Card".to_string(),
            description: "Prevents casting from being interrupted".to_string(),
            equip_slots: vec![EquipSlotForCard::Accessory1, EquipSlotForCard::Accessory2],
            effects: vec![CardEffect::GrantSkill {
                skill_id: 8, // Endure
                level: 1,
            }],
            is_mvp: false,
            is_mini_boss: false,
        });

        // Ghostring Card: Enchant Armor with Ghost element
        self.register(CardData {
            card_id: 4047,
            name: "Ghostring Card".to_string(),
            description: "Enchants armor with Ghost property. Decreases HP recovery by 25%".to_string(),
            equip_slots: vec![EquipSlotForCard::Armor],
            effects: vec![
                CardEffect::EnchantArmor { element: 8 }, // Ghost element
                CardEffect::AddStat { stat: CardStat::MaxHp, value: -25 },
            ],
            is_mvp: false,
            is_mini_boss: false,
        });

        // Golden Thief Bug Card: Immune to all magic (MVP)
        self.register(CardData {
            card_id: 4128,
            name: "Golden Thief Bug Card".to_string(),
            description: "Nullify all magic spells. Increases SP consumption by 100%".to_string(),
            equip_slots: vec![EquipSlotForCard::Shield],
            effects: vec![
                CardEffect::IgnoreMdefPercent(100),
                CardEffect::AddStat { stat: CardStat::MaxSp, value: -100 },
            ],
            is_mvp: true,
            is_mini_boss: false,
        });

        // Orc Hero Card: Immune to Stun (MVP)
        self.register(CardData {
            card_id: 4143,
            name: "Orc Hero Card".to_string(),
            description: "Immune to Stun status. VIT +3".to_string(),
            equip_slots: vec![EquipSlotForCard::HeadTop, EquipSlotForCard::HeadMid],
            effects: vec![
                CardEffect::ImmuneStatus { status_id: 1 }, // Stun
                CardEffect::AddStat { stat: CardStat::Vit, value: 3 },
            ],
            is_mvp: true,
            is_mini_boss: false,
        });
    }
}
