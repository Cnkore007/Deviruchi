//! 玩家状态管理器

use super::effect::{StackingRule, StatusEffect};
use super::types::StatusChange;
use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

/// 玩家状态管理器
pub struct PlayerStatus {
    /// 玩家ID
    player_id: Uuid,
    /// 活跃状态效果
    effects: RwLock<HashMap<StatusChange, StatusEffect>>,
    /// 状态改变回调
    on_change: RwLock<Vec<Box<dyn StatusChangeCallback>>>,
}

impl Clone for PlayerStatus {
    fn clone(&self) -> Self {
        Self {
            player_id: self.player_id,
            effects: RwLock::new(self.effects.read().clone()),
            on_change: RwLock::new(Vec::new()),
        }
    }
}

/// 状态改变回调接口
pub trait StatusChangeCallback: Send + Sync {
    fn on_add(&self, player_id: Uuid, effect: &StatusEffect);
    fn on_remove(&self, player_id: Uuid, status: StatusChange);
    fn on_expire(&self, player_id: Uuid, status: StatusChange);
}

impl PlayerStatus {
    /// 创建新的玩家状态管理器
    pub fn new(player_id: Uuid) -> Self {
        Self {
            player_id,
            effects: RwLock::new(HashMap::new()),
            on_change: RwLock::new(Vec::new()),
        }
    }

    /// 添加状态效果
    pub fn add_status(&self, effect: StatusEffect) -> Option<StatusEffect> {
        let status = effect.id;
        let mut effects = self.effects.write();

        let mut old_effect = effects.get(&status).cloned();
        let is_new = old_effect.is_none();

        // 根据叠加规则处理
        match status.stacking_rule() {
            StackingRule::Replace => {
                // 直接替换
                effects.insert(status, effect);
            }
            StackingRule::Maximum => {
                // 取大模式
                if let Some(old) = &old_effect {
                    if effect.val1 > old.val1 {
                        effects.insert(status, effect);
                    }
                    // else: 保持原效果，不替换
                } else {
                    effects.insert(status, effect);
                }
            }
            StackingRule::Additive => {
                // 叠加模式
                if let Some(ref mut old) = old_effect {
                    old.val1 += effect.val1;
                    old.val2 += effect.val2;
                    old.val3 += effect.val3;
                    old.stack += 1;
                    effects.insert(status, old.clone());
                } else {
                    effects.insert(status, effect);
                }
            }
            StackingRule::Extend => {
                // 时间延长模式
                if let Some(ref mut old) = old_effect {
                    old.extend_duration(effect.duration_ms);
                    // 取较大值
                    old.val1 = old.val1.max(effect.val1);
                    old.val2 = old.val2.max(effect.val2);
                    old.val3 = old.val3.max(effect.val3);
                    effects.insert(status, old.clone());
                } else {
                    effects.insert(status, effect);
                }
            }
        }

        drop(effects);

        // 触发回调
        if is_new {
            // 新增状态 - 需要重新获取效果
            if let Some(new_effect) = self.effects.read().get(&status) {
                self.notify_add(new_effect);
            }
        }

        old_effect
    }

    /// 移除状态效果
    pub fn remove_status(&self, status: StatusChange) -> Option<StatusEffect> {
        let mut effects = self.effects.write();
        let removed = effects.remove(&status);
        drop(effects);

        if let Some(_effect) = &removed {
            self.notify_remove(status);
        }

        removed
    }

    /// 检查是否有指定状态
    pub fn has_status(&self, status: StatusChange) -> bool {
        self.effects.read().contains_key(&status)
    }

    /// 获取状态效果
    pub fn get_status(&self, status: StatusChange) -> Option<StatusEffect> {
        self.effects.read().get(&status).cloned()
    }

    /// 获取所有活跃状态
    pub fn get_all_statuses(&self) -> Vec<StatusEffect> {
        self.effects.read().values().cloned().collect()
    }

    /// 获取所有活跃状态类型
    pub fn get_active_status_types(&self) -> Vec<StatusChange> {
        self.effects.read().keys().copied().collect()
    }

