//! WoE (War of Emperium) 数据定义

use std::time::Instant;

/// 星期
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl DayOfWeek {
    /// 获取星期名称
    pub fn name(&self) -> &'static str {
        match self {
            DayOfWeek::Monday => "Monday",
            DayOfWeek::Tuesday => "Tuesday",
            DayOfWeek::Wednesday => "Wednesday",
            DayOfWeek::Thursday => "Thursday",
            DayOfWeek::Friday => "Friday",
            DayOfWeek::Saturday => "Saturday",
            DayOfWeek::Sunday => "Sunday",
        }
    }

    /// 从数字获取（0 = 周一）
    pub fn from_num(num: u8) -> Option<Self> {
        match num % 7 {
            0 => Some(DayOfWeek::Monday),
            1 => Some(DayOfWeek::Tuesday),
            2 => Some(DayOfWeek::Wednesday),
            3 => Some(DayOfWeek::Thursday),
            4 => Some(DayOfWeek::Friday),
            5 => Some(DayOfWeek::Saturday),
            6 => Some(DayOfWeek::Sunday),
            _ => None,
        }
    }

    /// 转换为数字（0 = 周一）
    pub fn to_num(&self) -> u8 {
        match self {
            DayOfWeek::Monday => 0,
            DayOfWeek::Tuesday => 1,
            DayOfWeek::Wednesday => 2,
            DayOfWeek::Thursday => 3,
            DayOfWeek::Friday => 4,
            DayOfWeek::Saturday => 5,
            DayOfWeek::Sunday => 6,
        }
    }
}

/// WoE 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WoEState {
    /// 未激活
    NotActive,
    /// 准备中（开始前5分钟）
    Preparing,
    /// 进行中
    Active,
    /// 结束中
    Ending,
}

impl WoEState {
    /// 获取状态描述
    pub fn description(&self) -> &'static str {
        match self {
            WoEState::NotActive => "WoE not active",
            WoEState::Preparing => "WoE preparing (5 minutes before start)",
            WoEState::Active => "WoE in progress",
            WoEState::Ending => "WoE ending",
        }
    }

    /// 是否允许攻击城堡
    pub fn allows_attack(&self) -> bool {
        matches!(self, WoEState::Active | WoEState::Ending)
    }
}

/// 城堡
#[derive(Debug, Clone)]
pub struct Castle {
    /// 城堡ID
    pub castle_id: u32,
    /// 城堡名称
    pub castle_name: String,
    /// 地图名称
    pub map_name: String,
    /// 所属公会ID
    pub guild_id: Option<u32>,
    /// 守卫HP
    pub guardian_hp: u32,
    /// 占据开始时间
    pub occupied_since: Option<Instant>,
    /// 经济值
    pub economy: u32,
    /// 防御值
    pub defense: u32,
    /// 城堡等级
    pub castle_level: u8,
    /// 税收（百分比）
    pub tax: u8,
}

impl Castle {
    /// 创建新城堡
    pub fn new(castle_id: u32, castle_name: String, map_name: String) -> Self {
        Self {
            castle_id,
            castle_name,
            map_name,
            guild_id: None,
            guardian_hp: 10000,
            occupied_since: None,
            economy: 100,
            defense: 100,
            castle_level: 1,
            tax: 0,
        }
    }

    /// 检查城堡是否被占领
    pub fn is_occupied(&self) -> bool {
        self.guild_id.is_some()
    }

    /// 获取占领持续时间
    pub fn occupation_duration(&self) -> Option<std::time::Duration> {
        self.occupied_since.map(|since| since.elapsed())
    }

    /// 占领城堡
    pub fn capture(&mut self, guild_id: u32) {
        self.guild_id = Some(guild_id);
        self.occupied_since = Some(Instant::now());
    }

    /// 放弃城堡
    pub fn abandon(&mut self) {
        self.guild_id = None;
        self.occupied_since = None;
    }

