//! 装备精炼系统
//!
//! 实现 rAthena 风格的精炼规则，包括成功率计算、精炼执行和属性加成。

use super::inventory::InventorySlot;
use crate::game::rand::GameRng;
use std::sync::Arc;

/// 精炼结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefineResult {
    /// 精炼成功，新的精炼等级
    Success { new_refine: u8 },
    /// 精炼失败，等级不变
    Failure,
    /// 精炼失败，装备损坏
    Broken,
    /// 已达最大精炼等级
    MaxLevel,
}

/// 精炼属性加成
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefineBonus {
    /// 武器每级 +2 ATK
    pub atk_bonus: u16,
    /// 防具每级 +1 DEF
    pub def_bonus: u16,
}

/// 精炼最大等级
const REFINE_MAX_LEVEL: u8 = 20;

/// 精炼成功率表（万分比），索引为当前精炼等级
/// +0→+1: 10000 (100%), +1→+2: 10000, +2→+3: 10000, +3→+4: 10000
/// +4→+5: 6000 (60%), +5→+6: 4000, +6→+7: 2000, +7→+8: 1000
/// +8→+9: 500, +9→+10: 200, +10以上: 0
const REFINE_RATE_TABLE: [u32; 21] = [
    10000, // +0 → +1: 100%
    10000, // +1 → +2: 100%
    10000, // +2 → +3: 100%
    10000, // +3 → +4: 100%
    6000,  // +4 → +5: 60%
    4000,  // +5 → +6: 40%
    2000,  // +6 → +7: 20%
    1000,  // +7 → +8: 10%
    500,   // +8 → +9: 5%
    200,   // +9 → +10: 2%
    0,     // +10 → +11: 0%
    0,     // +11 → +12
    0,     // +12 → +13
    0,     // +13 → +14
    0,     // +14 → +15
    0,     // +15 → +16
    0,     // +16 → +17
    0,     // +17 → +18
    0,     // +18 → +19
    0,     // +19 → +20
    0,     // 已达最大等级，不使用
];

/// 精炼系统
pub struct RefineSystem;

impl RefineSystem {
    /// 计算精炼成功率（万分比，0-10000）
    ///
    /// 根据 rAthena 标准成功率表，返回从当前等级精炼到下一级的成功概率。
    /// 当前等级 >= 20 时返回 0。
    pub fn success_rate(current_refine: u8) -> u32 {
        if current_refine >= REFINE_MAX_LEVEL {
            return 0;
        }
        REFINE_RATE_TABLE[current_refine as usize]
    }

    /// 尝试精炼装备
    ///
    /// 使用注入的 `GameRng` 进行随机判定。如果当前等级已达上限则返回 `MaxLevel`。
    /// 随机值 < 成功率时精炼成功，否则失败。失败时（危险精炼模式）有概率损坏装备。
    ///
    /// 注意：当前实现对武器和防具使用相同的失败逻辑（50% 损坏率）。
    /// rAthena 中武器失败降低精炼等级，防具失败有损坏概率，后续可扩展差异化处理。
    pub fn refine(item: &mut InventorySlot, _is_weapon: bool, rng: &Arc<dyn GameRng>) -> RefineResult {
        let current = item.refine;

        // 检查是否已达最大等级
        if current >= REFINE_MAX_LEVEL {
            return RefineResult::MaxLevel;
        }

        let rate = Self::success_rate(current);

        // 生成 [0, 10000) 的随机数进行判定
        let roll = rng.rand_bp(rate);

        if roll < rate {
            // 精炼成功
            item.refine += 1;
            RefineResult::Success { new_refine: item.refine }
        } else {
            // 精炼失败，50% 概率装备损坏（危险精炼）
            let break_roll = rng.rand_bp(5000);
            if break_roll < 5000 {
                RefineResult::Broken
            } else {
                RefineResult::Failure
            }
        }
    }

