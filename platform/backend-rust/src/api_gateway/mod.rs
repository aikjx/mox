// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Q · API 网关与服务网格
//!
//! 核心能力：
//! - 路由转发与负载均衡
//! - 分布式限流（令牌桶 + 滑动窗口）
//! - 熔断器（Closed/Open/Half-Open 三态）
//! - 重试策略（指数退避 + 抖动）
//! - 超时控制
//! - 请求追踪

pub mod rate_limiter;
pub mod circuit_breaker;
pub mod retry;

use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    response::Response,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub use rate_limiter::{RateLimiter, RateLimitAlgorithm, RateLimitConfig};
pub use circuit_breaker::{CircuitBreaker, CircuitState, CircuitConfig};
pub use retry::{RetryPolicy, RetryConfig};

/// API 网关配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub service_name: String,
    pub listen_addr: String,
    pub default_timeout_ms: u64,
    pub max_request_size_bytes: usize,
    pub enable_request_tracing: bool,
    pub enable_access_log: bool,
    pub upstream_services: Vec<UpstreamService>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            service_name: "mox-api-gateway".to_string(),
            listen_addr: "0.0.0.0:8080".to_string(),
            default_timeout_ms: 30000,
            max_request_size_bytes: 50 * 1024 * 1024,
            enable_request_tracing: true,
            enable_access_log: true,
            upstream_services: vec![],
        }
    }
}

/// 上游服务定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamService {
    pub name: String,
    pub path_prefix: String,
    pub targets: Vec<String>,
    pub load_balance: LoadBalanceStrategy,
    pub timeout_ms: u64,
    pub retries: u32,
    pub rate_limit_per_second: u64,
    pub circuit_breaker_threshold: f64,
}

/// 负载均衡策略
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LoadBalanceStrategy {
    RoundRobin,
    Random,
    LeastConnections,
    WeightedRoundRobin,
    ConsistentHash,
}

/// API 网关
pub struct ApiGateway {
    config: GatewayConfig,
    rate_limiter: Arc<RateLimiter>,
    circuit_breakers: DashMap<String, Arc<CircuitBreaker>>,
    retry_policy: Arc<RetryPolicy>,
    round_robin_counters: Arc<DashMap<String, std::sync::atomic::AtomicUsize>>,
    request_count: std::sync::atomic::AtomicU64,
    error_count: std::sync::atomic::AtomicU64,
}

impl ApiGateway {
    /// 创建网关构建器
    pub fn builder() -> GatewayBuilder {
        GatewayBuilder::new()
    }

    /// 从配置创建
    pub fn new(config: GatewayConfig) -> Self {
        let rate_limiter = Arc::new(RateLimiter::new(RateLimitConfig {
            algorithm: RateLimitAlgorithm::TokenBucket,
            tokens_per_second: 1000,
            burst_size: 2000,
            window_seconds: 60,
            max_requests: 60000,
        }));

        let retry_policy = Arc::new(RetryPolicy::new(RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            retry_on_status: vec![500, 502, 503, 504],
        }));

        let circuit_breakers = DashMap::new();
        for svc in &config.upstream_services {
            circuit_breakers.insert(
                svc.name.clone(),
                Arc::new(CircuitBreaker::new(CircuitConfig {
                    failure_threshold: svc.circuit_breaker_threshold,
                    minimum_requests: 20,
                    open_duration_ms: 30000,
                    half_open_max_requests: 5,
                    window_size_ms: 60000,
                })),
            );
        }

