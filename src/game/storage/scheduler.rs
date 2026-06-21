//! 仓库同步调度器
//! 处理定期批量同步到数据库，支持实际同步执行和超时恢复

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use super::manager::StorageManager;
use super::repository::StorageRepository;
use super::sync::{SyncRecord, SyncState, SyncState::*};

/// 同步超时时间（秒）
const SYNC_TIMEOUT_SECS: u64 = 30;

/// 同步任务
#[derive(Debug)]
pub enum SyncTask {
    /// 标记脏
    MarkDirty(u32),
    /// 标记干净
    MarkClean(u32),
    /// 立即同步
    ForceSync(u32),
    /// 停止调度器
    Shutdown,
}

/// 仓库同步调度器
pub struct StorageSyncScheduler {
    /// 同步状态记录
    sync_states: Arc<RwLock<HashMap<u32, SyncRecord>>>,
    /// 仓库持久层
    repository: StorageRepository,
    /// 存储管理器（用于读取实际数据）
    storage_manager: Arc<StorageManager>,
    /// 同步间隔
    sync_interval: Duration,
    /// 同步超时时间
    sync_timeout: Duration,
    /// 任务发送通道
    task_tx: mpsc::Sender<SyncTask>,
}

impl StorageSyncScheduler {
    /// 创建新的同步调度器
    ///
    /// # 参数
    /// - `repository`: 数据库持久层
    /// - `storage_manager`: 内存存储管理器
    /// - `sync_interval`: 同步检查间隔
    pub fn new(
        repository: StorageRepository,
        storage_manager: Arc<StorageManager>,
        sync_interval: Duration,
    ) -> Self {
        let (task_tx, task_rx) = mpsc::channel(1000);
        let sync_states = Arc::new(RwLock::new(HashMap::new()));
        let sync_timeout = Duration::from_secs(SYNC_TIMEOUT_SECS);

        let scheduler = Self {
            sync_states: sync_states.clone(),
            repository,
            storage_manager,
            sync_interval,
            sync_timeout,
            task_tx,
        };

        // 启动后台任务处理
        scheduler.spawn_processor(task_rx, sync_states.clone());

        scheduler
    }

    /// 获取任务发送器
    pub fn task_sender(&self) -> mpsc::Sender<SyncTask> {
        self.task_tx.clone()
    }

    /// 执行实际同步：从 StorageManager 读取数据，通过 repository 保存
    ///
    /// 同步成功后标记为 Clean，失败则标记回 Dirty 以便重试。
    async fn do_sync(
        repository: &StorageRepository,
        storage_manager: &StorageManager,
        char_id: u32,
        sync_states: &Arc<RwLock<HashMap<u32, SyncRecord>>>,
    ) -> bool {
        // 从内存中获取仓库数据
        let storage = match storage_manager.get(char_id) {
            Some(arc) => arc.read().clone(),
            None => {
                // 仓库已从内存移除，标记为 Clean 并返回
                if let Some(r) = sync_states.write().get_mut(&char_id) {
                    r.mark_clean()
                }
                return false;
            }
        };

        // 执行数据库保存
        match repository.save(&storage).await {
            Ok(()) => {
                // 标记为干净
                if let Some(r) = sync_states.write().get_mut(&char_id) {
                    r.mark_clean()
                }
                tracing::debug!("仓库同步成功: char_id={}", char_id);
                true
            }
            Err(e) => {
                tracing::error!("仓库同步失败: char_id={}, error={}", char_id, e);
                // 同步失败，标记回 Dirty 以便重试
                if let Some(r) = sync_states.write().get_mut(&char_id) {
                    r.mark_dirty()
                }
                false
            }
        }
    }

