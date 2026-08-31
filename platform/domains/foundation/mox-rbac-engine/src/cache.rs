// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 评估结果缓存层
//!
//! 提供 LRU 缓存，缓存权限评估结果以提升性能。
//! 缓存键由 (subject_roles_hash, resource_path, action) 组成。
//!
//! 特性：
//! - 固定容量的 LRU 淘汰策略
//! - 策略变更时自动失效
//! - 线程安全
//! - 缓存统计（命中/未命中/淘汰数）

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::error::RbacError;
use crate::types::{Action, EvaluationResult};

/// 缓存键
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// 角色列表的排序后哈希（确保相同角色集合产生相同键）
    pub roles_hash: u64,
    /// 资源路径
    pub resource: String,
    /// 动作
    pub action: Action,
    /// 租户标识（用于租户隔离）
    pub tenant: Option<String>,
}

impl CacheKey {
    /// 从角色列表、资源、动作构建缓存键
    pub fn new(roles: &[String], resource: &str, action: Action, tenant: Option<&str>) -> Self {
        let mut sorted_roles: Vec<&str> = roles.iter().map(|r| r.as_str()).collect();
        sorted_roles.sort();

        let mut hash: u64 = 0;
        for role in &sorted_roles {
            // 简单的 FNV-1a 哈希
            for byte in role.as_bytes() {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(1099511628211u64);
            }
            // 角色间分隔符，避免 "ab" + "c" 和 "a" + "bc" 冲突
            hash ^= b':' as u64;
            hash = hash.wrapping_mul(1099511628211u64);
        }

        Self {
            roles_hash: hash,
            resource: resource.into(),
            action,
            tenant: tenant.map(|t| t.into()),
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// 缓存命中次数
    pub hits: u64,
    /// 缓存未命中次数
    pub misses: u64,
    /// 缓存淘汰次数
    pub evictions: u64,
    /// 当前缓存条目数
    pub size: usize,
    /// 缓存容量
    pub capacity: usize,
}

impl CacheStats {
    /// 命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// LRU 评估缓存
///
/// 缓存权限评估结果，避免重复计算。
/// 当策略/角色变更时，应调用 [`invalidate_all`] 清空缓存。
#[derive(Debug, Clone)]
pub struct EvaluationCache {
    inner: Arc<CacheInner>,
}

#[derive(Debug)]
struct CacheInner {
    capacity: usize,
    data: Mutex<CacheData>,
}

#[derive(Debug)]
struct CacheData {
    // key -> (index_in_lru, value)
    map: HashMap<CacheKey, EvaluationResult>,
    // LRU 顺序：front = most recent, back = least recent
    lru: VecDeque<CacheKey>,
    stats: CacheStats,
}

impl EvaluationCache {
    /// 创建指定容量的缓存
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            inner: Arc::new(CacheInner {
                capacity,
                data: Mutex::new(CacheData {
                    map: HashMap::new(),
                    lru: VecDeque::new(),
                    stats: CacheStats {
                        hits: 0,
                        misses: 0,
                        evictions: 0,
                        size: 0,
                        capacity,
                    },
                }),
            }),
        }
    }

    /// 默认容量（1024 条目）
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// 获取缓存值
    pub fn get(&self, key: &CacheKey) -> Option<EvaluationResult> {
        let mut data = self.inner.data.lock().ok()?;

        if let Some(value) = data.map.get(key) {
            // 命中：移到 LRU 前端
            let value = value.clone();
            if let Some(pos) = data.lru.iter().position(|k| k == key) {
                data.lru.remove(pos);
            }
            data.lru.push_front(key.clone());
            data.stats.hits += 1;
            Some(value)
        } else {
            data.stats.misses += 1;
            None
        }
    }

    /// 插入缓存值
    pub fn put(&self, key: CacheKey, value: EvaluationResult) -> Result<(), RbacError> {
        let mut data = self
            .inner
            .data
            .lock()
            .map_err(|e| RbacError::CacheError(format!("cache lock poisoned: {e}")))?;

        // 如果键已存在，更新值并移到前端
        if data.map.contains_key(&key) {
            data.map.insert(key.clone(), value);
            if let Some(pos) = data.lru.iter().position(|k| k == &key) {
                data.lru.remove(pos);
            }
            data.lru.push_front(key);
            return Ok(());
        }

        // 新键：检查容量
        if data.map.len() >= self.inner.capacity {
            // 淘汰最久未使用的
            if let Some(evicted_key) = data.lru.pop_back() {
                data.map.remove(&evicted_key);
                data.stats.evictions += 1;
            }
        }

        data.map.insert(key.clone(), value);
        data.lru.push_front(key);
        data.stats.size = data.map.len();

        Ok(())
    }

    /// 清空所有缓存
    pub fn invalidate_all(&self) -> Result<usize, RbacError> {
        let mut data = self
            .inner
            .data
            .lock()
            .map_err(|e| RbacError::CacheError(format!("cache lock poisoned: {e}")))?;

        let count = data.map.len();
        data.map.clear();
        data.lru.clear();
        data.stats.size = 0;

        Ok(count)
    }

    /// 失效与特定资源相关的缓存
    pub fn invalidate_resource(&self, resource_prefix: &str) -> Result<usize, RbacError> {
        let mut data = self
            .inner
            .data
            .lock()
            .map_err(|e| RbacError::CacheError(format!("cache lock poisoned: {e}")))?;

        let mut removed = 0;
        data.map.retain(|k, _| {
            if k.resource.starts_with(resource_prefix) {
                removed += 1;
                false
            } else {
                true
            }
        });

        data.lru.retain(|k| !k.resource.starts_with(resource_prefix));
        data.stats.size = data.map.len();

        Ok(removed)
    }

    /// 获取缓存统计
    pub fn stats(&self) -> CacheStats {
        let data = self.inner.data.lock().unwrap();
        let mut stats = data.stats.clone();
        stats.size = data.map.len();
        stats.capacity = self.inner.capacity;
        stats
    }

    /// 重置统计
    pub fn reset_stats(&self) {
        if let Ok(mut data) = self.inner.data.lock() {
            data.stats.hits = 0;
            data.stats.misses = 0;
            data.stats.evictions = 0;
        }
    }

    /// 缓存容量
    pub fn capacity(&self) -> usize {
        self.inner.capacity
    }

    /// 当前条目数
    pub fn len(&self) -> usize {
        self.inner.data.lock().map(|d| d.map.len()).unwrap_or(0)
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for EvaluationCache {
    fn default() -> Self {
        Self::new()
    }
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Action;

    fn make_key(roles: &[&str], resource: &str, action: Action) -> CacheKey {
        let roles: Vec<String> = roles.iter().map(|s| s.to_string()).collect();
        CacheKey::new(&roles, resource, action, None)
    }

    #[test]
    fn cache_put_and_get() {
        let cache = EvaluationCache::with_capacity(10);
        let key = make_key(&["admin"], "db:prod/*", Action::Write);
        let value = EvaluationResult::Granted {
            matched_policies: vec!["p1".into()],
        };

        assert!(cache.get(&key).is_none()); // 初始未命中
        cache.put(key.clone(), value.clone()).unwrap();
        assert_eq!(cache.len(), 1);

        let got = cache.get(&key).unwrap();
        assert_eq!(got, value);
    }

    #[test]
    fn cache_miss_updates_stats() {
        let cache = EvaluationCache::with_capacity(10);
        let key = make_key(&["viewer"], "db:prod/*", Action::Write);

        assert!(cache.get(&key).is_none());
        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 0);
    }

    #[test]
    fn cache_hit_updates_stats() {
        let cache = EvaluationCache::with_capacity(10);
        let key = make_key(&["admin"], "db:*", Action::Read);
        cache
            .put(
                key.clone(),
                EvaluationResult::Granted {
                    matched_policies: vec![],
                },
            )
            .unwrap();

        cache.get(&key);
        cache.get(&key);
        cache.get(&key);

        let stats = cache.stats();
        assert_eq!(stats.hits, 3);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.hit_rate(), 1.0);
    }

    #[test]
    fn cache_lru_eviction() {
        let cache = EvaluationCache::with_capacity(3);

        // 添加 3 个条目
        for i in 0..3 {
            let key = make_key(&["role"], &format!("res:{}", i), Action::Read);
            cache
                .put(key, EvaluationResult::Granted { matched_policies: vec![] })
                .unwrap();
        }
        assert_eq!(cache.len(), 3);

        // 访问第一个（使它变成最近使用）
        let key0 = make_key(&["role"], "res:0", Action::Read);
        cache.get(&key0);

        // 添加第 4 个，应淘汰最久未使用的（res:1）
        let key3 = make_key(&["role"], "res:3", Action::Read);
        cache
            .put(
                key3,
                EvaluationResult::Granted { matched_policies: vec![] },
            )
            .unwrap();

        assert_eq!(cache.len(), 3);
        assert!(cache.get(&key0).is_some()); // 最近使用，保留
        assert!(cache.get(&make_key(&["role"], "res:2", Action::Read)).is_some()); // 次新，保留
        assert!(cache.get(&make_key(&["role"], "res:1", Action::Read)).is_none()); // 最旧，被淘汰

        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
    }

    #[test]
    fn cache_invalidate_all() {
        let cache = EvaluationCache::with_capacity(10);
        for i in 0..5 {
            let key = make_key(&["r"], &format!("r:{}", i), Action::Read);
            cache
                .put(key, EvaluationResult::Granted { matched_policies: vec![] })
                .unwrap();
        }
        assert_eq!(cache.len(), 5);

        let invalidated = cache.invalidate_all().unwrap();
        assert_eq!(invalidated, 5);
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn cache_invalidate_resource() {
        let cache = EvaluationCache::with_capacity(10);
        cache
            .put(
                make_key(&["r"], "db:prod/a", Action::Read),
                EvaluationResult::Granted { matched_policies: vec![] },
            )
            .unwrap();
        cache
            .put(
                make_key(&["r"], "db:prod/b", Action::Write),
                EvaluationResult::Granted { matched_policies: vec![] },
            )
            .unwrap();
        cache
            .put(
                make_key(&["r"], "db:test/c", Action::Read),
                EvaluationResult::Granted { matched_policies: vec![] },
            )
            .unwrap();

        let removed = cache.invalidate_resource("db:prod/").unwrap();
        assert_eq!(removed, 2);
        assert_eq!(cache.len(), 1);
        assert!(cache
            .get(&make_key(&["r"], "db:test/c", Action::Read))
            .is_some());
    }

    #[test]
    fn cache_key_role_order_independent() {
        // 相同角色集合，不同顺序应产生相同键
        let key1 = CacheKey::new(
            &["admin".into(), "viewer".into()],
            "db:*",
            Action::Read,
            None,
        );
        let key2 = CacheKey::new(
            &["viewer".into(), "admin".into()],
            "db:*",
            Action::Read,
            None,
        );
        assert_eq!(key1.roles_hash, key2.roles_hash);
    }

    #[test]
    fn cache_key_tenant_isolation() {
        // 不同租户应产生不同键
        let key1 = CacheKey::new(
            &["admin".into()],
            "db:*",
            Action::Read,
            Some("tenant-A"),
        );
        let key2 = CacheKey::new(
            &["admin".into()],
            "db:*",
            Action::Read,
            Some("tenant-B"),
        );
        assert_ne!(key1, key2);
    }

    #[test]
    fn cache_update_existing_key() {
        let cache = EvaluationCache::with_capacity(10);
        let key = make_key(&["r"], "res:1", Action::Read);

        cache
            .put(
                key.clone(),
                EvaluationResult::Granted { matched_policies: vec![] },
            )
            .unwrap();
        cache
            .put(
                key.clone(),
                EvaluationResult::Denied {
                    reason: "test".into(),
                    denied_by_policy: None,
                },
            )
            .unwrap();

        assert_eq!(cache.len(), 1); // 不增加条目数
        let result = cache.get(&key).unwrap();
        assert!(result.is_denied()); // 值已更新
    }

    #[test]
    fn cache_stats_capacity() {
        let cache = EvaluationCache::with_capacity(42);
        let stats = cache.stats();
        assert_eq!(stats.capacity, 42);
        assert_eq!(stats.size, 0);
    }

    #[test]
    fn cache_reset_stats() {
        let cache = EvaluationCache::with_capacity(10);
        let key = make_key(&["r"], "res", Action::Read);
        cache
            .put(key.clone(), EvaluationResult::Granted { matched_policies: vec![] })
            .unwrap();
        cache.get(&key);
        cache.get(&key);

        assert_eq!(cache.stats().hits, 2);

        cache.reset_stats();
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.size, 1); // size 不重置
    }

    #[test]
    fn cache_default_capacity() {
        let cache = EvaluationCache::new();
        assert_eq!(cache.capacity(), 1024);
    }

    #[test]
    fn cache_min_capacity_is_1() {
        let cache = EvaluationCache::with_capacity(0);
        assert_eq!(cache.capacity(), 1);
    }

    #[test]
    fn cache_denied_result_caching() {
        let cache = EvaluationCache::with_capacity(10);
        let key = make_key(&["viewer"], "db:prod/secret", Action::Write);
        let denied = EvaluationResult::Denied {
            reason: "insufficient permissions".into(),
            denied_by_policy: Some("deny-prod".into()),
        };

        cache.put(key.clone(), denied.clone()).unwrap();
        let got = cache.get(&key).unwrap();
        assert_eq!(got, denied);
    }
}
