// =============================================================================
// 联盟引擎路由
// =============================================================================

use crate::app_state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 创建任务请求
#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub query: String,
    pub team_size: Option<usize>,
    pub enable_llm: Option<bool>,
    pub options: Option<serde_json::Value>,
}

/// 任务响应
#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub query: String,
    pub status: String,
    pub created_at: String,
}

/// 任务列表响应
#[derive(Debug, Serialize)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskResponse>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

/// 创建联盟任务（简化实现，实际应调用联盟引擎）
pub async fn create_task(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    let task_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // 记录指标
    state.record_request("POST", "/alliance/tasks", 201, 0);

    let response = TaskResponse {
        id: task_id,
        query: req.query,
        status: "pending".to_string(),
        created_at: now,
    };

    (StatusCode::CREATED, Json(response))
}

/// 获取任务详情
pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    state.record_request("GET", &format!("/alliance/tasks/{}", id), 200, 0);

    // 简化实现：返回模拟任务
    let response = TaskResponse {
        id,
        query: "示例查询".to_string(),
        status: "completed".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    (StatusCode::OK, Json(response))
}

/// 列出任务
pub async fn list_tasks(State(state): State<AppState>) -> impl IntoResponse {
    state.record_request("GET", "/alliance/tasks", 200, 0);

    let response = TaskListResponse {
        tasks: vec![],
        total: 0,
        page: 1,
        page_size: 20,
    };

    (StatusCode::OK, Json(response))
}