        Self {
            config,
            rate_limiter,
            circuit_breakers,
            retry_policy,
            round_robin_counters: Arc::new(DashMap::new()),
            request_count: std::sync::atomic::AtomicU64::new(0),
            error_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 监听地址
    pub fn listen_addr(&self) -> &str {
        &self.config.listen_addr
    }

    /// 限流速率
    pub fn rate_limit(&self) -> u64 {
        self.rate_limiter.tokens_per_second()
    }

    /// 重试次数
    pub fn retry_attempts(&self) -> u32 {
        self.retry_policy.max_attempts()
    }

    /// 代理请求处理器
    /// 注意：fallback 路由不提供路径参数，不能用 Path<String> 提取器（否则 500），
    /// 改为从 req.uri().path() 直接取请求路径。
    pub fn proxy_handler(&self) -> axum::routing::MethodRouter {
        let gateway = Arc::new(Clone::clone(self));
        axum::routing::any(move |req: Request| {
            let gw = gateway.clone();
            async move {
                let path = req.uri().path().trim_start_matches('/').to_string();
                gw.proxy(path, req).await
            }
        })
    }

    /// 代理请求
    async fn proxy(&self, path: String, req: Request) -> Response<Body> {
        let request_id = Uuid::new_v4().to_string();
        let start = Instant::now();

        // 限流检查
        if !self.rate_limiter.try_acquire("global").await {
            self.error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return self.rate_limit_response(&request_id);
        }

        self.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // 匹配上游服务
        let upstream = self.match_upstream(&path);

        let (target, svc_name) = match upstream {
            Some(svc) => {
                // 熔断器检查
                if let Some(cb) = self.circuit_breakers.get(&svc.name) {
                    if !cb.can_execute().await {
                        return self.circuit_open_response(&request_id, &svc.name);
                    }
                }
                let target = self.select_target(&svc);
                (target, svc.name.clone())
            }
            None => {
                return self.not_found_response(&request_id, &path);
            }
        };

        // 构建上游 URL
        let upstream_url = format!("{}/{}", target.trim_end_matches('/'), path);

        // 转换请求方法
        let method = reqwest::Method::from_bytes(req.method().as_str().as_bytes())
            .unwrap_or(reqwest::Method::GET);

        // 转换请求头
        let mut headers = reqwest::header::HeaderMap::new();
        for (k, v) in req.headers() {
            if let Ok(name) = reqwest::header::HeaderName::from_bytes(k.as_str().as_bytes()) {
                if let Ok(val) = reqwest::header::HeaderValue::from_bytes(v.as_bytes()) {
                    headers.insert(name, val);
                }
            }
        }

        // 收集请求体
        let body_bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
            .await
            .unwrap_or_default();

        // 执行请求（带重试）
        let upstream_url_clone = upstream_url.clone();
        let body_vec = body_bytes.to_vec();
        let headers_clone = headers.clone();
        let method_clone = method.clone();

        let result = self.retry_policy.execute_http(move || {
            let url = upstream_url_clone.clone();
            let body = body_vec.clone();
            let hdrs = headers_clone.clone();
            let mthd = method_clone.clone();
            async move {
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_millis(30000))
                    .build()
                    .unwrap();
                client.request(mthd, &url)
                    .headers(hdrs)
                    .body(body)
                    .send()
                    .await
            }
        }).await;

        let duration = start.elapsed();

        match result {
            Ok(resp) => {
                // 记录熔断器结果
                if let Some(cb) = self.circuit_breakers.get(&svc_name) {
                    if resp.status().is_server_error() {
                        cb.record_failure().await;
                    } else {
                        cb.record_success().await;
                    }
                }

                // 转换状态码
                let status = StatusCode::from_u16(resp.status().as_u16())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                let mut builder = Response::builder().status(status);

                // 转换响应头
                for (k, v) in resp.headers() {
                    if let Ok(name) = axum::http::HeaderName::from_bytes(k.as_str().as_bytes()) {
                        if let Ok(val) = axum::http::HeaderValue::from_bytes(v.as_bytes()) {
                            builder = builder.header(name, val);
                        }
                    }
                }
                builder = builder.header("X-Request-ID", request_id);
                builder = builder.header("X-Duration-Ms", duration.as_millis().to_string());

                let body = resp.bytes().await.unwrap_or_default();
                builder.body(Body::from(body)).unwrap_or_else(|_| {
                    Response::builder().status(500).body(Body::empty()).unwrap()
                })
            }
            Err(e) => {
                self.error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if let Some(cb) = self.circuit_breakers.get(&svc_name) {
                    cb.record_failure().await;
                }
                Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header("X-Request-ID", &request_id)
                    .body(Body::from(format!(
                        r#"{{"error":"bad_gateway","message":"{}","request_id":"{}"}}"#,
                        e, request_id
                    )))
                    .unwrap()
            }
        }
    }

    pub fn match_upstream(&self, path: &str) -> Option<UpstreamService> {
        self.config.upstream_services.iter()
            .find(|svc| path.starts_with(&svc.path_prefix))
            .cloned()
    }