    /// 获取城堡状态信息
    pub fn status(&self) -> CastleStatus {
        CastleStatus {
            castle_id: self.castle_id,
            castle_name: self.castle_name.clone(),
            map_name: self.map_name.clone(),
            guild_id: self.guild_id,
            is_occupied: self.is_occupied(),
            guardian_hp: self.guardian_hp,
            economy: self.economy,
            defense: self.defense,
            castle_level: self.castle_level,
            tax: self.tax,
        }
    }
}

/// 城堡状态信息（用于网络传输）
#[derive(Debug, Clone)]
pub struct CastleStatus {
    pub castle_id: u32,
    pub castle_name: String,
    pub map_name: String,
    pub guild_id: Option<u32>,
    pub is_occupied: bool,
    pub guardian_hp: u32,
    pub economy: u32,
    pub defense: u32,
    pub castle_level: u8,
    pub tax: u8,
}

/// WoE 时间安排
#[derive(Debug, Clone)]
pub struct WoESchedule {
    /// 安排ID
    pub schedule_id: u32,
    /// 星期
    pub day_of_week: DayOfWeek,
    /// 开始小时 (0-23)
    pub start_hour: u8,
    /// 开始分钟 (0-59)
    pub start_minute: u8,
    /// 结束小时 (0-23)
    pub end_hour: u8,
    /// 结束分钟 (0-59)
    pub end_minute: u8,
}

impl WoESchedule {
    /// 创建新的 WoE 时间安排
    pub fn new(
        schedule_id: u32,
        day_of_week: DayOfWeek,
        start_hour: u8,
        start_minute: u8,
        end_hour: u8,
        end_minute: u8,
    ) -> Self {
        Self {
            schedule_id,
            day_of_week,
            start_hour,
            start_minute,
            end_hour,
            end_minute,
        }
    }

    /// 获取开始时间的分钟数
    pub fn start_minutes(&self) -> u32 {
        (self.day_of_week.to_num() as u32) * 1440
            + (self.start_hour as u32) * 60
            + (self.start_minute as u32)
    }

    /// 获取结束时间的分钟数
    pub fn end_minutes(&self) -> u32 {
        (self.day_of_week.to_num() as u32) * 1440
            + (self.end_hour as u32) * 60
            + (self.end_minute as u32)
    }

    /// 获取持续时间（分钟）
    pub fn duration_minutes(&self) -> u32 {
        self.end_minutes().saturating_sub(self.start_minutes())
    }

    /// 获取时间描述
    pub fn time_description(&self) -> String {
        format!(
            "{} {:02}:{:02} - {:02}:{:02}",
            self.day_of_week.name(),
            self.start_hour,
            self.start_minute,
            self.end_hour,
            self.end_minute
        )
    }
}

/// 城堡攻击者
#[derive(Debug, Clone)]
pub struct CastleAttacker {
    pub guild_id: u32,
    pub castle_id: u32,
    pub attack_count: u32,
    pub last_attack_time: Option<Instant>,
}

/// WoE 错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WoEError {
    /// 城堡不存在
    CastleNotFound,
    /// 公会不存在
    GuildNotFound,
    /// 公会已在防守
    GuildAlreadyDefending,
    /// 公会不在攻击列表
    GuildNotAttacking,
    /// 不是城堡持有者
    NotCastleOwner,
    /// WoE 未激活
    WoENotActive,
    /// WoE 已激活
    WoEAlreadyActive,
    /// 无权操作
    PermissionDenied,
    /// 攻击者数量已达上限
    MaxAttackersReached,
    /// 城堡已被攻击
    CastleUnderAttack,
    /// 冷却时间未到
    CooldownNotExpired,
}

