use parking_lot::RwLock;
use uuid::Uuid;
use crate::storage::Character;
use crate::game::item::Equipment;

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
            true
        } else {
            *self.hp.write() = current_hp - damage;
            false
        }
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
