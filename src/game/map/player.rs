use crate::game::item::Equipment;
use crate::game::status::{PlayerStatus, StatusChange, StatusEffect, StatusSource};
use crate::storage::Character;
use crate::storage::character::{CharacterHotkeyData, CharacterInventoryData};
use parking_lot::RwLock;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    Alive,
    Dead,
}

pub struct Player {
    pub id: Uuid,
    pub char_id: u32,
    pub account_id: u32,
    pub name: String,
    pub pos_x: RwLock<u16>,
    pub pos_y: RwLock<u16>,
    pub map_name: String,
    pub hp: RwLock<u32>,
    pub max_hp: RwLock<u32>,
    pub sp: RwLock<u32>,
    pub max_sp: RwLock<u32>,
    pub base_level: RwLock<u16>,
    pub job_level: RwLock<u16>,
    pub base_exp: RwLock<u64>,
    pub job_exp: RwLock<u64>,
    pub state: RwLock<PlayerState>,
    pub str: RwLock<u16>,
    pub agi: RwLock<u16>,
    pub vit: RwLock<u16>,
    pub int: RwLock<u16>,
    pub dex: RwLock<u16>,
    pub luk: RwLock<u16>,
    pub walk_speed: RwLock<u16>,
    pub zeny: RwLock<u32>,
    pub current_weight: RwLock<u32>,
    pub max_weight: RwLock<u32>,
    pub equipment: RwLock<Equipment>,
    pub is_sitting: RwLock<bool>,
    /// 状态效果管理器
    pub status: PlayerStatus,
    /// 当前摆摊商店ID
    pub shop_id: RwLock<Option<Uuid>>,
    /// 物品栏数据
    pub inventory: RwLock<Vec<CharacterInventoryData>>,
    /// 快捷键数据
    pub hotkeys: RwLock<Vec<CharacterHotkeyData>>,
    /// 存档点 - 地图名
    pub save_map: RwLock<String>,
    /// 存档点 - X坐标
    pub save_x: RwLock<u16>,
    /// 存档点 - Y坐标
    pub save_y: RwLock<u16>,
    /// 职业ID
    pub job: RwLock<u16>,
    /// 是否处于战斗中
    pub in_combat: RwLock<bool>,
    /// 账户权限等级 (0=玩家, 10=GM, 50=Admin, 99=SuperAdmin)
    pub group_id: RwLock<i32>,
}

