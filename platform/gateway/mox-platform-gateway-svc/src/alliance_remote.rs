// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 联盟领域服务远程接入（Alliance Remote）
//!
//! 将独立运行的联盟领域服务接入网关专家联盟，实现**归一化调用**：
//!
//! - **调度器服务**（mox-alliance-scheduler-svc，默认 :3100）：
//!   任务创建/列表/详情/操作、专家搜索
//! - **执行器服务**（mox-alliance-executor-svc，默认 :3200）：
//!   执行状态/节点列表/节点详情+跳过/DAG
//!
//! ## 归一化语义（Norm-in / Norm-out）
//!
//! 网关对外契约（`/api/alliance/*` 响应信封与字段、枚举展示字符串）**保持
//! 不变**——远程服务的原生 DTO（mox-alliance-api）在此层归一化为网关本地
//! 形状：枚举字符串映射（`parallel`→`expert_alliance`、`active`→`online`、
//! `ready`→`pending` 等）、时间戳统一秒精度 RFC3339、字段名对齐本地 handler。
//!
//! ## 降级语义（远程优先，本地兜底）
//!
//! - **未启用**（未配置 URL 或 `MOX_ALLIANCE_REMOTE_MODE=off`）：
//!   全部走本地进程内实现（默认行为，零风险）
//! - **传输失败**（连接拒绝/超时，10s 上限）：返回明确的 503，保持远程数据源。
//!   不进行可能导致重复任务或状态丢失的本地兜底。
//! - **业务失败**（远程返回 4xx/5xx 错误体）：归一化为网关错误响应直接
//!   返回（远程已选定数据源，不产生本地脏写）
//!
//! ## 启用方式（环境变量）
//!
//! | 变量 | 说明 |
//! |------|------|
//! | `MOX_ALLIANCE_SCHEDULER_URL` | 调度器服务基址（如 `http://127.0.0.1:3100`），配置即启用调度器远程接入 |
//! | `MOX_ALLIANCE_EXECUTOR_URL` | 执行器服务基址（如 `http://127.0.0.1:3200`），配置即启用执行器远程接入 |
//! | `MOX_ALLIANCE_REMOTE_MODE` | `auto`（默认，已配置远程时固定使用远程）/ `off`（强制本地） |
//!
//! 日志 / SSE 实时流 / 协作计划 / 统计暂无远程对应端点，始终走本地实现。

use crate::alliance::{
    EXPERT_STATUS_NORM, NODE_STATUS_NORM, fusion_strategy_str, mode_str, now_ms, priority_str,
};
use mox_api_protocol::{api_error, api_ok, ApiResponse};
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

/// 远程调用超时（秒）
const REMOTE_TIMEOUT_SECS: u64 = 10;

// ====================================================================
// 远程客户端
// ====================================================================

/// 联盟领域服务远程客户端（scheduler-svc / executor-svc）
#[derive(Clone)]
pub struct RemoteAllianceClient {
    /// 调度器服务基址（None = 调度器端点走本地）
    pub scheduler_url: Option<String>,
    /// 执行器服务基址（None = 执行器端点走本地）
    pub executor_url: Option<String>,
    http: std::sync::Arc<reqwest::Client>,
}

impl RemoteAllianceClient {
    /// 从环境变量构造（未配置任何 URL 或 mode=off → None，全走本地）
    pub fn from_env() -> Option<Self> {
        let mode = std::env::var("MOX_ALLIANCE_REMOTE_MODE")
            .unwrap_or_else(|_| "auto".to_string());
        if mode.eq_ignore_ascii_case("off") {
            return None;
        }
        let scheduler_url = non_empty_env("MOX_ALLIANCE_SCHEDULER_URL");
        let executor_url = non_empty_env("MOX_ALLIANCE_EXECUTOR_URL");
        Self::explicit(scheduler_url, executor_url)
    }

    /// 显式构造（测试/编程式配置用；两者皆 None → None）
    pub fn explicit(scheduler_url: Option<String>, executor_url: Option<String>) -> Option<Self> {
        if scheduler_url.is_none() && executor_url.is_none() {
            return None;
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(REMOTE_TIMEOUT_SECS))
            .build()
            .ok()?;
        Some(Self {
            scheduler_url,
            executor_url,
            http: std::sync::Arc::new(http),
        })
    }

