//! WoE (War of Emperium) 管理器

use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::{Datelike, Timelike, Utc, Weekday};

use super::data::{
    Castle, CastleAttacker, CastleStatus, DEFAULT_CASTLES, DayOfWeek, WoEError, WoESchedule,
    WoEState,
};

/// WoE 管理器
pub struct WoEManager {
    /// 城堡列表
    castles: RwLock<HashMap<u32, Castle>>,
    /// WoE 时间安排
    schedule: RwLock<Vec<WoESchedule>>,
    /// 当前状态
    current_state: RwLock<WoEState>,
    /// 防守公会映射 (castle_id -> guild_id)
    defending_guilds: RwLock<HashMap<u32, u32>>,
    /// 攻击公会列表
    attackers: RwLock<HashMap<u32, Vec<CastleAttacker>>>, // castle_id -> attackers
    /// 攻击冷却 (guild_id -> last_attack_time)
    attack_cooldown: RwLock<HashMap<u32, Instant>>,
    /// 上次 WoE 结束时间
    last_woe_end: RwLock<Option<Instant>>,
    /// 下次 WoE 开始时间
    next_woe_start: RwLock<Option<Instant>>,
    /// 下一安排ID
    next_schedule_id: RwLock<u32>,
}

use parking_lot::RwLock;

impl WoEManager {
    /// 创建新的 WoE 管理器
    pub fn new() -> Self {
        Self {
            castles: RwLock::new(HashMap::new()),
            schedule: RwLock::new(Vec::new()),
            current_state: RwLock::new(WoEState::NotActive),
            defending_guilds: RwLock::new(HashMap::new()),
            attackers: RwLock::new(HashMap::new()),
            attack_cooldown: RwLock::new(HashMap::new()),
            last_woe_end: RwLock::new(None),
            next_woe_start: RwLock::new(None),
            next_schedule_id: RwLock::new(1),
        }
    }

    /// 初始化默认城堡
    pub fn init_default_castles(&self) {
        let mut castles = self.castles.write();
        for default in DEFAULT_CASTLES {
            let castle = Castle::new(
                default.castle_id,
                default.castle_name.to_string(),
                default.map_name.to_string(),
            );
            castles.insert(default.castle_id, castle);
        }
    }

    /// 获取下一个安排ID
    fn next_schedule_id(&self) -> u32 {
        let mut next = self.next_schedule_id.write();
        let id = *next;
        *next += 1;
        id
    }

    /// 注册城堡
    pub fn register_castle(&self, castle: Castle) {
        self.castles.write().insert(castle.castle_id, castle);
    }

    /// 获取城堡
    pub fn get_castle(&self, castle_id: u32) -> Option<Castle> {
        self.castles.read().get(&castle_id).cloned()
    }

    /// 获取所有城堡
    pub fn get_all_castles(&self) -> Vec<Castle> {
        self.castles.read().values().cloned().collect()
    }

    /// 获取公会占领的城堡
    pub fn get_castles_by_guild(&self, guild_id: u32) -> Vec<Castle> {
        self.castles
            .read()
            .values()
            .filter(|c| c.guild_id == Some(guild_id))
            .cloned()
            .collect()
    }

    /// 获取城堡状态
    pub fn get_castle_status(&self, castle_id: u32) -> Option<CastleStatus> {
        self.castles.read().get(&castle_id).map(|c| c.status())
    }

    /// 添加 WoE 时间安排
    pub fn add_schedule(&self, schedule: WoESchedule) {
        self.schedule.write().push(schedule);
    }

    /// 创建 WoE 时间安排
    pub fn create_schedule(
        &self,
        day_of_week: DayOfWeek,
        start_hour: u8,
        start_minute: u8,
        end_hour: u8,
        end_minute: u8,
    ) -> WoESchedule {
        let schedule = WoESchedule::new(
            self.next_schedule_id(),
            day_of_week,
            start_hour,
            start_minute,
            end_hour,
            end_minute,
        );
        self.add_schedule(schedule.clone());
        schedule
    }

    /// 获取所有时间安排
    pub fn get_schedules(&self) -> Vec<WoESchedule> {
        self.schedule.read().clone()
    }

    /// 移除时间安排
    pub fn remove_schedule(&self, schedule_id: u32) -> bool {
        let mut schedules = self.schedule.write();
        let len = schedules.len();
        schedules.retain(|s| s.schedule_id != schedule_id);
        schedules.len() != len
    }

    /// 获取当前状态
    pub fn get_state(&self) -> WoEState {
        *self.current_state.read()
    }

