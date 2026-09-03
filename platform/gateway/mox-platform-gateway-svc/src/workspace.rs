// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
//! # 工作台域（Workspace）HTTP 路由
//!
//! 提供通知、KPI、文件预览下载、白板持久化、历史记录、任务智能拆解与执行等工作台能力。
//!
//! 路径：`/notifications/*` · `/workspace/*` · `/files/*` · `/whiteboard/*` · `/tasks/*`

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use mox_api_protocol::{ApiResponse, api_ok, api_error, api_ok_empty};

// =====================================================================
// 共享状态
// =====================================================================

#[derive(Debug, Clone, Serialize)]
struct HistoryItem {
    id: String,
    user_id: String,
    action: String,
    target_type: String,
    target_id: String,
    target_name: String,
    detail: String,
    created_at: String,
}

#[derive(Clone)]
struct WorkspaceState {
    history: Arc<parking_lot::Mutex<Vec<HistoryItem>>>,
}

impl WorkspaceState {
    fn new() -> Self {
        let now = chrono::Utc::now();
        let seed: Vec<HistoryItem> = (0..25usize)
            .map(|i| {
                let t = now - chrono::Duration::minutes((i * 17) as i64);
                let actions = [
                    ("view_project", "project", "查看了项目"),
                    ("edit_task", "task", "编辑了任务"),
                    ("create_document", "document", "创建了文档"),
                    ("comment", "task", "发表了评论"),
                    ("upload_file", "file", "上传了文件"),
                    ("complete_task", "task", "完成了任务"),
                ];
                let (action, target_type, detail_prefix) = actions[i % actions.len()];
                HistoryItem {
                    id: format!("hist-{:04}", i + 1),
                    user_id: "admin-user".into(),
                    action: action.into(),
                    target_type: target_type.into(),
                    target_id: format!("{}-{:03}", target_type, (i % 10) + 1),
                    target_name: format!("{} #{:03}", target_type, (i % 10) + 1),
                    detail: format!("{}「{} #{}」", detail_prefix, target_type, (i % 10) + 1),
                    created_at: t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                }
            })
            .collect();
        Self { history: Arc::new(parking_lot::Mutex::new(seed)) }
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn ok(data: Value) -> ApiResponse<Value> {
    api_ok(data)
}

// =====================================================================
// 1. GET /notifications/unread-count — 未读通知数
// =====================================================================
async fn unread_count() -> ApiResponse<Value> {
    ok(json!({
        "total": 12,
        "by_type": {
            "task_assigned": 4,
            "mention": 2,
            "comment": 3,
            "system": 2,
            "alert": 1,
        },
        "latest_notification": {
            "id": "notif-001",
            "type": "task_assigned",
            "title": "新任务分配",
            "content": "您被分配了新任务「需求图谱构建」",
            "created_at": now_iso(),
            "read": false,
        },
    }))
}

// =====================================================================
// 2. GET /workspace/kpi — KPI 聚合统计
// =====================================================================
async fn workspace_kpi() -> ApiResponse<Value> {
    ok(json!({
        "tasks": {
            "todo": 8,
            "in_progress": 5,
            "completed": 42,
            "overdue": 2,
            "completion_rate": 73.7,
        },
        "projects": {
            "active": 6,
            "completed": 12,
            "at_risk": 1,
        },
        "reviews": {
            "pending": 3,
            "approved_this_week": 7,
        },
        "team": {
            "members": 15,
            "active_today": 11,
        },
        "period": "this_week",
        "ts": now_iso(),
    }))
}

// =====================================================================
// 3. GET /files/{id}/preview — 文件预览
// =====================================================================
async fn file_preview(Path(id): Path<String>) -> ApiResponse<Value> {
    let ext = if id.contains("pdf") { "pdf" }
        else if id.contains("img") || id.contains("png") || id.contains("jpg") { "image" }
        else if id.contains("doc") || id.contains("md") { "markdown" }
        else { "text" };
    ok(json!({
        "file_id": id,
        "name": format!("document-{}.{}", id, ext),
        "size_bytes": 245760,
        "mime_type": match ext {
            "pdf" => "application/pdf",
            "image" => "image/png",
            "markdown" => "text/markdown",
            _ => "text/plain",
        },
        "preview_type": ext,
        "preview_url": format!("/api/files/{}/preview-content", id),
        "thumbnail_url": format!("/api/files/{}/thumbnail", id),
        "content": if ext == "markdown" {
            "# 文档标题\n\n这是文档预览内容。\n\n## 第一节\n\n正文内容示例。".to_string()
        } else {
            String::new()
        },
        "uploaded_at": now_iso(),
    }))
}

// =====================================================================
// 4. GET /files/{id}/download — 文件下载
// =====================================================================
async fn file_download(Path(id): Path<String>) -> ApiResponse<Value> {
    ok(json!({
        "file_id": id,
        "name": format!("download-{}.bin", id),
        "size_bytes": 524288,
        "mime_type": "application/octet-stream",
        "download_url": format!("/api/files/{}/raw", id),
        "expires_in": 3600,
        "sha256": "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2",
        "ts": now_iso(),
    }))
}

// =====================================================================
// 5. POST /whiteboard/{sessionId}/save — 白板持久化
// =====================================================================

#[derive(Debug, Deserialize)]
struct WhiteboardSaveBody {
    elements: Option<Vec<Value>>,
    connections: Option<Vec<Value>>,
    viewport: Option<Value>,
    version: Option<i64>,
}

async fn whiteboard_save(
    Path(session_id): Path<String>,
    Json(body): Json<WhiteboardSaveBody>,
) -> ApiResponse<Value> {
    let element_count = body.elements.as_ref().map(|e| e.len()).unwrap_or(0);
    let connection_count = body.connections.as_ref().map(|c| c.len()).unwrap_or(0);
    ok(json!({
        "session_id": session_id,
        "saved": true,
        "version": body.version.unwrap_or(1) + 1,
        "elements_saved": element_count,
        "connections_saved": connection_count,
        "saved_at": now_iso(),
        "snapshot_id": format!("snap-{}", uuid::Uuid::new_v4().simple()),
    }))
}

// =====================================================================
// 6. GET /workspace/history — 历史记录列表（分页）
// =====================================================================

#[derive(Debug, Deserialize)]
struct HistoryQuery {
    page: Option<usize>,
    page_size: Option<usize>,
}

async fn workspace_history(
    State(s): State<Arc<WorkspaceState>>,
    Query(q): Query<HistoryQuery>,
) -> ApiResponse<Value> {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
    let history = s.history.lock();
    let total = history.len();
    let start = (page - 1) * page_size;
    let items: Vec<&HistoryItem> = history.iter().skip(start).take(page_size).collect();
    ok(json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))
}

// =====================================================================
// 7. POST /tasks/decompose — 任务智能拆解
// =====================================================================

#[derive(Debug, Deserialize)]
struct TaskDecomposeBody {
    title: String,
    description: Option<String>,
    task_type: Option<String>,
    subtask_count: Option<usize>,
}

async fn task_decompose(Json(body): Json<TaskDecomposeBody>) -> ApiResponse<Value> {
    let count = body.subtask_count.unwrap_or(5).clamp(2, 10);
    let task_type = body.task_type.unwrap_or_else(|| "general".into());
    let subtasks: Vec<Value> = (0..count)
        .map(|i| {
            let phases = ["需求分析", "方案设计", "开发实现", "测试验证", "文档交付", "评审验收", "部署上线", "运维监控"];
            let phase = phases[i % phases.len()];
            json!({
                "subtask_id": format!("sub-{:03}", i + 1),
                "title": format!("{} - {}", phase, body.title),
                "description": format!("自动拆解子任务：{}阶段，负责「{}」的{}工作", phase, body.title, phase),
                "phase": phase,
                "order": i + 1,
                "estimated_hours": (4 + i * 2) as f64,
                "dependencies": if i == 0 { vec![] as Vec<String> } else { vec![format!("sub-{:03}", i)] },
                "status": "pending",
            })
        })
        .collect();
    ok(json!({
        "original_title": body.title,
        "task_type": task_type,
        "subtasks": subtasks,
        "total_subtasks": count,
        "total_estimated_hours": subtasks.iter().filter_map(|s| s.get("estimated_hours").and_then(|v| v.as_f64())).sum::<f64>(),
        "decomposed_at": now_iso(),
    }))
}

// =====================================================================
// 8. POST /tasks/{id}/execute — 子任务执行
// =====================================================================

#[derive(Debug, Deserialize)]
struct TaskExecuteBody {
    subtask_id: Option<String>,
    executor: Option<String>,
    parameters: Option<Value>,
}

async fn task_execute(
    Path(id): Path<String>,
    Json(body): Json<TaskExecuteBody>,
) -> ApiResponse<Value> {
    let execution_id = format!("exec-{}", uuid::Uuid::new_v4().simple());
    ok(json!({
        "task_id": id,
        "subtask_id": body.subtask_id.unwrap_or_else(|| format!("{}-sub-001", id)),
        "execution_id": execution_id,
        "executor": body.executor.unwrap_or_else(|| "system".into()),
        "status": "running",
        "progress": 0.0,
        "started_at": now_iso(),
        "estimated_completion": (chrono::Utc::now() + chrono::Duration::minutes(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "parameters": body.parameters.unwrap_or(json!({})),
        "log": [
            { "ts": now_iso(), "level": "INFO", "message": "任务执行已启动" },
            { "ts": now_iso(), "level": "INFO", "message": "初始化执行环境..." },
        ],
    }))
}

// =====================================================================
// 路由装配
// =====================================================================

pub fn build_workspace_router() -> Router {
    let state = Arc::new(WorkspaceState::new());
    Router::new()
        .route("/notifications/unread-count", get(unread_count))
        .route("/workspace/kpi", get(workspace_kpi))
        .route("/files/:id/preview", get(file_preview))
        .route("/files/:id/download", get(file_download))
        .route("/whiteboard/:sessionId/save", post(whiteboard_save))
        .route("/workspace/history", get(workspace_history))
        .route("/tasks/decompose", post(task_decompose))
        .route("/tasks/:id/execute", post(task_execute))
        .with_state(state)
}
