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
                // 清理旧记录
                conn.execute(
                    "DELETE FROM storage WHERE char_id = ?",
                    rusqlite::params![char_id as i64],
                )?;

                // 插入新记录
                for slot in &slots {
                    conn.execute(
                        "INSERT INTO storage (char_id, slot_index, item_id, amount, identified, refine, card0, card1, card2, card3)
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

                // 保存仓库元数据（max_size）
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