    /// 检查 WoE 是否激活
    pub fn is_woe_active(&self) -> bool {
        matches!(
            *self.current_state.read(),
            WoEState::Active | WoEState::Preparing
        )
    }

    /// 开始 WoE
    pub fn start_woe(&self) -> Result<(), WoEError> {
        let mut state = self.current_state.write();
        if *state == WoEState::Active || *state == WoEState::Preparing {
            return Err(WoEError::WoEAlreadyActive);
        }

        *state = WoEState::Active;
        *self.last_woe_end.write() = None;
        drop(state);

        // 清空攻击者列表
        self.attackers.write().clear();

        Ok(())
    }

    /// 结束 WoE
    pub fn end_woe(&self) -> Result<Vec<Castle>, WoEError> {
        let mut state = self.current_state.write();
        if *state == WoEState::NotActive {
            return Err(WoEError::WoENotActive);
        }

        *state = WoEState::NotActive;
        *self.last_woe_end.write() = Some(Instant::now());
        *self.next_woe_start.write() = self.calculate_next_woe_time();
        drop(state);

        // 清空攻击者列表
        self.attackers.write().clear();

        // 返回所有城堡状态
        Ok(self.get_all_castles())
    }

    /// 进入准备阶段
    pub fn prepare_woe(&self) -> Result<(), WoEError> {
        let mut state = self.current_state.write();
        if *state == WoEState::Active || *state == WoEState::Preparing {
            return Err(WoEError::WoEAlreadyActive);
        }

        *state = WoEState::Preparing;
        Ok(())
    }

    /// 结束准备阶段，开始战斗
    pub fn begin_woe(&self) -> Result<(), WoEError> {
        let mut state = self.current_state.write();
        if *state != WoEState::Preparing {
            return Err(WoEError::WoENotActive);
        }

        *state = WoEState::Active;
        Ok(())
    }

    /// 攻击城堡
    pub fn attack_castle(&self, guild_id: u32, castle_id: u32) -> Result<(), WoEError> {
        // 检查 WoE 状态
        let state = self.get_state();
        if !state.allows_attack() {
            return Err(WoEError::WoENotActive);
        }

        // 检查城堡是否存在
        let castle_exists = self.castles.read().contains_key(&castle_id);
        if !castle_exists {
            return Err(WoEError::CastleNotFound);
        }

        // 检查攻击冷却
        if let Some(last_attack) = self.attack_cooldown.read().get(&guild_id)
            && last_attack.elapsed() < Duration::from_secs(300)
        {
            // 5分钟冷却
            return Err(WoEError::CooldownNotExpired);
        }

        // 添加攻击者
        let mut attackers = self.attackers.write();
        let castle_attackers = attackers.entry(castle_id).or_default();

        // 检查是否已在攻击列表中
        if let Some(attacker) = castle_attackers.iter_mut().find(|a| a.guild_id == guild_id) {
            attacker.attack_count += 1;
            attacker.last_attack_time = Some(Instant::now());
        } else {
            castle_attackers.push(CastleAttacker {
                guild_id,
                castle_id,
                attack_count: 1,
                last_attack_time: Some(Instant::now()),
            });
        }

        drop(attackers);

        // 设置攻击冷却
        self.attack_cooldown
            .write()
            .insert(guild_id, Instant::now());

        Ok(())
    }

    /// 放弃攻击城堡
    pub fn abandon_attack(&self, guild_id: u32, castle_id: u32) -> Result<(), WoEError> {
        let mut attackers = self.attackers.write();
        if let Some(castle_attackers) = attackers.get_mut(&castle_id) {
            castle_attackers.retain(|a| a.guild_id != guild_id);
            Ok(())
        } else {
            Err(WoEError::GuildNotAttacking)
        }
    }

    /// 防守城堡
    pub fn defend_castle(&self, guild_id: u32, castle_id: u32) -> bool {
        let castles = self.castles.read();
        let castle = castles.get(&castle_id);

        if castle
            .map(|c| c.guild_id == Some(guild_id))
            .unwrap_or(false)
        {
            self.defending_guilds.write().insert(castle_id, guild_id);
            true
        } else {
            false
        }
    }

