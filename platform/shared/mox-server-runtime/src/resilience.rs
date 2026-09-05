// =============================================================================
// 弹性容错中间件（Resilience Middleware）
// =============================================================================
//
// 集成 mox-resilience-core，提供：
// - CircuitBreakerRegistry：全局熔断器注册表，按服务/接口管理熔断器
// - 熔断中间件：对HTTP请求进行熔断保护，熔断器打开时快速失败
// - 重试配置：可配置的重试策略（用于下游调用）
// =============================================================================

use crate::config::ResilienceConfig;
use axum::{
    body::Body,
    extract::Extension,
    http::Request,
    middleware::Next,
    response::Response,
};
use mox_resilience_core::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// 熔断器注册表：管理多个命名熔断器
#[derive(Clone)]
pub struct CircuitBreakerRegistry {
    inner: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    default_config: CircuitBreakerConfig,
}

impl CircuitBreakerRegistry {
    /// 创建熔断器注册表
    pub fn new(default_config: CircuitBreakerConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            default_config,
        }
    }

    /// 从配置创建
    pub fn from_config(config: &ResilienceConfig) -> Self {
        let default_config = CircuitBreakerConfig {
            failure_rate_threshold: config.circuit_breaker.failure_rate_threshold,
            minimum_requests: config.circuit_breaker.minimum_requests,
            window_size: config.circuit_breaker.window_size,
            open_duration: Duration::from_secs(config.circuit_breaker.open_duration_secs),
            half_open_max_requests: config.circuit_breaker.half_open_max_requests,
        };
        Self::new(default_config)
    }

    /// 获取或创建熔断器
    pub fn get_or_create(&self, name: &str) -> CircuitBreaker {
        if let Some(cb) = self.inner.read().get(name) {
            return cb.clone();
        }
        let mut map = self.inner.write();
        // 双重检查
        if let Some(cb) = map.get(name) {
            return cb.clone();
        }
        let cb = CircuitBreaker::new(name, self.default_config.clone());
        map.insert(name.to_string(), cb.clone());
        cb
    }

    /// 获取指定熔断器（不存在返回None）
    pub fn get(&self, name: &str) -> Option<CircuitBreaker> {
        self.inner.read().get(name).cloned()
    }

    /// 获取所有熔断器状态
    pub fn all_states(&self) -> Vec<(String, CircuitState)> {
        self.inner
            .read()
            .iter()
            .map(|(name, cb)| (name.clone(), cb.state()))
            .collect()
    }

    /// 重置所有熔断器
    pub fn reset_all(&self) {
        for cb in self.inner.read().values() {
            cb.reset();
        }
    }
}

impl Default for CircuitBreakerRegistry {
    fn default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }
}

/// 熔断中间件：对请求路径进行熔断保护
///
/// 使用方式：
/// ```rust
/// use axum::middleware;
/// let app = Router::new()
///     .layer(middleware::from_fn_with_state(
///         registry.clone(),
///         circuit_breaker_middleware,
///     ));
/// ```
pub async fn circuit_breaker_middleware(
    Extension(registry): Extension<CircuitBreakerRegistry>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    // 健康检查和指标端点不经过熔断
    if path.starts_with("/health") || path == "/metrics" {
        return next.run(request).await;
    }

    let cb = registry.get_or_create(&path);

    // 检查熔断器状态
    if !cb.allow_request() {
        tracing::warn!(path = %path, "熔断器已打开，请求被快速失败");
        return axum::http::Response::builder()
            .status(axum::http::StatusCode::SERVICE_UNAVAILABLE)
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::json!({
                    "error": "service_unavailable",
                    "message": format!("服务熔断器已打开: {}", path),
                    "code": 503
                })
                .to_string(),
            ))
            .unwrap();
    }

    // 执行请求
    let response = next.run(request).await;

    // 记录结果
    let status = response.status();
    if status.is_server_error() {
        cb.record_failure();
    } else {
        cb.record_success();
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_get_or_create() {
        let registry = CircuitBreakerRegistry::default();
        let cb1 = registry.get_or_create("test-service");
        let cb2 = registry.get_or_create("test-service");
        assert_eq!(cb1.name(), "test-service");
        assert_eq!(cb2.name(), "test-service");
        // 应该返回同一个实例
        assert!(registry.get("test-service").is_some());
    }

    #[test]
    fn test_registry_all_states() {
        let registry = CircuitBreakerRegistry::default();
        registry.get_or_create("svc-a");
        registry.get_or_create("svc-b");
        let states = registry.all_states();
        assert_eq!(states.len(), 2);
        assert!(states.iter().all(|(_, s)| *s == CircuitState::Closed));
    }

    #[test]
    fn test_registry_reset_all() {
        let config = CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            minimum_requests: 2,
            window_size: 10,
            open_duration: Duration::from_secs(30),
            half_open_max_requests: 3,
        };
        let registry = CircuitBreakerRegistry::new(config);
        let cb = registry.get_or_create("test");
        // 触发熔断
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // 重置
        registry.reset_all();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_middleware_passthrough() {
        use axum::{body::Body, http::Request, routing::get, Router};
        use tower::ServiceExt;

        let registry = CircuitBreakerRegistry::default();
        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn_with_state(
                registry.clone(),
                circuit_breaker_middleware,
            ));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn test_circuit_breaker_middleware_health_skip() {
        use axum::{body::Body, http::Request, routing::get, Router};
        use tower::ServiceExt;

        let registry = CircuitBreakerRegistry::default();
        let app = Router::new()
            .route("/health/live", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn_with_state(
                registry.clone(),
                circuit_breaker_middleware,
            ));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health/live")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        // 健康检查端点不应该创建熔断器
        assert!(registry.get("/health/live").is_none());
    }
}
