// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 系统监控域（Monitor）HTTP 路由
//!
//! 提供服务质量、业务指标、告警管理、节点状态、时序查询等监控面能力。
//! 告警规则 CRUD 使用 JSON 文件持久化（data/alert_rules.json），启动时加载，变更时写回。
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
use mox_api_protocol::{ApiResponse, api_ok, api_error};
use crate::actuator::{LogStore, RuntimeMetrics};

// =====================================================================
// 告警规则 JSON 持久化
// =====================================================================

const ALERT_RULES_PATH: &str = "data/alert_rules.json";

fn load_alert_rules() -> Vec<AlertRule> {
    match std::fs::read_to_string(ALERT_RULES_PATH) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_alert_rules(rules: &[AlertRule]) {
    if let Some(parent) = std::path::Path::new(ALERT_RULES_PATH).parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("[monitor] 创建目录失败 {}: {}", parent.display(), e);
        }
    }
    if let Ok(json_str) = serde_json::to_string_pretty(rules) {
        if let Err(e) = std::fs::write(ALERT_RULES_PATH, json_str) {
            eprintln!("[monitor] 告警规则持久化失败 {}: {}", ALERT_RULES_PATH, e);
        }
    }
}

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
    /// 运行时指标（来自 GatewayState.actuator.RuntimeMetrics，真实 HTTP 请求统计）
    runtime: Arc<RuntimeMetrics>,
    /// 在线日志缓冲（来自 GatewayState.actuator.LogStore，真实进程内日志）
    logs: Arc<LogStore>,
}

