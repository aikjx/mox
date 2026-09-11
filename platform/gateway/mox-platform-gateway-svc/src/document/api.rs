//! 文档管理 + 电子签章 API 端点
//!
//! 支持文档上传 / 版本管理 / 权限控制 / 在线预览 / 电子签章 / 审批签署

use crate::document::*;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 文档管理状态
pub struct DocumentState {
    pub documents: Arc<RwLock<HashMap<String, Document>>>,
    pub versions: Arc<RwLock<HashMap<String, Vec<DocumentVersion>>>>,
    pub categories: Arc<RwLock<HashMap<String, DocumentCategory>>>,
    pub signatures: Arc<RwLock<HashMap<String, Vec<SignatureRecord>>>>,
    pub activities: Arc<RwLock<Vec<DocumentActivity>>>,
}

impl DocumentState {
    pub fn new() -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            versions: Arc::new(RwLock::new(HashMap::new())),
            categories: Arc::new(RwLock::new(HashMap::new())),
            signatures: Arc::new(RwLock::new(HashMap::new())),
            activities: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for DocumentState {
    fn default() -> Self {
        Self::new()
    }
}

/// GET /api/enterprise/document/types —— 获取支持的文档类型
pub async fn list_document_types_handler() -> Response {
    let types = supported_document_types();
    let result: Vec<serde_json::Value> = types.iter().map(|(code, name)| {
        json!({ "code": code, "name": name })
    }).collect();
    Json(json!({ "code": 0, "data": result, "total": result.len() })).into_response()
}

/// GET /api/enterprise/document/documents —— 获取文档列表
pub async fn list_documents_handler(
    State(state): State<Arc<DocumentState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let documents = state.documents.read().await;
    let mut list: Vec<&Document> = documents.values().collect();
    if let Some(dtype) = params.get("document_type") {
        list.retain(|d| format!("{:?}", d.document_type).to_lowercase() == *dtype);
    }
    if let Some(status) = params.get("status") {
        list.retain(|d| format!("{:?}", d.status).to_lowercase() == *status);
    }
    if let Some(owner) = params.get("owner_id") {
        list.retain(|d| d.owner_id == *owner);
    }
    list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Json(json!({ "code": 0, "data": list, "total": list.len() })).into_response()
}

/// GET /api/enterprise/document/documents/:id —— 获取文档详情
pub async fn get_document_handler(
    State(state): State<Arc<DocumentState>>,
    Path(id): Path<String>,
) -> Response {
    let documents = state.documents.read().await;
    match documents.get(&id) {
        Some(d) => Json(json!({ "code": 0, "data": d })).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "文档不存在" }))).into_response(),
    }
}

/// POST /api/enterprise/document/documents —— 创建文档
pub async fn create_document_handler(
    State(state): State<Arc<DocumentState>>,
    Json(req): Json<CreateDocumentRequest>,
) -> Response {
    let document_id = format!("doc_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();
    let document = Document {
        document_id: document_id.clone(),
        tenant_id: "default".to_string(),
        title: req.title,
        document_type: req.document_type,
        document_no: req.document_no,
        description: req.description,
        category_id: req.category_id,
        tags: req.tags.unwrap_or_default(),
        current_version: "1.0".to_string(),
        status: DocumentStatus::Draft,
        require_signature: req.require_signature.unwrap_or(false),
        signature_status: None,
        owner_id: "user_001".to_string(),
        department_id: None,
        file_path: req.file_path,
        file_size: req.file_size,
        mime_type: req.mime_type,
        page_count: None,
        view_count: 0,
        download_count: 0,
        permission_config: req.permission_config.unwrap_or_default(),
        metadata: req.metadata,
        created_by: "user_001".to_string(),
        created_at: now.clone(),
        updated_by: None,
        updated_at: now,
        published_at: None,
        archived_at: None,
    };
    state.documents.write().await.insert(document_id.clone(), document);

    // 记录活动
    let activity = DocumentActivity {
        activity_id: format!("act_{}", uuid::Uuid::new_v4().simple()),
        document_id: document_id.clone(),
        activity_type: "create".to_string(),
        actor_id: "user_001".to_string(),
        actor_name: "系统用户".to_string(),
        details: Some("创建文档".to_string()),
        ip_address: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    state.activities.write().await.push(activity);

    Json(json!({
        "code": 0,
        "message": "文档创建成功",
        "data": { "document_id": document_id }
    })).into_response()
}

/// PUT /api/enterprise/document/documents/:id —— 更新文档
pub async fn update_document_handler(
    State(state): State<Arc<DocumentState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateDocumentRequest>,
) -> Response {
    let mut documents = state.documents.write().await;
    match documents.get_mut(&id) {
        Some(d) => {
            if let Some(title) = req.title { d.title = title; }
            if let Some(dtype) = req.document_type { d.document_type = dtype; }
            if let Some(desc) = req.description { d.description = Some(desc); }
            if let Some(cid) = req.category_id { d.category_id = Some(cid); }
            if let Some(tags) = req.tags { d.tags = tags; }
            if let Some(status) = req.status { d.status = status; }
            if let Some(rs) = req.require_signature { d.require_signature = rs; }
            if let Some(pc) = req.permission_config { d.permission_config = pc; }
            if let Some(meta) = req.metadata { d.metadata = Some(meta); }
            d.updated_at = chrono::Utc::now().to_rfc3339();
            Json(json!({ "code": 0, "message": "文档更新成功" })).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "文档不存在" }))).into_response(),
    }
}

