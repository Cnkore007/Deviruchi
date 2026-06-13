use parking_lot::RwLock;
use std::collections::HashMap;
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

/// 怪物行为标记（从 Modes 解析）
#[derive(Debug, Clone)]
pub struct MobBehaviorFlags {
    /// 是否可以移动（false 表示固定不动）
    pub can_move: bool,
    /// 是否可以攻击（false 表示不会攻击）
    pub can_attack: bool,
    /// 是否为探测型（可以看见隐身目标）
    pub detector: bool,
    /// 是否为 Boss 级别
    pub boss: bool,
    /// 是否为植物型（不受物理/魔法伤害加成影响）
    pub plant: bool,
    /// 是否可以追击（false 表示不追击超出视野的目标）
    pub can_chase: bool,
}

impl Default for MobBehaviorFlags {
    fn default() -> Self {
        Self {
            can_move: true,
            can_attack: true,
            detector: false,
            boss: false,
            plant: false,
            can_chase: true,
        }
    }
}

/// 怪物种族
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum MobRace {
    #[default]
    Formless,
    Undead,
    Brute,
    Plant,
    Insect,
    Fish,
    Demon,
    DemiHuman,
    Angel,
    Dragon,
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

/// 怪物技能目标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum MobSkillTarget {
    /// 对攻击目标（敌人）使用
    #[default]
    Target,
    /// 对自身使用（如治疗、增益）
    Self_,
}


/// 怪物技能触发条件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum MobSkillCondition {
    /// 无特殊条件（任何时候都可触发）
    #[default]
    Any,
    /// 被围攻时（rAthena: rudeattacked）
    RudeAttacked,
    /// 远程目标时（rAthena: longrange）
    LongRange,
    /// HP 低于阈值时（rAthena: hpcertain）
    HpCertain,
}


/// 怪物技能数据
#[derive(Debug, Clone)]
pub struct MobSkill {
    pub skill_id: u16,
    pub level: u8,
    /// 使用概率（万分比）
    pub chance: u32,
    /// 技能目标类型（对敌人/对自身）
    pub target: MobSkillTarget,
    /// 触发条件类型
    pub condition: MobSkillCondition,
    /// 条件值（如 HP 百分比阈值）
    pub condition_value: u32,
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
    /// 逃跑状态：HP 过低时远离攻击者
    Flee,
    Dead,
}

/// 怪物数据
#[derive(Debug)]
pub struct Mob {
    pub id: Uuid,
    /// 客户端可见的实体 ID（u32），由 MobSpawnManager 注册时分配
    pub entity_id: std::sync::atomic::AtomicU32,
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

    // 种族与类型
    pub race: MobRace,
    pub mob_type: MobType,

    // AI状态
    pub ai_state: RwLock<MobAIState>,
    pub target_id: RwLock<Option<Uuid>>,
    pub behavior: MobBehavior,
    pub skills: Vec<MobSkill>,
    /// 技能冷却记录：skill_id -> 上次使用时间
    pub skill_cooldowns: RwLock<HashMap<u16, Instant>>,

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
    // 伤害记录（用于血条同步，参考 rAthena dmglog）
    pub dmglog: RwLock<HashMap<Uuid, u32>>,
    /// 逃跑目标（记录要逃离的攻击者 ID，用于 FleeWhenLowHp 行为）
    pub flee_from: RwLock<Option<Uuid>>,
}

