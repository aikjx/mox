// =============================================================================
// 内存 LRU 缓存后端（MemoryCache）
// =============================================================================
//
// 基于 parking_lot::RwLock + 自定义 LRU 的高性能本地缓存。
// 支持：容量上限、TTL 过期、LRU 淘汰、空值防穿透、统计指标。
// 适用于 L1 级缓存（单进程内，纳秒级访问）。
// =============================================================================

use crate::{Cache, CacheError, CacheResult, CacheStats, CacheValue};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// LRU 节点包装
struct LruEntry {
    value: CacheValue,
    /// 最后访问时间（用于 LRU 淘汰）
    last_access: Instant,
    /// 插入序号（用于稳定排序）
    seq: u64,
}

/// 内存 LRU 缓存
pub struct MemoryCache {
    name: String,
    /// 最大容量（条目数），0 表示不限制
    max_capacity: usize,
    inner: RwLock<HashMap<String, LruEntry>>,
    /// 全局递增序号
    seq_counter: AtomicU64,
    // 统计指标
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    total_latency_ns: AtomicU64,
    total_ops: AtomicU64,
}

impl MemoryCache {
    /// 创建内存缓存
    pub fn new(name: impl Into<String>, max_capacity: usize) -> Self {
        Self {
            name: name.into(),
            max_capacity,
            inner: RwLock::new(HashMap::new()),
            seq_counter: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
            total_ops: AtomicU64::new(0),
        }
    }

    /// 创建无容量限制的内存缓存
    pub fn unbounded(name: impl Into<String>) -> Self {
        Self::new(name, 0)
    }

    fn record_op(&self, latency_ns: u64) {
        self.total_ops.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ns.fetch_add(latency_ns, Ordering::Relaxed);
    }

    /// 清理过期条目（惰性清理，在 set 时触发）
    fn evict_if_needed(&self, map: &mut HashMap<String, LruEntry>) {
        if self.max_capacity == 0 || map.len() < self.max_capacity {
            return;
        }
        // 先清理过期的
        let expired: Vec<String> = map
            .iter()
            .filter(|(_, v)| v.value.is_expired())
            .map(|(k, _)| k.clone())
            .collect();
        for k in &expired {
            map.remove(k);
        }
        self.evictions.fetch_add(expired.len() as u64, Ordering::Relaxed);

        // 如果还超容量，按 LRU 淘汰
        while map.len() >= self.max_capacity {
            if let Some((oldest_key, _)) = map
                .iter()
                .min_by_key(|(_, v)| (v.last_access, v.seq))
                .map(|(k, v)| (k.clone(), v.seq))
            {
                map.remove(&oldest_key);
                self.evictions.fetch_add(1, Ordering::Relaxed);
            } else {
                break;
            }
        }
    }
}

#[async_trait]
impl Cache for MemoryCache {
    async fn get(&self, key: &str) -> CacheResult<Option<CacheValue>> {
        let start = Instant::now();
        if key.is_empty() {
            return Err(CacheError::EmptyKey);
        }
        let mut map = self.inner.write();
        let result = match map.get_mut(key) {
            Some(entry) => {
                if entry.value.is_expired() {
                    map.remove(key);
                    self.misses.fetch_add(1, Ordering::Relaxed);
                    None
                } else {
                    entry.last_access = Instant::now();
                    self.hits.fetch_add(1, Ordering::Relaxed);
                    Some(entry.value.clone())
                }
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        };
        self.record_op(start.elapsed().as_nanos() as u64);
        Ok(result)
    }

    async fn set(&self, key: &str, value: CacheValue) -> CacheResult<()> {
        let start = Instant::now();
        if key.is_empty() {
            return Err(CacheError::EmptyKey);
        }
        let mut map = self.inner.write();
        let seq = self.seq_counter.fetch_add(1, Ordering::Relaxed);
        map.insert(
            key.to_string(),
            LruEntry { value, last_access: Instant::now(), seq },
        );
        self.evict_if_needed(&mut map);
        self.record_op(start.elapsed().as_nanos() as u64);
        Ok(())
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        let start = Instant::now();
        if key.is_empty() {
            return Err(CacheError::EmptyKey);
        }
        let mut map = self.inner.write();
        map.remove(key);
        self.record_op(start.elapsed().as_nanos() as u64);
        Ok(())
    }

    async fn invalidate_prefix(&self, prefix: &str) -> CacheResult<usize> {
        let start = Instant::now();
        let mut map = self.inner.write();
        let before = map.len();
        map.retain(|k, _| !k.starts_with(prefix));
        let removed = before - map.len();
        self.record_op(start.elapsed().as_nanos() as u64);
        Ok(removed)
    }

    async fn clear(&self) -> CacheResult<()> {
        let mut map = self.inner.write();
        map.clear();
        Ok(())
    }

    fn stats(&self) -> CacheStats {
        CacheStats {
            name: self.name.clone(),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            total_ops: self.total_ops.load(Ordering::Relaxed),
            total_latency_ns: self.total_latency_ns.load(Ordering::Relaxed),
            entry_count: self.inner.read().len() as u64,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_set_and_get() {
        let cache = MemoryCache::new("test", 100);
        let val = CacheValue::new(b"hello".to_vec(), Some(Duration::from_secs(60)));
        cache.set("key1", val).await.unwrap();
        let got = cache.get("key1").await.unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().data.unwrap(), b"hello");
    }

    #[tokio::test]
    async fn test_miss() {
        let cache = MemoryCache::new("test", 100);
        let got = cache.get("nonexistent").await.unwrap();
        assert!(got.is_none());
        assert_eq!(cache.stats().misses, 1);
    }

    #[tokio::test]
    async fn test_delete() {
        let cache = MemoryCache::new("test", 100);
        cache.set("key1", CacheValue::new(b"v".to_vec(), None)).await.unwrap();
        cache.delete("key1").await.unwrap();
        assert!(cache.get("key1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let cache = MemoryCache::new("test", 2);
        cache.set("k1", CacheValue::new(b"1".to_vec(), None)).await.unwrap();
        cache.set("k2", CacheValue::new(b"2".to_vec(), None)).await.unwrap();
        // 访问 k1，使其变为最近使用
        cache.get("k1").await.unwrap();
        // 插入 k3，应淘汰 k2（最久未使用）
        cache.set("k3", CacheValue::new(b"3".to_vec(), None)).await.unwrap();
        assert!(cache.get("k2").await.unwrap().is_none());
        assert!(cache.get("k1").await.unwrap().is_some());
        assert!(cache.get("k3").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_invalidate_prefix() {
        let cache = MemoryCache::new("test", 100);
        cache.set("dsql:q1", CacheValue::new(b"1".to_vec(), None)).await.unwrap();
        cache.set("dsql:q2", CacheValue::new(b"2".to_vec(), None)).await.unwrap();
        cache.set("other:k1", CacheValue::new(b"3".to_vec(), None)).await.unwrap();
        let removed = cache.invalidate_prefix("dsql:").await.unwrap();
        assert_eq!(removed, 2);
        assert!(cache.get("dsql:q1").await.unwrap().is_none());
        assert!(cache.get("other:k1").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_empty_key_error() {
        let cache = MemoryCache::new("test", 100);
        assert!(cache.get("").await.is_err());
        assert!(cache.set("", CacheValue::new(b"x".to_vec(), None)).await.is_err());
    }
}
