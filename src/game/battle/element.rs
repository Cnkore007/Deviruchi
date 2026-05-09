/// 元素属性（10种基本元素）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Element {
    Neutral = 0,
    Water = 1,
    Earth = 2,
    Fire = 3,
    Wind = 4,
    Poison = 5,
    Holy = 6,
    Dark = 7,
    Ghost = 8,
    Undead = 9,
}

impl Element {
    /// 从 u8 转换
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Neutral),
            1 => Some(Self::Water),
            2 => Some(Self::Earth),
            3 => Some(Self::Fire),
            4 => Some(Self::Wind),
            5 => Some(Self::Poison),
            6 => Some(Self::Holy),
            7 => Some(Self::Dark),
            8 => Some(Self::Ghost),
            9 => Some(Self::Undead),
            _ => None,
        }
    }
}

/// 元素等级（1-4级）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementLevel {
    Level1 = 1,
    Level2 = 2,
    Level3 = 3,
    Level4 = 4,
}

/// 目标体型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MobSize {
    Small = 0,
    Medium = 1,
    Large = 2,
}

/// 武器类型（用于体型修正）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeaponType {
    Fist,
    Dagger,
    OneHandSword,
    TwoHandSword,
    OneHandSpear,
    TwoHandSpear,
    OneHandAxe,
    TwoHandAxe,
    Mace,
    TwoHandMace,
    Staff,
    Bow,
    Musical,
    Whip,
    Book,
    Katar,
    Revolver,
    Rifle,
    Gatling,
    Shotgun,
    Grenade,
    Huuma,
    TwoHandStaff,
}

/// 元素相克修正表
/// 行 = 攻击者元素, 列 = 防御者元素
/// 值 = 伤害倍率百分比 (100 = 100%)
///
/// Level 1 元素修正表
const ELEMENT_TABLE_LV1: [[i32; 10]; 10] = [
    // NE WA EA FI WI PO HO DA GH UN
    [100,100,100,100,100,100,100,100, 70,100], // Neutral
    [100, 25,100, 90,175,100,100,100,100,100], // Water
    [100,100, 25,175, 90,100,100,100,100,100], // Earth
    [100, 90,175, 25,100,100,100,100,100,100], // Fire
    [100,175, 90,100, 25,100,100,100,100,100], // Wind
    [100,100,100,100,100,  0,125, 50,100,-25], // Poison
    [100,100,100,100,100,100,  0,125, 75,100], // Holy
    [100,100,100,100,100, 50,125,  0, 75,-25], // Dark
    [ 70,100,100,100,100,100,100,100,125,100], // Ghost
    [100,100,100,100,100, 50,100,100,100,  0], // Undead
];

/// Level 2 元素修正表
const ELEMENT_TABLE_LV2: [[i32; 10]; 10] = [
    [100,100,100,100,100,100,100,100, 50,100],
    [100,  0,100, 80,185,100,100,100,100,100],
    [100,100,  0,185, 80,100,100,100,100,100],
    [100, 80,185,  0,100,100,100,100,100,100],
    [100,185, 80,100,  0,100,100,100,100,100],
    [100,100,100,100,100,  0,150, 25,100,-50],
    [100,100,100,100,100,100,-25,150, 50,125],
    [100,100,100,100,100, 25,150,-25, 50,  0],
    [100,100,100,100,100,100,100,100,150,100],
    [100,100,100,100,100, 25,125,100,100,  0],
];

/// Level 3 元素修正表
const ELEMENT_TABLE_LV3: [[i32; 10]; 10] = [
    [100,100,100,100,100,100,100,100,  0,100],
    [100,-25,100, 70,190,100,100,100,100,100],
    [100,100,-25,190, 70,100,100,100,100,100],
    [100, 70,190,-25,100,100,100,100,100,100],
    [100,190, 70,100,-25,100,100,100,100,100],
    [100,100,100,100,100,  0,175,  0,100,-75],
    [100,100,100,100,100,100,-50,175, 25,150],
    [100,100,100,100,100,  0,175,-50, 25, 25],
    [100,100,100,100,100,100,100,100,175,100],
    [100,100,100,100,100,  0,150, 75,100,  0],
];

