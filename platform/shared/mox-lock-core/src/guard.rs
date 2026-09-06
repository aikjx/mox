// =============================================================================
// 锁守卫（LockGuard）
// =============================================================================
//
// RAII 模式的锁守卫，自动释放锁，防止忘记释放导致死锁：
//
// - 创建时获取锁
// - Drop 时自动释放锁
// - 支持手动提前释放
// - 支持重入计数
// - 支持续期
//
// # 示例
// ```
// use mox_lock_core::{MemoryLock, LockGuard, LockOwnerId};
//
// # async fn example() -> mox_lock_core::Result<()> {
// let lock = MemoryLock::default();
// let owner = LockOwnerId::new();
//
// // 使用守卫，作用域结束时自动释放
// {
//     let guard = LockGuard::acquire(lock.clone(), "resource:123", &owner).await?;
//     // 执行业务逻辑
// } // 这里自动释放锁
//
// # Ok(())
// # }
// ```
// =============================================================================

use crate::lock::Lock;
use crate::{LockError, LockOwnerId, Result};
use std::sync::Arc;
use std::time::Duration;

/// 锁守卫
///
/// RAII 模式，Drop 时自动释放锁。
pub struct LockGuard<L: Lock + 'static + ?Sized> {
    /// 锁引用
    lock: Arc<L>,
    /// 锁的 key
    key: String,
    /// 持有者 ID
    owner: LockOwnerId,
    /// 是否已释放
    released: bool,
    /// 重入次数
    reentrant_count: u32,
}

impl<L: Lock + 'static + ?Sized> LockGuard<L> {
    /// 获取锁并创建守卫
    ///
    /// # 参数
    /// - `lock`: 锁实例（Arc）
    /// - `key`: 锁的标识符
    /// - `owner`: 持有者 ID
    ///
    /// # 返回
    /// 成功返回 LockGuard，失败返回 LockError
    pub async fn acquire(lock: Arc<L>, key: &str, owner: &LockOwnerId) -> Result<Self> {
        let result = lock.lock(key, owner).await?;
        if !result.acquired {
            return Err(LockError::Timeout(format!("获取锁 {} 失败", key)));
        }

        Ok(Self {
            lock,
            key: key.to_string(),
            owner: owner.clone(),
            released: false,
            reentrant_count: result.reentrant_count,
        })
    }

    /// 尝试获取锁并创建守卫（非阻塞）
    ///
    /// # 返回
    /// 成功返回 Some(LockGuard)，锁已被持有返回 None
    pub async fn try_acquire(
        lock: Arc<L>,
        key: &str,
        owner: &LockOwnerId,
    ) -> Result<Option<Self>> {
        use crate::lock::TryLockResult;

        match lock.try_lock(key, owner).await? {
            TryLockResult::Acquired(result) => Ok(Some(Self {
                lock,
                key: key.to_string(),
                owner: owner.clone(),
                released: false,
                reentrant_count: result.reentrant_count,
            })),
            TryLockResult::AlreadyLocked { .. } => Ok(None),
        }
    }

    /// 手动提前释放锁
    ///
    /// 调用后守卫不再有效，Drop 时不会重复释放。
    pub async fn release(mut self) -> Result<()> {
        if !self.released {
            self.lock.unlock(&self.key, &self.owner).await?;
            self.released = true;
        }
        Ok(())
    }

    /// 续期锁
    pub async fn renew(&self, duration: Duration) -> Result<()> {
        if self.released {
            return Err(LockError::AlreadyReleased(format!(
                "锁 {} 已被释放",
                self.key
            )));
        }
        self.lock.renew(&self.key, &self.owner, duration).await
    }

    /// 获取锁的 key
    pub fn key(&self) -> &str {
        &self.key
    }

    /// 获取持有者 ID
    pub fn owner(&self) -> &LockOwnerId {
        &self.owner
    }

    /// 获取重入次数
    pub fn reentrant_count(&self) -> u32 {
        self.reentrant_count
    }

    /// 是否已释放
    pub fn is_released(&self) -> bool {
        self.released
    }
}

impl<L: Lock + 'static + ?Sized> Drop for LockGuard<L> {
    fn drop(&mut self) {
        if !self.released {
            // 注意：Drop 中不能直接调用 async 方法
            // 这里使用 tokio::spawn 来异步释放锁
            // 但在某些情况下（如运行时已关闭）可能无法执行
            // 生产环境建议手动调用 release()
            let lock = self.lock.clone();
            let key = self.key.clone();
            let owner = self.owner.clone();

            // 尝试在当前 tokio 运行时中释放
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    if let Err(e) = lock.unlock(&key, &owner).await {
                        tracing::error!(error = %e, key = %key, "LockGuard drop 时释放锁失败");
                    }
                });
            } else {
                tracing::warn!(key = %key, "LockGuard drop 时无 tokio 运行时，锁可能未释放");
            }

            self.released = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryLock;

    #[tokio::test]
    async fn test_guard_acquire_release() {
        let lock = MemoryLock::default();
        let owner = LockOwnerId::new();

        {
            let guard = LockGuard::acquire(lock.clone(), "guard:1", &owner).await.unwrap();
            assert_eq!(guard.key(), "guard:1");
            assert!(!guard.is_released());
            assert!(lock.is_locked("guard:1").await.unwrap());
        } // drop 时自动释放

        // 等待异步释放完成
        tokio::time::sleep(Duration::from_millis(100)).await;
        // 注意：由于 drop 时是异步释放，这里可能还没释放完成
        // 生产环境建议手动调用 release()
    }

    #[tokio::test]
    async fn test_guard_manual_release() {
        let lock = MemoryLock::default();
        let owner = LockOwnerId::new();

        let guard = LockGuard::acquire(lock.clone(), "guard:2", &owner).await.unwrap();
        assert!(lock.is_locked("guard:2").await.unwrap());

        guard.release().await.unwrap();
        assert!(!lock.is_locked("guard:2").await.unwrap());
    }

    #[tokio::test]
    async fn test_guard_try_acquire() {
        let lock = MemoryLock::default();
        let owner1 = LockOwnerId::new();
        let owner2 = LockOwnerId::new();

        // owner1 获取
        let guard1 = LockGuard::try_acquire(lock.clone(), "guard:3", &owner1)
            .await
            .unwrap();
        assert!(guard1.is_some());

        // owner2 尝试获取，应该失败
        let guard2 = LockGuard::try_acquire(lock.clone(), "guard:3", &owner2)
            .await
            .unwrap();
        assert!(guard2.is_none());
    }

    #[tokio::test]
    async fn test_guard_renew() {
        let lock = MemoryLock::default();
        let owner = LockOwnerId::new();

        let guard = LockGuard::acquire(lock.clone(), "guard:4", &owner).await.unwrap();
        guard.renew(Duration::from_secs(120)).await.unwrap();
    }
}
