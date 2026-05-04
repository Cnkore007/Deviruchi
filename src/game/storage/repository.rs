//! 仓库数据库操作
use super::data::{Storage, StorageSlot};
use crate::error::Result;
use crate::storage::Database;
use std::sync::Arc;

/// 仓库仓库 - 处理数据库持久化
pub struct StorageRepository {
    db: Arc<Database>,
}

impl StorageRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// 加载仓库数据
    ///
    /// 优先从 storage_meta 表读取 max_size，如果元数据不存在则回退到 100。
    pub async fn load(&self, char_id: u32) -> Result<Option<Storage>> {
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || {
            // 1. 尝试从 storage_meta 读取 max_size
            let max_size: u16 = db
                .query_row_optional(
                    "SELECT max_size FROM storage_meta WHERE char_id = ?",
                    [char_id as i64],
                    |row| row.get::<_, i32>(0),
                )?
                .map(|v| v as u16)
                .unwrap_or(100); // 默认 100 格

            // 2. 加载物品数据
            let slots: Vec<(i32, i32, i32, i32, i32, i32, i32, i32, i32)> = db.query(
                "SELECT slot_index, item_id, amount, identified, refine, card0, card1, card2, card3
                 FROM storage WHERE char_id = ? ORDER BY slot_index",
                [char_id as i64],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                },
            )?;

            if slots.is_empty() {
                return Ok(None);
            }

            let storage_slots: Vec<StorageSlot> = slots
                .into_iter()
                .map(
                    |(slot_index, item_id, amount, identified, refine, c0, c1, c2, c3)| {
                        StorageSlot {
                            index: slot_index as u16,
                            item_id: item_id as u16,
                            amount: amount as u16,
                            identified: identified != 0,
                            refine: refine as u8,
                            cards: [c0 as u16, c1 as u16, c2 as u16, c3 as u16],
                        }
                    },
                )
                .collect();

            let storage = Storage::from_slots(char_id, max_size, storage_slots);

            Ok(Some(storage))
        })
        .await
        .map_err(|e| crate::error::Error::Game(e.to_string()))?
    }

    /// 保存仓库数据（在单个事务中原子性完成）
    ///
    /// 在单个 IMMEDIATE 事务中：先清理旧记录，再插入新记录，
    /// 同时保存 max_size 到 storage_meta 表。
    pub async fn save(&self, storage: &Storage) -> Result<()> {
        let char_id = storage.char_id();
        let max_size = storage.max_size();
        let slots: Vec<_> = storage
            .slots()
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect();

        let db = self.db.clone();

        tokio::task::spawn_blocking(move || {
            db.with_transaction(|conn| {
                // 先清理该角色的旧记录，再插入新记录（事务保证原子性）
                conn.execute(
                    "DELETE FROM storage WHERE char_id = ?",
                    rusqlite::params![char_id as i64],
                )?;

                // 使用 INSERT OR REPLACE 防止 slot_index 冲突
                for slot in &slots {
                    conn.execute(
                        "INSERT OR REPLACE INTO storage (char_id, slot_index, item_id, amount, identified, refine, card0, card1, card2, card3)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        rusqlite::params![
                            char_id as i64,
                            slot.index as i32,
                            slot.item_id as i32,
                            slot.amount as i32,
                            slot.identified as i32,
                            slot.refine as i32,
                            slot.cards[0] as i32,
                            slot.cards[1] as i32,
                            slot.cards[2] as i32,
                            slot.cards[3] as i32,
                        ],
                    )?;
                }

                // 保存仓库元数据（max_size）—— 使用 UPSERT
                conn.execute(
                    "INSERT INTO storage_meta (char_id, max_size)
                     VALUES (?, ?)
                     ON CONFLICT(char_id) DO UPDATE SET max_size = excluded.max_size",
                    rusqlite::params![char_id as i64, max_size as i32],
                )?;

                Ok(())
            })
        })
        .await
        .map_err(|e| crate::error::Error::Game(e.to_string()))?
    }

    /// 删除仓库数据
    pub async fn delete(&self, char_id: u32) -> Result<()> {
        let db = self.db.clone();
        let char_id = char_id as i64;

        tokio::task::spawn_blocking(move || {
            db.execute_with_params("DELETE FROM storage WHERE char_id = ?", [char_id])?;
            Ok(())
        })
        .await
        .map_err(|e| crate::error::Error::Game(e.to_string()))?
    }

    /// 检查仓库是否存在
    pub async fn exists(&self, char_id: u32) -> Result<bool> {
        let db = self.db.clone();
        let char_id = char_id as i64;

        tokio::task::spawn_blocking(move || {
            let count: i32 = db.query_row(
                "SELECT COUNT(*) FROM storage WHERE char_id = ?",
                [char_id],
                |row| row.get(0),
            )?;
            Ok(count > 0)
        })
        .await
        .map_err(|e| crate::error::Error::Game(e.to_string()))?
    }
}

impl Clone for StorageRepository {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema::init_schema;