    /// 清除所有状态
    pub fn clear_all(&self) {
        let statuses: Vec<StatusChange> = self.effects.read().keys().copied().collect();
        self.effects.write().clear();

        for status in statuses {
            self.notify_remove(status);
        }
    }

    /// 清除所有指定分类的状态
    pub fn clear_by_category(&self, category: super::types::StatusCategory) {
        let statuses: Vec<StatusChange> = self
            .effects
            .read()
            .iter()
            .filter(|(_, e)| e.id.category() == category)
            .map(|(s, _)| *s)
            .collect();

        for status in statuses {
            self.remove_status(status);
        }
    }

    /// 更新状态效果（用于修改效果值）
    pub fn update_status(&self, status: StatusChange, val1: i32, val2: i32, val3: i32) -> bool {
        let mut effects = self.effects.write();
        if let Some(effect) = effects.get_mut(&status) {
            effect.set_values(val1, val2, val3);
            true
        } else {
            false
        }
    }

    /// 刷新状态效果
    pub fn refresh_status(&self, status: StatusChange) -> bool {
        let mut effects = self.effects.write();
        if let Some(effect) = effects.get_mut(&status) {
            effect.refresh();
            true
        } else {
            false
        }
    }

    /// 清除所有已过期的状态
    /// 返回被清除的状态列表
    pub fn cleanup_expired(&self) -> Vec<StatusChange> {
        let mut effects = self.effects.write();
        let expired: Vec<StatusChange> = effects
            .iter()
            .filter(|(_, e)| e.is_expired())
            .map(|(s, _)| *s)
            .collect();

        for status in &expired {
            effects.remove(status);
        }

        drop(effects);

        // 触发过期回调
        for status in &expired {
            self.notify_expire(*status);
        }

        expired
    }

    /// 获取状态数量
    pub fn count(&self) -> usize {
        self.effects.read().len()
    }

    /// 检查是否有任何战斗限制状态
    pub fn has_combat_restriction(&self) -> bool {
        let effects = self.effects.read();
        effects.contains_key(&StatusChange::Stun)
            || effects.contains_key(&StatusChange::Freeze)
            || effects.contains_key(&StatusChange::Sleep)
            || effects.contains_key(&StatusChange::Stone)
            || effects.contains_key(&StatusChange::Confusion)
    }

    /// 检查是否有沉默状态
    pub fn is_silenced(&self) -> bool {
        self.effects.read().contains_key(&StatusChange::Silence)
    }

    /// 检查是否无敌
    pub fn is_invincible(&self) -> bool {
        self.effects.read().contains_key(&StatusChange::Invincible)
    }

    /// 检查是否隐身
    pub fn is_invisible(&self) -> bool {
        let effects = self.effects.read();
        effects.contains_key(&StatusChange::Hide)
            || effects.contains_key(&StatusChange::Cloak)
            || effects.contains_key(&StatusChange::Invisible)
    }

    /// 获取玩家ID
    pub fn player_id(&self) -> Uuid {
        self.player_id
    }

    /// 注册状态改变回调
    pub fn register_callback<F>(&self, callback: F)
    where
        F: StatusChangeCallback + 'static,
    {
        self.on_change.write().push(Box::new(callback));
    }

    fn notify_add(&self, effect: &StatusEffect) {
        let callbacks = self.on_change.read();
        for cb in callbacks.iter() {
            cb.on_add(self.player_id, effect);
        }
    }

    fn notify_remove(&self, status: StatusChange) {
        let callbacks = self.on_change.read();
        for cb in callbacks.iter() {
            cb.on_remove(self.player_id, status);
        }
    }

    fn notify_expire(&self, status: StatusChange) {
        let callbacks = self.on_change.read();
        for cb in callbacks.iter() {
            cb.on_expire(self.player_id, status);
        }
    }
}

