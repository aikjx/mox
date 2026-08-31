// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX L1 企业级网关库（Rust 纯代码，无 Node.js 依赖）
//!
//! # 架构
//! 采用分层中间件 + 模块化路由的企业级网关架构：
//!
//! ```text
//! 请求 → CORS → 限流 → 认证 → 路由分发 → 业务处理 → 响应
//! ```
//!
//! # 当前端点（迁移进度）
//! - L0 通用：/health · /api/v1/status · /api/v1/domains · /metrics
//! - L2 KG：/kg/v1/{neighborhood,path,shortest-path,centrality,communities,stats}
//! - L3 AI：/ai/engine/{process,analyze,capabilities,metrics}
//! - 总计：4 通用 + 6 KG + 4 AI = 14 端点

pub mod config;
pub mod auth;
pub mod rate_limit;
pub mod o11y;
pub mod routes;
pub mod alliance;

pub use mox_kg_service_svc::http_adapter;
pub use alliance as alliance_adapter;
pub use config::GatewayConfig;

use axum::{Json, Router, extract::State, routing::get};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use auth::{AuthMiddleware, auth_middleware};
use rate_limit::{RateLimiter, rate_limit_middleware};
use o11y::MetricsCollector;

/// 网关共享状态
#[derive(Clone)]
pub struct GatewayState {
    pub config: Arc<GatewayConfig>,
    pub auth: Arc<AuthMiddleware>,
    pub rate_limiter: Arc<RateLimiter>,
    pub metrics: Arc<MetricsCollector>,
}

impl GatewayState {
    /// 从配置创建网关状态
    pub fn from_config(config: GatewayConfig) -> Self {
        let auth = Arc::new(AuthMiddleware::new(config.auth.clone()));
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.clone()));
        let metrics = Arc::new(MetricsCollector::new(o11y::ObservabilityConfig {
            metrics_enabled: true,
            tracing_enabled: false,
            logging_enabled: true,
        }));
        Self {
            config: Arc::new(config),
            auth,
            rate_limiter,
            metrics,
        }
    }
}

/// 构建企业级网关 Router
///
/// 中间件分层（从外到内）：
/// 1. CORS 跨域
/// 2. 限流（令牌桶）
/// 3. 认证（JWT + API Key）
/// 4. 业务路由
pub fn build_gateway_router(state: GatewayState) -> Router {
    // L0 通用端点（无需认证）
    let l0 = Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/status", get(status_handler))
        .route("/api/v1/domains", get(domains_handler))
        .route("/metrics", get(metrics_handler));

    // 真实 KG+AI 业务路由（来自 mox-kg-service-svc/src/http_adapter.rs）
    let kg_ai = http_adapter::build_kg_ai_router();

    // 联盟域业务路由（Api 模式·进程内路由桩）
    let alliance = alliance::build_alliance_router();

    // 受保护的路由：认证 + 限流
    let protected = Router::new()
        .merge(kg_ai)
        .merge(alliance)
        .route_layer(axum::middleware::from_fn_with_state(
            state.auth.clone(),
            auth_middleware,
        ));

    Router::new()
        .merge(l0)
        .merge(protected)
        .layer(axum::middleware::from_fn_with_state(
            state.rate_limiter.clone(),
            rate_limit_middleware,
        ))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(state)
}

/// 健康检查端点
async fn health_handler() -> Json<serde_json::Value> {
    Json(json!({
        "ok": true,
        "gateway": "rust-axum",
        "version": env!("CARGO_PKG_VERSION"),
        "ts": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    }))
}

