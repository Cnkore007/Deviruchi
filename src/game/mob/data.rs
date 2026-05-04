use parking_lot::RwLock;
use std::time::Instant;
use uuid::Uuid;
use crate::game::battle::element::{Element, ElementLevel, MobSize};
use crate::game::constants;

/// 怪物掉落物品
#[derive(Debug, Clone)]
pub struct MobDrop {
    pub item_id: u32,
    pub min_amount: u16,
    pub max_amount: u16,
    /// 掉落概率（万分比，10000 = 100%）
    pub chance: u32,
}

impl MobDrop {
    pub fn new(item_id: u32, chance: u32) -> Self {
        Self {
            item_id,
            min_amount: 1,
            max_amount: 1,
            chance,
        }
    }

    pub fn with_amount(item_id: u32, min: u16, max: u16, chance: u32) -> Self {
        Self {
            item_id,
            min_amount: min,
            max_amount: max,
            chance,
        }
    }
}

/// 怪物路径管理器
#[derive(Debug, Clone)]
pub struct MobPathManager {
    /// 是否正在追击
    pub is_chasing: bool,
    /// 追击目标坐标
    pub target_pos: Option<(u16, u16)>,
    /// 缓存的路径点列表（不含起点）
    pub cached_path: Vec<(u16, u16)>,
    /// 当前路径索引
    pub current_step: usize,
    /// 路径是否失效
    pub path_invalid: bool,
}

impl MobPathManager {
    pub fn new() -> Self {
        Self {
            is_chasing: false,
            target_pos: None,
            cached_path: Vec::new(),
            current_step: 0,
            path_invalid: false,
        }
    }

    pub fn start_chase(&mut self, target: (u16, u16)) {
        self.is_chasing = true;
        self.target_pos = Some(target);
        self.cached_path.clear();
        self.current_step = 0;
        self.path_invalid = false;
    }

    pub fn stop_chase(&mut self) {
        self.is_chasing = false;
        self.target_pos = None;
        self.cached_path.clear();
        self.current_step = 0;
        self.path_invalid = false;
    }

    pub fn set_path(&mut self, path: Vec<(u16, u16)>) {
        self.cached_path = path;
        self.current_step = 0;
        self.path_invalid = false;
    }

    pub fn invalidate(&mut self) {
        self.path_invalid = true;
    }

    pub fn advance_step(&mut self) -> Option<(u16, u16)> {
        if self.current_step < self.cached_path.len() {
            let pos = self.cached_path[self.current_step];
            self.current_step += 1;
            Some(pos)
        } else {
            None
        }
    }
}

impl Default for MobPathManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 怪物类型
#[derive(Debug, Clone, Copy)]
pub enum MobType {
    Normal,
    Boss,
    Guardian,
    Event,
}

/// 怪物行为模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobBehavior {
    /// 被动：不主动攻击，被攻击后反击
    Passive,
    /// 主动攻击：进入视野范围后主动攻击
    Aggressive,
    /// 协助：友方被攻击时加入战斗
    Assist,
    /// 被动+协助
    PassiveAssist,
    /// 逃跑：HP低于阈值时逃跑
    FleeWhenLowHp,
    /// 固定不动：不会移动，仅攻击进入范围的目标
    Immobile,
}

/// 怪物技能数据
#[derive(Debug, Clone)]
pub struct MobSkill {
    pub skill_id: u16,
    pub level: u8,
    /// 使用概率（万分比）
    pub chance: u32,
    /// 施放条件：HP百分比低于此值时使用
    pub hp_condition: Option<u32>,
    /// 冷却时间（毫秒）
    pub cooldown_ms: u64,
}

/// 怪物坐标（保证 x/y 原子性读写）
#[derive(Debug, Clone, Copy)]
pub struct MobPosition {
    pub x: u16,
    pub y: u16,
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
    pub pos: RwLock<MobPosition>,
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

    // 元素与体型
    pub element: Element,
    pub element_level: ElementLevel,
    pub size: MobSize,

    // AI状态
    pub ai_state: RwLock<MobAIState>,
    pub target_id: RwLock<Option<Uuid>>,
    pub behavior: MobBehavior,
    pub skills: Vec<MobSkill>,

    // AI参数
    pub sight_range: u16,
    pub chase_range: u16,
    pub aggro_rate: i16,

    // 刷新参数
    pub spawn_delay: u32,
    pub respawn_time: u32,

    // 出生点
    pub spawn_x: u16,
    pub spawn_y: u16,
    pub spawn_map: String,

    // 死亡时间（用于重生计时）
    pub death_time: RwLock<Option<Instant>>,