/// DELETE /api/enterprise/document/documents/:id —— 删除文档
pub async fn delete_document_handler(
    State(state): State<Arc<DocumentState>>,
    Path(id): Path<String>,
) -> Response {
    let mut documents = state.documents.write().await;
    if documents.remove(&id).is_some() {
        Json(json!({ "code": 0, "message": "文档删除成功" })).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "文档不存在" }))).into_response()
    }
}

/// GET /api/enterprise/document/documents/:id/versions —— 获取文档版本列表
pub async fn list_versions_handler(
    State(state): State<Arc<DocumentState>>,
    Path(id): Path<String>,
) -> Response {
    let versions = state.versions.read().await;
    match versions.get(&id) {
        Some(v) => Json(json!({ "code": 0, "data": v, "total": v.len() })).into_response(),
        None => Json(json!({ "code": 0, "data": [], "total": 0 })).into_response(),
    }
}

/// POST /api/enterprise/document/documents/:id/sign —— 发起电子签章
pub async fn sign_document_handler(
    State(state): State<Arc<DocumentState>>,
    Path(id): Path<String>,
    Json(req): Json<SignDocumentRequest>,
) -> Response {
    let documents = state.documents.read().await;
    match documents.get(&id) {
        Some(d) => {
            // 创建签章记录
            let mut signatures = state.signatures.write().await;
            let mut doc_signatures = signatures.remove(&id).unwrap_or_default();
            for signer_id in &req.signer_ids {
                let record = SignatureRecord {
                    signature_id: format!("sig_{}", uuid::Uuid::new_v4().simple()),
                    document_id: id.clone(),
                    document_version: d.current_version.clone(),
                    signer_id: signer_id.clone(),
                    signer_name: "签署人".to_string(),
                    signature_type: "personal".to_string(),
                    status: "pending".to_string(),
                    position: req.position.clone(),
                    signature_image_url: None,
                    certificate_serial: None,
                    certificate_issuer: None,
                    signature_value: None,
                    signature_algorithm: None,
                    signed_at: None,
                    ip_address: None,
                    reject_reason: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                };
                doc_signatures.push(record);
            }
            signatures.insert(id.clone(), doc_signatures);

            Json(json!({
                "code": 0,
                "message": "签章流程已发起",
                "data": { "document_id": id, "signer_count": req.signer_ids.len() }
            })).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "文档不存在" }))).into_response(),
    }
}

/// GET /api/enterprise/document/documents/:id/signatures —— 获取签章记录
pub async fn list_signatures_handler(
    State(state): State<Arc<DocumentState>>,
    Path(id): Path<String>,
) -> Response {
    let signatures = state.signatures.read().await;
    match signatures.get(&id) {
        Some(s) => Json(json!({ "code": 0, "data": s, "total": s.len() })).into_response(),
        None => Json(json!({ "code": 0, "data": [], "total": 0 })).into_response(),
    }
}

/// GET /api/enterprise/document/documents/:id/activities —— 获取文档活动记录
pub async fn list_activities_handler(
    State(state): State<Arc<DocumentState>>,
    Path(id): Path<String>,
) -> Response {
    let activities = state.activities.read().await;
    let doc_activities: Vec<&DocumentActivity> = activities.iter().filter(|a| a.document_id == id).collect();
    Json(json!({ "code": 0, "data": doc_activities, "total": doc_activities.len() })).into_response()
}

/// GET /api/enterprise/document/stats —— 获取文档统计
pub async fn document_stats_handler(
    State(state): State<Arc<DocumentState>>,
) -> Response {
    let documents = state.documents.read().await;
    let mut stats = DocumentStats::default();
    stats.total = documents.len() as i64;
    for doc in documents.values() {
        match doc.status {
            DocumentStatus::Draft => stats.draft += 1,
            DocumentStatus::Reviewing => stats.reviewing += 1,
            DocumentStatus::Published => stats.published += 1,
            DocumentStatus::Archived => stats.archived += 1,
            _ => {}
        }
        if doc.require_signature && doc.signature_status.as_deref() == Some("pending") {
            stats.pending_signature += 1;
        }
        if doc.signature_status.as_deref() == Some("signed") {
            stats.signed += 1;
        }
    }
    Json(json!({ "code": 0, "data": stats })).into_response()
}

/// 构建文档管理路由（泛型版本）
pub fn build_document_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<DocumentState>: axum::extract::FromRef<S>,
{
    use axum::routing::{get, post, put, delete};

    axum::Router::new()
        .route("/types", get(list_document_types_handler))
        .route("/documents", get(list_documents_handler).post(create_document_handler))
        .route("/documents/:id", get(get_document_handler).put(update_document_handler).delete(delete_document_handler))
        .route("/documents/:id/versions", get(list_versions_handler))
        .route("/documents/:id/sign", post(sign_document_handler))
        .route("/documents/:id/signatures", get(list_signatures_handler))
        .route("/documents/:id/activities", get(list_activities_handler))
        .route("/stats", get(document_stats_handler))
}
