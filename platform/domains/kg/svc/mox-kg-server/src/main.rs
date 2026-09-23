// =============================================================================
// mox-kg-server: 知识图谱独立微服务入口
// =============================================================================
//
// 独立部署：cargo run -p mox-kg-server
// 默认端口：3411
// 健康检查：http://localhost:3411/health/live
//
// 复用 mox-kg-service-svc 的 http_adapter（10个真实端点）：
//   - 6个KG查询：邻域BFS / Yen k-最短 / Dijkstra / 中心性 / CNM社区 / 图统计
//   - 4个AI引擎：个性化PageRank / 实体图谱分析 / 能力声明 / 健康度指标
// =============================================================================

use async_trait::async_trait;
use axum::Router;
use clap::Parser;
use mox_kg_storage_svc::storage_server::StorageServer;
use mox_server_runtime::{Server, ServerConfig, ServiceModule};
use std::path::PathBuf;
use std::sync::Arc;

struct KgModule {
    storage: Arc<StorageServer>,
}

impl KgModule {
    /// 引擎选择归一化：MOX_KG_DATA_DIR 显式指定目录；
    /// 未指定时，persist-rocksdb 构建默认 `data/kg-storage`（部署即 durable），
    /// 内存构建回退临时目录。分片数 MOX_KG_SHARD_COUNT（2^k，默认 16）。
    fn open_storage() -> Result<Arc<StorageServer>, Box<dyn std::error::Error>> {
        let shard_count: u16 = std::env::var("MOX_KG_SHARD_COUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(16);
        let path: Option<PathBuf> = match std::env::var("MOX_KG_DATA_DIR") {
            Ok(dir) if !dir.trim().is_empty() => Some(PathBuf::from(dir)),
            _ if cfg!(feature = "persist-rocksdb") => Some(PathBuf::from("data/kg-storage")),
            _ => None,
        };
        if let Some(p) = &path {
            std::fs::create_dir_all(p)?;
        }
        let srv = StorageServer::start_cluster(shard_count, &[], path.as_deref())?;
        Ok(Arc::new(srv))
    }
}

#[async_trait]
impl ServiceModule for KgModule {
    fn name(&self) -> &str { "mox-kg-server" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }

    async fn routes(&self, _config: &ServerConfig) -> Router {
        // 直接复用 mox-kg-service-svc 的 http_adapter（含10个真实端点）
        // 路由前缀：/kg/v1/* 和 /ai/v1/*；存储写读面：/storage/v1/*
        mox_kg_service_svc::http_adapter::build_kg_ai_router().nest(
            "/storage/v1",
            mox_kg_storage_svc::http_api::build_storage_router(self.storage.clone()),
        )
    }

    async fn init(&self, _config: &ServerConfig) -> Result<(), mox_server_runtime::RuntimeError> {
        tracing::info!(
            "知识图谱服务初始化完成（http_adapter 10 端点 + storage {} 分片，rocksdb_persist={}",
            self.storage.shard_vertex_counts().len(),
            cfg!(feature = "persist-rocksdb"),
        );
        Ok(())
    }

    async fn ready_checks(&self) -> Vec<(&'static str, bool)> {
        let storage_ok = self
            .storage
            .read_vertex("__kg_server_ready_probe__")
            .is_none(); // 探针 vid 本就不存在；真正校验是引擎句柄可查询不 panic
        vec![
            ("kg_graph_loaded", true),
            ("kg_algo_engine", true),
            ("ai_engine", true),
            ("kg_storage_engine", storage_ok),
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
        let mut config = ServerConfig::default();
        config.server.port = 3411;
        config
    };
    config.apply_env_overrides();
    if let Some(port) = cli.port { config.server.port = port; }

    let module = KgModule {
        storage: KgModule::open_storage()?,
    };
    Server::new(Box::new(module), config).run().await?;
    Ok(())
}
