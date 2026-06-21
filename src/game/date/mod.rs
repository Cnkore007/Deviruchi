//! 游戏内时间系统
//!
//! 对应 rAthena `src/map/date.cpp`，提供游戏内时间查询接口。
//! 用于 NPC 脚本条件判断、星芒系统等。

use chrono::{Datelike, Local, Timelike};

/// 月份枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Month {
    January = 1,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

impl Month {
    /// 从 u8 转换，1-12 有效
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            1 => Some(Self::January),
            2 => Some(Self::February),
            3 => Some(Self::March),
            4 => Some(Self::April),
            5 => Some(Self::May),
            6 => Some(Self::June),
            7 => Some(Self::July),
            8 => Some(Self::August),
            9 => Some(Self::September),
            10 => Some(Self::October),
            11 => Some(Self::November),
            12 => Some(Self::December),
            _ => None,
        }
    }
}

/// 星期枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GameDayOfWeek {
    Sunday = 0,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl GameDayOfWeek {
    /// 从 chrono::Weekday 转换
    pub fn from_chrono(wd: chrono::Weekday) -> Self {
        match wd {
            chrono::Weekday::Sun => Self::Sunday,
            chrono::Weekday::Mon => Self::Monday,
            chrono::Weekday::Tue => Self::Tuesday,
            chrono::Weekday::Wed => Self::Wednesday,
            chrono::Weekday::Thu => Self::Thursday,
            chrono::Weekday::Fri => Self::Friday,
            chrono::Weekday::Sat => Self::Saturday,
        }
    }
}

/// 日期查询类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DateType {
    Second = 1,
    Minute,
    Hour,
    DayOfWeek,
    DayOfMonth,
    Month,
    Year,
    DayOfYear,
    Yyyymmdd,
}

impl DateType {
    /// 从 u8 转换
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            1 => Some(Self::Second),
            2 => Some(Self::Minute),
            3 => Some(Self::Hour),
            4 => Some(Self::DayOfWeek),
            5 => Some(Self::DayOfMonth),
            6 => Some(Self::Month),
            7 => Some(Self::Year),
            8 => Some(Self::DayOfYear),
            9 => Some(Self::Yyyymmdd),
            _ => None,
        }
    }
}

/// 游戏时间查询器
///
/// 所有方法基于本地时间，对应 rAthena 的 `date_get_*` 系列函数。
/// 使用 `chrono::Local` 替代 C 的 `localtime`，线程安全且无 UB。
pub struct GameDate;

impl GameDate {
    /// 获取当前年份
    pub fn year() -> i32 {
        Local::now().year()
    }

    /// 获取当前月份
    pub fn month() -> Month {
        Month::from_u8(Local::now().month() as u8).unwrap_or(Month::January)
    }

    /// 获取当前日（1-31）
    pub fn day_of_month() -> u32 {
        Local::now().day()
    }

    /// 获取当前星期
    pub fn day_of_week() -> GameDayOfWeek {
        GameDayOfWeek::from_chrono(Local::now().weekday())
    }

    /// 获取当前是年内第几天（0-365）
    pub fn day_of_year() -> u32 {
        // chrono 没有直接提供 day_of_year，用 ordinal() - 1 保持与 rAthena 一致
        Local::now().ordinal() - 1
    }

    /// 获取当前小时（0-23）
    pub fn hour() -> u32 {
        Local::now().hour()
    }

    /// 获取当前分钟（0-59）
    pub fn minute() -> u32 {
        Local::now().minute()
    }

    /// 获取当前秒（0-59）
    pub fn second() -> u32 {
        Local::now().second()
    }

    /// 按类型查询日期值
    ///
    /// 对应 rAthena 的 `date_get(enum e_date_type type)` 函数。
    pub fn get(dtype: DateType) -> i32 {
        match dtype {
            DateType::Second => Self::second() as i32,
            DateType::Minute => Self::minute() as i32,
            DateType::Hour => Self::hour() as i32,
            DateType::DayOfWeek => Self::day_of_week() as i32,
            DateType::DayOfMonth => Self::day_of_month() as i32,
            DateType::Month => Self::month() as i32,
            DateType::Year => Self::year(),
            DateType::DayOfYear => Self::day_of_year() as i32,
            DateType::Yyyymmdd => {
                Self::year() * 10000 + Self::month() as i32 * 100 + Self::day_of_month() as i32
            }
        }
    }

    /// 今天是否是太阳日（星芒系统）
    ///
    /// 对应 rAthena 的 `is_day_of_sun()`：`(day_of_year + 1) % 2 == 0`
    pub fn is_day_of_sun() -> bool {
        (Self::day_of_year() + 1).is_multiple_of(2)
    }

