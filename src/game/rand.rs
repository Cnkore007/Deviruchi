//! 可注入的随机数生成器模块
//!
//! 提供 `GameRng` trait，允许在测试时注入可控的 RNG 实现。

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
#[cfg(test)]
use std::cell::UnsafeCell;
use std::sync::Arc;

/// 可注入的随机数生成器 trait
pub trait GameRng: Send + Sync {
    /// 生成 [min, max] 范围内的随机 u32
    fn rand_range(&self, min: u32, max: u32) -> u32;

    /// 根据概率返回 true/false
    fn rand_bool(&self, probability: f32) -> bool;

    /// 基础点判定 (basis points)
    /// 生成 [0, 10000) 范围内的随机数，返回随机值
    /// 调用者通过比较 rand_bp(chance) < chance 来判定是否成功
    fn rand_bp(&self, chance: u32) -> u32;
}

/// 使用 `StdRng` 的生产实现 (Send + Sync)
pub struct ThreadRng(parking_lot::Mutex<StdRng>);

impl ThreadRng {
    pub fn new() -> Self {
        // 使用当前时间种子初始化
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        Self(parking_lot::Mutex::new(StdRng::seed_from_u64(seed)))
    }
}

impl Default for ThreadRng {
    fn default() -> Self {
        Self::new()
    }
}

impl GameRng for ThreadRng {
    fn rand_range(&self, min: u32, max: u32) -> u32 {
        self.0.lock().gen_range(min..=max)
    }

    fn rand_bool(&self, probability: f32) -> bool {
        let normalized = probability.clamp(0.0, 100.0) / 100.0;
        self.0.lock().gen_bool(normalized as f64)
    }

    fn rand_bp(&self, _chance: u32) -> u32 {
        // Generate random value in [0, 10000) range for basis points
        // 调用者通过 rand_bp(chance) < chance 判定成功
        self.0.lock().gen_range(0..10000)
    }
}

/// 便捷构造函数 - 创建 Arc 化的 ThreadRng
pub fn thread_rng() -> Arc<dyn GameRng> {
    Arc::new(ThreadRng::new())
}

/// 测试用的确定性 Mock RNG（模块级别，供其他测试模块导入）
#[cfg(test)]
pub struct MockRng {
    values: Vec<u32>,
    index: UnsafeCell<usize>,
}

#[cfg(test)]
impl MockRng {
    pub fn new(values: Vec<u32>) -> Self {
        Self {
            values,
            index: UnsafeCell::new(0),
        }
    }
}

// Safety: MockRng is only used in single-threaded test context
#[cfg(test)]
unsafe impl Send for MockRng {}
#[cfg(test)]
unsafe impl Sync for MockRng {}

#[cfg(test)]
impl GameRng for MockRng {
    fn rand_range(&self, min: u32, max: u32) -> u32 {
        let idx = {
            let p = unsafe { &mut *self.index.get() };
            let current = *p;
            *p = current.wrapping_add(1);
            current
        };
        let val = self
            .values
            .get(idx % self.values.len())
            .copied()
            .unwrap_or(min);
        val.min(max).max(min)
    }

    fn rand_bool(&self, _probability: f32) -> bool {
        let idx = {
            let p = unsafe { &mut *self.index.get() };
            let current = *p;
            *p = current.wrapping_add(1);
            current
        };
        let val = self
            .values
            .get(idx % self.values.len())
            .copied()
            .unwrap_or(0);
        val % 2 == 0
    }

    fn rand_bp(&self, _chance: u32) -> u32 {
        let idx = {
            let p = unsafe { &mut *self.index.get() };
            let current = *p;
            *p = current.wrapping_add(1);
            current
        };
        self.values
            .get(idx % self.values.len())
            .copied()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_rng_rand_range() {
        let rng = ThreadRng::new();
        // 验证在范围内
        for _ in 0..100 {
            let val = rng.rand_range(1, 10);
            assert!(val >= 1 && val <= 10);
        }
    }

    #[test]
    fn test_thread_rng_rand_bool() {
        let rng = ThreadRng::new();
        // 验证返回布尔值
        for _ in 0..100 {
            let _ = rng.rand_bool(50.0);
        }
    }

    #[test]
    fn test_mock_rng_deterministic() {
        let rng = MockRng::new(vec![5, 10, 15, 20]);
        assert_eq!(rng.rand_range(0, 100), 5);
        assert_eq!(rng.rand_range(0, 100), 10);
        assert_eq!(rng.rand_range(0, 100), 15);
        assert_eq!(rng.rand_range(0, 100), 20);
    }

    #[test]
    fn test_mock_rng_bool() {
        let rng = MockRng::new(vec![0, 1, 2, 3]);
        assert!(rng.rand_bool(50.0)); // 0 -> even
        assert!(!rng.rand_bool(50.0)); // 1 -> odd
        assert!(rng.rand_bool(50.0)); // 2 -> even
        assert!(!rng.rand_bool(50.0)); // 3 -> odd
    }
}
