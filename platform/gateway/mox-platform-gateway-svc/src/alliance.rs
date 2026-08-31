// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 联盟域（Alliance）HTTP 路由适配层
//!
//! 专家联盟域 API 网关接入：
//! - 调度器子域：任务提交/查询/取消、专家匹配
//! - 执行器子域：执行状态查询、节点管理、人工干预
//!
//! 当前阶段：**Api 模式（进程内路由桩）**，先跑通端点契约。
//! 真实实现将由 mox-alliance-scheduler-svc / mox-alliance-executor-svc 挂接。
//!
//! 路径前缀：`/alliance/v1/*`

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use mox_alliance_api::dto::*;

// ====================================================================
// 共享状态：联盟域网关状态（轻量 stub，不挂接真实核心避免 API 漂移）
// ====================================================================
#[derive(Debug, Clone)]
pub struct AllianceGatewayState {
    pub started_unix_ms: i64,
    pub stub_note: &'static str,
}

impl AllianceGatewayState {
    pub fn new() -> Self {
        Self {
            started_unix_ms: Utc::now().timestamp_millis(),
            stub_note: "联盟域 API 路由桩已就绪，真实实现将由 mox-alliance-scheduler-svc / mox-alliance-executor-svc 桥接",
        }
    }
}

impl Default for AllianceGatewayState {
    fn default() -> Self {
        Self::new()
    }
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

// ====================================================================
// 调度器子域 · 任务管理 API
// ====================================================================

/// POST /alliance/v1/tasks — 创建任务
async fn create_task(
    State(s): State<Arc<AllianceGatewayState>>,
    Json(req): Json<CreateTaskRequest>,
) -> Json<Value> {
    let t0 = now_ms();
    let task_id = Uuid::new_v4();
    let now = Utc::now();

    Json(json!({
        "ok": true,
        "elapsed_ms": now_ms() - t0,
        "stub": true,
        "note": s.stub_note,
        "data": {
            "task_id": task_id,
            "title": req.title,
            "status": "pending",
            "created_at": now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        },
        "params": {
            "description": req.description,
            "task_type": req.task_type,
            "priority": req.priority.map(|p| format!("{:?}", p)),
            "mode": req.mode.map(|m| format!("{:?}", m)),
            "fusion_strategy": req.fusion_strategy.map(|f| format!("{:?}", f)),
        },
    }))
}

/// GET /alliance/v1/tasks — 任务列表
async fn list_tasks(State(s): State<Arc<AllianceGatewayState>>) -> Json<Value> {
    let t0 = now_ms();

    Json(json!({
        "ok": true,
        "elapsed_ms": now_ms() - t0,
        "stub": true,
        "note": s.stub_note,
        "data": {
            "tasks": [],
            "total": 0,
            "page": 1,
            "page_size": 20,
        },
    }))
}

/// GET /alliance/v1/tasks/:task_id — 任务详情
async fn get_task(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
) -> Json<Value> {
    let t0 = now_ms();

    Json(json!({
        "ok": true,
        "elapsed_ms": now_ms() - t0,
        "stub": true,
        "note": s.stub_note,
        "data": {
            "task_id": task_id,
            "title": "示例任务",
            "description": "这是一个路由桩示例任务",
            "status": "pending",
            "priority": "normal",
            "progress": 0.0,
            "mode": "expert_alliance",
            "created_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "started_at": null,
            "completed_at": null,
            "duration_ms": null,
        },
    }))
}

/// POST /alliance/v1/tasks/:task_id — 任务操作（暂停/恢复/取消）
async fn handle_task_action(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<TaskActionRequest>,
) -> Json<Value> {
    let t0 = now_ms();
    let action_str = format!("{:?}", req.action);

    Json(json!({
        "ok": true,
        "elapsed_ms": now_ms() - t0,
        "stub": true,
        "note": s.stub_note,
        "data": {
            "success": true,
            "message": format!("任务 {} 操作已接受: {}", task_id, action_str),
        },
        "params": {
            "task_id": task_id,
            "action": action_str,
            "reason": req.reason,
        },
    }))
}

// ====================================================================
// 调度器子域 · 专家匹配 API
// ====================================================================

/// POST /alliance/v1/experts/search — 搜索专家
async fn search_experts(
    State(s): State<Arc<AllianceGatewayState>>,
    Json(req): Json<ExpertSearchRequest>,
) -> Json<Value> {
    let t0 = now_ms();

    Json(json!({
        "ok": true,
        "elapsed_ms": now_ms() - t0,
        "stub": true,
        "note": s.stub_note,
        "data": {
            "experts": [
                {
                    "expert_id": "expert-arch-001",
                    "name": "架构优化专家",
                    "description": "专注于系统架构设计与性能优化",
                    "domains": ["architecture", "performance"],
                    "status": "online",
                },
                {
                    "expert_id": "expert-data-001",
                    "name": "数据工程专家",
                    "description": "数据管道、ETL 与数据标准化",
                    "domains": ["data", "algorithm"],
                    "status": "online",
                },
            ],
            "total": 2,
        },
        "params": {
            "query": req.query,
            "domains": req.domains,
            "limit": req.limit,
        },
    }))
}

// ====================================================================
// 执行器子域 · 执行状态 API
// ====================================================================

/// GET /alliance/v1/tasks/:task_id/status — 执行状态查询
async fn get_execution_status(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
) -> Json<Value> {
    let t0 = now_ms();

    Json(json!({
        "ok": true,
        "elapsed_ms": now_ms() - t0,
        "stub": true,
        "note": s.stub_note,
        "data": {
            "task_id": task_id,
            "status": "running",
            "progress": 0.35,
            "total_nodes": 10,
            "completed_nodes": 3,
            "running_nodes": 2,
            "failed_nodes": 0,
            "pending_nodes": 5,
        },
    }))
}

/// GET /alliance/v1/tasks/:task_id/nodes — 节点列表
async fn list_nodes(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
) -> Json<Value> {
    let t0 = now_ms();

    Json(json!({
        "ok": true,
        "elapsed_ms": now_ms() - t0,
        "stub": true,
        "note": s.stub_note,
        "data": {
            "nodes": [
                {
                    "node_id": "node-1",
                    "name": "需求分析",
                    "expert_id": "expert-requirement-001",
                    "status": "completed",
                    "dependencies": [],
                    "started_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    "completed_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    "duration_ms": 5200,
                    "error_message": null,
                },
                {
                    "node_id": "node-2",
                    "name": "架构设计",
                    "expert_id": "expert-arch-001",
                    "status": "running",
                    "dependencies": ["node-1"],
                    "started_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    "completed_at": null,
                    "duration_ms": null,
                    "error_message": null,
                },
            ],
            "total": 2,
        },
        "params": {
            "task_id": task_id,
        },
    }))
}

/// GET /alliance/v1/tasks/:task_id/nodes/:node_id — 节点详情
async fn get_node(
    State(s): State<Arc<AllianceGatewayState>>,
    Path((task_id, node_id)): Path<(Uuid, String)>,
) -> Json<Value> {
    let t0 = now_ms();

    Json(json!({
        "ok": true,
        "elapsed_ms": now_ms() - t0,
        "stub": true,
        "note": s.stub_note,
        "data": {
            "node_id": node_id,
            "name": "示例节点",
            "expert_id": "expert-demo-001",
            "status": "running",
            "dependencies": ["node-prev-1", "node-prev-2"],
            "started_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "completed_at": null,
            "duration_ms": null,
            "error_message": null,
        },
        "params": {
            "task_id": task_id,
            "node_id": node_id,
        },
    }))
}

/// POST /alliance/v1/tasks/:task_id/nodes/:node_id — 跳过节点（人工干预）
async fn skip_node(
    State(s): State<Arc<AllianceGatewayState>>,
    Path((task_id, node_id)): Path<(Uuid, String)>,
) -> Json<Value> {
    let t0 = now_ms();

    Json(json!({
        "ok": true,
        "elapsed_ms": now_ms() - t0,
        "stub": true,
        "note": s.stub_note,
        "data": {
            "success": true,
            "message": format!("节点 {} 已跳过", node_id),
        },
        "params": {
            "task_id": task_id,
            "node_id": node_id,
        },
    }))
}

// ====================================================================
// 路由装配入口：联盟域 8 端点
// ====================================================================
/// 构建联盟域 HTTP 路由（Api 模式·进程内调用）
///
/// 包含：
/// - 调度器 5 接口：任务创建/列表/详情/操作 + 专家搜索
/// - 执行器 3 接口：执行状态/节点列表/节点详情+跳过
pub fn build_alliance_router() -> Router {
    let state = Arc::new(AllianceGatewayState::new());
    Router::new()
        // —— 调度器子域 · 任务管理 ——
        .route("/alliance/v1/tasks", post(create_task).get(list_tasks))
        .route("/alliance/v1/tasks/:task_id", get(get_task).post(handle_task_action))
        // —— 调度器子域 · 专家匹配 ——
        .route("/alliance/v1/experts/search", post(search_experts))
        // —— 执行器子域 · 执行状态 ——
        .route("/alliance/v1/tasks/:task_id/status", get(get_execution_status))
        .route("/alliance/v1/tasks/:task_id/nodes", get(list_nodes))
        .route("/alliance/v1/tasks/:task_id/nodes/:node_id", get(get_node).post(skip_node))
        .with_state(state)
}