    /// 创建测试用内存数据库，包含必要的外键数据
    fn setup_db() -> Arc<Database> {
        let db = Arc::new(Database::open_memory().expect("创建内存数据库失败"));
        init_schema(&db).expect("初始化 schema 失败");

        // 创建测试账户（外键约束需要）
        db.execute(
            "INSERT INTO accounts (account_id, user_id, password_hash, sex, created_at)
             VALUES (1, 'test', 'hash', 0, 0)",
        )
        .expect("创建测试账户失败");
        // 创建测试角色（外键约束需要）
        db.execute(
            "INSERT INTO characters (char_id, account_id, char_num, name, created_at, updated_at)
             VALUES (1, 1, 0, 'Test', 0, 0)",
        )
        .expect("创建测试角色失败");

        db
    }

    /// 保存后加载往返一致性
    #[tokio::test]
    async fn save_and_load_roundtrip() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        let mut storage = Storage::new(100).with_char_id(1);
        storage.add_item(501, 10);
        storage.add_item(601, 1);

        repo.save(&storage).await.unwrap();

        let loaded = repo.load(1).await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.get_slot(0).unwrap().item_id, 501);
        assert_eq!(loaded.get_slot(0).unwrap().amount, 10);
        assert_eq!(loaded.get_slot(1).unwrap().item_id, 601);
    }

    /// 加载不存在的角色返回 None
    #[tokio::test]
    async fn load_returns_none_for_empty() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        let loaded = repo.load(999).await.unwrap();
        assert!(loaded.is_none());
    }

    /// 保存精炼和卡片数据不丢失
    #[tokio::test]
    async fn save_preserves_refine_and_cards() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        let mut storage = Storage::new(100).with_char_id(1);
        storage.add_item_full(1101, 1, true, 7, [4001, 4002, 0, 0]);

        repo.save(&storage).await.unwrap();

        let loaded = repo.load(1).await.unwrap().unwrap();
        let slot = loaded.get_slot(0).unwrap();
        assert_eq!(slot.item_id, 1101);
        assert_eq!(slot.refine, 7);
        assert_eq!(slot.cards, [4001, 4002, 0, 0]);
    }

    /// 重复保存覆盖旧数据
    #[tokio::test]
    async fn save_overwrites_existing_data() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        // 第一次保存
        let mut storage = Storage::new(100).with_char_id(1);
        storage.add_item(501, 10);
        repo.save(&storage).await.unwrap();

        // 第二次保存（不同物品）
        let mut storage = Storage::new(100).with_char_id(1);
        storage.add_item(601, 5);
        repo.save(&storage).await.unwrap();

        let loaded = repo.load(1).await.unwrap().unwrap();
        assert_eq!(loaded.get_slot(0).unwrap().item_id, 601);
        assert_eq!(loaded.get_slot(0).unwrap().amount, 5);
        // 旧数据不应存在
        assert!(loaded.find_item_slot(501).is_none());
    }

    /// 删除仓库数据
    #[tokio::test]
    async fn delete_removes_storage() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        let mut storage = Storage::new(100).with_char_id(1);
        storage.add_item(501, 10);
        repo.save(&storage).await.unwrap();

        assert!(repo.exists(1).await.unwrap());

        repo.delete(1).await.unwrap();

        assert!(!repo.exists(1).await.unwrap());
        assert!(repo.load(1).await.unwrap().is_none());
    }

    /// 删除不存在的角色不报错
    #[tokio::test]
    async fn delete_nonexistent_is_noop() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        // 删除不存在的角色，不应报错
        repo.delete(999).await.unwrap();
    }

    /// exists 正确反映仓库存在状态
    #[tokio::test]
    async fn exists_returns_correctly() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        assert!(!repo.exists(1).await.unwrap());

        // 空仓库 save 后 storage 表无记录，但 storage_meta 有记录
        let storage = Storage::new(100).with_char_id(1);
        repo.save(&storage).await.unwrap();

        // 添加物品后 save
        let mut storage = Storage::new(100).with_char_id(1);
        storage.add_item(501, 1);
        repo.save(&storage).await.unwrap();

        assert!(repo.exists(1).await.unwrap());
    }

    /// 保存并恢复 max_size
    #[tokio::test]
    async fn save_load_preserves_max_size() {
        let db = setup_db();
        let repo = StorageRepository::new(db);

        let mut storage = Storage::new(200).with_char_id(1);
        storage.add_item(501, 1);
        repo.save(&storage).await.unwrap();

        let loaded = repo.load(1).await.unwrap().unwrap();
        assert_eq!(loaded.max_size(), 200);
    }

    /// clone 共享同一数据库连接
    #[tokio::test]
    async fn clone_shares_same_db() {
        let db = setup_db();
        let repo = StorageRepository::new(db.clone());
        let repo2 = repo.clone();

        let mut storage = Storage::new(100).with_char_id(1);
        storage.add_item(501, 10);
        repo.save(&storage).await.unwrap();

        // 通过 clone 出的 repo 也能读到数据
        let loaded = repo2.load(1).await.unwrap().unwrap();
        assert_eq!(loaded.get_slot(0).unwrap().item_id, 501);
    }
}
