// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! mox-project-graph-svc · 入口

use mox_project_graph_svc::{AppState, router};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let port: u16 = std::env::var("MOX_PROJECT_GRAPH_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8770);

    let state = AppState::new();
    let app = router(state).layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("📊 mox-project-graph-svc 启动成功: http://{}", addr);
    tracing::info!("  项目: POST/GET/PUT /api/v1/projects");
    tracing::info!("  需求: POST/GET/PUT /api/v1/requirements");
    tracing::info!("  任务: POST/GET/PUT /api/v1/tasks");
    tracing::info!("  人员: POST/GET /api/v1/persons");
    tracing::info!("  里程碑: POST/GET /api/v1/projects/:id/milestones");
    tracing::info!("  问题: POST/GET /api/v1/projects/:id/issues");
    tracing::info!("  依赖: POST /api/v1/dependencies");
    tracing::info!("  图谱遍历: POST /api/v1/graph/traverse");
    tracing::info!("  影响分析: GET /api/v1/graph/impact/:id");
    tracing::info!("  关键路径: GET /api/v1/projects/:id/critical-path");
    tracing::info!("  健康检查: GET /health");

    axum::serve(listener, app).await?;
    Ok(())
}