    /// 获取精炼提供的属性加成
    ///
    /// - 武器：每级 +2 ATK
    /// - 防具：每级 +1 DEF
    pub fn get_refine_bonus(refine: u8, is_weapon: bool) -> RefineBonus {
        let level = refine.min(REFINE_MAX_LEVEL) as u16;
        if is_weapon {
            RefineBonus {
                atk_bonus: level * 2,
                def_bonus: 0,
            }
        } else {
            RefineBonus {
                atk_bonus: 0,
                def_bonus: level,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rand::MockRng;

    /// 创建测试用的 InventorySlot
    fn make_slot(refine: u8) -> InventorySlot {
        InventorySlot {
            index: 0,
            item_id: 1201,
            amount: 1,
            identified: true,
            refine,
            cards: [0; 4],
        }
    }

    // ========== 成功率测试 ==========

    #[test]
    fn test_success_rate_low_levels() {
        // +0 到 +3 都是 100%
        assert_eq!(RefineSystem::success_rate(0), 10000);
        assert_eq!(RefineSystem::success_rate(1), 10000);
        assert_eq!(RefineSystem::success_rate(2), 10000);
        assert_eq!(RefineSystem::success_rate(3), 10000);
    }

    #[test]
    fn test_success_rate_mid_levels() {
        assert_eq!(RefineSystem::success_rate(4), 6000); // 60%
        assert_eq!(RefineSystem::success_rate(5), 4000); // 40%
        assert_eq!(RefineSystem::success_rate(6), 2000); // 20%
        assert_eq!(RefineSystem::success_rate(7), 1000); // 10%
        assert_eq!(RefineSystem::success_rate(8), 500);  // 5%
        assert_eq!(RefineSystem::success_rate(9), 200);  // 2%
    }

    #[test]
    fn test_success_rate_high_levels() {
        // +10 及以上成功率均为 0
        for level in 10..=20 {
            assert_eq!(RefineSystem::success_rate(level), 0);
        }
    }

    // ========== 精炼执行测试 ==========

    #[test]
    fn test_refine_success() {
        let mut slot = make_slot(0);
        // rand_bp 返回值 100 < rate 10000 => 成功
        let rng: Arc<dyn GameRng> = Arc::new(MockRng::new(vec![100]));
        let result = RefineSystem::refine(&mut slot, true, &rng);
        assert_eq!(result, RefineResult::Success { new_refine: 1 });
        assert_eq!(slot.refine, 1);
    }

    #[test]
    fn test_refine_failure_not_broken() {
        let mut slot = make_slot(0);
        // roll=9999 (>= rate 0 的情况不会发生，改用 +4→+5，rate=6000)
        slot.refine = 4;
        // 第一次 rand_bp 返回 8000 (>=6000，失败)，第二次 rand_bp 返回 8000 (>=5000，不损坏)
        let rng: Arc<dyn GameRng> = Arc::new(MockRng::new(vec![8000, 8000]));
        let result = RefineSystem::refine(&mut slot, true, &rng);
        assert_eq!(result, RefineResult::Failure);
        assert_eq!(slot.refine, 4); // 等级不变
    }

    #[test]
    fn test_refine_failure_broken() {
        let mut slot = make_slot(4);
        // 第一次 rand_bp 返回 8000 (>=6000，失败)，第二次 rand_bp 返回 1000 (<5000，损坏)
        let rng: Arc<dyn GameRng> = Arc::new(MockRng::new(vec![8000, 1000]));
        let result = RefineSystem::refine(&mut slot, true, &rng);
        assert_eq!(result, RefineResult::Broken);
    }

    #[test]
    fn test_refine_max_level() {
        let mut slot = make_slot(20);
        let rng: Arc<dyn GameRng> = Arc::new(MockRng::new(vec![0]));
        let result = RefineSystem::refine(&mut slot, true, &rng);
        assert_eq!(result, RefineResult::MaxLevel);
        assert_eq!(slot.refine, 20);
    }

    #[test]
    fn test_refine_level_10_has_zero_rate() {
        let mut slot = make_slot(10);
        // rate = 0, roll=0 < 0 为 false，所以必然失败
        let rng: Arc<dyn GameRng> = Arc::new(MockRng::new(vec![0, 8000]));
        let result = RefineSystem::refine(&mut slot, true, &rng);
        // rate=0, 0 < 0 为 false => 失败
        assert!(matches!(result, RefineResult::Failure | RefineResult::Broken));
    }

    // ========== 属性加成测试 ==========

    #[test]
    fn test_refine_bonus_weapon() {
        let bonus = RefineSystem::get_refine_bonus(7, true);
        assert_eq!(bonus.atk_bonus, 14); // 7 * 2
        assert_eq!(bonus.def_bonus, 0);
    }

    #[test]
    fn test_refine_bonus_armor() {
        let bonus = RefineSystem::get_refine_bonus(5, false);
        assert_eq!(bonus.atk_bonus, 0);
        assert_eq!(bonus.def_bonus, 5); // 5 * 1
    }

    #[test]
    fn test_refine_bonus_zero() {
        let weapon_bonus = RefineSystem::get_refine_bonus(0, true);
        assert_eq!(weapon_bonus.atk_bonus, 0);
        let armor_bonus = RefineSystem::get_refine_bonus(0, false);
        assert_eq!(armor_bonus.def_bonus, 0);
    }

    #[test]
    fn test_refine_bonus_capped_at_max() {
        // 超过 20 级也应被限制
        let bonus = RefineSystem::get_refine_bonus(25, true);
        assert_eq!(bonus.atk_bonus, 40); // 20 * 2
    }
}
