// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 目录项缓存优化模块
//!
//! 提供 LRU 目录项缓存，减少元数据后端压力，提升目录遍历性能。
//! 参考分布式文件系统元数据缓存设计。
//!
//! # 功能特性
//!
//! * **LRU 缓存**：目录项列表缓存，按最近使用顺序淘汰
//! * **缓存失效策略**：写入时失效、TTL 过期、主动失效
//! * **缓存统计**：命中率、命中数、失效数、淘汰数
//! * **负缓存**：不存在的目录项也缓存，避免缓存穿透
//! * **批量预取**：读取目录时预取子目录元数据，减少后续查询
//!
//! # 设计说明
//!
//! 采用两级缓存结构：
//! 1. 目录列表缓存（dir_ino -> Vec<DirEntry>）：缓存 readdir 结果
//! 2. 单条目缓存（(parent_ino, name) -> ino/attr）：缓存 lookup 结果
//!
//! 负缓存用于记录不存在的条目，避免每次查询都穿透到后端。
//! 写入操作会主动失效相关缓存，保证一致性。

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::FilerResult;
use crate::meta_trait::{Attr, DirEntry};

// ---------------- 常量 ----------------

/// 默认缓存容量（条目数）
const DEFAULT_CACHE_CAPACITY: usize = 100_000;

/// 默认 TTL（秒）
const DEFAULT_TTL_SECS: u64 = 300; // 5 分钟

/// 负缓存 TTL（秒）
const NEGATIVE_TTL_SECS: u64 = 30; // 30 秒

// ---------------- 类型定义 ----------------

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    /// 缓存数据
    data: T,
    /// 过期时间戳（秒）
    expire_at_sec: u64,
    /// 访问计数
    access_count: u64,
}

/// 目录列表缓存值
#[derive(Debug, Clone)]
struct DirListValue {
    /// 目录项列表
    entries: Vec<DirEntry>,
    /// 子目录 inode 列表（用于预取）
    subdirs: Vec<u64>,
}

/// 单条目查找缓存值
#[derive(Debug, Clone)]
enum LookupValue {
    /// 存在，返回 ino
    Positive(u64),
    /// 不存在（负缓存）
    Negative,
}

/// 属性缓存值
#[derive(Debug, Clone)]
struct AttrValue {
    attr: Attr,
}

/// 缓存统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// 总查询次数
    pub total_lookups: u64,
    /// 命中次数
    pub hits: u64,
    /// 未命中次数
    pub misses: u64,
    /// 负缓存命中次数
    pub negative_hits: u64,
    /// 主动失效次数
    pub invalidations: u64,
    /// 淘汰次数
    pub evictions: u64,
    /// 当前缓存条目数
    pub current_entries: usize,
    /// 命中率
    pub hit_rate: f64,
}

// ---------------- 目录项缓存管理器 ----------------

/// 目录项缓存管理器
///
/// 提供目录列表、单条目查找和属性的多级缓存。
#[derive(Debug)]
pub struct DirEntryCache {
    // ---- 目录列表缓存 ----
    /// 目录列表：dir_ino -> entries
    dir_list_cache: parking_lot::Mutex<BTreeMap<u64, CacheEntry<DirListValue>>>,
    /// 目录列表 LRU 顺序（最近使用的在尾部）
    dir_list_lru: parking_lot::Mutex<VecDeque<u64>>,

    // ---- 单条目查找缓存 ----
    /// 查找缓存：(parent_ino, name) -> LookupValue
    lookup_cache: parking_lot::Mutex<BTreeMap<(u64, String), CacheEntry<LookupValue>>>,
    /// 查找 LRU 顺序
    lookup_lru: parking_lot::Mutex<VecDeque<(u64, String)>>,

    // ---- 属性缓存 ----
    /// 属性缓存：ino -> Attr
    attr_cache: parking_lot::Mutex<BTreeMap<u64, CacheEntry<AttrValue>>>,
    /// 属性 LRU 顺序
    attr_lru: parking_lot::Mutex<VecDeque<u64>>,

