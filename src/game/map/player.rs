use crate::game::constants;
use crate::game::item::Equipment;
use crate::game::status::{PlayerStatus, StatusChange, StatusEffect, StatusSource};
use crate::storage::Character;
use crate::storage::character::{CharacterHotkeyData, CharacterInventoryData};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    Alive,
    Dead,
}

// ==================== 分组内部结构 ====================

/// 战斗相关状态（HP/SP/状态/移动速度）
/// 合并为单个锁，避免 TOCTOU：如读 hp 判断后再写 hp 之间被其他线程修改
pub struct CombatStats {
    pub(crate) hp: u32,
    pub(crate) max_hp: u32,
    pub(crate) sp: u32,
    pub(crate) max_sp: u32,
    pub(crate) state: PlayerState,
    pub(crate) in_combat: bool,
    pub(crate) is_sitting: bool,
    pub(crate) walk_speed: u16,
}

/// 位置（原子更新 x/y）
pub struct Position {
    pub(crate) x: u16,
    pub(crate) y: u16,
}

/// 等级与经验值
pub struct LevelStats {
    pub(crate) base_level: u16,
    pub(crate) job_level: u16,
    pub(crate) base_exp: u64,
    pub(crate) job_exp: u64,
}

/// 六维属性
pub struct Attributes {
    pub(crate) str: u16,
    pub(crate) agi: u16,
    pub(crate) vit: u16,
    pub(crate) int: u16,
    pub(crate) dex: u16,
    pub(crate) luk: u16,
}

/// 经济与负重
pub struct Economy {
    pub(crate) zeny: u32,
    pub(crate) current_weight: u32,
    pub(crate) max_weight: u32,
    pub(crate) job: u16,
    pub(crate) shop_id: Option<Uuid>,
    pub(crate) group_id: i32,
}

/// 存档点
pub struct SavePoint {
    pub(crate) map: String,
    pub(crate) x: u16,
    pub(crate) y: u16,
}

// ==================== Player 主结构 ====================

pub struct Player {
    pub(crate) id: Uuid,
    pub(crate) char_id: u32,
    pub(crate) account_id: u32,
    pub(crate) name: String,
    pub(crate) map_name: String,

    // 分组锁（6 个 RwLock 替代原来 26 个）
    pub(crate) combat: RwLock<CombatStats>,
    pub(crate) pos: RwLock<Position>,
    pub(crate) level: RwLock<LevelStats>,
    pub(crate) attrs: RwLock<Attributes>,
    pub(crate) economy: RwLock<Economy>,
    pub(crate) save_point: RwLock<SavePoint>,

    // 独立锁（复杂类型或低频访问）
    pub(crate) equipment: RwLock<Equipment>,
    pub(crate) status: PlayerStatus,
    pub(crate) inventory: RwLock<Vec<CharacterInventoryData>>,
    pub(crate) hotkeys: RwLock<Vec<CharacterHotkeyData>>,
}

impl Clone for Player {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            char_id: self.char_id,
            account_id: self.account_id,
            name: self.name.clone(),
            map_name: self.map_name.clone(),
            combat: RwLock::new(self.combat.read().clone()),
            pos: RwLock::new(self.pos.read().clone()),
            level: RwLock::new(self.level.read().clone()),
            attrs: RwLock::new(self.attrs.read().clone()),
            economy: RwLock::new(self.economy.read().clone()),
            save_point: RwLock::new(self.save_point.read().clone()),
            equipment: RwLock::new(self.equipment.read().clone()),
            status: self.status.clone(),
            inventory: RwLock::new(self.inventory.read().clone()),
            hotkeys: RwLock::new(self.hotkeys.read().clone()),
        }
    }
}

