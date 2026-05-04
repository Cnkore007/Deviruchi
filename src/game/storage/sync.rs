//! 仓库同步状态和记录
use std::time::{Duration, Instant};

/// 仓库同步状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    /// 干净状态 - 已同步到数据库
    Clean,
    /// 脏状态 - 已修改待同步
    Dirty,
    /// 同步中状态
    Syncing,
}

impl SyncState {
    pub fn is_dirty(&self) -> bool {
        matches!(self, SyncState::Dirty)
    }

    pub fn is_syncing(&self) -> bool {
        matches!(self, SyncState::Syncing)
    }
}

/// 仓库同步记录
#[derive(Debug, Clone)]
pub struct SyncRecord {
    /// 角色ID
    pub char_id: u32,
    /// 同步状态
    pub sync_state: SyncState,
    /// 上次修改时间
    pub last_modified: Instant,
    /// 版本号 (递增)
    pub version: u64,
}

impl SyncRecord {
    pub fn new(char_id: u32) -> Self {
        Self {
            char_id,
            sync_state: SyncState::Dirty,
            last_modified: Instant::now(),
            version: 1,
        }
    }

    /// 标记为脏
    pub fn mark_dirty(&mut self) {
        if self.sync_state != SyncState::Syncing {
            self.sync_state = SyncState::Dirty;
            self.last_modified = Instant::now();
            self.version += 1;
        }
    }

    /// 标记为同步中
    pub fn mark_syncing(&mut self) {
        self.sync_state = SyncState::Syncing;
    }

    /// 标记为已同步
    pub fn mark_clean(&mut self) {
        self.sync_state = SyncState::Clean;
    }

    /// 检查是否超时需要同步
    pub fn is_stale(&self, interval: Duration) -> bool {
        self.sync_state == SyncState::Dirty && self.last_modified.elapsed() >= interval
    }
}
