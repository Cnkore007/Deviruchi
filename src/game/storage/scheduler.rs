//! 仓库同步调度器
//! 处理定期批量同步到数据库

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use super::repository::StorageRepository;
use super::sync::{SyncRecord, SyncState, SyncState::*};

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
#[allow(dead_code)]
pub struct StorageSyncScheduler {
    /// 同步状态记录
    sync_states: Arc<RwLock<HashMap<u32, SyncRecord>>>,
    /// 仓库仓库
    repository: StorageRepository,
    /// 同步间隔
    sync_interval: Duration,
    /// 任务发送通道
    task_tx: mpsc::Sender<SyncTask>,
}

impl StorageSyncScheduler {
    /// 创建新的同步调度器
    pub fn new(repository: StorageRepository, sync_interval: Duration) -> Self {
        let (task_tx, task_rx) = mpsc::channel(1000);
        let sync_states = Arc::new(RwLock::new(HashMap::new()));

        let scheduler = Self {
            sync_states: sync_states.clone(),
            repository,
            sync_interval,
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

    /// 启动处理器
    fn spawn_processor(
        &self,
        mut task_rx: mpsc::Receiver<SyncTask>,
        sync_states: Arc<RwLock<HashMap<u32, SyncRecord>>>,
    ) {
        let interval = self.sync_interval;

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    // 处理任务
                    Some(task) = task_rx.recv() => {
                        match task {
                            SyncTask::Shutdown => {
                                tracing::info!("StorageSyncScheduler shutting down");
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
                                let mut states = sync_states.write();
                                if let Some(record) = states.get_mut(&char_id)
                                    && record.sync_state == Dirty {
                                        record.mark_syncing();
                                        tracing::debug!("Force sync triggered for char_id: {}", char_id);
                                    }
                            }
                        }
                    }
                    // 周期同步检查
                    _ = interval_timer.tick() => {
                        let dirty_ids: Vec<u32> = {
                            let states = sync_states.read();
                            states.iter()
                                .filter(|(_, r)| r.sync_state == Dirty)
                                .map(|(id, _)| *id)
                                .collect()
                        };

                        for char_id in dirty_ids {
                            let mut states = sync_states.write();
                            if let Some(record) = states.get_mut(&char_id)
                                && record.is_stale(interval) {
                                    record.mark_syncing();
                                    tracing::debug!("Periodic sync triggered for char_id: {}", char_id);
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