impl Mob {
    pub fn new(mob_id: u16, x: u16, y: u16, map: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            entity_id: std::sync::atomic::AtomicU32::new(0),
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
            race: MobRace::Formless,
            mob_type: MobType::Normal,
            ai_state: RwLock::new(MobAIState::Idle),
            target_id: RwLock::new(None),
            behavior: MobBehavior::Passive,
            skills: Vec::new(),
            skill_cooldowns: RwLock::new(HashMap::new()),
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
            dmglog: RwLock::new(HashMap::new()),
            flee_from: RwLock::new(None),
        }
    }

    pub fn from_template(mob_id: u16, x: u16, y: u16, map: &str) -> Self {
        let template = MobDatabase::default_instance().get(mob_id);
        Self {
            id: Uuid::new_v4(),
            entity_id: std::sync::atomic::AtomicU32::new(0),
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
            race: template.race,
            mob_type: template.mob_type,
            ai_state: RwLock::new(MobAIState::Idle),
            target_id: RwLock::new(None),
            behavior: template.behavior,
            skills: template.skills.clone(),
            skill_cooldowns: RwLock::new(HashMap::new()),
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
            dmglog: RwLock::new(HashMap::new()),
            flee_from: RwLock::new(None),
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

    /// 获取客户端可见的实体 ID
    pub fn get_entity_id(&self) -> u32 {
        self.entity_id.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 设置实体 ID（由 MobSpawnManager 在注册时调用）
    pub fn set_entity_id(&self, id: u32) {
        self.entity_id.store(id, std::sync::atomic::Ordering::Relaxed);
    }

    /// 获取当前 HP 百分比（0-100）
    pub fn hp_percent(&self) -> u32 {
        let hp = *self.hp.read();
        if self.max_hp == 0 {
            return 0;
        }
        (hp as u64 * 100 / self.max_hp as u64) as u32
    }

    /// 记录玩家对此怪物造成的伤害
    pub fn add_damage(&self, player_id: Uuid, damage: u32) {
        let mut log = self.dmglog.write();
        let entry = log.entry(player_id).or_insert(0);
        *entry += damage;
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
        // 清空技能冷却，重生后所有技能可用
        self.skill_cooldowns.write().clear();
        // 清空伤害记录（血条同步数据）
        self.dmglog.write().clear();
        // 清空逃跑目标
        *self.flee_from.write() = None;
    }

    /// 恢复自身 HP（用于怪物自愈技能）
    pub fn heal(&self, amount: u32) {
        let mut hp = self.hp.write();
        *hp = (*hp + amount).min(self.max_hp);
    }
}

/// 怪物数据库（支持 YAML 加载 + 硬编码回退）
pub struct MobDatabase {
    templates: std::collections::HashMap<u16, MobTemplate>,
}

impl MobDatabase {
    /// 创建怪物数据库，优先从 YAML 加载，失败则使用硬编码数据
    pub fn new() -> Self {
        let mut db = Self {
            templates: std::collections::HashMap::new(),
        };

        // 尝试从 YAML 加载
        let yaml_paths = ["db/mob_db.yml"];

        for path in &yaml_paths {
            if std::path::Path::new(path).exists() {
                match crate::game::mob::yaml_loader::load_mob_db(path) {
                    Ok(mobs) => {
                        let count = mobs.len();
                        db.templates.extend(mobs);
                        tracing::info!("从 {} 加载了 {} 个怪物模板", path, count);
                        return db;
                    }
                    Err(e) => {
                        tracing::warn!("加载 {} 失败: {}", path, e);
                    }
                }
            }
        }

        // 回退到硬编码数据
        tracing::info!("使用硬编码怪物数据");
        db.init_hardcoded();
        db
    }

    /// 初始化硬编码怪物数据（作为 YAML 不可用时的回退）
    fn init_hardcoded(&mut self) {
        self.templates.insert(
            1001,
            MobTemplate {
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
                race: MobRace::Plant,
                mob_type: MobType::Normal,
                mvp_drops: Vec::new(),
                behavior_flags: MobBehaviorFlags::default(),
            },
        );
        self.templates.insert(
            1002,
            MobTemplate {
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
                skills: vec![
                    MobSkill {
                        skill_id: 28,   // Heal
                        level: 3,
                        chance: 1000,   // 10% 概率
                        target: MobSkillTarget::Self_,
                        condition: MobSkillCondition::HpCertain,
                        condition_value: 50, // HP 低于 50% 时使用
                        cooldown_ms: 10000,  // 10 秒冷却
                    },
                ],
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
                race: MobRace::Brute,
                mob_type: MobType::Normal,
                mvp_drops: Vec::new(),
                behavior_flags: MobBehaviorFlags::default(),
            },
        );
        self.templates.insert(
            1003,
            MobTemplate {
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
                race: MobRace::Plant,
                mob_type: MobType::Normal,
                mvp_drops: Vec::new(),
                behavior_flags: MobBehaviorFlags::default(),
            },
        );
        self.templates.insert(
            1312,
            MobTemplate {
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
                skills: vec![
                    MobSkill {
                        skill_id: 5,    // Bash (SM_BASH)
                        level: 3,
                        chance: 500,    // 5% 概率
                        target: MobSkillTarget::Target,
                        condition: MobSkillCondition::Any,
                        condition_value: 0,
                        cooldown_ms: 5000, // 5 秒冷却
                    },
                ],
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
                race: MobRace::Insect,
                mob_type: MobType::Normal,
                mvp_drops: Vec::new(),
                behavior_flags: MobBehaviorFlags::default(),
            },
        );
    }

    /// 获取怪物模板（返回引用，不存在则返回默认模板）
    pub fn get(&self, mob_id: u16) -> &MobTemplate {
        self.templates
            .get(&mob_id)
            .unwrap_or_else(|| {
                // 返回默认模板的静态引用
                static DEFAULT: std::sync::OnceLock<MobTemplate> = std::sync::OnceLock::new();
                DEFAULT.get_or_init(|| MobTemplate::default(0))
            })
    }

    /// 获取怪物模板（可选引用）
    pub fn get_opt(&self, mob_id: u16) -> Option<&MobTemplate> {
        self.templates.get(&mob_id)
    }

    /// 获取所有模板的迭代器
    pub fn all(&self) -> impl Iterator<Item = (&u16, &MobTemplate)> {
        self.templates.iter()
    }

    /// 获取模板总数
    pub fn count(&self) -> usize {
        self.templates.len()
    }

    /// 获取全局默认数据库实例
    pub fn default_instance() -> &'static MobDatabase {
        static INSTANCE: std::sync::OnceLock<MobDatabase> = std::sync::OnceLock::new();
        INSTANCE.get_or_init(MobDatabase::new)
    }
}

impl Default for MobDatabase {
    fn default() -> Self {
        Self::new()
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
    /// 怪物种族
    pub race: MobRace,
    /// 怪物类型（Normal/Boss/Guardian/Event）
    pub mob_type: MobType,
    /// MVP 掉落列表
    pub mvp_drops: Vec<MobDrop>,
    /// 行为标记（从 Modes 字段解析）
    pub behavior_flags: MobBehaviorFlags,
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
            race: MobRace::Formless,
            mob_type: MobType::Normal,
            mvp_drops: Vec::new(),
            behavior_flags: MobBehaviorFlags::default(),
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

    // ============================================
    // 怪物技能系统测试
    // ============================================

    #[test]
    fn test_mob_hp_percent_full() {
        let mob = Mob::new(1001, 100, 100, "test_map");
        // 默认 hp=100, max_hp=100
        assert_eq!(mob.hp_percent(), 100);
    }

    #[test]
    fn test_mob_hp_percent_half() {
        let mob = Mob::new(1001, 100, 100, "test_map");
        *mob.hp.write() = 50;
        assert_eq!(mob.hp_percent(), 50);
    }

    #[test]
    fn test_mob_hp_percent_zero() {
        let mob = Mob::new(1001, 100, 100, "test_map");
        *mob.hp.write() = 0;
        assert_eq!(mob.hp_percent(), 0);
    }

    #[test]
    fn test_mob_hp_percent_one_hp() {
        let mob = Mob::new(1001, 100, 100, "test_map");
        *mob.hp.write() = 1;
        assert_eq!(mob.hp_percent(), 1);
    }

    #[test]
    fn test_mob_heal_basic() {
        let mob = Mob::new(1001, 100, 100, "test_map");
        *mob.hp.write() = 50;
        mob.heal(20);
        assert_eq!(*mob.hp.read(), 70);
    }

    #[test]
    fn test_mob_heal_cap_at_max() {
        let mob = Mob::new(1001, 100, 100, "test_map");
        *mob.hp.write() = 90;
        mob.heal(50);
        // 不应超过 max_hp (100)
        assert_eq!(*mob.hp.read(), 100);
    }

    #[test]
    fn test_mob_heal_at_full_hp() {
        let mob = Mob::new(1001, 100, 100, "test_map");
        // HP 已满
        mob.heal(100);
        assert_eq!(*mob.hp.read(), 100);
    }

    #[test]
    fn test_mob_skill_cooldowns_initialized_empty() {
        let mob = Mob::new(1001, 100, 100, "test_map");
        assert!(mob.skill_cooldowns.read().is_empty());
    }

    #[test]
    fn test_mob_respawn_clears_cooldowns() {
        let mob = Mob::new(1001, 100, 100, "test_map");

        // 模拟设置冷却
        mob.skill_cooldowns.write().insert(5, Instant::now());
        mob.skill_cooldowns.write().insert(28, Instant::now());
        assert_eq!(mob.skill_cooldowns.read().len(), 2);

        // 设置死亡状态
        *mob.hp.write() = 0;
        *mob.ai_state.write() = MobAIState::Dead;
        *mob.death_time.write() = Some(Instant::now());

        // 重生
        mob.respawn();

        // 冷却应被清空
        assert!(mob.skill_cooldowns.read().is_empty());
        assert_eq!(*mob.hp.read(), 100);
    }

    #[test]
    fn test_mob_from_template_has_empty_cooldowns() {
        // from_template 应该初始化空的 skill_cooldowns
        let mob = Mob::from_template(1001, 50, 50, "test_map");
        assert!(mob.skill_cooldowns.read().is_empty());
    }

    #[test]
    fn test_mob_skill_target_equality() {
        assert_eq!(MobSkillTarget::Target, MobSkillTarget::Target);
        assert_eq!(MobSkillTarget::Self_, MobSkillTarget::Self_);
        assert_ne!(MobSkillTarget::Target, MobSkillTarget::Self_);
    }

    #[test]
    fn test_mob_skill_condition_equality() {
        assert_eq!(MobSkillCondition::Any, MobSkillCondition::Any);
        assert_eq!(MobSkillCondition::HpCertain, MobSkillCondition::HpCertain);
        assert_ne!(MobSkillCondition::Any, MobSkillCondition::HpCertain);
    }
}
