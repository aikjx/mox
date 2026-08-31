//! Mox 平台网关运行时入口
//!
//! 启动 HTTP/gRPC 服务，注册路由，装配中间件

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod routes;
mod middleware;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载配置
    dotenvy::dotenv().ok();

    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Mox Platform Gateway starting...");
    tracing::info!("Version: {}", mox_platform_gateway_runtime::VERSION);

    // 加载配置
    let config = config::AppConfig::load()?;
    tracing::info!("Config loaded: server on {}:{}", config.server.host, config.server.port);

    // 构建路由
    let app = routes::build_router();

    // 启动 HTTP 服务
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("HTTP server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
