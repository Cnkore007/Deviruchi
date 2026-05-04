//! 物品冷却系统
//!
//! 追踪和管理物品使用的冷却时间

use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// 物品冷却信息
#[derive(Debug, Clone)]
pub struct ItemDelay {
    /// 物品ID
    pub item_id: u16,
    /// 冷却结束时间
    pub ready_time: Instant,
    /// 冷却时长（毫秒）
    pub delay_ms: u64,
}

impl ItemDelay {
    /// 检查冷却是否结束
    pub fn is_ready(&self) -> bool {
        Instant::now() >= self.ready_time
    }

    /// 获取剩余冷却时间（毫秒）
    pub fn remaining_ms(&self) -> u64 {
        if self.is_ready() {
            0
        } else {
            self.ready_time.duration_since(Instant::now()).as_millis() as u64
        }
    }
}

/// 物品冷却追踪器
///
/// 使用玩家ID + 物品ID 作为复合键来追踪冷却状态
pub struct ItemDelayTracker {
    /// 冷却数据: (player_id, item_id) -> ready_time
    delays: RwLock<HashMap<(Uuid, u16), Instant>>,
    /// 默认冷却时间（毫秒）
    default_delay_ms: u64,
}

impl ItemDelayTracker {
    /// 创建新的冷却追踪器
    pub fn new() -> Self {
        Self {
            delays: RwLock::new(HashMap::new()),
            default_delay_ms: 1000, // 默认1秒冷却
        }
    }

    /// 创建带有默认冷却时间的追踪器
    pub fn with_default_delay(default_delay_ms: u64) -> Self {
        Self {
            delays: RwLock::new(HashMap::new()),
            default_delay_ms,
        }
    }

    /// 检查物品是否在冷却中
    pub fn is_on_cooldown(&self, player_id: Uuid, item_id: u16) -> bool {
        let delays = self.delays.read();
        if let Some(ready_time) = delays.get(&(player_id, item_id)) {
            Instant::now() < *ready_time
        } else {
            false
        }
    }

    /// 获取物品剩余冷却时间（毫秒）
    pub fn remaining_cooldown(&self, player_id: Uuid, item_id: u16) -> u64 {
        let delays = self.delays.read();
        if let Some(ready_time) = delays.get(&(player_id, item_id)) {
            if Instant::now() >= *ready_time {
                0
            } else {
                ready_time.duration_since(Instant::now()).as_millis() as u64
            }
        } else {
            0
        }
    }

    /// 开始物品冷却
    pub fn start_cooldown(&self, player_id: Uuid, item_id: u16) {
        self.start_cooldown_with_duration(player_id, item_id, self.default_delay_ms);
    }

    /// 开始物品冷却（指定时长）
    pub fn start_cooldown_with_duration(&self, player_id: Uuid, item_id: u16, delay_ms: u64) {
        let ready_time = Instant::now() + Duration::from_millis(delay_ms);
        let mut delays = self.delays.write();
        delays.insert((player_id, item_id), ready_time);
    }

    /// 清除物品冷却
    pub fn clear_cooldown(&self, player_id: Uuid, item_id: u16) {
        let mut delays = self.delays.write();
        delays.remove(&(player_id, item_id));
    }

    /// 清除玩家所有冷却
    pub fn clear_player_cooldowns(&self, player_id: Uuid) {
        let mut delays = self.delays.write();
        delays.retain(|(pid, _), _| *pid != player_id);
    }

    /// 清除所有冷却
    pub fn clear_all(&self) {
        let mut delays = self.delays.write();
        delays.clear();
    }

    /// 获取玩家所有在冷却中的物品
    pub fn get_active_cooldowns(&self, player_id: Uuid) -> Vec<ItemDelay> {
        let now = Instant::now();
        let delays = self.delays.read();

        delays
            .iter()
            .filter(|((pid, _), _)| *pid == player_id)
            .filter(|(_, ready_time)| **ready_time > now)
            .map(|((_, item_id), ready_time)| {
                let delay_ms = ready_time.duration_since(now).as_millis() as u64;
                ItemDelay {
                    item_id: *item_id,
                    ready_time: *ready_time,
                    delay_ms,
                }
            })
            .collect()
    }

