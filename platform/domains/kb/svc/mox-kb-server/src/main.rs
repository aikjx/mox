// =============================================================================
// mox-kb-server: 知识库独立微服务入口
// =============================================================================
//
// 独立部署：cargo run -p mox-kb-server -- --config /etc/mox/kb.toml
// 默认端口：3414
// 健康检查：http://localhost:3414/health/live
//
// 从 kg/svc/mox-kb-svc 独立迁出，成为 kb 域的独立微服务。
// 提供：文档管理 / 版本控制 / 全文检索 / 知识分析 / 关联链接 / 专家门禁
// =============================================================================

use async_trait::async_trait;
use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use clap::Parser;
use mox_kb_core::{Document, KbError, KbManager, SearchQuery, SqliteKbStore};
use mox_server_runtime::{Server, ServerConfig, ServiceModule};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};

struct KbModule {
    manager: Arc<KbManager>,
}

impl KbModule {
    fn new() -> Self {
        // SQLite + FTS5 持久化：data/kb.db（可用 KB_DB_PATH 环境变量覆盖路径）
        let store = Box::new(SqliteKbStore::open_default().expect("打开知识库 SQLite 失败"));
        Self { manager: Arc::new(KbManager::new(store)) }
    }
}

#[async_trait]
impl ServiceModule for KbModule {
    fn name(&self) -> &str {
        "mox-kb-server"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    async fn routes(&self, _config: &ServerConfig) -> Router {
        let manager = self.manager.clone();
        Router::new()
            .route("/api/v1/kb/info", get(kb_info_handler))
            .route(
                "/api/v1/kb/documents",
                get(list_documents_handler).post(create_document_handler),
            )
            .route(
                "/api/v1/kb/documents/:id",
                get(get_document_handler)
                    .put(update_document_handler)
                    .delete(delete_document_handler),
            )
            .route("/api/v1/kb/documents/:id/versions", get(list_versions_handler))
            .route("/api/v1/kb/search", get(search_handler))
            .layer(Extension(manager))
    }
    async fn init(&self, _config: &ServerConfig) -> Result<(), mox_server_runtime::RuntimeError> {
        tracing::info!("知识库服务初始化完成（独立域，从 kg 迁出）");
        Ok(())
    }
    async fn ready_checks(&self) -> Vec<(&'static str, bool)> {
        let ready = self
            .manager
            .search(&SearchQuery { page_size: 1, ..Default::default() })
            .await
            .is_ok();
        vec![("kb_storage", ready), ("kb_search", ready)]
    }
}

// ── 处理器 ──────────────────────────────────────────────────────────────────

async fn kb_info_handler() -> Json<serde_json::Value> {
    Json(json!({
        "service": "mox-kb-server",
        "module": "knowledge-base",
        "capabilities": ["document_management", "version_control", "fulltext_search"],
        "status": "running",
        "planned_capabilities": ["knowledge_analysis", "relation_linking", "expert_gate"],
        "note": "独立域，从 kg/svc/mox-kb-svc 迁出",
    }))
}

fn failed(error: KbError) -> Response {
    let status = match &error {
        KbError::DocumentNotFound(_) => StatusCode::NOT_FOUND,
        KbError::VersionError(_) | KbError::DocumentExists(_) => StatusCode::CONFLICT,
        KbError::InvalidParam(_) => StatusCode::BAD_REQUEST,
        KbError::PermissionDenied(_) => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    tracing::warn!(error=%error, "knowledge request failed");
    let message = if status == StatusCode::INTERNAL_SERVER_ERROR {
        "知识库操作失败".to_owned()
    } else {
        error.to_string()
    };
    (status, Json(json!({ "success": false, "error": message }))).into_response()
}
fn query_from_params(
    params: &std::collections::HashMap<String, String>,
    keyword: bool,
) -> SearchQuery {
    SearchQuery {
        keyword: if keyword { params.get("q").cloned().unwrap_or_default() } else { String::new() },
        page: params.get("page").and_then(|v| v.parse().ok()).unwrap_or(1).max(1),
        page_size: params.get("page_size").and_then(|v| v.parse().ok()).unwrap_or(20).clamp(1, 100),
        doc_type: params.get("doc_type").cloned(),
        tags: params
            .get("tags")
            .map(|tags| tags.split(',').filter(|tag| !tag.is_empty()).map(str::to_owned).collect())
            .unwrap_or_default(),
    }
}
async fn list_documents_handler(
    Extension(manager): Extension<Arc<KbManager>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    match manager.search(&query_from_params(&params,false)).await {
        Ok(result) => Json(json!({ "success":true, "documents":result.items, "total":result.total, "page":result.page, "page_size":result.page_size })).into_response(),
        Err(error) => failed(error),
    }
}
#[derive(serde::Deserialize)]
struct CreateDocument {
    title: String,
    content: String,
    author: String,
}
#[derive(serde::Deserialize)]
struct UpdateDocument {
    title: String,
    content: String,
    expected_version: u32,
}
async fn create_document_handler(
    Extension(manager): Extension<Arc<KbManager>>,
    Json(req): Json<CreateDocument>,
) -> Response {
    if req.title.trim().is_empty() || req.author.trim().is_empty() {
        return failed(KbError::InvalidParam("标题与作者不能为空".into()));
    }
    match manager.create_document(Document::new(req.title, req.content, req.author)).await {
        Ok(doc) => {
            (StatusCode::CREATED, Json(json!({ "success":true,"document":doc }))).into_response()
        },
        Err(error) => failed(error),
    }
}
async fn get_document_handler(
    Extension(manager): Extension<Arc<KbManager>>,
    Path(id): Path<String>,
) -> Response {
    match manager.get_document(&id).await {
        Ok(Some(doc)) => Json(json!({ "success":true,"document":doc })).into_response(),
        Ok(None) => failed(KbError::DocumentNotFound(id)),
        Err(error) => failed(error),
    }
}
async fn update_document_handler(
    Extension(manager): Extension<Arc<KbManager>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateDocument>,
) -> Response {
    if req.title.trim().is_empty() {
        return failed(KbError::InvalidParam("标题不能为空".into()));
    }
    let mut doc = match manager.get_document(&id).await {
        Ok(Some(doc)) => doc,
        Ok(None) => return failed(KbError::DocumentNotFound(id)),
        Err(error) => return failed(error),
    };
    if doc.version != req.expected_version {
        return failed(KbError::VersionError("文档版本已改变，请重新读取".into()));
    }
    let Some(version) = doc.version.checked_add(1) else {
        return failed(KbError::VersionError("版本号已达上限".into()));
    };
    doc.version = version;
    doc.title = req.title;
    doc.content = req.content;
    doc.updated_at = chrono::Utc::now().to_rfc3339();
    match manager.create_document(doc).await {
        Ok(doc) => Json(json!({ "success":true,"document":doc })).into_response(),
        Err(error) => failed(error),
    }
}
async fn delete_document_handler(
    Extension(manager): Extension<Arc<KbManager>>,
    Path(id): Path<String>,
) -> Response {
    match manager.delete_document(&id).await {
        Ok(()) => Json(json!({"success":true})).into_response(),
        Err(error) => failed(error),
    }
}
async fn list_versions_handler(
    Extension(manager): Extension<Arc<KbManager>>,
    Path(id): Path<String>,
) -> Response {
    match manager.list_versions(&id).await {
        Ok(versions) => Json(json!({ "success":true,"versions":versions })).into_response(),
        Err(error) => failed(error),
    }
}
async fn search_handler(
    Extension(manager): Extension<Arc<KbManager>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    match manager.search(&query_from_params(&params, true)).await {
        Ok(result) => Json(json!({ "success":true,"result":result })).into_response(),
        Err(error) => failed(error),
    }
}

// ── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "mox-kb-server", about = "MOX 知识库独立微服务", version)]
struct Cli {
    #[arg(short, long, default_value = "config/kb-server.toml")]
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
        config.server.port = 3414;
        config
    };
    config.apply_env_overrides();
    if let Some(port) = cli.port {
        config.server.port = port;
    }
    let module = KbModule::new();
    Server::new(Box::new(module), config).run().await?;
    Ok(())
}

#[cfg(test)]
mod tests;
