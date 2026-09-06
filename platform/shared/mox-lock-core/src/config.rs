// =============================================================================
// 锁配置（LockConfig）
// =============================================================================
//
// 锁的配置参数，控制锁的行为：
//
// - 锁模式：公平锁 / 非公平锁
// - 获取超时：最长等待时间
// - 持有超时：最长持有时间（自动释放）
// - 自动续期：是否启用后台自动续期
// - 续期间隔：自动续期的时间间隔
// - 可重入：是否允许同一持有者多次获取
// =============================================================================

use std::time::Duration;

/// 锁模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockMode {
    /// 非公平锁（默认，性能更好）
    NonFair,
    /// 公平锁（FIFO，先到先得）
    Fair,
}

impl Default for LockMode {
    fn default() -> Self {
        Self::NonFair
    }
}

/// 锁配置
#[derive(Debug, Clone)]
pub struct LockConfig {
    /// 锁模式
    pub mode: LockMode,
    /// 获取锁超时（0 表示不等待，立即返回）
    pub acquire_timeout: Duration,
    /// 持有锁超时（自动释放，防止死锁）
    pub hold_timeout: Duration,
    /// 是否启用自动续期
    pub auto_renew: bool,
    /// 自动续期间隔（必须小于 hold_timeout）
    pub renew_interval: Duration,
    /// 是否可重入（同一持有者可多次获取）
    pub reentrant: bool,
    /// 最大重入次数
    pub max_reentrant_count: u32,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self {
            mode: LockMode::default(),
            acquire_timeout: Duration::from_secs(30),
            hold_timeout: Duration::from_secs(60),
            auto_renew: true,
            renew_interval: Duration::from_secs(20),
            reentrant: true,
            max_reentrant_count: 10,
        }
    }
}

impl LockConfig {
    /// 创建新的锁配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置锁模式
    pub fn with_mode(mut self, mode: LockMode) -> Self {
        self.mode = mode;
        self
    }

    /// 设置获取超时
    pub fn with_acquire_timeout(mut self, timeout: Duration) -> Self {
        self.acquire_timeout = timeout;
        self
    }

    /// 设置持有超时
    pub fn with_hold_timeout(mut self, timeout: Duration) -> Self {
        self.hold_timeout = timeout;
        self
    }

    /// 设置是否自动续期
    pub fn with_auto_renew(mut self, enabled: bool) -> Self {
        self.auto_renew = enabled;
        self
    }

    /// 设置续期间隔
    pub fn with_renew_interval(mut self, interval: Duration) -> Self {
        self.renew_interval = interval;
        self
    }

    /// 设置是否可重入
    pub fn with_reentrant(mut self, reentrant: bool) -> Self {
        self.reentrant = reentrant;
        self
    }

    /// 设置最大重入次数
    pub fn with_max_reentrant_count(mut self, count: u32) -> Self {
        self.max_reentrant_count = count;
        self
    }

    /// 验证配置有效性
    pub fn validate(&self) -> Result<(), String> {
        if self.auto_renew && self.renew_interval >= self.hold_timeout {
            return Err(format!(
                "续期间隔({:?})必须小于持有超时({:?})",
                self.renew_interval, self.hold_timeout
            ));
        }
        if self.reentrant && self.max_reentrant_count == 0 {
            return Err("可重入锁的最大重入次数必须大于0".to_string());
        }
        Ok(())
    }

    /// 快速创建：短超时锁（适用于快速操作）
    pub fn short_lived() -> Self {
        Self {
            acquire_timeout: Duration::from_secs(5),
            hold_timeout: Duration::from_secs(10),
            auto_renew: false,
            ..Default::default()
        }
    }

    /// 快速创建：长持有锁（适用于长时间操作，自动续期）
    pub fn long_lived() -> Self {
        Self {
            acquire_timeout: Duration::from_secs(60),
            hold_timeout: Duration::from_secs(300),
            auto_renew: true,
            renew_interval: Duration::from_secs(60),
            ..Default::default()
        }
    }

    /// 快速创建：非阻塞锁（立即返回，不等待）
    pub fn non_blocking() -> Self {
        Self {
            acquire_timeout: Duration::from_secs(0),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LockConfig::default();
        assert_eq!(config.mode, LockMode::NonFair);
        assert!(config.auto_renew);
        assert!(config.reentrant);
    }

    #[test]
    fn test_config_builder() {
        let config = LockConfig::new()
            .with_mode(LockMode::Fair)
            .with_acquire_timeout(Duration::from_secs(10))
            .with_hold_timeout(Duration::from_secs(30))
            .with_auto_renew(false)
            .with_reentrant(false);

        assert_eq!(config.mode, LockMode::Fair);
        assert_eq!(config.acquire_timeout, Duration::from_secs(10));
        assert_eq!(config.hold_timeout, Duration::from_secs(30));
        assert!(!config.auto_renew);
        assert!(!config.reentrant);
    }

    #[test]
    fn test_config_validation() {
        // 有效配置
        let valid = LockConfig::default();
        assert!(valid.validate().is_ok());

        // 续期间隔 >= 持有超时
        let invalid = LockConfig::default()
            .with_renew_interval(Duration::from_secs(60))
            .with_hold_timeout(Duration::from_secs(30));
        assert!(invalid.validate().is_err());

        // 可重入但最大次数为0
        let invalid2 = LockConfig::default()
            .with_max_reentrant_count(0);
        assert!(invalid2.validate().is_err());
    }

    #[test]
    fn test_preset_configs() {
        let short = LockConfig::short_lived();
        assert!(!short.auto_renew);
        assert!(short.hold_timeout <= Duration::from_secs(10));

        let long = LockConfig::long_lived();
        assert!(long.auto_renew);
        assert!(long.hold_timeout >= Duration::from_secs(120));

        let non_blocking = LockConfig::non_blocking();
        assert_eq!(non_blocking.acquire_timeout, Duration::from_secs(0));
    }
}
