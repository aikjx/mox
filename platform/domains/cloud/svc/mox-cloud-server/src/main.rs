// =============================================================================
// mox-cloud-server: 云盘独立微服务入口（控制面 Master）
// =============================================================================
//
// 独立部署：cargo run -p mox-cloud-server
// 默认端口：8102
// 健康检查：http://localhost:8102/health/live
//
// 基于 mox-cloud-master-svc 的 MasterServer 构建 REST API：
//   - 卷注册 / 心跳 / 分配 / 列表
//   - 快照 / 恢复
//   - 指标 / Leader 状态
// =============================================================================

use async_trait::async_trait;
use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use mox_cloud_master_svc::{MasterConfig, MasterServer, VolumeLoadReport};
use mox_server_runtime::{Server, ServerConfig, ServiceModule};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

// ── 请求/响应类型 ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RegisterVolumeRequest {
    addr: String,
    capacity: u64,
}

#[derive(Debug, Deserialize)]
struct AllocateVolumeRequest {
    size: u64,
    #[serde(default = "default_replica")]
    replica: u8,
}
fn default_replica() -> u8 { 3 }

#[derive(Debug, Deserialize)]
struct RestoreRequest {
    snapshot_id: String,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Self { Self { success: true, data: Some(data), error: None } }
    fn err(msg: impl Into<String>) -> Self { Self { success: false, data: None, error: Some(msg.into()) } }
}

// ── 模块 ─────────────────────────────────────────────────────────────────────

struct CloudModule {
    master: Arc<MasterServer>,
}

impl CloudModule {
    fn new() -> Self {
        let config = MasterConfig::default();
        Self { master: Arc::new(MasterServer::new(config)) }
    }
}

#[async_trait]
impl ServiceModule for CloudModule {
    fn name(&self) -> &str { "mox-cloud-server" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }

    async fn routes(&self, _config: &ServerConfig) -> Router {
        let master = self.master.clone();
        Router::new()
            // 卷管理
            .route("/api/v1/cloud/volumes", get(list_volumes_handler))
            .route("/api/v1/cloud/volumes/register", post(register_volume_handler))
            .route("/api/v1/cloud/volumes/allocate", post(allocate_volume_handler))
            .route("/api/v1/cloud/volumes/{id}/heartbeat", post(heartbeat_handler))
            .route("/api/v1/cloud/volumes/{id}/snapshot", post(snapshot_handler))
            .route("/api/v1/cloud/volumes/{id}/restore", post(restore_handler))
            // 集群状态
            .route("/api/v1/cloud/metrics", get(metrics_handler))
            .route("/api/v1/cloud/leader", get(leader_handler))
            .layer(Extension(master))
    }

    async fn init(&self, _config: &ServerConfig) -> Result<(), mox_server_runtime::RuntimeError> {
        tracing::info!("云盘 Master 服务初始化完成（卷管理/调度/快照/Raft）");
        Ok(())
    }

    async fn ready_checks(&self) -> Vec<(&'static str, bool)> {
        vec![
            ("cloud_master", true),
            ("volume_allocator", true),
            ("snapshot_manager", true),
        ]
    }
}

// ── 处理器 ───────────────────────────────────────────────────────────────────

async fn list_volumes_handler(
    Extension(master): Extension<Arc<MasterServer>>,
) -> impl IntoResponse {
    let volumes = master.list_volumes();
    (StatusCode::OK, Json(ApiResponse::ok(json!({ "volumes": volumes, "total": volumes.len() }))))
}

async fn register_volume_handler(
    Extension(master): Extension<Arc<MasterServer>>,
    Json(req): Json<RegisterVolumeRequest>,
) -> impl IntoResponse {
    if req.addr.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value>::err("addr 不能为空")));
    }
    let volume_id = master.register_volume(req.addr, req.capacity);
    (StatusCode::CREATED, Json(ApiResponse::ok(json!({ "volume_id": volume_id }))))
}

async fn allocate_volume_handler(
    Extension(master): Extension<Arc<MasterServer>>,
    Json(req): Json<AllocateVolumeRequest>,
) -> impl IntoResponse {
    match master.allocate_volume(req.size, req.replica) {
        Ok(alloc) => (StatusCode::OK, Json(ApiResponse::<serde_json::Value>::ok(json!(alloc)))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<serde_json::Value>::err(e.to_string()))),
    }
}

async fn heartbeat_handler(
    Extension(master): Extension<Arc<MasterServer>>,
    Path(id): Path<String>,
    Json(load): Json<VolumeLoadReport>,
) -> impl IntoResponse {
    match master.heartbeat(&id, load) {
        Ok(()) => (StatusCode::OK, Json(ApiResponse::ok(json!({ "status": "heartbeat_accepted" })))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value>::err(e.to_string()))),
    }
}

async fn snapshot_handler(
    Extension(master): Extension<Arc<MasterServer>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match master.snapshot_volume(&id) {
        Ok(snap_id) => (StatusCode::OK, Json(ApiResponse::ok(json!({ "snapshot_id": snap_id })))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<serde_json::Value>::err(e.to_string()))),
    }
}

async fn restore_handler(
    Extension(master): Extension<Arc<MasterServer>>,
    Path(id): Path<String>,
    Json(req): Json<RestoreRequest>,
) -> impl IntoResponse {
    match master.restore_snapshot(&id, &req.snapshot_id) {
        Ok(()) => (StatusCode::OK, Json(ApiResponse::ok(json!({ "status": "restored" })))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<serde_json::Value>::err(e.to_string()))),
    }
}

async fn metrics_handler(
    Extension(master): Extension<Arc<MasterServer>>,
) -> impl IntoResponse {
    let metrics = master.get_metrics();
    (StatusCode::OK, Json(ApiResponse::ok(metrics)))
}

async fn leader_handler(
    Extension(master): Extension<Arc<MasterServer>>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(ApiResponse::ok(json!({
        "is_leader": master.is_leader(),
        "leader_addr": master.leader_addr(),
    }))))
}

// ── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "mox-cloud-server", about = "MOX 云盘独立微服务（Master 控制面）", version)]
struct Cli {
    #[arg(short, long, default_value = "config/cloud-server.toml")]
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
    if config.server.port == 8080 { config.server.port = 8102; }

    let module = CloudModule::new();
    Server::new(Box::new(module), config).run().await?;
    Ok(())
}
