// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 杂项域（Misc）HTTP 路由
//!
//! 提供用户头像上传、市场模板审核、工作流更新，以及任务/项目分页列表等通用能力。
//!
//! 路径：`/users/*` · `/market/*` · `/ai/*` · `/tasks` · `/projects`

use axum::{
    Json, Router,
    extract::{Multipart, Path, Query, State},
    routing::{get, post, put},
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use mox_api_protocol::{ApiResponse, api_ok, api_error};

// =====================================================================
// 共享状态
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskItem {
    id: String,
    title: String,
    description: String,
    status: String,
    priority: String,
    project_id: String,
    assignee: String,
    due_date: String,
    created_at: String,
    updated_at: String,
    progress: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectItem {
    id: String,
    name: String,
    description: String,
    status: String,
    owner: String,
    progress: i64,
    member_count: i64,
    task_count: i64,
    created_at: String,
    updated_at: String,
    start_date: String,
    end_date: String,
}

// =====================================================================
// JSON 持久化（data/misc_data.json）
// =====================================================================

const MISC_DATA_PATH: &str = "data/misc_data.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct MiscPersistent {
    tasks: Vec<TaskItem>,
    projects: Vec<ProjectItem>,
}

fn load_misc_data() -> MiscPersistent {
    match std::fs::read_to_string(MISC_DATA_PATH) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => MiscPersistent::default(),
    }
}

fn save_misc_data(tasks: &[TaskItem], projects: &[ProjectItem]) {
    if let Some(parent) = std::path::Path::new(MISC_DATA_PATH).parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("[misc] 创建目录失败 {}: {}", parent.display(), e);
        }
    }
    let data = MiscPersistent {
        tasks: tasks.to_vec(),
        projects: projects.to_vec(),
    };
    if let Ok(json_str) = serde_json::to_string_pretty(&data) {
        if let Err(e) = std::fs::write(MISC_DATA_PATH, json_str) {
            eprintln!("[misc] 数据持久化失败 {}: {}", MISC_DATA_PATH, e);
        }
    }
}

#[derive(Clone)]
pub struct MiscState {
    tasks: Arc<Mutex<Vec<TaskItem>>>,
    projects: Arc<Mutex<Vec<ProjectItem>>>,
}