impl std::fmt::Display for WoEError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WoEError::CastleNotFound => write!(f, "Castle not found"),
            WoEError::GuildNotFound => write!(f, "Guild not found"),
            WoEError::GuildAlreadyDefending => write!(f, "Guild already defending"),
            WoEError::GuildNotAttacking => write!(f, "Guild not attacking"),
            WoEError::NotCastleOwner => write!(f, "Not the castle owner"),
            WoEError::WoENotActive => write!(f, "WoE not active"),
            WoEError::WoEAlreadyActive => write!(f, "WoE already active"),
            WoEError::PermissionDenied => write!(f, "Permission denied"),
            WoEError::MaxAttackersReached => write!(f, "Maximum attackers reached"),
            WoEError::CastleUnderAttack => write!(f, "Castle is under attack"),
            WoEError::CooldownNotExpired => write!(f, "Cooldown not expired"),
        }
    }
}

impl std::error::Error for WoEError {}

/// 默认城堡数据配置
#[derive(Debug, Clone)]
pub struct DefaultCastle {
    pub castle_id: u32,
    pub castle_name: &'static str,
    pub map_name: &'static str,
    pub guardian_hp: u32,
    pub economy: u32,
    pub defense: u32,
}

/// 默认城堡列表
pub const DEFAULT_CASTLES: &[DefaultCastle] = &[
    DefaultCastle {
        castle_id: 1,
        castle_name: "Al De Baran",
        map_name: "aldeg_cas01",
        guardian_hp: 10000,
        economy: 100,
        defense: 100,
    },
    DefaultCastle {
        castle_id: 2,
        castle_name: "Geffen",
        map_name: "gefg_cas01",
        guardian_hp: 10000,
        economy: 100,
        defense: 100,
    },
    DefaultCastle {
        castle_id: 3,
        castle_name: "Glast Heim",
        map_name: "glastk_cas01",
        guardian_hp: 10000,
        economy: 100,
        defense: 100,
    },
    DefaultCastle {
        castle_id: 4,
        castle_name: "Kriemhild",
        map_name: "schg_cas01",
        guardian_hp: 10000,
        economy: 100,
        defense: 100,
    },
    DefaultCastle {
        castle_id: 5,
        castle_name: "Swan",
        map_name: "swat01",
        guardian_hp: 10000,
        economy: 100,
        defense: 100,
    },
    DefaultCastle {
        castle_id: 6,
        castle_name: "Werner",
        map_name: "teg_cas01",
        guardian_hp: 10000,
        economy: 100,
        defense: 100,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_of_week() {
        assert_eq!(DayOfWeek::from_num(0), Some(DayOfWeek::Monday));
        assert_eq!(DayOfWeek::from_num(6), Some(DayOfWeek::Sunday));
        assert_eq!(DayOfWeek::Monday.to_num(), 0);
        assert_eq!(DayOfWeek::Sunday.to_num(), 6);
    }

    #[test]
    fn test_castle_operations() {
        let mut castle = Castle::new(1, "Test Castle".to_string(), "test_map".to_string());

        assert!(!castle.is_occupied());
        assert!(castle.guild_id.is_none());

        castle.capture(100);
        assert!(castle.is_occupied());
        assert_eq!(castle.guild_id, Some(100));
        assert!(castle.occupied_since.is_some());

        castle.abandon();
        assert!(!castle.is_occupied());
    }

    #[test]
    fn test_woe_schedule() {
        let schedule = WoESchedule::new(1, DayOfWeek::Saturday, 20, 0, 22, 0);

        assert_eq!(schedule.start_hour, 20);
        assert_eq!(schedule.end_hour, 22);
        assert_eq!(schedule.duration_minutes(), 120);
        assert_eq!(schedule.time_description(), "Saturday 20:00 - 22:00");
    }

    #[test]
    fn test_woe_state() {
        assert!(WoEState::Active.allows_attack());
        assert!(WoEState::Ending.allows_attack());
        assert!(!WoEState::NotActive.allows_attack());
        assert!(!WoEState::Preparing.allows_attack());
    }
}