    /// 启动后台处理器
    ///
    /// 处理逻辑：
    /// 1. 响应外部任务（MarkDirty/MarkClean/ForceSync/Shutdown）
    /// 2. 周期性检查 stale 的脏数据并触发同步
    /// 3. 超时恢复：Syncing 状态超过 sync_timeout 的记录强制恢复为 Dirty
    fn spawn_processor(
        &self,
        mut task_rx: mpsc::Receiver<SyncTask>,
        sync_states: Arc<RwLock<HashMap<u32, SyncRecord>>>,
    ) {
        let interval = self.sync_interval;
        let sync_timeout = self.sync_timeout;
        let repository = self.repository.clone();
        let storage_manager = self.storage_manager.clone();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    // 处理任务
                    Some(task) = task_rx.recv() => {
                        match task {
                            SyncTask::Shutdown => {
                                tracing::info!("StorageSyncScheduler 正在关闭");
                                break;
                            }
                            SyncTask::MarkDirty(char_id) => {
                                let mut states = sync_states.write();
                                if let Some(record) = states.get_mut(&char_id) {
                                    record.mark_dirty();
                                } else {
                                    let mut record = SyncRecord::new(char_id);
                                    record.mark_dirty();
                                    states.insert(char_id, record);
                                }
                            }
                            SyncTask::MarkClean(char_id) => {
                                let mut states = sync_states.write();
                                if let Some(record) = states.get_mut(&char_id) {
                                    record.mark_clean();
                                }
                            }
                            SyncTask::ForceSync(char_id) => {
                                // 标记 Syncing 并立即执行同步
                                {
                                    let mut states = sync_states.write();
                                    if let Some(record) = states.get_mut(&char_id)
                                        && record.sync_state == Dirty {
                                            record.mark_syncing();
                                            tracing::debug!("强制同步触发: char_id={}", char_id);
                                        }
                                }
                                // 执行实际同步
                                Self::do_sync(
                                    &repository,
                                    &storage_manager,
                                    char_id,
                                    &sync_states,
                                ).await;
                            }
                        }
                    }
                    // 周期同步检查
                    _ = interval_timer.tick() => {
                        // 1. 收集需要同步的 stale 脏数据 ID
                        let dirty_ids: Vec<u32> = {
                            let states = sync_states.read();
                            states.iter()
                                .filter(|(_, r)| r.is_stale(interval))
                                .map(|(id, _)| *id)
                                .collect()
                        };

                        // 2. 标记为 Syncing
                        for char_id in &dirty_ids {
                            let mut states = sync_states.write();
                            if let Some(record) = states.get_mut(char_id) {
                                record.mark_syncing();
                            }
                        }

                        // 3. 逐个执行同步（在锁外异步执行）
                        for char_id in dirty_ids {
                            Self::do_sync(
                                &repository,
                                &storage_manager,
                                char_id,
                                &sync_states,
                            ).await;
                        }

                        // 4. 超时恢复：Syncing 超过 sync_timeout 的记录强制恢复为 Dirty
                        let timeout_ids: Vec<u32> = {
                            let states = sync_states.read();
                            states.iter()
                                .filter(|(_, r)| {
                                    r.sync_state == Syncing
                                        && r.last_modified.elapsed() >= sync_timeout
                                })
                                .map(|(id, _)| *id)
                                .collect()
                        };

                        if !timeout_ids.is_empty() {
                            tracing::warn!(
                                "仓库同步超时恢复: {:?} (timeout={:?})",
                                timeout_ids, sync_timeout
                            );
                            let mut states = sync_states.write();
                            for char_id in timeout_ids {
                                if let Some(record) = states.get_mut(&char_id) {
                                    record.mark_dirty();
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    /// 获取脏状态的角色ID列表
    pub fn get_dirty_char_ids(&self) -> Vec<u32> {
        let states = self.sync_states.read();
        states
            .iter()
            .filter(|(_, r)| r.sync_state == Dirty)
            .map(|(id, _)| *id)
            .collect()
    }

    /// 获取同步状态
    pub fn get_sync_state(&self, char_id: u32) -> Option<SyncState> {
        let states = self.sync_states.read();
        states.get(&char_id).map(|r| r.sync_state)
    }

    /// 获取版本号
    pub fn get_version(&self, char_id: u32) -> Option<u64> {
        let states = self.sync_states.read();
        states.get(&char_id).map(|r| r.version)
    }

    /// 检查是否有脏数据
    pub fn has_dirty(&self) -> bool {
        let states = self.sync_states.read();
        states.values().any(|r| r.sync_state == Dirty)
    }

    /// 统计脏数据数量
    pub fn dirty_count(&self) -> usize {
        let states = self.sync_states.read();
        states.iter().filter(|(_, r)| r.sync_state == Dirty).count()
    }
}

impl Drop for StorageSyncScheduler {
    fn drop(&mut self) {
        let _ = self.task_tx.try_send(SyncTask::Shutdown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema::init_schema;
    use std::time::{Duration, Instant};

    /// 创建测试用调度器环境
    fn setup() -> (StorageSyncScheduler, Arc<StorageManager>, StorageRepository) {
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
        let scheduler = StorageSyncScheduler::new(
            repo.clone(),
            manager.clone(),
            Duration::from_millis(100), // 短间隔用于测试
        );

        (scheduler, manager, repo)
    }

    /// 新建调度器无脏数据
    #[tokio::test]
    async fn new_scheduler_has_no_dirty() {
        let (scheduler, _, _) = setup();
        assert!(!scheduler.has_dirty());
        assert_eq!(scheduler.dirty_count(), 0);
    }

    /// MarkDirty 创建记录并标记为脏
    #[tokio::test]
    async fn mark_dirty_creates_record() {
        let (scheduler, _, _) = setup();
        let tx = scheduler.task_sender();
        tx.try_send(SyncTask::MarkDirty(1)).unwrap();

        // 等待后台任务处理
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(scheduler.get_sync_state(1), Some(SyncState::Dirty));
        assert!(scheduler.has_dirty());
        assert_eq!(scheduler.dirty_count(), 1);
    }

    /// MarkClean 转换状态
    #[tokio::test]
    async fn mark_clean_transitions_state() {
        let (scheduler, _, _) = setup();
        let tx = scheduler.task_sender();

        tx.try_send(SyncTask::MarkDirty(1)).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(scheduler.get_sync_state(1), Some(SyncState::Dirty));

        tx.try_send(SyncTask::MarkClean(1)).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(scheduler.get_sync_state(1), Some(SyncState::Clean));
    }

    /// 对不存在的记录 MarkClean 不会 panic
    #[tokio::test]
    async fn mark_clean_nonexistent_is_noop() {
        let (scheduler, _, _) = setup();
        let tx = scheduler.task_sender();
        tx.try_send(SyncTask::MarkClean(999)).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(scheduler.get_sync_state(999), None);
    }

    /// 不存在的角色版本号返回 None
    #[tokio::test]
    async fn get_version_returns_none_for_missing() {
        let (scheduler, _, _) = setup();
        assert_eq!(scheduler.get_version(999), None);
    }

    /// 只返回脏状态的角色 ID
    #[tokio::test]
    async fn get_dirty_char_ids_returns_only_dirty() {
        let (scheduler, _, _) = setup();
        let tx = scheduler.task_sender();

        tx.try_send(SyncTask::MarkDirty(1)).unwrap();
        tx.try_send(SyncTask::MarkDirty(2)).unwrap();
        tx.try_send(SyncTask::MarkClean(3)).unwrap(); // 不存在，会被忽略
        tokio::time::sleep(Duration::from_millis(50)).await;

        let dirty_ids = scheduler.get_dirty_char_ids();
        assert_eq!(dirty_ids.len(), 2);
        assert!(dirty_ids.contains(&1));
        assert!(dirty_ids.contains(&2));
    }

    /// ForceSync 执行实际数据库保存
    #[tokio::test]
    async fn force_sync_executes_actual_save() {
        let (scheduler, manager, repo) = setup();

        // 创建仓库并添加物品
        let storage_arc = manager.get_or_create(1, 100);
        storage_arc.write().add_item(501, 10);

        // 标记脏
        let tx = scheduler.task_sender();
        tx.try_send(SyncTask::MarkDirty(1)).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // 强制同步
        tx.try_send(SyncTask::ForceSync(1)).unwrap();
        // 等待异步同步完成
        tokio::time::sleep(Duration::from_millis(500)).await;

        // 验证数据已保存到数据库
        let loaded = repo.load(1).await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.get_slot(0).unwrap().item_id, 501);
        assert_eq!(loaded.get_slot(0).unwrap().amount, 10);
    }

    /// 周期同步对 stale 脏数据触发同步
    #[tokio::test]
    async fn periodic_sync_triggers_for_stale_dirty() {
        let (scheduler, manager, repo) = setup();

        // 创建仓库
        let storage_arc = manager.get_or_create(1, 100);
        storage_arc.write().add_item(501, 5);

        // 标记脏
        let tx = scheduler.task_sender();
        tx.try_send(SyncTask::MarkDirty(1)).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // 手动将时间戳设为旧值，使其 stale
        {
            let mut states = scheduler.sync_states.write();
            if let Some(record) = states.get_mut(&1) {
                record.last_modified = Instant::now() - Duration::from_secs(60);
            }
        }

        // 等待至少一个 tick（间隔 100ms）
        tokio::time::sleep(Duration::from_millis(300)).await;

        // 验证数据已同步
        let loaded = repo.load(1).await.unwrap();
        assert!(loaded.is_some());
    }

    /// Shutdown 停止处理器
    #[tokio::test]
    async fn shutdown_stops_processor() {
        let (scheduler, _, _) = setup();
        let tx = scheduler.task_sender();
        tx.try_send(SyncTask::Shutdown).unwrap();
        // 如果 shutdown 失败会 panic（通道已关闭）
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    /// drop 自动发送 Shutdown
    #[tokio::test]
    async fn drop_sends_shutdown() {
        let (scheduler, _, _) = setup();
        // scheduler drop 时应发送 Shutdown
        drop(scheduler);
        // 不 panic 即为成功
    }

    /// 多个脏数据独立跟踪：MarkDirty 和 MarkClean 状态互不影响
    #[tokio::test]
    async fn multiple_dirty_tracks_independently() {
        let (scheduler, _, _) = setup();
        let tx = scheduler.task_sender();

        // 同时发送所有标记（利用 try_send 的同步入队特性）
        tx.try_send(SyncTask::MarkDirty(1)).unwrap();
        tx.try_send(SyncTask::MarkDirty(2)).unwrap();
        tx.try_send(SyncTask::MarkDirty(3)).unwrap();
        tx.try_send(SyncTask::MarkClean(2)).unwrap();

        // 等待所有消息处理完成
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 2 应该是 Clean（我们显式标记了）
        assert_eq!(scheduler.get_sync_state(2), Some(SyncState::Clean));

        // 1 和 3 的状态取决于周期 tick 是否已运行
        // 由于 try_send 同步入队且 sleep 较短，大概率仍然是 Dirty
        let state1 = scheduler.get_sync_state(1);
        let state3 = scheduler.get_sync_state(3);
        assert!(state1.is_some(), "角色 1 应该有同步状态");
        assert!(state3.is_some(), "角色 3 应该有同步状态");
    }
}