/// Level 4 元素修正表
const ELEMENT_TABLE_LV4: [[i32; 10]; 10] = [
    [100,100,100,100,100,100,100,100,  0,100],
    [100,-50,100, 60,195,100,100,100,100,100],
    [100,100,-50,195, 60,100,100,100,100,100],
    [100, 60,195,-50,100,100,100,100,100,100],
    [100,195, 60,100,-50,100,100,100,100,100],
    [100,100,100,100,100,  0,200,-25,100,-100],
    [100,100,100,100,100,100,-75,200,  0,175],
    [100,100,100,100,100,-25,200,-75,  0, 50],
    [  0,100,100,100,100,100,100,100,200,100],
    [100,100,100,100,100,-25,175, 75,100,  0],
];

/// 武器体型修正表
/// 键 = (武器类型, 目标体型), 值 = 伤害百分比
const SIZE_FIX_TABLE: &[(WeaponType, MobSize, i32)] = &[
    // 默认所有武器对所有体型 100%
    // 匕首
    (WeaponType::Dagger, MobSize::Medium, 75),
    (WeaponType::Dagger, MobSize::Large, 50),
    // 单手剑
    (WeaponType::OneHandSword, MobSize::Small, 75),
    (WeaponType::OneHandSword, MobSize::Large, 75),
    // 双手剑
    (WeaponType::TwoHandSword, MobSize::Small, 75),
    (WeaponType::TwoHandSword, MobSize::Medium, 75),
    // 单手矛
    (WeaponType::OneHandSpear, MobSize::Small, 75),
    (WeaponType::OneHandSpear, MobSize::Medium, 75),
    // 双手矛
    (WeaponType::TwoHandSpear, MobSize::Small, 75),
    (WeaponType::TwoHandSpear, MobSize::Medium, 75),
    // 单手斧
    (WeaponType::OneHandAxe, MobSize::Small, 50),
    (WeaponType::OneHandAxe, MobSize::Medium, 75),
    // 双手斧
    (WeaponType::TwoHandAxe, MobSize::Small, 50),
    (WeaponType::TwoHandAxe, MobSize::Medium, 75),
    // 钝器
    (WeaponType::Mace, MobSize::Small, 75),
    // 弓
    (WeaponType::Bow, MobSize::Large, 75),
    // 乐器
    (WeaponType::Musical, MobSize::Small, 75),
    (WeaponType::Musical, MobSize::Large, 75),
    // 鞭子
    (WeaponType::Whip, MobSize::Small, 75),
    (WeaponType::Whip, MobSize::Large, 75),
    // 书
    (WeaponType::Book, MobSize::Large, 50),
    // 拳刃
    (WeaponType::Katar, MobSize::Small, 75),
    (WeaponType::Katar, MobSize::Large, 75),
];

/// 获取元素相克伤害修正百分比
/// 参数: attacker_element, defender_element, element_level
pub fn get_element_modifier(
    attacker: Element,
    defender: Element,
    level: ElementLevel,
) -> i32 {
    let table = match level {
        ElementLevel::Level1 => &ELEMENT_TABLE_LV1,
        ElementLevel::Level2 => &ELEMENT_TABLE_LV2,
        ElementLevel::Level3 => &ELEMENT_TABLE_LV3,
        ElementLevel::Level4 => &ELEMENT_TABLE_LV4,
    };
    table[attacker as usize][defender as usize]
}

/// 获取武器体型修正百分比
/// 默认 100%，部分武器对特定体型有惩罚
pub fn get_size_modifier(weapon: WeaponType, target_size: MobSize) -> i32 {
    SIZE_FIX_TABLE
        .iter()
        .find(|(w, s, _)| *w == weapon && *s == target_size)
        .map(|(_, _, v)| *v)
        .unwrap_or(100)
}

