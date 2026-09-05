// =============================================================================
// MOX 统一缓存核心（mox-cache-core）
// =============================================================================
//
// 企业级多级缓存基础设施，提供：
//
// 1. **统一缓存抽象**（Cache trait）— get/set/delete/invalidate，异步接口
// 2. **内存 LRU 缓存**（memory）— 高性能本地缓存，支持容量上限 + TTL
// 3. **Redis 缓存**（redis，可选）— 分布式共享缓存，支持 TTL
// 4. **多级缓存**（MultiCache）— L1 内存 + L2 Redis，自动穿透回填
// 5. **缓存统计**（CacheStats）— 命中/未命中/命中率/延迟指标
// 6. **版本化失效** — key 包含版本哈希，模板变更自动失效
//
// 设计原则：
// - 统一抽象：所有后端实现同一 Cache trait，业务代码无感知切换
// - 异步优先：基于 async-trait，适配 tokio 运行时
// - 可观测：内置命中率、延迟指标，对接 mox-observability-core
// - 防穿透：空值缓存、singleflight 合并并发请求
// - 防雪崩：TTL 抖动（jitter），避免大量 key 同时过期
// =============================================================================

pub mod memory;
#[cfg(feature = "redis-backend")]
pub mod redis;
pub mod stats;
pub mod multi;

// ── 重导出 ────────────────────────────────────────────────────────────────

pub use memory::MemoryCache;
#[cfg(feature = "redis-backend")]
pub use redis::RedisCache;
pub use stats::CacheStats;
pub use multi::MultiCache;

// ── Crate 元数据 ──────────────────────────────────────────────────────────

pub const CRATE_ID: &str = "mox-cache-core";
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 缓存值包装：支持标记空值（防止缓存穿透）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheValue {
    /// 实际数据（JSON 序列化）
    pub data: Option<Vec<u8>>,
    /// 是否为空值标记（用于防穿透）
    pub is_null: bool,
    /// 过期时间戳（Unix 秒），None 表示永不过期
    pub expires_at: Option<i64>,
    /// 版本哈希（用于模板变更时自动失效）
    pub version: Option<String>,
}

impl CacheValue {
    /// 创建正常值
    pub fn new(data: Vec<u8>, ttl: Option<Duration>) -> Self {
        let expires_at = ttl.map(|d| chrono::Utc::now().timestamp() + d.as_secs() as i64);
        Self { data: Some(data), is_null: false, expires_at, version: None }
    }

    /// 创建空值标记（防穿透）
    pub fn null(ttl: Duration) -> Self {
        let expires_at = chrono::Utc::now().timestamp() + ttl.as_secs() as i64;
        Self { data: None, is_null: true, expires_at: Some(expires_at), version: None }
    }

    /// 设置版本
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// 是否已过期
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(exp) => chrono::Utc::now().timestamp() > exp,
            None => false,
        }
    }
}

/// 统一缓存抽象 trait
#[async_trait]
pub trait Cache: Send + Sync {
    /// 获取缓存值
    async fn get(&self, key: &str) -> CacheResult<Option<CacheValue>>;

    /// 设置缓存值
    async fn set(&self, key: &str, value: CacheValue) -> CacheResult<()>;

    /// 删除缓存
    async fn delete(&self, key: &str) -> CacheResult<()>;

    /// 按前缀批量失效（用于模板变更时清除所有相关查询）
    async fn invalidate_prefix(&self, prefix: &str) -> CacheResult<usize>;

    /// 清空全部缓存
    async fn clear(&self) -> CacheResult<()>;

    /// 获取缓存统计
    fn stats(&self) -> CacheStats;

    /// 缓存名称（用于日志/指标区分）
    fn name(&self) -> &str;
}

/// 缓存错误
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("缓存后端错误: {0}")]
    BackendError(String),
    #[error("缓存序列化失败: {0}")]
    SerializationError(String),
    #[error("缓存反序列化失败: {0}")]
    DeserializationError(String),
    #[error("缓存键为空")]
    EmptyKey,
    #[error("缓存连接超时")]
    ConnectionTimeout,
    #[error("内部错误: {0}")]
    InternalError(String),
}

/// 缓存结果类型
pub type CacheResult<T> = Result<T, CacheError>;

/// 构建标准缓存 key：{namespace}:{identifier}:{params_hash}
pub fn build_key(namespace: &str, identifier: &str, params_hash: &str) -> String {
    format!("{namespace}:{identifier}:{params_hash}")
}

/// TTL 抖动：在基础 TTL 上叠加随机偏移，防止雪崩
pub fn ttl_with_jitter(base_ttl: Duration, jitter_ratio: f64) -> Duration {
    use std::time::Duration;
    let jitter_secs = (base_ttl.as_secs_f64() * jitter_ratio) as u64;
    if jitter_secs == 0 {
        return base_ttl;
    }
    // 简单伪随机：基于当前纳秒时间
    let nanos = chrono::Utc::now().timestamp_subsec_nanos() as u64;
    let offset = nanos % (jitter_secs + 1);
    base_ttl + Duration::from_secs(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_value_expiry() {
        let v = CacheValue::new(b"hello".to_vec(), Some(Duration::from_secs(60)));
        assert!(!v.is_expired());
        assert!(!v.is_null);
        assert_eq!(v.data.unwrap(), b"hello");
    }

    #[test]
    fn test_null_value() {
        let v = CacheValue::null(Duration::from_secs(30));
        assert!(v.is_null);
        assert!(v.data.is_none());
    }

    #[test]
    fn test_build_key() {
        let key = build_key("dsql", "get_user", "abc123");
        assert_eq!(key, "dsql:get_user:abc123");
    }

    #[test]
    fn test_ttl_jitter() {
        let base = Duration::from_secs(60);
        let jittered = ttl_with_jitter(base, 0.1);
        assert!(jittered >= base);
        assert!(jittered <= base + Duration::from_secs(7));
    }

    #[test]
    fn cache_error_display() {
        let err = CacheError::EmptyKey;
        assert!(format!("{err}").contains("键为空"));
    }
}