impl MonitorState {
    fn new(runtime: Arc<RuntimeMetrics>, logs: Arc<LogStore>) -> Self {
        Self {
            alert_rules: Arc::new(Mutex::new(load_alert_rules())),
            runtime,
            logs,
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
// 1. GET /actuator/metrics/detail & /monitor/metrics/detail — 详细指标聚合
// =====================================================================
async fn metrics_detail(State(s): State<Arc<MonitorState>>) -> ApiResponse<Value> {
    // 真实数据：从 actuator RuntimeMetrics 获取 HTTP 请求统计
    let m = s.runtime.snapshot();
    let total = m["requests_total"].as_u64().unwrap_or(0);
    let ok_count = m["requests_2xx"].as_u64().unwrap_or(0);
    let client_err = m["requests_4xx"].as_u64().unwrap_or(0);
    let server_err = m["requests_5xx"].as_u64().unwrap_or(0);
    let active = m["active_requests"].as_i64().unwrap_or(0);
    let avg_latency = m["latency_avg_ms"].as_f64().unwrap_or(0.0);
    let error_count = client_err + server_err;
    let error_rate = if total > 0 { error_count as f64 / total as f64 } else { 0.0 };
    let success_rate = if total > 0 { ok_count as f64 / total as f64 } else { 0.0 };
    // 估算 QPS（基于存活时长）
    let uptime = m["uptime_secs"].as_u64().unwrap_or(1).max(1);
    let per_minute = (total as f64 / uptime as f64 * 60.0).round() as u64;

    ok(json!({
        "cpu": {
            // 待接入: OS 级 CPU 采集（sysinfo / procfs）；RuntimeMetrics 仅覆盖 HTTP 层
            "usage_percent": 0.0,
            "cores": 0,
            "load_avg_1m": 0.0,
            "load_avg_5m": 0.0,
            "load_avg_15m": 0.0,
        },
        "memory": {
            // 待接入: OS 级内存采集；当前无进程内存统计源
            "total_mb": 0,
            "used_mb": 0,
            "free_mb": 0,
            "usage_percent": 0.0,
        },
        "gc": {
            // 待接入: Rust 无 GC，此字段为 JVM 对标保留；可后续接入 jemalloc 统计
            "young_gc_count": 0,
            "young_gc_time_ms": 0,
            "full_gc_count": 0,
            "full_gc_time_ms": 0,
            "heap_used_mb": 0,
            "heap_max_mb": 0,
        },
        "threads": {
            // 待接入: tokio runtime 线程统计（tokio::runtime::RuntimeMetrics）
            "active": active,
            "daemon": 0,
            "peak": 0,
            "deadlocked": 0,
        },
        "requests": {
            // 真实数据：来自 RuntimeMetrics
            "total": total,
            "per_minute": per_minute,
            "success_rate": (success_rate * 1000.0).round() / 1000.0,
            "error_rate": (error_rate * 1000.0).round() / 1000.0,
        },
        "latency": {
            // 真实数据：avg 来自 RuntimeMetrics；p50/p90/p95/p99 待接入直方图统计
            "p50_ms": 0.0,
            "p90_ms": 0.0,
            "p95_ms": 0.0,
            "p99_ms": 0.0,
            "avg_ms": avg_latency,
        },
        "ts": now_iso(),
    }))
}

// =====================================================================
// 2. GET /monitor/quality — 服务质量指标
// =====================================================================
async fn quality(State(s): State<Arc<MonitorState>>) -> ApiResponse<Value> {
    // 真实数据：从 actuator RuntimeMetrics 计算服务质量
    let m = s.runtime.snapshot();
    let total = m["requests_total"].as_u64().unwrap_or(0);
    let ok_count = m["requests_2xx"].as_u64().unwrap_or(0);
    let client_err = m["requests_4xx"].as_u64().unwrap_or(0);
    let server_err = m["requests_5xx"].as_u64().unwrap_or(0);
    let avg_latency = m["latency_avg_ms"].as_f64().unwrap_or(0.0);
    let error_count = client_err + server_err;
    let error_rate = if total > 0 { error_count as f64 / total as f64 } else { 0.0 };
    let availability = if total > 0 { ok_count as f64 / total as f64 } else { 1.0 };
    let sla_status = if error_rate <= 0.01 { "healthy" } else if error_rate <= 0.05 { "warning" } else { "critical" };

    ok(json!({
        "sla": {
            // 真实数据：基于错误率评估 SLA 状态
            "target": 0.999,
            "actual": availability,
            "status": sla_status,
        },
        "availability": {
            // 真实数据：2xx 请求占比
            "uptime_percent": availability,
            "downtime_minutes_30d": 0,
            "last_incident": null,
        },
        "error_rate": {
            // 真实数据：(4xx+5xx)/total
            "current": (error_rate * 1000.0).round() / 1000.0,
            "threshold": 0.01,
            "trend": if error_rate <= 0.01 { "stable" } else { "elevated" },
        },
        // 真实数据：avg 来自 RuntimeMetrics；p99 待接入直方图
        "avg_response_time_ms": avg_latency,
        "p99_response_time_ms": 0.0,
        "apdex": 0.0,
        "ts": now_iso(),
    }))
}

// =====================================================================
// 3. GET /monitor/business — 业务指标聚合
// =====================================================================
async fn business() -> ApiResponse<Value> {
    ok(json!({
        "tasks": {
            "total": 0,
            "running": 0,
            "completed": 0,
            "failed": 0,
            "pending": 0,
            "today_new": 0,
        },
        "projects": {
            "total": 0,
            "active": 0,
            "completed": 0,
            "archived": 0,
            "today_new": 0,
        },
        "experts": {
            "total": 0,
            "online": 0,
            "busy": 0,
            "offline": 0,
            "avg_rating": 0.0,
        },
        "users": {
            "total": 0,
            "active_today": 0,
            "active_7d": 0,
            "new_today": 0,
            "retention_rate": 0.0,
        },
        "ts": now_iso(),
    }))
}

// =====================================================================
// 4. GET /monitor/alerts/summary — 告警统计
// =====================================================================
async fn alerts_summary() -> ApiResponse<Value> {
    ok(json!({
        "by_severity": {
            "critical": 0,
            "warning": 0,
            "info": 0,
        },
        "by_status": {
            "active": 0,
            "acknowledged": 0,
            "resolved": 0,
            "suppressed": 0,
        },
        "total_active": 0,
        "total_today": 0,
        "avg_resolution_minutes": 0.0,
        "ts": now_iso(),
    }))
}

// =====================================================================
// 5. GET /monitor/nodes — 服务节点状态列表
// =====================================================================
async fn nodes(State(s): State<Arc<MonitorState>>) -> ApiResponse<Value> {
    // 真实数据：网关自身节点（来自 RuntimeMetrics + LogStore）
    // 待接入: 服务发现 / 注册中心（Consul / Nacos / etcd）以获取下游服务节点列表
    let m = s.runtime.snapshot();
    let total = m["requests_total"].as_u64().unwrap_or(0);
    let server_err = m["requests_5xx"].as_u64().unwrap_or(0);
    let avg_latency = m["latency_avg_ms"].as_f64().unwrap_or(0.0);
    let health = if total == 0 { "healthy" } else if server_err as f64 / total as f64 <= 0.01 { "healthy" } else { "degraded" };

    let gateway_node = json!({
        "name": "mox-gateway",
        "type": "gateway",
        "status": "up",
        "health": health,
        "version": env!("CARGO_PKG_VERSION"),
        "address": "local",
        "requests_total": total,
        "avg_latency_ms": avg_latency,
        "5xx_errors": server_err,
        "logs_buffered": s.logs.len(),
        "uptime_secs": m["uptime_secs"],
    });

    ok(json!({
        "nodes": [gateway_node],
        "total": 1,
        "healthy": if health == "healthy" { 1 } else { 0 },
        "degraded": if health == "degraded" { 1 } else { 0 },
        "unhealthy": 0,
        "ts": now_iso(),
    }))
}

// =====================================================================
// 6. GET /monitor/nodes/{name}/logs — 节点日志查询
// =====================================================================

#[derive(Debug, Deserialize)]
struct NodeLogsQuery {
    limit: Option<usize>,
    level: Option<String>,
}

async fn node_logs(
    Path(name): Path<String>,
    Query(q): Query<NodeLogsQuery>,
    State(s): State<Arc<MonitorState>>,
) -> ApiResponse<Value> {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    // 真实数据：从 actuator LogStore 查询包含节点名的日志（target/message 匹配）
    let (total, entries) = s.logs.query(q.level.as_deref(), Some(&name), limit, 0);
    ok(json!({
        "node": name,
        "total": total,
        "returned": entries.len(),
        "logs": entries,
    }))
}

// =====================================================================
// 7. GET /monitor/nodes/{name}/trace — 节点链路追踪
// =====================================================================

#[derive(Debug, Deserialize)]
struct NodeTraceQuery {
    trace_id: Option<String>,
    limit: Option<usize>,
}

async fn node_trace(
    Path(name): Path<String>,
    Query(q): Query<NodeTraceQuery>,
    State(_s): State<Arc<MonitorState>>,
) -> ApiResponse<Value> {
    let _limit = q.limit.unwrap_or(50);
    let _trace_id = q.trace_id;
    // 待接入: 真实链路追踪源（OpenTelemetry / Jaeger / tracing-distributed）；
    // 当前 LogStore 仅存储结构化日志，无 span/trace 上下文。无真实数据源时返回空数组。
    let _ = name;
    ok(json!([]))
}

// =====================================================================
// 8-12. 告警规则 CRUD（JSON 文件持久化）
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
async fn list_alert_rules(State(s): State<Arc<MonitorState>>) -> ApiResponse<Value> {
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
) -> ApiResponse<Value> {
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
    let mut rules = s.alert_rules.lock();
    rules.push(rule.clone());
    save_alert_rules(&rules);
    ok(json!(rule))
}

/// PUT /monitor/alert-rules/{id} — 更新告警规则
async fn update_alert_rule(
    State(s): State<Arc<MonitorState>>,
    Path(id): Path<String>,
    Json(body): Json<AlertRuleUpdate>,
) -> ApiResponse<Value> {
    let mut rules = s.alert_rules.lock();
    if let Some(rule) = rules.iter_mut().find(|r| r.id == id) {
        if let Some(name) = body.name { rule.name = name; }
        if let Some(metric) = body.metric { rule.metric = metric; }
        if let Some(cond) = body.condition { rule.condition = cond; }
        if let Some(th) = body.threshold { rule.threshold = th; }
        if let Some(sev) = body.severity { rule.severity = sev; }
        if let Some(en) = body.enabled { rule.enabled = en; }
        rule.updated_at = now_iso();
        let result = rule.clone();
        save_alert_rules(&rules);
        return ok(json!(result));
    }
    api_error(404, format!("alert rule not found: {id}"))
}

/// DELETE /monitor/alert-rules/{id} — 删除告警规则
async fn delete_alert_rule(
    State(s): State<Arc<MonitorState>>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    let mut rules = s.alert_rules.lock();
    let before = rules.len();
    rules.retain(|r| r.id != id);
    if rules.len() < before {
        save_alert_rules(&rules);
        ok(json!({ "deleted": true, "id": id }))
    } else {
        api_error(404, format!("alert rule not found: {id}"))
    }
}

/// PUT /monitor/alert-rules/{id}/toggle — 启用/禁用告警规则
async fn toggle_alert_rule(
    State(s): State<Arc<MonitorState>>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    let mut rules = s.alert_rules.lock();
    if let Some(rule) = rules.iter_mut().find(|r| r.id == id) {
        rule.enabled = !rule.enabled;
        rule.updated_at = now_iso();
        let result = json!({
            "id": rule.id,
            "enabled": rule.enabled,
            "name": rule.name,
        });
        save_alert_rules(&rules);
        return ok(result);
    }
    api_error(404, format!("alert rule not found: {id}"))
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

async fn timeseries(Query(q): Query<TimeseriesQuery>, State(_s): State<Arc<MonitorState>>) -> ApiResponse<Value> {
    let metric = q.metric.unwrap_or_else(|| "cpu_usage".into());
    // 待接入: 真实指标时序存储（Prometheus / metrics crate 历史环形缓冲 / InfluxDB）；
    // RuntimeMetrics 仅维护当前快照计数，无历史时间序列。无真实数据源时返回空 points 数组。
    let _ = (q.start, q.end, q.step);
    ok(json!({
        "metric": metric,
        "points": [],
    }))
}

// =====================================================================
// 14. GET /monitor/business/timeseries — 业务指标时序查询
// =====================================================================

#[derive(Debug, Deserialize)]
struct BusinessTimeseriesQuery {
    metric: Option<String>,
    start: Option<String>,
    end: Option<String>,
    step: Option<String>,
}

async fn business_timeseries(Query(q): Query<BusinessTimeseriesQuery>, State(_s): State<Arc<MonitorState>>) -> ApiResponse<Value> {
    let metric = q.metric.unwrap_or_else(|| "task_completions".into());
    // 待接入: 真实业务指标时序源（任务/项目/专家统计的历史聚合存储）；
    // 当前无业务指标历史存储。无真实数据源时返回空 points 数组。
    let _ = (q.start, q.end, q.step);
    ok(json!({
        "metric": metric,
        "points": [],
    }))
}

// =====================================================================
// 路由装配
// =====================================================================

pub fn build_monitor_router(runtime: Arc<RuntimeMetrics>, logs: Arc<LogStore>) -> Router {
    let state = Arc::new(MonitorState::new(runtime, logs));
    Router::new()
        .route("/api/monitor/metrics/detail", get(metrics_detail))
        .route("/api/monitor/quality", get(quality))
        .route("/api/monitor/business", get(business))
        .route("/api/monitor/alerts/summary", get(alerts_summary))
        .route("/api/monitor/nodes", get(nodes))
        .route("/api/monitor/nodes/:name/logs", get(node_logs))
        .route("/api/monitor/nodes/:name/trace", get(node_trace))
        .route("/api/monitor/alert-rules", get(list_alert_rules).post(create_alert_rule))
        .route("/api/monitor/alert-rules/:id", put(update_alert_rule).delete(delete_alert_rule))
        .route("/api/monitor/alert-rules/:id/toggle", put(toggle_alert_rule))
        .route("/api/monitor/timeseries", get(timeseries))
        .route("/api/monitor/business/timeseries", get(business_timeseries))
        .with_state(state)
}
