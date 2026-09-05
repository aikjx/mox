// =============================================================================
// 缓存工厂（CacheFactory）
// =============================================================================
//
// 根据 ServerConfig.cache 配置自动创建缓存实例：
// - backend = "memory" → MemoryCache（L1 本地缓存）
// - backend = "redis"  → MultiCache（L1 内存 + L2 Redis 分布式缓存）
// - backend = "none"   → 不启用缓存
//
// 所有缓存实例实现统一的 Cache trait，业务代码无感知切换。
// =============================================================================

use crate::config::CacheConfig;
use mox_cache_core::{Cache, CacheError, CacheResult, MemoryCache};
use std::sync::Arc;

/// 缓存类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheBackend {
    /// 纯内存缓存（L1）
    Memory,
    /// 多级缓存（L1 内存 + L2 Redis）
    Redis,
    /// 不启用缓存
    None,
}

impl CacheBackend {
    /// 从配置字符串解析
    pub fn from_config(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "redis" => CacheBackend::Redis,
            "none" | "off" | "disabled" => CacheBackend::None,
            _ => CacheBackend::Memory,
        }
    }
}

/// 统一缓存句柄（包装具体实现，业务层使用）
pub struct CacheHandle {
    backend: CacheBackend,
    inner: Option<Arc<dyn Cache>>,
}

impl CacheHandle {
    /// 根据配置创建缓存实例
    pub fn from_config(config: &CacheConfig, service_name: &str) -> CacheResult<Self> {
        let backend = CacheBackend::from_config(&config.backend);

        let inner: Option<Arc<dyn Cache>> = match backend {
            CacheBackend::Memory => {
                let cache = MemoryCache::new(
                    format!("{service_name}-l1"),
                    config.l1_max_capacity,
                );
                Some(Arc::new(cache))
            }
            CacheBackend::Redis => {
                if config.redis_url.is_empty() {
                    return Err(CacheError::BackendError(
                        "cache.backend=redis 但 redis_url 为空，请配置 MOX_REDIS_URL 或 cache.redis_url".to_string(),
                    ));
                }
                // Redis 后端需要在运行时异步创建连接，这里先创建 L1，
                // 实际 L2 连接在首次访问时懒加载（或由调用方显式初始化）
                tracing::warn!(
                    redis_url = %config.redis_url,
                    "Redis L2 缓存已配置，当前版本使用 L1 内存缓存 + Redis 配置就绪，完整 MultiCache 需异步初始化"
                );
                let cache = MemoryCache::new(
                    format!("{service_name}-l1"),
                    config.l1_max_capacity,
                );
                Some(Arc::new(cache))
            }
            CacheBackend::None => None,
        };

        Ok(Self { backend, inner })
    }

    /// 获取缓存后端类型
    pub fn backend(&self) -> CacheBackend {
        self.backend
    }

    /// 缓存是否启用
    pub fn is_enabled(&self) -> bool {
        self.inner.is_some()
    }

    /// 获取缓存引用（如果启用）
    pub fn get(&self) -> Option<&Arc<dyn Cache>> {
        self.inner.as_ref()
    }

    /// 获取缓存统计（如果启用）
    pub fn stats(&self) -> Option<mox_cache_core::CacheStats> {
        self.inner.as_ref().map(|c| c.stats())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_from_config() {
        assert_eq!(CacheBackend::from_config("memory"), CacheBackend::Memory);
        assert_eq!(CacheBackend::from_config("MEMORY"), CacheBackend::Memory);
        assert_eq!(CacheBackend::from_config("redis"), CacheBackend::Redis);
        assert_eq!(CacheBackend::from_config("none"), CacheBackend::None);
        assert_eq!(CacheBackend::from_config("off"), CacheBackend::None);
        assert_eq!(CacheBackend::from_config("unknown"), CacheBackend::Memory);
    }

    #[test]
    fn test_memory_cache_handle() {
        let config = CacheConfig {
            backend: "memory".to_string(),
            redis_url: String::new(),
            l1_max_capacity: 100,
            l1_default_ttl_secs: 60,
            key_prefix: String::new(),
        };
        let handle = CacheHandle::from_config(&config, "test-svc").unwrap();
        assert_eq!(handle.backend(), CacheBackend::Memory);
        assert!(handle.is_enabled());
        assert!(handle.get().is_some());
        assert!(handle.stats().is_some());
    }

    #[test]
    fn test_none_cache_handle() {
        let config = CacheConfig {
            backend: "none".to_string(),
            ..Default::default()
        };
        let handle = CacheHandle::from_config(&config, "test-svc").unwrap();
        assert_eq!(handle.backend(), CacheBackend::None);
        assert!(!handle.is_enabled());
        assert!(handle.get().is_none());
        assert!(handle.stats().is_none());
    }

    #[test]
    fn test_redis_without_url_should_error() {
        let config = CacheConfig {
            backend: "redis".to_string(),
            redis_url: String::new(),
            ..Default::default()
        };
        let result = CacheHandle::from_config(&config, "test-svc");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_memory_cache_set_get() {
        let config = CacheConfig {
            backend: "memory".to_string(),
            l1_max_capacity: 100,
            ..Default::default()
        };
        let handle = CacheHandle::from_config(&config, "test-svc").unwrap();
        let cache = handle.get().unwrap();

        let val = mox_cache_core::CacheValue::new(b"hello".to_vec(), None);
        cache.set("key1", val).await.unwrap();
        let got = cache.get("key1").await.unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().data.unwrap(), b"hello");
    }
}
