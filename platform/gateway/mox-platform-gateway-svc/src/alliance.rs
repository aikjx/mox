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
    routing::{get, post, put},
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
// 联盟任务扩展子域 · 日志/融合/DAG/完成切换/状态轮询
// ====================================================================

/// GET /alliance/tasks/:id/logs — 任务执行日志
async fn get_task_logs(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
) -> Json<Value> {
    let t0 = now_ms();
    let now = Utc::now();
    let logs: Vec<Value> = (0..15usize)
        .map(|i| {
            let t = now - chrono::Duration::seconds(((14 - i) * 30) as i64);
            let levels = ["INFO", "INFO", "INFO", "DEBUG", "WARN", "INFO"];
            let messages = [
                "任务初始化完成",
                "加载专家配置",
                "匹配专家节点",
                "启动节点 node-1: 需求分析",
                "节点 node-1 执行中...",
                "节点 node-1 完成，耗时 5.2s",
                "启动节点 node-2: 架构设计",
                "节点 node-2 执行中...",
                "检测到依赖满足",
                "融合中间结果",
                "进度更新: 35%",
                "节点 node-2 完成，耗时 12.8s",
                "启动节点 node-3: 方案评审",
                "等待人工确认",
                "任务继续执行",
            ];
            json!({
                "seq": i + 1,
                "ts": t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                "level": levels[i % levels.len()],
                "node_id": format!("node-{}", (i / 4) + 1),
                "message": messages[i % messages.len()],
            })
        })
        .collect();
    Json(json!({
        "ok": true,
        "elapsed_ms": now_ms() - t0,
        "stub": true,
        "note": s.stub_note,
        "data": {
            "task_id": task_id,
            "logs": logs,
            "total": logs.len(),
        },
    }))
}

/// GET /alliance/tasks/:id/fusion-result — 融合结果
async fn get_fusion_result(
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
            "fusion_status": "completed",
            "fusion_strategy": "weighted_voting",
            "participating_nodes": 3,
            "result": {
                "summary": "综合三位专家的分析结果，建议采用微服务架构拆分方案，优先拆分订单和用户服务。",
                "confidence": 0.87,
                "key_findings": [
                    "订单服务耦合度最高，应优先拆分",
                    "用户服务可独立部署，建议使用独立数据库",
                    "网关层需要增加限流和熔断能力",
                ],
                "recommendations": [
                    "第一阶段：拆分订单服务（预计2周）",
                    "第二阶段：拆分用户服务（预计1周）",
                    "第三阶段：优化网关和监控（预计1周）",
                ],
            },
            "node_contributions": [
                { "node_id": "node-1", "expert": "需求分析专家", "weight": 0.3, "contribution": "需求梳理和优先级排序" },
                { "node_id": "node-2", "expert": "架构设计专家", "weight": 0.4, "contribution": "架构方案设计和技术选型" },
                { "node_id": "node-3", "expert": "运维专家", "weight": 0.3, "contribution": "部署方案和运维成本评估" },
            ],
            "fused_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        },
    }))
}

/// GET /alliance/tasks/:id/dag — DAG 节点
async fn get_task_dag(
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
            "nodes": [
                {
                    "id": "node-1",
                    "name": "需求分析",
                    "expert_id": "expert-requirement-001",
                    "status": "completed",
                    "progress": 100,
                    "dependencies": [],
                    "started_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    "completed_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    "duration_ms": 5200,
                    "position": { "x": 100, "y": 200 },
                },
                {
                    "id": "node-2",
                    "name": "架构设计",
                    "expert_id": "expert-arch-001",
                    "status": "running",
                    "progress": 65,
                    "dependencies": ["node-1"],
                    "started_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    "completed_at": null,
                    "duration_ms": null,
                    "position": { "x": 350, "y": 150 },
                },
                {
                    "id": "node-3",
                    "name": "数据建模",
                    "expert_id": "expert-data-001",
                    "status": "running",
                    "progress": 40,
                    "dependencies": ["node-1"],
                    "started_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    "completed_at": null,
                    "duration_ms": null,
                    "position": { "x": 350, "y": 280 },
                },
                {
                    "id": "node-4",
                    "name": "方案评审",
                    "expert_id": "expert-review-001",
                    "status": "pending",
                    "progress": 0,
                    "dependencies": ["node-2", "node-3"],
                    "started_at": null,
                    "completed_at": null,
                    "duration_ms": null,
                    "position": { "x": 600, "y": 200 },
                },
                {
                    "id": "node-5",
                    "name": "融合输出",
                    "expert_id": "expert-fusion-001",
                    "status": "pending",
                    "progress": 0,
                    "dependencies": ["node-4"],
                    "started_at": null,
                    "completed_at": null,
                    "duration_ms": null,
                    "position": { "x": 850, "y": 200 },
                },
            ],
            "edges": [
                { "source": "node-1", "target": "node-2", "label": "依赖" },
                { "source": "node-1", "target": "node-3", "label": "依赖" },
                { "source": "node-2", "target": "node-4", "label": "依赖" },
                { "source": "node-3", "target": "node-4", "label": "依赖" },
                { "source": "node-4", "target": "node-5", "label": "依赖" },
            ],
            "stats": {
                "total": 5,
                "completed": 1,
                "running": 2,
                "pending": 2,
                "failed": 0,
            },
        },
    }))
}

/// PUT /alliance/tasks/:id/toggle-done — 完成状态切换
async fn toggle_task_done(
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
            "previous_status": "in_progress",
            "current_status": "completed",
            "toggled": true,
            "completed_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "message": format!("任务 {} 已标记为完成", task_id),
        },
    }))
}

/// GET /alliance/tasks/:id/status — 任务状态（供轮询）
async fn get_task_status_poll(
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
            "progress": 0.45,
            "current_node": "node-2",
            "current_node_name": "架构设计",
            "total_nodes": 5,
            "completed_nodes": 1,
            "running_nodes": 2,
            "pending_nodes": 2,
            "failed_nodes": 0,
            "estimated_remaining_minutes": 18,
            "updated_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
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
        // —— 联盟任务扩展 · 日志/融合/DAG/完成切换/状态轮询 ——
        .route("/alliance/tasks/:id/logs", get(get_task_logs))
        .route("/alliance/tasks/:id/fusion-result", get(get_fusion_result))
        .route("/alliance/tasks/:id/dag", get(get_task_dag))
        .route("/alliance/tasks/:id/toggle-done", put(toggle_task_done))
        .route("/alliance/tasks/:id/status", get(get_task_status_poll))
        .with_state(state)
}
