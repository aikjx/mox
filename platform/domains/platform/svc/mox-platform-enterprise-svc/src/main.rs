// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! enterprise-svc 二进制入口：监听 0.0.0.0:3002，提供企业级 HTTP 真源
//!
//! 启动参数（env）：
//! - LISTEN_ADDR        默认 0.0.0.0:3002
//! - DB_PATH            默认 :memory: （文件路径持久化）
//! - INSTALL_INDUSTRIES 默认 common,finance（逗号分隔）
//! - AUTH_SECRET        默认 off → 认证中间件关闭；"on" 或任意非空值开启并使用该 secret

use std::sync::Arc;

use axum::Router;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

use mox_platform_enterprise_svc::app_state::AppState;
use mox_platform_enterprise_svc::routes::build_router;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mox_platform_enterprise_svc::auth::init_logging();

    let listen_addr = env_or("LISTEN_ADDR", "0.0.0.0:3002");
    let db_path = env_or("DB_PATH", ":memory:");
    let install_industries_raw = env_or("INSTALL_INDUSTRIES", "common,finance");
    let install_industries: Vec<&str> = install_industries_raw
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let auth_secret_raw = env_or("AUTH_SECRET", "off");
    let auth_enabled = auth_secret_raw != "off";
    let auth_secret = if auth_enabled {
        auth_secret_raw.clone()
    } else {
        "enterprise-dev-secret-change-me".to_string()
    };
    std::env::set_var("AUTH_SECRET", &auth_secret);

    tracing::info!(
        "enterprise-svc starting: listen={} db={} industries={:?} auth={}",
        listen_addr,
        db_path,
        install_industries,
        if auth_enabled { "on" } else { "off" }
    );

    let app_state = Arc::new(AppState::open_memory_or_file(&db_path, &install_industries).await?);

    let auth_state = mox_platform_enterprise_svc::auth::AuthState::new(auth_secret, auth_enabled);

    let api_router = build_router(app_state.clone());

    let middleware = ServiceBuilder::new().layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
            .expose_headers(Any),
    );

    let app: Router = if auth_enabled {
        let auth_mw = tower::ServiceBuilder::new().layer(axum::middleware::from_fn_with_state(
            auth_state,
            mox_platform_enterprise_svc::auth::auth_middleware,
        ));
        Router::new()
            .merge(api_router)
            .layer(middleware)
            .layer(auth_mw)
    } else {
        Router::new().merge(api_router).layer(middleware)
    };

    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
    tracing::info!("enterprise-svc listening on http://{}", listen_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