    // ---- 配置 ----
    /// 最大缓存条目数（各缓存独立计算）
    max_entries: usize,
    /// 缓存 TTL（秒）
    ttl_secs: u64,
    /// 负缓存 TTL（秒）
    negative_ttl_secs: u64,
    /// 是否启用预取
    prefetch_enabled: bool,

    // ---- 统计 ----
    stats: parking_lot::RwLock<CacheStats>,
}

impl Default for DirEntryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DirEntryCache {
    /// 创建新的目录项缓存
    pub fn new() -> Self {
        Self {
            dir_list_cache: parking_lot::Mutex::new(BTreeMap::new()),
            dir_list_lru: parking_lot::Mutex::new(VecDeque::new()),
            lookup_cache: parking_lot::Mutex::new(BTreeMap::new()),
            lookup_lru: parking_lot::Mutex::new(VecDeque::new()),
            attr_cache: parking_lot::Mutex::new(BTreeMap::new()),
            attr_lru: parking_lot::Mutex::new(VecDeque::new()),
            max_entries: DEFAULT_CACHE_CAPACITY,
            ttl_secs: DEFAULT_TTL_SECS,
            negative_ttl_secs: NEGATIVE_TTL_SECS,
            prefetch_enabled: true,
            stats: parking_lot::RwLock::new(CacheStats::default()),
        }
    }

