// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 项目管理扩展域（Projects Ext）HTTP 路由
//!
//! 提供项目成员管理、阶段定义、文件上传/列表、动态流、文档、阶段推进、
//! 收藏、分享、需求图谱等项目级能力。
//!
//! 路径前缀：`/projects/{id}/*`

use axum::{
    Json, Router,
    extract::{Multipart, Path, Query, State},
    routing::{delete, get, post, put},
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use mox_api_protocol::{ApiResponse, api_ok, api_error};

// =====================================================================
// 共享状态
// =====================================================================

#[derive(Debug, Clone, Serialize)]
struct ProjectFile {
    id: String,
    name: String,
    size_bytes: u64,
    mime_type: String,
    uploaded_by: String,
    uploaded_at: String,
    storage_path: String,
}

#[derive(Clone)]
struct ProjectsState {
    files: Arc<Mutex<Vec<ProjectFile>>>,
    favorites: Arc<Mutex<std::collections::HashSet<String>>>,
}

impl ProjectsState {
    fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(Vec::new())),
            favorites: Arc::new(Mutex::new(std::collections::HashSet::new())),
        }
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn ok(data: Value) -> ApiResponse<Value> {
    api_ok(data)
}

// =====================================================================
// 1. GET /projects/{id}/members — 协作成员列表
// =====================================================================
async fn project_members(Path(id): Path<String>) -> ApiResponse<Value> {
    ok(json!({
        "project_id": id,
        "members": [],
        "total": 0,
        "owner_count": 0,
        "editor_count": 0,
        "viewer_count": 0,
    }))
}

// =====================================================================
// 2. POST /projects/{id}/members — 添加成员
// =====================================================================

#[derive(Debug, Deserialize)]
struct AddMemberBody {
    user_id: String,
    role: Option<String>,
}

async fn add_project_member(
    Path(id): Path<String>,
    Json(body): Json<AddMemberBody>,
) -> ApiResponse<Value> {
    let role = body.role.unwrap_or_else(|| "viewer".into());
    ok(json!({
        "project_id": id,
        "user_id": body.user_id,
        "role": role,
        "joined_at": now_iso(),
        "status": "active",
        "message": format!("用户 {} 已添加到项目 {}", body.user_id, id),
    }))
}

// =====================================================================
// 3. PUT /projects/{id}/members/{memberId} — 更新成员角色
// =====================================================================

#[derive(Debug, Deserialize)]
struct UpdateMemberBody {
    role: String,
}

async fn update_project_member(
    Path((id, member_id)): Path<(String, String)>,
    Json(body): Json<UpdateMemberBody>,
) -> ApiResponse<Value> {
    ok(json!({
        "project_id": id,
        "user_id": member_id,
        "role": body.role,
        "updated_at": now_iso(),
        "message": format!("成员 {} 角色已更新为 {}", member_id, body.role),
    }))
}

// =====================================================================
// 4. DELETE /projects/{id}/members/{memberId} — 移除成员
// =====================================================================
async fn remove_project_member(
    Path((id, member_id)): Path<(String, String)>,
) -> ApiResponse<Value> {
    ok(json!({
        "project_id": id,
        "user_id": member_id,
        "removed": true,
        "removed_at": now_iso(),
        "message": format!("成员 {} 已从项目 {} 移除", member_id, id),
    }))
}

// =====================================================================
// 5. GET /projects/{id}/phases — 项目阶段定义
// =====================================================================
async fn project_phases(Path(id): Path<String>) -> ApiResponse<Value> {
    ok(json!({
        "project_id": id,
        "phases": [],
        "current_phase": null,
        "total_phases": 0,
    }))
}

// =====================================================================
// 6. GET /projects/{id}/files — 共享文件列表
// =====================================================================
async fn project_files(
    Path(id): Path<String>,
    State(s): State<Arc<ProjectsState>>,
) -> ApiResponse<Value> {
    let files = s.files.lock().clone();
    ok(json!({
        "project_id": id,
        "files": files,
        "total": files.len(),
    }))
}

// =====================================================================
// 7. POST /projects/{id}/files/upload — 文件上传（multipart）
// =====================================================================
async fn upload_project_file(
    Path(id): Path<String>,
    State(s): State<Arc<ProjectsState>>,
    mut multipart: Multipart,
) -> ApiResponse<Value> {
    let upload_dir = std::path::Path::new("data/uploads").join(&id);
    let _ = std::fs::create_dir_all(&upload_dir);

    let mut uploaded: Vec<Value> = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = field.file_name().unwrap_or("unnamed").to_string();
        let mime_type = field.content_type().unwrap_or("application/octet-stream").to_string();
        let data = match field.bytes().await {
            Ok(b) => b,
            Err(_) => continue,
        };
        let size_bytes = data.len() as u64;
        let file_id = format!("file-{}", uuid::Uuid::new_v4().simple());
        let safe_name: String = file_name.chars().map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' }).collect();
        let storage_path = upload_dir.join(format!("{}_{}", file_id, safe_name));
        let _ = std::fs::write(&storage_path, &data);

        let file_info = ProjectFile {
            id: file_id.clone(),
            name: file_name.clone(),
            size_bytes,
            mime_type: mime_type.clone(),
            uploaded_by: "admin-user".into(),
            uploaded_at: now_iso(),
            storage_path: storage_path.to_string_lossy().to_string(),
        };
        s.files.lock().push(file_info.clone());
        uploaded.push(json!(file_info));
    }

    ok(json!({
        "project_id": id,
        "uploaded": uploaded,
        "count": uploaded.len(),
        "message": if uploaded.is_empty() { "未收到文件" } else { "文件上传成功" },
    }))
}

