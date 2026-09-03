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
use mox_api_protocol::{ApiResponse, api_ok, api_error, api_ok_empty};

// =====================================================================
// 共享状态
// =====================================================================

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Clone)]
struct MiscState {
    tasks: Arc<Mutex<Vec<TaskItem>>>,
    projects: Arc<Mutex<Vec<ProjectItem>>>,
}

impl MiscState {
    fn new() -> Self {
        let now = chrono::Utc::now();
        let statuses = ["pending", "in_progress", "completed", "cancelled", "in_review"];
        let priorities = ["low", "medium", "high", "critical"];
        let tasks: Vec<TaskItem> = (0..87usize)
            .map(|i| {
                let created = now - chrono::Duration::days((i * 2) as i64);
                let status = statuses[i % statuses.len()];
                let progress = match status {
                    "completed" => 100i64,
                    "in_progress" => (20 + (i * 7) % 60) as i64,
                    "in_review" => 90,
                    "cancelled" => 0,
                    _ => 0,
                };
                TaskItem {
                    id: format!("task-{:04}", i + 1),
                    title: format!("任务 #{:04} - {} 模块开发", i + 1, ["前端", "后端", "数据", "AI", "运维"][i % 5]),
                    description: format!("这是任务 #{:04} 的详细描述，涉及{}模块的开发与测试工作。", i + 1, ["前端", "后端", "数据", "AI", "运维"][i % 5]),
                    status: status.into(),
                    priority: priorities[i % priorities.len()].into(),
                    project_id: format!("proj-{:03}", (i % 12) + 1),
                    assignee: format!("user-{:03}", (i % 8) + 1),
                    due_date: (now + chrono::Duration::days((i % 30) as i64)).format("%Y-%m-%d").to_string(),
                    created_at: created.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    updated_at: (created + chrono::Duration::hours(i as i64 % 100)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    progress,
                }
            })
            .collect();

        let proj_statuses = ["active", "completed", "archived", "paused", "planning"];
        let projects: Vec<ProjectItem> = (0..43usize)
            .map(|i| {
                let created = now - chrono::Duration::days((i * 5) as i64);
                let status = proj_statuses[i % proj_statuses.len()];
                let progress = match status {
                    "completed" => 100i64,
                    "active" => (10 + (i * 11) % 80) as i64,
                    "planning" => 5,
                    "paused" => (30 + (i * 3) % 40) as i64,
                    _ => 100,
                };
                ProjectItem {
                    id: format!("proj-{:04}", i + 1),
                    name: format!("项目 #{:04} - {} 平台", i + 1, ["知识图谱", "AI 编排", "数据治理", "联盟调度", "低代码"][i % 5]),
                    description: format!("项目 #{:04} 的详细描述，致力于构建{}相关能力。", i + 1, ["知识图谱", "AI 编排", "数据治理", "联盟调度", "低代码"][i % 5]),
                    status: status.into(),
                    owner: format!("user-{:03}", (i % 6) + 1),
                    progress,
                    member_count: ((i % 10) + 3) as i64,
                    task_count: ((i % 20) + 5) as i64,
                    created_at: created.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    updated_at: (created + chrono::Duration::hours((i * 3) as i64 % 200)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    start_date: created.format("%Y-%m-%d").to_string(),
                    end_date: (created + chrono::Duration::days(60 + (i % 30) as i64)).format("%Y-%m-%d").to_string(),
                }
            })
            .collect();

        Self {
            tasks: Arc::new(Mutex::new(tasks)),
            projects: Arc::new(Mutex::new(projects)),
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
    let _ = std::fs::create_dir_all(&upload_dir);

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
        let _ = std::fs::write(&storage_path, &data);
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

#[derive(Debug, Deserialize)]
struct MarketReviewBody {
    action: String,
    reason: Option<String>,
    reviewer: Option<String>,
}

async fn market_review(
    Path(id): Path<String>,
    Json(body): Json<MarketReviewBody>,
) -> ApiResponse<Value> {
    let action = body.action.to_lowercase();
    let approved = action == "approve";
    ok(json!({
        "template_id": id,
        "action": if approved { "approve" } else { "reject" },
        "status": if approved { "approved" } else { "rejected" },
        "reason": body.reason,
        "reviewer": body.reviewer.unwrap_or_else(|| "admin".into()),
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

pub fn build_misc_router() -> Router {
    let state = Arc::new(MiscState::new());
    Router::new()
        .route("/users/:id/avatar", post(upload_avatar))
        .route("/market/:id/review", post(market_review))
        .route("/ai/flows/:id", put(update_flow))
        .route("/tasks", get(list_tasks_paginated))
        .route("/projects", get(list_projects_paginated))
        .with_state(state)
}
