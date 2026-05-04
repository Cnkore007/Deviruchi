//! 数据库迁移框架
//!
//! 使用 schema_version 表追踪当前数据库版本。
//! 每个迁移有 up (升级) 和 down (降级) 操作。
//! 迁移按版本号单调递增，幂等执行。

use crate::error::Result;
use crate::storage::Database;
use std::collections::BTreeMap;

/// 迁移定义
pub struct Migration {
    /// 版本号（单调递增）
    pub version: u32,
    /// 迁移描述
    pub description: &'static str,
    /// 升级 SQL
    pub up: &'static str,
    /// 降级 SQL（可选）
    pub down: Option<&'static str>,
}

/// 迁移管理器
pub struct MigrationManager {
    migrations: BTreeMap<u32, Migration>,
}

impl MigrationManager {
    pub fn new() -> Self {
        Self {
            migrations: BTreeMap::new(),
        }
    }

    /// 注册迁移
    pub fn register(&mut self, migration: Migration) {
        self.migrations.insert(migration.version, migration);
    }

    /// 初始化 schema_version 表
    fn ensure_version_table(&self, db: &Database) -> Result<()> {
        db.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at INTEGER NOT NULL
            )",
        )?;
        Ok(())
    }

    /// 获取当前数据库版本
    pub fn current_version(&self, db: &Database) -> Result<u32> {
        self.ensure_version_table(db)?;

        let version = db.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get::<_, u32>(0),
        )?;

        Ok(version)
    }

    /// 执行所有待执行的升级迁移
    pub fn migrate_up(&self, db: &Database) -> Result<u32> {
        self.ensure_version_table(db)?;
        let current = self.current_version(db)?;
        let mut applied = 0;

        for (version, migration) in &self.migrations {
            if *version > current {
                tracing::info!(
                    "执行迁移 v{}: {}",
                    version,
                    migration.description
                );

                // 将迁移 SQL 和版本记录写入同一个事务，保证原子性
                let up_sql = migration.up;
                let desc = migration.description;
                db.with_transaction(|conn| {
                    // 执行迁移 SQL（可能包含多条语句，用 execute_batch）
                    conn.execute_batch(up_sql)
                        .map_err(|e| crate::error::Error::Database(e))?;

                    // 获取当前时间戳（秒）
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;

                    // 记录迁移版本
                    conn.execute(
                        "INSERT INTO schema_version (version, description, applied_at) VALUES (?1, ?2, ?3)",
                        rusqlite::params![version, desc, now],
                    )?;

                    Ok(())
                })?;

                applied += 1;
            }
        }

        if applied > 0 {
            tracing::info!("应用了 {} 个迁移", applied);
        }

        Ok(applied)
    }

    /// 降级到指定版本
    pub fn migrate_down(&self, db: &Database, target_version: u32) -> Result<u32> {
        self.ensure_version_table(db)?;
        let current = self.current_version(db)?;
        let mut reverted = 0;

        // 按版本降序执行降级
        for (version, migration) in self.migrations.iter().rev() {
            if *version > target_version && *version <= current {
                if let Some(down_sql) = migration.down {
                    tracing::info!(
                        "回滚迁移 v{}: {}",
                        version,
                        migration.description
                    );

                    db.execute(down_sql)?;

                    db.execute_with_params(
                        "DELETE FROM schema_version WHERE version = ?1",
                        rusqlite::params![version],
                    )?;

                    reverted += 1;
                } else {
                    tracing::warn!("迁移 v{} 不支持降级", version);
                }
            }
        }

        Ok(reverted)
    }

    /// 检查是否有待执行的迁移
    pub fn has_pending(&self, db: &Database) -> Result<bool> {
        let current = self.current_version(db)?;
        Ok(self.migrations.keys().any(|v| *v > current))
    }

    /// 获取所有已注册的迁移版本
    pub fn registered_versions(&self) -> Vec<u32> {
        self.migrations.keys().copied().collect()
    }
}

