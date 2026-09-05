// =============================================================================
// 多级缓存（MultiCache）— L1 内存 + L2 Redis
// =============================================================================
//
// 自动穿透回填：L1 未命中 → 查 L2 → L2 命中则回填 L1 → L2 未命中 → 回源。
// 写操作：同时写 L1 + L2（write-through）。
// 失效：同时失效 L1 + L2。
// =============================================================================

use crate::{Cache, CacheError, CacheResult, CacheStats, CacheValue};
use async_trait::async_trait;
use std::sync::Arc;

/// 多级缓存：L1（内存）+ L2（Redis/远程）
pub struct MultiCache {
    name: String,
    l1: Arc<dyn Cache>,
    l2: Option<Arc<dyn Cache>>,
}

impl MultiCache {
    /// 创建多级缓存
    pub fn new(name: impl Into<String>, l1: Arc<dyn Cache>, l2: Option<Arc<dyn Cache>>) -> Self {
        Self { name: name.into(), l1, l2 }
    }

    /// 仅 L1（便捷构造）
    pub fn l1_only(name: impl Into<String>, l1: Arc<dyn Cache>) -> Self {
        Self { name: name.into(), l1, l2: None }
    }
}

#[async_trait]
impl Cache for MultiCache {
    async fn get(&self, key: &str) -> CacheResult<Option<CacheValue>> {
        // L1 查找
        if let Some(v) = self.l1.get(key).await? {
            return Ok(Some(v));
        }
        // L2 查找
        if let Some(l2) = &self.l2 {
            if let Some(v) = l2.get(key).await? {
                // 回填 L1
                let _ = self.l1.set(key, v.clone()).await;
                return Ok(Some(v));
            }
        }
        Ok(None)
    }

    async fn set(&self, key: &str, value: CacheValue) -> CacheResult<()> {
        // write-through：同时写 L1 + L2
        self.l1.set(key, value.clone()).await?;
        if let Some(l2) = &self.l2 {
            let _ = l2.set(key, value).await;
        }
        Ok(())
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        self.l1.delete(key).await?;
        if let Some(l2) = &self.l2 {
            let _ = l2.delete(key).await;
        }
        Ok(())
    }

    async fn invalidate_prefix(&self, prefix: &str) -> CacheResult<usize> {
        let mut total = self.l1.invalidate_prefix(prefix).await?;
        if let Some(l2) = &self.l2 {
            total += l2.invalidate_prefix(prefix).await.unwrap_or(0);
        }
        Ok(total)
    }

    async fn clear(&self) -> CacheResult<()> {
        self.l1.clear().await?;
        if let Some(l2) = &self.l2 {
            let _ = l2.clear().await;
        }
        Ok(())
    }

    fn stats(&self) -> CacheStats {
        // 聚合 L1 + L2 统计
        let mut s = self.l1.stats();
        s.name = self.name.clone();
        if let Some(l2) = &self.l2 {
            let l2s = l2.stats();
            s.hits += l2s.hits;
            s.misses += l2s.misses;
            s.evictions += l2s.evictions;
            s.total_ops += l2s.total_ops;
            s.total_latency_ns += l2s.total_latency_ns;
        }
        s
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryCache;
    use std::time::Duration;

    #[tokio::test]
    async fn test_l1_only() {
        let l1 = Arc::new(MemoryCache::new("l1", 100));
        let cache = MultiCache::l1_only("multi", l1.clone());
        cache.set("k", CacheValue::new(b"v".to_vec(), None)).await.unwrap();
        assert!(cache.get("k").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_l2_backfill() {
        let l1 = Arc::new(MemoryCache::new("l1", 100));
        let l2 = Arc::new(MemoryCache::new("l2", 100));
        // 直接写入 L2
        l2.set("k", CacheValue::new(b"v".to_vec(), None)).await.unwrap();
        let cache = MultiCache::new("multi", l1.clone(), Some(l2));
        // L1 未命中，L2 命中，应回填 L1
        let got = cache.get("k").await.unwrap();
        assert!(got.is_some());
        // L1 现在应该有了
        assert!(l1.get("k").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_write_through() {
        let l1 = Arc::new(MemoryCache::new("l1", 100));
        let l2 = Arc::new(MemoryCache::new("l2", 100));
        let cache = MultiCache::new("multi", l1.clone(), Some(l2.clone()));
        cache.set("k", CacheValue::new(b"v".to_vec(), None)).await.unwrap();
        assert!(l1.get("k").await.unwrap().is_some());
        assert!(l2.get("k").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_miss_both() {
        let l1 = Arc::new(MemoryCache::new("l1", 100));
        let l2 = Arc::new(MemoryCache::new("l2", 100));
        let cache = MultiCache::new("multi", l1, Some(l2));
        assert!(cache.get("nonexistent").await.unwrap().is_none());
    }
}
