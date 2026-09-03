// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 对象数据缓存（阶段3，feature `erasure`）——加权 LRU + singleflight，全自研。
//!
//! [`ObjectCache`] 是 [`ObjectStore`] 装饰器：按**字节加权容量**做 LRU 驱逐，
//! 并用 **singleflight** 合并并发读（同一 key 并发未命中只落一次底层）。
//!
//! - 加权：`current_bytes` 累计缓存对象字节，超容量按 LRU 顺序驱逐。
//! - singleflight：`inflight` 表挂起并发读者，完成者广播结果（oneshot 多消费者）。
//! - 命中/未命中/驱逐计数供监控。

use crate::StoreResult;
use async_trait::async_trait;
use bytes::Bytes;
use mox_base_store_core::{BlobObject, ObjectStore};
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::oneshot;

/// 缓存配置。
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 总容量（字节），0 = 禁用缓存。
    pub capacity_bytes: usize,
    /// 单个对象最大可缓存字节（防大对象挤占）。
    pub max_entry_bytes: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            capacity_bytes: 64 * 1024 * 1024, // 64 MiB
            max_entry_bytes: 4 * 1024 * 1024, // 4 MiB
        }
    }
}

struct CacheEntry {
    data: Bytes,
    size: usize,
}

/// 加权 LRU + singleflight 对象缓存装饰器。
pub struct ObjectCache {
    inner: Arc<dyn ObjectStore>,
    capacity_bytes: usize,
    max_entry_bytes: usize,
    map: Mutex<HashMap<String, CacheEntry>>,
    /// LRU 顺序（front = 最近使用）
    order: Mutex<VecDeque<String>>,
    current_bytes: std::sync::atomic::AtomicUsize,
    /// singleflight：path -> 等待者集合
    inflight: Mutex<HashMap<String, Vec<oneshot::Sender<Option<Bytes>>>>>,
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
}

