// =============================================================================
// 锁接口（Lock）
// =============================================================================
//
// 统一的分布式锁接口，所有锁实现必须实现此 trait：
//
// - lock：获取锁（阻塞，直到获取成功或超时）
// - try_lock：尝试获取锁（非阻塞，立即返回）
// - unlock：释放锁
// - renew：续期锁
// - is_locked：检查锁是否被持有
// - owner：获取当前锁持有者
// =============================================================================

use crate::config::LockConfig;
use crate::{LockError, LockOwnerId, Result};
use async_trait::async_trait;
use std::time::Duration;

/// 获取锁的结果
#[derive(Debug)]
pub struct LockResult {
    /// 是否成功获取锁
    pub acquired: bool,
    /// 等待时间
    pub wait_duration: Duration,
    /// 锁的 key
    pub key: String,
    /// 持有者 ID
    pub owner: LockOwnerId,
    /// 重入次数（第一次获取为1）
    pub reentrant_count: u32,
}

/// 尝试获取锁的结果
#[derive(Debug)]
pub enum TryLockResult {
    /// 成功获取锁
    Acquired(LockResult),
    /// 锁已被其他持有者持有
    AlreadyLocked {
        /// 当前持有者
        owner: LockOwnerId,
        /// 锁的 key
        key: String,
    },
}

impl TryLockResult {
    /// 是否成功获取
    pub fn is_acquired(&self) -> bool {
        matches!(self, Self::Acquired(_))
    }

    /// 获取锁结果（如果成功）
    pub fn acquired(self) -> Option<LockResult> {
        match self {
            Self::Acquired(r) => Some(r),
            _ => None,
        }
    }
}

/// 分布式锁接口
///
/// 所有锁实现必须实现此 trait。
///
/// # 示例
/// ```
/// use mox_lock_core::{Lock, MemoryLock, LockConfig, LockOwnerId};
/// use std::time::Duration;
///
/// # async fn example() -> mox_lock_core::Result<()> {
/// let lock = MemoryLock::new(LockConfig::default());
/// let owner = LockOwnerId::new();
///
/// // 获取锁
/// let result = lock.lock("resource:123", &owner).await?;
/// assert!(result.acquired);
///
/// // 执行业务逻辑
///
/// // 释放锁
/// lock.unlock("resource:123", &owner).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait Lock: Send + Sync {
    /// 获取锁（阻塞，直到获取成功或超时）
    ///
    /// # 参数
    /// - `key`: 锁的标识符
    /// - `owner`: 锁持有者 ID
    ///
    /// # 返回
    /// 成功返回 LockResult，失败返回 LockError
    async fn lock(&self, key: &str, owner: &LockOwnerId) -> Result<LockResult>;

    /// 尝试获取锁（非阻塞，立即返回）
    ///
    /// # 参数
    /// - `key`: 锁的标识符
    /// - `owner`: 锁持有者 ID
    ///
    /// # 返回
    /// 返回 TryLockResult，表示是否成功获取
    async fn try_lock(&self, key: &str, owner: &LockOwnerId) -> Result<TryLockResult>;

    /// 释放锁
    ///
    /// # 参数
    /// - `key`: 锁的标识符
    /// - `owner`: 锁持有者 ID（必须与获取时一致）
    async fn unlock(&self, key: &str, owner: &LockOwnerId) -> Result<()>;

    /// 续期锁（延长持有时间）
    ///
    /// # 参数
    /// - `key`: 锁的标识符
    /// - `owner`: 锁持有者 ID
    /// - `duration`: 续期时长
    async fn renew(&self, key: &str, owner: &LockOwnerId, duration: Duration) -> Result<()>;

    /// 检查锁是否被持有
    async fn is_locked(&self, key: &str) -> Result<bool>;

    /// 获取当前锁持有者（如果锁被持有）
    async fn owner(&self, key: &str) -> Result<Option<LockOwnerId>>;

    /// 获取锁配置
    fn config(&self) -> &LockConfig;
}

/// 为 Arc<dyn Lock> 实现 Lock，方便使用
#[async_trait]
impl Lock for std::sync::Arc<dyn Lock> {
    async fn lock(&self, key: &str, owner: &LockOwnerId) -> Result<LockResult> {
        (**self).lock(key, owner).await
    }

    async fn try_lock(&self, key: &str, owner: &LockOwnerId) -> Result<TryLockResult> {
        (**self).try_lock(key, owner).await
    }

    async fn unlock(&self, key: &str, owner: &LockOwnerId) -> Result<()> {
        (**self).unlock(key, owner).await
    }

    async fn renew(&self, key: &str, owner: &LockOwnerId, duration: Duration) -> Result<()> {
        (**self).renew(key, owner, duration).await
    }

    async fn is_locked(&self, key: &str) -> Result<bool> {
        (**self).is_locked(key).await
    }

    async fn owner(&self, key: &str) -> Result<Option<LockOwnerId>> {
        (**self).owner(key).await
    }

    fn config(&self) -> &LockConfig {
        (**self).config()
    }
}