    /// 清理过期的冷却记录（可选的维护操作）
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        let mut delays = self.delays.write();
        delays.retain(|_, ready_time| *ready_time > now);
    }

    /// 设置默认冷却时间
    pub fn set_default_delay(&mut self, delay_ms: u64) {
        self.default_delay_ms = delay_ms;
    }

    /// 获取默认冷却时间
    pub fn get_default_delay(&self) -> u64 {
        self.default_delay_ms
    }

    /// 检查多个物品是否都在冷却中
    pub fn are_all_on_cooldown(&self, player_id: Uuid, item_ids: &[u16]) -> bool {
        item_ids
            .iter()
            .all(|id| self.is_on_cooldown(player_id, *id))
    }

    /// 获取冷却中的物品数量
    pub fn count_active_cooldowns(&self, player_id: Uuid) -> usize {
        let now = Instant::now();
        let delays = self.delays.read();
        delays
            .iter()
            .filter(|((pid, _), ready_time)| *pid == player_id && **ready_time > now)
            .count()
    }
}

impl Default for ItemDelayTracker {
    fn default() -> Self {
        Self::new()
    }
}

use parking_lot::Mutex;
/// 全局物品冷却管理器（用于单例模式）
use std::sync::Arc;

/// 全局物品冷却管理器
pub struct GlobalItemDelayManager {
    tracker: Arc<Mutex<ItemDelayTracker>>,
}

impl GlobalItemDelayManager {
    /// 创建新的全局管理器
    pub fn new() -> Self {
        Self {
            tracker: Arc::new(Mutex::new(ItemDelayTracker::new())),
        }
    }

    /// 获取追踪器
    pub fn tracker(&self) -> Arc<Mutex<ItemDelayTracker>> {
        self.tracker.clone()
    }

    /// 检查物品是否在冷却中
    pub fn is_on_cooldown(&self, player_id: Uuid, item_id: u16) -> bool {
        self.tracker.lock().is_on_cooldown(player_id, item_id)
    }

    /// 获取剩余冷却时间
    pub fn remaining_cooldown(&self, player_id: Uuid, item_id: u16) -> u64 {
        self.tracker.lock().remaining_cooldown(player_id, item_id)
    }

    /// 开始冷却
    pub fn start_cooldown(&self, player_id: Uuid, item_id: u16) {
        self.tracker.lock().start_cooldown(player_id, item_id);
    }

    /// 开始冷却（指定时长）
    pub fn start_cooldown_with_duration(&self, player_id: Uuid, item_id: u16, delay_ms: u64) {
        self.tracker
            .lock()
            .start_cooldown_with_duration(player_id, item_id, delay_ms);
    }

    /// 清除冷却
    pub fn clear_cooldown(&self, player_id: Uuid, item_id: u16) {
        self.tracker.lock().clear_cooldown(player_id, item_id);
    }

    /// 清除玩家所有冷却
    pub fn clear_player_cooldowns(&self, player_id: Uuid) {
        self.tracker.lock().clear_player_cooldowns(player_id);
    }
}

impl Default for GlobalItemDelayManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cooldown_check() {
        let tracker = ItemDelayTracker::new();
        let player_id = Uuid::new_v4();
        let item_id = 501;