impl ObjectCache {
    /// 包装底层存储。
    pub fn new(inner: Arc<dyn ObjectStore>, cfg: CacheConfig) -> Self {
        Self {
            inner,
            capacity_bytes: cfg.capacity_bytes,
            max_entry_bytes: cfg.max_entry_bytes,
            map: Mutex::new(HashMap::new()),
            order: Mutex::new(VecDeque::new()),
            current_bytes: std::sync::atomic::AtomicUsize::new(0),
            inflight: Mutex::new(HashMap::new()),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }

    /// 底层存储引用。
    pub fn inner(&self) -> &Arc<dyn ObjectStore> {
        &self.inner
    }

    /// 命中/未命中/驱逐统计。
    pub fn stats(&self) -> (u64, u64, u64) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
            self.evictions.load(Ordering::Relaxed),
        )
    }

    fn capacity(&self) -> usize {
        self.capacity_bytes
    }

    /// 缓存查找 + LRU 触碰（命中则移到 front）。
    fn lookup(&self, path: &str) -> Option<Bytes> {
        let map = self.map.lock();
        let e = map.get(path)?;
        let data = e.data.clone();
        // LRU 触碰：移到 front
        let mut order = self.order.lock();
        if let Some(pos) = order.iter().position(|k| k == path) {
            let k = order.remove(pos).unwrap();
            order.push_front(k);
        }
        drop(order);
        drop(map);
        Some(data)
    }

    /// 插入缓存（含驱逐）。
    fn insert(&self, path: &str, data: Bytes) {
        if self.capacity() == 0 || data.len() > self.max_entry_bytes {
            return;
        }
        let size = data.len();
        let mut map = self.map.lock();
        let mut order = self.order.lock();
        // 已存在 → 先移除旧尺寸
        if let Some(old) = map.remove(path) {
            self.current_bytes.fetch_sub(old.size, Ordering::Relaxed);
            if let Some(pos) = order.iter().position(|k| k == path) {
                order.remove(pos);
            }
        }
        map.insert(path.to_string(), CacheEntry { data, size });
        order.push_front(path.to_string());
        self.current_bytes.fetch_add(size, Ordering::Relaxed);
        // 驱逐：超容量从尾部（最久未用）移除
        while self.current_bytes.load(Ordering::Relaxed) > self.capacity() && !order.is_empty() {
            if let Some(lru) = order.pop_back() {
                if let Some(e) = map.remove(&lru) {
                    self.current_bytes.fetch_sub(e.size, Ordering::Relaxed);
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    /// 底层读 + 缓存（singleflight 合并）。
    async fn fetch(&self, path: &str) -> StoreResult<Bytes> {
        // 注册为 singleflight 等待者：已有人在途 → 挂起；否则成为 leader
        let rx = {
            let mut inf = self.inflight.lock();
            if inf.contains_key(path) {
                let (tx, rx) = oneshot::channel();
                inf.get_mut(path).unwrap().push(tx);
                drop(inf);
                Some(rx)
            } else {
                inf.insert(path.to_string(), Vec::new());
                drop(inf);
                None
            }
        };
        if let Some(rx) = rx {
            // 已在途：等广播
            match rx.await {
                Ok(Some(b)) => return Ok(b),
                _ => {
                    // 完成者失败或已取消 → 自行取
                    let res = self.inner.get(path).await;
                    if let Ok(b) = &res {
                        self.insert(path, b.clone());
                    }
                    return res;
                }
            }
        }
        // 首个请求者：真正读底层
        self.misses.fetch_add(1, Ordering::Relaxed);
        let res = self.inner.get(path).await;
        let broadcast: Option<Bytes> = match &res {
            Ok(b) => {
                self.insert(path, b.clone());
                Some(b.clone())
            }
            Err(_) => None,
        };
        // 广播给等待者
        let waiters = {
            let mut inf = self.inflight.lock();
            inf.remove(path).unwrap_or_default()
        };
        for tx in waiters {
            let _ = tx.send(broadcast.clone());
        }
        res
    }
}

#[async_trait]
impl ObjectStore for ObjectCache {
    async fn put(&self, path: &str, content_type: &str, data: Bytes) -> StoreResult<BlobObject> {
        let res = self.inner.put(path, content_type, data.clone()).await?;
        self.insert(path, data);
        Ok(res)
    }

    async fn get(&self, path: &str) -> StoreResult<Bytes> {
        if let Some(data) = self.lookup(path) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(data);
        }
        self.fetch(path).await
    }

    async fn get_range(&self, path: &str, offset: u64, length: u64) -> StoreResult<Bytes> {
        let full = self.get(path).await?;
        let start = offset as usize;
        let end = std::cmp::min(start + length as usize, full.len());
        if start >= full.len() {
            return Ok(Bytes::new());
        }
        Ok(full.slice(start..end))
    }

    async fn delete(&self, path: &str) -> StoreResult<()> {
        // 先摘除缓存（块作用域结束即释放 guard，避免跨 await 持有非 Send 状态）
        {
            let mut map = self.map.lock();
            let mut order = self.order.lock();
            if let Some(e) = map.remove(path) {
                self.current_bytes.fetch_sub(e.size, Ordering::Relaxed);
                if let Some(pos) = order.iter().position(|k| k == path) {
                    order.remove(pos);
                }
            }
        }
        // 取消在途等待
        let waiters = self.inflight.lock().remove(path).unwrap_or_default();
        for tx in waiters {
            let _ = tx.send(None);
        }
        self.inner.delete(path).await
    }

    async fn head(&self, path: &str) -> StoreResult<BlobObject> {
        self.inner.head(path).await
    }

    async fn exists(&self, path: &str) -> StoreResult<bool> {
        self.inner.exists(path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_backend::FsObjectStore;
    use std::path::Path;

    fn base(dir: &Path) -> Arc<dyn ObjectStore> {
        Arc::new(FsObjectStore::new(dir.to_path_buf()).unwrap())
    }

    fn cache(dir: &Path, cap: usize) -> ObjectCache {
        ObjectCache::new(
            base(dir),
            CacheConfig {
                capacity_bytes: cap,
                max_entry_bytes: cap.max(1) * 4,
            },
        )
    }

    #[tokio::test]
    async fn hit_after_put() {
        let dir = tempfile::tempdir().unwrap();
        let c = cache(dir.path(), 1024 * 1024);
        c.put("a", "text/plain", Bytes::from_static(b"hello")).await.unwrap();
        assert_eq!(&c.get("a").await.unwrap()[..], b"hello");
        let (hits, misses, _) = c.stats();
        assert!(hits >= 1);
        assert_eq!(misses, 0);
    }

    #[tokio::test]
    async fn miss_then_fetch() {
        let dir = tempfile::tempdir().unwrap();
        let c = cache(dir.path(), 1024 * 1024);
        // 直接底层写入，绕过缓存
        c.inner().put("b", "text/plain", Bytes::from_static(b"data")).await.unwrap();
        assert_eq!(&c.get("b").await.unwrap()[..], b"data");
        let (_, misses, _) = c.stats();
        assert_eq!(misses, 1);
        // 二次命中
        assert_eq!(&c.get("b").await.unwrap()[..], b"data");
        let (hits2, _, _) = c.stats();
        assert!(hits2 >= 1);
    }

    #[tokio::test]
    async fn lru_eviction_respects_weighted_capacity() {
        let dir = tempfile::tempdir().unwrap();
        // 容量 100 字节，max_entry 400
        let c = cache(dir.path(), 100);
        c.put("a", "x", Bytes::from(vec![1u8; 60])).await.unwrap();
        c.put("b", "x", Bytes::from(vec![2u8; 60])).await.unwrap();
        // 总 120 > 100 → 驱逐一个（a 最久未用）
        assert!(c.lookup("a").is_none());
        assert!(c.lookup("b").is_some());
        let (_, _, ev) = c.stats();
        assert!(ev >= 1);
    }

    #[tokio::test]
    async fn delete_clears_cache() {
        let dir = tempfile::tempdir().unwrap();
        let c = cache(dir.path(), 1024 * 1024);
        c.put("a", "x", Bytes::from_static(b"hello")).await.unwrap();
        assert!(c.lookup("a").is_some());
        c.delete("a").await.unwrap();
        assert!(c.lookup("a").is_none());
    }

    #[tokio::test]
    async fn concurrent_reads_singleflight() {
        let dir = tempfile::tempdir().unwrap();
        let c = Arc::new(cache(dir.path(), 1024 * 1024));
        c.inner().put("hot", "x", Bytes::from(vec![9u8; 4096])).await.unwrap();
        // 并发 8 读同一 key
        let mut handles = Vec::new();
        for _ in 0..8 {
            let c2 = c.clone();
            handles.push(tokio::spawn(async move {
                let d = c2.get("hot").await.unwrap();
                assert_eq!(d.len(), 4096);
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        let (_, misses, _) = c.stats();
        // singleflight：底层只 miss 1 次
        assert_eq!(misses, 1);
    }
}
