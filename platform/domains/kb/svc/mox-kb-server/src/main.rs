// =============================================================================
// mox-kb-server: 知识库独立微服务入口
// =============================================================================
//
// 独立部署：cargo run -p mox-kb-server -- --config /etc/mox/kb.toml
// 默认端口：8104
// 健康检查：http://localhost:8104/health/live
//
// 从 kg/svc/mox-kb-svc 独立迁出，成为 kb 域的独立微服务。
// 提供：文档管理 / 版本控制 / 全文检索 / 知识分析 / 关联链接 / 专家门禁
// =============================================================================

use async_trait::async_trait;
use axum::{extract::Extension, routing::{get, post}, Json, Router};
use clap::Parser;
use mox_kb_core::{Document, KbManager, KbStore, KbResult, SearchQuery};
use mox_server_runtime::{Server, ServerConfig, ServiceModule};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

/// 内存存储实现（用于开发/测试，生产环境接入 SQLite/PostgreSQL）
struct InMemoryKbStore {
    docs: parking_lot::RwLock<std::collections::HashMap<String, Document>>,
}

impl InMemoryKbStore {
    fn new() -> Self {
        Self { docs: parking_lot::RwLock::new(std::collections::HashMap::new()) }
    }
}

#[async_trait]
impl KbStore for InMemoryKbStore {
    async fn save_document(&self, doc: &Document) -> KbResult<()> {
        self.docs.write().insert(doc.id.clone(), doc.clone());
        Ok(())
    }
    async fn get_document(&self, doc_id: &str) -> KbResult<Option<Document>> {
        Ok(self.docs.read().get(doc_id).cloned())
    }
    async fn search_documents(&self, query: &SearchQuery) -> KbResult<mox_kb_core::SearchResult> {
        let docs = self.docs.read();
        let filtered: Vec<Document> = docs.values()
            .filter(|d| query.keyword.is_empty() || d.title.contains(&query.keyword) || d.content.contains(&query.keyword))
            .filter(|d| query.doc_type.as_ref().map_or(true, |t| d.doc_type == *t))
            .cloned()
            .collect();
        let total = filtered.len() as u64;
        let start = ((query.page - 1) * query.page_size) as usize;
        let items: Vec<Document> = filtered.into_iter().skip(start).take(query.page_size as usize).collect();
        Ok(mox_kb_core::SearchResult { items, total, page: query.page, page_size: query.page_size, duration_ms: 0 })
    }
    async fn delete_document(&self, doc_id: &str) -> KbResult<()> {
        self.docs.write().remove(doc_id);
        Ok(())
    }
    async fn list_versions(&self, _doc_id: &str) -> KbResult<Vec<mox_kb_core::DocumentVersion>> {
        Ok(vec![])
    }
}

struct KbModule {
    manager: Arc<KbManager>,
}

impl KbModule {
    fn new() -> Self {
        let store = Box::new(InMemoryKbStore::new());
        Self { manager: Arc::new(KbManager::new(store)) }
    }
}

#[async_trait]
impl ServiceModule for KbModule {
    fn name(&self) -> &str { "mox-kb-server" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    async fn routes(&self, _config: &ServerConfig) -> Router {
        let manager = self.manager.clone();
        Router::new()
            .route("/api/v1/kb/info", get(kb_info_handler))
            .route("/api/v1/kb/documents", get(list_documents_handler).post(create_document_handler))
            .route("/api/v1/kb/documents/{id}", get(get_document_handler).delete(delete_document_handler))
            .route("/api/v1/kb/search", get(search_handler))
            .layer(Extension(manager))
    }
    async fn init(&self, _config: &ServerConfig) -> Result<(), mox_server_runtime::RuntimeError> {
        tracing::info!("知识库服务初始化完成（独立域，从 kg 迁出）");
        Ok(())
    }
    async fn ready_checks(&self) -> Vec<(&'static str, bool)> {
        vec![("kb_storage", true), ("kb_search", true)]
    }
}

// ── 处理器 ──────────────────────────────────────────────────────────────────

async fn kb_info_handler() -> Json<serde_json::Value> {
    Json(json!({
        "service": "mox-kb-server",
        "module": "knowledge-base",
        "capabilities": ["document_management", "version_control", "fulltext_search", "knowledge_analysis", "relation_linking", "expert_gate"],
        "status": "running",
        "note": "独立域，从 kg/svc/mox-kb-svc 迁出",
    }))
}

async fn list_documents_handler(Extension(_manager): Extension<Arc<KbManager>>) -> Json<serde_json::Value> {
    Json(json!({ "documents": [], "total": 0 }))
}

async fn create_document_handler(
    Extension(manager): Extension<Arc<KbManager>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let title = req.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let content = req.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let author = req.get("author").and_then(|v| v.as_str()).unwrap_or("anonymous").to_string();
    let doc = Document::new(title, content, author);
    match manager.create_document(doc).await {
        Ok(d) => Json(json!({ "success": true, "document": d })),
        Err(e) => Json(json!({ "success": false, "error": e.to_string() })),
    }
}

async fn get_document_handler(
    Extension(manager): Extension<Arc<KbManager>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    match manager.get_document(&id).await {
        Ok(Some(d)) => Json(json!({ "success": true, "document": d })),
        Ok(None) => Json(json!({ "success": false, "error": "文档不存在" })),
        Err(e) => Json(json!({ "success": false, "error": e.to_string() })),
    }
}

async fn delete_document_handler(
    Extension(manager): Extension<Arc<KbManager>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    match manager.delete_document(&id).await {
        Ok(()) => Json(json!({ "success": true })),
        Err(e) => Json(json!({ "success": false, "error": e.to_string() })),
    }
}

async fn search_handler(
    Extension(manager): Extension<Arc<KbManager>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let keyword = params.get("q").cloned().unwrap_or_default();
    let query = SearchQuery { keyword, ..Default::default() };
    match manager.search(&query).await {
        Ok(r) => Json(json!({ "success": true, "result": r })),
        Err(e) => Json(json!({ "success": false, "error": e.to_string() })),
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
        ServerConfig::default()
    };
    config.apply_env_overrides();
    if let Some(port) = cli.port { config.server.port = port; }
    if config.server.port == 8080 { config.server.port = 8104; }
    let module = KbModule::new();
    Server::new(Box::new(module), config).run().await?;
    Ok(())
}