/// 应用元素和体型修正后的伤害
/// damage * element_modifier% * size_modifier%
pub fn apply_element_and_size_modifier(
    base_damage: i32,
    attacker_element: Element,
    defender_element: Element,
    element_level: ElementLevel,
    weapon: WeaponType,
    target_size: MobSize,
) -> i32 {
    let elem_mod = get_element_modifier(attacker_element, defender_element, element_level);
    let size_mod = get_size_modifier(weapon, target_size);
    (base_damage as i64 * elem_mod as i64 * size_mod as i64 / 10000) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_water_vs_fire_lv1() {
        // Water attacks Fire at level 1: 90%
        assert_eq!(
            get_element_modifier(Element::Water, Element::Fire, ElementLevel::Level1),
            90
        );
    }

    #[test]
    fn test_fire_vs_water_lv1() {
        // Fire attacks Water at level 1: 90%
        assert_eq!(
            get_element_modifier(Element::Fire, Element::Water, ElementLevel::Level1),
            90
        );
    }

    #[test]
    fn test_wind_vs_water_lv1() {
        // Wind attacks Water at level 1: 175%
        assert_eq!(
            get_element_modifier(Element::Wind, Element::Water, ElementLevel::Level1),
            175
        );
    }

    #[test]
    fn test_poison_vs_poison_lv1() {
        // Poison attacks Poison: 0% (immune)
        assert_eq!(
            get_element_modifier(Element::Poison, Element::Poison, ElementLevel::Level1),
            0
        );
    }

    #[test]
    fn test_ghost_vs_neutral_lv4() {
        // Ghost attacks Neutral at Lv4: 0% (immune at Lv3+)
        assert_eq!(
            get_element_modifier(Element::Ghost, Element::Neutral, ElementLevel::Level4),
            0
        );
    }

    #[test]
    fn test_ghost_vs_neutral_lv1() {
        // Ghost attacks Neutral at Lv1: 70% (resisted)
        assert_eq!(
            get_element_modifier(Element::Ghost, Element::Neutral, ElementLevel::Level1),
            70
        );
    }

    #[test]
    fn test_dagger_vs_medium() {
        assert_eq!(
            get_size_modifier(WeaponType::Dagger, MobSize::Medium),
            75
        );
    }

    #[test]
    fn test_fist_vs_all_sizes() {
        // 拳套对所有体型 100%
        assert_eq!(get_size_modifier(WeaponType::Fist, MobSize::Small), 100);
        assert_eq!(get_size_modifier(WeaponType::Fist, MobSize::Medium), 100);
        assert_eq!(get_size_modifier(WeaponType::Fist, MobSize::Large), 100);
    }

    #[test]
    fn test_bow_vs_large() {
        assert_eq!(get_size_modifier(WeaponType::Bow, MobSize::Large), 75);
    }

    #[test]
    fn test_combined_modifier() {
        // Fire sword vs Earth monster (medium size)
        // Fire Lv1 vs Earth: 175% | 1hSword vs Medium: 100%
        let damage = apply_element_and_size_modifier(
            100,
            Element::Fire,
            Element::Earth,
            ElementLevel::Level1,
            WeaponType::OneHandSword,
            MobSize::Medium,
        );
        // 100 * 175 * 100 / 10000 = 175
        assert_eq!(damage, 175);
    }

    #[test]
    fn test_combined_modifier_penalty() {
        // Water dagger vs Large Fire mob
        // Water Lv1 vs Fire: 90% | Dagger vs Large: 50%
        let damage = apply_element_and_size_modifier(
            100,
            Element::Water,
            Element::Fire,
            ElementLevel::Level1,
            WeaponType::Dagger,
            MobSize::Large,
        );
        // 100 * 90 * 50 / 10000 = 45
        assert_eq!(damage, 45);
    }
}
