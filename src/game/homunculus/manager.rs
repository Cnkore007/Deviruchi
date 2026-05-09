//! 生命体管理器
//!
//! 负责生命体的创建、召唤、喂食、经验、进化等操作。
//! 所有写操作实时持久化到数据库。

use super::data::{
    EvolutionStage, Homunculus, HomunculusDatabase, HomunculusRace, HomunculusTemplate,
    HomunculusType,
};
use crate::storage::Database;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use thiserror::Error;

/// 生命体错误类型
#[derive(Debug, Error)]
pub enum HomunculusError {
    #[error("生命体未找到: {0}")]
    NotFound(u32),

    #[error("玩家未找到: {0}")]
    PlayerNotFound(u32),

    #[error("玩家已有召唤的生命体")]
    AlreadySummoned,

    #[error("生命体已死亡")]
    Dead,

    #[error("生命体未召唤")]
    NotSummoned,

    #[error("不是你的生命体")]
    NotYours,

    #[error("食物类型不匹配")]
    WrongFood,

    #[error("进化条件不满足")]
    EvolutionFailed,

    #[error("技能前置条件不满足")]
    SkillPrereqNotMet,

    #[error("数据库错误: {0}")]
    Database(String),
}

/// 生命体管理器
///
/// 所有写操作（create/feed/add_exp/evolve/learn_skill）先改内存再写数据库。
/// load_for_character() 在角色登录时调用，从数据库加载到内存。
pub struct HomunculusManager {
    /// 数据库引用
    db: Arc<Database>,
    /// 所有生命体实例 (homun_id -> Homunculus)
    homunculi: RwLock<HashMap<u32, Homunculus>>,
    /// 当前召唤的生命体 (char_id -> homun_id)
    summoned: RwLock<HashMap<u32, u32>>,
    /// 生命体模板数据库
    database: HomunculusDatabase,
    /// 下一个可用 ID（原子操作，避免写锁）
    next_id: AtomicU32,
}

impl HomunculusManager {
    /// 创建管理器（仅硬编码模板，供测试使用）
    pub fn new_hardcoded(db: Arc<Database>) -> Self {
        let max_id = db
            .query_row(
                "SELECT COALESCE(MAX(homun_id), 0) FROM homunculus",
                &[],
                |row| row.get_i64(0),
            )
            .unwrap_or(0) as u32;

        Self {
            db,
            homunculi: RwLock::new(HashMap::new()),
            summoned: RwLock::new(HashMap::new()),
            database: HomunculusDatabase::new_hardcoded(),
            next_id: AtomicU32::new(max_id + 1),
        }
    }

    /// 创建管理器，从数据库初始化 next_id
    pub fn new(db: Arc<Database>) -> Self {
        // 从数据库初始化 next_id（如果 homunculus 表存在）
        let max_id = db
            .query_row(
                "SELECT COALESCE(MAX(homun_id), 0) FROM homunculus",
                &[],
                |row| row.get_i64(0),
            )
            .unwrap_or(0) as u32;

        Self {
            db,
            homunculi: RwLock::new(HashMap::new()),
            summoned: RwLock::new(HashMap::new()),
            database: HomunculusDatabase::new(),
            next_id: AtomicU32::new(max_id + 1),
        }
    }