impl Default for MigrationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 创建默认迁移管理器（包含所有内置迁移）
pub fn create_default_migrations() -> MigrationManager {
    let mut manager = MigrationManager::new();

    // 迁移 v1: 添加 homunculus 表
    manager.register(Migration {
        version: 1,
        description: "创建 homunculus 表",
        up: "CREATE TABLE IF NOT EXISTS homunculus (
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
            created_at INTEGER NOT NULL
        )",
        down: Some("DROP TABLE IF EXISTS homunculus"),
    });

    // 迁移 v2: 添加 mercenary 表
    manager.register(Migration {
        version: 2,
        description: "创建 mercenary 表",
        up: "CREATE TABLE IF NOT EXISTS mercenaries (
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
            created_at INTEGER NOT NULL
        )",
        down: Some("DROP TABLE IF EXISTS mercenaries"),
    });

    // 迁移 v3: 添加 pet 持久化表
    manager.register(Migration {
        version: 3,
        description: "创建 pets 表",
        up: "CREATE TABLE IF NOT EXISTS pets (
            pet_id INTEGER PRIMARY KEY,
            owner_id INTEGER NOT NULL,
            monster_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            renamed INTEGER DEFAULT 0,
            intimacy INTEGER DEFAULT 10000,
            hunger INTEGER DEFAULT 500,
            level INTEGER DEFAULT 1,
            egg_id INTEGER DEFAULT 0,
            equip_id INTEGER DEFAULT 0,
            born_at INTEGER NOT NULL
        )",
        down: Some("DROP TABLE IF EXISTS pets"),
    });

    manager
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> Database {
        Database::open(":memory:").expect("创建测试数据库失败")
    }

    #[test]
    fn test_schema_version_table_created() {
        let db = create_test_db();
        let manager = MigrationManager::new();

        manager.ensure_version_table(&db).unwrap();

        // 验证表存在
        let version = manager.current_version(&db).unwrap();
        assert_eq!(version, 0);
    }

    #[test]
    fn test_migrate_up_single() {
        let db = create_test_db();
        let mut manager = MigrationManager::new();

        manager.register(Migration {
            version: 1,
            description: "测试迁移",
            up: "CREATE TABLE test_table (id INTEGER PRIMARY KEY)",
            down: Some("DROP TABLE test_table"),
        });

        let applied = manager.migrate_up(&db).unwrap();
        assert_eq!(applied, 1);

        let version = manager.current_version(&db).unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn test_migrate_up_idempotent() {
        let db = create_test_db();
        let mut manager = MigrationManager::new();

        manager.register(Migration {
            version: 1,
            description: "测试迁移",
            up: "CREATE TABLE test_table (id INTEGER PRIMARY KEY)",
            down: Some("DROP TABLE test_table"),
        });

        // 第一次执行
        let applied1 = manager.migrate_up(&db).unwrap();
        assert_eq!(applied1, 1);

        // 第二次执行应该不应用任何迁移
        let applied2 = manager.migrate_up(&db).unwrap();
        assert_eq!(applied2, 0);
    }

    #[test]
    fn test_migrate_down() {
        let db = create_test_db();
        let mut manager = MigrationManager::new();

        manager.register(Migration {
            version: 1,
            description: "创建表",
            up: "CREATE TABLE test_table (id INTEGER PRIMARY KEY)",
            down: Some("DROP TABLE test_table"),
        });

        manager.register(Migration {
            version: 2,
            description: "添加列",
            up: "ALTER TABLE test_table ADD COLUMN name TEXT",
            down: Some("ALTER TABLE test_table DROP COLUMN name"),
        });

        // 升级到 v2
        manager.migrate_up(&db).unwrap();
        assert_eq!(manager.current_version(&db).unwrap(), 2);

        // 降级到 v1
        let reverted = manager.migrate_down(&db, 1).unwrap();
        assert_eq!(reverted, 1);
        assert_eq!(manager.current_version(&db).unwrap(), 1);
    }

    #[test]
    fn test_migrate_down_to_zero() {
        let db = create_test_db();
        let mut manager = MigrationManager::new();

        manager.register(Migration {
            version: 1,
            description: "测试",
            up: "CREATE TABLE t1 (id INTEGER)",
            down: Some("DROP TABLE t1"),
        });

        manager.migrate_up(&db).unwrap();
        let reverted = manager.migrate_down(&db, 0).unwrap();
        assert_eq!(reverted, 1);
        assert_eq!(manager.current_version(&db).unwrap(), 0);
    }

    #[test]
    fn test_has_pending() {
        let db = create_test_db();
        let mut manager = MigrationManager::new();

        manager.register(Migration {
            version: 1,
            description: "测试",
            up: "CREATE TABLE t1 (id INTEGER)",
            down: None,
        });

        assert!(manager.has_pending(&db).unwrap());

        manager.migrate_up(&db).unwrap();

        assert!(!manager.has_pending(&db).unwrap());
    }

    #[test]
    fn test_multiple_migrations_order() {
        let db = create_test_db();
        let mut manager = MigrationManager::new();

        // 故意乱序注册
        manager.register(Migration {
            version: 3,
            description: "第三个",
            up: "CREATE TABLE t3 (id INTEGER)",
            down: Some("DROP TABLE t3"),
        });
        manager.register(Migration {
            version: 1,
            description: "第一个",
            up: "CREATE TABLE t1 (id INTEGER)",
            down: Some("DROP TABLE t1"),
        });
        manager.register(Migration {
            version: 2,
            description: "第二个",
            up: "CREATE TABLE t2 (id INTEGER)",
            down: Some("DROP TABLE t2"),
        });

        let applied = manager.migrate_up(&db).unwrap();
        assert_eq!(applied, 3);
        assert_eq!(manager.current_version(&db).unwrap(), 3);
    }

    #[test]
    fn test_no_down_migration_warning() {
        let db = create_test_db();
        let mut manager = MigrationManager::new();

        manager.register(Migration {
            version: 1,
            description: "不可降级",
            up: "CREATE TABLE t1 (id INTEGER)",
            down: None,
        });

        manager.migrate_up(&db).unwrap();

        // 尝试降级，应该返回 0（没有执行任何降级）
        let reverted = manager.migrate_down(&db, 0).unwrap();
        assert_eq!(reverted, 0);
        // 版本不变
        assert_eq!(manager.current_version(&db).unwrap(), 1);
    }
}
