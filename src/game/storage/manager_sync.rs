//! 仓库同步管理器
//! 整合 StorageManager 和 StorageSyncScheduler

use std::sync::Arc;
use std::time::Duration;

use super::manager::StorageManager;
use super::protocol::{StorageRequest, StorageResponse};
use super::repository::StorageRepository;
use super::scheduler::{StorageSyncScheduler, SyncTask};
use super::sync::SyncState;
use crate::error::Result;

/// 仓库同步管理器
/// 负责管理仓库的加载、保存和同步
pub struct StorageSyncManager {
    /// 存储管理器
    storage_manager: Arc<StorageManager>,
    /// 仓库仓库
    repository: StorageRepository,
    /// 同步调度器
    scheduler: StorageSyncScheduler,
    /// 默认仓库大小
    default_storage_size: u16,
}

impl StorageSyncManager {
    /// 创建新的同步管理器
    pub fn new(
        storage_manager: Arc<StorageManager>,
        repository: StorageRepository,
        sync_interval: Duration,
        default_storage_size: u16,
    ) -> Self {
        let scheduler = StorageSyncScheduler::new(
            repository.clone(),
            storage_manager.clone(),
            sync_interval,
        );

        Self {
            storage_manager,
            repository,
            scheduler,
            default_storage_size,
        }
    }

    /// 获取默认仓库大小
    pub fn default_size(&self) -> u16 {
        self.default_storage_size
    }

    /// 获取存储管理器
    pub fn storage_manager(&self) -> Arc<StorageManager> {
        self.storage_manager.clone()
    }

    /// 获取仓库仓库
    pub fn repository(&self) -> &StorageRepository {
        &self.repository
    }

    /// 获取同步调度器
    pub fn scheduler(&self) -> &StorageSyncScheduler {
        &self.scheduler
    }

    /// 处理仓库请求
    pub async fn handle_request(&self, request: StorageRequest) -> StorageResponse {
        match request {
            StorageRequest::Load { char_id } => self.load_storage(char_id).await,
            StorageRequest::Save { char_id, slots } => self.save_storage(char_id, slots).await,
            StorageRequest::Resize { char_id, new_size } => {
                self.resize_storage(char_id, new_size).await
            }
            StorageRequest::Unlock { char_id } => self.unlock_storage(char_id),
            StorageRequest::SyncStatus { char_id } => self.get_sync_status(char_id),
        }
    }

    /// 加载仓库
    /// 先从数据库加载，如果不存在则创建新的
    pub async fn load_storage(&self, char_id: u32) -> StorageResponse {
        match self.repository.load(char_id).await {
            Ok(Some(storage)) => {
                // 加载到内存
                let max_size = storage.len() as u16;
                let storage_arc = self.storage_manager.get_or_create(char_id, max_size);

                // 复制数据到内存
                {
                    let mut mem_storage = storage_arc.write();
                    *mem_storage = storage;
                }

                // 标记为干净
                if let Err(e) = self
                    .scheduler
                    .task_sender()
                    .try_send(SyncTask::MarkClean(char_id))
                {
                    tracing::error!("Storage sync channel full or closed, data may be lost: {}", e);
                }

                // 返回数据
                let slots: Vec<_> = storage_arc.read().slots().to_vec();
                StorageResponse::Data { char_id, slots }
            }
            Ok(None) => {
                // 创建新仓库
                let storage_arc = self
                    .storage_manager
                    .get_or_create(char_id, self.default_storage_size);
                let slots: Vec<_> = storage_arc.read().slots().to_vec();
                StorageResponse::Data { char_id, slots }
            }
            Err(e) => {
                tracing::error!("Failed to load storage for char_id {}: {}", char_id, e);
                StorageResponse::error(char_id, format!("Load failed: {}", e))
            }
        }
    }