// 为分组结构派生 Clone
impl Clone for CombatStats {
    fn clone(&self) -> Self {
        Self {
            hp: self.hp,
            max_hp: self.max_hp,
            sp: self.sp,
            max_sp: self.max_sp,
            state: self.state,
            in_combat: self.in_combat,
            is_sitting: self.is_sitting,
            walk_speed: self.walk_speed,
        }
    }
}
impl Clone for Position {
    fn clone(&self) -> Self {
        Self { x: self.x, y: self.y }
    }
}
impl Clone for LevelStats {
    fn clone(&self) -> Self {
        Self {
            base_level: self.base_level,
            job_level: self.job_level,
            base_exp: self.base_exp,
            job_exp: self.job_exp,
        }
    }
}
impl Clone for Attributes {
    fn clone(&self) -> Self {
        Self {
            str: self.str,
            agi: self.agi,
            vit: self.vit,
            int: self.int,
            dex: self.dex,
            luk: self.luk,
        }
    }
}
impl Clone for Economy {
    fn clone(&self) -> Self {
        Self {
            zeny: self.zeny,
            current_weight: self.current_weight,
            max_weight: self.max_weight,
            job: self.job,
            shop_id: self.shop_id,
            group_id: self.group_id,
        }
    }
}
impl Clone for SavePoint {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            x: self.x,
            y: self.y,
        }
    }
}

impl Player {
    /// 从 Character 创建 Player
    pub fn from_character(char: Character) -> Self {
        Self {
            id: Uuid::new_v4(),
            char_id: char.char_id,
            account_id: 0,
            name: char.name,
            map_name: char.last_map.clone(),
            combat: RwLock::new(CombatStats {
                hp: char.hp,
                max_hp: char.max_hp,
                sp: char.sp,
                max_sp: char.max_sp,
                state: PlayerState::Alive,
                in_combat: false,
                is_sitting: false,
                walk_speed: constants::DEFAULT_WALK_SPEED,
            }),
            pos: RwLock::new(Position {
                x: char.last_x as u16,
                y: char.last_y as u16,
            }),
            level: RwLock::new(LevelStats {
                base_level: char.base_level,
                job_level: char.job_level,
                base_exp: char.base_exp as u64,
                job_exp: char.job_exp as u64,
            }),
            attrs: RwLock::new(Attributes {
                str: char.str,
                agi: char.agi,
                vit: char.vit,
                int: char.int,
                dex: char.dex,
                luk: char.luk,
            }),
            economy: RwLock::new(Economy {
                zeny: char.zeny,
                current_weight: 0,
                max_weight: constants::BASE_MAX_WEIGHT + (char.str as u32) * constants::WEIGHT_PER_STR,
                job: char.class,
                shop_id: None,
                group_id: 0,
            }),
            save_point: RwLock::new(SavePoint {
                map: char.save_map.clone(),
                x: char.save_x as u16,
                y: char.save_y as u16,
            }),
            equipment: RwLock::new(Equipment::new()),
            status: PlayerStatus::new(Uuid::new_v4()),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
        }
    }

    /// 从 CharacterData 创建 Player
    #[allow(dead_code)]
    pub fn from_character_data(
        _db: &crate::storage::Database,
        char: crate::game::map::CharacterData,
    ) -> Arc<Self> {
        Arc::new(Self {
            id: Uuid::new_v4(),
            char_id: char.char_id,
            account_id: char.account_id,
            name: char.name.clone(),
            map_name: char.last_map.clone(),
            combat: RwLock::new(CombatStats {
                hp: char.hp,
                max_hp: char.max_hp,
                sp: char.sp,
                max_sp: char.max_sp,
                state: PlayerState::Alive,
                in_combat: false,
                is_sitting: false,
                walk_speed: constants::DEFAULT_WALK_SPEED,
            }),
            pos: RwLock::new(Position {
                x: char.last_x as u16,
                y: char.last_y as u16,
            }),
            level: RwLock::new(LevelStats {
                base_level: char.base_level,
                job_level: char.job_level,
                base_exp: char.base_exp,
                job_exp: char.job_exp,
            }),
            attrs: RwLock::new(Attributes {
                str: char.str,
                agi: char.agi,
                vit: char.vit,
                int: char.int,
                dex: char.dex,
                luk: char.luk,
            }),
            economy: RwLock::new(Economy {
                zeny: char.zeny,
                current_weight: 0,
                max_weight: constants::BASE_MAX_WEIGHT + (char.str as u32) * constants::WEIGHT_PER_STR,
                job: char.job,
                shop_id: None,
                group_id: 0,
            }),
            save_point: RwLock::new(SavePoint {
                map: char.save_map.clone(),
                x: char.save_x as u16,
                y: char.save_y as u16,
            }),
            equipment: RwLock::new(Equipment::new()),
            status: PlayerStatus::new(Uuid::new_v4()),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
        })
    }

    // ==================== 分组锁访问器 ====================

