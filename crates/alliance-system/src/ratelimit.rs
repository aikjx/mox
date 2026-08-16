//! 限流器（I-04 安全防护）：固定窗口计数器
//!
//! 以「令牌 / 匿名 IP」为键，在滑动窗口内累计请求数，超过阈值即拒绝。
//! 用于防御暴力探测、令牌喷洒与资源耗尽类攻击（安全 P1）。
//! 进程内实现，零外部依赖；后续可替换为 Redis 共享限流以支持水平扩展。
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct Bucket {
    count: u32,
    start: Instant,
}

pub struct RateLimiter {
    limit: u32,
    window: Duration,
    buckets: Mutex<HashMap<String, Bucket>>,
}

impl RateLimiter {
    pub fn new(limit: u32, window_secs: u64) -> Self {
        Self {
            limit,
            window: Duration::from_secs(window_secs),
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// 检查并记账：返回 `true` 表示放行，返回 `false` 表示触发限流。
    /// 窗口过期后自动重置计数。
    pub fn check(&self, key: &str) -> bool {
        let mut map = self.buckets.lock().unwrap();
        let now = Instant::now();
        match map.get_mut(key) {
            Some(b) => {
                if now.duration_since(b.start) >= self.window {
                    b.count = 1;
                    b.start = now;
                    true
                } else if b.count < self.limit {
                    b.count += 1;
                    true
                } else {
                    false
                }
            }
            None => {
                map.insert(
                    key.to_string(),
                    Bucket {
                        count: 1,
                        start: now,
                    },
                );
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_limit_then_blocks() {
        let rl = RateLimiter::new(3, 60);
        assert!(rl.check("k"));
        assert!(rl.check("k"));
        assert!(rl.check("k"));
        assert!(!rl.check("k"), "超过上限应被拦截");
        // 不同键互不影响
        assert!(rl.check("other"));
    }
}