impl Clone for Player {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            char_id: self.char_id,
            account_id: self.account_id,
            name: self.name.clone(),
            pos_x: RwLock::new(*self.pos_x.read()),
            pos_y: RwLock::new(*self.pos_y.read()),
            map_name: self.map_name.clone(),
            hp: RwLock::new(*self.hp.read()),
            max_hp: RwLock::new(*self.max_hp.read()),
            sp: RwLock::new(*self.sp.read()),
            max_sp: RwLock::new(*self.max_sp.read()),
            base_level: RwLock::new(*self.base_level.read()),
            job_level: RwLock::new(*self.job_level.read()),
            base_exp: RwLock::new(*self.base_exp.read()),
            job_exp: RwLock::new(*self.job_exp.read()),
            state: RwLock::new(*self.state.read()),
            str: RwLock::new(*self.str.read()),
            agi: RwLock::new(*self.agi.read()),
            vit: RwLock::new(*self.vit.read()),
            int: RwLock::new(*self.int.read()),
            dex: RwLock::new(*self.dex.read()),
            luk: RwLock::new(*self.luk.read()),
            walk_speed: RwLock::new(*self.walk_speed.read()),
            zeny: RwLock::new(*self.zeny.read()),
            current_weight: RwLock::new(*self.current_weight.read()),
            max_weight: RwLock::new(*self.max_weight.read()),
            equipment: RwLock::new(self.equipment.read().clone()),
            is_sitting: RwLock::new(*self.is_sitting.read()),
            status: self.status.clone(),
            shop_id: RwLock::new(*self.shop_id.read()),
            inventory: RwLock::new(self.inventory.read().clone()),
            hotkeys: RwLock::new(self.hotkeys.read().clone()),
            save_map: RwLock::new(self.save_map.read().clone()),
            save_x: RwLock::new(*self.save_x.read()),
            save_y: RwLock::new(*self.save_y.read()),
            job: RwLock::new(*self.job.read()),
            in_combat: RwLock::new(*self.in_combat.read()),
            group_id: RwLock::new(*self.group_id.read()),
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
            pos_x: RwLock::new(char.last_x as u16),
            pos_y: RwLock::new(char.last_y as u16),
            map_name: char.last_map.clone(),
            hp: RwLock::new(char.hp),
            max_hp: RwLock::new(char.max_hp),
            sp: RwLock::new(char.sp),
            max_sp: RwLock::new(char.max_sp),
            base_level: RwLock::new(char.base_level),
            job_level: RwLock::new(char.job_level),
            base_exp: RwLock::new(char.base_exp as u64),
            job_exp: RwLock::new(char.job_exp as u64),
            state: RwLock::new(PlayerState::Alive),
            str: RwLock::new(char.str),
            agi: RwLock::new(char.agi),
            vit: RwLock::new(char.vit),
            int: RwLock::new(char.int),
            dex: RwLock::new(char.dex),
            luk: RwLock::new(char.luk),
            walk_speed: RwLock::new(150),
            zeny: RwLock::new(char.zeny),
            current_weight: RwLock::new(0),
            max_weight: RwLock::new(20000 + (char.str as u32) * 200),
            equipment: RwLock::new(Equipment::new()),
            is_sitting: RwLock::new(false),
            status: PlayerStatus::new(Uuid::new_v4()),
            shop_id: RwLock::new(None),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
            save_map: RwLock::new(char.save_map.clone()),
            save_x: RwLock::new(char.save_x as u16),
            save_y: RwLock::new(char.save_y as u16),
            job: RwLock::new(char.class),
            in_combat: RwLock::new(false),
            group_id: RwLock::new(0),
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
            pos_x: RwLock::new(char.last_x as u16),
            pos_y: RwLock::new(char.last_y as u16),
            map_name: char.last_map.clone(),
            hp: RwLock::new(char.hp),
            max_hp: RwLock::new(char.max_hp),
            sp: RwLock::new(char.sp),
            max_sp: RwLock::new(char.max_sp),
            base_level: RwLock::new(char.base_level),
            job_level: RwLock::new(char.job_level),
            base_exp: RwLock::new(char.base_exp),
            job_exp: RwLock::new(char.job_exp),
            state: RwLock::new(PlayerState::Alive),
            str: RwLock::new(char.str),
            agi: RwLock::new(char.agi),
            vit: RwLock::new(char.vit),
            int: RwLock::new(char.int),
            dex: RwLock::new(char.dex),
            luk: RwLock::new(char.luk),
            walk_speed: RwLock::new(150),
            zeny: RwLock::new(char.zeny),
            current_weight: RwLock::new(0),
            max_weight: RwLock::new(20000 + (char.str as u32) * 200),
            equipment: RwLock::new(Equipment::new()),
            is_sitting: RwLock::new(false),
            status: PlayerStatus::new(Uuid::new_v4()),
            shop_id: RwLock::new(None),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
            save_map: RwLock::new(char.save_map.clone()),
            save_x: RwLock::new(char.save_x as u16),
            save_y: RwLock::new(char.save_y as u16),
            job: RwLock::new(char.job),
            in_combat: RwLock::new(false),
            group_id: RwLock::new(0),
        })
    }

    // ==================== 存档点相关 ====================

    /// 获取存档点坐标
    #[allow(dead_code)]
    pub fn get_save_point(&self) -> (u16, u16) {
        (*self.save_x.read(), *self.save_y.read())
    }

    /// 获取存档点地图
    #[allow(dead_code)]
    pub fn get_save_map(&self) -> String {
        self.save_map.read().clone()
    }

    /// 设置存档点
    #[allow(dead_code)]
    pub fn set_save_point(&self) {
        *self.save_map.write() = self.map_name.clone();
        *self.save_x.write() = *self.pos_x.read();
        *self.save_y.write() = *self.pos_y.read();
    }

    /// 设置存档点（指定位置）
    #[allow(dead_code)]
    pub fn set_save_point_at(&self, map: &str, x: u16, y: u16) {
        *self.save_map.write() = map.to_string();
        *self.save_x.write() = x;
        *self.save_y.write() = y;
    }

    /// 移动到指定位置
    pub fn move_to(&self, x: u16, y: u16) {
        *self.pos_x.write() = x;
        *self.pos_y.write() = y;
    }

    /// 获取当前位置
    pub fn get_position(&self) -> (u16, u16) {
        (*self.pos_x.read(), *self.pos_y.read())
    }

    /// 受到伤害，返回是否死亡
    pub fn take_damage(&self, damage: u32) -> bool {
        let current_hp = *self.hp.read();
        if current_hp <= damage {
            *self.hp.write() = 0;
            *self.state.write() = PlayerState::Dead;
            self.apply_death_penalty();
            true
        } else {
            *self.hp.write() = current_hp - damage;
            false
        }
    }

    /// 角色死亡：设置状态并施加死亡惩罚
    pub fn die(&self) {
        *self.hp.write() = 0;
        *self.state.write() = PlayerState::Dead;
        self.apply_death_penalty();
    }

    /// 重生：恢复 HP/SP 并设置状态为 Alive
    pub fn respawn(&self, x: u16, y: u16) {
        *self.hp.write() = *self.max_hp.read();
        *self.sp.write() = *self.max_sp.read();
        *self.pos_x.write() = x;
        *self.pos_y.write() = y;
        *self.state.write() = PlayerState::Alive;
        *self.is_sitting.write() = false;
    }

    /// 是否死亡
    pub fn is_dead(&self) -> bool {
        *self.state.read() == PlayerState::Dead
    }

    /// 是否存活
    pub fn is_alive(&self) -> bool {
        *self.state.read() == PlayerState::Alive
    }

    /// 是否在战斗中
    pub fn is_in_combat(&self) -> bool {
        *self.in_combat.read()
    }

    /// 设置战斗状态
    #[allow(dead_code)]
    pub fn set_combat(&self, in_combat: bool) {
        *self.in_combat.write() = in_combat;
    }

    /// 获取职业ID
    pub fn get_job(&self) -> u16 {
        *self.job.read()
    }

    /// 设置职业ID
    #[allow(dead_code)]
    pub fn set_job(&self, job: u16) {
        *self.job.write() = job;
    }

    /// 坐下
    pub fn sit(&self) {
        *self.is_sitting.write() = true;
    }

    /// 站起
    pub fn stand(&self) {
        *self.is_sitting.write() = false;
    }

    /// 是否坐下
    pub fn is_sitting(&self) -> bool {
        *self.is_sitting.read()
    }

    /// 施加死亡惩罚：损失 1% 当前经验值
    fn apply_death_penalty(&self) {
        let base = *self.base_exp.read();
        let job = *self.job_exp.read();
        *self.base_exp.write() = base.saturating_sub(base / 100);
        *self.job_exp.write() = job.saturating_sub(job / 100);
    }

    /// 死亡时掉落 Zeny（默认掉落50%）
    ///
    /// 返回掉落的 Zeny 数量
    pub fn drop_zeny_on_death(&self) -> u32 {
        let zeny = *self.zeny.read();
        let drop_amount = zeny / 2; // 50%
        if drop_amount > 0 {
            *self.zeny.write() = zeny - drop_amount;
        }
        drop_amount
    }

    /// 获得基础经验值
    pub fn add_base_exp(&self, exp: u64) {
        *self.base_exp.write() += exp;
    }

    /// 获得职业经验值
    pub fn add_job_exp(&self, exp: u64) {
        *self.job_exp.write() += exp;
    }

    /// 获得 Zeny
    pub fn add_zeny(&self, zeny: u64) {
        *self.zeny.write() += zeny as u32;
    }

    /// 计算最大负重 (基础20000 + STR*200, 单位0.1)
    pub fn calc_max_weight(&self) -> u32 {
        let str = *self.str.read();
        20000 + (str as u32) * 200
    }

    /// 更新最大负重
    pub fn update_max_weight(&self) {
        let new_max = self.calc_max_weight();
        *self.max_weight.write() = new_max;
    }

    /// 检查是否超重(50%)
    pub fn is_overweight(&self) -> bool {
        let current = *self.current_weight.read();
        let max = *self.max_weight.read();
        current > max * 50 / 100
    }

    /// 检查是否严重超重(90%)
    pub fn is_overweight_90(&self) -> bool {
        let current = *self.current_weight.read();
        let max = *self.max_weight.read();
        current > max * 90 / 100
    }

    // ==================== 状态效果管理 ====================

    /// 添加状态效果
    pub fn add_status(&self, effect: StatusEffect) -> Option<StatusEffect> {
        self.status.add_status(effect)
    }

    /// 移除状态效果
    pub fn remove_status(&self, status: StatusChange) -> Option<StatusEffect> {
        self.status.remove_status(status)
    }

    /// 检查是否有指定状态
    pub fn has_status(&self, status: StatusChange) -> bool {
        self.status.has_status(status)
    }

    /// 获取状态效果
    pub fn get_status(&self, status: StatusChange) -> Option<StatusEffect> {
        self.status.get_status(status)
    }

    /// 清除所有状态效果
    pub fn clear_all_status(&self) {
        self.status.clear_all();
    }

    /// 清除所有指定分类的状态
    pub fn clear_status_by_category(&self, category: crate::game::status::StatusCategory) {
        self.status.clear_by_category(category);
    }

    /// 获取所有活跃状态
    pub fn get_all_statuses(&self) -> Vec<StatusEffect> {
        self.status.get_all_statuses()
    }

    /// 是否有战斗限制状态（眩晕、冰冻、睡眠、石化、混乱）
    pub fn has_combat_restriction(&self) -> bool {
        self.status.has_combat_restriction()
    }

    /// 是否被沉默
    pub fn is_silenced(&self) -> bool {
        self.status.is_silenced()
    }

    /// 是否无敌
    pub fn is_invincible(&self) -> bool {
        self.status.is_invincible()
    }

    /// 是否隐身
    pub fn is_invisible(&self) -> bool {
        self.status.is_invisible()
    }

    /// 清除所有已过期的状态
    pub fn cleanup_expired_status(&self) -> Vec<StatusChange> {
        self.status.cleanup_expired()
    }

    /// 应用常见BUFF（快捷方法）
    pub fn apply_blessing(&self, level: u8) {
        let effect = StatusEffect::with_values(
            StatusChange::Blessing,
            (level as u64 * 30000).min(300000), // 30秒 * 等级，最大5分钟
            StatusSource::Skill(9),             // SM_BLESSING skill ID
            level as i32,
            0,
            0,
        );
        self.add_status(effect);
    }

    /// 应用加速术
    pub fn apply_haste(&self, level: u8) {
        let effect = StatusEffect::with_values(
            StatusChange::Haste,
            (level as u64 * 30000).min(300000),
            StatusSource::Skill(29), // AC_CONCENTRATION or HASTE skill ID
            level as i32 * 10,
            0,
            0,
        );
        self.add_status(effect);
    }

    /// 应用治愈术
    pub fn apply_heal(&self, amount: u32) {
        let current_hp = *self.hp.read();
        let max_hp = *self.max_hp.read();
        let heal_amount = amount.min(max_hp - current_hp);
        *self.hp.write() = current_hp + heal_amount;
    }

    /// 检查是否可以执行动作（考虑状态限制）
    pub fn can_act(&self) -> bool {
        if !self.is_alive() {
            return false;
        }
        !self.has_combat_restriction()
    }

    /// 检查是否可以攻击
    pub fn can_attack(&self) -> bool {
        self.can_act() && !self.has_status(StatusChange::Silence)
    }

    /// 检查是否可以移动
    pub fn can_move(&self) -> bool {
        self.can_act()
    }

    /// 检查是否可以施放技能
    pub fn can_cast(&self) -> bool {
        self.can_act() && !self.is_silenced()
    }

    /// ==================== 数据持久化 ====================
    /// 转换为可保存的数据格式
    pub fn to_save_data(&self) -> PlayerSaveData {
        PlayerSaveData {
            char_id: self.char_id,
            last_map: self.map_name.clone(),
            last_x: *self.pos_x.read() as i32,
            last_y: *self.pos_y.read() as i32,
            hp: *self.hp.read(),
            max_hp: *self.max_hp.read(),
            sp: *self.sp.read(),
            max_sp: *self.max_sp.read(),
            base_exp: *self.base_exp.read(),
            job_exp: *self.job_exp.read(),
            base_level: *self.base_level.read(),
            job_level: *self.job_level.read(),
            zeny: *self.zeny.read(),
            str: *self.str.read(),
            agi: *self.agi.read(),
            vit: *self.vit.read(),
            int: *self.int.read(),
            dex: *self.dex.read(),
            luk: *self.luk.read(),
            status_effects: self.status.get_all_statuses(),
            inventory: self.inventory.read().clone(),
            hotkeys: self.hotkeys.read().clone(),
        }
    }

    /// 保存到数据库
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
    pub char_id: u32,
    pub last_map: String,
    pub last_x: i32,
    pub last_y: i32,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub base_exp: u64,
    pub job_exp: u64,
    pub base_level: u16,
    pub job_level: u16,
    pub zeny: u32,
    pub str: u16,
    pub agi: u16,
    pub vit: u16,
    pub int: u16,
    pub dex: u16,
    pub luk: u16,
    pub status_effects: Vec<crate::game::status::effect::StatusEffect>,
    pub inventory: Vec<CharacterInventoryData>,
    pub hotkeys: Vec<CharacterHotkeyData>,
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
            pos_x: RwLock::new(100),
            pos_y: RwLock::new(100),
            map_name: "test_map".to_string(),
            hp: RwLock::new(100),
            max_hp: RwLock::new(100),
            sp: RwLock::new(50),
            max_sp: RwLock::new(50),
            base_level: RwLock::new(10),
            job_level: RwLock::new(5),
            base_exp: RwLock::new(5000),
            job_exp: RwLock::new(3000),
            state: RwLock::new(PlayerState::Alive),
            str: RwLock::new(1),
            agi: RwLock::new(1),
            vit: RwLock::new(1),
            int: RwLock::new(1),
            dex: RwLock::new(1),
            luk: RwLock::new(1),
            walk_speed: RwLock::new(150),
            zeny: RwLock::new(0),
            current_weight: RwLock::new(0),
            max_weight: RwLock::new(20000),
            equipment: RwLock::new(Equipment::new()),
            is_sitting: RwLock::new(false),
            status: PlayerStatus::new(Uuid::new_v4()),
            shop_id: RwLock::new(None),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
            save_map: RwLock::new("test_map".to_string()),
            save_x: RwLock::new(50),
            save_y: RwLock::new(50),
            job: RwLock::new(0),
            in_combat: RwLock::new(false),
            group_id: RwLock::new(0),
        }
    }

    #[test]
    fn test_player_die_sets_state_dead() {
        let player = make_player();
        assert!(player.is_alive());
        player.die();
        assert!(player.is_dead());
        assert_eq!(*player.hp.read(), 0);
    }

    #[test]
    fn test_player_die_applies_exp_penalty() {
        let player = make_player();
        assert_eq!(*player.base_exp.read(), 5000);
        assert_eq!(*player.job_exp.read(), 3000);

        player.die();

        // 1% loss: 5000 - 50 = 4950, 3000 - 30 = 2970
        assert_eq!(*player.base_exp.read(), 4950);
        assert_eq!(*player.job_exp.read(), 2970);
    }

    #[test]
    fn test_player_die_small_exp_clamped_to_zero() {
        let player = make_player();
        *player.base_exp.write() = 50; // 1% = 0, saturating_sub keeps 50
        *player.job_exp.write() = 30; // 1% = 0, saturating_sub keeps 30

        player.die();

        // 50 - 0 = 50, 30 - 0 = 30 (50/100 = 0 in integer division)
        assert_eq!(*player.base_exp.read(), 50);
        assert_eq!(*player.job_exp.read(), 30);
    }

    #[test]
    fn test_player_respawn_restores_hp_and_state() {
        let player = make_player();
        player.die();
        assert!(player.is_dead());

        player.respawn(50, 60);
        assert!(player.is_alive());
        assert_eq!(*player.hp.read(), 100); // max_hp
        assert_eq!(*player.sp.read(), 50); // max_sp
        assert_eq!(player.get_position(), (50, 60));
    }

    #[test]
    fn test_take_damage_killing_blow_triggers_death() {
        let player = make_player();
        assert!(player.is_alive());

        let killed = player.take_damage(200); // > 100 hp
        assert!(killed);
        assert!(player.is_dead());
        assert_eq!(*player.hp.read(), 0);
    }

    #[test]
    fn test_take_damage_non_lethal_keeps_alive() {
        let player = make_player();
        let killed = player.take_damage(30);
        assert!(!killed);
        assert!(player.is_alive());
        assert_eq!(*player.hp.read(), 70);
    }

    #[test]
    fn test_add_base_exp() {
        let player = make_player();
        player.add_base_exp(500);
        assert_eq!(*player.base_exp.read(), 5500);
    }

    #[test]
    fn test_add_job_exp() {
        let player = make_player();
        player.add_job_exp(200);
        assert_eq!(*player.job_exp.read(), 3200);
    }

    #[test]
    fn test_player_drop_zeny_on_death_drops_50_percent() {
        let player = make_player();
        *player.zeny.write() = 1000;

        let dropped = player.drop_zeny_on_death();

        assert_eq!(dropped, 500); // 50% of 1000
        assert_eq!(*player.zeny.read(), 500); // Remaining 50%
    }

    #[test]
    fn test_player_drop_zeny_on_death_small_zeny() {
        let player = make_player();
        *player.zeny.write() = 1;

        let dropped = player.drop_zeny_on_death();

        // 1 / 2 = 0 in integer division, so no zeny is dropped
        assert_eq!(dropped, 0);
        assert_eq!(*player.zeny.read(), 1); // All zeny remains
    }

    #[test]
    fn test_player_drop_zeny_on_death_zero_zeny() {
        let player = make_player();
        *player.zeny.write() = 0;

        let dropped = player.drop_zeny_on_death();

        assert_eq!(dropped, 0);
        assert_eq!(*player.zeny.read(), 0);
    }

    #[test]
    fn test_player_drop_zeny_on_death_odd_number() {
        let player = make_player();
        *player.zeny.write() = 101;

        let dropped = player.drop_zeny_on_death();

        // 101 / 2 = 50 (floor division)
        assert_eq!(dropped, 50);
        assert_eq!(*player.zeny.read(), 51);
    }

    // ==================== 持久化测试 ====================

    #[test]
    fn test_to_save_data_captures_all_fields() {
        let player = make_player();
        *player.hp.write() = 500;
        *player.max_hp.write() = 1000;
        *player.sp.write() = 100;
        *player.max_sp.write() = 200;
        *player.base_exp.write() = 10000;
        *player.job_exp.write() = 5000;
        *player.base_level.write() = 50;
        *player.job_level.write() = 30;
        *player.zeny.write() = 100000;
        *player.str.write() = 50;
        *player.agi.write() = 40;
        *player.vit.write() = 30;
        *player.int.write() = 20;
        *player.dex.write() = 60;
        *player.luk.write() = 10;

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
        let mut player = make_player();

        // 添加状态效果
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