    /// 获取战斗状态读锁
    pub fn combat(&self) -> RwLockReadGuard<'_, CombatStats> {
        self.combat.read()
    }

    /// 获取战斗状态写锁
    pub fn combat_mut(&self) -> RwLockWriteGuard<'_, CombatStats> {
        self.combat.write()
    }

    /// 获取位置读锁
    pub fn position(&self) -> RwLockReadGuard<'_, Position> {
        self.pos.read()
    }

    /// 获取位置写锁
    pub fn position_mut(&self) -> RwLockWriteGuard<'_, Position> {
        self.pos.write()
    }

    /// 获取等级/经验读锁
    pub fn level_stats(&self) -> RwLockReadGuard<'_, LevelStats> {
        self.level.read()
    }

    /// 获取等级/经验写锁
    pub fn level_stats_mut(&self) -> RwLockWriteGuard<'_, LevelStats> {
        self.level.write()
    }

    /// 获取属性读锁
    pub fn attributes(&self) -> RwLockReadGuard<'_, Attributes> {
        self.attrs.read()
    }

    /// 获取属性写锁
    pub fn attributes_mut(&self) -> RwLockWriteGuard<'_, Attributes> {
        self.attrs.write()
    }

    /// 获取经济读锁
    pub fn economy(&self) -> RwLockReadGuard<'_, Economy> {
        self.economy.read()
    }

    /// 获取经济写锁
    pub fn economy_mut(&self) -> RwLockWriteGuard<'_, Economy> {
        self.economy.write()
    }

    /// 获取存档点读锁
    pub fn get_save_point_lock(&self) -> RwLockReadGuard<'_, SavePoint> {
        self.save_point.read()
    }

    /// 获取存档点写锁
    pub fn save_point_mut(&self) -> RwLockWriteGuard<'_, SavePoint> {
        self.save_point.write()
    }

    // ==================== 向后兼容的字段访问器 ====================

    // --- CombatStats 字段 ---
    pub fn hp(&self) -> u32 {
        self.combat.read().hp
    }
    pub fn max_hp(&self) -> u32 {
        self.combat.read().max_hp
    }
    pub fn sp(&self) -> u32 {
        self.combat.read().sp
    }
    pub fn max_sp(&self) -> u32 {
        self.combat.read().max_sp
    }
    pub fn walk_speed(&self) -> u16 {
        self.combat.read().walk_speed
    }

    // --- Position 字段 ---
    pub fn pos_x(&self) -> u16 {
        self.pos.read().x
    }
    pub fn pos_y(&self) -> u16 {
        self.pos.read().y
    }

    // --- LevelStats 字段 ---
    pub fn base_level(&self) -> u16 {
        self.level.read().base_level
    }
    pub fn job_level(&self) -> u16 {
        self.level.read().job_level
    }
    pub fn base_exp(&self) -> u64 {
        self.level.read().base_exp
    }
    pub fn job_exp(&self) -> u64 {
        self.level.read().job_exp
    }

    // --- Attributes 字段 ---
    pub fn str(&self) -> u16 {
        self.attrs.read().str
    }
    pub fn agi(&self) -> u16 {
        self.attrs.read().agi
    }
    pub fn vit(&self) -> u16 {
        self.attrs.read().vit
    }
    pub fn int(&self) -> u16 {
        self.attrs.read().int
    }
    pub fn dex(&self) -> u16 {
        self.attrs.read().dex
    }
    pub fn luk(&self) -> u16 {
        self.attrs.read().luk
    }

    // --- Economy 字段 ---
    pub fn zeny(&self) -> u32 {
        self.economy.read().zeny
    }
    pub fn job(&self) -> u16 {
        self.economy.read().job
    }
    pub fn group_id(&self) -> i32 {
        self.economy.read().group_id
    }
    pub fn max_weight(&self) -> u32 {
        self.economy.read().max_weight
    }
    pub fn current_weight(&self) -> u32 {
        self.economy.read().current_weight
    }

    // ==================== 存档点相关 ====================

    #[allow(dead_code)]
    pub fn get_save_point(&self) -> (u16, u16) {
        let sp = self.save_point.read();
        (sp.x, sp.y)
    }

    #[allow(dead_code)]
    pub fn get_save_map(&self) -> String {
        self.save_point.read().map.clone()
    }

    #[allow(dead_code)]
    pub fn set_save_point(&self) {
        let pos = self.pos.read();
        let mut sp = self.save_point.write();
        sp.map = self.map_name.clone();
        sp.x = pos.x;
        sp.y = pos.y;
    }

    #[allow(dead_code)]
    pub fn set_save_point_at(&self, map: &str, x: u16, y: u16) {
        let mut sp = self.save_point.write();
        sp.map = map.to_string();
        sp.x = x;
        sp.y = y;
    }

    /// 移动到指定位置
    pub fn move_to(&self, x: u16, y: u16) {
        let mut pos = self.pos.write();
        pos.x = x;
        pos.y = y;
    }

    /// 获取当前位置
    pub fn get_position(&self) -> (u16, u16) {
        let pos = self.pos.read();
        (pos.x, pos.y)
    }

    /// 受到伤害，返回是否死亡
    pub fn take_damage(&self, damage: u32) -> bool {
        let mut c = self.combat.write();
        if c.hp <= damage {
            c.hp = 0;
            c.state = PlayerState::Dead;
            drop(c);
            self.apply_death_penalty();
            true
        } else {
            c.hp -= damage;
            false
        }
    }

    /// 角色死亡：设置状态并施加死亡惩罚
    pub fn die(&self) {
        {
            let mut c = self.combat.write();
            c.hp = 0;
            c.state = PlayerState::Dead;
        }
        self.apply_death_penalty();
    }

    /// 重生：恢复 HP/SP 并设置状态为 Alive
    pub fn respawn(&self, x: u16, y: u16) {
        let mut c = self.combat.write();
        c.hp = c.max_hp;
        c.sp = c.max_sp;
        c.state = PlayerState::Alive;
        c.is_sitting = false;
        drop(c);
        let mut pos = self.pos.write();
        pos.x = x;
        pos.y = y;
    }

    /// 是否死亡
    pub fn is_dead(&self) -> bool {
        self.combat.read().state == PlayerState::Dead
    }

    /// 是否存活
    pub fn is_alive(&self) -> bool {
        self.combat.read().state == PlayerState::Alive
    }

    /// 是否在战斗中
    pub fn is_in_combat(&self) -> bool {
        self.combat.read().in_combat
    }

    /// 设置战斗状态
    #[allow(dead_code)]
    pub fn set_combat(&self, in_combat: bool) {
        self.combat.write().in_combat = in_combat;
    }

    /// 设置职业ID
    #[allow(dead_code)]
    pub fn set_job(&self, job: u16) {
        self.economy.write().job = job;
    }

    /// 坐下
    pub fn sit(&self) {
        self.combat.write().is_sitting = true;
    }

    /// 站起
    pub fn stand(&self) {
        self.combat.write().is_sitting = false;
    }

    /// 是否坐下
    pub fn is_sitting(&self) -> bool {
        self.combat.read().is_sitting
    }

    /// 施加死亡惩罚：损失 1% 当前经验值
    fn apply_death_penalty(&self) {
        let mut lvl = self.level.write();
        lvl.base_exp = lvl.base_exp.saturating_sub(lvl.base_exp / 100);
        lvl.job_exp = lvl.job_exp.saturating_sub(lvl.job_exp / 100);
    }

    /// 死亡时掉落 Zeny（默认掉落50%）
    pub fn drop_zeny_on_death(&self) -> u32 {
        let mut eco = self.economy.write();
        let drop_amount = eco.zeny / 2;
        if drop_amount > 0 {
            eco.zeny -= drop_amount;
        }
        drop_amount
    }

    /// 获得基础经验值
    pub fn add_base_exp(&self, exp: u64) {
        self.level.write().base_exp += exp;
    }

    /// 获得职业经验值
    pub fn add_job_exp(&self, exp: u64) {
        self.level.write().job_exp += exp;
    }

    /// 获得 Zeny
    pub fn add_zeny(&self, zeny: u64) {
        let mut eco = self.economy.write();
        let amount = zeny.min(u32::MAX as u64) as u32;
        let can_add = crate::game::zeny::MAX_ZENY.saturating_sub(eco.zeny);
        eco.zeny += amount.min(can_add);
    }

    /// 计算最大负重 (基础负重 + STR*每点负重, 单位0.1)
    pub fn calc_max_weight(&self) -> u32 {
        let s = self.attrs.read();
        constants::BASE_MAX_WEIGHT + (s.str as u32) * constants::WEIGHT_PER_STR
    }

    /// 更新最大负重
    pub fn update_max_weight(&self) {
        let new_max = self.calc_max_weight();
        self.economy.write().max_weight = new_max;
    }

    /// 检查是否超重(50%)
    pub fn is_overweight(&self) -> bool {
        let eco = self.economy.read();
        eco.current_weight > eco.max_weight * 50 / 100
    }

    /// 检查是否严重超重(90%)
    pub fn is_overweight_90(&self) -> bool {
        let eco = self.economy.read();
        eco.current_weight > eco.max_weight * 90 / 100
    }

    // ==================== 状态效果管理 ====================

    pub fn add_status(&self, effect: StatusEffect) -> Option<StatusEffect> {
        self.status.add_status(effect)
    }
    pub fn remove_status(&self, status: StatusChange) -> Option<StatusEffect> {
        self.status.remove_status(status)
    }
    pub fn has_status(&self, status: StatusChange) -> bool {
        self.status.has_status(status)
    }
    pub fn get_status(&self, status: StatusChange) -> Option<StatusEffect> {
        self.status.get_status(status)
    }
    pub fn clear_all_status(&self) {
        self.status.clear_all();
    }
    pub fn clear_status_by_category(&self, category: crate::game::status::StatusCategory) {
        self.status.clear_by_category(category);
    }
    pub fn get_all_statuses(&self) -> Vec<StatusEffect> {
        self.status.get_all_statuses()
    }
    pub fn has_combat_restriction(&self) -> bool {
        self.status.has_combat_restriction()
    }
    pub fn is_silenced(&self) -> bool {
        self.status.is_silenced()
    }
    pub fn is_invincible(&self) -> bool {
        self.status.is_invincible()
    }
    pub fn is_invisible(&self) -> bool {
        self.status.is_invisible()
    }
    pub fn cleanup_expired_status(&self) -> Vec<StatusChange> {
        self.status.cleanup_expired()
    }

    pub fn apply_blessing(&self, level: u8) {
        let effect = StatusEffect::with_values(
            StatusChange::Blessing,
            (level as u64 * 30000).min(300000),
            StatusSource::Skill(9),
            level as i32,
            0,
            0,
        );
        self.add_status(effect);
    }

    pub fn apply_haste(&self, level: u8) {
        let effect = StatusEffect::with_values(
            StatusChange::Haste,
            (level as u64 * 30000).min(300000),
            StatusSource::Skill(29),
            level as i32 * 10,
            0,
            0,
        );
        self.add_status(effect);
    }

    pub fn apply_heal(&self, amount: u32) {
        let mut c = self.combat.write();
        let heal_amount = amount.min(c.max_hp - c.hp);
        c.hp += heal_amount;
    }

    pub fn can_act(&self) -> bool {
        if !self.is_alive() {
            return false;
        }
        !self.has_combat_restriction()
    }

    pub fn can_attack(&self) -> bool {
        self.can_act() && !self.has_status(StatusChange::Silence)
    }

    pub fn can_move(&self) -> bool {
        self.can_act()
    }

    pub fn can_cast(&self) -> bool {
        self.can_act() && !self.is_silenced()
    }

    /// ==================== 数据持久化 ====================
    pub fn to_save_data(&self) -> PlayerSaveData {
        let c = self.combat.read();
        let pos = self.pos.read();
        let lvl = self.level.read();
        let s = self.attrs.read();
        let eco = self.economy.read();

        PlayerSaveData {
            char_id: self.char_id,
            last_map: self.map_name.clone(),
            last_x: pos.x as i32,
            last_y: pos.y as i32,
            hp: c.hp,
            max_hp: c.max_hp,
            sp: c.sp,
            max_sp: c.max_sp,
            base_exp: lvl.base_exp,
            job_exp: lvl.job_exp,
            base_level: lvl.base_level,
            job_level: lvl.job_level,
            zeny: eco.zeny,
            str: s.str,
            agi: s.agi,
            vit: s.vit,
            int: s.int,
            dex: s.dex,
            luk: s.luk,
            status_effects: self.status.get_all_statuses(),
            inventory: self.inventory.read().clone(),
            hotkeys: self.hotkeys.read().clone(),
        }
    }

    pub fn save_to_db(&self, db: &crate::storage::Database) -> anyhow::Result<()> {
        let save_data = self.to_save_data();

        db.save_character(
            save_data.char_id,
            &save_data.last_map,
            save_data.last_x,
            save_data.last_y,
            save_data.hp,
            save_data.max_hp,
            save_data.sp,
            save_data.max_sp,
            save_data.base_exp,
            save_data.job_exp,
            save_data.base_level,
            save_data.job_level,
            save_data.zeny,
            save_data.str,
            save_data.agi,
            save_data.vit,
            save_data.int,
            save_data.dex,
            save_data.luk,
            &save_data.status_effects,
            &save_data.inventory,
            &save_data.hotkeys,
        )?;

        tracing::debug!("Player {} saved to database", self.name);
        Ok(())
    }
}

