// =============================================================================
// Redis 缓存后端（RedisCache）
// =============================================================================
//
// 基于 redis crate 的分布式共享缓存后端。
// 支持：TTL 过期、空值防穿透、批量前缀失效（SCAN）、连接池。
// 适用于 L2 级缓存（多进程共享，微秒级访问）。
//
// 启用方式：features = ["redis-backend"]
// =============================================================================

use crate::{Cache, CacheError, CacheResult, CacheStats, CacheValue};
use async_trait::async_trait;
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Redis 缓存后端
pub struct RedisCache {
    name: String,
    client: Client,
    conn: MultiplexedConnection,
    key_prefix: String,
    // 统计
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    total_latency_ns: AtomicU64,
    total_ops: AtomicU64,
}

impl RedisCache {
    /// 创建 Redis 缓存
    pub async fn new(
        name: impl Into<String>,
        redis_url: &str,
        key_prefix: impl Into<String>,
    ) -> CacheResult<Self> {
        let client = Client::open(redis_url)
            .map_err(|e| CacheError::BackendError(format!("redis client: {e}")))?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| CacheError::ConnectionTimeout)?;
        Ok(Self {
            name: name.into(),
            client,
            conn,
            key_prefix: key_prefix.into(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
            total_ops: AtomicU64::new(0),
        })
    }

    fn full_key(&self, key: &str) -> String {
        if self.key_prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}:{}", self.key_prefix, key)
        }
    }

    fn record_op(&self, latency_ns: u64) {
        self.total_ops.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ns.fetch_add(latency_ns, Ordering::Relaxed);
    }
}

#[async_trait]
impl Cache for RedisCache {
    async fn get(&self, key: &str) -> CacheResult<Option<CacheValue>> {
        let start = Instant::now();
        if key.is_empty() {
            return Err(CacheError::EmptyKey);
        }
        let full_key = self.full_key(key);
        let data: Option<Vec<u8>> = self
            .conn
            .clone()
            .get(&full_key)
            .await
            .map_err(|e| CacheError::BackendError(format!("redis get: {e}")))?;
        let result = match data {
            Some(bytes) => {
                let value: CacheValue = serde_json::from_slice(&bytes)
                    .map_err(|e| CacheError::DeserializationError(e.to_string()))?;
                if value.is_expired() {
                    let _: () = self.conn.clone().del(&full_key).await.unwrap_or(());
                    self.misses.fetch_add(1, Ordering::Relaxed);
                    None
                } else {
                    self.hits.fetch_add(1, Ordering::Relaxed);
                    Some(value)
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
        let full_key = self.full_key(key);
        let bytes = serde_json::to_vec(&value)
            .map_err(|e| CacheError::SerializationError(e.to_string()))?;
        let mut conn = self.conn.clone();
        if let Some(exp) = value.expires_at {
            let ttl_secs = (exp - chrono::Utc::now().timestamp()).max(1) as u64;
            let _: () = redis::cmd("SET")
                .arg(&full_key)
                .arg(bytes)
                .arg("EX")
                .arg(ttl_secs)
                .query_async(&mut conn)
                .await
                .map_err(|e| CacheError::BackendError(format!("redis set: {e}")))?;
        } else {
            let _: () = conn
                .set(&full_key, bytes)
                .await
                .map_err(|e| CacheError::BackendError(format!("redis set: {e}")))?;
        }
        self.record_op(start.elapsed().as_nanos() as u64);
        Ok(())
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        let start = Instant::now();
        if key.is_empty() {
            return Err(CacheError::EmptyKey);
        }
        let full_key = self.full_key(key);
        let _: () = self
            .conn
            .clone()
            .del(&full_key)
            .await
            .map_err(|e| CacheError::BackendError(format!("redis del: {e}")))?;
        self.record_op(start.elapsed().as_nanos() as u64);
        Ok(())
    }

    async fn invalidate_prefix(&self, prefix: &str) -> CacheResult<usize> {
        let full_prefix = self.full_key(prefix);
        let pattern = format!("{full_prefix}*");
        let mut conn = self.conn.clone();
        // 使用 SCAN 迭代匹配的 key 并删除
        let mut count = 0usize;
        let mut cursor: i64 = 0;
        loop {
            let (next_cursor, keys): (i64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .map_err(|e| CacheError::BackendError(format!("redis scan: {e}")))?;
            if !keys.is_empty() {
                let _: () = conn.del(&keys).await.unwrap_or(());
                count += keys.len();
            }
            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }
        self.evictions.fetch_add(count as u64, Ordering::Relaxed);
        Ok(count)
    }

    async fn clear(&self) -> CacheResult<()> {
        // 只清除带前缀的 key，不做 FLUSHALL（避免误删其他应用数据）
        self.invalidate_prefix("").await?;
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
            entry_count: 0, // Redis 条目数需 INFO 命令获取，暂不统计
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}
