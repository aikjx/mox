// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX Enterprise · API 网关入口
//!
//! 启动 API 网关，集成限流、熔断、重试、路由、零信任认证

use axum::{routing::any, Router};
use mox_enterprise_backend::api::{api_router, AppState};
use mox_enterprise_backend::api_gateway::ApiGateway;
use std::net::SocketAddr;
use tokio::signal;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "mox_gateway=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    info!("MOX Enterprise API Gateway v2.0.0 启动中...");

    // 创建网关
    let gateway = ApiGateway::builder()
        .service_name("mox-api-gateway")
        .listen_addr("0.0.0.0:8080")
        .default_timeout_ms(30000)
        .retry_attempts(3)
        .rate_limit_per_second(1000)
        .circuit_breaker_threshold(0.5)
        .build().map_err(|e| anyhow::anyhow!(e))?;

    info!("网关配置完成: 监听 {}, 限流 {} QPS, 重试 {} 次",
        gateway.listen_addr(), gateway.rate_limit(), gateway.retry_attempts());

    // 构建路由
    let app_state = AppState::default();
    let app = Router::new()
        .route("/health", any(health_handler))
        .route("/ready", any(ready_handler))
        .nest("/api", api_router(app_state))
        .fallback(any(gateway.proxy_handler()));

    // 启动服务
    let addr: SocketAddr = gateway.listen_addr().parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("网关监听于 {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| anyhow::anyhow!("服务错误: {}", e))?;

    info!("网关已优雅关闭");
    Ok(())
}

async fn health_handler() -> &'static str {
    "{\"status\":\"healthy\"}"
}

async fn ready_handler() -> &'static str {
    "{\"status\":\"ready\"}"
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
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
        _ = ctrl_c => { info!("收到 SIGINT，开始优雅关闭..."); }
        _ = terminate => { info!("收到 SIGTERM，开始优雅关闭..."); }
    }
}