/// 玩家保存数据结构
#[derive(Debug, Clone)]
pub struct PlayerSaveData {
    pub(crate) char_id: u32,
    pub(crate) last_map: String,
    pub(crate) last_x: i32,
    pub(crate) last_y: i32,
    pub(crate) hp: u32,
    pub(crate) max_hp: u32,
    pub(crate) sp: u32,
    pub(crate) max_sp: u32,
    pub(crate) base_exp: u64,
    pub(crate) job_exp: u64,
    pub(crate) base_level: u16,
    pub(crate) job_level: u16,
    pub(crate) zeny: u32,
    pub(crate) str: u16,
    pub(crate) agi: u16,
    pub(crate) vit: u16,
    pub(crate) int: u16,
    pub(crate) dex: u16,
    pub(crate) luk: u16,
    pub(crate) status_effects: Vec<crate::game::status::effect::StatusEffect>,
    pub(crate) inventory: Vec<CharacterInventoryData>,
    pub(crate) hotkeys: Vec<CharacterHotkeyData>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::status::effect::StatusSource;
    use crate::game::status::types::StatusChange;

    fn make_player() -> Player {
        Player {
            id: Uuid::new_v4(),
            char_id: 1,
            account_id: 1,
            name: "Test".to_string(),
            map_name: "test_map".to_string(),
            combat: RwLock::new(CombatStats {
                hp: 100,
                max_hp: 100,
                sp: 50,
                max_sp: 50,
                state: PlayerState::Alive,
                in_combat: false,
                is_sitting: false,
                walk_speed: constants::DEFAULT_WALK_SPEED,
            }),
            pos: RwLock::new(Position { x: 100, y: 100 }),
            level: RwLock::new(LevelStats {
                base_level: 10,
                job_level: 5,
                base_exp: 5000,
                job_exp: 3000,
            }),
            attrs: RwLock::new(Attributes {
                str: 1,
                agi: 1,
                vit: 1,
                int: 1,
                dex: 1,
                luk: 1,
            }),
            economy: RwLock::new(Economy {
                zeny: 0,
                current_weight: 0,
                max_weight: constants::BASE_MAX_WEIGHT,
                job: 0,
                shop_id: None,
                group_id: 0,
            }),
            save_point: RwLock::new(SavePoint {
                map: "test_map".to_string(),
                x: 50,
                y: 50,
            }),
            equipment: RwLock::new(Equipment::new()),
            status: PlayerStatus::new(Uuid::new_v4()),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
        }
    }