impl MiscState {
    pub fn new() -> Self {
        let persisted = load_misc_data();
        Self {
            tasks: Arc::new(Mutex::new(persisted.tasks)),
            projects: Arc::new(Mutex::new(persisted.projects)),
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
// 1. POST /users/{id}/avatar — 用户头像上传（multipart）
// =====================================================================
async fn upload_avatar(Path(id): Path<String>, mut multipart: Multipart) -> ApiResponse<Value> {
    let upload_dir = std::path::Path::new("data/uploads/avatars");
    if let Err(e) = std::fs::create_dir_all(&upload_dir) {
        eprintln!("[misc] 创建头像目录失败: {}", e);
    }

    let mut avatar_url: Option<String> = None;
    let mut file_size: u64 = 0;
    let mut mime_type = String::from("image/png");

    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = field.file_name().unwrap_or("avatar.png").to_string();
        mime_type = field.content_type().unwrap_or("image/png").to_string();
        let data = match field.bytes().await {
            Ok(b) => b,
            Err(_) => continue,
        };
        file_size = data.len() as u64;
        let ext = std::path::Path::new(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");
        let stored_name = format!("{}_{}.{}", id, uuid::Uuid::new_v4().simple(), ext);
        let storage_path = upload_dir.join(&stored_name);
        if let Err(e) = std::fs::write(&storage_path, &data) {
            eprintln!("[misc] 头像写入失败 {}: {}", storage_path.display(), e);
        }
        avatar_url = Some(format!("/avatars/{}", stored_name));
        break; // 只处理第一个文件字段
    }

    let has_avatar = avatar_url.is_some();
    let final_avatar_url = avatar_url.unwrap_or_else(|| format!("/avatars/{}.png", id));

    ok(json!({
        "user_id": id,
        "avatar_url": final_avatar_url,
        "file_size": file_size,
        "mime_type": mime_type,
        "uploaded_at": now_iso(),
        "message": if has_avatar { "头像上传成功" } else { "未收到头像文件" },
    }))
}

// =====================================================================
// 2. POST /market/{id}/review — 市场模板审核
// =====================================================================

/// 市场模板审核请求体（字段归一化 RC-4）。
///
/// 历史上存在两套前端契约：
/// - 标准契约：`{action: "approve"|"reject", reason?, reviewer?}`（market.api.js）
/// - 变体契约：`{review_status: "approved"|"rejected", reject_reason?}`（operators.api.js）
///
/// 此前仅支持标准契约，变体调用因缺 `action` 导致 serde 反序列化失败 → 400。
/// 归一化策略：**后端同时接受两套字段并归一到同一语义**，前端零改动。
#[derive(Debug, Deserialize)]
struct MarketReviewBody {
    /// 标准动作字段
    #[serde(default)]
    action: Option<String>,
    /// 标准原因字段
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    reviewer: Option<String>,
    /// 变体动作字段（operators.api.js 的 marketApprove/marketReject）
    #[serde(default)]
    review_status: Option<String>,
    /// 变体原因字段
    #[serde(default)]
    reject_reason: Option<String>,
}

impl MarketReviewBody {
    /// 归一化动作：优先标准 `action`，回退 `review_status`，并兼容 approve(d)/reject(ed) 词形。
    fn normalized_action(&self) -> String {
        let raw = self
            .action
            .clone()
            .or_else(|| self.review_status.clone())
            .unwrap_or_default()
            .to_lowercase();
        match raw.as_str() {
            "approve" | "approved" | "pass" | "passed" => "approve".to_string(),
            _ => "reject".to_string(),
        }
    }

    /// 归一化原因：优先标准 `reason`，回退 `reject_reason`。
    fn normalized_reason(&self) -> Option<String> {
        self.reason.clone().or_else(|| self.reject_reason.clone())
    }
}

async fn market_review(
    Path(id): Path<String>,
    Json(body): Json<MarketReviewBody>,
) -> ApiResponse<Value> {
    let action = body.normalized_action();
    let approved = action == "approve";
    ok(json!({
        "template_id": id,
        "action": action,
        "status": if approved { "approved" } else { "rejected" },
        "reason": body.normalized_reason(),
        "reviewer": body.reviewer.clone().unwrap_or_else(|| "admin".into()),
        "reviewed_at": now_iso(),
        "message": if approved {
            format!("模板 {} 已审核通过", id)
        } else {
            format!("模板 {} 已被驳回", id)
        },
    }))
}

// =====================================================================
// 3. PUT /ai/flows/{id} — 工作流更新
// =====================================================================

#[derive(Debug, Deserialize)]
struct FlowUpdateBody {
    name: Option<String>,
    description: Option<String>,
    nodes: Option<Vec<Value>>,
    edges: Option<Vec<Value>>,
    variables: Option<Value>,
    trigger: Option<Value>,
    status: Option<String>,
    version: Option<i64>,
}

async fn update_flow(
    Path(id): Path<String>,
    Json(body): Json<FlowUpdateBody>,
) -> ApiResponse<Value> {
    let node_count = body.nodes.as_ref().map(|n| n.len()).unwrap_or(0);
    let edge_count = body.edges.as_ref().map(|e| e.len()).unwrap_or(0);
    ok(json!({
        "flow_id": id,
        "name": body.name.unwrap_or_else(|| format!("工作流-{}", id)),
        "description": body.description,
        "nodes": body.nodes.unwrap_or_default(),
        "edges": body.edges.unwrap_or_default(),
        "variables": body.variables.unwrap_or(json!({})),
        "trigger": body.trigger,
        "status": body.status.unwrap_or_else(|| "draft".into()),
        "version": body.version.unwrap_or(1) + 1,
        "node_count": node_count,
        "edge_count": edge_count,
        "updated_at": now_iso(),
        "message": format!("工作流 {} 已更新", id),
    }))
}

// =====================================================================
// 4. GET /tasks — 任务分页列表
// =====================================================================

#[derive(Debug, Deserialize)]
struct PaginationQuery {
    page: Option<usize>,
    page_size: Option<usize>,
    keyword: Option<String>,
    status: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
}

async fn list_tasks_paginated(
    State(s): State<Arc<MiscState>>,
    Query(q): Query<PaginationQuery>,
) -> ApiResponse<Value> {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
    let keyword = q.keyword.as_deref().unwrap_or("").to_lowercase();
    let status_filter = q.status.as_deref().unwrap_or("").to_lowercase();
    let sort_by = q.sort_by.as_deref().unwrap_or("created_at");
    let sort_order = q.sort_order.as_deref().unwrap_or("desc");

    let mut tasks = s.tasks.lock().clone();
    if !keyword.is_empty() {
        tasks.retain(|t| {
            t.title.to_lowercase().contains(&keyword)
                || t.description.to_lowercase().contains(&keyword)
                || t.id.to_lowercase().contains(&keyword)
        });
    }
    if !status_filter.is_empty() && status_filter != "all" {
        tasks.retain(|t| t.status == status_filter);
    }

    // 排序
    let descending = sort_order == "desc";
    match sort_by {
        "title" => tasks.sort_by(|a, b| if descending { b.title.cmp(&a.title) } else { a.title.cmp(&b.title) }),
        "status" => tasks.sort_by(|a, b| if descending { b.status.cmp(&a.status) } else { a.status.cmp(&b.status) }),
        "priority" => tasks.sort_by(|a, b| if descending { b.priority.cmp(&a.priority) } else { a.priority.cmp(&b.priority) }),
        "progress" => tasks.sort_by(|a, b| if descending { b.progress.cmp(&a.progress) } else { a.progress.cmp(&b.progress) }),
        "due_date" => tasks.sort_by(|a, b| if descending { b.due_date.cmp(&a.due_date) } else { a.due_date.cmp(&b.due_date) }),
        _ => tasks.sort_by(|a, b| if descending { b.created_at.cmp(&a.created_at) } else { a.created_at.cmp(&b.created_at) }),
    }

    let total = tasks.len();
    let start = (page - 1) * page_size;
    let items: Vec<&TaskItem> = tasks.iter().skip(start).take(page_size).collect();

    ok(json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
        "total_pages": (total + page_size - 1) / page_size,
        "has_next": start + page_size < total,
        "has_prev": page > 1,
        "filters": {
            "keyword": q.keyword,
            "status": q.status,
            "sort_by": sort_by,
            "sort_order": sort_order,
        },
    }))
}

// =====================================================================
// 5. GET /projects — 项目分页列表
// =====================================================================
async fn list_projects_paginated(
    State(s): State<Arc<MiscState>>,
    Query(q): Query<PaginationQuery>,
) -> ApiResponse<Value> {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
    let keyword = q.keyword.as_deref().unwrap_or("").to_lowercase();
    let status_filter = q.status.as_deref().unwrap_or("").to_lowercase();
    let sort_by = q.sort_by.as_deref().unwrap_or("created_at");
    let sort_order = q.sort_order.as_deref().unwrap_or("desc");

    let mut projects = s.projects.lock().clone();
    if !keyword.is_empty() {
        projects.retain(|p| {
            p.name.to_lowercase().contains(&keyword)
                || p.description.to_lowercase().contains(&keyword)
                || p.id.to_lowercase().contains(&keyword)
        });
    }
    if !status_filter.is_empty() && status_filter != "all" {
        projects.retain(|p| p.status == status_filter);
    }

    let descending = sort_order == "desc";
    match sort_by {
        "name" => projects.sort_by(|a, b| if descending { b.name.cmp(&a.name) } else { a.name.cmp(&b.name) }),
        "status" => projects.sort_by(|a, b| if descending { b.status.cmp(&a.status) } else { a.status.cmp(&b.status) }),
        "progress" => projects.sort_by(|a, b| if descending { b.progress.cmp(&a.progress) } else { a.progress.cmp(&b.progress) }),
        "member_count" => projects.sort_by(|a, b| if descending { b.member_count.cmp(&a.member_count) } else { a.member_count.cmp(&b.member_count) }),
        "task_count" => projects.sort_by(|a, b| if descending { b.task_count.cmp(&a.task_count) } else { a.task_count.cmp(&b.task_count) }),
        _ => projects.sort_by(|a, b| if descending { b.created_at.cmp(&a.created_at) } else { a.created_at.cmp(&b.created_at) }),
    }

    let total = projects.len();
    let start = (page - 1) * page_size;
    let items: Vec<&ProjectItem> = projects.iter().skip(start).take(page_size).collect();

    ok(json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
        "total_pages": (total + page_size - 1) / page_size,
        "has_next": start + page_size < total,
        "has_prev": page > 1,
        "filters": {
            "keyword": q.keyword,
            "status": q.status,
            "sort_by": sort_by,
            "sort_order": sort_order,
        },
    }))
}

// =====================================================================
// 路由装配
// =====================================================================

pub fn build_misc_router(state: Arc<MiscState>) -> Router {
    Router::new()
        .route("/api/users/:id/avatar", post(upload_avatar))
        .route("/api/market/:id/review", post(market_review))
        .route("/api/ai/flows/:id", put(update_flow))
        .route("/api/tasks", get(list_tasks_paginated))
        .route("/api/projects", get(list_projects_paginated))
        .with_state(state)
}
