// =============================================================================
// MOX API 网关 - 主入口
// =============================================================================

mod app_state;
mod routes;

use crate::app_state::AppState;
use axum::http::Method;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mox_api_gateway=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    tracing::info!("MOX API 网关启动中...");

    // 创建应用状态
    let state = AppState::new();
    tracing::info!(
        service = %state.service_name,
        version = %state.version,
        "应用状态初始化完成"
    );

    // CORS 配置
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any);

    // 创建路由
    let app = routes::create_router(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    // 监听地址
    let host = std::env::var("MOX_GATEWAY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("MOX_GATEWAY_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{}:{}", host, port);

    tracing::info!(addr = %addr, "开始监听");

    // 启动服务器
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("绑定端口失败");

    tracing::info!("MOX API 网关已启动，监听于 {}", addr);

    axum::serve(listener, app)
        .await
        .expect("服务器运行失败");
}
