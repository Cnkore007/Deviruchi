use parking_lot::RwLock;
use uuid::Uuid;

/// 怪物类型
#[derive(Debug, Clone, Copy)]
pub enum MobType {
    Normal,
    Boss,
    Guardian,
    Event,
}

/// 怪物AI状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobAIState {
    Idle,
    Patrol,
    Chase,
    Attack,
    Return,
    Dead,
}

/// 怪物数据
#[derive(Debug)]
pub struct Mob {
    pub id: Uuid,
    pub mob_id: u16,
    pub name: String,
    pub pos_x: RwLock<u16>,
    pub pos_y: RwLock<u16>,
    pub map_name: String,

    // 属性
    pub level: u16,
    pub hp: RwLock<u32>,
    pub max_hp: u32,
    pub sp: RwLock<u32>,
    pub max_sp: u32,

    // 战斗属性
    pub atk: u16,
    pub matk: u16,
    pub defense: u16,
    pub magic_defense: u16,
    pub hit: i16,
    pub flee: i16,
    pub crit: i16,
    pub walk_speed: u16,
    pub atk_range: u16,

    // AI状态
    pub ai_state: RwLock<MobAIState>,
    pub target_id: RwLock<Option<Uuid>>,

    // AI参数
    pub sight_range: u16,
    pub chase_range: u16,
    pub aggro_rate: i16,

    // 刷新参数
    pub spawn_delay: u32,
    pub respawn_time: u32,
}

impl Mob {
    pub fn new(mob_id: u16, x: u16, y: u16, map: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            mob_id,
            name: format!("Mob_{}", mob_id),
            pos_x: RwLock::new(x),
            pos_y: RwLock::new(y),
            map_name: map.to_string(),
            level: 1,
            hp: RwLock::new(100),
            max_hp: 100,
            sp: RwLock::new(0),
            max_sp: 0,
            atk: 10,
            matk: 0,
            defense: 0,
            magic_defense: 0,
            hit: 0,
            flee: 0,
            crit: 0,
            walk_speed: 150,
            atk_range: 1,
            ai_state: RwLock::new(MobAIState::Idle),
            target_id: RwLock::new(None),
            sight_range: 12,
            chase_range: 20,
            aggro_rate: 0,
            spawn_delay: 0,
            respawn_time: 60000,
        }
    }

    pub fn from_template(mob_id: u16, x: u16, y: u16, map: &str) -> Self {
        let template = MobDatabase::get(mob_id);
        Self {
            id: Uuid::new_v4(),
            mob_id,
            name: template.name.to_string(),
            pos_x: RwLock::new(x),
            pos_y: RwLock::new(y),
            map_name: map.to_string(),
            level: template.level,
            hp: RwLock::new(template.hp),
            max_hp: template.hp,
            sp: RwLock::new(template.sp),
            max_sp: template.sp,
            atk: template.atk,
            matk: template.matk,
            defense: template.defense,
            magic_defense: template.magic_defense,
            hit: template.hit,
            flee: template.flee,
            crit: template.crit,
            walk_speed: template.walk_speed,
            atk_range: template.atk_range,
            ai_state: RwLock::new(MobAIState::Idle),
            target_id: RwLock::new(None),
            sight_range: template.sight_range,
            chase_range: template.chase_range,
            aggro_rate: template.aggro_rate,
            spawn_delay: template.spawn_delay,
            respawn_time: template.respawn_time,
        }
    }

    pub fn get_position(&self) -> (u16, u16) {
        (*self.pos_x.read(), *self.pos_y.read())
    }

    pub fn move_to(&self, x: u16, y: u16) {
        *self.pos_x.write() = x;
        *self.pos_y.write() = y;
    }

    pub fn take_damage(&self, damage: u32) -> bool {
        let current_hp = *self.hp.read();
        if current_hp <= damage {
            *self.hp.write() = 0;
            *self.ai_state.write() = MobAIState::Dead;
            true
        } else {
            *self.hp.write() = current_hp - damage;
            false
        }
    }

    pub fn is_dead(&self) -> bool {
        *self.hp.read() == 0
    }
}

/// 怪物数据库
pub struct MobDatabase;

impl MobDatabase {
    pub fn get(mob_id: u16) -> MobTemplate {
        match mob_id {
            1001 => MobTemplate {
                name: "Poring".to_string(),
                level: 1,
                hp: 50,
                sp: 0,
                atk: 7,
                matk: 0,
                defense: 0,
                magic_defense: 0,
                hit: 7,
                flee: 5,
                crit: 0,
                walk_speed: 150,
                atk_range: 1,
                sight_range: 12,
                chase_range: 20,
                aggro_rate: 0,
                spawn_delay: 1000,
                respawn_time: 60000,
            },
            1002 => MobTemplate {
                name: "Lunatic".to_string(),
                level: 3,
                hp: 80,
                sp: 0,
                atk: 12,
                matk: 0,
                defense: 0,
                magic_defense: 0,
                hit: 12,
                flee: 10,
                crit: 5,
                walk_speed: 200,
                atk_range: 1,
                sight_range: 12,
                chase_range: 20,
                aggro_rate: 0,
                spawn_delay: 1000,
                respawn_time: 60000,
            },
            1003 => MobTemplate {
                name: "Blue Poring".to_string(),
                level: 2,
                hp: 60,
                sp: 0,
                atk: 8,
                matk: 5,
                defense: 0,
                magic_defense: 5,
                hit: 8,
                flee: 7,
                crit: 0,
                walk_speed: 150,
                atk_range: 1,
                sight_range: 12,
                chase_range: 20,
                aggro_rate: 0,
                spawn_delay: 1000,
                respawn_time: 60000,
            },
            1312 => MobTemplate {
                name: "Fabre".to_string(),
                level: 4,
                hp: 120,
                sp: 0,
                atk: 15,
                matk: 0,
                defense: 0,
                magic_defense: 0,
                hit: 15,
                flee: 12,
                crit: 0,
                walk_speed: 150,
                atk_range: 1,
                sight_range: 12,
                chase_range: 20,
                aggro_rate: 0,
                spawn_delay: 1000,
                respawn_time: 60000,
            },
            _ => MobTemplate::default(mob_id),
        }
    }
}

/// 怪物模板
#[derive(Debug, Clone)]
pub struct MobTemplate {
    pub name: String,
    pub level: u16,
    pub hp: u32,
    pub sp: u32,
    pub atk: u16,
    pub matk: u16,
    pub defense: u16,
    pub magic_defense: u16,
    pub hit: i16,
    pub flee: i16,
    pub crit: i16,
    pub walk_speed: u16,
    pub atk_range: u16,
    pub sight_range: u16,
    pub chase_range: u16,
    pub aggro_rate: i16,
    pub spawn_delay: u32,
    pub respawn_time: u32,
}

impl MobTemplate {
    fn default(mob_id: u16) -> Self {
        Self {
            name: format!("Unknown_{}", mob_id),
            level: 1,
            hp: 50,
            sp: 0,
            atk: 10,
            matk: 0,
            defense: 0,
            magic_defense: 0,
            hit: 10,
            flee: 10,
            crit: 0,
            walk_speed: 150,
            atk_range: 1,
            sight_range: 12,
            chase_range: 20,
            aggro_rate: 0,
            spawn_delay: 1000,
            respawn_time: 60000,
        }
    }
}