    /// 从数据库加载角色的所有生命体到内存
    ///
    /// 在角色登录时调用，将数据库中的生命体数据加载到 homunculi HashMap。
    pub fn load_for_character(&self, char_id: u32) -> Result<Vec<Homunculus>, HomunculusError> {
        let rows = self
            .db
            .query_rows(
                "SELECT homun_id, owner_id, homunculus_type, name, level, exp, hunger, intimacy, \
                 hp, max_hp, sp, max_sp, str, agi, vit, int, dex, luk, evolved, alive, \
                 race, element, element_level, evolution_stage, skill_points, \
                 atk, matk, defense, magic_defense, hit, flee, walk_speed, attack_delay \
                 FROM homunculus WHERE owner_id = ?",
                &[&char_id],
            )
            .map_err(|e| HomunculusError::Database(e.to_string()))?;

        let mut homunculi = Vec::new();
        for row in &rows {
            let homun_id = row.get_i32(0).unwrap_or(0) as u32;

            // 加载技能（从 homunculus_skills 表）
            let skill_rows = self
                .db
                .query_rows(
                    "SELECT skill_name, skill_level FROM homunculus_skills WHERE homun_id = ?",
                    &[&homun_id],
                )
                .unwrap_or_default();

            let skills: Vec<super::data::HomunculusSkill> = skill_rows
                .iter()
                .map(|sr| super::data::HomunculusSkill {
                    skill_id: 0,
                    skill_name: sr.get_string(0).unwrap_or_default(),
                    level: sr.get_i32(1).unwrap_or(0) as u8,
                    max_level: 5,
                    required_level: 0,
                    required_intimacy: 0,
                    require_evolution: false,
                    prerequisites: Vec::new(),
                })
                .collect();

            let homun = Homunculus {
                homun_id,
                owner_id: row.get_i32(1).unwrap_or(0) as u32,
                homunculus_type: HomunculusType::from_str(
                    &row.get_string(2).unwrap_or_default(),
                ),
                name: row.get_string(3).unwrap_or_default(),
                level: row.get_i32(4).unwrap_or(1) as u16,
                exp: row.get_i64(5).unwrap_or(0) as u64,
                hunger: row.get_i32(6).unwrap_or(100) as u32,
                intimacy: row.get_i32(7).unwrap_or(100) as u32,
                hp: row.get_i32(8).unwrap_or(500) as u32,
                max_hp: row.get_i32(9).unwrap_or(500) as u32,
                sp: row.get_i32(10).unwrap_or(100) as u32,
                max_sp: row.get_i32(11).unwrap_or(100) as u32,
                str: row.get_i32(12).unwrap_or(1) as u16,
                agi: row.get_i32(13).unwrap_or(1) as u16,
                vit: row.get_i32(14).unwrap_or(1) as u16,
                int: row.get_i32(15).unwrap_or(1) as u16,
                dex: row.get_i32(16).unwrap_or(1) as u16,
                luk: row.get_i32(17).unwrap_or(1) as u16,
                evolved: row.get_i32(18).unwrap_or(0) != 0,
                alive: row.get_i32(19).unwrap_or(1) != 0,
                race: HomunculusRace::from_str(
                    &row.get_string(20).unwrap_or_else(|_| "Formless".to_string()),
                ),
                element: row.get_string(21).unwrap_or_else(|_| "Neutral".to_string()),
                // element_level (index 21) 在数据库中存在但 Homunculus 结构体中无对应字段，
                // 此处跳过读取，保持索引对齐
                evolution_stage: EvolutionStage::from_str(
                    &row.get_string(23).unwrap_or_else(|_| "Base".to_string()),
                ),
                skill_points: row.get_i32(24).unwrap_or(0) as u16,
                atk: row.get_i32(25).unwrap_or(0) as u16,
                matk: row.get_i32(26).unwrap_or(0) as u16,
                defense: row.get_i32(27).unwrap_or(0) as u16,
                magic_defense: row.get_i32(28).unwrap_or(0) as u16,
                hit: row.get_i32(29).unwrap_or(0) as i16,
                flee: row.get_i32(30).unwrap_or(0) as i16,
                walk_speed: row.get_i32(31).unwrap_or(200) as u16,
                attack_delay: row.get_i32(32).unwrap_or(1000) as u32,
                skills,
            };

            // 加载到内存
            self.homunculi.write().insert(homun_id, homun.clone());
            homunculi.push(homun);
        }

        Ok(homunculi)
    }

