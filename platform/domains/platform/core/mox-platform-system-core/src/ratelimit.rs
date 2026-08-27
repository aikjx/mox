// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

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
                let expired =
                    self.window > Duration::ZERO && now.duration_since(b.start) >= self.window;
                if expired {
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

    #[test]
    fn window_expiry_resets_counter() {
        // 非零窗口：窗口过期后计数应重置
        let rl = RateLimiter::new(2, 1);
        assert!(rl.check("k"));
        assert!(rl.check("k"));
        assert!(!rl.check("k"), "窗口内应被拦截");
        // 等待窗口过期，下一次 check 应视为新窗口放行
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert!(rl.check("k"), "窗口过期后计数应重置并放行");
    }

    #[test]
    fn zero_window_never_resets() {
        // window=0 视为一次性配额：达到上限后永不再放行（无时间基准可重置）
        let rl = RateLimiter::new(2, 0);
        assert!(rl.check("z"));
        assert!(rl.check("z"));
        assert!(!rl.check("z"), "0 窗口达上限后必须持续拦截");
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(!rl.check("z"), "0 窗口不应因时差重置");
    }

    #[test]
    fn independent_keys() {
        let rl = RateLimiter::new(1, 60);
        assert!(rl.check("a"));
        assert!(!rl.check("a"));
        assert!(rl.check("b"));
        assert!(!rl.check("b"));
    }
}
