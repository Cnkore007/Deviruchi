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
    /// 同步调度器（使用 Arc 共享，clone 时不创建新后台任务）
    scheduler: Arc<StorageSyncScheduler>,
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
        let scheduler = Arc::new(StorageSyncScheduler::new(
            repository.clone(),
            storage_manager.clone(),
            sync_interval,
        ));

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
    ///
    /// 更新内存中的仓库格子数，并持久化到数据库。
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
            tracing::error!("仓库同步通道已满或已关闭，数据可能丢失: {}", e);
        }

        // 重新创建仓库（保持现有数据）
        let current_slots: Vec<_> = storage_arc.read().slots().to_vec();
        let mut new_slots: Vec<_> = current_slots.clone();
        new_slots.resize_with(new_size as usize, || super::data::StorageSlot::empty(0));

        // 更新内存
        {
            let mut storage = storage_arc.write();
            *storage = super::data::Storage::from_db_format(
                char_id,
                new_size,
                new_slots
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

        // 持久化到数据库
        let storage = storage_arc.read().clone();
        match self.repository.save(&storage).await {
            Ok(()) => {
                if let Err(e) = self
                    .scheduler
                    .task_sender()
                    .try_send(SyncTask::MarkClean(char_id))
                {
                    tracing::error!("仓库同步通道已满或已关闭: {}", e);
                }
                StorageResponse::success(char_id)
            }
            Err(e) => {
                tracing::error!("仓库大小调整持久化失败: char_id={}, error={}", char_id, e);
                StorageResponse::error(char_id, format!("Resize persist failed: {}", e))
            }
        }
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
            scheduler: self.scheduler.clone(), // Arc 共享同一个调度器实例
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema::init_schema;
    use std::time::Duration;

    /// 创建测试用同步管理器
    fn setup() -> StorageSyncManager {
        let db = Arc::new(crate::storage::Database::open_memory().expect("创建内存数据库失败"));
        init_schema(&db).expect("初始化 schema 失败");

        // 创建测试账户和角色（外键约束）
        db.execute(
            "INSERT INTO accounts (account_id, user_id, password_hash, sex, created_at)
             VALUES (1, 'test', 'hash', 0, 0)",
        )
        .unwrap();
        db.execute(
            "INSERT INTO characters (char_id, account_id, char_num, name, created_at, updated_at)
             VALUES (1, 1, 0, 'Test', 0, 0)",
        )
        .unwrap();

        let repo = StorageRepository::new(db);
        let manager = Arc::new(StorageManager::new());

        StorageSyncManager::new(manager, repo, Duration::from_millis(100), 100)
    }

    /// 加载不存在的仓库时创建新的
    #[tokio::test]
    async fn load_storage_creates_new_when_empty() {
        let sync_mgr = setup();
        let response = sync_mgr.load_storage(1).await;

        match response {
            StorageResponse::Data { char_id, slots } => {
                assert_eq!(char_id, 1);
                assert_eq!(slots.len(), 100); // default_storage_size
                assert!(slots.iter().all(|s| s.is_empty()));
            }
            _ => panic!("期望 Data 响应"),
        }
    }

    /// 完整的 load -> modify -> save -> reload 流程
    #[tokio::test]
    async fn load_then_save_then_reload() {
        let sync_mgr = setup();

        // 1. 加载（创建新仓库）
        sync_mgr.load_storage(1).await;

        // 2. 修改内存中的仓库
        {
            let storage_arc = sync_mgr.storage_manager().get(1).unwrap();
            let mut storage = storage_arc.write();
            storage.add_item(501, 10);
            storage.add_item(601, 1);
        }

        // 3. 保存
        let save_response = sync_mgr.save_storage(1, vec![]).await;
        match save_response {
            StorageResponse::Saved { char_id } => assert_eq!(char_id, 1),
            _ => panic!("期望 Saved 响应"),
        }

        // 4. 重新加载
        sync_mgr.storage_manager().remove(&1); // 清除内存缓存
        let load_response = sync_mgr.load_storage(1).await;
        match load_response {
            StorageResponse::Data { slots, .. } => {
                assert_eq!(slots[0].item_id, 501);
                assert_eq!(slots[0].amount, 10);
                assert_eq!(slots[1].item_id, 601);
            }
            _ => panic!("期望 Data 响应"),
        }
    }

    /// resize 后数据持久化到数据库
    #[tokio::test]
    async fn resize_storage_persists() {
        let sync_mgr = setup();

        // 加载并添加物品
        sync_mgr.load_storage(1).await;
        {
            let storage_arc = sync_mgr.storage_manager().get(1).unwrap();
            storage_arc.write().add_item(501, 5);
        }

        // 保存
        sync_mgr.save_storage(1, vec![]).await;

        // 调整大小
        let resize_response = sync_mgr.resize_storage(1, 200).await;
        match resize_response {
            StorageResponse::Saved { char_id } => assert_eq!(char_id, 1),
            _ => panic!("期望 Saved 响应"),
        }

        // 验证内存中的大小
        {
            let storage_arc = sync_mgr.storage_manager().get(1).unwrap();
            assert_eq!(storage_arc.read().max_size(), 200);
            assert_eq!(storage_arc.read().get_slot(0).unwrap().item_id, 501);
        }

        // 清除缓存后重新加载，验证持久化
        sync_mgr.storage_manager().remove(&1);
        let load_response = sync_mgr.load_storage(1).await;
        match load_response {
            StorageResponse::Data { slots, .. } => {
                assert_eq!(slots.len(), 200); // 新大小
                assert_eq!(slots[0].item_id, 501);
            }
            _ => panic!("期望 Data 响应"),
        }
    }

    /// resize 不存在的仓库返回错误
    #[tokio::test]
    async fn resize_nonexistent_returns_error() {
        let sync_mgr = setup();
        let response = sync_mgr.resize_storage(999, 200).await;
        match response {
            StorageResponse::Error { char_id, .. } => assert_eq!(char_id, 999),
            _ => panic!("期望 Error 响应"),
        }
    }

    /// save 不存在的仓库返回错误
    #[tokio::test]
    async fn save_nonexistent_returns_error() {
        let sync_mgr = setup();
        let response = sync_mgr.save_storage(999, vec![]).await;
        match response {
            StorageResponse::Error { char_id, .. } => assert_eq!(char_id, 999),
            _ => panic!("期望 Error 响应"),
        }
    }

    /// unlock 从内存移除仓库
    #[tokio::test]
    async fn unlock_removes_from_memory() {
        let sync_mgr = setup();

        // 先加载到内存
        sync_mgr.load_storage(1).await;

        assert!(sync_mgr.storage_manager().has_storage(1));

        let response = sync_mgr.unlock_storage(1);
        match response {
            StorageResponse::Saved { char_id } => assert_eq!(char_id, 1),
            _ => panic!("期望 Saved 响应"),
        }

        assert!(!sync_mgr.storage_manager().has_storage(1));
    }

    /// get_sync_status 返回默认状态
    #[tokio::test]
    async fn get_sync_status_returns_default() {
        let sync_mgr = setup();
        let response = sync_mgr.get_sync_status(1);
        match response {
            StorageResponse::SyncStatus {
                char_id,
                is_dirty,
                version,
            } => {
                assert_eq!(char_id, 1);
                assert!(!is_dirty); // 默认 Clean
                assert_eq!(version, 0);
            }
            _ => panic!("期望 SyncStatus 响应"),
        }
    }

    /// force_sync 实际保存到数据库
    #[tokio::test]
    async fn force_sync_saves_to_db() {
        let sync_mgr = setup();

        // 加载并修改
        sync_mgr.load_storage(1).await;
        {
            let storage_arc = sync_mgr.storage_manager().get(1).unwrap();
            storage_arc.write().add_item(501, 10);
        }

        // 强制同步
        sync_mgr.force_sync(1).await.unwrap();

        // 清除缓存后重新加载
        sync_mgr.storage_manager().remove(&1);
        let load_response = sync_mgr.load_storage(1).await;
        match load_response {
            StorageResponse::Data { slots, .. } => {
                assert_eq!(slots[0].item_id, 501);
                assert_eq!(slots[0].amount, 10);
            }
            _ => panic!("期望 Data 响应"),
        }
    }

    /// flush_dirty 保存所有脏数据
    #[tokio::test]
    async fn flush_dirty_saves_all_dirty() {
        let sync_mgr = setup();

        // 创建仓库并添加物品
        {
            let sm = sync_mgr.storage_manager();
            sm.get_or_create(1, 100).write().add_item(501, 1);
        }

        // 标记脏
        sync_mgr
            .scheduler()
            .task_sender()
            .try_send(SyncTask::MarkDirty(1))
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        // flush
        let count = sync_mgr.flush_dirty().await.unwrap();
        assert_eq!(count, 1);
    }

    /// dirty_stats 初始状态正确
    #[tokio::test]
    async fn dirty_stats_reports_correctly() {
        let sync_mgr = setup();
        let stats = sync_mgr.dirty_stats();
        assert_eq!(stats.count, 0);
        assert!(!stats.has_dirty);
    }

    /// default_size 可配置
    #[tokio::test]
    async fn default_size_is_configurable() {
        let sync_mgr = setup();
        assert_eq!(sync_mgr.default_size(), 100);
    }

    /// handle_request 路由正确
    #[tokio::test]
    async fn handle_request_routes_correctly() {
        let sync_mgr = setup();

        // Load
        let response = sync_mgr.handle_request(StorageRequest::Load { char_id: 1 }).await;
        assert!(matches!(response, StorageResponse::Data { char_id: 1, .. }));

        // SyncStatus
        let response = sync_mgr
            .handle_request(StorageRequest::SyncStatus { char_id: 1 })
            .await;
        assert!(matches!(response, StorageResponse::SyncStatus { .. }));

        // Unlock
        let response = sync_mgr.handle_request(StorageRequest::Unlock { char_id: 1 }).await;
        assert!(matches!(response, StorageResponse::Saved { char_id: 1 }));
    }
}