    /// 保存仓库
    pub async fn save_storage(
        &self,
        char_id: u32,
        slots: Vec<super::data::StorageSlot>,
    ) -> StorageResponse {
        // 获取内存中的仓库
        let storage_arc = match self.storage_manager.get(char_id) {
            Some(s) => s,
            None => {
                return StorageResponse::error(char_id, "Storage not found");
            }
        };

        // 更新内存数据
        {
            let mut storage = storage_arc.write();
            for slot in slots.iter() {
                if let Some(s) = storage.get_slot_mut(slot.index) {
                    *s = slot.clone();
                }
            }
        }

        // 标记为脏
        if let Err(e) = self
            .scheduler
            .task_sender()
            .try_send(SyncTask::MarkDirty(char_id))
        {
            tracing::error!("Storage sync channel full or closed, data may be lost: {}", e);
        }

        // 保存到数据库
        let storage = storage_arc.read().clone();
        match self.repository.save(&storage).await {
            Ok(()) => {
                // 标记为干净
                if let Err(e) = self
                    .scheduler
                    .task_sender()
                    .try_send(SyncTask::MarkClean(char_id))
                {
                    tracing::error!("Storage sync channel full or closed, data may be lost: {}", e);
                }
                StorageResponse::success(char_id)
            }
            Err(e) => {
                tracing::error!("Failed to save storage for char_id {}: {}", char_id, e);
                StorageResponse::error(char_id, format!("Save failed: {}", e))
            }
        }
    }

    /// 调整仓库大小
    pub async fn resize_storage(&self, char_id: u32, new_size: u16) -> StorageResponse {
        let storage_arc = match self.storage_manager.get(char_id) {
            Some(s) => s,
            None => {
                return StorageResponse::error(char_id, "Storage not found");
            }
        };

        // 标记为脏
        if let Err(e) = self
            .scheduler
            .task_sender()
            .try_send(SyncTask::MarkDirty(char_id))
        {
            tracing::error!("Storage sync channel full or closed, data may be lost: {}", e);
        }

        // 重新创建仓库 (保持现有数据)
        let mut current_slots: Vec<_> = storage_arc.read().slots().to_vec();
        current_slots.resize_with(new_size as usize, || super::data::StorageSlot::empty(0));

        // 更新内存
        {
            let mut storage = storage_arc.write();
            *storage = super::data::Storage::from_db_format(
                char_id,
                new_size,
                current_slots
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        (
                            i as u16,
                            s.item_id,
                            s.amount,
                            s.identified,
                            s.refine,
                            s.cards,
                        )
                    })
                    .collect(),
            );
        }

        StorageResponse::success(char_id)
    }

    /// 解锁仓库
    pub fn unlock_storage(&self, char_id: u32) -> StorageResponse {
        self.storage_manager.remove(&char_id);
        StorageResponse::success(char_id)
    }

    /// 获取同步状态
    pub fn get_sync_status(&self, char_id: u32) -> StorageResponse {
        let state = self
            .scheduler
            .get_sync_state(char_id)
            .unwrap_or(SyncState::Clean);
        let version = self.scheduler.get_version(char_id).unwrap_or(0);

        StorageResponse::SyncStatus {
            char_id,
            is_dirty: state.is_dirty(),
            version,
        }
    }

    /// 强制同步到数据库
    pub async fn force_sync(&self, char_id: u32) -> Result<()> {
        let storage_arc = match self.storage_manager.get(char_id) {
            Some(s) => s,
            None => return Ok(()),
        };

        let storage = storage_arc.read().clone();
        self.repository.save(&storage).await?;

        if let Err(e) = self
            .scheduler
            .task_sender()
            .try_send(SyncTask::MarkClean(char_id))
        {
            tracing::error!("Storage sync channel full or closed, data may be lost: {}", e);
        }

        Ok(())
    }

    /// 保存所有脏数据到数据库
    pub async fn flush_dirty(&self) -> Result<usize> {
        let dirty_ids = self.scheduler.get_dirty_char_ids();
        let mut count = 0;

        for char_id in dirty_ids {
            if let Err(e) = self.force_sync(char_id).await {
                tracing::error!("Failed to flush storage for char_id {}: {}", char_id, e);
            } else {
                count += 1;
            }
        }

        Ok(count)
    }

    /// 获取脏数据统计
    pub fn dirty_stats(&self) -> DirtyStats {
        DirtyStats {
            count: self.scheduler.dirty_count(),
            has_dirty: self.scheduler.has_dirty(),
        }
    }
}

impl Clone for StorageSyncManager {
    fn clone(&self) -> Self {
        Self {
            storage_manager: self.storage_manager.clone(),
            repository: self.repository.clone(),
            scheduler: StorageSyncScheduler::new(
                self.repository.clone(),
                self.storage_manager.clone(),
                Duration::from_secs(30),
            ),
            default_storage_size: self.default_storage_size,
        }
    }
}

/// 脏数据统计
#[derive(Debug, Clone)]
pub struct DirtyStats {
    pub count: usize,
    pub has_dirty: bool,
}
