//! 雇佣兵管理器
//!
//! 负责雇佣兵的创建、召唤、合同管理、忠诚度等操作。
//! 所有写操作实时持久化到数据库。

use super::data::{Mercenary, MercenaryDatabase, MercenarySkill};
use crate::storage::Database;
use chrono::{Duration, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use thiserror::Error;

/// 雇佣兵错误类型
#[derive(Debug, Error)]
pub enum MercenaryError {
    #[error("雇佣兵未找到: {0}")]
    NotFound(u32),

    #[error("玩家已有召唤的雇佣兵")]
    AlreadySummoned,

    #[error("雇佣兵未召唤")]
    NotSummoned,

    #[error("不是你的雇佣兵")]
    NotYours,

    #[error("合同已到期")]
    ContractExpired,

    #[error("雇佣兵已死亡")]
    Dead,

    #[error("数据库错误: {0}")]
    Database(String),
}

/// 雇佣兵管理器
///
/// 所有写操作（create/increase_loyalty/update_contracts）先改内存再写数据库。
/// load_for_character() 在角色登录时调用，从数据库加载到内存。
pub struct MercenaryManager {
    /// 数据库引用
    db: Arc<Database>,
    /// 所有雇佣兵实例
    mercenaries: RwLock<HashMap<u32, Mercenary>>,
    /// 当前召唤的雇佣兵 (char_id -> mercenary_id)
    summoned: RwLock<HashMap<u32, u32>>,
    /// 雇佣兵模板数据库
    database: MercenaryDatabase,
    /// 下一个可用 ID（原子操作，避免写锁）
    next_id: AtomicU32,
}

impl MercenaryManager {
    /// 创建管理器（仅硬编码模板，供测试使用）
    pub fn new_hardcoded(db: Arc<Database>) -> Self {
        let max_id = db
            .query_row(
                "SELECT COALESCE(MAX(mercenary_id), 0) FROM mercenaries",
                &[],
                |row| row.get_i64(0),
            )
            .unwrap_or(0) as u32;

        Self {
            db,
            mercenaries: RwLock::new(HashMap::new()),
            summoned: RwLock::new(HashMap::new()),
            database: MercenaryDatabase::new_hardcoded(),
            next_id: AtomicU32::new(max_id + 1),
        }
    }

    /// 创建管理器，从数据库初始化 next_id
    pub fn new(db: Arc<Database>) -> Self {
        let max_id = db
            .query_row(
                "SELECT COALESCE(MAX(mercenary_id), 0) FROM mercenaries",
                &[],
                |row| row.get_i64(0),
            )
            .unwrap_or(0) as u32;

        Self {
            db,
            mercenaries: RwLock::new(HashMap::new()),
            summoned: RwLock::new(HashMap::new()),
            database: MercenaryDatabase::new(),
            next_id: AtomicU32::new(max_id + 1),
        }
    }

    /// 从数据库加载角色的所有雇佣兵到内存
    ///
    /// 在角色登录时调用，将数据库中的雇佣兵数据加载到 mercenaries HashMap。
    pub fn load_for_character(&self, char_id: u32) -> Result<Vec<Mercenary>, MercenaryError> {
        let rows = self
            .db
            .query_rows(
                "SELECT mercenary_id, owner_id, mercenary_class, name, level, \
                 hp, max_hp, sp, max_sp, atk, defense, magic_defense, \
                 str, agi, vit, int, dex, luk, hit, flee, walk_speed, attack_range, \
                 loyalty, contract_end, contract_cost, alive \
                 FROM mercenaries WHERE owner_id = ?",
                &[&char_id],
            )
            .map_err(|e| MercenaryError::Database(e.to_string()))?;

        let mut mercenaries = Vec::new();
        for row in &rows {
            let mercenary_id = row.get_i32(0).unwrap_or(0) as u32;

            // 加载技能（从 mercenary_skills 表）
            let skill_rows = self
                .db
                .query_rows(
                    "SELECT skill_name, skill_level FROM mercenary_skills WHERE mercenary_id = ?",
                    &[&mercenary_id],
                )
                .unwrap_or_default();

            let skills: Vec<MercenarySkill> = skill_rows
                .iter()
                .map(|sr| MercenarySkill {
                    skill_name: sr.get_string(0).unwrap_or_default(),
                    max_level: 5,
                    current_level: sr.get_i32(1).unwrap_or(0) as u8,
                })
                .collect();

            // contract_end 存储为 Unix 时间戳（秒），转换为 DateTime<Utc>
            let contract_end = row
                .get_optional_i64(23)
                .unwrap_or(None)
                .map(|ts| chrono::DateTime::from_timestamp(ts, 0).unwrap_or_default());

            let merc = Mercenary {
                mercenary_id,
                owner_id: row.get_i32(1).unwrap_or(0) as u32,
                mercenary_class: row.get_i32(2).unwrap_or(0) as u16,
                name: row.get_string(3).unwrap_or_default(),
                level: row.get_i32(4).unwrap_or(1) as u16,
                hp: row.get_i32(5).unwrap_or(1000) as u32,
                max_hp: row.get_i32(6).unwrap_or(1000) as u32,
                sp: row.get_i32(7).unwrap_or(100) as u32,
                max_sp: row.get_i32(8).unwrap_or(100) as u32,
                atk: row.get_i32(9).unwrap_or(50) as u32,
                defense: row.get_i32(10).unwrap_or(0) as u32,
                magic_defense: row.get_i32(11).unwrap_or(0) as u32,
                str: row.get_i32(12).unwrap_or(1) as u16,
                agi: row.get_i32(13).unwrap_or(1) as u16,
                vit: row.get_i32(14).unwrap_or(1) as u16,
                int: row.get_i32(15).unwrap_or(1) as u16,
                dex: row.get_i32(16).unwrap_or(1) as u16,
                luk: row.get_i32(17).unwrap_or(1) as u16,
                hit: row.get_i32(18).unwrap_or(0) as i16,
                flee: row.get_i32(19).unwrap_or(0) as i16,
                walk_speed: row.get_i32(20).unwrap_or(200) as u16,
                attack_range: row.get_i32(21).unwrap_or(1) as u16,
                loyalty: row.get_i32(22).unwrap_or(100) as u32,
                contract_end,
                contract_cost: row.get_i32(24).unwrap_or(0) as u32,
                alive: row.get_i32(25).unwrap_or(1) != 0,
                skills,
            };

            self.mercenaries.write().insert(mercenary_id, merc.clone());
            mercenaries.push(merc);
        }

        Ok(mercenaries)
    }

    /// 创建雇佣兵（INSERT 到数据库 + 加入内存）
    pub fn create(
        &self,
        owner_id: u32,
        class_id: u16,
    ) -> Result<Mercenary, MercenaryError> {
        let template = self
            .database
            .get(class_id)
            .ok_or(MercenaryError::NotFound(class_id as u32))?;

        let mercenary_id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let contract_end = Some(Utc::now() + Duration::hours(48));
        let contract_end_ts = contract_end
            .map(|dt| dt.timestamp())
            .unwrap_or(0);

        let mercenary = Mercenary {
            mercenary_id,
            owner_id,
            mercenary_class: class_id,
            name: template.name.clone(),
            level: template.level,
            hp: template.hp,
            max_hp: template.hp,
            sp: template.sp,
            max_sp: template.sp,
            atk: template.atk,
            defense: template.defense,
            magic_defense: template.magic_defense,
            str: template.str,
            agi: template.agi,
            vit: template.vit,
            int: template.int,
            dex: template.dex,
            luk: template.luk,
            hit: template.dex as i16,
            flee: template.agi as i16,
            walk_speed: template.walk_speed,
            attack_range: template.attack_range,
            loyalty: 100,
            contract_end,
            contract_cost: template.contract_cost,
            alive: true,
            skills: template
                .skills
                .iter()
                .map(|(name, max)| MercenarySkill {
                    skill_name: name.clone(),
                    max_level: *max,
                    current_level: 0,
                })
                .collect(),
        };

        // INSERT 到数据库
        self.db
            .execute_params(
                "INSERT INTO mercenaries (mercenary_id, owner_id, mercenary_class, name, level, \
                 hp, max_hp, sp, max_sp, atk, defense, magic_defense, \
                 str, agi, vit, int, dex, luk, hit, flee, walk_speed, attack_range, \
                 loyalty, contract_end, contract_cost, alive, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                &[
                    &mercenary_id as &dyn crate::storage::backend::IntoValue,
                    &owner_id,
                    &(class_id as i32),
                    &template.name,
                    &(template.level as i32),
                    &(template.hp as i32),
                    &(template.hp as i32),
                    &(template.sp as i32),
                    &(template.sp as i32),
                    &(template.atk as i32),
                    &(template.defense as i32),
                    &(template.magic_defense as i32),
                    &(template.str as i32),
                    &(template.agi as i32),
                    &(template.vit as i32),
                    &(template.int as i32),
                    &(template.dex as i32),
                    &(template.luk as i32),
                    &(template.dex as i32),
                    &(template.agi as i32),
                    &(template.walk_speed as i32),
                    &(template.attack_range as i32),
                    &100i32,
                    &contract_end_ts,
                    &(template.contract_cost as i32),
                    &1i32,
                    &crate::storage::chrono_now(),
                ],
            )
            .map_err(|e| MercenaryError::Database(e.to_string()))?;

        // 写入技能到 mercenary_skills 表
        for skill in &mercenary.skills {
            self.db
                .execute_params(
                    "INSERT INTO mercenary_skills (mercenary_id, skill_name, skill_level) VALUES (?, ?, ?)",
                    &[
                        &mercenary_id as &dyn crate::storage::backend::IntoValue,
                        &skill.skill_name,
                        &(skill.current_level as i32),
                    ],
                )
                .map_err(|e| MercenaryError::Database(e.to_string()))?;
        }

        self.mercenaries
            .write()
            .insert(mercenary_id, mercenary.clone());
        Ok(mercenary)
    }

    /// 召唤雇佣兵
    pub fn summon(&self, char_id: u32, mercenary_id: u32) -> Result<(), MercenaryError> {
        if self.summoned.read().contains_key(&char_id) {
            return Err(MercenaryError::AlreadySummoned);
        }

        let mercenaries = self.mercenaries.read();
        let mercenary = mercenaries
            .get(&mercenary_id)
            .ok_or(MercenaryError::NotFound(mercenary_id))?;

        if mercenary.owner_id != char_id {
            return Err(MercenaryError::NotYours);
        }

        if mercenary.is_contract_expired() {
            return Err(MercenaryError::ContractExpired);
        }

        if !mercenary.alive {
            return Err(MercenaryError::Dead);
        }

        drop(mercenaries);
        self.summoned.write().insert(char_id, mercenary_id);
        Ok(())
    }

    /// 解散雇佣兵
    pub fn dismiss(&self, char_id: u32) -> Option<u32> {
        self.summoned.write().remove(&char_id)
    }

    /// 获取玩家召唤的雇佣兵
    pub fn get_summoned(&self, char_id: u32) -> Option<Mercenary> {
        let summoned = self.summoned.read();
        let mercenary_id = summoned.get(&char_id)?;
        self.mercenaries.read().get(mercenary_id).cloned()
    }

    /// 获取雇佣兵（通过 ID）
    pub fn get(&self, mercenary_id: u32) -> Option<Mercenary> {
        self.mercenaries.read().get(&mercenary_id).cloned()
    }

    /// 更新合同（检查到期，自动解散，从数据库删除）
    pub fn update_contracts(&self) -> Vec<u32> {
        let mut dismissed = Vec::new();
        let summoned = self.summoned.read();
        let mercenaries = self.mercenaries.read();

        for (char_id, mercenary_id) in summoned.iter() {
            if let Some(mercenary) = mercenaries.get(mercenary_id) {
                if mercenary.is_contract_expired() {
                    dismissed.push(*char_id);
                }
            }
        }

        drop(summoned);
        drop(mercenaries);

        // 解散到期的雇佣兵并从数据库删除
        for char_id in &dismissed {
            if let Some(mercenary_id) = self.summoned.write().remove(char_id) {
                self.mercenaries.write().remove(&mercenary_id);
                // 从数据库删除
                if let Err(e) = self.db.execute_params(
                    "DELETE FROM mercenaries WHERE mercenary_id = ?",
                    &[&mercenary_id as &dyn crate::storage::backend::IntoValue],
                ) {
                    tracing::warn!("Failed to delete mercenary {}: {}", mercenary_id, e);
                }
                if let Err(e) = self.db.execute_params(
                    "DELETE FROM mercenary_skills WHERE mercenary_id = ?",
                    &[&mercenary_id as &dyn crate::storage::backend::IntoValue],
                ) {
                    tracing::warn!("Failed to delete mercenary skills {}: {}", mercenary_id, e);
                }
            }
        }

        dismissed
    }

    /// 增加忠诚度（UPDATE loyalty 到数据库）
    pub fn increase_loyalty(
        &self,
        mercenary_id: u32,
        amount: u32,
    ) -> Result<(), MercenaryError> {
        let mut mercenaries = self.mercenaries.write();
        let mercenary = mercenaries
            .get_mut(&mercenary_id)
            .ok_or(MercenaryError::NotFound(mercenary_id))?;
        mercenary.increase_loyalty(amount);

        // 实时写库
        self.db
            .execute_params(
                "UPDATE mercenaries SET loyalty = ? WHERE mercenary_id = ?",
                &[
                    &(mercenary.loyalty as i32) as &dyn crate::storage::backend::IntoValue,
                    &mercenary_id,
                ],
            )
            .map_err(|e| MercenaryError::Database(e.to_string()))?;

        Ok(())
    }

    /// 获取雇佣兵模板数据库
    pub fn database(&self) -> &MercenaryDatabase {
        &self.database
    }
}