/// 调试用打印所有状态
impl std::fmt::Debug for PlayerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let effects = self.effects.read();
        f.debug_struct("PlayerStatus")
            .field("player_id", &self.player_id)
            .field("effects", &*effects)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_status() -> PlayerStatus {
        PlayerStatus::new(Uuid::new_v4())
    }

    #[test]
    fn test_add_and_get_status() {
        let status = make_test_status();
        let effect = StatusEffect::new(StatusChange::Blessing, 5000, StatusSource::Skill(10));

        status.add_status(effect);
        assert!(status.has_status(StatusChange::Blessing));
        assert!(status.get_status(StatusChange::Blessing).is_some());
    }

    #[test]
    fn test_remove_status() {
        let status = make_test_status();
        let effect = StatusEffect::new(StatusChange::Haste, 5000, StatusSource::Skill(1));

        status.add_status(effect);
        assert!(status.has_status(StatusChange::Haste));

        let removed = status.remove_status(StatusChange::Haste);
        assert!(removed.is_some());
        assert!(!status.has_status(StatusChange::Haste));
    }

    #[test]
    fn test_replace_stacking_rule() {
        let status = make_test_status();

        // 添加第一个效果
        let effect1 =
            StatusEffect::with_values(StatusChange::Stun, 5000, StatusSource::Skill(1), 100, 0, 0);
        status.add_status(effect1);

        // 添加第二个同类型效果（应该替换）
        let effect2 =
            StatusEffect::with_values(StatusChange::Stun, 3000, StatusSource::Skill(2), 200, 0, 0);
        status.add_status(effect2);

        let current = status.get_status(StatusChange::Stun).unwrap();
        // val1 应该是新值（替换模式）
        assert_eq!(current.val1, 200);
    }

    #[test]
    fn test_maximum_stacking_rule() {
        let status = make_test_status();

        // 添加第一个效果
        let effect1 = StatusEffect::with_values(
            StatusChange::IncreaseStr,
            5000,
            StatusSource::Skill(1),
            10,
            0,
            0,
        );
        status.add_status(effect1);

        // 添加第二个同类型效果（应该取大）
        let effect2 = StatusEffect::with_values(
            StatusChange::IncreaseStr,
            3000,
            StatusSource::Skill(2),
            15,
            0,
            0,
        );
        status.add_status(effect2);

        let current = status.get_status(StatusChange::IncreaseStr).unwrap();
        assert_eq!(current.val1, 15);
    }

    #[test]
    fn test_clear_all() {
        let status = make_test_status();

        status.add_status(StatusEffect::new(
            StatusChange::Haste,
            5000,
            StatusSource::Skill(1),
        ));
        status.add_status(StatusEffect::new(
            StatusChange::Blessing,
            5000,
            StatusSource::Skill(2),
        ));

        assert_eq!(status.count(), 2);
        status.clear_all();
        assert_eq!(status.count(), 0);
    }

    #[test]
    fn test_combat_restriction() {
        let status = make_test_status();
        assert!(!status.has_combat_restriction());

        status.add_status(StatusEffect::new(
            StatusChange::Stun,
            5000,
            StatusSource::Skill(1),
        ));
        assert!(status.has_combat_restriction());

        status.remove_status(StatusChange::Stun);
        assert!(!status.has_combat_restriction());
    }

    #[test]
    fn test_is_silenced() {
        let status = make_test_status();
        assert!(!status.is_silenced());

        status.add_status(StatusEffect::new(
            StatusChange::Silence,
            5000,
            StatusSource::Skill(1),
        ));
        assert!(status.is_silenced());
    }

    #[test]
    fn test_get_all_statuses() {
        let status = make_test_status();

        status.add_status(StatusEffect::new(
            StatusChange::Haste,
            5000,
            StatusSource::Skill(1),
        ));
        status.add_status(StatusEffect::new(
            StatusChange::Blessing,
            5000,
            StatusSource::Skill(2),
        ));

        let all = status.get_all_statuses();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_update_status() {
        let status = make_test_status();

        status.add_status(StatusEffect::with_values(
            StatusChange::IncreaseStr,
            5000,
            StatusSource::Skill(1),
            10,
            0,
            0,
        ));

        let updated = status.update_status(StatusChange::IncreaseStr, 20, 5, 0);
        assert!(updated);

        let effect = status.get_status(StatusChange::IncreaseStr).unwrap();
        assert_eq!(effect.val1, 20);
    }
}