    // 掉落与经验（运行时从 template 复制）
    pub drops: Vec<MobDrop>,
    pub base_exp: u64,
    pub job_exp: u64,
    pub zeny: Option<u64>, // Zeny 掉落（运行时设置）
    pub drops_processed: RwLock<bool>,

    // 路径管理
    pub path_manager: RwLock<MobPathManager>,

    // 伤害记录（用于确定 MVP）
    pub damage_log: RwLock<std::collections::HashMap<Uuid, u64>>,
}

impl Mob {
    pub fn new(mob_id: u16, x: u16, y: u16, map: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            mob_id,
            name: format!("Mob_{}", mob_id),
            pos: RwLock::new(MobPosition { x, y }),
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
            walk_speed: constants::DEFAULT_WALK_SPEED,
            atk_range: 1,
            element: Element::Neutral,
            element_level: ElementLevel::Level1,
            size: MobSize::Medium,
            ai_state: RwLock::new(MobAIState::Idle),
            target_id: RwLock::new(None),
            behavior: MobBehavior::Passive,
            skills: Vec::new(),
            sight_range: 12,
            chase_range: 20,
            aggro_rate: 0,
            spawn_delay: 0,
            respawn_time: 60000,
            spawn_x: x,
            spawn_y: y,
            spawn_map: map.to_string(),
            death_time: RwLock::new(None),
            drops: Vec::new(),
            base_exp: 0,
            job_exp: 0,
            zeny: None,
            drops_processed: RwLock::new(false),
            path_manager: RwLock::new(MobPathManager::new()),
            damage_log: RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn from_template(mob_id: u16, x: u16, y: u16, map: &str) -> Self {
        let template = MobDatabase::get(mob_id);
        Self {
            id: Uuid::new_v4(),
            mob_id,
            name: template.name.to_string(),
            pos: RwLock::new(MobPosition { x, y }),
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
            element: template.element,
            element_level: template.element_level,
            size: template.size,
            ai_state: RwLock::new(MobAIState::Idle),
            target_id: RwLock::new(None),
            behavior: template.behavior,
            skills: template.skills.clone(),
            sight_range: template.sight_range,
            chase_range: template.chase_range,
            aggro_rate: template.aggro_rate,
            spawn_delay: template.spawn_delay,
            respawn_time: template.respawn_time,
            spawn_x: x,
            spawn_y: y,
            spawn_map: map.to_string(),
            death_time: RwLock::new(None),
            drops: template.drops.clone(),
            base_exp: template.base_exp,
            job_exp: template.job_exp,
            zeny: template.zeny,
            drops_processed: RwLock::new(false),
            path_manager: RwLock::new(MobPathManager::new()),
            damage_log: RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn get_position(&self) -> (u16, u16) {
        let p = self.pos.read();
        (p.x, p.y)
    }

    pub fn move_to(&self, x: u16, y: u16) {
        *self.pos.write() = MobPosition { x, y };
    }

    pub fn take_damage(&self, damage: u32) -> bool {
        let mut hp = self.hp.write();
        if *hp <= damage {
            *hp = 0;
            // Hold HP lock while setting death state to prevent TOCTOU window
            // where hp==0 but ai_state!=Dead
            *self.ai_state.write() = MobAIState::Dead;
            *self.death_time.write() = Some(Instant::now());
            true
        } else {
            *hp -= damage;
            false
        }
    }

    pub fn is_dead(&self) -> bool {
        *self.hp.read() == 0
    }

    /// 重生：回到出生点并恢复满血
    pub fn respawn(&self) {
        *self.hp.write() = self.max_hp;
        *self.sp.write() = self.max_sp;
        *self.pos.write() = MobPosition { x: self.spawn_x, y: self.spawn_y };
        *self.ai_state.write() = MobAIState::Idle;
        *self.target_id.write() = None;
        *self.death_time.write() = None;
        *self.drops_processed.write() = false;
        *self.path_manager.write() = MobPathManager::new();
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
                walk_speed: constants::DEFAULT_WALK_SPEED,
                atk_range: 1,
                sight_range: 12,
                chase_range: 20,
                aggro_rate: 0,
                spawn_delay: 1000,
                respawn_time: 60000,
                behavior: MobBehavior::Passive,
                skills: Vec::new(),
                drops: vec![
                    MobDrop::new(909, 7000), // Jellopy 70%
                    MobDrop::new(1202, 500), // Knife 5%
                    MobDrop::new(938, 100),  // Sticky Mucus 1%
                ],
                base_exp: 2,
                job_exp: 1,
                zeny: Some(10),
                element: Element::Water,
                element_level: ElementLevel::Level1,
                size: MobSize::Small,
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
                behavior: MobBehavior::Passive,
                skills: Vec::new(),
                drops: vec![
                    MobDrop::new(910, 6000), // Fluff 60%
                    MobDrop::new(938, 200),  // Sticky Mucus 2%
                ],
                base_exp: 6,
                job_exp: 4,
                zeny: Some(15),
                element: Element::Neutral,
                element_level: ElementLevel::Level1,
                size: MobSize::Small,
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
                walk_speed: constants::DEFAULT_WALK_SPEED,
                atk_range: 1,
                sight_range: 12,
                chase_range: 20,
                aggro_rate: 0,
                spawn_delay: 1000,
                respawn_time: 60000,
                behavior: MobBehavior::Passive,
                skills: Vec::new(),
                drops: vec![
                    MobDrop::new(909, 5000), // Jellopy 50%
                    MobDrop::new(947, 300),  // Scale Shell 3%
                ],
                base_exp: 4,
                job_exp: 3,
                zeny: Some(12),
                element: Element::Water,
                element_level: ElementLevel::Level1,
                size: MobSize::Small,
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
                walk_speed: constants::DEFAULT_WALK_SPEED,
                atk_range: 1,
                sight_range: 12,
                chase_range: 20,
                aggro_rate: 0,
                spawn_delay: 1000,
                respawn_time: 60000,
                behavior: MobBehavior::Passive,
                skills: Vec::new(),
                drops: vec![
                    MobDrop::new(914, 5500), // Fluff 55%
                    MobDrop::new(949, 400),  // Feather 4%
                ],
                base_exp: 8,
                job_exp: 5,
                zeny: Some(20),
                element: Element::Earth,
                element_level: ElementLevel::Level1,
                size: MobSize::Small,
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
    pub behavior: MobBehavior,
    pub skills: Vec<MobSkill>,
    pub drops: Vec<MobDrop>,
    pub base_exp: u64,
    pub job_exp: u64,
    pub zeny: Option<u64>,
    pub element: Element,
    pub element_level: ElementLevel,
    pub size: MobSize,
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
            walk_speed: constants::DEFAULT_WALK_SPEED,
            atk_range: 1,
            sight_range: 12,
            chase_range: 20,
            aggro_rate: 0,
            spawn_delay: 1000,
            respawn_time: 60000,
            behavior: MobBehavior::Passive,
            skills: Vec::new(),
            drops: Vec::new(),
            base_exp: 10,
            job_exp: 5,
            zeny: None,
            element: Element::Neutral,
            element_level: ElementLevel::Level1,
            size: MobSize::Medium,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_manager_start_chase() {
        let mut manager = MobPathManager::new();
        assert!(!manager.is_chasing);
        assert!(manager.cached_path.is_empty());

        manager.start_chase((100, 200));

        assert!(manager.is_chasing);
        assert_eq!(manager.target_pos, Some((100, 200)));
        assert!(manager.cached_path.is_empty());
        assert_eq!(manager.current_step, 0);
        assert!(!manager.path_invalid);
    }

    #[test]
    fn test_path_manager_set_path() {
        let mut manager = MobPathManager::new();
        manager.start_chase((100, 200));

        manager.set_path(vec![(10, 10), (11, 11), (12, 12)]);
        assert_eq!(manager.cached_path.len(), 3);
        assert_eq!(manager.current_step, 0);
        assert!(!manager.path_invalid);
    }

    #[test]
    fn test_path_manager_advance_step() {
        let mut manager = MobPathManager::new();
        manager.set_path(vec![(10, 10), (11, 11), (12, 12)]);

        assert_eq!(manager.advance_step(), Some((10, 10)));
        assert_eq!(manager.advance_step(), Some((11, 11)));
        assert_eq!(manager.advance_step(), Some((12, 12)));
        assert_eq!(manager.advance_step(), None);
    }

    #[test]
    fn test_path_manager_invalidate() {
        let mut manager = MobPathManager::new();
        manager.set_path(vec![(10, 10)]);
        assert!(!manager.path_invalid);

        manager.invalidate();
        assert!(manager.path_invalid);
    }

    #[test]
    fn test_path_manager_stop_chase() {
        let mut manager = MobPathManager::new();
        manager.start_chase((100, 200));
        manager.set_path(vec![(10, 10)]);

        manager.stop_chase();

        assert!(!manager.is_chasing);
        assert_eq!(manager.target_pos, None);
        assert!(manager.cached_path.is_empty());
    }
}
