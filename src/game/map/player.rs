use crate::game::constants;
use crate::game::item::{Equipment, ItemDatabase};
use crate::game::script::commands::ScriptContext;
use crate::game::status::{PlayerStatus, StatusChange, StatusEffect, StatusSource};
use crate::storage::Character;
use crate::storage::character::{CharacterHotkeyData, CharacterInventoryData};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    Alive,
    Dead,
}

// ==================== 分组内部结构 ====================

/// 战斗相关状态（HP/SP/状态/移动速度/朝向）
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
    /// 朝向方向：0-7（8方向），对应 rAthena 方向编码
    pub(crate) direction: u16,
}

/// 位置（原子更新 x/y）
pub struct Position {
    pub(crate) x: u16,
    pub(crate) y: u16,
}

/// 等级与经验值、状态点、技能点
pub struct LevelStats {
    pub(crate) base_level: u16,
    pub(crate) job_level: u16,
    pub(crate) base_exp: u64,
    pub(crate) job_exp: u64,
    /// 可分配的状态点数（升级获得）
    pub(crate) status_point: u16,
    /// 可分配的技能点数（职业升级获得）
    pub(crate) skill_point: u16,
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

    // 社交关系
    pub(crate) party_id: RwLock<Option<Uuid>>,
    pub(crate) guild_id: RwLock<Option<Uuid>>,
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
            party_id: RwLock::new(*self.party_id.read()),
            guild_id: RwLock::new(*self.guild_id.read()),
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
            direction: self.direction,
        }
    }
}
impl Clone for Position {
    fn clone(&self) -> Self {
        Self {
            x: self.x,
            y: self.y,
        }
    }
}
impl Clone for LevelStats {
    fn clone(&self) -> Self {
        Self {
            base_level: self.base_level,
            job_level: self.job_level,
            base_exp: self.base_exp,
            job_exp: self.job_exp,
            status_point: self.status_point,
            skill_point: self.skill_point,
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
                direction: 0,
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
                status_point: char.status_point,
                skill_point: char.skill_point,
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
                max_weight: constants::BASE_MAX_WEIGHT
                    + (char.str as u32) * constants::WEIGHT_PER_STR,
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
            party_id: RwLock::new(None),
            guild_id: RwLock::new(None),
        }
    }