    async fn call(
        &self,
        base: Option<&String>,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> Option<Result<(u16, Value), String>> {
        let base = base?;
        let url = format!("{}{}", base.trim_end_matches('/'), path);
        let mut req = self.http.request(method, &url);
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => return Some(Err(format!("{} 请求失败: {}", url, e))),
        };
        let status = resp.status().as_u16();
        match resp.json::<Value>().await {
            Ok(v) => Some(Ok((status, v))),
            Err(e) => Some(Err(format!("{} 响应解析失败: {}", url, e))),
        }
    }

    /// 调度器 GET（未配置 → None 走本地）
    pub async fn scheduler_get(&self, path: &str) -> Option<Result<(u16, Value), String>> {
        let base = self.scheduler_url.clone();
        self.call(base.as_ref(), reqwest::Method::GET, path, None)
            .await
    }

    /// 调度器 POST
    pub async fn scheduler_post(
        &self,
        path: &str,
        body: &Value,
    ) -> Option<Result<(u16, Value), String>> {
        let base = self.scheduler_url.clone();
        self.call(base.as_ref(), reqwest::Method::POST, path, Some(body))
            .await
    }

    /// 执行器 GET（未配置 → None 走本地）
    pub async fn executor_get(&self, path: &str) -> Option<Result<(u16, Value), String>> {
        let base = self.executor_url.clone();
        self.call(base.as_ref(), reqwest::Method::GET, path, None)
            .await
    }

