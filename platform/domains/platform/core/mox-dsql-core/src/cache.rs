// mox-dsql-core 缓存层：内存LRU缓存
use crate::model::ExecuteResult;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// 缓存条目
struct CacheEntry {
    result: ExecuteResult,
    expire_at: Instant,
}

/// 简单LRU缓存（基于HashMap+时间过期）
pub struct DsqlCache {
    entries: Mutex<HashMap<String, CacheEntry>>,
    max_size: usize,
}

impl DsqlCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            max_size,
        }
    }

    /// 获取缓存
    pub fn get(&self, key: &str) -> Option<ExecuteResult> {
        let mut entries = self.entries.lock();
        if let Some(entry) = entries.get(key) {
            if entry.expire_at > Instant::now() {
                return Some(entry.result.clone());
            } else {
                entries.remove(key);
            }
        }
        None
    }

    /// 写入缓存
    pub fn set(&self, key: String, result: ExecuteResult, ttl_seconds: i32) {
        let mut entries = self.entries.lock();
        // 容量控制：超过最大值时清理过期条目
        if entries.len() >= self.max_size {
            entries.retain(|_, v| v.expire_at > Instant::now());
            // 如果清理后仍满，清空一半
            if entries.len() >= self.max_size {
                let keys: Vec<String> = entries.keys().take(self.max_size / 2).cloned().collect();
                for k in keys {
                    entries.remove(&k);
                }
            }
        }
        let ttl = if ttl_seconds > 0 { ttl_seconds as u64 } else { 300 };
        entries.insert(key, CacheEntry {
            result,
            expire_at: Instant::now() + Duration::from_secs(ttl),
        });
    }

    /// 使指定SQL的所有缓存失效
    pub fn invalidate_sql(&self, sql_code: &str) {
        let mut entries = self.entries.lock();
        entries.retain(|k, _| !k.starts_with(&format!("{sql_code}:")));
    }

    /// 清空所有缓存
    pub fn clear(&self) {
        self.entries.lock().clear();
    }

    /// 缓存大小
    pub fn len(&self) -> usize {
        self.entries.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.lock().is_empty()
    }

    /// 生成缓存键
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
