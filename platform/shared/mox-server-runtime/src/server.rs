// =============================================================================
// 统一 HTTP 服务器（Server）
// =============================================================================
//
// 封装 axum 服务器启动，提供：
// - 统一中间件链（CORS / 超时 / 请求体限制 / 追踪 / 日志）
// - 自动挂载健康检查端点
// - 优雅停机（SIGTERM / SIGINT）
// - 服务模块路由注册
// =============================================================================

use crate::config::ServerConfig;
use crate::health::HealthRegistry;
use crate::shutdown::shutdown_signal;
use crate::{init_logging, RuntimeError, RuntimeResult, ServiceModule};
use axum::{
    extract::{DefaultBodyLimit, Extension},
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

/// 共享应用状态（通过 Extension 注入，保持 Router<()>）
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServerConfig>,
    pub health: Arc<HealthRegistry>,
    pub service_name: String,
    pub service_version: String,
}

/// 统一 HTTP 服务器
pub struct Server {
    module: Box<dyn ServiceModule>,
    config: ServerConfig,
}

impl Server {
    /// 创建服务器实例
    pub fn new(module: Box<dyn ServiceModule>, config: ServerConfig) -> Self {
        Self { module, config }
    }

    /// 构建路由（健康检查 + 业务路由 + 中间件），返回 Router<()>
    async fn build_router(&self) -> Router {
        let health = Arc::new(HealthRegistry::new(self.module.name().to_string()));
        let state = Arc::new(AppState {
            config: Arc::new(self.config.clone()),
            health: health.clone(),
            service_name: self.module.name().to_string(),
            service_version: self.module.version().to_string(),
        });

        // 健康检查路由
        let health_routes = Router::new()
            .route("/live", get(live_handler))
            .route("/ready", get(ready_handler))
            .route("/metrics", get(metrics_handler));

        // 业务路由
        let business_routes = self.module.routes(&self.config).await;

        // 合并路由 + 中间件（Extension 注入 state，保持 Router<()>）
        Router::new()
            .nest("/health", health_routes)
            .merge(business_routes)
            .layer(
                ServiceBuilder::new()
                    .layer(Extension(state))
                    .layer(TraceLayer::new_for_http())
                    .layer(TimeoutLayer::new(Duration::from_secs(
                        self.config.server.timeout_secs,
                    )))
                    .layer(DefaultBodyLimit::max(
                        self.config.server.body_limit,
                    ))
                    .layer(
                        CorsLayer::new()
                            .allow_origin(Any)
                            .allow_methods([
                                Method::GET,
                                Method::POST,
                                Method::PUT,
                                Method::DELETE,
                                Method::OPTIONS,
                            ])
                            .allow_headers(Any),
                    ),
            )
    }

    /// 启动服务器（阻塞直到停机信号）
    pub async fn run(self) -> RuntimeResult<()> {
        let service_name = self.module.name().to_string();
        let service_version = self.module.version().to_string();

        // 初始化日志
        init_logging(
            &service_name,
            self.config.observability.json_log,
            &self.config.observability.log_level,
        );

        tracing::info!(
            service = %service_name,
            version = %service_version,
            listen = %self.config.listen_addr(),
            "MOX 服务启动中..."
        );

        // 模块初始化
        self.module
            .init(&self.config)
            .await
            .map_err(|e| RuntimeError::InitError(e.to_string()))?;

        // 构建路由
        let app = self.build_router().await;
        let addr = self.config.listen_addr();
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| RuntimeError::ServerError(format!("绑定 {addr} 失败: {e}")))?;

        tracing::info!(
            service = %service_name,
            listen = %addr,
            "MOX 服务已启动，健康检查: http://{}/health/live",
            addr
        );

        // 启动服务器（带优雅停机）
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| RuntimeError::ServerError(format!("服务器运行错误: {e}")))?;

        // 模块优雅关闭
        tracing::info!(service = %service_name, "正在关闭服务...");
        self.module.shutdown().await;
        tracing::info!(service = %service_name, "服务已安全关闭");

        Ok(())
    }
}

// ── 健康检查处理器 ─────────────────────────────────────────────────────────

async fn live_handler(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "alive",
            "service": state.service_name,
            "version": state.service_version,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
}

async fn ready_handler(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let checks = state.health.check_all().await;
    let all_ready = checks.iter().all(|(_, ok)| *ok);
    let status = if all_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(serde_json::json!({
            "status": if all_ready { "ready" } else { "not_ready" },
            "service": state.service_name,
            "checks": checks.iter().map(|(name, ok)| {
                serde_json::json!({ "name": name, "ok": ok })
            }).collect::<Vec<_>>(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
}

async fn metrics_handler(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let metrics = state.health.metrics();
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4")],
        metrics,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_live_handler() {
        let state = Arc::new(AppState {
            config: Arc::new(ServerConfig::default()),
            health: Arc::new(HealthRegistry::new("test".to_string())),
            service_name: "test".to_string(),
            service_version: "1.0.0".to_string(),
        });
        let response = live_handler(Extension(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