    #[test]
    fn test_player_die_sets_state_dead() {
        let player = make_player();
        assert!(player.is_alive());
        player.die();
        assert!(player.is_dead());
        assert_eq!(player.hp(), 0);
    }

    #[test]
    fn test_player_die_applies_exp_penalty() {
        let player = make_player();
        assert_eq!(player.base_exp(), 5000);
        assert_eq!(player.job_exp(), 3000);

        player.die();

        assert_eq!(player.base_exp(), 4950);
        assert_eq!(player.job_exp(), 2970);
    }

    #[test]
    fn test_player_die_small_exp_clamped_to_zero() {
        let player = make_player();
        player.level.write().base_exp = 50;
        player.level.write().job_exp = 30;

        player.die();

        assert_eq!(player.base_exp(), 50);
        assert_eq!(player.job_exp(), 30);
    }

    #[test]
    fn test_player_respawn_restores_hp_and_state() {
        let player = make_player();
        player.die();
        assert!(player.is_dead());

        player.respawn(50, 60);
        assert!(player.is_alive());
        assert_eq!(player.hp(), 100);
        assert_eq!(player.sp(), 50);
        assert_eq!(player.get_position(), (50, 60));
    }

    #[test]
    fn test_take_damage_killing_blow_triggers_death() {
        let player = make_player();
        assert!(player.is_alive());

        let killed = player.take_damage(200);
        assert!(killed);
        assert!(player.is_dead());
        assert_eq!(player.hp(), 0);
    }