    /// 今天是否是月亮日（星芒系统）
    ///
    /// 对应 rAthena 的 `is_day_of_moon()`：`(day_of_year + 1) % 2 == 1`
    pub fn is_day_of_moon() -> bool {
        (Self::day_of_year() + 1) % 2 == 1
    }

    /// 今天是否是星辰日（星芒系统）
    ///
    /// 对应 rAthena 的 `is_day_of_star()`：`(day_of_year + 1) % 5 == 0`
    pub fn is_day_of_star() -> bool {
        (Self::day_of_year() + 1).is_multiple_of(5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_month_from_u8() {
        assert_eq!(Month::from_u8(1), Some(Month::January));
        assert_eq!(Month::from_u8(12), Some(Month::December));
        assert_eq!(Month::from_u8(0), None);
        assert_eq!(Month::from_u8(13), None);
    }

    #[test]
    fn test_month_repr() {
        // 确保枚举值与 rAthena 一致（1-12）
        assert_eq!(Month::January as u8, 1);
        assert_eq!(Month::December as u8, 12);
    }

    #[test]
    fn test_day_of_week_from_chrono() {
        assert_eq!(
            GameDayOfWeek::from_chrono(chrono::Weekday::Sun),
            GameDayOfWeek::Sunday
        );
        assert_eq!(
            GameDayOfWeek::from_chrono(chrono::Weekday::Mon),
            GameDayOfWeek::Monday
        );
        assert_eq!(
            GameDayOfWeek::from_chrono(chrono::Weekday::Sat),
            GameDayOfWeek::Saturday
        );
    }

    #[test]
    fn test_day_of_week_repr() {
        assert_eq!(GameDayOfWeek::Sunday as u8, 0);
        assert_eq!(GameDayOfWeek::Saturday as u8, 6);
    }

    #[test]
    fn test_date_type_from_u8() {
        assert_eq!(DateType::from_u8(1), Some(DateType::Second));
        assert_eq!(DateType::from_u8(9), Some(DateType::Yyyymmdd));
        assert_eq!(DateType::from_u8(0), None);
        assert_eq!(DateType::from_u8(10), None);
    }

    #[test]
    fn test_year_range() {
        let year = GameDate::year();
        assert!((2024..=2100).contains(&year));
    }

    #[test]
    fn test_month_valid() {
        let month = GameDate::month();
        assert!((1..=12).contains(&(month as u8)));
    }

    #[test]
    fn test_day_of_month_range() {
        let day = GameDate::day_of_month();
        assert!((1..=31).contains(&day));
    }

    #[test]
    fn test_day_of_week_range() {
        let dow = GameDate::day_of_week();
        assert!((0..=6).contains(&(dow as u8)));
    }

    #[test]
    fn test_day_of_year_range() {
        let doy = GameDate::day_of_year();
        assert!(doy <= 365);
    }

    #[test]
    fn test_hour_range() {
        let hour = GameDate::hour();
        assert!(hour <= 23);
    }

    #[test]
    fn test_minute_range() {
        let min = GameDate::minute();
        assert!(min <= 59);
    }

    #[test]
    fn test_second_range() {
        let sec = GameDate::second();
        assert!(sec <= 59);
    }

    #[test]
    fn test_get_yyyymmdd() {
        let yyyymmdd = GameDate::get(DateType::Yyyymmdd);
        // 格式：YYYYMMDD，至少 20240101
        assert!(yyyymmdd >= 20240101);
        // 年份部分正确
        let year = yyyymmdd / 10000;
        assert!((2024..=2100).contains(&year));
    }

    #[test]
    fn test_get_consistency() {
        // date_get(X) 应与直接调用一致
        assert_eq!(GameDate::get(DateType::Year), GameDate::year());
        assert_eq!(GameDate::get(DateType::Month), GameDate::month() as i32);
        assert_eq!(
            GameDate::get(DateType::DayOfMonth),
            GameDate::day_of_month() as i32
        );
        assert_eq!(GameDate::get(DateType::Hour), GameDate::hour() as i32);
        assert_eq!(GameDate::get(DateType::Minute), GameDate::minute() as i32);
        assert_eq!(GameDate::get(DateType::Second), GameDate::second() as i32);
    }

    #[test]
    fn test_star_gladiator_days() {
        // 星芒系统：sun + moon 应该互斥且覆盖所有天
        let sun = GameDate::is_day_of_sun();
        let moon = GameDate::is_day_of_moon();
        assert!(sun ^ moon, "sun 和 moon 必须互斥");

        // star 每 5 天一次
        let star = GameDate::is_day_of_star();
        let doy = GameDate::day_of_year();
        assert_eq!(star, (doy + 1) % 5 == 0);
    }
}
