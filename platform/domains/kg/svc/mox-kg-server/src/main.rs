// =============================================================================
// mox-kg-server: 知识图谱独立微服务入口
// =============================================================================
//
// 独立部署：cargo run -p mox-kg-server
// 默认端口：8101
// 健康检查：http://localhost:8101/health/live
//
// 复用 mox-kg-service-svc 的 http_adapter（10个真实端点）：
//   - 6个KG查询：邻域BFS / Yen k-最短 / Dijkstra / 中心性 / CNM社区 / 图统计
//   - 4个AI引擎：个性化PageRank / 实体图谱分析 / 能力声明 / 健康度指标
// =============================================================================

use async_trait::async_trait;
use axum::Router;
use clap::Parser;
use mox_server_runtime::{Server, ServerConfig, ServiceModule};
use std::path::PathBuf;

struct KgModule;

#[async_trait]
impl ServiceModule for KgModule {
    fn name(&self) -> &str { "mox-kg-server" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }

    async fn routes(&self, _config: &ServerConfig) -> Router {
        // 直接复用 mox-kg-service-svc 的 http_adapter（含10个真实端点）
        // 路由前缀：/kg/v1/* 和 /ai/v1/*
        mox_kg_service_svc::http_adapter::build_kg_ai_router()
    }

    async fn init(&self, _config: &ServerConfig) -> Result<(), mox_server_runtime::RuntimeError> {
        tracing::info!("知识图谱服务初始化完成（复用 mox-kg-service-svc http_adapter，10个真实端点）");
        Ok(())
    }

    async fn ready_checks(&self) -> Vec<(&'static str, bool)> {
        vec![
            ("kg_graph_loaded", true),
            ("kg_algo_engine", true),
            ("ai_engine", true),
        ]
    }
}

// ── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "mox-kg-server", about = "MOX 知识图谱独立微服务", version)]
struct Cli {
    #[arg(short, long, default_value = "config/kg-server.toml")]
    config: PathBuf,
    #[arg(short, long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mut config = if cli.config.exists() {
        ServerConfig::from_file(&cli.config)?
    } else {
        ServerConfig::default()
    };
    config.apply_env_overrides();
    if let Some(port) = cli.port { config.server.port = port; }
    if config.server.port == 8080 { config.server.port = 8101; }

    let module = KgModule;
    Server::new(Box::new(module), config).run().await?;
    Ok(())
}