    /// 执行器 POST（无请求体）
    pub async fn executor_post_raw(&self, path: &str) -> Option<Result<(u16, Value), String>> {
        let base = self.executor_url.clone();
        self.call(base.as_ref(), reqwest::Method::POST, path, None)
            .await
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

// ====================================================================
// 归一化工具（远程 DTO → 网关本地形状）
// ====================================================================

fn now_secs() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// 时间戳统一秒精度 RFC3339（解析失败透传原值）
fn secs(v: &Value) -> Value {
    match v.as_str().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()) {
        Some(dt) => Value::String(
            dt.with_timezone(&chrono::Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        ),
        None => v.clone(),
    }
}

fn secs_opt(v: &Value) -> Value {
    if v.is_null() {
        Value::Null
    } else {
        secs(v)
    }
}

/// 协作模式：proto serde 名 → 网关展示名（对齐本地 mode_str）
fn norm_mode(s: &str) -> Value {
    match s {
        "sequential" => "single_expert",
        "parallel" => "expert_alliance",
        "iterative" => "human_in_loop",
        "hierarchical" => "autonomous",
        other => other,
    }
    .into()
}

/// 融合策略：proto serde 名 → 网关展示名（对齐本地 fusion_strategy_str）
fn norm_fusion(s: &str) -> Value {
    match s {
        "best_of" => "first_wins",
        "weighted" => "weighted_voting",
        "voting" => "rrf",
        "confidence_weighted" => "llm_judge",
        "concatenation" => "consensus",
        other => other,
    }
    .into()
}

/// 专家状态：proto serde 名 → 网关展示名
///
/// 映射表复用 `alliance::EXPERT_STATUS_NORM`（唯一真源），避免与本地枚举映射漂移。
fn norm_expert_status(s: &str) -> Value {
    lookup_norm(&EXPERT_STATUS_NORM, s).into()
}

/// 节点状态：proto serde 名 → 网关展示名（ready 归一化为 pending）
///
/// 映射表复用 `alliance::NODE_STATUS_NORM`（唯一真源）。
fn norm_node_status(s: &str) -> Value {
    lookup_norm(&NODE_STATUS_NORM, s).into()
}

/// 按归一化表查表，未命中则原样返回（向前兼容未知枚举）
fn lookup_norm(table: &[(&str, &str)], key: &str) -> String {
    table
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| (*v).to_string())
        .unwrap_or_else(|| key.to_string())
}

/// 远程 TaskDetailResponse → 网关任务 JSON（对齐本地 list/detail 字段）
fn norm_task(v: &Value) -> Value {
    json!({
        "task_id": v["task_id"],
        "title": v["title"],
        "description": v["description"],
        "status": v["status"],
        "priority": v["priority"],
        "progress": v["progress"],
        "mode": v["mode"].as_str().map(norm_mode).unwrap_or(Value::Null),
        "created_at": secs(&v["created_at"]),
        "started_at": secs_opt(&v["started_at"]),
        "completed_at": secs_opt(&v["completed_at"]),
        "duration_ms": v["duration_ms"],
    })
}

/// 远程执行器 NodeDetailResponse → 网关节点 JSON（对齐本地 nodes 字段）
fn norm_node(v: &Value) -> Value {
    json!({
        "node_id": v["node_id"],
        "name": v["name"],
        "expert_id": v["expert_id"],
        "status": v["status"].as_str().map(norm_node_status).unwrap_or(Value::Null),
        "dependencies": v["dependencies"],
        "started_at": secs_opt(&v["started_at"]),
        "completed_at": secs_opt(&v["completed_at"]),
        "duration_ms": v["duration_ms"],
        "error_message": v["error_message"],
    })
}

/// Once a remote data source is selected, an uncertain outcome must never create
/// a second local task or substitute an unrelated local execution state.
fn transport_fallback(op: &str, e: String) -> Option<ApiResponse<Value>> {
    tracing::warn!(operation = op, error = %e, "alliance remote unavailable");
    Some(api_error(503, "任务服务暂时不可用；未切换数据源。若刚提交任务，请恢复连接后刷新列表确认结果。".to_string()))
}

/// 远程业务错误（4xx/5xx 错误体）→ 归一化网关错误响应（不降级，远程已选定数据源）
fn http_err(status: u16, body: &Value, fallback_msg: String) -> ApiResponse<Value> {
    let msg = body["message"]
        .as_str()
        .map(str::to_string)
        .unwrap_or(fallback_msg);
    api_error(status as i32, msg)
}

// ====================================================================
// 调度器端点：远程优先 + 归一化
// ====================================================================

/// POST /api/alliance/tasks → 远程 POST {scheduler}/tasks
pub async fn remote_create_task(
    s: &crate::alliance::AllianceGatewayState,
    req: &mox_alliance_api::dto::CreateTaskRequest,
) -> Option<ApiResponse<Value>> {
    let t0 = now_ms();
    let body = serde_json::to_value(req).ok()?;
    let (status, v) = match s
        .remote
        .as_ref()?
        .scheduler_post("/tasks", &body)
        .await?
    {
        Ok(r) if (200..300).contains(&r.0) => r,
        Ok((st, v)) => return Some(http_err(st, &v, "任务创建失败（远程调度器）".into())),
        Err(e) => return transport_fallback("create_task", e),
    };
    let _ = status;
    Some(api_ok(json!({
        "elapsed_ms": now_ms() - t0,
        "data": {
            "task_id": v["task_id"],
            "title": v["title"].as_str().unwrap_or(req.title.as_str()),
            "status": v["status"].as_str().unwrap_or("pending"),
            "created_at": secs(&v["created_at"]),
        },
        "params": {
            "description": req.description.clone(),
            "task_type": req.task_type.clone(),
            "priority": req.priority.map(priority_str),
            "mode": req.mode.map(mode_str),
            "fusion_strategy": req.fusion_strategy.map(fusion_strategy_str),
        },
    })))
}

/// GET /api/alliance/tasks → 远程 GET {scheduler}/tasks
pub async fn remote_list_tasks(
    s: &crate::alliance::AllianceGatewayState,
) -> Option<ApiResponse<Value>> {
    let t0 = now_ms();
    let (_, v) = match s.remote.as_ref()?.scheduler_get("/tasks").await? {
        Ok(r) if (200..300).contains(&r.0) => r,
        Ok((st, v)) => return Some(http_err(st, &v, "任务列表读取失败（远程调度器）".into())),
        Err(e) => return transport_fallback("list_tasks", e),
    };
    let tasks: Vec<Value> = v["tasks"]
        .as_array()
        .map(|a| a.iter().map(norm_task).collect())
        .unwrap_or_default();
    let total = v["total"].as_u64().unwrap_or(tasks.len() as u64);
    Some(api_ok(json!({
        "elapsed_ms": now_ms() - t0,
        "data": {
            "tasks": tasks,
            "total": total,
            "page": v["page"].as_u64().unwrap_or(1),
            "page_size": v["page_size"].as_u64().unwrap_or(20),
        },
    })))
}

/// GET /api/alliance/tasks/:id → 远程 GET {scheduler}/tasks/:id
pub async fn remote_get_task(
    s: &crate::alliance::AllianceGatewayState,
    task_id: Uuid,
) -> Option<ApiResponse<Value>> {
    let t0 = now_ms();
    let (_, v) = match s
        .remote
        .as_ref()?
        .scheduler_get(&format!("/tasks/{}", task_id))
        .await?
    {
        Ok(r) if (200..300).contains(&r.0) => r,
        Ok((st, v)) => {
            return Some(http_err(st, &v, format!("任务 {} 不存在", task_id)))
        }
        Err(e) => return transport_fallback("get_task", e),
    };
    Some(api_ok(json!({
        "elapsed_ms": now_ms() - t0,
        "data": norm_task(&v),
    })))
}

/// 任务操作（暂停/恢复/取消）→ 远程 POST {scheduler}/tasks/:id
pub async fn remote_task_action(
    s: &crate::alliance::AllianceGatewayState,
    task_id: Uuid,
    req: &mox_alliance_api::dto::TaskActionRequest,
) -> Option<ApiResponse<Value>> {
    let t0 = now_ms();
    let body = serde_json::to_value(req).ok()?;
    match s
        .remote
        .as_ref()?
        .scheduler_post(&format!("/tasks/{}", task_id), &body)
        .await?
    {
        Ok((st, v)) if (200..300).contains(&st) => {
            // 归一化为本地消息文案（远程 SuccessResponse.message 为通用 "OK"）
            let message = match req.action {
                mox_alliance_api::dto::TaskAction::Pause => format!("任务 {} 已暂停", task_id),
                mox_alliance_api::dto::TaskAction::Resume => {
                    format!("任务 {} 已恢复执行", task_id)
                }
                mox_alliance_api::dto::TaskAction::Cancel => format!("任务 {} 已取消", task_id),
            };
            Some(api_ok(json!({
                "ok": true,
                "elapsed_ms": now_ms() - t0,
                "data": { "success": true, "message": message },
                "params": {
                    "task_id": task_id,
                    "action": format!("{:?}", req.action),
                    "reason": req.reason.clone(),
                },
            })))
        }
        Ok((st, v)) => Some(http_err(st, &v, format!("任务 {} 状态更新失败（远程调度器）", task_id))),
        Err(e) => transport_fallback("task_action", e),
    }
}

/// POST /api/alliance/experts/search → 远程 POST {scheduler}/experts/search
pub async fn remote_search_experts(
    s: &crate::alliance::AllianceGatewayState,
    req: &mox_alliance_api::dto::ExpertSearchRequest,
) -> Option<ApiResponse<Value>> {
    let t0 = now_ms();
    let body = serde_json::to_value(req).ok()?;
    let (_, v) = match s
        .remote
        .as_ref()?
        .scheduler_post("/experts/search", &body)
        .await?
    {
        Ok(r) if (200..300).contains(&r.0) => r,
        Ok((st, v)) => return Some(http_err(st, &v, "专家匹配失败（远程调度器）".into())),
        Err(e) => return transport_fallback("search_experts", e),
    };
    let experts: Vec<Value> = v["experts"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|e| {
                    json!({
                        "expert_id": e["expert_id"],
                        "name": e["name"],
                        "description": e["description"],
                        "domains": e["domains"],
                        "status": e["status"].as_str().map(norm_expert_status).unwrap_or(Value::Null),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    // 对齐本地语义：total = 本次匹配到的专家数
    Some(api_ok(json!({
        "elapsed_ms": now_ms() - t0,
        "data": {
            "experts": experts,
            "total": experts.len(),
        },
        "params": {
            "query": req.query.clone(),
            "domains": req.domains.clone(),
            "limit": req.limit,
        },
    })))
}

// ====================================================================
// 执行器端点：远程优先 + 归一化
// ====================================================================

/// GET /api/alliance/tasks/:id/execution-status → 远程 GET {executor}/tasks/:id/status
pub async fn remote_execution_status(
    s: &crate::alliance::AllianceGatewayState,
    task_id: Uuid,
) -> Option<ApiResponse<Value>> {
    let t0 = now_ms();
    let (_, v) = match s
        .remote
        .as_ref()?
        .executor_get(&format!("/tasks/{}/status", task_id))
        .await?
    {
        Ok(r) if (200..300).contains(&r.0) => r,
        Ok((st, v)) => return Some(http_err(st, &v, format!("任务 {} 不存在", task_id))),
        Err(e) => return transport_fallback("execution_status", e),
    };
    Some(api_ok(json!({
        "elapsed_ms": now_ms() - t0,
        "data": {
            "task_id": v["task_id"],
            "status": v["status"],
            "progress": v["progress"],
            "total_nodes": v["total_nodes"],
            "completed_nodes": v["completed_nodes"],
            "running_nodes": v["running_nodes"],
            "failed_nodes": v["failed_nodes"],
            "pending_nodes": v["pending_nodes"],
            "skipped_nodes": v["skipped_nodes"],
            "cancelled_nodes": v["cancelled_nodes"],
        },
    })))
}

/// GET /api/alliance/tasks/:id/nodes → 远程 GET {executor}/tasks/:id/nodes
pub async fn remote_list_nodes(
    s: &crate::alliance::AllianceGatewayState,
    task_id: Uuid,
) -> Option<ApiResponse<Value>> {
    let t0 = now_ms();
    let (_, v) = match s
        .remote
        .as_ref()?
        .executor_get(&format!("/tasks/{}/nodes", task_id))
        .await?
    {
        Ok(r) if (200..300).contains(&r.0) => r,
        Ok((st, v)) => return Some(http_err(st, &v, format!("任务 {} 不存在", task_id))),
        Err(e) => return transport_fallback("list_nodes", e),
    };
    let nodes: Vec<Value> = v["nodes"]
        .as_array()
        .map(|a| a.iter().map(norm_node).collect())
        .unwrap_or_default();
    Some(api_ok(json!({
        "elapsed_ms": now_ms() - t0,
        "data": {
            "nodes": nodes,
            "total": v["total"].as_u64().unwrap_or(nodes.len() as u64),
        },
        "params": { "task_id": task_id },
    })))
}

/// GET /api/alliance/tasks/:id/nodes/:node_id → 远程 GET {executor}/tasks/:id/nodes/:node_id
pub async fn remote_get_node(
    s: &crate::alliance::AllianceGatewayState,
    task_id: Uuid,
    node_id: &str,
) -> Option<ApiResponse<Value>> {
    let t0 = now_ms();
    let (_, v) = match s
        .remote
        .as_ref()?
        .executor_get(&format!("/tasks/{}/nodes/{}", task_id, node_id))
        .await?
    {
        Ok(r) if (200..300).contains(&r.0) => r,
        Ok((st, v)) => {
            return Some(http_err(
                st,
                &v,
                format!("节点 {} 不存在于任务 {}", node_id, task_id),
            ))
        }
        Err(e) => return transport_fallback("get_node", e),
    };
    Some(api_ok(json!({
        "ok": true,
        "elapsed_ms": now_ms() - t0,
        "data": norm_node(&v),
        "params": { "task_id": task_id, "node_id": node_id },
    })))
}

/// POST /api/alliance/tasks/:id/nodes/:node_id（跳过）→ 远程 POST {executor}
pub async fn remote_skip_node(
    s: &crate::alliance::AllianceGatewayState,
    task_id: Uuid,
    node_id: &str,
) -> Option<ApiResponse<Value>> {
    let t0 = now_ms();
    match s
        .remote
        .as_ref()?
        .executor_post_raw(&format!("/tasks/{}/nodes/{}", task_id, node_id))
        .await?
    {
        Ok((st, v)) if (200..300).contains(&st) => {
            Some(api_ok(json!({
                "elapsed_ms": now_ms() - t0,
                "data": {
                    "success": true,
                    "message": format!("节点 {} 已跳过", node_id),
                },
                "params": { "task_id": task_id, "node_id": node_id },
            })))
        }
        Ok((st, v)) => Some(http_err(st, &v, format!("节点 {} 跳过失败（远程执行器）", node_id))),
        Err(e) => transport_fallback("skip_node", e),
    }
}

/// GET /api/alliance/tasks/:id/dag → 远程 GET {executor}/tasks/:id/nodes
/// （DAG 形状归一化：位置按序生成、边由依赖推导、stats 汇总）
pub async fn remote_dag(
    s: &crate::alliance::AllianceGatewayState,
    task_id: Uuid,
) -> Option<ApiResponse<Value>> {
    let t0 = now_ms();
    let (_, v) = match s
        .remote
        .as_ref()?
        .executor_get(&format!("/tasks/{}/nodes", task_id))
        .await?
    {
        Ok(r) if (200..300).contains(&r.0) => r,
        Ok((st, v)) => return Some(http_err(st, &v, format!("任务 {} 不存在", task_id))),
        Err(e) => return transport_fallback("dag", e),
    };
    let raw = v["nodes"].as_array()?.clone();
    let mut total = 0i64;
    let mut completed = 0i64;
    let mut running = 0i64;
    let mut pending = 0i64;
    let mut failed = 0i64;
    let mut skipped = 0i64;
    let mut edges: Vec<Value> = Vec::new();
    let nodes: Vec<Value> = raw
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let status = n["status"].as_str().unwrap_or("pending");
            match status {
                "completed" => completed += 1,
                "running" => running += 1,
                // ready（待就绪）与本地 pending 语义一致（对齐 norm_node_status）
                "ready" | "pending" => pending += 1,
                "failed" => failed += 1,
                "skipped" | "cancelled" => skipped += 1,
                _ => pending += 1,
            }
            total += 1;
            let deps = n["dependencies"].as_array().cloned().unwrap_or_default();
            for dep in &deps {
                if let Some(dep_id) = dep.as_str() {
                    edges.push(json!({
                        "source": dep_id,
                        "target": n["node_id"],
                        "label": "依赖",
                    }));
                }
            }
            let progress = match status {
                "completed" => 100,
                "running" => 50,
                _ => 0,
            };
            json!({
                "id": n["node_id"],
                "label": n["name"],
                "name": n["name"],
                "type": "expert",
                "expert_id": n["expert_id"],
                "status": norm_node_status(status),
                "progress": progress,
                "dependencies": n["dependencies"],
                "started_at": secs_opt(&n["started_at"]),
                "completed_at": secs_opt(&n["completed_at"]),
                "duration_ms": n["duration_ms"],
                "position": { "x": 100 + i as i64 * 250, "y": 200 },
            })
        })
        .collect();
    Some(api_ok(json!({
        "elapsed_ms": now_ms() - t0,
        "data": {
            "task_id": task_id,
            "nodes": nodes,
            "edges": edges,
            "stats": {
                "total": total,
                "completed": completed,
                "running": running,
                "pending": pending,
                "failed": failed,
                "skipped": skipped,
            },
        },
    })))
}

/// GET /api/alliance/tasks/:id/fusion-result → 远程 GET {executor}/tasks/:id/result
///
/// 执行器 FusionOutput 形状与本地不同，做宽容归一化：已知字段取用，
/// 整体作为 fusion_result / result 透传；无结果（404）→ 本地 pending 形状。
pub async fn remote_fusion_result(
    s: &crate::alliance::AllianceGatewayState,
    task_id: Uuid,
) -> Option<ApiResponse<Value>> {
    let t0 = now_ms();
    match s
        .remote
        .as_ref()?
        .executor_get(&format!("/tasks/{}/result", task_id))
        .await?
    {
        Ok((st, body)) if (200..300).contains(&st) => {
            Some(api_ok(json!({
                "elapsed_ms": now_ms() - t0,
                "data": {
                    "task_id": task_id,
                    "status": "completed",
                    "fusion_status": body.get("fusion_status").cloned().unwrap_or(json!("completed")),
                    "fusion_strategy": body.get("fusion_strategy").cloned().unwrap_or(Value::Null),
                    "participating_nodes": body.get("participating_nodes").cloned().unwrap_or(json!(0)),
                    "fusion_result": body,
                    "result": body,
                    "expert_contributions": body.get("node_contributions").cloned().unwrap_or(json!([])),
                    "node_contributions": body.get("node_contributions").cloned().unwrap_or(json!([])),
                    "fused_at": body.get("fused_at").cloned().unwrap_or(Value::Null),
                },
            })))
        }
        // 远程暂无融合结果 → 归一化为本地 pending 形状（对齐本地空融合输出）
        Ok((404, _)) => Some(api_ok(json!({
            "elapsed_ms": now_ms() - t0,
            "data": {
                "task_id": task_id,
                "status": "pending",
                "fusion_status": "pending",
                "fusion_strategy": "unknown",
                "participating_nodes": 0,
                "fusion_result": {
                    "summary": "任务尚未产生可融合的节点输出",
                    "confidence": 0.0,
                    "key_findings": [],
                    "recommendations": [],
                },
                "result": {
                    "summary": "任务尚未产生可融合的节点输出",
                    "confidence": 0.0,
                    "key_findings": [],
                    "recommendations": [],
                },
                "expert_contributions": [],
                "node_contributions": [],
                "fused_at": Value::Null,
            },
        }))),
        Ok((status, body)) => Some(http_err(status, &body, "任务结果读取失败".into())),
        Err(e) => transport_fallback("fusion_result", e),
    }
}

/// GET /api/alliance/tasks/:id/status（轮询）→ 远程调度器详情 + 执行器状态合并
pub async fn remote_status_poll(
    s: &crate::alliance::AllianceGatewayState,
    task_id: Uuid,
) -> Option<ApiResponse<Value>> {
    let t0 = now_ms();
    let client = s.remote.as_ref()?;
    let task = match client
        .scheduler_get(&format!("/tasks/{}", task_id))
        .await?
    {
        Ok((st, v)) if (200..300).contains(&st) => v,
        Ok((st, v)) => return Some(http_err(st, &v, format!("任务 {} 不存在", task_id))),
        Err(e) => return transport_fallback("status_poll", e),
    };
    let exec = match client
        .executor_get(&format!("/tasks/{}/status", task_id))
        .await?
    {
        Ok((st, v)) if (200..300).contains(&st) => v,
        Ok((st, v)) => {
            return Some(http_err(
                st,
                &v,
                format!("任务 {} 执行状态不可用", task_id),
            ))
        }
        Err(e) => return transport_fallback("status_poll", e),
    };
    let total = exec["total_nodes"].as_i64().unwrap_or(0);
    let completed = exec["completed_nodes"].as_i64().unwrap_or(0);
    Some(api_ok(json!({
        "elapsed_ms": now_ms() - t0,
        "data": {
            "task_id": task["task_id"],
            "status": effective_task_status(&task, &exec),
            "progress": exec["progress"],
            // 执行器状态接口无当前节点概念，归一化为 null（P2 精化）
            "current_phase": Value::Null,
            "current_node": Value::Null,
            "current_node_name": Value::Null,
            "started_at": secs_opt(&task["started_at"]),
            "completed_at": secs_opt(&task["completed_at"]),
            "total_nodes": total,
            "completed_nodes": completed,
            "running_nodes": exec["running_nodes"],
            "pending_nodes": exec["pending_nodes"],
            "failed_nodes": exec["failed_nodes"],
            "estimated_remaining_minutes": if total > 0 && completed >= total { 0 } else { (total - completed).max(0) * 3 },
            "updated_at": now_secs(),
        },
    })))
}

fn effective_task_status(task: &Value, execution: &Value) -> Value {
    match task["status"].as_str() {
        Some("paused" | "cancelled" | "failed" | "completed") => task["status"].clone(),
        _ => match execution["status"].as_str() {
            Some("completed" | "failed" | "cancelled") => execution["status"].clone(),
            _ => task["status"].clone(),
        },
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    #[test]
    fn terminal_execution_is_not_hidden_by_stale_scheduler() {
        for status in ["completed", "failed", "cancelled"] {
            assert_eq!(effective_task_status(&json!({"status":"running"}), &json!({"status":status})), status);
        }
        assert_eq!(effective_task_status(&json!({"status":"paused"}), &json!({"status":"completed"})), "paused");
        assert_eq!(effective_task_status(&json!({"status":"running"}), &json!({"progress":1.0})), "running");
    }
}
