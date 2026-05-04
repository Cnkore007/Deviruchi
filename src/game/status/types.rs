//! 状态效果类型定义

/// 状态效果分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusCategory {
    /// 增益效果
    Buff,
    /// 减益效果
    Debuff,
    /// 中性状态（如坐下、交易中）
    Neutral,
    /// 特殊状态
    Special,
}

/// 状态变化枚举 - 定义所有可用的状态效果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum StatusChange {
    // ==================== 通用状态 ====================
    /// 坐下
    Sit = 0,
    /// 交易状态
    Trade = 1,

    // ==================== 移动限制类 (Movement Restriction) ====================
    /// 眩晕 - 无法移动、攻击
    Stun = 2,
    /// 冰冻 - 无法移动、攻击（比Stun更严重）
    Freeze = 3,
    /// 睡眠 - 无法移动、攻击，被攻击会唤醒
    Sleep = 4,
    /// 石化 - 无法移动、攻击
    Stone = 5,
    /// 疑惑 - 随机移动
    Confusion = 6,
    /// 隐匿 - 隐身状态
    Hide = 7,
    /// 伪装 - 伪装成其他生物
    Cloak = 8,

    // ==================== 攻击限制类 (Attack Restriction) ====================
    /// 沉默 - 无法使用技能
    Silence = 9,
    /// 诅咒 - 降低所有属性，无法使用高级技能
    Curse = 10,

    // ==================== 增益BUFF (Positive Buffs) ====================
    /// 属性提升 - STR+增加
    IncreaseStr = 20,
    /// 敏捷提升 - AGI+增加
    IncreaseAgi = 21,
    /// 体力提升 - VIT+增加
    IncreaseVit = 22,
    /// 智力提升 - INT+增加
    IncreaseInt = 23,
    /// 灵巧提升 - DEX+增加
    IncreaseDex = 24,
    /// 幸运提升 - LUK+增加
    IncreaseLuk = 25,

    /// 加速 - ASPD+增加
    Haste = 30,
    /// 攻击加速
    AttackSpeedUp = 31,
    /// 最高速度
    MaxSpeedUp = 32,

    /// 祝福 - 全属性+增加，攻击力增加
    Blessing = 40,
    /// 集中 - HIT+增加
    Concentration = 41,
    /// 神佑 - 免疫诅咒
    SignumCrucis = 42,

    /// 力量提升 - ATK+增加
    PowerUp = 50,
    /// 魔法增强 - MATK+增加
    MagicPowerUp = 51,

    /// 防护罩 - 伤害减少
    Shield = 60,
    /// 反射盾 - 反射物理伤害
    ReflectPhysical = 61,
    /// 反射魔法 - 反射魔法伤害
    ReflectMagic = 62,

    /// HP自动恢复
    Regen = 70,
    /// SP自动恢复
    SpRegen = 71,
    /// 灵魂状态 - HP/SP回复加速
    Soul = 72,

    /// 无敌 - 免疫所有伤害
    Invincible = 80,
    /// 不可视化 - 隐身
    Invisible = 81,
    /// 圣体 - 免疫异常状态
    HolyBody = 82,

    // ==================== 减益DEBUFF (Negative Debuffs) ====================
    /// 中毒 - 持续HP伤害
    Poison = 100,
    /// 出血 - 持续HP伤害（比中毒严重）
    Bleeding = 101,
    /// 饥饿 - HP/SP回复停止
    Hunger = 102,

    /// 黑暗 - 命中率降低
    Blind = 110,
    /// 耳鸣 - 视野缩小
    Deafness = 111,
    /// 混乱 - 攻击命中率降低
    Chaos = 112,

    /// 昏迷 - ASPD-降低
    Slow = 120,
    /// 减速
    SpeedDown = 121,

    /// 虚弱 - ATK降低
    Weakness = 130,
    /// 魔法虚弱 - MATK降低
    MagicWeakness = 131,
    /// 防御下降 - DEF降低
    DefenseDown = 132,
    /// 魔法防御下降 - MDEF降低
    MagicDefenseDown = 133,

    // ==================== 元素属性 ====================
    /// 火属性
    FireProperty = 150,
    /// 水属性
    WaterProperty = 151,
    /// 土属性
    EarthProperty = 152,
    /// 风属性
    WindProperty = 153,
    /// 圣属性
    HolyProperty = 154,
    /// 暗属性
    ShadowProperty = 155,
    /// 幽灵属性
    GhostProperty = 156,
    /// 毒属性
    PoisonProperty = 157,

    // ==================== 特殊状态 ====================
    /// 物免
    BodyDefDown = 160,
    /// 灵魂封印
    SoulStrike = 170,
    /// 战斗状态
    Battle = 180,
    /// 警觉
    Alert = 181,
    /// 感知
    Perception = 182,

    // ==================== 元素抗性 ====================
    /// 火抗性
    FireResist = 200,
    /// 水抗性
    WaterResist = 201,
    /// 土抗性
    EarthResist = 202,
    /// 风抗性
    WindResist = 203,
    /// 圣抗性
    HolyResist = 204,
    /// 暗抗性
    ShadowResist = 205,
    /// 物理防御
    DefenseUp = 210,
    /// 魔法防御
    MagicDefenseUp = 211,

    // ==================== 技能特殊状态 ====================
    /// 致命伤 - 攻击时附带额外伤害
    CriticalDamage = 220,
    /// 追击
    ChaseWalk = 221,

    /// 复活保护
    Resurrection = 230,
    /// 死亡保护
    DeathProtection = 231,

    /// 物攻
    AtkUp = 240,
    /// 物防
    DefUp = 241,

    /// 未知状态（用于扩展）
    Unknown = 9999,
}

