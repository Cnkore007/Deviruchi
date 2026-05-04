//! 状态效果实例

use super::types::StatusChange;
use std::time::Instant;

/// 状态效果来源
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusSource {
    /// 技能来源 (skill_id)
    Skill(u16),
    /// 物品来源 (item_id)
    Item(u16),
    /// 被动技能
    Passive,
    /// 任务奖励
    Quest,
    /// 环境效果
    Environment,
}

impl StatusSource {
    /// 获取来源名称
    pub fn name(&self) -> &'static str {
        match self {
            StatusSource::Skill(_) => "Skill",
            StatusSource::Item(_) => "Item",
            StatusSource::Passive => "Passive",
            StatusSource::Quest => "Quest",
            StatusSource::Environment => "Environment",
        }
    }
}

/// 状态效果实例
#[derive(Debug, Clone)]
pub struct StatusEffect {
    /// 状态类型
    pub id: StatusChange,
    /// 持续时间（毫秒），0 表示无限
    pub duration_ms: u64,
    /// 效果开始时间
    pub started_at: Instant,
    /// 效果来源
    pub source: StatusSource,
    /// 效果值1（用途取决于状态类型，如加成的具体数值）
    pub val1: i32,
    /// 效果值2
    pub val2: i32,
    /// 效果值3
    pub val3: i32,
    /// 效果层数（用于可叠加状态）
    pub stack: u8,
}

impl StatusEffect {
    /// 创建新的状态效果
    pub fn new(id: StatusChange, duration_ms: u64, source: StatusSource) -> Self {
        Self {
            id,
            duration_ms,
            started_at: Instant::now(),
            source,
            val1: 0,
            val2: 0,
            val3: 0,
            stack: 1,
        }
    }

    /// 创建带有效果值的状态效果
    pub fn with_values(
        id: StatusChange,
        duration_ms: u64,
        source: StatusSource,
        val1: i32,
        val2: i32,
        val3: i32,
    ) -> Self {
        Self {
            id,
            duration_ms,
            started_at: Instant::now(),
            source,
            val1,
            val2,
            val3,
            stack: 1,
        }
    }

    /// 检查效果是否已过期
    pub fn is_expired(&self) -> bool {
        // 持续时间为0表示无限效果
        if self.duration_ms == 0 {
            return false;
        }
        self.elapsed_ms() >= self.duration_ms
    }

    /// 获取已过去的时间（毫秒）
    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }

    /// 获取剩余时间（毫秒）
    pub fn remaining_ms(&self) -> u64 {
        if self.duration_ms == 0 {
            return u64::MAX; // 无限效果返回最大值
        }
        self.duration_ms.saturating_sub(self.elapsed_ms())
    }

    /// 更新效果值
    pub fn set_values(&mut self, val1: i32, val2: i32, val3: i32) {
        self.val1 = val1;
        self.val2 = val2;
        self.val3 = val3;
    }

    /// 增加层数
    pub fn add_stack(&mut self, amount: u8) {
        self.stack = self.stack.saturating_add(amount);
    }

    /// 刷新效果（重置持续时间）
    pub fn refresh(&mut self) {
        self.started_at = Instant::now();
    }

    /// 获取有效时间（取较大值，用于叠加刷新）
    pub fn extend_duration(&mut self, additional_ms: u64) {
        // 如果当前剩余时间少于新增时间，则刷新
        if self.remaining_ms() < additional_ms {
            self.refresh();
        }
    }
}

/// 状态效果的叠加规则
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackingRule {
    /// 替换模式：同类型效果直接替换
    Replace,
    /// 叠加模式：数值叠加
    Additive,
    /// 取大模式：取较大值
    Maximum,
    /// 时间延长：刷新到较长时间
    Extend,
}

impl StatusChange {
    /// 获取该状态的叠加规则
    pub fn stacking_rule(&self) -> StackingRule {
        match self {
            // 属性提升类采用取大模式
            StatusChange::IncreaseStr
            | StatusChange::IncreaseAgi
            | StatusChange::IncreaseVit
            | StatusChange::IncreaseInt
            | StatusChange::IncreaseDex
            | StatusChange::IncreaseLuk => StackingRule::Maximum,

            // 加速类采用时间延长模式
            StatusChange::Haste | StatusChange::AttackSpeedUp | StatusChange::MaxSpeedUp => {
                StackingRule::Extend
            }

            // 防御类采用取大模式
            StatusChange::Shield | StatusChange::DefenseUp | StatusChange::MagicDefenseUp => {
                StackingRule::Maximum
            }

            // 持续伤害类采用替换模式
            StatusChange::Poison
            | StatusChange::Bleeding
            | StatusChange::Slow
            | StatusChange::SpeedDown => StackingRule::Replace,

            // 其他采用替换模式
            _ => StackingRule::Replace,
        }
    }

    /// 检查该状态是否可以与其他同类叠加
    pub fn can_stack(&self) -> bool {
        matches!(
            self,
            StatusChange::IncreaseStr
                | StatusChange::IncreaseAgi
                | StatusChange::IncreaseVit
                | StatusChange::IncreaseInt
                | StatusChange::IncreaseDex
                | StatusChange::IncreaseLuk
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_status_effect() {
        let effect = StatusEffect::new(StatusChange::Blessing, 10000, StatusSource::Skill(10));

        assert_eq!(effect.id, StatusChange::Blessing);
        assert_eq!(effect.duration_ms, 10000);
        assert!(!effect.is_expired());
    }

    #[test]
    fn test_effect_with_values() {
        let effect = StatusEffect::with_values(
            StatusChange::IncreaseStr,
            5000,
            StatusSource::Skill(1),
            10,
            0,
            0,
        );

        assert_eq!(effect.val1, 10);
        assert_eq!(effect.val2, 0);
    }

    #[test]
    fn test_effect_expiry() {
        let mut effect = StatusEffect::new(StatusChange::Poison, 100, StatusSource::Skill(1));

        // 刚创建应该没过期
        assert!(!effect.is_expired());

        // 刷新后应该重置
        effect.refresh();
        assert!(!effect.is_expired());
    }

    #[test]
    fn test_infinite_duration() {
        let effect = StatusEffect::new(
            StatusChange::Invincible,
            0, // 无限持续
            StatusSource::Passive,
        );

        assert!(!effect.is_expired());
        assert_eq!(effect.remaining_ms(), u64::MAX);
    }

    #[test]
    fn test_stack_operations() {
        let mut effect = StatusEffect::new(StatusChange::IncreaseStr, 5000, StatusSource::Skill(1));

        assert_eq!(effect.stack, 1);
        effect.add_stack(2);
        assert_eq!(effect.stack, 3);
    }

    #[test]
    fn test_stacking_rule() {
        assert_eq!(
            StatusChange::IncreaseStr.stacking_rule(),
            StackingRule::Maximum
        );
        assert_eq!(StatusChange::Haste.stacking_rule(), StackingRule::Extend);
        assert_eq!(StatusChange::Poison.stacking_rule(), StackingRule::Replace);
    }
}