    pub fn select_target(&self, svc: &UpstreamService) -> String {
        match svc.load_balance {
            LoadBalanceStrategy::RoundRobin => {
                let counter = self.round_robin_counters
                    .entry(svc.name.clone())
                    .or_insert_with(|| std::sync::atomic::AtomicUsize::new(0));
                let idx = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % svc.targets.len();
                svc.targets[idx].clone()
            }
            LoadBalanceStrategy::Random => {
                use rand::Rng;
                let idx = rand::thread_rng().gen_range(0..svc.targets.len());
                svc.targets[idx].clone()
            }
            _ => svc.targets.first().cloned().unwrap_or_default(),
        }
    }

    fn rate_limit_response(&self, request_id: &str) -> Response<Body> {
        Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("X-Request-ID", request_id)
            .header("Retry-After", "1")
            .body(Body::from(format!(
                r#"{{"error":"rate_limited","message":"请求频率超过限制","request_id":"{}"}}"#,
                request_id
            )))
            .unwrap()
    }

    fn circuit_open_response(&self, request_id: &str, service: &str) -> Response<Body> {
        Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header("X-Request-ID", request_id)
            .header("Retry-After", "30")
            .body(Body::from(format!(
                r#"{{"error":"circuit_open","message":"服务 {} 熔断器已打开","request_id":"{}"}}"#,
                service, request_id
            )))
            .unwrap()
    }

    fn not_found_response(&self, request_id: &str, path: &str) -> Response<Body> {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("X-Request-ID", request_id)
            .body(Body::from(format!(
                r#"{{"error":"not_found","message":"未匹配到上游服务: {}","request_id":"{}"}}"#,
                path, request_id
            )))
            .unwrap()
    }

    /// 获取网关统计
    pub fn stats(&self) -> GatewayStats {
        GatewayStats {
            service_name: self.config.service_name.clone(),
            total_requests: self.request_count.load(std::sync::atomic::Ordering::Relaxed),
            total_errors: self.error_count.load(std::sync::atomic::Ordering::Relaxed),
            upstream_services: self.config.upstream_services.len(),
            circuit_breakers: self.circuit_breakers.len(),
        }
    }
}

impl Clone for ApiGateway {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            rate_limiter: self.rate_limiter.clone(),
            circuit_breakers: self.circuit_breakers.clone(),
            retry_policy: self.retry_policy.clone(),
            round_robin_counters: self.round_robin_counters.clone(),
            request_count: std::sync::atomic::AtomicU64::new(
                self.request_count.load(std::sync::atomic::Ordering::Relaxed)
            ),
            error_count: std::sync::atomic::AtomicU64::new(
                self.error_count.load(std::sync::atomic::Ordering::Relaxed)
            ),
        }
    }
}

/// 网关统计
#[derive(Debug, Clone, Serialize)]
pub struct GatewayStats {
    pub service_name: String,
    pub total_requests: u64,
    pub total_errors: u64,
    pub upstream_services: usize,
    pub circuit_breakers: usize,
}

/// 网关构建器
pub struct GatewayBuilder {
    config: GatewayConfig,
}

impl GatewayBuilder {
    fn new() -> Self {
        Self { config: GatewayConfig::default() }
    }

    pub fn service_name(mut self, name: &str) -> Self {
        self.config.service_name = name.to_string();
        self
    }

    pub fn listen_addr(mut self, addr: &str) -> Self {
        self.config.listen_addr = addr.to_string();
        self
    }

    pub fn default_timeout_ms(mut self, ms: u64) -> Self {
        self.config.default_timeout_ms = ms;
        self
    }

    pub fn retry_attempts(mut self, n: u32) -> Self {
        for svc in &mut self.config.upstream_services {
            svc.retries = n;
        }
        self
    }

    pub fn rate_limit_per_second(mut self, rps: u64) -> Self {
        for svc in &mut self.config.upstream_services {
            svc.rate_limit_per_second = rps;
        }
        self
    }

    pub fn circuit_breaker_threshold(mut self, threshold: f64) -> Self {
        for svc in &mut self.config.upstream_services {
            svc.circuit_breaker_threshold = threshold;
        }
        self
    }

    pub fn upstream_service(mut self, svc: UpstreamService) -> Self {
        self.config.upstream_services.push(svc);
        self
    }

    pub fn build(self) -> Result<ApiGateway, String> {
        Ok(ApiGateway::new(self.config))
    }
}
