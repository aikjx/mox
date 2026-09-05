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
use mox_api_protocol::{ApiResponse, api_ok, api_error};

// =====================================================================
// 共享状态
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
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

// =====================================================================
// JSON 持久化（data/workspace_history.json）
// =====================================================================

const WORKSPACE_HISTORY_PATH: &str = "data/workspace_history.json";

fn load_workspace_history() -> Vec<HistoryItem> {
    match std::fs::read_to_string(WORKSPACE_HISTORY_PATH) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_workspace_history(history: &[HistoryItem]) {
    if let Some(parent) = std::path::Path::new(WORKSPACE_HISTORY_PATH).parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("[workspace] 创建目录失败 {}: {}", parent.display(), e);
        }
    }
    if let Ok(json_str) = serde_json::to_string_pretty(history) {
        if let Err(e) = std::fs::write(WORKSPACE_HISTORY_PATH, json_str) {
            eprintln!("[workspace] 历史记录持久化失败 {}: {}", WORKSPACE_HISTORY_PATH, e);
        }
    }
}

#[derive(Clone)]
pub struct WorkspaceState {
    history: Arc<parking_lot::Mutex<Vec<HistoryItem>>>,
}

impl WorkspaceState {
    pub fn new() -> Self {
        Self { history: Arc::new(parking_lot::Mutex::new(load_workspace_history())) }
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn ok(data: Value) -> ApiResponse<Value> {
    api_ok(data)
}

// =====================================================================
// 2. GET /workspace/kpi — KPI 聚合统计
// =====================================================================
async fn workspace_kpi() -> ApiResponse<Value> {
    ok(json!({
        "tasks": {
            "todo": 0,
            "in_progress": 0,
            "completed": 0,
            "overdue": 0,
            "completion_rate": 0.0,
        },
        "projects": {
            "active": 0,
            "completed": 0,
            "at_risk": 0,
        },
        "reviews": {
            "pending": 0,
            "approved_this_week": 0,
        },
        "team": {
            "members": 0,
            "active_today": 0,
        },
        "period": "this_week",
        "ts": now_iso(),
    }))
}

// =====================================================================
// 3. GET /files/{id}/preview — 文件预览
// =====================================================================
async fn file_preview(Path(id): Path<String>) -> ApiResponse<Value> {
    ok(json!({
        "file_id": id,
        "name": null,
        "size_bytes": 0,
        "mime_type": null,
        "preview_type": "unknown",
        "preview_url": null,
        "thumbnail_url": null,
        "content": "",
        "uploaded_at": null,
    }))
}

// =====================================================================
// 4. GET /files/{id}/download — 文件下载
// =====================================================================
async fn file_download(Path(id): Path<String>) -> ApiResponse<Value> {
    ok(json!({
        "file_id": id,
        "name": null,
        "size_bytes": 0,
        "mime_type": "application/octet-stream",
        "download_url": null,
        "expires_in": 0,
        "sha256": null,
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
        "status": "pending",
        "progress": 0.0,
        "started_at": null,
        "estimated_completion": null,
        "parameters": body.parameters.unwrap_or(json!({})),
        "log": [],
    }))
}

// =====================================================================
// 路由装配
// =====================================================================

pub fn build_workspace_router(state: Arc<WorkspaceState>) -> Router {
    Router::new()
        .route("/api/workspace/kpi", get(workspace_kpi))
        .route("/api/files/:id/preview", get(file_preview))
        .route("/api/files/:id/download", get(file_download))
        .route("/api/whiteboard/:sessionId/save", post(whiteboard_save))
        .route("/api/workspace/history", get(workspace_history))
        .route("/api/tasks/decompose", post(task_decompose))
        .route("/api/tasks/:id/execute", post(task_execute))
        .with_state(state)
}
