use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::time::{Duration, Instant};

static TIMER_QUEUE: Lazy<Mutex<BinaryHeap<TimerEntry>>> =
    Lazy::new(|| Mutex::new(BinaryHeap::new()));

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TimerId(u64);

impl TimerId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

struct TimerEntry {
    due: Instant,
    id: u64,
    callback: std::sync::Arc<dyn Fn() + Send + Sync + 'static>,
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TimerEntry {}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap 是 max-heap，我们用 reverse 实现 min-heap
        other.due.cmp(&self.due)
    }
}

pub struct Timer;

impl Timer {
    /// 添加一个一次性定时器
    pub fn add<F>(delay: Duration, callback: F) -> TimerId
    where
        F: Fn() + Send + Sync + 'static,
    {
        let id = generate_timer_id();
        let entry = TimerEntry {
            due: Instant::now() + delay,
            id,
            callback: std::sync::Arc::new(callback),
        };

        TIMER_QUEUE.lock().push(entry);
        TimerId(id)
    }

    /// 添加一个重复执行的定时器
    pub fn add_interval<F>(interval: Duration, callback: F) -> TimerId
    where
        F: Fn() + Send + Sync + 'static,
    {
        let id = generate_timer_id();
        let due = Instant::now() + interval;

        // 使用 Arc<dyn Fn()> 实现重复调用的回调
        let callback: std::sync::Arc<dyn Fn() + Send + Sync + 'static> =
            std::sync::Arc::new(callback);
        let callback_clone = callback.clone();

        let wrapper = std::sync::Arc::new(move || {
            callback_clone();
            let entry = TimerEntry {
                due: Instant::now() + interval,
                id,
                callback: callback.clone(),
            };
            TIMER_QUEUE.lock().push(entry);
        });

        let entry = TimerEntry {
            due,
            id,
            callback: wrapper,
        };
        TIMER_QUEUE.lock().push(entry);
        TimerId(id)
    }

    /// 处理所有到期的定时器
    pub fn process() {
        let now = Instant::now();
        let mut queue = TIMER_QUEUE.lock();

        while let Some(entry) = queue.peek() {
            if entry.due <= now {
                if let Some(entry) = queue.pop() {
                    (entry.callback)();
                }
            } else {
                break;
            }
        }
    }

    /// 获取下一个定时器到期的时间
    pub fn next_due() -> Option<Duration> {
        let queue = TIMER_QUEUE.lock();
        queue.peek().map(|entry| {
            if entry.due > Instant::now() {
                entry.due - Instant::now()
            } else {
                Duration::ZERO
            }
        })
    }
}

static TIMER_ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn generate_timer_id() -> u64 {
    TIMER_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_basic() {
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();

        Timer::add(Duration::from_millis(10), move || {
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        std::thread::sleep(Duration::from_millis(20));
        Timer::process();

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }
}