        // 初始应该不在冷却
        assert!(!tracker.is_on_cooldown(player_id, item_id));
    }

    #[test]
    fn test_start_cooldown() {
        let tracker = ItemDelayTracker::new();
        let player_id = Uuid::new_v4();
        let item_id = 501;

        // 开始冷却
        tracker.start_cooldown(player_id, item_id);

        // 应该在冷却中
        assert!(tracker.is_on_cooldown(player_id, item_id));
    }

    #[test]
    fn test_clear_cooldown() {
        let tracker = ItemDelayTracker::new();
        let player_id = Uuid::new_v4();
        let item_id = 501;

        // 开始冷却
        tracker.start_cooldown(player_id, item_id);
        assert!(tracker.is_on_cooldown(player_id, item_id));

        // 清除冷却
        tracker.clear_cooldown(player_id, item_id);
        assert!(!tracker.is_on_cooldown(player_id, item_id));
    }

    #[test]
    fn test_cooldown_with_duration() {
        let tracker = ItemDelayTracker::new();
        let player_id = Uuid::new_v4();
        let item_id = 501;

        // 开始1毫秒冷却
        tracker.start_cooldown_with_duration(player_id, item_id, 1);

        // 立即检查应该在冷却中
        assert!(tracker.is_on_cooldown(player_id, item_id));

        // 等待后检查
        std::thread::sleep(Duration::from_millis(5));

        // 应该不在冷却中
        assert!(!tracker.is_on_cooldown(player_id, item_id));
    }

    #[test]
    fn test_multiple_players() {
        let tracker = ItemDelayTracker::new();
        let player1 = Uuid::new_v4();
        let player2 = Uuid::new_v4();
        let item_id = 501;

        // 玩家1开始冷却
        tracker.start_cooldown(player1, item_id);

        // 玩家1在冷却中，玩家2不在
        assert!(tracker.is_on_cooldown(player1, item_id));
        assert!(!tracker.is_on_cooldown(player2, item_id));
    }

    #[test]
    fn test_multiple_items() {
        let tracker = ItemDelayTracker::new();
        let player_id = Uuid::new_v4();

        // 开始两个物品的冷却
        tracker.start_cooldown(player_id, 501);
        tracker.start_cooldown(player_id, 502);

        // 两个都在冷却中
        assert!(tracker.is_on_cooldown(player_id, 501));
        assert!(tracker.is_on_cooldown(player_id, 502));

        // 清除一个
        tracker.clear_cooldown(player_id, 501);

        // 501不在冷却，502还在
        assert!(!tracker.is_on_cooldown(player_id, 501));
        assert!(tracker.is_on_cooldown(player_id, 502));
    }

    #[test]
    fn test_remaining_cooldown() {
        let tracker = ItemDelayTracker::new();
        let player_id = Uuid::new_v4();
        let item_id = 501;

        // 开始100毫秒冷却
        tracker.start_cooldown_with_duration(player_id, item_id, 100);

        // 获取剩余时间
        let remaining = tracker.remaining_cooldown(player_id, item_id);

        // 剩余时间应该在 0-100 之间
        assert!(remaining > 0 && remaining <= 100);
    }

    #[test]
    fn test_get_active_cooldowns() {
        let tracker = ItemDelayTracker::new();
        let player_id = Uuid::new_v4();

        tracker.start_cooldown(player_id, 501);
        tracker.start_cooldown(player_id, 502);

        let active = tracker.get_active_cooldowns(player_id);
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn test_clear_player_cooldowns() {
        let tracker = ItemDelayTracker::new();
        let player1 = Uuid::new_v4();
        let player2 = Uuid::new_v4();

        tracker.start_cooldown(player1, 501);
        tracker.start_cooldown(player1, 502);
        tracker.start_cooldown(player2, 501);

        // 清除玩家1的所有冷却
        tracker.clear_player_cooldowns(player1);

        // 玩家1的冷却应该都清除了
        assert!(!tracker.is_on_cooldown(player1, 501));
        assert!(!tracker.is_on_cooldown(player1, 502));

        // 玩家2的冷却应该还在
        assert!(tracker.is_on_cooldown(player2, 501));
    }

    #[test]
    fn test_count_active_cooldowns() {
        let tracker = ItemDelayTracker::new();
        let player_id = Uuid::new_v4();

        assert_eq!(tracker.count_active_cooldowns(player_id), 0);

        tracker.start_cooldown(player_id, 501);
        assert_eq!(tracker.count_active_cooldowns(player_id), 1);

        tracker.start_cooldown(player_id, 502);
        assert_eq!(tracker.count_active_cooldowns(player_id), 2);

        tracker.clear_cooldown(player_id, 501);
        assert_eq!(tracker.count_active_cooldowns(player_id), 1);
    }
}