    /// 占领城堡
    pub fn capture_castle(&self, guild_id: u32, castle_id: u32) -> Result<Castle, WoEError> {
        // 检查 WoE 是否激活
        if !self.is_woe_active() {
            return Err(WoEError::WoENotActive);
        }

        // 检查城堡是否存在并检查攻击者
        let castle_exists = self.castles.read().contains_key(&castle_id);
        if !castle_exists {
            return Err(WoEError::CastleNotFound);
        }

        // 检查攻击者是否在列表中
        let is_attacker = {
            let attackers = self.attackers.read();
            attackers
                .get(&castle_id)
                .map(|a| a.iter().any(|attacker| attacker.guild_id == guild_id))
                .unwrap_or(false)
        };

        if !is_attacker {
            return Err(WoEError::GuildNotAttacking);
        }

        // 占领城堡
        let mut castles = self.castles.write();
        let castle = castles
            .get_mut(&castle_id)
            .ok_or(WoEError::CastleNotFound)?;
        castle.capture(guild_id);

        // 更新经济值
        castle.economy = castle.economy.saturating_sub(10);
        drop(castles);

        // 更新防守公会
        self.defending_guilds.write().insert(castle_id, guild_id);

        self.get_castle(castle_id).ok_or(WoEError::CastleNotFound)
    }

    /// 放弃城堡所有权
    pub fn abandon_castle(&self, castle_id: u32) -> Result<Castle, WoEError> {
        let mut castles = self.castles.write();
        let castle = castles
            .get_mut(&castle_id)
            .ok_or(WoEError::CastleNotFound)?;

        castle.abandon();
        self.defending_guilds.write().remove(&castle_id);

        Ok(castle.clone())
    }

    /// 获取城堡的攻击者列表
    pub fn get_attackers(&self, castle_id: u32) -> Vec<CastleAttacker> {
        self.attackers
            .read()
            .get(&castle_id)
            .cloned()
            .unwrap_or_default()
    }

    /// 获取城堡的防守公会
    pub fn get_defender(&self, castle_id: u32) -> Option<u32> {
        self.defending_guilds.read().get(&castle_id).copied()
    }

    /// 获取攻击城堡的公会数量
    pub fn get_attacker_count(&self, castle_id: u32) -> usize {
        self.attackers
            .read()
            .get(&castle_id)
            .map(|a| a.len())
            .unwrap_or(0)
    }

    /// 计算下次 WoE 时间
    pub fn calculate_next_woe_time(&self) -> Option<Instant> {
        let schedules = self.schedule.read();
        if schedules.is_empty() {
            return None;
        }

        let now = Utc::now();
        let _current_weekday = match now.weekday() {
            Weekday::Mon => DayOfWeek::Monday,
            Weekday::Tue => DayOfWeek::Tuesday,
            Weekday::Wed => DayOfWeek::Wednesday,
            Weekday::Thu => DayOfWeek::Thursday,
            Weekday::Fri => DayOfWeek::Friday,
            Weekday::Sat => DayOfWeek::Saturday,
            Weekday::Sun => DayOfWeek::Sunday,
        };
        let current_minutes =
            (now.weekday().num_days_from_monday()) * 1440 + now.hour() * 60 + now.minute();

        let mut next_schedule: Option<&WoESchedule> = None;
        let mut next_minutes: Option<u32> = None;

        for schedule in schedules.iter() {
            let schedule_minutes = schedule.start_minutes();
            if schedule_minutes > current_minutes
                && (next_minutes.is_none() || schedule_minutes < next_minutes.unwrap())
            {
                next_minutes = Some(schedule_minutes);
                next_schedule = Some(schedule);
            }
        }

        // 如果没有找到本周的，使用下周第一个
        if next_schedule.is_none()
            && let Some(first) = schedules.iter().min_by_key(|s| s.start_minutes())
        {
            // 下周
            next_schedule = Some(first);
        }

        next_schedule.map(|_s| {
            // 简单计算：假设下周的这个时间
            let _next = now + Duration::from_secs(7 * 24 * 60 * 60);
            Instant::now() + Duration::from_secs(60) // 简化：返回1分钟后
        })
    }

    /// 获取下次 WoE 时间
    pub fn get_next_woe_time(&self) -> Option<Instant> {
        *self.next_woe_start.read()
    }

    /// 获取上次 WoE 结束时间
    pub fn get_last_woe_end(&self) -> Option<Instant> {
        *self.last_woe_end.read()
    }

    /// 检查公会是否是城堡的防守方
    pub fn is_defender(&self, guild_id: u32, castle_id: u32) -> bool {
        self.defending_guilds
            .read()
            .get(&castle_id)
            .map(|&g| g == guild_id)
            .unwrap_or(false)
    }

    /// 检查公会是否在攻击城堡
    pub fn is_attacking(&self, guild_id: u32, castle_id: u32) -> bool {
        self.attackers
            .read()
            .get(&castle_id)
            .map(|a| a.iter().any(|attacker| attacker.guild_id == guild_id))
            .unwrap_or(false)
    }

