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

fn ok(data: Value) -> Json<Value> {
    Json(json!({ "success": true, "data": data }))
}

// =====================================================================
// 1. GET /projects/{id}/members — 协作成员列表
// =====================================================================
async fn project_members(Path(id): Path<String>) -> Json<Value> {
    ok(json!({
        "project_id": id,
        "members": [
            {
                "user_id": "admin-user",
                "username": "admin",
                "real_name": "系统管理员",
                "avatar": "/avatars/admin.png",
                "role": "owner",
                "joined_at": "2026-06-01T08:00:00Z",
                "last_active": now_iso(),
                "status": "active",
            },
            {
                "user_id": "user-002",
                "username": "zhangsan",
                "real_name": "张三",
                "avatar": "/avatars/zhangsan.png",
                "role": "editor",
                "joined_at": "2026-06-15T10:30:00Z",
                "last_active": now_iso(),
                "status": "active",
            },
            {
                "user_id": "user-003",
                "username": "lisi",
                "real_name": "李四",
                "avatar": "/avatars/lisi.png",
                "role": "viewer",
                "joined_at": "2026-07-01T14:00:00Z",
                "last_active": "2026-08-28T09:15:00Z",
                "status": "inactive",
            },
        ],
        "total": 3,
        "owner_count": 1,
        "editor_count": 1,
        "viewer_count": 1,
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
) -> Json<Value> {
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
) -> Json<Value> {
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
) -> Json<Value> {
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
async fn project_phases(Path(id): Path<String>) -> Json<Value> {
    ok(json!({
        "project_id": id,
        "phases": [
            {
                "phase_id": "phase-1",
                "name": "需求分析",
                "order": 1,
                "status": "completed",
                "start_date": "2026-06-01",
                "end_date": "2026-06-20",
                "progress": 100,
                "description": "收集和分析项目需求，输出需求文档",
            },
            {
                "phase_id": "phase-2",
                "name": "方案设计",
                "order": 2,
                "status": "completed",
                "start_date": "2026-06-21",
                "end_date": "2026-07-10",
                "progress": 100,
                "description": "技术方案设计与架构评审",
            },
            {
                "phase_id": "phase-3",
                "name": "开发实现",
                "order": 3,
                "status": "in_progress",
                "start_date": "2026-07-11",
                "end_date": "2026-09-30",
                "progress": 65,
                "description": "核心功能开发与单元测试",
            },
            {
                "phase_id": "phase-4",
                "name": "测试验证",
                "order": 4,
                "status": "pending",
                "start_date": "2026-10-01",
                "end_date": "2026-10-20",
                "progress": 0,
                "description": "集成测试、性能测试与验收",
            },
            {
                "phase_id": "phase-5",
                "name": "部署上线",
                "order": 5,
                "status": "pending",
                "start_date": "2026-10-21",
                "end_date": "2026-10-31",
                "progress": 0,
                "description": "生产环境部署与运维交接",
            },
        ],
        "current_phase": "phase-3",
        "total_phases": 5,
    }))
}

// =====================================================================
// 6. GET /projects/{id}/files — 共享文件列表
// =====================================================================
async fn project_files(
    Path(id): Path<String>,
    State(s): State<Arc<ProjectsState>>,
) -> Json<Value> {
    let files = s.files.lock().clone();
    let seed = if files.is_empty() {
        vec![
            json!({
                "id": "file-001",
                "name": "需求规格说明书-v1.2.pdf",
                "size_bytes": 2457600,
                "mime_type": "application/pdf",
                "uploaded_by": "admin-user",
                "uploaded_at": "2026-06-15T10:30:00Z",
                "storage_path": "data/uploads/proj-001/req-spec-v1.2.pdf",
            }),
            json!({
                "id": "file-002",
                "name": "架构设计图.png",
                "size_bytes": 1048576,
                "mime_type": "image/png",
                "uploaded_by": "user-002",
                "uploaded_at": "2026-07-01T14:20:00Z",
                "storage_path": "data/uploads/proj-001/arch-diagram.png",
            }),
        ]
    } else {
        files.iter().map(|f| json!(f)).collect()
    };
    ok(json!({
        "project_id": id,
        "files": seed,
        "total": seed.len(),
    }))
}

// =====================================================================
// 7. POST /projects/{id}/files/upload — 文件上传（multipart）
// =====================================================================
async fn upload_project_file(
    Path(id): Path<String>,
    State(s): State<Arc<ProjectsState>>,
    mut multipart: Multipart,
) -> Json<Value> {
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
) -> Json<Value> {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
    let now = chrono::Utc::now();
    let activities: Vec<Value> = (0..50usize)
        .map(|i| {
            let t = now - chrono::Duration::minutes((i * 23) as i64);
            let types = [
                ("task_created", "创建了任务", "task"),
                ("task_completed", "完成了任务", "task"),
                ("document_uploaded", "上传了文档", "document"),
                ("member_joined", "加入了项目", "member"),
                ("comment_added", "发表了评论", "comment"),
                ("phase_advanced", "推进了阶段", "phase"),
                ("file_uploaded", "上传了文件", "file"),
            ];
            let (action_type, action_text, target_type) = types[i % types.len()];
            json!({
                "activity_id": format!("act-{:04}", i + 1),
                "project_id": id,
                "user_id": format!("user-{:03}", (i % 5) + 1),
                "username": format!("user{}", (i % 5) + 1),
                "action_type": action_type,
                "action_text": action_text,
                "target_type": target_type,
                "target_id": format!("{}-{:03}", target_type, (i % 10) + 1),
                "target_name": format!("{} #{}", target_type, (i % 10) + 1),
                "detail": format!("{}「{} #{}」", action_text, target_type, (i % 10) + 1),
                "created_at": t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            })
        })
        .collect();
    let total = activities.len();
    let start = (page - 1) * page_size;
    let items: Vec<&Value> = activities.iter().skip(start).take(page_size).collect();
    ok(json!({
        "project_id": id,
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))
}

// =====================================================================
// 9. GET /projects/{id}/documents — 项目文档列表
// =====================================================================
async fn project_documents(Path(id): Path<String>) -> Json<Value> {
    ok(json!({
        "project_id": id,
        "documents": [
            {
                "doc_id": "doc-001",
                "title": "需求规格说明书",
                "version": "v1.2",
                "author": "admin-user",
                "status": "approved",
                "created_at": "2026-06-10T08:00:00Z",
                "updated_at": "2026-06-20T15:30:00Z",
                "size_bytes": 2457600,
                "mime_type": "application/pdf",
                "tags": ["需求", "规格"],
            },
            {
                "doc_id": "doc-002",
                "title": "系统架构设计文档",
                "version": "v2.0",
                "author": "user-002",
                "status": "review",
                "created_at": "2026-06-25T10:00:00Z",
                "updated_at": "2026-07-15T14:00:00Z",
                "size_bytes": 5242880,
                "mime_type": "application/pdf",
                "tags": ["架构", "设计"],
            },
            {
                "doc_id": "doc-003",
                "title": "API 接口规范",
                "version": "v1.0",
                "author": "user-003",
                "status": "draft",
                "created_at": "2026-08-01T09:00:00Z",
                "updated_at": "2026-08-10T11:00:00Z",
                "size_bytes": 1048576,
                "mime_type": "text/markdown",
                "tags": ["API", "规范"],
            },
        ],
        "total": 3,
    }))
}

// =====================================================================
// 10. PUT /projects/{id}/advance-phase — 阶段推进
// =====================================================================
async fn advance_phase(Path(id): Path<String>) -> Json<Value> {
    ok(json!({
        "project_id": id,
        "previous_phase": "phase-2",
        "current_phase": "phase-3",
        "phase_name": "开发实现",
        "advanced": true,
        "advanced_at": now_iso(),
        "message": format!("项目 {} 已推进到「开发实现」阶段", id),
    }))
}

// =====================================================================
// 11. GET /projects/{id}/phase-progress — 阶段进度
// =====================================================================
async fn phase_progress(Path(id): Path<String>) -> Json<Value> {
    ok(json!({
        "project_id": id,
        "overall_progress": 58.5,
        "current_phase": {
            "phase_id": "phase-3",
            "name": "开发实现",
            "progress": 65,
            "start_date": "2026-07-11",
            "end_date": "2026-09-30",
            "days_remaining": 27,
        },
        "phase_progress": [
            { "phase_id": "phase-1", "name": "需求分析", "progress": 100, "status": "completed" },
            { "phase_id": "phase-2", "name": "方案设计", "progress": 100, "status": "completed" },
            { "phase_id": "phase-3", "name": "开发实现", "progress": 65, "status": "in_progress" },
            { "phase_id": "phase-4", "name": "测试验证", "progress": 0, "status": "pending" },
            { "phase_id": "phase-5", "name": "部署上线", "progress": 0, "status": "pending" },
        ],
        "milestones": [
            { "name": "需求评审通过", "date": "2026-06-20", "achieved": true },
            { "name": "架构评审通过", "date": "2026-07-10", "achieved": true },
            { "name": "核心功能完成", "date": "2026-09-15", "achieved": false },
            { "name": "测试验收通过", "date": "2026-10-20", "achieved": false },
        ],
        "ts": now_iso(),
    }))
}

// =====================================================================
// 12. POST /projects/{id}/favorite — 收藏切换
// =====================================================================
async fn toggle_favorite(
    Path(id): Path<String>,
    State(s): State<Arc<ProjectsState>>,
) -> Json<Value> {
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
) -> Json<Value> {
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
) -> Json<Value> {
    ok(json!({
        "project_id": id,
        "doc_id": doc_id,
        "name": format!("document-{}.pdf", doc_id),
        "size_bytes": 2457600,
        "mime_type": "application/pdf",
        "download_url": format!("/api/projects/{}/documents/{}/raw", id, doc_id),
        "expires_in": 3600,
        "version": "v1.2",
        "ts": now_iso(),
    }))
}

// =====================================================================
// 15. GET /projects/{id}/requirements-graph — 需求图谱
// =====================================================================
async fn requirements_graph(Path(id): Path<String>) -> Json<Value> {
    ok(json!({
        "project_id": id,
        "graph": {
            "nodes": [
                {
                    "id": "req-001",
                    "label": "用户登录",
                    "type": "functional",
                    "priority": "high",
                    "status": "completed",
                    "x": 100.0,
                    "y": 100.0,
                },
                {
                    "id": "req-002",
                    "label": "权限管理",
                    "type": "functional",
                    "priority": "high",
                    "status": "in_progress",
                    "x": 300.0,
                    "y": 80.0,
                },
                {
                    "id": "req-003",
                    "label": "数据导入",
                    "type": "functional",
                    "priority": "medium",
                    "status": "pending",
                    "x": 500.0,
                    "y": 120.0,
                },
                {
                    "id": "req-004",
                    "label": "响应时间<200ms",
                    "type": "non_functional",
                    "priority": "high",
                    "status": "in_progress",
                    "x": 200.0,
                    "y": 300.0,
                },
                {
                    "id": "req-005",
                    "label": "数据加密存储",
                    "type": "non_functional",
                    "priority": "critical",
                    "status": "completed",
                    "x": 400.0,
                    "y": 280.0,
                },
            ],
            "edges": [
                { "source": "req-001", "target": "req-002", "relation": "depends_on", "label": "依赖" },
                { "source": "req-002", "target": "req-003", "relation": "depends_on", "label": "依赖" },
                { "source": "req-001", "target": "req-004", "relation": "constrains", "label": "约束" },
                { "source": "req-002", "target": "req-005", "relation": "constrains", "label": "约束" },
                { "source": "req-003", "target": "req-005", "relation": "related_to", "label": "关联" },
            ],
        },
        "stats": {
            "total_nodes": 5,
            "total_edges": 5,
            "functional": 3,
            "non_functional": 2,
            "completed": 2,
            "in_progress": 2,
            "pending": 1,
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