impl StatusChange {
    /// 获取状态分类
    pub fn category(&self) -> StatusCategory {
        match self {
            // 通用状态
            StatusChange::Sit | StatusChange::Trade => StatusCategory::Neutral,

            // 移动限制
            StatusChange::Stun
            | StatusChange::Freeze
            | StatusChange::Sleep
            | StatusChange::Stone
            | StatusChange::Confusion
            | StatusChange::Hide
            | StatusChange::Cloak => StatusCategory::Debuff,

            // 攻击限制
            StatusChange::Silence | StatusChange::Curse => StatusCategory::Debuff,

            // 增益BUFF
            StatusChange::IncreaseStr
            | StatusChange::IncreaseAgi
            | StatusChange::IncreaseVit
            | StatusChange::IncreaseInt
            | StatusChange::IncreaseDex
            | StatusChange::IncreaseLuk
            | StatusChange::Haste
            | StatusChange::AttackSpeedUp
            | StatusChange::MaxSpeedUp
            | StatusChange::Blessing
            | StatusChange::Concentration
            | StatusChange::SignumCrucis
            | StatusChange::PowerUp
            | StatusChange::MagicPowerUp
            | StatusChange::Shield
            | StatusChange::ReflectPhysical
            | StatusChange::ReflectMagic
            | StatusChange::Regen
            | StatusChange::SpRegen
            | StatusChange::Soul
            | StatusChange::Invincible
            | StatusChange::Invisible
            | StatusChange::HolyBody
            | StatusChange::FireResist
            | StatusChange::WaterResist
            | StatusChange::EarthResist
            | StatusChange::WindResist
            | StatusChange::HolyResist
            | StatusChange::ShadowResist
            | StatusChange::DefenseUp
            | StatusChange::MagicDefenseUp
            | StatusChange::AtkUp
            | StatusChange::DefUp => StatusCategory::Buff,

            // 减益DEBUFF
            StatusChange::Poison
            | StatusChange::Bleeding
            | StatusChange::Hunger
            | StatusChange::Blind
            | StatusChange::Deafness
            | StatusChange::Chaos
            | StatusChange::Slow
            | StatusChange::SpeedDown
            | StatusChange::Weakness
            | StatusChange::MagicWeakness
            | StatusChange::DefenseDown
            | StatusChange::MagicDefenseDown
            | StatusChange::BodyDefDown => StatusCategory::Debuff,

            // 元素属性
            StatusChange::FireProperty
            | StatusChange::WaterProperty
            | StatusChange::EarthProperty
            | StatusChange::WindProperty
            | StatusChange::HolyProperty
            | StatusChange::ShadowProperty
            | StatusChange::GhostProperty
            | StatusChange::PoisonProperty => StatusCategory::Special,

            // 特殊状态
            StatusChange::SoulStrike
            | StatusChange::Battle
            | StatusChange::Alert
            | StatusChange::Perception
            | StatusChange::CriticalDamage
            | StatusChange::ChaseWalk
            | StatusChange::Resurrection
            | StatusChange::DeathProtection => StatusCategory::Special,

            StatusChange::Unknown => StatusCategory::Neutral,
        }
    }

