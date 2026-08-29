// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! mox-ai-intent-svc · 入口

use mox_ai_intent_svc::{AppState, router};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 读取端口
    let port: u16 = std::env::var("MOX_AI_INTENT_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8765);

    let state = AppState::new();
    let app = router(state).layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("🤖 mox-ai-intent-svc 启动成功: http://{}", addr);
    tracing::info!("  POST /api/v1/intent/understand       端到端意图理解");
    tracing::info!("  POST /api/v1/intent/extract-entities 实体提取");
    tracing::info!("  POST /api/v1/intent/decompose        任务拆解");
    tracing::info!("  GET  /api/v1/intent/definitions      内置意图列表");
    tracing::info!("  POST /api/v1/sessions                创建会话");
    tracing::info!("  POST /api/v1/sessions/:id/chat       对话");
    tracing::info!("  GET  /health                          健康检查");

    axum::serve(listener, app).await?;
    Ok(())
}