// 注意：MercenaryManager 不再实现 Default，
// 因为构造时必须传入 Database 实例。

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// 创建测试用管理器（内存数据库 + 建表）
    fn create_test_manager() -> MercenaryManager {
        let db = Arc::new(Database::open_memory().expect("创建测试数据库失败"));
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS mercenaries (
                mercenary_id INTEGER PRIMARY KEY,
                owner_id INTEGER NOT NULL,
                mercenary_class INTEGER NOT NULL,
                name TEXT NOT NULL,
                level INTEGER DEFAULT 1,
                hp INTEGER DEFAULT 1000,
                max_hp INTEGER DEFAULT 1000,
                sp INTEGER DEFAULT 100,
                max_sp INTEGER DEFAULT 100,
                atk INTEGER DEFAULT 50,
                loyalty INTEGER DEFAULT 100,
                contract_end INTEGER,
                alive INTEGER DEFAULT 1,
                created_at INTEGER NOT NULL,
                defense INTEGER DEFAULT 0,
                magic_defense INTEGER DEFAULT 0,
                str INTEGER DEFAULT 1,
                agi INTEGER DEFAULT 1,
                vit INTEGER DEFAULT 1,
                int INTEGER DEFAULT 1,
                dex INTEGER DEFAULT 1,
                luk INTEGER DEFAULT 1,
                hit INTEGER DEFAULT 0,
                flee INTEGER DEFAULT 0,
                walk_speed INTEGER DEFAULT 200,
                attack_range INTEGER DEFAULT 1,
                contract_cost INTEGER DEFAULT 0
            )",
        )
        .expect("创建 mercenaries 表失败");
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS mercenary_skills (
                mercenary_id INTEGER NOT NULL,
                skill_name TEXT NOT NULL,
                skill_level INTEGER DEFAULT 1,
                PRIMARY KEY (mercenary_id, skill_name)
            )",
        )
        .expect("创建 mercenary_skills 表失败");
        MercenaryManager::new_hardcoded(db)
    }

    #[test]
    fn test_create_mercenary() {
        let manager = create_test_manager();
        let merc = manager.create(100, 6017).unwrap();

        assert_eq!(merc.owner_id, 100);
        assert_eq!(merc.name, "Mina");
        assert_eq!(merc.level, 20);
        assert!(merc.alive);
        assert!(merc.contract_end.is_some());
        assert_eq!(merc.loyalty, 100);
    }

    #[test]
    fn test_create_mercenary_not_found() {
        let manager = create_test_manager();
        assert!(matches!(
            manager.create(100, 9999),
            Err(MercenaryError::NotFound(9999))
        ));
    }

    #[test]
    fn test_summon_dismiss() {
        let manager = create_test_manager();
        let merc = manager.create(100, 6017).unwrap();

        assert!(manager.summon(100, merc.mercenary_id).is_ok());
        assert!(manager.get_summoned(100).is_some());

        manager.dismiss(100);
        assert!(manager.get_summoned(100).is_none());
    }

    #[test]
    fn test_cannot_summon_twice() {
        let manager = create_test_manager();
        let merc1 = manager.create(100, 6017).unwrap();
        let merc2 = manager.create(100, 6018).unwrap();

        manager.summon(100, merc1.mercenary_id).unwrap();
        assert!(matches!(
            manager.summon(100, merc2.mercenary_id),
            Err(MercenaryError::AlreadySummoned)
        ));
    }

    #[test]
    fn test_wrong_owner() {
        let manager = create_test_manager();
        let merc = manager.create(100, 6017).unwrap();

        assert!(matches!(
            manager.summon(999, merc.mercenary_id),
            Err(MercenaryError::NotYours)
        ));
    }

    #[test]
    fn test_contract_expired() {
        let manager = create_test_manager();
        let merc = manager.create(100, 6017).unwrap();

        // 手动设置合同已过期
        {
            let mut mercenaries = manager.mercenaries.write();
            let m = mercenaries.get_mut(&merc.mercenary_id).unwrap();
            m.contract_end = Some(Utc::now() - Duration::hours(1));
        }

        assert!(matches!(
            manager.summon(100, merc.mercenary_id),
            Err(MercenaryError::ContractExpired)
        ));
    }

    #[test]
    fn test_loyalty() {
        let manager = create_test_manager();
        let merc = manager.create(100, 6017).unwrap();

        manager
            .increase_loyalty(merc.mercenary_id, 50)
            .unwrap();

        let m = manager.get(merc.mercenary_id).unwrap();
        assert_eq!(m.loyalty, 150); // 100 + 50
    }

    #[test]
    fn test_update_contracts() {
        let manager = create_test_manager();
        let merc = manager.create(100, 6017).unwrap();
        manager.summon(100, merc.mercenary_id).unwrap();

        // 设置合同即将到期
        {
            let mut mercenaries = manager.mercenaries.write();
            let m = mercenaries.get_mut(&merc.mercenary_id).unwrap();
            m.contract_end = Some(Utc::now() - Duration::seconds(1));
        }

        let dismissed = manager.update_contracts();
        assert_eq!(dismissed.len(), 1);
        assert_eq!(dismissed[0], 100);
        assert!(manager.get_summoned(100).is_none());
    }

    #[test]
    fn test_mercenary_skills_from_template() {
        let manager = create_test_manager();
        let merc = manager.create(100, 6017).unwrap();

        assert_eq!(merc.skills.len(), 2);
        assert_eq!(merc.skills[0].skill_name, "MA_DOUBLE");
        assert_eq!(merc.skills[0].max_level, 2);
    }

    /// 测试数据库持久化：create 后重新加载应一致
    #[test]
    fn test_persistence_create_and_load() {
        let manager = create_test_manager();
        let merc = manager.create(100, 6017).unwrap();

        // 清空内存缓存
        manager.mercenaries.write().clear();

        // 从数据库加载
        let loaded = manager.load_for_character(100).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Mina");
        assert_eq!(loaded[0].mercenary_id, merc.mercenary_id);
        assert_eq!(loaded[0].owner_id, 100);
        assert_eq!(loaded[0].mercenary_class, 6017);
        assert_eq!(loaded[0].level, 20);
        assert_eq!(loaded[0].loyalty, 100);
    }

    /// 测试持久化：increase_loyalty 后数据库应更新
    #[test]
    fn test_persistence_loyalty() {
        let manager = create_test_manager();
        let merc = manager.create(100, 6017).unwrap();

        manager.increase_loyalty(merc.mercenary_id, 50).unwrap();

        // 清空内存，从数据库重新加载
        manager.mercenaries.write().clear();

        let loaded = manager.load_for_character(100).unwrap();
        assert_eq!(loaded[0].loyalty, 150); // 100 + 50
    }

    /// 测试持久化：update_contracts 删除过期雇佣兵
    #[test]
    fn test_persistence_update_contracts() {
        let manager = create_test_manager();
        let merc = manager.create(100, 6017).unwrap();
        manager.summon(100, merc.mercenary_id).unwrap();

        // 设置合同已过期
        {
            let mut mercenaries = manager.mercenaries.write();
            let m = mercenaries.get_mut(&merc.mercenary_id).unwrap();
            m.contract_end = Some(Utc::now() - Duration::seconds(1));
        }

        manager.update_contracts();

        // 验证数据库中也已删除
        let loaded = manager.load_for_character(100).unwrap();
        assert_eq!(loaded.len(), 0);
    }

    /// 测试多角色隔离
    #[test]
    fn test_multi_character_isolation() {
        let manager = create_test_manager();
        manager.create(100, 6017).unwrap();
        manager.create(200, 6019).unwrap();

        let loaded_100 = manager.load_for_character(100).unwrap();
        let loaded_200 = manager.load_for_character(200).unwrap();

        assert_eq!(loaded_100.len(), 1);
        assert_eq!(loaded_200.len(), 1);
        assert_eq!(loaded_100[0].name, "Mina");
        assert_eq!(loaded_200[0].name, "Lance");
    }
}