    /// 是否为增益效果
    pub fn is_buff(&self) -> bool {
        self.category() == StatusCategory::Buff
    }

    /// 是否为减益效果
    pub fn is_debuff(&self) -> bool {
        self.category() == StatusCategory::Debuff
    }

    /// 是否为增益效果
    pub fn is_negative(&self) -> bool {
        self.category() == StatusCategory::Debuff
    }

    /// 获取状态图标ID (用于客户端显示)
    pub fn icon_id(&self) -> u16 {
        match self {
            StatusChange::Sit => 100,
            StatusChange::Trade => 101,
            StatusChange::Stun => 1,
            StatusChange::Freeze => 2,
            StatusChange::Sleep => 3,
            StatusChange::Stone => 4,
            StatusChange::Confusion => 5,
            StatusChange::Hide => 6,
            StatusChange::Cloak => 7,
            StatusChange::Silence => 10,
            StatusChange::Curse => 11,
            StatusChange::IncreaseStr => 20,
            StatusChange::IncreaseAgi => 21,
            StatusChange::IncreaseVit => 22,
            StatusChange::IncreaseInt => 23,
            StatusChange::IncreaseDex => 24,
            StatusChange::IncreaseLuk => 25,
            StatusChange::Haste => 30,
            StatusChange::AttackSpeedUp => 31,
            StatusChange::MaxSpeedUp => 32,
            StatusChange::Blessing => 40,
            StatusChange::Concentration => 41,
            StatusChange::SignumCrucis => 42,
            StatusChange::PowerUp => 50,
            StatusChange::MagicPowerUp => 51,
            StatusChange::Shield => 60,
            StatusChange::ReflectPhysical => 61,
            StatusChange::ReflectMagic => 62,
            StatusChange::Regen => 70,
            StatusChange::SpRegen => 71,
            StatusChange::Soul => 72,
            StatusChange::Invincible => 80,
            StatusChange::Invisible => 81,
            StatusChange::HolyBody => 82,
            StatusChange::Poison => 100,
            StatusChange::Bleeding => 101,
            StatusChange::Hunger => 102,
            StatusChange::Blind => 110,
            StatusChange::Deafness => 111,
            StatusChange::Chaos => 112,
            StatusChange::Slow => 120,
            StatusChange::SpeedDown => 121,
            StatusChange::Weakness => 130,
            StatusChange::MagicWeakness => 131,
            StatusChange::DefenseDown => 132,
            StatusChange::MagicDefenseDown => 133,
            _ => 0,
        }
    }

