// 游戏循环 Tick 配置模块
// 控制客户端游戏逻辑的更新频率，影响动画、物理模拟等的精度

/// Tick 配置，控制游戏逻辑更新频率
///
/// 用于定义客户端游戏循环的逻辑帧率。
/// 默认 50Hz（20ms/tick），与 rAthena 服务端默认 tick 频率一致。
#[derive(Debug, Clone)]
pub struct TickConfig {
    /// 每 tick 毫秒数
    pub tick_rate_ms: u32,
    /// 每秒 tick 数（由 tick_rate_ms 计算得出）
    pub tick_rate_hz: f64,
}

/// 默认配置：20ms/tick = 50Hz，与服务端同步
impl Default for TickConfig {
    fn default() -> Self {
        Self::new(20)
    }
}

impl TickConfig {
    /// 创建自定义 tick 配置
    ///
    /// # 参数
    /// * `tick_rate_ms` - 每 tick 的毫秒数
    ///
    /// # 示例
    /// ```
    /// use devi::core::tick::TickConfig;
    /// let config = TickConfig::new(16); // 16ms/tick ≈ 62.5Hz
    /// assert_eq!(config.tick_rate_ms, 16);
    /// ```
    pub fn new(tick_rate_ms: u32) -> Self {
        Self {
            tick_rate_ms,
            tick_rate_hz: 1000.0 / tick_rate_ms as f64,
        }
    }
}
