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

    /// 尝试状态转换，返回是否成功
    ///
    /// 合法的状态转换：
    /// - Dirty -> Syncing: 开始同步
    /// - Dirty -> Dirty: 重复标记（刷新时间戳）
    /// - Syncing -> Clean: 同步完成
    /// - Syncing -> Dirty: 同步期间有新修改（重新标记脏）
    /// - Clean -> Dirty: 有新修改
    ///
    /// 非法的状态转换（会被拒绝）：
    /// - Clean -> Syncing: 干净数据不需要同步
    /// - Clean -> Clean: 无意义操作
    pub fn try_transition(&mut self, target: SyncState) -> bool {
        match (&self.sync_state, target) {
            // 合法转换
            (SyncState::Dirty, SyncState::Syncing) => {
                self.sync_state = SyncState::Syncing;
                true
            }
            (SyncState::Dirty, SyncState::Dirty) => {
                // 重复标记脏，刷新时间戳
                self.last_modified = Instant::now();
                self.version += 1;
                true
            }
            (SyncState::Syncing, SyncState::Clean) => {
                self.sync_state = SyncState::Clean;
                true
            }
            (SyncState::Syncing, SyncState::Dirty) => {
                // 同步期间有新修改
                self.sync_state = SyncState::Dirty;
                self.last_modified = Instant::now();
                self.version += 1;
                true
            }
            (SyncState::Clean, SyncState::Dirty) => {
                self.sync_state = SyncState::Dirty;
                self.last_modified = Instant::now();
                self.version += 1;
                true
            }
            // 非法转换：拒绝
            _ => {
                tracing::warn!(
                    "非法状态转换: {:?} -> {:?} (char_id: {})",
                    self.sync_state,
                    target,
                    self.char_id
                );
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ========== SyncState 测试 ==========

    /// is_dirty 正确识别脏状态
    #[test]
    fn sync_state_is_dirty() {
        assert!(SyncState::Dirty.is_dirty());
        assert!(!SyncState::Clean.is_dirty());
        assert!(!SyncState::Syncing.is_dirty());
    }

    /// is_syncing 正确识别同步中状态
    #[test]
    fn sync_state_is_syncing() {
        assert!(SyncState::Syncing.is_syncing());
        assert!(!SyncState::Clean.is_syncing());
        assert!(!SyncState::Dirty.is_syncing());
    }

    /// 状态相等性比较
    #[test]
    fn sync_state_equality() {
        assert_eq!(SyncState::Clean, SyncState::Clean);
        assert_ne!(SyncState::Clean, SyncState::Dirty);
        assert_ne!(SyncState::Dirty, SyncState::Syncing);
    }

    // ========== SyncRecord 基础测试 ==========

    /// 新建记录初始为脏状态
    #[test]
    fn new_record_starts_as_dirty() {
        let record = SyncRecord::new(42);
        assert_eq!(record.char_id, 42);
        assert_eq!(record.sync_state, SyncState::Dirty);
        assert_eq!(record.version, 1);
    }

    /// mark_dirty 递增版本号
    #[test]
    fn mark_dirty_increments_version() {
        let mut record = SyncRecord::new(1);
        assert_eq!(record.version, 1);
        record.mark_dirty();
        assert_eq!(record.version, 2);
        assert_eq!(record.sync_state, SyncState::Dirty);
    }

    /// Syncing 状态下 mark_dirty 被忽略
    #[test]
    fn mark_dirty_while_syncing_is_ignored() {
        let mut record = SyncRecord::new(1);
        record.mark_syncing();
        record.mark_dirty(); // 应该被忽略
        assert_eq!(record.sync_state, SyncState::Syncing);
        assert_eq!(record.version, 1); // 版本不应增加
    }

    /// mark_syncing 转换到同步中状态
    #[test]
    fn mark_syncing_transitions_to_syncing() {
        let mut record = SyncRecord::new(1);
        record.mark_dirty();
        record.mark_syncing();
        assert_eq!(record.sync_state, SyncState::Syncing);
    }

    /// mark_clean 从 Syncing 转换到 Clean
    #[test]
    fn mark_clean_transitions_to_clean() {
        let mut record = SyncRecord::new(1);
        record.mark_syncing();
        record.mark_clean();
        assert_eq!(record.sync_state, SyncState::Clean);
    }

    /// mark_clean 从 Dirty 也可转换到 Clean
    #[test]
    fn mark_clean_from_dirty() {
        let mut record = SyncRecord::new(1);
        record.mark_clean();
        assert_eq!(record.sync_state, SyncState::Clean);
    }

    /// 新建的脏记录不是 stale
    #[test]
    fn is_stale_returns_false_for_fresh_dirty() {
        let record = SyncRecord::new(1);
        assert!(!record.is_stale(Duration::from_secs(60)));
    }

    /// 超时的脏记录是 stale
    #[test]
    fn is_stale_returns_true_for_old_dirty() {
        let mut record = SyncRecord::new(1);
        // 模拟旧时间戳
        record.last_modified = Instant::now() - Duration::from_secs(120);
        assert!(record.is_stale(Duration::from_secs(60)));
    }

    /// Clean 状态不会 stale
    #[test]
    fn is_stale_returns_false_for_clean() {
        let mut record = SyncRecord::new(1);
        record.mark_clean();
        record.last_modified = Instant::now() - Duration::from_secs(120);
        assert!(!record.is_stale(Duration::from_secs(60)));
    }

    /// Syncing 状态不会 stale
    #[test]
    fn is_stale_returns_false_for_syncing() {
        let mut record = SyncRecord::new(1);
        record.mark_syncing();
        record.last_modified = Instant::now() - Duration::from_secs(120);
        assert!(!record.is_stale(Duration::from_secs(60)));
    }

    // ========== try_transition 测试 ==========

    /// Dirty -> Syncing 合法转换
    #[test]
    fn transition_dirty_to_syncing() {
        let mut record = SyncRecord::new(1);
        assert!(record.try_transition(SyncState::Syncing));
        assert_eq!(record.sync_state, SyncState::Syncing);
    }

    /// Dirty -> Dirty 刷新版本号
    #[test]
    fn transition_dirty_to_dirty_refreshes() {
        let mut record = SyncRecord::new(1);
        let old_version = record.version;
        assert!(record.try_transition(SyncState::Dirty));
        assert_eq!(record.version, old_version + 1);
    }

    /// Syncing -> Clean 合法转换
    #[test]
    fn transition_syncing_to_clean() {
        let mut record = SyncRecord::new(1);
        record.mark_syncing();
        assert!(record.try_transition(SyncState::Clean));
        assert_eq!(record.sync_state, SyncState::Clean);
    }

    /// Syncing -> Dirty（同步期间新修改）
    #[test]
    fn transition_syncing_to_dirty() {
        let mut record = SyncRecord::new(1);
        record.mark_syncing();
        assert!(record.try_transition(SyncState::Dirty));
        assert_eq!(record.sync_state, SyncState::Dirty);
        assert_eq!(record.version, 2);
    }

    /// Clean -> Dirty 合法转换
    #[test]
    fn transition_clean_to_dirty() {
        let mut record = SyncRecord::new(1);
        record.mark_clean();
        assert!(record.try_transition(SyncState::Dirty));
        assert_eq!(record.sync_state, SyncState::Dirty);
        assert_eq!(record.version, 2);
    }

    /// Clean -> Syncing 非法转换被拒绝
    #[test]
    fn transition_clean_to_syncing_is_rejected() {
        let mut record = SyncRecord::new(1);
        record.mark_clean();
        assert!(!record.try_transition(SyncState::Syncing));
        assert_eq!(record.sync_state, SyncState::Clean); // 状态不变
    }

    /// Clean -> Clean 非法转换被拒绝
    #[test]
    fn transition_clean_to_clean_is_rejected() {
        let mut record = SyncRecord::new(1);
        record.mark_clean();
        assert!(!record.try_transition(SyncState::Clean));
    }

    // ========== 典型生命周期测试 ==========

    /// 完整的生命周期: Dirty -> Syncing -> Clean -> Dirty -> Syncing
    #[test]
    fn typical_lifecycle() {
        let mut record = SyncRecord::new(1);
        // 初始: Dirty (v1)
        assert_eq!(record.sync_state, SyncState::Dirty);
        assert_eq!(record.version, 1);

        // 开始同步: Dirty -> Syncing
        record.mark_syncing();
        assert_eq!(record.sync_state, SyncState::Syncing);

        // 同步完成: Syncing -> Clean
        record.mark_clean();
        assert_eq!(record.sync_state, SyncState::Clean);

        // 新修改: Clean -> Dirty (v2)
        record.mark_dirty();
        assert_eq!(record.sync_state, SyncState::Dirty);
        assert_eq!(record.version, 2);

        // 再次修改: Dirty -> Dirty (v3)
        record.mark_dirty();
        assert_eq!(record.version, 3);

        // 开始同步: Dirty -> Syncing
        record.mark_syncing();

        // 同步期间 mark_dirty 被忽略（mark_dirty 实现限制）
        record.mark_dirty();
        assert_eq!(record.sync_state, SyncState::Syncing);
        assert_eq!(record.version, 3);

        // 但 try_transition 允许 Syncing -> Dirty
        assert!(record.try_transition(SyncState::Dirty));
        assert_eq!(record.version, 4);
    }
}