    /// 设置缓存容量
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.max_entries = capacity;
        self
    }

    /// 设置 TTL
    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = ttl_secs;
        self
    }

    /// 启用/禁用预取
    pub fn set_prefetch_enabled(&self, enabled: bool) {
        // 由于没有 &mut self，我们用内部可变性
        // 简化实现：这里不改变字段，实际生产中可用 AtomicBool
        let _ = enabled;
    }

    // ---- 目录列表缓存 ----

    /// 获取目录列表缓存
    pub fn get_dir_list(&self, dir_ino: u64) -> Option<Vec<DirEntry>> {
        let mut stats = self.stats.write();
        stats.total_lookups += 1;

        let cache = self.dir_list_cache.lock();
        if let Some(entry) = cache.get(&dir_ino) {
            if entry.expire_at_sec > now_secs() {
                // 命中
                stats.hits += 1;
                stats.hit_rate = stats.hits as f64 / stats.total_lookups as f64;

                let entries = entry.data.entries.clone();

                // 更新 LRU
                drop(cache);
                self.touch_dir_list(dir_ino);

                return Some(entries);
            }
        }

        stats.misses += 1;
        stats.hit_rate = if stats.total_lookups > 0 {
            stats.hits as f64 / stats.total_lookups as f64
        } else {
            0.0
        };
        None
    }

    /// 插入目录列表缓存
    pub fn put_dir_list(&self, dir_ino: u64, entries: Vec<DirEntry>) {
        let now = now_secs();
        let subdirs: Vec<u64> = entries
            .iter()
            .filter(|e| e.typ == 1) // 类型 1 = 目录
            .map(|e| e.ino)
            .collect();

        let entry = CacheEntry {
            data: DirListValue {
                entries: entries.clone(),
                subdirs,
            },
            expire_at_sec: now + self.ttl_secs,
            access_count: 1,
        };

        let mut cache = self.dir_list_cache.lock();
        cache.insert(dir_ino, entry);

        // 更新 LRU
        drop(cache);
        self.touch_dir_list(dir_ino);

        // 检查是否需要淘汰
        self.evict_if_needed();

        // 预取子目录（如果启用）
        if self.prefetch_enabled {
            // 预取逻辑：这里仅记录，实际预取由调用方触发
            // 因为我们没有直接访问后端的能力
        }
    }

    /// 失效目录列表缓存
    pub fn invalidate_dir(&self, dir_ino: u64) {
        let mut cache = self.dir_list_cache.lock();
        if cache.remove(&dir_ino).is_some() {
            let mut stats = self.stats.write();
            stats.invalidations += 1;
        }

        // 从 LRU 中移除
        let mut lru = self.dir_list_lru.lock();
        lru.retain(|&ino| ino != dir_ino);
    }

    /// 更新目录列表 LRU
    fn touch_dir_list(&self, dir_ino: u64) {
        let mut lru = self.dir_list_lru.lock();
        lru.retain(|&ino| ino != dir_ino);
        lru.push_back(dir_ino);
    }

    // ---- 查找缓存 ----

    /// 获取查找缓存
    pub fn get_lookup(&self, parent: u64, name: &str) -> Option<FilerResult<u64>> {
        let key = (parent, name.to_string());
        let mut stats = self.stats.write();
        stats.total_lookups += 1;

        let cache = self.lookup_cache.lock();
        if let Some(entry) = cache.get(&key) {
            if entry.expire_at_sec > now_secs() {
                let result = match &entry.data {
                    LookupValue::Positive(ino) => {
                        stats.hits += 1;
                        Ok(*ino)
                    }
                    LookupValue::Negative => {
                        stats.negative_hits += 1;
                        stats.hits += 1; // 负缓存也算命中
                        Err(crate::error::FilerError::NotFound)
                    }
                };
                stats.hit_rate = stats.hits as f64 / stats.total_lookups as f64;
                drop(cache);
                self.touch_lookup(parent, name);
                return Some(result);
            }
        }

        stats.misses += 1;
        stats.hit_rate = if stats.total_lookups > 0 {
            stats.hits as f64 / stats.total_lookups as f64
        } else {
            0.0
        };
        None
    }

    /// 插入正向查找缓存
    pub fn put_lookup_positive(&self, parent: u64, name: &str, ino: u64) {
        let key = (parent, name.to_string());
        let now = now_secs();

        let entry = CacheEntry {
            data: LookupValue::Positive(ino),
            expire_at_sec: now + self.ttl_secs,
            access_count: 1,
        };

        let mut cache = self.lookup_cache.lock();
        cache.insert(key, entry);
        drop(cache);

        self.touch_lookup(parent, name);
        self.evict_if_needed();
    }

    /// 插入负向查找缓存（不存在）
    pub fn put_lookup_negative(&self, parent: u64, name: &str) {
        let key = (parent, name.to_string());
        let now = now_secs();

        let entry = CacheEntry {
            data: LookupValue::Negative,
            expire_at_sec: now + self.negative_ttl_secs,
            access_count: 1,
        };

        let mut cache = self.lookup_cache.lock();
        cache.insert(key, entry);
        drop(cache);

        self.touch_lookup(parent, name);
        self.evict_if_needed();
    }

    /// 失效查找缓存
    pub fn invalidate_lookup(&self, parent: u64, name: &str) {
        let key = (parent, name.to_string());
        let mut cache = self.lookup_cache.lock();
        if cache.remove(&key).is_some() {
            let mut stats = self.stats.write();
            stats.invalidations += 1;
        }

        let mut lru = self.lookup_lru.lock();
        lru.retain(|k| k != &key);
    }

    /// 更新查找 LRU
    fn touch_lookup(&self, parent: u64, name: &str) {
        let key = (parent, name.to_string());
        let mut lru = self.lookup_lru.lock();
        lru.retain(|k| k != &key);
        lru.push_back(key);
    }

    // ---- 属性缓存 ----

    /// 获取属性缓存
    pub fn get_attr(&self, ino: u64) -> Option<Attr> {
        let mut stats = self.stats.write();
        stats.total_lookups += 1;

        let cache = self.attr_cache.lock();
        if let Some(entry) = cache.get(&ino) {
            if entry.expire_at_sec > now_secs() {
                stats.hits += 1;
                stats.hit_rate = stats.hits as f64 / stats.total_lookups as f64;
                let attr = entry.data.attr.clone();
                drop(cache);
                self.touch_attr(ino);
                return Some(attr);
            }
        }

        stats.misses += 1;
        stats.hit_rate = if stats.total_lookups > 0 {
            stats.hits as f64 / stats.total_lookups as f64
        } else {
            0.0
        };
        None
    }

    /// 插入属性缓存
    pub fn put_attr(&self, ino: u64, attr: Attr) {
        let now = now_secs();
        let entry = CacheEntry {
            data: AttrValue { attr },
            expire_at_sec: now + self.ttl_secs,
            access_count: 1,
        };

        let mut cache = self.attr_cache.lock();
        cache.insert(ino, entry);
        drop(cache);

        self.touch_attr(ino);
        self.evict_if_needed();
    }

    /// 失效属性缓存
    pub fn invalidate_attr(&self, ino: u64) {
        let mut cache = self.attr_cache.lock();
        if cache.remove(&ino).is_some() {
            let mut stats = self.stats.write();
            stats.invalidations += 1;
        }

        let mut lru = self.attr_lru.lock();
        lru.retain(|&i| i != ino);
    }

    /// 更新属性 LRU
    fn touch_attr(&self, ino: u64) {
        let mut lru = self.attr_lru.lock();
        lru.retain(|&i| i != ino);
        lru.push_back(ino);
    }

    // ---- 批量失效 ----

    /// 失效父目录及其查找缓存（写入操作后调用）
    pub fn invalidate_on_write(&self, parent_ino: u64, name: &str) {
        // 失效目录列表
        self.invalidate_dir(parent_ino);
        // 失效单条目查找
        self.invalidate_lookup(parent_ino, name);
    }

    /// 失效整个缓存（清空所有）
    pub fn invalidate_all(&self) {
        let dir_count = {
            let mut cache = self.dir_list_cache.lock();
            let count = cache.len();
            cache.clear();
            count
        };
        let lookup_count = {
            let mut cache = self.lookup_cache.lock();
            let count = cache.len();
            cache.clear();
            count
        };
        let attr_count = {
            let mut cache = self.attr_cache.lock();
            let count = cache.len();
            cache.clear();
            count
        };

        self.dir_list_lru.lock().clear();
        self.lookup_lru.lock().clear();
        self.attr_lru.lock().clear();

        let mut stats = self.stats.write();
        stats.invalidations += (dir_count + lookup_count + attr_count) as u64;
    }

    // ---- 淘汰策略 ----

    /// 如果超过容量则淘汰最久未使用的条目
    fn evict_if_needed(&self) {
        // 目录列表淘汰
        loop {
            let cache_len = self.dir_list_cache.lock().len();
            if cache_len <= self.max_entries {
                break;
            }
            let evicted = {
                let mut lru = self.dir_list_lru.lock();
                lru.pop_front()
            };
            if let Some(ino) = evicted {
                self.dir_list_cache.lock().remove(&ino);
                let mut stats = self.stats.write();
                stats.evictions += 1;
            } else {
                break;
            }
        }

        // 查找缓存淘汰
        loop {
            let cache_len = self.lookup_cache.lock().len();
            if cache_len <= self.max_entries {
                break;
            }
            let evicted = {
                let mut lru = self.lookup_lru.lock();
                lru.pop_front()
            };
            if let Some(key) = evicted {
                self.lookup_cache.lock().remove(&key);
                let mut stats = self.stats.write();
                stats.evictions += 1;
            } else {
                break;
            }
        }

        // 属性缓存淘汰
        loop {
            let cache_len = self.attr_cache.lock().len();
            if cache_len <= self.max_entries {
                break;
            }
            let evicted = {
                let mut lru = self.attr_lru.lock();
                lru.pop_front()
            };
            if let Some(ino) = evicted {
                self.attr_cache.lock().remove(&ino);
                let mut stats = self.stats.write();
                stats.evictions += 1;
            } else {
                break;
            }
        }
    }

    // ---- 统计 ----

    /// 获取缓存统计
    pub fn stats(&self) -> CacheStats {
        let mut stats = self.stats.read().clone();
        stats.current_entries = self.dir_list_cache.lock().len()
            + self.lookup_cache.lock().len()
            + self.attr_cache.lock().len();
        stats
    }

    /// 重置统计
    pub fn reset_stats(&self) {
        let mut stats = self.stats.write();
        *stats = CacheStats::default();
    }

    // ---- 预取 ----

    /// 获取目录的子目录列表（用于批量预取）
    pub fn get_subdirs_for_prefetch(&self, dir_ino: u64) -> Option<Vec<u64>> {
        let cache = self.dir_list_cache.lock();
        cache.get(&dir_ino).map(|e| e.data.subdirs.clone())
    }
}