    /// 获取状态名称
    pub fn name(&self) -> &'static str {
        match self {
            StatusChange::Sit => "Sit",
            StatusChange::Trade => "Trade",
            StatusChange::Stun => "Stun",
            StatusChange::Freeze => "Freeze",
            StatusChange::Sleep => "Sleep",
            StatusChange::Stone => "Stone",
            StatusChange::Confusion => "Confusion",
            StatusChange::Hide => "Hide",
            StatusChange::Cloak => "Cloak",
            StatusChange::Silence => "Silence",
            StatusChange::Curse => "Curse",
            StatusChange::IncreaseStr => "Increase STR",
            StatusChange::IncreaseAgi => "Increase AGI",
            StatusChange::IncreaseVit => "Increase VIT",
            StatusChange::IncreaseInt => "Increase INT",
            StatusChange::IncreaseDex => "Increase DEX",
            StatusChange::IncreaseLuk => "Increase LUK",
            StatusChange::Haste => "Haste",
            StatusChange::AttackSpeedUp => "Attack Speed Up",
            StatusChange::MaxSpeedUp => "Max Speed Up",
            StatusChange::Blessing => "Blessing",
            StatusChange::Concentration => "Concentration",
            StatusChange::SignumCrucis => "Signum Crucis",
            StatusChange::PowerUp => "Power Up",
            StatusChange::MagicPowerUp => "Magic Power Up",
            StatusChange::Shield => "Shield",
            StatusChange::ReflectPhysical => "Reflect Physical",
            StatusChange::ReflectMagic => "Reflect Magic",
            StatusChange::Regen => "Regeneration",
            StatusChange::SpRegen => "SP Regeneration",
            StatusChange::Soul => "Soul",
            StatusChange::Invincible => "Invincible",
            StatusChange::Invisible => "Invisible",
            StatusChange::HolyBody => "Holy Body",
            StatusChange::Poison => "Poison",
            StatusChange::Bleeding => "Bleeding",
            StatusChange::Hunger => "Hunger",
            StatusChange::Blind => "Blind",
            StatusChange::Deafness => "Deafness",
            StatusChange::Chaos => "Chaos",
            StatusChange::Slow => "Slow",
            StatusChange::SpeedDown => "Speed Down",
            StatusChange::Weakness => "Weakness",
            StatusChange::MagicWeakness => "Magic Weakness",
            StatusChange::DefenseDown => "Defense Down",
            StatusChange::MagicDefenseDown => "Magic Defense Down",
            StatusChange::FireProperty => "Fire Property",
            StatusChange::WaterProperty => "Water Property",
            StatusChange::EarthProperty => "Earth Property",
            StatusChange::WindProperty => "Wind Property",
            StatusChange::HolyProperty => "Holy Property",
            StatusChange::ShadowProperty => "Shadow Property",
            StatusChange::GhostProperty => "Ghost Property",
            StatusChange::PoisonProperty => "Poison Property",
            StatusChange::BodyDefDown => "Body Defense Down",
            StatusChange::SoulStrike => "Soul Strike",
            StatusChange::Battle => "Battle",
            StatusChange::Alert => "Alert",
            StatusChange::Perception => "Perception",
            StatusChange::FireResist => "Fire Resist",
            StatusChange::WaterResist => "Water Resist",
            StatusChange::EarthResist => "Earth Resist",
            StatusChange::WindResist => "Wind Resist",
            StatusChange::HolyResist => "Holy Resist",
            StatusChange::ShadowResist => "Shadow Resist",
            StatusChange::DefenseUp => "Defense Up",
            StatusChange::MagicDefenseUp => "Magic Defense Up",
            StatusChange::CriticalDamage => "Critical Damage",
            StatusChange::ChaseWalk => "Chase Walk",
            StatusChange::Resurrection => "Resurrection",
            StatusChange::DeathProtection => "Death Protection",
            StatusChange::AtkUp => "Attack Up",
            StatusChange::DefUp => "Defense Up",
            StatusChange::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for StatusChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl From<u32> for StatusChange {
    fn from(value: u32) -> Self {
        // 使用 as 转换，这应该直接映射到枚举值
        // StatusChange 有 #[repr(u32)]
        match value {
            0 => StatusChange::Sit,
            1 => StatusChange::Trade,
            2 => StatusChange::Stun,
            3 => StatusChange::Freeze,
            4 => StatusChange::Sleep,
            5 => StatusChange::Stone,
            6 => StatusChange::Confusion,
            7 => StatusChange::Hide,
            8 => StatusChange::Cloak,
            9 => StatusChange::Silence,
            10 => StatusChange::Curse,
            20 => StatusChange::IncreaseStr,
            21 => StatusChange::IncreaseAgi,
            22 => StatusChange::IncreaseVit,
            23 => StatusChange::IncreaseInt,
            24 => StatusChange::IncreaseDex,
            25 => StatusChange::IncreaseLuk,
            30 => StatusChange::Haste,
            31 => StatusChange::AttackSpeedUp,
            32 => StatusChange::MaxSpeedUp,
            40 => StatusChange::Blessing,
            41 => StatusChange::Concentration,
            42 => StatusChange::SignumCrucis,
            50 => StatusChange::PowerUp,
            51 => StatusChange::MagicPowerUp,
            60 => StatusChange::Shield,
            61 => StatusChange::ReflectPhysical,
            62 => StatusChange::ReflectMagic,
            70 => StatusChange::Regen,
            71 => StatusChange::SpRegen,
            72 => StatusChange::Soul,
            80 => StatusChange::Invincible,
            81 => StatusChange::Invisible,
            82 => StatusChange::HolyBody,
            100 => StatusChange::Poison,
            101 => StatusChange::Bleeding,
            102 => StatusChange::Hunger,
            110 => StatusChange::Blind,
            111 => StatusChange::Deafness,
            112 => StatusChange::Chaos,
            120 => StatusChange::Slow,
            121 => StatusChange::SpeedDown,
            130 => StatusChange::Weakness,
            131 => StatusChange::MagicWeakness,
            132 => StatusChange::DefenseDown,
            133 => StatusChange::MagicDefenseDown,
            150 => StatusChange::FireProperty,
            151 => StatusChange::WaterProperty,
            152 => StatusChange::EarthProperty,
            153 => StatusChange::WindProperty,
            154 => StatusChange::HolyProperty,
            155 => StatusChange::ShadowProperty,
            156 => StatusChange::GhostProperty,
            157 => StatusChange::PoisonProperty,
            160 => StatusChange::BodyDefDown,
            170 => StatusChange::SoulStrike,
            180 => StatusChange::Battle,
            181 => StatusChange::Alert,
            182 => StatusChange::Perception,
            200 => StatusChange::FireResist,
            201 => StatusChange::WaterResist,
            202 => StatusChange::EarthResist,
            203 => StatusChange::WindResist,
            204 => StatusChange::HolyResist,
            205 => StatusChange::ShadowResist,
            210 => StatusChange::DefenseUp,
            211 => StatusChange::MagicDefenseUp,
            220 => StatusChange::CriticalDamage,
            221 => StatusChange::ChaseWalk,
            230 => StatusChange::Resurrection,
            231 => StatusChange::DeathProtection,
            240 => StatusChange::AtkUp,
            241 => StatusChange::DefUp,
            _ => StatusChange::Unknown,
        }
    }
}

impl StatusChange {
    /// 从 u32 值创建 StatusChange
    pub fn from_u32(value: u32) -> Self {
        Self::from(value)
    }

    /// 转换为 u32 值
    pub fn to_u32(&self) -> u32 {
        *self as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_categories() {
        assert_eq!(StatusChange::Stun.category(), StatusCategory::Debuff);
        assert_eq!(StatusChange::Blessing.category(), StatusCategory::Buff);
        assert_eq!(StatusChange::Sit.category(), StatusCategory::Neutral);
        assert_eq!(
            StatusChange::FireProperty.category(),
            StatusCategory::Special
        );
    }

    #[test]
    fn test_is_buff_debuff() {
        assert!(StatusChange::Blessing.is_buff());
        assert!(!StatusChange::Blessing.is_debuff());
        assert!(StatusChange::Poison.is_debuff());
        assert!(!StatusChange::Poison.is_buff());
    }

    #[test]
    fn test_icon_id() {
        assert_eq!(StatusChange::Stun.icon_id(), 1);
        assert_eq!(StatusChange::Blessing.icon_id(), 40);
    }

    #[test]
    fn test_status_name() {
        assert_eq!(StatusChange::Stun.name(), "Stun");
        assert_eq!(StatusChange::Blessing.name(), "Blessing");
    }
}