    /// 获取城堡列表状态
    pub fn get_castle_list(&self) -> Vec<CastleStatus> {
        self.castles.read().values().map(|c| c.status()).collect()
    }

    /// 更新城堡守卫HP
    pub fn damage_guardian(&self, castle_id: u32, damage: u32) -> Result<(), WoEError> {
        let mut castles = self.castles.write();
        let castle = castles
            .get_mut(&castle_id)
            .ok_or(WoEError::CastleNotFound)?;

        castle.guardian_hp = castle.guardian_hp.saturating_sub(damage);

        // 如果守卫HP为0，城堡可能被占领
        if castle.guardian_hp == 0 {
            // 重置守卫HP
            castle.guardian_hp = 10000;
        }

        Ok(())
    }

    /// 修复守卫
    pub fn repair_guardian(&self, castle_id: u32) -> Result<u32, WoEError> {
        let mut castles = self.castles.write();
        let castle = castles
            .get_mut(&castle_id)
            .ok_or(WoEError::CastleNotFound)?;

        let new_hp = 10000;
        let old_hp = castle.guardian_hp;
        castle.guardian_hp = new_hp;

        Ok(new_hp - old_hp)
    }

    /// 升级城堡
    pub fn upgrade_castle(
        &self,
        castle_id: u32,
        economy_cost: u32,
        defense_cost: u32,
    ) -> Result<(), WoEError> {
        let mut castles = self.castles.write();
        let castle = castles
            .get_mut(&castle_id)
            .ok_or(WoEError::CastleNotFound)?;

        if castle.economy < economy_cost {
            return Err(WoEError::PermissionDenied);
        }

        castle.economy -= economy_cost;
        castle.defense += defense_cost;
        castle.castle_level = (castle.castle_level + 1).min(10);

        Ok(())
    }
}

impl Default for WoEManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_woe_lifecycle() {
        let manager = WoEManager::new();
        manager.init_default_castles();

        // 检查默认城堡
        assert_eq!(manager.get_all_castles().len(), 6);

        // 开始 WoE
        assert!(manager.start_woe().is_ok());
        assert!(manager.is_woe_active());

        // 结束 WoE
        let castles = manager.end_woe().unwrap();
        assert!(!manager.is_woe_active());
        assert_eq!(castles.len(), 6);
    }

    #[test]
    fn test_castle_capture() {
        let manager = WoEManager::new();
        manager.init_default_castles();

        // 开始 WoE
        manager.start_woe().unwrap();

        // 攻击城堡
        assert!(manager.attack_castle(100, 1).is_ok());

        // 占领城堡
        let castle = manager.capture_castle(100, 1).unwrap();
        assert_eq!(castle.guild_id, Some(100));
        assert!(castle.is_occupied());
    }

    #[test]
    fn test_attack_during_non_woe() {
        let manager = WoEManager::new();
        manager.init_default_castles();

        // 尝试在非 WoE 期间攻击
        assert_eq!(
            manager.attack_castle(100, 1).unwrap_err(),
            WoEError::WoENotActive
        );
    }

    #[test]
    fn test_schedule_management() {
        let manager = WoEManager::new();

        // 添加安排
        let schedule = manager.create_schedule(DayOfWeek::Saturday, 20, 0, 22, 0);
        assert_eq!(schedule.start_hour, 20);
        assert_eq!(schedule.end_hour, 22);

        let schedules = manager.get_schedules();
        assert_eq!(schedules.len(), 1);

        // 移除安排
        assert!(manager.remove_schedule(schedule.schedule_id));
        assert_eq!(manager.get_schedules().len(), 0);
    }

    #[test]
    fn test_castle_defense() {
        let manager = WoEManager::new();
        manager.init_default_castles();

        // 占领城堡
        manager.start_woe().unwrap();
        manager.attack_castle(100, 1).unwrap();
        manager.capture_castle(100, 1).unwrap();

        // 检查防守状态
        assert!(manager.is_defender(100, 1));
        assert!(!manager.is_defender(200, 1));
    }

    #[test]
    fn test_guardian_damage() {
        let manager = WoEManager::new();
        manager.init_default_castles();

        let initial_hp = manager.get_castle(1).unwrap().guardian_hp;
        assert_eq!(initial_hp, 10000);

        manager.damage_guardian(1, 5000).unwrap();
        assert_eq!(manager.get_castle(1).unwrap().guardian_hp, 5000);

        manager.repair_guardian(1).unwrap();
        assert_eq!(manager.get_castle(1).unwrap().guardian_hp, 10000);
    }
}