    #[test]
    fn test_take_damage_non_lethal_keeps_alive() {
        let player = make_player();
        let killed = player.take_damage(30);
        assert!(!killed);
        assert!(player.is_alive());
        assert_eq!(player.hp(), 70);
    }

    #[test]
    fn test_add_base_exp() {
        let player = make_player();
        player.add_base_exp(500);
        assert_eq!(player.base_exp(), 5500);
    }

    #[test]
    fn test_add_job_exp() {
        let player = make_player();
        player.add_job_exp(200);
        assert_eq!(player.job_exp(), 3200);
    }

    #[test]
    fn test_player_drop_zeny_on_death_drops_50_percent() {
        let player = make_player();
        player.economy.write().zeny = 1000;

        let dropped = player.drop_zeny_on_death();

        assert_eq!(dropped, 500);
        assert_eq!(player.zeny(), 500);
    }

    #[test]
    fn test_player_drop_zeny_on_death_small_zeny() {
        let player = make_player();
        player.economy.write().zeny = 1;

        let dropped = player.drop_zeny_on_death();

        assert_eq!(dropped, 0);
        assert_eq!(player.zeny(), 1);
    }

    #[test]
    fn test_player_drop_zeny_on_death_zero_zeny() {
        let player = make_player();

        let dropped = player.drop_zeny_on_death();

        assert_eq!(dropped, 0);
        assert_eq!(player.zeny(), 0);
    }

