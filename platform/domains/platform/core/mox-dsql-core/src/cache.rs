// =============================================================================
// mox-dsql-core 缓存层（已归一化到 mox-cache-core）
// =============================================================================
//
// 内部使用 mox-cache-core::MemoryCache（LRU + TTL + 统计），
// 对外保持同步 API（通过本地 tokio runtime 适配 async 接口）。
// ExecuteResult 以 JSON 序列化存储。
// =============================================================================

use crate::model::ExecuteResult;
use mox_cache_core::{Cache, CacheValue, MemoryCache};
use std::time::Duration;

/// 动态 SQL 缓存（同步适配层，内部基于 mox-cache-core::MemoryCache）
pub struct DsqlCache {
    inner: MemoryCache,
    rt: tokio::runtime::Runtime,
}

impl DsqlCache {
    /// 创建缓存实例
    pub fn new(max_size: usize) -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("创建 tokio runtime 失败");
        Self {
            inner: MemoryCache::new("dsql-query-cache", max_size),
            rt,
        }
    }

    /// 获取缓存（同步）
    pub fn get(&self, key: &str) -> Option<ExecuteResult> {
        let val = self.rt.block_on(self.inner.get(key)).ok()??;
        if val.is_null {
            return None;
        }
        let data = val.data?;
        serde_json::from_slice::<ExecuteResult>(&data).ok()
    }

    /// 写入缓存（同步）
    pub fn set(&self, key: String, result: ExecuteResult, ttl_seconds: i32) {
        let ttl = if ttl_seconds > 0 {
            Some(Duration::from_secs(ttl_seconds as u64))
        } else {
            Some(Duration::from_secs(300))
        };
        match serde_json::to_vec(&result) {
            Ok(bytes) => {
                let val = CacheValue::new(bytes, ttl);
                let _ = self.rt.block_on(self.inner.set(&key, val));
            }
            Err(e) => {
                tracing::warn!(error = %e, "DSQL 缓存序列化失败，跳过缓存");
            }
        }
    }

    /// 使指定 SQL 的所有缓存失效（按前缀）
    pub fn invalidate_sql(&self, sql_code: &str) {
        let prefix = format!("{sql_code}:");
        let _ = self.rt.block_on(self.inner.invalidate_prefix(&prefix));
    }

    /// 清空所有缓存
    pub fn clear(&self) {
        let _ = self.rt.block_on(self.inner.clear());
    }

    /// 缓存大小
    pub fn len(&self) -> usize {
        self.inner.stats().entry_count as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 缓存统计（命中/未命中/淘汰）
    pub fn stats(&self) -> mox_cache_core::CacheStats {
        self.inner.stats()
    }

    /// 生成缓存键（SQL代码 + 版本哈希 + 参数哈希）
    pub fn cache_key(sql_code: &str, version_hash: &str, params: &serde_json::Value) -> String {
        use sha2::{Digest, Sha256};
        let param_str = params.to_string();
        let mut hasher = Sha256::new();
        hasher.update(version_hash.as_bytes());
        hasher.update(param_str.as_bytes());
        let param_hash = hex::encode(hasher.finalize());
        format!("{sql_code}:{param_hash}")
    }
}

impl Default for DsqlCache {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ExecuteResult, OperationType, ResultType};

    fn make_result(sql_code: &str) -> ExecuteResult {
        ExecuteResult {
            sql_code: sql_code.to_string(),
            success: true,
            data: Some(serde_json::json!([{"id": 1, "name": "test"}])),
            row_count: Some(1),
            duration_ms: 5,
            cache_hit: false,
            error: None,
            trace_id: None,
        }
    }

    #[test]
    fn test_set_and_get() {
        let cache = DsqlCache::new(100);
        let result = make_result("test_sql");
        cache.set("key1".to_string(), result.clone(), 60);
        let got = cache.get("key1");
        assert!(got.is_some());
        assert_eq!(got.unwrap().sql_code, "test_sql");
    }

    #[test]
    fn test_miss() {
        let cache = DsqlCache::new(100);
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_invalidate_sql() {
        let cache = DsqlCache::new(100);
        cache.set("sql_a:hash1".to_string(), make_result("sql_a"), 60);
        cache.set("sql_a:hash2".to_string(), make_result("sql_a"), 60);
        cache.set("sql_b:hash1".to_string(), make_result("sql_b"), 60);
        cache.invalidate_sql("sql_a");
        assert!(cache.get("sql_a:hash1").is_none());
        assert!(cache.get("sql_a:hash2").is_none());
        assert!(cache.get("sql_b:hash1").is_some());
    }

    #[test]
    fn test_clear() {
        let cache = DsqlCache::new(100);
        cache.set("k1".to_string(), make_result("t1"), 60);
        cache.set("k2".to_string(), make_result("t2"), 60);
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_key_deterministic() {
        let params = serde_json::json!({"a": 1, "b": "hello"});
        let k1 = DsqlCache::cache_key("sql1", "v1", &params);
        let k2 = DsqlCache::cache_key("sql1", "v1", &params);
        assert_eq!(k1, k2);
        // 不同参数应产生不同 key
        let params2 = serde_json::json!({"a": 2});
        let k3 = DsqlCache::cache_key("sql1", "v1", &params2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn test_stats() {
        let cache = DsqlCache::new(100);
        cache.set("k1".to_string(), make_result("t1"), 60);
        let _ = cache.get("k1"); // hit
        let _ = cache.get("missing"); // miss
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.entry_count, 1);
    }
}