    /// 创建新生命体（INSERT 到数据库 + 加入内存）
    pub fn create(
        &self,
        owner_id: u32,
        htype: HomunculusType,
        name: &str,
    ) -> Result<Homunculus, HomunculusError> {
        let template = self
            .database
            .get_by_type(htype)
            .ok_or(HomunculusError::NotFound(0))?;

        let homun_id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let homun = Homunculus::from_template(homun_id, owner_id, template, name.to_string());

        // INSERT 到数据库
        self.db
            .execute_params(
                "INSERT INTO homunculus (homun_id, owner_id, homunculus_type, name, level, exp, \
                 hunger, intimacy, hp, max_hp, sp, max_sp, str, agi, vit, int, dex, luk, \
                 evolved, alive, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                &[
                    &homun_id as &dyn crate::storage::backend::IntoValue,
                    &owner_id,
                    &format!("{:?}", htype),
                    &name,
                    &(homun.level as i32),
                    &(homun.exp as i64),
                    &(homun.hunger as i32),
                    &(homun.intimacy as i32),
                    &(homun.hp as i32),
                    &(homun.max_hp as i32),
                    &(homun.sp as i32),
                    &(homun.max_sp as i32),
                    &(homun.str as i32),
                    &(homun.agi as i32),
                    &(homun.vit as i32),
                    &(homun.int as i32),
                    &(homun.dex as i32),
                    &(homun.luk as i32),
                    &0i32,
                    &1i32,
                    &crate::storage::chrono_now(),
                ],
            )
            .map_err(|e| HomunculusError::Database(e.to_string()))?;