    #[test]
    fn test_player_drop_zeny_on_death_odd_number() {
        let player = make_player();
        player.economy.write().zeny = 101;

        let dropped = player.drop_zeny_on_death();

        assert_eq!(dropped, 50);
        assert_eq!(player.zeny(), 51);
    }

    // ==================== 持久化测试 ====================

    #[test]
    fn test_to_save_data_captures_all_fields() {
        let player = make_player();
        {
            let mut c = player.combat.write();
            c.hp = 500;
            c.max_hp = 1000;
            c.sp = 100;
            c.max_sp = 200;
        }
        {
            let mut lvl = player.level.write();
            lvl.base_exp = 10000;
            lvl.job_exp = 5000;
            lvl.base_level = 50;
            lvl.job_level = 30;
        }
        player.economy.write().zeny = 100000;
        {
            let mut a = player.attrs.write();
            a.str = 50;
            a.agi = 40;
            a.vit = 30;
            a.int = 20;
            a.dex = 60;
            a.luk = 10;
        }

        let save_data = player.to_save_data();

        assert_eq!(save_data.char_id, 1);
        assert_eq!(save_data.last_map, "test_map");
        assert_eq!(save_data.last_x, 100);
        assert_eq!(save_data.last_y, 100);
        assert_eq!(save_data.hp, 500);
        assert_eq!(save_data.max_hp, 1000);
        assert_eq!(save_data.sp, 100);
        assert_eq!(save_data.max_sp, 200);
        assert_eq!(save_data.base_exp, 10000);
        assert_eq!(save_data.job_exp, 5000);
        assert_eq!(save_data.base_level, 50);
        assert_eq!(save_data.job_level, 30);
        assert_eq!(save_data.zeny, 100000);
        assert_eq!(save_data.str, 50);
        assert_eq!(save_data.agi, 40);
        assert_eq!(save_data.vit, 30);
        assert_eq!(save_data.int, 20);
        assert_eq!(save_data.dex, 60);
        assert_eq!(save_data.luk, 10);
    }

    #[test]
    fn test_player_save_data_with_status_effects() {
        let player = make_player();

        let effect = StatusEffect::with_values(
            StatusChange::Blessing,
            60000,
            StatusSource::Skill(9),
            10,
            0,
            0,
        );
        player.add_status(effect);

        let save_data = player.to_save_data();

        assert_eq!(save_data.status_effects.len(), 1);
        assert_eq!(save_data.status_effects[0].id, StatusChange::Blessing);
    }
}
