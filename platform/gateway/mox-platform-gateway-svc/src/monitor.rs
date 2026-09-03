// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 系统监控域（Monitor）HTTP 路由
//!
//! 提供服务质量、业务指标、告警管理、节点状态、时序查询等监控面能力。
//! 告警规则 CRUD 使用进程内 `Mutex<Vec<AlertRule>>` 存储（带初始种子数据）。
//!
//! 路径前缀：`/monitor/*` · `/actuator/metrics/detail`

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

// =====================================================================
// 共享状态
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AlertRule {
    id: String,
    name: String,
    metric: String,
    condition: String,
    threshold: f64,
    severity: String,
    enabled: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Clone)]
struct MonitorState {
    alert_rules: Arc<Mutex<Vec<AlertRule>>>,
}

impl MonitorState {
    fn new() -> Self {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let seed = vec![
            AlertRule {
                id: "rule-cpu-high".into(),
                name: "CPU 使用率过高".into(),
                metric: "cpu_usage".into(),
                condition: ">".into(),
                threshold: 85.0,
                severity: "critical".into(),
                enabled: true,
                created_at: now.clone(),
                updated_at: now.clone(),
            },
            AlertRule {
                id: "rule-mem-high".into(),
                name: "内存使用率过高".into(),
                metric: "mem_usage".into(),
                condition: ">".into(),
                threshold: 90.0,
                severity: "warning".into(),
                enabled: true,
                created_at: now.clone(),
                updated_at: now.clone(),
            },
            AlertRule {
                id: "rule-err-rate".into(),
                name: "错误率异常".into(),
                metric: "error_rate".into(),
                condition: ">".into(),
                threshold: 5.0,
                severity: "critical".into(),
                enabled: false,
                created_at: now.clone(),
                updated_at: now,
            },
        ];
        Self { alert_rules: Arc::new(Mutex::new(seed)) }
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn ok(data: Value) -> Json<Value> {
    Json(json!({ "success": true, "data": data }))
}

// =====================================================================
// 1. GET /actuator/metrics/detail — 详细指标聚合
// =====================================================================
async fn metrics_detail() -> Json<Value> {
    ok(json!({
        "cpu": {
            "usage_percent": 42.7,
            "cores": 16,
            "load_avg_1m": 3.2,
            "load_avg_5m": 2.8,
            "load_avg_15m": 2.5,
        },
        "memory": {
            "total_mb": 32768,
            "used_mb": 18432,
            "free_mb": 14336,
            "usage_percent": 56.25,
        },
        "gc": {
            "young_gc_count": 1284,
            "young_gc_time_ms": 5230,
            "full_gc_count": 12,
            "full_gc_time_ms": 890,
            "heap_used_mb": 4096,
            "heap_max_mb": 8192,
        },
        "threads": {
            "active": 48,
            "daemon": 12,
            "peak": 64,
            "deadlocked": 0,
        },
        "requests": {
            "total": 152340,
            "per_minute": 320,
            "success_rate": 99.2,
            "error_rate": 0.8,
        },
        "latency": {
            "p50_ms": 12.5,
            "p90_ms": 45.2,
            "p95_ms": 78.3,
            "p99_ms": 156.8,
            "avg_ms": 18.4,
        },
        "ts": now_iso(),
    }))
}

// =====================================================================
// 2. GET /monitor/quality — 服务质量指标
// =====================================================================
async fn quality() -> Json<Value> {
    ok(json!({
        "sla": {
            "target": 99.9,
            "actual": 99.95,
            "status": "healthy",
        },
        "availability": {
            "uptime_percent": 99.97,
            "downtime_minutes_30d": 12,
            "last_incident": "2026-08-28T10:23:00Z",
        },
        "error_rate": {
            "current": 0.35,
            "threshold": 1.0,
            "trend": "decreasing",
        },
        "avg_response_time_ms": 22.4,
        "p99_response_time_ms": 180.2,
        "apdex": 0.94,
        "ts": now_iso(),
    }))
}

// =====================================================================
// 3. GET /monitor/business — 业务指标聚合
// =====================================================================
async fn business() -> Json<Value> {
    ok(json!({
        "tasks": {
            "total": 1284,
            "running": 23,
            "completed": 1198,
            "failed": 42,
            "pending": 21,
            "today_new": 45,
        },
        "projects": {
            "total": 156,
            "active": 89,
            "completed": 52,
            "archived": 15,
            "today_new": 3,
        },
        "experts": {
            "total": 342,
            "online": 128,
            "busy": 45,
            "offline": 169,
            "avg_rating": 4.7,
        },
        "users": {
            "total": 2847,
            "active_today": 423,
            "active_7d": 1856,
            "new_today": 18,
            "retention_rate": 68.5,
        },
        "ts": now_iso(),
    }))
}

// =====================================================================
// 4. GET /monitor/alerts/summary — 告警统计
// =====================================================================
async fn alerts_summary() -> Json<Value> {
    ok(json!({
        "by_severity": {
            "critical": 3,
            "warning": 7,
            "info": 12,
        },
        "by_status": {
            "active": 8,
            "acknowledged": 4,
            "resolved": 10,
            "suppressed": 2,
        },
        "total_active": 8,
        "total_today": 22,
        "avg_resolution_minutes": 18.5,
        "ts": now_iso(),
    }))
}

// =====================================================================
// 5. GET /monitor/nodes — 服务节点状态列表
// =====================================================================
async fn nodes() -> Json<Value> {
    ok(json!({
        "nodes": [
            {
                "name": "gateway-01",
                "address": "10.0.1.11:8080",
                "status": "healthy",
                "latency_ms": 2.1,
                "version": "1.2.0",
                "uptime_secs": 86400 * 5,
                "cpu_percent": 35.2,
                "mem_percent": 48.1,
            },
            {
                "name": "gateway-02",
                "address": "10.0.1.12:8080",
                "status": "healthy",
                "latency_ms": 2.8,
                "version": "1.2.0",
                "uptime_secs": 86400 * 3,
                "cpu_percent": 41.7,
                "mem_percent": 52.3,
            },
            {
                "name": "alliance-scheduler-01",
                "address": "10.0.2.21:9090",
                "status": "healthy",
                "latency_ms": 5.4,
                "version": "0.9.3",
                "uptime_secs": 86400 * 12,
                "cpu_percent": 28.9,
                "mem_percent": 35.6,
            },
            {
                "name": "kg-service-01",
                "address": "10.0.3.31:7070",
                "status": "degraded",
                "latency_ms": 45.2,
                "version": "2.1.0",
                "uptime_secs": 86400 * 1,
                "cpu_percent": 78.4,
                "mem_percent": 82.1,
            },
        ],
        "total": 4,
        "healthy": 3,
        "degraded": 1,
        "unhealthy": 0,
        "ts": now_iso(),
    }))
}

// =====================================================================
// 6. GET /monitor/nodes/{name}/logs — 节点日志跳转
// =====================================================================
async fn node_logs(Path(name): Path<String>) -> Json<Value> {
    ok(json!({
        "node": name,
        "log_viewer_url": format!("/actuator/logs?search={name}&limit=200"),
        "recent_logs": [
            {
                "ts": now_iso(),
                "level": "INFO",
                "target": name.as_str(),
                "message": format!("[{name}] health check passed, latency=2.1ms"),
            },
            {
                "ts": now_iso(),
                "level": "INFO",
                "target": name.as_str(),
                "message": format!("[{name}] processed 152 requests in last minute"),
            },
            {
                "ts": now_iso(),
                "level": "WARN",
                "target": name.as_str(),
                "message": format!("[{name}] connection pool at 75% capacity"),
            },
        ],
    }))
}

// =====================================================================
// 7. GET /monitor/nodes/{name}/trace — 节点链路追踪
// =====================================================================
async fn node_trace(Path(name): Path<String>) -> Json<Value> {
    ok(json!({
        "node": name,
        "traces": [
            {
                "trace_id": "trace-abc123",
                "span_id": "span-001",
                "operation": format!("{name}.handle_request"),
                "start_time": now_iso(),
                "duration_ms": 24.5,
                "status": "ok",
                "http_method": "GET",
                "http_path": "/api/v1/status",
            },
            {
                "trace_id": "trace-def456",
                "span_id": "span-002",
                "operation": format!("{name}.forward_to_upstream"),
                "start_time": now_iso(),
                "duration_ms": 45.2,
                "status": "ok",
                "upstream": "alliance-scheduler",
            },
            {
                "trace_id": "trace-ghi789",
                "span_id": "span-003",
                "operation": format!("{name}.db_query"),
                "start_time": now_iso(),
                "duration_ms": 8.3,
                "status": "ok",
                "query": "SELECT * FROM tasks LIMIT 20",
            },
        ],
        "total": 3,
    }))
}

// =====================================================================
// 8-12. 告警规则 CRUD
// =====================================================================

#[derive(Debug, Deserialize)]
struct AlertRuleCreate {
    name: String,
    metric: String,
    condition: Option<String>,
    threshold: f64,
    severity: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AlertRuleUpdate {
    name: Option<String>,
    metric: Option<String>,
    condition: Option<String>,
    threshold: Option<f64>,
    severity: Option<String>,
    enabled: Option<bool>,
}

/// GET /monitor/alert-rules — 告警规则列表
async fn list_alert_rules(State(s): State<Arc<MonitorState>>) -> Json<Value> {
    let rules = s.alert_rules.lock().clone();
    ok(json!({
        "rules": rules,
        "total": rules.len(),
    }))
}

/// POST /monitor/alert-rules — 创建告警规则
async fn create_alert_rule(
    State(s): State<Arc<MonitorState>>,
    Json(body): Json<AlertRuleCreate>,
) -> Json<Value> {
    let now = now_iso();
    let rule = AlertRule {
        id: format!("rule-{}", uuid::Uuid::new_v4().simple()),
        name: body.name,
        metric: body.metric,
        condition: body.condition.unwrap_or_else(|| ">".into()),
        threshold: body.threshold,
        severity: body.severity.unwrap_or_else(|| "warning".into()),
        enabled: true,
        created_at: now.clone(),
        updated_at: now,
    };
    s.alert_rules.lock().push(rule.clone());
    ok(json!(rule))
}

/// PUT /monitor/alert-rules/{id} — 更新告警规则
async fn update_alert_rule(
    State(s): State<Arc<MonitorState>>,
    Path(id): Path<String>,
    Json(body): Json<AlertRuleUpdate>,
) -> Json<Value> {
    let mut rules = s.alert_rules.lock();
    if let Some(rule) = rules.iter_mut().find(|r| r.id == id) {
        if let Some(name) = body.name { rule.name = name; }
        if let Some(metric) = body.metric { rule.metric = metric; }
        if let Some(cond) = body.condition { rule.condition = cond; }
        if let Some(th) = body.threshold { rule.threshold = th; }
        if let Some(sev) = body.severity { rule.severity = sev; }
        if let Some(en) = body.enabled { rule.enabled = en; }
        rule.updated_at = now_iso();
        return ok(json!(rule.clone()));
    }
    Json(json!({ "success": false, "error": format!("alert rule not found: {id}") }))
}

/// DELETE /monitor/alert-rules/{id} — 删除告警规则
async fn delete_alert_rule(
    State(s): State<Arc<MonitorState>>,
    Path(id): Path<String>,
) -> Json<Value> {
    let mut rules = s.alert_rules.lock();
    let before = rules.len();
    rules.retain(|r| r.id != id);
    if rules.len() < before {
        ok(json!({ "deleted": true, "id": id }))
    } else {
        Json(json!({ "success": false, "error": format!("alert rule not found: {id}") }))
    }
}

/// PUT /monitor/alert-rules/{id}/toggle — 启用/禁用告警规则
async fn toggle_alert_rule(
    State(s): State<Arc<MonitorState>>,
    Path(id): Path<String>,
) -> Json<Value> {
    let mut rules = s.alert_rules.lock();
    if let Some(rule) = rules.iter_mut().find(|r| r.id == id) {
        rule.enabled = !rule.enabled;
        rule.updated_at = now_iso();
        return ok(json!({
            "id": rule.id,
            "enabled": rule.enabled,
            "name": rule.name,
        }));
    }
    Json(json!({ "success": false, "error": format!("alert rule not found: {id}") }))
}

// =====================================================================
// 13. GET /monitor/timeseries — 时序指标查询
// =====================================================================

#[derive(Debug, Deserialize)]
struct TimeseriesQuery {
    metric: Option<String>,
    start: Option<String>,
    end: Option<String>,
    step: Option<String>,
}

async fn timeseries(Query(q): Query<TimeseriesQuery>) -> Json<Value> {
    let metric = q.metric.unwrap_or_else(|| "cpu_usage".into());
    let step_secs: u64 = q.step.as_deref().and_then(|s| s.parse().ok()).unwrap_or(60);
    let points: Vec<Value> = (0..30)
        .map(|i| {
            let t = chrono::Utc::now() - chrono::Duration::seconds((29 - i) as i64 * step_secs as i64);
            let base = match metric.as_str() {
                "cpu_usage" => 40.0,
                "mem_usage" => 55.0,
                "request_rate" => 300.0,
                "latency_ms" => 20.0,
                _ => 50.0,
            };
            let value = base + ((i * 7 % 15) as f64) - 5.0 + (i as f64 * 0.3);
            json!({
                "ts": t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                "value": (value * 100.0).round() / 100.0,
            })
        })
        .collect();
    ok(json!({
        "metric": metric,
        "start": q.start,
        "end": q.end,
        "step": q.step.unwrap_or_else(|| "60".into()),
        "points": points,
        "count": points.len(),
    }))
}

// =====================================================================
// 14. GET /monitor/business/timeseries — 业务量时序
// =====================================================================

#[derive(Debug, Deserialize)]
struct BusinessTimeseriesQuery {
    metric: Option<String>,
    start: Option<String>,
    end: Option<String>,
}

async fn business_timeseries(Query(q): Query<BusinessTimeseriesQuery>) -> Json<Value> {
    let metric = q.metric.unwrap_or_else(|| "task_completions".into());
    let points: Vec<Value> = (0..14)
        .map(|i| {
            let t = chrono::Utc::now() - chrono::Duration::days((13 - i) as i64);
            let base = match metric.as_str() {
                "task_completions" => 85,
                "project_creations" => 5,
                "expert_consultations" => 32,
                "user_logins" => 420,
                _ => 50,
            };
            let value = base + ((i * 13 % 20) as i64) - 8;
            json!({
                "date": t.format("%Y-%m-%d").to_string(),
                "value": value,
            })
        })
        .collect();
    ok(json!({
        "metric": metric,
        "start": q.start,
        "end": q.end,
        "points": points,
        "count": points.len(),
        "granularity": "daily",
    }))
}

// =====================================================================
// 路由装配
// =====================================================================

pub fn build_monitor_router() -> Router {
    let state = Arc::new(MonitorState::new());
    Router::new()
        .route("/actuator/metrics/detail", get(metrics_detail))
        .route("/monitor/quality", get(quality))
        .route("/monitor/business", get(business))
        .route("/monitor/alerts/summary", get(alerts_summary))
        .route("/monitor/nodes", get(nodes))
        .route("/monitor/nodes/:name/logs", get(node_logs))
        .route("/monitor/nodes/:name/trace", get(node_trace))
        .route("/monitor/alert-rules", get(list_alert_rules).post(create_alert_rule))
        .route("/monitor/alert-rules/:id", put(update_alert_rule).delete(delete_alert_rule))
        .route("/monitor/alert-rules/:id/toggle", put(toggle_alert_rule))
        .route("/monitor/timeseries", get(timeseries))
        .route("/monitor/business/timeseries", get(business_timeseries))
        .with_state(state)
}