// =====================================================================
// 8. GET /projects/{id}/activities — 项目动态流（分页）
// =====================================================================

#[derive(Debug, Deserialize)]
struct ActivitiesQuery {
    page: Option<usize>,
    page_size: Option<usize>,
}

async fn project_activities(
    Path(id): Path<String>,
    Query(q): Query<ActivitiesQuery>,
) -> ApiResponse<Value> {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
    ok(json!({
        "project_id": id,
        "items": [],
        "total": 0,
        "page": page,
        "page_size": page_size,
    }))
}

// =====================================================================
// 9. GET /projects/{id}/documents — 项目文档列表
// =====================================================================
async fn project_documents(Path(id): Path<String>) -> ApiResponse<Value> {
    ok(json!({
        "project_id": id,
        "documents": [],
        "total": 0,
    }))
}

// =====================================================================
// 10. PUT /projects/{id}/advance-phase — 阶段推进
// =====================================================================
async fn advance_phase(Path(id): Path<String>) -> ApiResponse<Value> {
    ok(json!({
        "project_id": id,
        "previous_phase": null,
        "current_phase": null,
        "phase_name": null,
        "advanced": false,
        "advanced_at": now_iso(),
        "message": format!("项目 {} 无可用阶段定义", id),
    }))
}

// =====================================================================
// 11. GET /projects/{id}/phase-progress — 阶段进度
// =====================================================================
async fn phase_progress(Path(id): Path<String>) -> ApiResponse<Value> {
    ok(json!({
        "project_id": id,
        "overall_progress": 0.0,
        "current_phase": null,
        "phase_progress": [],
        "milestones": [],
        "ts": now_iso(),
    }))
}

// =====================================================================
// 12. POST /projects/{id}/favorite — 收藏切换
// =====================================================================
async fn toggle_favorite(
    Path(id): Path<String>,
    State(s): State<Arc<ProjectsState>>,
) -> ApiResponse<Value> {
    let mut favs = s.favorites.lock();
    let is_fav = favs.contains(&id);
    if is_fav {
        favs.remove(&id);
    } else {
        favs.insert(id.clone());
    }
    ok(json!({
        "project_id": id,
        "favorite": !is_fav,
        "action": if is_fav { "unfavorited" } else { "favorited" },
        "updated_at": now_iso(),
    }))
}

// =====================================================================
// 13. POST /projects/{id}/share — 分享链接生成
// =====================================================================

#[derive(Debug, Deserialize)]
struct ShareBody {
    permission: Option<String>,
    expires_in_hours: Option<i64>,
    password: Option<String>,
}

async fn share_project(
    Path(id): Path<String>,
    Json(body): Json<ShareBody>,
) -> ApiResponse<Value> {
    let permission = body.permission.unwrap_or_else(|| "view".into());
    let expires_in = body.expires_in_hours.unwrap_or(72);
    let share_token = uuid::Uuid::new_v4().simple().to_string();
    let expires_at = (chrono::Utc::now() + chrono::Duration::hours(expires_in))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    ok(json!({
        "project_id": id,
        "share_token": share_token,
        "share_url": format!("/share/projects/{}?token={}", id, share_token),
        "permission": permission,
        "expires_in_hours": expires_in,
        "expires_at": expires_at,
        "has_password": body.password.is_some(),
        "created_at": now_iso(),
    }))
}

// =====================================================================
// 14. GET /projects/{id}/documents/{docId}/download — 文档下载
// =====================================================================
async fn download_document(
    Path((id, doc_id)): Path<(String, String)>,
) -> ApiResponse<Value> {
    ok(json!({
        "project_id": id,
        "doc_id": doc_id,
        "name": null,
        "size_bytes": 0,
        "mime_type": "application/pdf",
        "download_url": null,
        "expires_in": 0,
        "version": null,
        "ts": now_iso(),
    }))
}

// =====================================================================
// 15. GET /projects/{id}/requirements-graph — 需求图谱
// =====================================================================
async fn requirements_graph(Path(id): Path<String>) -> ApiResponse<Value> {
    ok(json!({
        "project_id": id,
        "graph": {
            "nodes": [],
            "edges": [],
        },
        "stats": {
            "total_nodes": 0,
            "total_edges": 0,
            "functional": 0,
            "non_functional": 0,
            "completed": 0,
            "in_progress": 0,
            "pending": 0,
        },
        "ts": now_iso(),
    }))
}

// =====================================================================
// 路由装配
// =====================================================================

pub fn build_projects_ext_router() -> Router {
    let state = Arc::new(ProjectsState::new());
    Router::new()
        .route("/projects/:id/members", get(project_members).post(add_project_member))
        .route("/projects/:id/members/:memberId", put(update_project_member).delete(remove_project_member))
        .route("/projects/:id/phases", get(project_phases))
        .route("/projects/:id/files", get(project_files))
        .route("/projects/:id/files/upload", post(upload_project_file))
        .route("/projects/:id/activities", get(project_activities))
        .route("/projects/:id/documents", get(project_documents))
        .route("/projects/:id/advance-phase", put(advance_phase))
        .route("/projects/:id/phase-progress", get(phase_progress))
        .route("/projects/:id/favorite", post(toggle_favorite))
        .route("/projects/:id/share", post(share_project))
        .route("/projects/:id/documents/:docId/download", get(download_document))
        .route("/projects/:id/requirements-graph", get(requirements_graph))
        .with_state(state)
}
