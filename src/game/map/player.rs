use parking_lot::RwLock;
use uuid::Uuid;
use crate::storage::Character;
use crate::game::item::Equipment;

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
            map_name: char.last_map,
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
            zeny: RwLock::new(char.zeny as u32),
            current_weight: RwLock::new(0),
            max_weight: RwLock::new(20000 + (char.str as u32) * 200),
            equipment: RwLock::new(Equipment::new()),
            is_sitting: RwLock::new(false),
        }
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
        *player.job_exp.write() = 30;  // 1% = 0, saturating_sub keeps 30

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
        assert_eq!(*player.sp.read(), 50);  // max_sp
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
}