        self.homunculi.write().insert(homun_id, homun.clone());
        Ok(homun)
    }

    /// 召唤生命体
    pub fn summon(&self, char_id: u32, homun_id: u32) -> Result<(), HomunculusError> {
        if self.summoned.read().contains_key(&char_id) {
            return Err(HomunculusError::AlreadySummoned);
        }

        let homunculi = self.homunculi.read();
        let homun = homunculi
            .get(&homun_id)
            .ok_or(HomunculusError::NotFound(homun_id))?;

        if homun.owner_id != char_id {
            return Err(HomunculusError::NotYours);
        }

        if homun.is_dead() {
            return Err(HomunculusError::Dead);
        }

        drop(homunculi);
        self.summoned.write().insert(char_id, homun_id);
        Ok(())
    }

    /// 解散生命体
    pub fn dismiss(&self, char_id: u32) {
        self.summoned.write().remove(&char_id);
    }

    /// 获取玩家召唤的生命体
    pub fn get_summoned(&self, char_id: u32) -> Option<Homunculus> {
        let summoned = self.summoned.read();
        let homun_id = summoned.get(&char_id)?;
        self.homunculi.read().get(homun_id).cloned()
    }

    /// 喂食生命体（UPDATE hunger, intimacy 到数据库）
    pub fn feed(&self, char_id: u32, _item_id: u16) -> Result<(), HomunculusError> {
        let summoned = self.summoned.read();
        let homun_id = summoned
            .get(&char_id)
            .ok_or(HomunculusError::NotSummoned)?;

        let mut homunculi = self.homunculi.write();
        let homun = homunculi
            .get_mut(homun_id)
            .ok_or(HomunculusError::NotFound(*homun_id))?;

        // TODO: 检查食物类型是否匹配
        homun.feed(20);
        homun.increase_intimacy(10);

        // 实时写库
        self.db
            .execute_params(
                "UPDATE homunculus SET hunger = ?, intimacy = ? WHERE homun_id = ?",
                &[
                    &(homun.hunger as i32) as &dyn crate::storage::backend::IntoValue,
                    &(homun.intimacy as i32),
                    homun_id,
                ],
            )
            .map_err(|e| HomunculusError::Database(e.to_string()))?;

        Ok(())
    }

    /// 增加经验（返回是否升级，实时写库）
    pub fn add_exp(&self, char_id: u32, exp: u64) -> Result<bool, HomunculusError> {
        let summoned = self.summoned.read();
        let homun_id = summoned
            .get(&char_id)
            .ok_or(HomunculusError::NotSummoned)?;

        let mut homunculi = self.homunculi.write();
        let homun = homunculi
            .get_mut(homun_id)
            .ok_or(HomunculusError::NotFound(*homun_id))?;

        homun.exp += exp;

        // 检查升级
        let exp_needed = Self::exp_for_level(homun.level + 1);
        let leveled = if homun.exp >= exp_needed {
            homun.level += 1;
            homun.exp -= exp_needed;

            // 属性成长
            homun.max_hp += 20;
            homun.hp = homun.max_hp;
            homun.max_sp += 5;
            homun.sp = homun.max_sp;
            homun.str += 1;
            homun.agi += 1;
            homun.vit += 1;
            homun.int += 1;
            homun.dex += 1;
            homun.luk += 1;

            if homun.level % 5 == 0 {
                homun.skill_points += 1;
            }
            true
        } else {
            false
        };

        // 实时写库
        self.db
            .execute_params(
                "UPDATE homunculus SET level = ?, exp = ?, hp = ?, max_hp = ?, sp = ?, max_sp = ?, \
                 str = ?, agi = ?, vit = ?, int = ?, dex = ?, luk = ?, skill_points = ? WHERE homun_id = ?",
                &[
                    &(homun.level as i32) as &dyn crate::storage::backend::IntoValue,
                    &(homun.exp as i64),
                    &(homun.hp as i32),
                    &(homun.max_hp as i32),
                    &(homun.sp as i32),
                    &(homun.max_sp as i32),
                    &(homun.str as i32),
                    &(homun.agi as i32),
                    &(homun.vit as i32),
                    &(homun.int as i32),
                    &(homun.dex as i32),
                    &(homun.luk as i32),
                    &(homun.skill_points as i32),
                    homun_id,
                ],
            )
            .map_err(|e| HomunculusError::Database(e.to_string()))?;

        Ok(leveled)
    }

    /// 经验表查询
    fn exp_for_level(level: u16) -> u64 {
        match level {
            1..=10 => (level as u64) * 100,
            11..=50 => (level as u64) * 500,
            51..=99 => (level as u64) * 2000,
            _ => 999999,
        }
    }

    /// 进化生命体（实时写库）
    pub fn evolve(&self, char_id: u32) -> Result<(), HomunculusError> {
        let summoned = self.summoned.read();
        let homun_id = summoned
            .get(&char_id)
            .ok_or(HomunculusError::NotSummoned)?;

        let mut homunculi = self.homunculi.write();
        let homun = homunculi
            .get_mut(homun_id)
            .ok_or(HomunculusError::NotFound(*homun_id))?;

        if homun.level < 99 || homun.intimacy < 910 {
            return Err(HomunculusError::EvolutionFailed);
        }
        if homun.evolved {
            return Err(HomunculusError::EvolutionFailed);
        }

        homun.evolved = true;
        homun.evolution_stage = EvolutionStage::Evolved;
        homun.max_hp += 500;
        homun.hp = homun.max_hp;
        homun.max_sp += 100;
        homun.sp = homun.max_sp;
        homun.str += 10;
        homun.agi += 10;
        homun.vit += 10;
        homun.int += 10;
        homun.dex += 10;
        homun.luk += 10;

        // 实时写库
        self.db
            .execute_params(
                "UPDATE homunculus SET evolved = 1, evolution_stage = ?, max_hp = ?, hp = ?, \
                 max_sp = ?, sp = ?, str = ?, agi = ?, vit = ?, int = ?, dex = ?, luk = ? \
                 WHERE homun_id = ?",
                &[
                    &"Evolved" as &dyn crate::storage::backend::IntoValue,
                    &(homun.max_hp as i32),
                    &(homun.hp as i32),
                    &(homun.max_sp as i32),
                    &(homun.sp as i32),
                    &(homun.str as i32),
                    &(homun.agi as i32),
                    &(homun.vit as i32),
                    &(homun.int as i32),
                    &(homun.dex as i32),
                    &(homun.luk as i32),
                    homun_id,
                ],
            )
            .map_err(|e| HomunculusError::Database(e.to_string()))?;

        Ok(())
    }

    /// 学习技能（写入 homunculus_skills 表）
    pub fn learn_skill(&self, homun_id: u32, skill_name: &str) -> Result<(), HomunculusError> {
        let mut homunculi = self.homunculi.write();
        let homun = homunculi
            .get_mut(&homun_id)
            .ok_or(HomunculusError::NotFound(homun_id))?;

        // 检查技能点
        if homun.skill_points == 0 {
            return Err(HomunculusError::SkillPrereqNotMet);
        }

        // 查找或创建技能
        if let Some(skill) = homun.skills.iter_mut().find(|s| s.skill_name == skill_name) {
            if skill.level >= skill.max_level {
                return Err(HomunculusError::SkillPrereqNotMet);
            }
            skill.level += 1;
        } else {
            homun.skills.push(super::data::HomunculusSkill {
                skill_id: 0,
                skill_name: skill_name.to_string(),
                level: 1,
                max_level: 5,
                required_level: 0,
                required_intimacy: 0,
                require_evolution: false,
                prerequisites: Vec::new(),
            });
        }

        homun.skill_points -= 1;

        // 写入技能表（UPSERT）
        self.db
            .execute_params(
                "INSERT INTO homunculus_skills (homun_id, skill_name, skill_level) VALUES (?, ?, ?) \
                 ON CONFLICT(homun_id, skill_name) DO UPDATE SET skill_level = skill_level + 1",
                &[
                    &homun_id as &dyn crate::storage::backend::IntoValue,
                    &skill_name,
                    &1i32,
                ],
            )
            .map_err(|e| HomunculusError::Database(e.to_string()))?;

        // 更新技能点到主表
        self.db
            .execute_params(
                "UPDATE homunculus SET skill_points = ? WHERE homun_id = ?",
                &[
                    &(homun.skill_points as i32) as &dyn crate::storage::backend::IntoValue,
                    &homun_id,
                ],
            )
            .map_err(|e| HomunculusError::Database(e.to_string()))?;

        Ok(())
    }

    /// 获取模板数据库引用
    pub fn database(&self) -> &HomunculusDatabase {
        &self.database
    }
}

