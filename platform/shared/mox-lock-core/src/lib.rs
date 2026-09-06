// =============================================================================
// MOX 统一分布式锁核心（mox-lock-core）
// =============================================================================
//
// 企业级分布式锁基础设施，参考 minio lock/dsync 架构设计：
//
// 1. **锁接口**（Lock）— 统一的分布式锁接口，支持多种后端实现
// 2. **内存锁**（MemoryLock）— 单进程内的锁实现，适用于单体应用和测试
// 3. **锁守卫**（LockGuard）— RAII 模式，自动释放锁，防止死锁
// 4. **可重入锁**— 同一持有者可多次获取同一把锁
// 5. **自动续期**— 后台线程自动续期，防止业务未完成锁已过期
// 6. **锁统计**— 锁获取次数、等待时间、持有时间等指标
//
// 设计原则：
// - 安全性：超时自动释放，防止死锁
// - 公平性：支持公平锁（FIFO）和非公平锁
// - 可观测：锁获取/释放/超时全链路追踪
// - 可扩展：支持 Redis、etcd、ZooKeeper 等多种后端
// =============================================================================

pub mod lock;
pub mod memory;
pub mod guard;
pub mod config;
pub mod stats;

// ── 重导出 ────────────────────────────────────────────────────────────────

pub use lock::{Lock, LockResult, TryLockResult};
pub use memory::MemoryLock;
pub use guard::LockGuard;
pub use config::{LockConfig, LockMode};
pub use stats::{LockStats, LockMetrics};

// ── Crate 元数据 ──────────────────────────────────────────────────────────

pub const CRATE_ID: &str = "mox-lock-core";
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

use thiserror::Error;

/// 分布式锁错误
#[derive(Debug, Error)]
pub enum LockError {
    #[error("获取锁超时: {0}")]
    Timeout(String),

    #[error("锁已被其他持有者持有: {0}")]
    AlreadyLocked(String),

    #[error("锁不存在: {0}")]
    NotFound(String),

    #[error("锁已过期: {0}")]
    Expired(String),

    #[error("非法的锁持有者: {0}")]
    InvalidOwner(String),

    #[error("锁已被释放: {0}")]
    AlreadyReleased(String),

    #[error("后端错误: {0}")]
    BackendError(String),

    #[error("内部错误: {0}")]
    InternalError(String),
}

/// 锁结果类型
pub type Result<T> = std::result::Result<T, LockError>;

/// 锁持有者 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LockOwnerId(String);

impl LockOwnerId {
    /// 生成新的持有者 ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// 从字符串创建持有者 ID
    pub fn from_str<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    /// 获取持有者 ID 字符串
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for LockOwnerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for LockOwnerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for LockOwnerId {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_owner_id() {
        let id1 = LockOwnerId::new();
        let id2 = LockOwnerId::new();
        assert_ne!(id1, id2);
        assert!(!id1.as_str().is_empty());
    }

    #[test]
    fn test_lock_error_display() {
        let err = LockError::Timeout("test".to_string());
        assert!(err.to_string().contains("获取锁超时"));
    }
}