    /// 从 CharacterTransfer 创建 Player（用于跨进程 CharToMap 传输）
    pub fn from_character_transfer(transfer: &crate::game::inter_server::CharacterTransfer) -> Self {
        Self {
            id: Uuid::new_v4(),
            char_id: transfer.char_id,
            account_id: transfer.account_id,
            name: transfer.name.clone(),
            map_name: transfer.map_name.clone(),
            combat: RwLock::new(CombatStats {
                hp: transfer.hp,
                max_hp: transfer.max_hp,
                sp: transfer.sp,
                max_sp: transfer.max_sp,
                state: PlayerState::Alive,
                in_combat: false,
                is_sitting: false,
                walk_speed: constants::DEFAULT_WALK_SPEED,
                direction: 0,
            }),
            pos: RwLock::new(Position {
                x: transfer.pos_x,
                y: transfer.pos_y,
            }),
            level: RwLock::new(LevelStats {
                base_level: transfer.level,
                job_level: 1,
                base_exp: 0,
                job_exp: 0,
                status_point: 0,
                skill_point: 0,
            }),
            attrs: RwLock::new(Attributes {
                str: transfer.str as u16,
                agi: transfer.agi as u16,
                vit: transfer.vit as u16,
                int: transfer.int as u16,
                dex: transfer.dex as u16,
                luk: transfer.luk as u16,
            }),
            economy: RwLock::new(Economy {
                zeny: transfer.zeny,
                current_weight: 0,
                max_weight: constants::BASE_MAX_WEIGHT
                    + (transfer.str as u32) * constants::WEIGHT_PER_STR,
                job: transfer.job,
                shop_id: None,
                group_id: 0,
            }),
            save_point: RwLock::new(SavePoint {
                map: transfer.save_map.clone(),
                x: transfer.save_x,
                y: transfer.save_y,
            }),
            equipment: RwLock::new(Equipment::new()),
            status: PlayerStatus::new(Uuid::new_v4()),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
            party_id: RwLock::new(None),
            guild_id: RwLock::new(None),
        }
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
    /// 获取可分配的状态点数
    pub fn status_point(&self) -> u16 {
        self.level.read().status_point
    }
    /// 获取可分配的技能点数
    pub fn skill_point(&self) -> u16 {
        self.level.read().skill_point
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

    /// 计算最终魔法防御（装备 + INT/2 + 状态修正）
    pub fn mdef(&self, item_db: &ItemDatabase) -> u32 {
        use crate::game::status::StatusCalculator;
        let equipment = self.equipment.read();
        let equipment_mdef = equipment.total_magic_defense(item_db) as i32;
        let base_mdef = (self.int() as i32) / 2 + equipment_mdef;
        let modifiers = self.status.modifiers();
        StatusCalculator::calculate_mdef(base_mdef, &modifiers).max(0) as u32
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

    // --- 社交关系 ---
    pub fn party_id(&self) -> Option<Uuid> {
        *self.party_id.read()
    }
    pub fn set_party_id(&self, id: Option<Uuid>) {
        *self.party_id.write() = id;
    }
    pub fn guild_id(&self) -> Option<Uuid> {
        *self.guild_id.read()
    }
    pub fn set_guild_id(&self, id: Option<Uuid>) {
        *self.guild_id.write() = id;
    }

    // ==================== 存档点相关 ====================

    pub fn get_save_point(&self) -> (u16, u16) {
        let sp = self.save_point.read();
        (sp.x, sp.y)
    }

    pub fn get_save_map(&self) -> String {
        self.save_point.read().map.clone()
    }

    pub fn set_save_point(&self) {
        let pos = self.pos.read();
        let mut sp = self.save_point.write();
        sp.map = self.map_name.clone();
        sp.x = pos.x;
        sp.y = pos.y;
    }

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
    pub fn set_combat(&self, in_combat: bool) {
        self.combat.write().in_combat = in_combat;
    }

    /// 获取朝向方向（0-7）
    pub fn direction(&self) -> u16 {
        self.combat.read().direction
    }

    /// 设置朝向方向（0-7），超过7则取模
    pub fn set_direction(&self, dir: u16) {
        self.combat.write().direction = dir % 8;
    }

    /// 设置职业ID
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

    /// 恢复 SP，不超过上限
    pub fn apply_sp_heal(&self, amount: u32) {
        let mut c = self.combat.write();
        let heal_amount = amount.min(c.max_sp - c.sp);
        c.sp += heal_amount;
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
            class: eco.job,
            zeny: eco.zeny,
            str: s.str,
            agi: s.agi,
            vit: s.vit,
            int: s.int,
            dex: s.dex,
            luk: s.luk,
            status_point: lvl.status_point,
            skill_point: lvl.skill_point,
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
            save_data.class,
            save_data.zeny,
            save_data.str,
            save_data.agi,
            save_data.vit,
            save_data.int,
            save_data.dex,
            save_data.luk,
            save_data.status_point,
            save_data.skill_point,
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
    /// 职业 ID（class 字段）
    pub(crate) class: u16,
    pub(crate) zeny: u32,
    pub(crate) str: u16,
    pub(crate) agi: u16,
    pub(crate) vit: u16,
    pub(crate) int: u16,
    pub(crate) dex: u16,
    pub(crate) luk: u16,
    pub(crate) status_point: u16,
    pub(crate) skill_point: u16,
    pub(crate) status_effects: Vec<crate::game::status::effect::StatusEffect>,
    pub(crate) inventory: Vec<CharacterInventoryData>,
    pub(crate) hotkeys: Vec<CharacterHotkeyData>,
}

impl Player {
    /// 从玩家数据构建脚本上下文
    pub fn to_script_context(&self) -> ScriptContext {
        let combat = self.combat();
        let level = self.level_stats();
        let attrs = self.attributes();
        let economy = self.economy();

        // 构建背包物品映射
        let inventory = self.inventory.read();
        let mut inventory_map = std::collections::HashMap::new();
        for item in inventory.iter() {
            *inventory_map.entry(item.item_id).or_insert(0) += 1;
        }

        ScriptContext {
            char_id: self.char_id,
            char_name: self.name.clone(),
            guild_name: String::new(), // TODO: 从公会系统获取名称
            party_name: String::new(), // TODO: 从组队系统获取名称
            base_level: level.base_level,
            job_level: level.job_level,
            zeny: economy.zeny,
            current_hp: combat.hp,
            max_hp: combat.max_hp,
            current_sp: combat.sp,
            max_sp: combat.max_sp,
            inventory: inventory_map,
            npc_variables: std::collections::HashMap::new(),
            need_broadcast: false,
            broadcast_message: String::new(),
            party_id: self.party_id().map(|id| id.as_u128() as u32).unwrap_or(0),
            guild_id: self.guild_id().map(|id| id.as_u128() as u32).unwrap_or(0),
            account_id: self.account_id,
            str: attrs.str,
            agi: attrs.agi,
            vit: attrs.vit,
            int_: attrs.int,
            dex: attrs.dex,
            luk: attrs.luk,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                direction: 0,
            }),
            pos: RwLock::new(Position { x: 100, y: 100 }),
            level: RwLock::new(LevelStats {
                base_level: 10,
                job_level: 5,
                base_exp: 5000,
                job_exp: 3000,
                status_point: 100,
                skill_point: 0,
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
            party_id: RwLock::new(None),
            guild_id: RwLock::new(None),
        }
    }

    #[test]
    fn test_mdef_from_int_and_equipment() {
        use crate::game::item::{EquipSlot, InventorySlot, Item, ItemDatabase, ItemType};

        let player = make_player();
        // make_player 默认 int=1 => base mdef = 1/2 = 0
        let item_db = ItemDatabase::new();
        assert_eq!(player.mdef(&item_db), 0);

        // 设置 INT=10 => base mdef = 10/2 = 5
        {
            let mut attrs = player.attrs.write();
            attrs.int = 10;
        }
        assert_eq!(player.mdef(&item_db), 5);

        // 装备 +20 MDEF 的铠甲
        let mut db = ItemDatabase::new();
        db.insert(Item {
            id: 9901,
            name: "Mage Robe".to_string(),
            type_: ItemType::Armor,
            magic_defense: 20,
            ..Default::default()
        });
        {
            let mut eq = player.equipment.write();
            eq.equip(
                EquipSlot::Body,
                InventorySlot {
                    index: 0,
                    item_id: 9901,
                    amount: 1,
                    identified: true,
                    refine: 0,
                    cards: [0; 4],
                },
            );
        }

        // base = 5 + 20 = 25，无状态修正
        assert_eq!(player.mdef(&db), 25);
    }
}