// 注意：HomunculusManager 不再实现 Default，
// 因为构造时必须传入 Database 实例。

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// 创建测试用管理器（内存数据库 + 建表）
    fn create_test_manager() -> HomunculusManager {
        let db = Arc::new(Database::open_memory().expect("创建测试数据库失败"));
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS homunculus (
                homun_id INTEGER PRIMARY KEY,
                owner_id INTEGER NOT NULL,
                homunculus_type TEXT NOT NULL,
                name TEXT NOT NULL,
                level INTEGER DEFAULT 1,
                exp INTEGER DEFAULT 0,
                hunger INTEGER DEFAULT 100,
                intimacy INTEGER DEFAULT 100,
                hp INTEGER DEFAULT 500,
                max_hp INTEGER DEFAULT 500,
                sp INTEGER DEFAULT 100,
                max_sp INTEGER DEFAULT 100,
                str INTEGER DEFAULT 1,
                agi INTEGER DEFAULT 1,
                vit INTEGER DEFAULT 1,
                int INTEGER DEFAULT 1,
                dex INTEGER DEFAULT 1,
                luk INTEGER DEFAULT 1,
                evolved INTEGER DEFAULT 0,
                alive INTEGER DEFAULT 1,
                created_at INTEGER NOT NULL,
                race TEXT DEFAULT 'Formless',
                element TEXT DEFAULT 'Neutral',
                element_level INTEGER DEFAULT 1,
                evolution_stage TEXT DEFAULT 'Base',
                skill_points INTEGER DEFAULT 0,
                atk INTEGER DEFAULT 0,
                matk INTEGER DEFAULT 0,
                defense INTEGER DEFAULT 0,
                magic_defense INTEGER DEFAULT 0,
                hit INTEGER DEFAULT 0,
                flee INTEGER DEFAULT 0,
                walk_speed INTEGER DEFAULT 200,
                attack_delay INTEGER DEFAULT 1000
            )",
        )
        .expect("创建 homunculus 表失败");
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS homunculus_skills (
                homun_id INTEGER NOT NULL,
                skill_name TEXT NOT NULL,
                skill_level INTEGER DEFAULT 1,
                PRIMARY KEY (homun_id, skill_name)
            )",
        )
        .expect("创建 homunculus_skills 表失败");
        HomunculusManager::new_hardcoded(db)
    }

    #[test]
    fn test_create_homunculus() {
        let manager = create_test_manager();
        let homun = manager.create(100, HomunculusType::Lif, "MyLif").unwrap();

        assert_eq!(homun.owner_id, 100);
        assert_eq!(homun.name, "MyLif");
        assert_eq!(homun.level, 1);
        assert!(homun.alive);
        assert_eq!(homun.homunculus_type, HomunculusType::Lif);
    }

    #[test]
    fn test_summon_dismiss() {
        let manager = create_test_manager();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();

        // 召唤
        assert!(manager.summon(100, homun.homun_id).is_ok());
        assert!(manager.get_summoned(100).is_some());

        // 解散
        manager.dismiss(100);
        assert!(manager.get_summoned(100).is_none());
    }

    #[test]
    fn test_cannot_summon_twice() {
        let manager = create_test_manager();
        let homun1 = manager.create(100, HomunculusType::Lif, "Lif1").unwrap();
        let homun2 = manager
            .create(100, HomunculusType::Amistr, "Ami1")
            .unwrap();

        manager.summon(100, homun1.homun_id).unwrap();
        assert!(matches!(
            manager.summon(100, homun2.homun_id),
            Err(HomunculusError::AlreadySummoned)
        ));
    }

    #[test]
    fn test_summon_wrong_owner() {
        let manager = create_test_manager();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();

        assert!(matches!(
            manager.summon(999, homun.homun_id),
            Err(HomunculusError::NotYours)
        ));
    }

    #[test]
    fn test_feed() {
        let manager = create_test_manager();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        // 降低饥饿度（直接改内存，模拟游戏中的消耗）
        {
            let mut homunculi = manager.homunculi.write();
            let h = homunculi.get_mut(&homun.homun_id).unwrap();
            h.hunger = 50;
        }

        manager.feed(100, 0).unwrap();

        let h = manager.get_summoned(100).unwrap();
        assert_eq!(h.hunger, 70); // 50 + 20
    }

    #[test]
    fn test_add_exp_and_level_up() {
        let manager = create_test_manager();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        // 添加足够升级的经验（level 1->2 需要 100）
        let leveled = manager.add_exp(100, 200).unwrap();
        assert!(leveled);

        let h = manager.get_summoned(100).unwrap();
        assert_eq!(h.level, 2);
    }

    #[test]
    fn test_evolve() {
        let manager = create_test_manager();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        // 设置进化条件
        {
            let mut homunculi = manager.homunculi.write();
            let h = homunculi.get_mut(&homun.homun_id).unwrap();
            h.level = 99;
            h.intimacy = 910;
        }

        assert!(manager.evolve(100).is_ok());

        let h = manager.get_summoned(100).unwrap();
        assert!(h.evolved);
        assert_eq!(h.evolution_stage, EvolutionStage::Evolved);
    }

    #[test]
    fn test_evolve_insufficient_level() {
        let manager = create_test_manager();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        assert!(matches!(
            manager.evolve(100),
            Err(HomunculusError::EvolutionFailed)
        ));
    }

    /// 测试数据库持久化：create 后重新加载应一致
    #[test]
    fn test_persistence_create_and_load() {
        let manager = create_test_manager();
        let homun = manager.create(100, HomunculusType::Lif, "PersistLif").unwrap();

        // 清空内存缓存
        manager.homunculi.write().clear();

        // 从数据库加载
        let loaded = manager.load_for_character(100).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "PersistLif");
        assert_eq!(loaded[0].homun_id, homun.homun_id);
        assert_eq!(loaded[0].owner_id, 100);
        assert_eq!(loaded[0].homunculus_type, HomunculusType::Lif);
    }

    /// 测试持久化：feed 后数据库应更新
    #[test]
    fn test_persistence_feed() {
        let manager = create_test_manager();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        // 降低饥饿度
        {
            let mut homunculi = manager.homunculi.write();
            let h = homunculi.get_mut(&homun.homun_id).unwrap();
            h.hunger = 30;
        }

        manager.feed(100, 0).unwrap();

        // 清空内存，从数据库重新加载
        manager.homunculi.write().clear();
        manager.summoned.write().clear();

        let loaded = manager.load_for_character(100).unwrap();
        assert_eq!(loaded[0].hunger, 50); // 30 + 20
        assert_eq!(loaded[0].intimacy, 110); // 100 + 10
    }

    /// 测试持久化：add_exp 升级后数据库应更新
    #[test]
    fn test_persistence_level_up() {
        let manager = create_test_manager();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        manager.add_exp(100, 200).unwrap();

        // 清空内存，从数据库重新加载
        manager.homunculi.write().clear();
        manager.summoned.write().clear();

        let loaded = manager.load_for_character(100).unwrap();
        assert_eq!(loaded[0].level, 2);
        // Lif 模板 hp_growth.base = 150，升级后 +20 = 170
        assert!(loaded[0].max_hp > 150); // 升级后 max_hp 增加
    }

    /// 测试持久化：evolve 后数据库应更新
    #[test]
    fn test_persistence_evolve() {
        let manager = create_test_manager();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        // 设置进化条件
        {
            let mut homunculi = manager.homunculi.write();
            let h = homunculi.get_mut(&homun.homun_id).unwrap();
            h.level = 99;
            h.intimacy = 910;
            // 需要同时更新数据库中的 level 和 intimacy
        }
        manager
            .db
            .execute_params(
                "UPDATE homunculus SET level = 99, intimacy = 910 WHERE homun_id = ?",
                &[&homun.homun_id as &dyn crate::storage::backend::IntoValue],
            )
            .unwrap();

        manager.evolve(100).unwrap();

        // 清空内存，从数据库重新加载
        manager.homunculi.write().clear();
        manager.summoned.write().clear();

        let loaded = manager.load_for_character(100).unwrap();
        assert!(loaded[0].evolved);
        assert_eq!(loaded[0].evolution_stage, EvolutionStage::Evolved);
        // Lif 模板 hp_growth.base = 150，进化后 +500 = 650
        assert!(loaded[0].max_hp > 600); // 进化后 max_hp 大幅增加
    }

    /// 测试多角色隔离
    #[test]
    fn test_multi_character_isolation() {
        let manager = create_test_manager();
        manager.create(100, HomunculusType::Lif, "Char100_Lif").unwrap();
        manager.create(200, HomunculusType::Amistr, "Char200_Ami").unwrap();

        let loaded_100 = manager.load_for_character(100).unwrap();
        let loaded_200 = manager.load_for_character(200).unwrap();

        assert_eq!(loaded_100.len(), 1);
        assert_eq!(loaded_200.len(), 1);
        assert_eq!(loaded_100[0].name, "Char100_Lif");
        assert_eq!(loaded_200[0].name, "Char200_Ami");
    }
}