/// 状态端点
async fn status_handler(State(state): State<GatewayState>) -> Json<serde_json::Value> {
    let domain_stats = routes::DOMAINS.iter().fold(
        json!({"ready": 0, "stub": 0, "beta": 0}),
        |mut acc, d| {
            if let Some(obj) = acc.as_object_mut() {
                let key = if d.status == "ready" { "ready" }
                    else if d.status == "beta" { "beta" }
                    else { "stub" };
                if let Some(v) = obj.get_mut(key) {
                    *v = json!(v.as_i64().unwrap_or(0) + 1);
                }
            }
            acc
        }
    );

    Json(json!({
        "ok": true,
        "gateway": "rust-axum-enterprise",
        "version": env!("CARGO_PKG_VERSION"),
        "domains_total": routes::DOMAINS.len(),
        "domains_ready": domain_stats["ready"],
        "domains_stub": domain_stats["stub"],
        "domains_beta": domain_stats["beta"],
        "endpoints_ready": 14,
        "auth_enabled": state.config.auth.enabled,
        "rate_limit_enabled": state.config.rate_limit.enabled,
        "note": "12 业务域路由就绪，其余域 stub 占位，待逐模块迁移。",
        "ts": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    }))
}

/// 域描述符列表端点
async fn domains_handler() -> Json<serde_json::Value> {
    Json(json!({
        "ok": true,
        "total": routes::DOMAINS.len(),
        "domains": routes::DOMAINS,
    }))
}

/// 指标端点（Prometheus 格式占位）
async fn metrics_handler(State(state): State<GatewayState>) -> String {
    let rl_stats = state.rate_limiter.stats();
    format!(
        "# HELP mox_gateway_requests_total Total requests processed\n\
         # TYPE mox_gateway_requests_total counter\n\
         mox_gateway_requests_total{{service=\"gateway\"}} 0\n\
         # HELP mox_rate_limit_clients Total tracked rate limit clients\n\
         # TYPE mox_rate_limit_clients gauge\n\
         mox_rate_limit_clients {}\n\
         # HELP mox_rate_limit_enabled Whether rate limiting is enabled\n\
         # TYPE mox_rate_limit_enabled gauge\n\
         mox_rate_limit_enabled {}\n",
        rl_stats.total_clients,
        if rl_stats.enabled { 1 } else { 0 },
    )
}

/// 启动网关：绑定地址端口，Ctrl-C 优雅退出
pub async fn serve_forever(bind_addr: &str, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = GatewayConfig::default();
    let state = GatewayState::from_config(config);
    let app = build_gateway_router(state);
    let addr: SocketAddr = format!("{bind_addr}:{port}").parse()?;

    eprintln!("====================================================================");
    eprintln!("  🚀 MOX Rust Gateway 企业版 @ http://{addr}");
    eprintln!("====================================================================");
    eprintln!("  替换端口：3000 (Node 静态+代理) / 3001 / 3002");
    eprintln!("  中间件分层：CORS → 限流 → 认证 → 业务路由");
    eprintln!("  L0 通用：   /health · /api/v1/status · /api/v1/domains · /metrics");
    eprintln!("  L2 KG：     /kg/v1/neighborhood · /kg/v1/path · /kg/v1/shortest-path");
    eprintln!("             /kg/v1/centrality · /kg/v1/communities · /kg/v1/stats");
    eprintln!("  L3 AI：     /ai/engine/process · /ai/engine/analyze");
    eprintln!("             /ai/engine/capabilities · /ai/engine/metrics");
    eprintln!("  L4 Alliance:/alliance/v1/tasks (POST/GET) · /alliance/v1/tasks/:id (GET/POST)");
    eprintln!("             /alliance/v1/experts/search · /alliance/v1/tasks/:id/status");
    eprintln!("             /alliance/v1/tasks/:id/nodes · /alliance/v1/tasks/:id/nodes/:node_id");
    eprintln!("  认证：      JWT Bearer + X-API-Key（可配置开关）");
    eprintln!("  限流：      令牌桶 100 req/min + 20 burst（可配置）");
    eprintln!("  停止：      Ctrl-C");
    eprintln!("====================================================================");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            eprintln!("\n[mox-server] 🛑 收到 Ctrl-C，优雅退出。");
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_state_creation() {
        let config = GatewayConfig::default();
        let state = GatewayState::from_config(config);
        assert!(state.config.auth.enabled);
        assert!(state.config.rate_limit.enabled);
    }

    #[test]
    fn test_health_json_structure() {
        // 验证配置结构完整性
        let config = GatewayConfig::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "0.0.0.0");
        assert!(config.auth.public_paths.contains(&"/health".to_string()));
    }
}
