// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 标准化服务器启动器 — 统一生命周期/优雅关停/零配置

use axum::{middleware, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::config::FrameworkConfig;
use crate::health::HealthChecker;
use crate::metrics::MetricsCollector;

/// 标准化服务器
pub struct FrameworkServer {
    config: FrameworkConfig,
    health: HealthChecker,
    metrics: MetricsCollector,
    app_router: Option<Router>,
}

impl FrameworkServer {
    /// 创建服务器（零配置，使用默认值+环境变量）
    pub fn new(service_name: impl Into<String>) -> Self {
        let mut config = FrameworkConfig::from_env();
        config.service_name = service_name.into();
        let health = HealthChecker::new(&config.service_name, &config.service_version);
        let metrics = MetricsCollector::new(&config.service_name);
        Self {
            config,
            health,
            metrics,
            app_router: None,
        }
    }

    /// 从配置创建
    pub fn from_config(config: FrameworkConfig) -> Self {
        let health = HealthChecker::new(&config.service_name, &config.service_version);
        let metrics = MetricsCollector::new(&config.service_name);
        Self {
            config,
            health,
            metrics,
            app_router: None,
        }
    }

    /// 设置业务路由
    pub fn with_router(mut self, router: Router) -> Self {
        self.app_router = Some(router);
        self
    }

    /// 注册健康检查组件
    pub async fn register_health_component(&self, name: impl Into<String>) {
        self.health.register_component(name).await;
    }

    /// 获取配置引用
    pub fn config(&self) -> &FrameworkConfig {
        &self.config
    }

    /// 获取健康检查器
    pub fn health(&self) -> &HealthChecker {
        &self.health
    }

    /// 获取指标收集器
    pub fn metrics(&self) -> &MetricsCollector {
        &self.metrics
    }

    /// 构建完整路由（业务路由 + 健康 + 指标 + 中间件）
    fn build_router(&self) -> Router {
        let app = self.app_router.clone().unwrap_or_default();

        // 合并路由
        let router = Router::new()
            .merge(app)
            .merge(self.health.routes())
            .merge(self.metrics.routes());

        // 添加中间件
        router
            .layer(CorsLayer::permissive())
            .layer(TraceLayer::new_for_http())
    }

    /// 启动服务器（阻塞直到收到关停信号）
    pub async fn run(self) -> std::io::Result<()> {
        let router = self.build_router();
        let addr: SocketAddr = self.config.listen_addr.parse().unwrap_or_else(|_| {
            tracing::warn!("Invalid listen_addr, using default 0.0.0.0:8080");
            "0.0.0.0:8080".parse().unwrap()
        });

        tracing::info!(
            service = %self.config.service_name,
            version = %self.config.service_version,
            addr = %addr,
            "server starting"
        );

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        tracing::info!(service = %self.config.service_name, "server stopped gracefully");
        Ok(())
    }
}

/// 关停信号监听（Ctrl+C + SIGTERM）
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("received Ctrl+C, shutting down");
        }
        _ = terminate => {
            tracing::info!("received SIGTERM, shutting down");
        }
    }
}