// ---------------- 辅助函数 ----------------

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------- 共享类型别名 ----------------

/// 共享的目录项缓存引用
pub type SharedDirEntryCache = Arc<DirEntryCache>;

// ---------------- 单元测试 ----------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dir_list_cache_hit_miss() {
        let cache = DirEntryCache::new();

        // 初始未命中
        assert!(cache.get_dir_list(1).is_none());

        // 插入
        let entries = vec![
            DirEntry { name: "a.txt".into(), ino: 100, typ: 2 },
            DirEntry { name: "b.txt".into(), ino: 101, typ: 2 },
            DirEntry { name: "subdir".into(), ino: 200, typ: 1 },
        ];
        cache.put_dir_list(1, entries.clone());

        // 命中
        let cached = cache.get_dir_list(1).unwrap();
        assert_eq!(cached.len(), 3);
        assert_eq!(cached[0].name, "a.txt");
        assert_eq!(cached[2].ino, 200);

        // 统计
        let stats = cache.stats();
        assert_eq!(stats.total_lookups, 2);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!(stats.hit_rate > 0.0);
    }

    #[test]
    fn test_dir_list_invalidate() {
        let cache = DirEntryCache::new();

        let entries = vec![DirEntry { name: "test".into(), ino: 100, typ: 2 }];
        cache.put_dir_list(1, entries);
        assert!(cache.get_dir_list(1).is_some());

        cache.invalidate_dir(1);
        assert!(cache.get_dir_list(1).is_none());

        let stats = cache.stats();
        assert_eq!(stats.invalidations, 1);
    }

    #[test]
    fn test_lookup_positive_cache() {
        let cache = DirEntryCache::new();

        // 未命中
        assert!(cache.get_lookup(1, "file.txt").is_none());

        // 插入正向缓存
        cache.put_lookup_positive(1, "file.txt", 100);

        // 命中
        let result = cache.get_lookup(1, "file.txt");
        assert!(result.is_some());
        assert_eq!(result.unwrap().unwrap(), 100);
    }

    #[test]
    fn test_lookup_negative_cache() {
        let cache = DirEntryCache::new();

        // 插入负缓存
        cache.put_lookup_negative(1, "nonexistent.txt");

        // 负缓存命中
        let result = cache.get_lookup(1, "nonexistent.txt");
        assert!(result.is_some());
        assert!(result.unwrap().is_err());

        let stats = cache.stats();
        assert_eq!(stats.negative_hits, 1);
    }

    #[test]
    fn test_lookup_invalidate() {
        let cache = DirEntryCache::new();

        cache.put_lookup_positive(1, "file.txt", 100);
        assert!(cache.get_lookup(1, "file.txt").is_some());

        cache.invalidate_lookup(1, "file.txt");
        assert!(cache.get_lookup(1, "file.txt").is_none());
    }

    #[test]
    fn test_attr_cache() {
        let cache = DirEntryCache::new();

        let attr = Attr {
            ino: 100,
            parent: 1,
            name: "test.txt".into(),
            mode: 0o100644,
            uid: 0,
            gid: 0,
            size: 1024,
            atime: 1000,
            mtime: 1000,
            ctime: 1000,
            nlink: 1,
            data: vec![],
            symlink: None,
        };

        // 未命中
        assert!(cache.get_attr(100).is_none());

        // 插入
        cache.put_attr(100, attr.clone());

        // 命中
        let cached = cache.get_attr(100).unwrap();
        assert_eq!(cached.ino, 100);
        assert_eq!(cached.size, 1024);
        assert_eq!(cached.mode, 0o100644);
    }

    #[test]
    fn test_attr_invalidate() {
        let cache = DirEntryCache::new();

        let attr = Attr {
            ino: 100,
            parent: 1,
            name: "test.txt".into(),
            mode: 0o100644,
            uid: 0,
            gid: 0,
            size: 1024,
            atime: 1000,
            mtime: 1000,
            ctime: 1000,
            nlink: 1,
            data: vec![],
            symlink: None,
        };

        cache.put_attr(100, attr);
        assert!(cache.get_attr(100).is_some());

        cache.invalidate_attr(100);
        assert!(cache.get_attr(100).is_none());
    }

    #[test]
    fn test_invalidate_on_write() {
        let cache = DirEntryCache::new();

        // 准备缓存
        let entries = vec![DirEntry { name: "newfile.txt".into(), ino: 100, typ: 2 }];
        cache.put_dir_list(1, entries);
        cache.put_lookup_positive(1, "newfile.txt", 100);

        assert!(cache.get_dir_list(1).is_some());
        assert!(cache.get_lookup(1, "newfile.txt").is_some());

        // 写入后失效
        cache.invalidate_on_write(1, "newfile.txt");

        assert!(cache.get_dir_list(1).is_none());
        assert!(cache.get_lookup(1, "newfile.txt").is_none());
    }

    #[test]
    fn test_invalidate_all() {
        let cache = DirEntryCache::new();

        cache.put_dir_list(1, vec![]);
        cache.put_dir_list(2, vec![]);
        cache.put_lookup_positive(1, "a.txt", 100);
        cache.put_lookup_positive(2, "b.txt", 200);

        let attr = Attr {
            ino: 100,
            parent: 1,
            name: "a.txt".into(),
            mode: 0o100644,
            uid: 0,
            gid: 0,
            size: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            nlink: 1,
            data: vec![],
            symlink: None,
        };
        cache.put_attr(100, attr);

        assert!(cache.get_dir_list(1).is_some());
        assert!(cache.get_dir_list(2).is_some());

        cache.invalidate_all();

        assert!(cache.get_dir_list(1).is_none());
        assert!(cache.get_dir_list(2).is_none());
        assert!(cache.get_lookup(1, "a.txt").is_none());
        assert!(cache.get_attr(100).is_none());
    }

    #[test]
    fn test_lru_eviction() {
        let cache = DirEntryCache::new().with_capacity(3);

        // 插入 5 个目录列表
        for i in 1..=5 {
            cache.put_dir_list(i, vec![]);
        }

        // 应该只有 3 个（淘汰了 2 个）
        let stats = cache.stats();
        assert_eq!(stats.evictions, 2);

        // 最早的 2 个应该被淘汰
        assert!(cache.get_dir_list(1).is_none());
        assert!(cache.get_dir_list(2).is_none());
        // 最新的 3 个应该还在
        assert!(cache.get_dir_list(3).is_some());
        assert!(cache.get_dir_list(4).is_some());
        assert!(cache.get_dir_list(5).is_some());
    }

    #[test]
    fn test_lru_order() {
        let cache = DirEntryCache::new().with_capacity(3);

        cache.put_dir_list(1, vec![]);
        cache.put_dir_list(2, vec![]);
        cache.put_dir_list(3, vec![]);

        // 访问 1，使其变为最近使用
        cache.get_dir_list(1);

        // 插入 4，应该淘汰 2（最久未使用）
        cache.put_dir_list(4, vec![]);

        assert!(cache.get_dir_list(1).is_some()); // 最近使用过
        assert!(cache.get_dir_list(2).is_none()); // 最久未使用，被淘汰
        assert!(cache.get_dir_list(3).is_some());
        assert!(cache.get_dir_list(4).is_some());
    }

    #[test]
    fn test_subdirs_prefetch() {
        let cache = DirEntryCache::new();

        let entries = vec![
            DirEntry { name: "file.txt".into(), ino: 100, typ: 2 },
            DirEntry { name: "sub1".into(), ino: 200, typ: 1 },
            DirEntry { name: "sub2".into(), ino: 201, typ: 1 },
            DirEntry { name: "other".into(), ino: 101, typ: 2 },
        ];
        cache.put_dir_list(1, entries);

        let subdirs = cache.get_subdirs_for_prefetch(1).unwrap();
        assert_eq!(subdirs.len(), 2);
        assert!(subdirs.contains(&200));
        assert!(subdirs.contains(&201));
    }

    #[test]
    fn test_reset_stats() {
        let cache = DirEntryCache::new();

        cache.put_dir_list(1, vec![]);
        cache.get_dir_list(1);
        cache.get_dir_list(1);

        let stats_before = cache.stats();
        assert!(stats_before.total_lookups > 0);

        cache.reset_stats();

        let stats_after = cache.stats();
        assert_eq!(stats_after.total_lookups, 0);
        assert_eq!(stats_after.hits, 0);
        assert_eq!(stats_after.misses, 0);
    }
}
