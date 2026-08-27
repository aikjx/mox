//! MOX L1 企业级网关库（Rust 纯代码，无 Node.js 依赖）
//!
//! 本版本采用"最小可运行 + 真实业务挂接"的保守策略：
//! 1. 不编译历史遗留的 auth/cli/o11y/http_server/rate_limit/config/routing 7 模块
//!    （以上模块存在大量未迁移的 API 引用问题，待后续逐模块迁移后再启用）
//! 2. 直接挂接唯一就绪的业务域：`mox-kg-service-svc` 的 http_adapter
//!    （6 KG + 4 AI 接口，已通过 cargo check）
//! 3. 暴露 2 个通用端点：/health · /api/v1/status
//!
//! → 总计 10 + 2 = 12 个真实接口，端口 0.0.0.0:8080

pub use mox_kg_service_svc::http_adapter;

use axum::{Json, Router, routing::get};
use serde_json::json;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

/// 构建企业级网关 Router：12 端点
///  - L0 通用：/health · /api/v1/status
///  - L2 KG：/kg/v1/{neighborhood,path,shortest-path,centrality,communities,stats}
///  - L3 AI：/ai/engine/{process,analyze,capabilities,metrics}
pub fn build_gateway_router() -> Router {
    let l0 = Router::new()
        .route("/health", get(|| async {
            Json(json!({
                "ok": true,
                "gateway": "rust-axum",
                "bind": "0.0.0.0:8080",
                "replaced_backend_node_ports": [3000, 3001, 3002],
                "ts": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            }))
        }))
        .route("/api/v1/status", get(|| async {
            Json(json!({
                "ok": true,
                "domains_ready": ["kg/v1", "ai/engine"],
                "domains_stub_count": 28,
                "endpoints_ready": 12,
                "note": "全面接管 backend-node 3000/3001/3002 → Rust 8080；其余 28 域待逐模块迁移。",
            }))
        }));

    // 真实 KG+AI 业务路由（来自 mox-kg-service-svc/src/http_adapter.rs）
    let kg_ai = http_adapter::build_kg_ai_router();

    Router::new()
        .merge(l0)
        .merge(kg_ai)
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
}

/// 启动网关：绑定 0.0.0.0:8080，Ctrl-C 优雅退出
pub async fn serve_forever(bind_addr: &str, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = build_gateway_router();
    let addr: SocketAddr = format!("{bind_addr}:{port}").parse()?;

    eprintln!("====================================================================");
    eprintln!("  🚀 MOX Rust Gateway 全维接管 @ http://{addr}");
    eprintln!("====================================================================");
    eprintln!("  替换端口：3000 (Node 静态+代理) / 3001 (Rust operator) / 3002 (Rust enterprise)");
    eprintln!("  L0 通用：   /health · /api/v1/status");
    eprintln!("  L2 KG：     /kg/v1/neighborhood · /kg/v1/path · /kg/v1/shortest-path");
    eprintln!("             /kg/v1/centrality · /kg/v1/communities · /kg/v1/stats");
    eprintln!("  L3 AI：     /ai/engine/process · /ai/engine/analyze");
    eprintln!("             /ai/engine/capabilities · /ai/engine/metrics");
    eprintln!("  其余 28 域：stub 占位，待逐模块迁移（见迁移覆盖度报告）");
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
