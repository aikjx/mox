// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # MOX Gateway Actuator —— Spring Boot 风格的统一 API 管理与在线日志
//!
//! 对标 Spring Boot Actuator 提供管理面能力，全部挂载在 `/actuator/*`（公开管理面，独立于业务路由）：
//!
//! | 端点 | 说明 | Spring Boot 对标 |
//! |---|---|---|
//! | `GET /actuator` | 管理端点索引 | `/actuator` |
//! | `GET /actuator/health` | 健康检查（含依赖面） | `/actuator/health` |
//! | `GET /actuator/info` | 构建/应用信息 | `/actuator/info` |
//! | `GET /actuator/mappings` | **全部 API 注册表**（method/path/域/层/状态/启停） | `/actuator/mappings` |
//! | `GET /actuator/metrics` | 运行时指标（请求数/延迟/活跃连接/存活时长） | `/actuator/metrics` |
//! | `GET /actuator/env` | 网关配置（密钥脱敏） | `/actuator/env` |
//! | `GET/POST /actuator/loggers` | 查看/动态调整日志级别 | `/actuator/loggers` |
//! | `GET /actuator/logs` | **在线查询近期日志**（级别/关键词/分页） | `/actuator/logfile` |
//! | `GET /actuator/logs/tail` | **SSE 实时日志流（tail -f）** | `/actuator/logfile` + websocket |
//! | `DELETE /actuator/logs` | 清空日志缓冲 | — |
//! | `GET/POST /actuator/api/{id}/enable\|disable` | **按 API 运行时启停管理** | `/actuator` endpoint 启停 |
//!
//! # 三大核心能力
//!
//! 1. **API 统一注册表（RouteRegistry）**：把网关暴露的全部路由登记为静态目录
//!    （`ROUTES`），`/actuator/mappings` 动态输出；每个路由带 `AtomicBool` 启停开关，
//!    由请求可观测中间件在入口处按“最具体匹配”拦截，停用的 API 直接返回 403。
//! 2. **在线日志（LogStore）**：进程内环形缓冲（默认 4096 条）+ `broadcast` 实时广播；
//!    接入 `tracing` 管线（自定义 `Layer` 把格式化事件写入缓冲），并在请求中间件里
//!    记录 `METHOD path -> status (ms)` 的访问日志；`/actuator/logs/tail` 通过 SSE 推送。
//! 3. **运行时指标（RuntimeMetrics）**：请求总数/按状态码/按方法/平均延迟/活跃连接/存活时长。
//!
//! # 安全说明
//! 管理面 `/actuator/*` 与 `/health`、`/metrics` 一样放在 L0 公开层（与现有迁移期
//! `/api/system` 一致），便于本地运维；生产环境应把管理面回收为受保护路由或做 IP 白名单。

use axum::{
    extract::{Path, Query, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
    Json, Router,
};
use futures::stream::{self, Stream};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    convert::Infallible,
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::broadcast;

use crate::GatewayState;

// =====================================================================
// 工具函数
// =====================================================================

fn now_ms() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// 日志级别权重：TRACE=0 … ERROR=4（用于 >= 阈值过滤）
fn level_rank(level: &str) -> usize {
    match level.to_ascii_uppercase().as_str() {
        "TRACE" => 0,
        "DEBUG" => 1,
        "INFO" => 2,
        "WARN" => 3,
        "ERROR" => 4,
        _ => 2,
    }
}

fn level_from_rank(rank: usize) -> &'static str {
    match rank {
        0 => "TRACE",
        1 => "DEBUG",
        2 => "INFO",
        3 => "WARN",
        _ => "ERROR",
    }
}

// =====================================================================
// 1) LogStore —— 在线日志环形缓冲 + 实时广播
// =====================================================================

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub seq: u64,
    pub ts: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

/// 进程内日志环形缓冲：写多读少（Rust 服务端高频写入、查询低频），
/// 用 parking_lot Mutex 保护 VecDeque，写入时同步广播给 SSE 订阅者。
pub struct LogStore {
    inner: Mutex<LogInner>,
    tx: broadcast::Sender<LogEntry>,
    min_level: AtomicUsize,
    max: usize,
}

struct LogInner {
    entries: VecDeque<LogEntry>,
    seq: u64,
}

impl LogStore {
    pub fn new(max: usize) -> Arc<Self> {
        let cap = max.max(1);
        // broadcast 通道容量上限 2048，避免过量订阅缓冲占用
        let (tx, _) = broadcast::channel(cap.clamp(1, 2048));
        Arc::new(Self {
            inner: Mutex::new(LogInner {
                entries: VecDeque::with_capacity(cap),
                seq: 0,
            }),
            tx,
            min_level: AtomicUsize::new(2), // 默认 INFO
            max: cap,
        })
    }

    pub fn push(&self, level: &str, target: &str, message: impl AsRef<str>) {
        if level_rank(level) < self.min_level.load(Ordering::Relaxed) {
            return;
        }
        let entry = {
            let mut inner = self.inner.lock();
            inner.seq += 1;
            let e = LogEntry {
                seq: inner.seq,
                ts: now_ms(),
                level: level.to_ascii_uppercase(),
                target: target.to_string(),
                message: message.as_ref().to_string(),
            };
            if inner.entries.len() >= self.max {
                inner.entries.pop_front();
            }
            inner.entries.push_back(e.clone());
            e
        };
        // 广播失败（无订阅者）忽略
        let _ = self.tx.send(entry);
    }

    /// 查询日志：级别阈值 + 关键词过滤；返回 (命中的总条数, 分页结果，按最新在前)。
    pub fn query(
        &self,
        level: Option<&str>,
        search: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> (usize, Vec<LogEntry>) {
        let inner = self.inner.lock();
        let lr = level.map(level_rank).unwrap_or(0);
        let mut matched: Vec<LogEntry> = inner
            .entries
            .iter()
            .filter(|e| level_rank(&e.level) >= lr)
            .filter(|e| {
                search.is_none_or(|s| {
                    e.message.contains(s) || e.target.contains(s) || e.level.contains(s)
                })
            })
            .cloned()
            .collect();
        let total = matched.len();
        matched.reverse();
        (total, matched.into_iter().skip(offset).take(limit).collect())
    }

    /// 取最近 n 条（时间正序）
    pub fn recent(&self, n: usize) -> Vec<LogEntry> {
        let inner = self.inner.lock();
        let skip = inner.entries.len().saturating_sub(n);
        inner.entries.iter().skip(skip).cloned().collect()
    }

    pub fn clear(&self) -> usize {
        let mut inner = self.inner.lock();
        let n = inner.entries.len();
        inner.entries.clear();
        n
    }

    pub fn set_min_level(&self, level: &str) {
        self.min_level
            .store(level_rank(level), Ordering::Relaxed);
    }

    pub fn min_level(&self) -> &'static str {
        level_from_rank(self.min_level.load(Ordering::Relaxed))
    }

    pub fn len(&self) -> usize {
        self.inner.lock().entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LogEntry> {
        self.tx.subscribe()
    }
}

// =====================================================================
// 2) tracing 接入 —— 自定义 MakeWriter 把格式化事件同时回显 stdout 并写入 LogStore
// =====================================================================

/// 从已格式化日志行中探测级别（默认 full 格式：`时间 LEVEL target: msg`）
fn detect_level(line: &str) -> &'static str {
    for lvl in ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"] {
        if line.contains(&format!(" {lvl} ")) || line.starts_with(lvl) {
            return lvl;
        }
    }
    "INFO"
}

/// 每次写调用对应一条格式化日志：回显 stdout + 按行写入在线日志库
struct StoreLineWriter {
    store: Arc<LogStore>,
    out: io::Stdout,
    buf: String,
}

impl io::Write for StoreLineWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        self.out.write_all(buf)?;
        self.buf.push_str(&s);
        while let Some(nl) = self.buf.find('\n') {
            let line = self.buf[..nl].to_string();
            self.buf.drain(..=nl);
            if line.trim().is_empty() {
                continue;
            }
            let level = detect_level(&line);
            self.store.push(level, "app", line);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

struct StoreMakeWriter {
    store: Arc<LogStore>,
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for StoreMakeWriter {
    type Writer = StoreLineWriter;

    fn make_writer(&'a self) -> Self::Writer {
        StoreLineWriter {
            store: self.store.clone(),
            out: io::stdout(),
            buf: String::new(),
        }
    }
}

/// 安装全局 tracing subscriber：格式化为一行输出到 stdout，同时镜像到在线 LogStore。
/// 若进程内已有 subscriber（如测试环境），try_init 静默失败，不影响运行。
pub fn init_logging(store: &Arc<LogStore>) {
    let writer = StoreMakeWriter { store: store.clone() };
    let _ = tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .try_init();
}

// =====================================================================
// 3) RuntimeMetrics —— 运行时指标
// =====================================================================

pub struct RuntimeMetrics {
    started: Instant,
    total: AtomicU64,
    ok: AtomicU64,
    client_err: AtomicU64,
    server_err: AtomicU64,
    active: AtomicI64,
    latency_ms_sum: AtomicU64,
    by_method: Mutex<BTreeMap<String, u64>>,
}

impl Default for RuntimeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeMetrics {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            total: AtomicU64::new(0),
            ok: AtomicU64::new(0),
            client_err: AtomicU64::new(0),
            server_err: AtomicU64::new(0),
            active: AtomicI64::new(0),
            latency_ms_sum: AtomicU64::new(0),
            by_method: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn begin(&self) {
        self.active.fetch_add(1, Ordering::Relaxed);
    }

    pub fn end(&self, status: u16, dur: Duration, method: &str) {
        self.active.fetch_sub(1, Ordering::Relaxed);
        self.total.fetch_add(1, Ordering::Relaxed);
        match status {
            200..=299 => {
                self.ok.fetch_add(1, Ordering::Relaxed);
            }
            400..=499 => {
                self.client_err.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                if status >= 500 {
                    self.server_err.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        self.latency_ms_sum
            .fetch_add(dur.as_millis() as u64, Ordering::Relaxed);
        self.by_method
            .lock()
            .entry(method.to_string())
            .and_modify(|c| *c += 1)
            .or_insert(1);
    }

    pub fn snapshot(&self) -> Value {
        let total = self.total.load(Ordering::Relaxed);
        let avg = if total > 0 {
            self.latency_ms_sum.load(Ordering::Relaxed) as f64 / total as f64
        } else {
            0.0
        };
        let methods: Value = {
            let m = self.by_method.lock();
            let map: BTreeMap<String, Value> = m
                .iter()
                .map(|(k, v)| (k.clone(), json!(v)))
                .collect();
            serde_json::to_value(map).unwrap_or(json!({}))
        };
        json!({
            "uptime_secs": self.started.elapsed().as_secs(),
            "requests_total": total,
            "requests_2xx": self.ok.load(Ordering::Relaxed),
            "requests_4xx": self.client_err.load(Ordering::Relaxed),
            "requests_5xx": self.server_err.load(Ordering::Relaxed),
            "active_requests": self.active.load(Ordering::Relaxed),
            "latency_avg_ms": (avg * 10.0).round() / 10.0,
            "by_method": methods,
        })
    }
}

// =====================================================================
// 4) RouteRegistry —— 全部 API 注册表（Spring Boot mappings 对标）
// =====================================================================

#[derive(Debug)]
pub struct ApiRoute {
    pub id: &'static str,
    pub method: &'static str, // GET/POST/PUT/DELETE/ANY
    pub path: &'static str,   // axum 风格路径模式（:param / {*path}）
    pub layer: &'static str,
    pub domain: &'static str,
    pub status: &'static str, // ready | stub | proxy
    pub description: &'static str,
    pub enabled: AtomicBool,
}

const fn r(
    id: &'static str,
    method: &'static str,
    path: &'static str,
    layer: &'static str,
    domain: &'static str,
    status: &'static str,
    description: &'static str,
) -> ApiRoute {
    ApiRoute {
        id,
        method,
        path,
        layer,
        domain,
        status,
        description,
        enabled: AtomicBool::new(true),
    }
}

/// 网关暴露的全部 API 注册表（与 lib.rs / system.rs / alliance.rs / proxy.rs 逐条对齐）。
pub static ROUTES: [ApiRoute; 77] = [
    // ---- L0 接入通用 ----
    r("l0-health", "GET", "/health", "L0", "System", "ready", "存活检查"),
    r("l0-metrics", "GET", "/metrics", "L0", "System", "ready", "Prometheus 指标"),
    r("l0-status", "GET", "/api/v1/status", "L0", "System", "ready", "网关状态"),
    r("l0-domains", "GET", "/api/v1/domains", "L0", "System", "ready", "业务域描述符"),
    // ---- Actuator 管理面（默认不可停用，防自锁）----
    r("actuator-index", "GET", "/actuator", "L0", "Actuator", "ready", "管理端点索引"),
    r("actuator-health", "GET", "/actuator/health", "L0", "Actuator", "ready", "健康检查"),
    r("actuator-info", "GET", "/actuator/info", "L0", "Actuator", "ready", "构建信息"),
    r("actuator-mappings", "GET", "/actuator/mappings", "L0", "Actuator", "ready", "API 注册表"),
    r("actuator-metrics", "GET", "/actuator/metrics", "L0", "Actuator", "ready", "运行时指标"),
    r("actuator-env", "GET", "/actuator/env", "L0", "Actuator", "ready", "网关配置"),
    r("actuator-loggers", "ANY", "/actuator/loggers", "L0", "Actuator", "ready", "日志级别查看/调整"),
    r("actuator-logs", "ANY", "/actuator/logs", "L0", "Actuator", "ready", "在线日志查询/清空"),
    r("actuator-logs-tail", "GET", "/actuator/logs/tail", "L0", "Actuator", "ready", "SSE 实时日志流"),
    r("actuator-api", "ANY", "/actuator/api/:id", "L0", "Actuator", "ready", "API 启停管理"),
    // ---- L2 KG ----
    r("kg-neighborhood", "GET", "/kg/v1/neighborhood", "L2", "KG", "ready", "邻域子图"),
    r("kg-path", "GET", "/kg/v1/path", "L2", "KG", "ready", "K 条路径"),
    r("kg-shortest-path", "GET", "/kg/v1/shortest-path", "L2", "KG", "ready", "最短路径"),
    r("kg-centrality", "GET", "/kg/v1/centrality", "L2", "KG", "ready", "中心性分析"),
    r("kg-communities", "GET", "/kg/v1/communities", "L2", "KG", "ready", "社区发现"),
    r("kg-stats", "GET", "/kg/v1/stats", "L2", "KG", "ready", "图谱统计"),
    // ---- L3 AI ----
    r("ai-process", "POST", "/ai/engine/process", "L3", "AI", "ready", "意图识别→能力路由"),
    r("ai-analyze", "POST", "/ai/engine/analyze", "L3", "AI", "ready", "显式能力执行"),
    r("ai-capabilities", "GET", "/ai/engine/capabilities", "L3", "AI", "ready", "能力矩阵"),
    r("ai-metrics", "GET", "/ai/engine/metrics", "L3", "AI", "ready", "AI 引擎指标"),
    // ---- L4 Alliance ----
    r("alliance-tasks", "ANY", "/alliance/v1/tasks", "L4", "Alliance", "ready", "任务创建/列表"),
    r("alliance-task-detail", "ANY", "/alliance/v1/tasks/:task_id", "L4", "Alliance", "ready", "任务详情/动作"),
    r("alliance-experts-search", "POST", "/alliance/v1/experts/search", "L4", "Alliance", "ready", "专家搜索"),
    r("alliance-task-status", "GET", "/alliance/v1/tasks/:task_id/status", "L4", "Alliance", "ready", "执行状态"),
    r("alliance-task-nodes", "GET", "/alliance/v1/tasks/:task_id/nodes", "L4", "Alliance", "ready", "节点列表"),
    r("alliance-task-node", "ANY", "/alliance/v1/tasks/:task_id/nodes/:node_id", "L4", "Alliance", "ready", "节点详情/跳过"),
    // ---- L5 系统管理（IAM 真实链路）----
    r("sys-permissions", "GET", "/api/system/permissions", "L5", "System", "ready", "当前用户权限"),
    r("sys-dept", "ANY", "/api/system/dept", "L5", "System", "ready", "部门列表/新增"),
    r("sys-dept-tree", "GET", "/api/system/dept/tree", "L5", "System", "ready", "部门树"),
    r("sys-dept-detail", "ANY", "/api/system/dept/:id", "L5", "System", "ready", "部门详情/改/删"),
    r("sys-dept-users", "GET", "/api/system/dept/:id/users", "L5", "System", "ready", "部门用户"),
    r("sys-post", "ANY", "/api/system/post", "L5", "System", "ready", "岗位列表/新增"),
    r("sys-post-dept", "GET", "/api/system/post/dept/:deptId", "L5", "System", "ready", "部门岗位"),
    r("sys-post-detail", "ANY", "/api/system/post/:id", "L5", "System", "ready", "岗位详情/改/删"),
    r("sys-user", "ANY", "/api/system/user", "L5", "System", "ready", "用户列表/新增"),
    r("sys-user-detail", "ANY", "/api/system/user/:id", "L5", "System", "ready", "用户详情/改/删"),
    r("sys-user-resetpwd", "PUT", "/api/system/user/:id/resetPwd", "L5", "System", "ready", "重置密码"),
    r("sys-user-status", "PUT", "/api/system/user/:id/changeStatus", "L5", "System", "ready", "变更用户状态"),
    r("sys-user-roles", "ANY", "/api/system/user/:id/roles", "L5", "System", "ready", "用户角色分配"),
    r("sys-role", "ANY", "/api/system/role", "L5", "System", "ready", "角色列表/新增"),
    r("sys-role-detail", "ANY", "/api/system/role/:id", "L5", "System", "ready", "角色详情/改/删"),
    r("sys-role-menus", "ANY", "/api/system/role/:id/menuPerms", "L5", "System", "ready", "角色菜单权限"),
    r("sys-role-datapers", "ANY", "/api/system/role/:id/dataPerms", "L5", "System", "ready", "角色数据权限"),
    r("sys-role-users", "GET", "/api/system/role/:id/users", "L5", "System", "ready", "角色用户"),
    r("sys-role-copy", "POST", "/api/system/role/:id/copy", "L5", "System", "ready", "复制角色"),
    r("sys-menu-tree", "GET", "/api/system/menu/tree", "L5", "System", "ready", "菜单树"),
    r("sys-menu", "ANY", "/api/system/menu", "L5", "System", "ready", "菜单列表/新增"),
    r("sys-menu-detail", "ANY", "/api/system/menu/:id", "L5", "System", "ready", "菜单详情/改/删"),
    r("sys-dict-type", "ANY", "/api/system/dict/type", "L5", "System", "ready", "字典类型列表/新增"),
    r("sys-dict-type-all", "GET", "/api/system/dict/type/all", "L5", "System", "ready", "全部字典类型"),
    r("sys-dict-type-detail", "ANY", "/api/system/dict/type/:id", "L5", "System", "ready", "字典类型详情/改/删"),
    r("sys-dict-data", "ANY", "/api/system/dict/data", "L5", "System", "ready", "字典数据列表/新增"),
    r("sys-dict-data-type", "GET", "/api/system/dict/data/type/:dictType", "L5", "System", "ready", "按类型字典数据"),
    r("sys-dict-data-detail", "ANY", "/api/system/dict/data/:id", "L5", "System", "ready", "字典数据详情/改/删"),
    r("sys-config", "ANY", "/api/system/config", "L5", "System", "ready", "参数配置列表/新增"),
    r("sys-config-refresh", "DELETE", "/api/system/config/refresh-cache", "L5", "System", "ready", "刷新配置缓存"),
    r("sys-config-detail", "ANY", "/api/system/config/:id", "L5", "System", "ready", "参数配置详情/改/删"),
    r("sys-config-key", "GET", "/api/system/config/key/:key", "L5", "System", "ready", "按 key 查配置"),
    r("sys-operlog", "GET", "/api/system/operlog", "L5", "System", "ready", "操作日志列表"),
    r("sys-operlog-clean", "DELETE", "/api/system/operlog/clean", "L5", "System", "ready", "清空操作日志"),
    r("sys-operlog-detail", "ANY", "/api/system/operlog/:id", "L5", "System", "ready", "操作日志详情/删"),
    r("sys-operlog-export", "GET", "/api/system/operlog/export", "L5", "System", "ready", "导出操作日志"),
    r("sys-loginlog", "GET", "/api/system/logininfor", "L5", "System", "ready", "登录日志列表"),
    r("sys-loginlog-clean", "DELETE", "/api/system/logininfor/clean", "L5", "System", "ready", "清空登录日志"),
    r("sys-loginlog-detail", "DELETE", "/api/system/logininfor/:id", "L5", "System", "ready", "删除登录日志"),
    r("sys-loginlog-export", "GET", "/api/system/logininfor/export", "L5", "System", "ready", "导出登录日志"),
    // ---- L5 安全域 ----
    r("sec-status", "GET", "/api/security/status", "L5", "Security", "ready", "安全状态"),
    r("sec-api-keys", "ANY", "/api/security/api-keys", "L5", "Security", "ready", "凭证列表/创建"),
    r("sec-api-key-revoke", "DELETE", "/api/security/api-keys/:id", "L5", "Security", "ready", "吊销凭证"),
    r("sec-api-key-validate", "POST", "/api/security/validate", "L5", "Security", "ready", "校验凭证"),
    r("sec-audit-log", "GET", "/api/security/audit-log", "L5", "Security", "ready", "审计日志"),
    // ---- L6 业务域反向代理（网关→编排器/PrimiFlow）----
    r("proxy-orchestrator", "ANY", "/api/{*path}", "L6", "Proxy", "proxy", "转发编排器(:3001)"),
    r("proxy-primiflow", "ANY", "/api/projects/{*path}", "L6", "Proxy", "proxy", "转发 PrimiFlow(:8000)"),
];

/// 判断路径是否属于管理面（管理端点不允许被停用，防止自锁）
fn is_management(path: &str) -> bool {
    path == "/health"
        || path == "/metrics"
        || path.starts_with("/actuator")
}

fn segments(s: &str) -> Vec<&str> {
    s.split('/').filter(|x| !x.is_empty()).collect()
}

/// 返回 (是否参数段, 是否通配段)
fn seg_kind(seg: &str) -> (bool, bool) {
    if seg.starts_with('{') && seg.ends_with('}') {
        let inner = &seg[1..seg.len() - 1];
        return (true, inner.starts_with('*'));
    }
    (seg.starts_with(':'), seg.starts_with('*'))
}

fn path_matches(pattern: &str, path: &str) -> bool {
    let p = segments(pattern);
    let a = segments(path);
    let mut i = 0usize;
    for seg in p {
        let (is_param, is_wild) = seg_kind(seg);
        if is_wild {
            return true; // 通配段匹配剩余全部
        }
        if i >= a.len() {
            return false;
        }
        if !is_param && seg != a[i] {
            return false;
        }
        i += 1;
    }
    i == a.len()
}

/// 具体度 = 字面量段数量（越高越具体）
fn specificity(pattern: &str) -> usize {
    segments(pattern)
        .iter()
        .filter(|s| !seg_kind(s).0)
        .count()
}

fn method_ok(route: &ApiRoute, method: &str) -> bool {
    route.method == "ANY" || route.method == method
}

/// 在注册表中查找“最具体”匹配的路由（用于启停拦截）。
pub fn match_best(method: &str, path: &str) -> Option<&'static ApiRoute> {
    let mut best: Option<&'static ApiRoute> = None;
    let mut best_spec = 0usize;
    for route in ROUTES.iter() {
        if !method_ok(route, method) {
            continue;
        }
        if path_matches(route.path, path) {
            let s = specificity(route.path);
            if best.is_none() || s > best_spec {
                best = Some(route);
                best_spec = s;
            }
        }
    }
    best
}

pub fn get_route(id: &str) -> Option<&'static ApiRoute> {
    ROUTES.iter().find(|r| r.id == id)
}

// =====================================================================
// 5) 请求可观测中间件（日志 + 指标 + API 启停拦截）
// =====================================================================

pub async fn observability_middleware(
    State(state): State<GatewayState>,
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().as_str().to_string();
    let path = req.uri().path().to_string();
    state.runtime.begin();

    // API 启停拦截：最具体匹配的路由被停用 → 403（管理面豁免）
    if let Some(route) = match_best(&method, &path) {
        if !route.enabled.load(Ordering::Relaxed) && !is_management(&path) {
            state.runtime.end(403, Duration::ZERO, &method);
            state.logs.push(
                "WARN",
                "gateway",
                format!("API 已停用被拦截: {method} {path} (id={})", route.id),
            );
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "ok": false,
                    "code": "API_DISABLED",
                    "id": route.id,
                    "path": path,
                    "message": format!("API `{}` 已被管理端停用，请在 /actuator/api/{} 恢复", route.id, route.id),
                })),
            )
                .into_response();
        }
    }

    let start = Instant::now();
    let resp = next.run(req).await;
    let status = resp.status().as_u16();
    let dur = start.elapsed();
    state.runtime.end(status, dur, &method);

    let level = if status >= 500 {
        "ERROR"
    } else if status >= 400 {
        "WARN"
    } else {
        "INFO"
    };
    state.logs.push(
        level,
        "gateway",
        format!("{method} {path} -> {status} ({}ms)", dur.as_millis()),
    );
    resp
}

// =====================================================================
// 6) Actuator 管理端点
// =====================================================================

pub fn build_actuator_router() -> Router<GatewayState> {
    Router::new()
        .route("/actuator", get(actuator_index))
        .route("/actuator/health", get(actuator_health))
        .route("/actuator/info", get(actuator_info))
        .route("/actuator/mappings", get(actuator_mappings))
        .route("/actuator/metrics", get(actuator_metrics))
        .route("/actuator/env", get(actuator_env))
        .route("/actuator/loggers", get(actuator_loggers).post(actuator_loggers_set))
        .route("/actuator/logs", get(actuator_logs).delete(actuator_logs_clear))
        .route("/actuator/logs/tail", get(actuator_logs_tail))
        .route("/actuator/api/:id", get(actuator_api_get))
        .route("/actuator/api/:id/enable", post(actuator_api_enable))
        .route("/actuator/api/:id/disable", post(actuator_api_disable))
}

/// GET /actuator —— 管理端点索引
async fn actuator_index() -> Json<Value> {
    let endpoints = vec![
        json!({"id":"health","href":"/actuator/health","method":"GET","desc":"健康检查"}),
        json!({"id":"info","href":"/actuator/info","method":"GET","desc":"构建信息"}),
        json!({"id":"mappings","href":"/actuator/mappings","method":"GET","desc":"全部 API 注册表"}),
        json!({"id":"metrics","href":"/actuator/metrics","method":"GET","desc":"运行时指标"}),
        json!({"id":"env","href":"/actuator/env","method":"GET","desc":"网关配置(脱敏)"}),
        json!({"id":"loggers","href":"/actuator/loggers","method":"GET/POST","desc":"日志级别查看/调整"}),
        json!({"id":"logs","href":"/actuator/logs","method":"GET/DELETE","desc":"在线日志查询/清空"}),
        json!({"id":"logs-tail","href":"/actuator/logs/tail","method":"GET(SSE)","desc":"SSE 实时日志流"}),
        json!({"id":"api","href":"/actuator/api/{id}","method":"GET/POST","desc":"按 API 启停管理"}),
    ];
    Json(json!({
        "_links": endpoints,
        "total_endpoints": endpoints.len(),
        "framework": "MOX Gateway Actuator (Spring Boot style)",
        "ts": now_ms(),
    }))
}

/// GET /actuator/health
async fn actuator_health(State(state): State<GatewayState>) -> Json<Value> {
    Json(json!({
        "status": "UP",
        "components": {
            "gateway": {"status": "UP", "version": env!("CARGO_PKG_VERSION")},
            "iam": {"status": "UP", "db": "sqlite"},
            "logs": {"status": "UP", "buffered": state.logs.len()},
            "auth": {"status": if state.config.auth.enabled {"ENABLED"} else {"DISABLED"}},
            "rate_limit": {"status": if state.config.rate_limit.enabled {"ENABLED"} else {"DISABLED"}},
        },
        "uptime_secs": state.runtime.snapshot()["uptime_secs"],
        "ts": now_ms(),
    }))
}

/// GET /actuator/info
async fn actuator_info() -> Json<Value> {
    Json(json!({
        "app": {
            "name": "mox-gateway",
            "description": "MOX 全维低代码平台 · 企业级网关",
            "version": env!("CARGO_PKG_VERSION"),
            "framework": "Rust / axum 0.7",
            "build_time": option_env!("BUILD_TIME_UTC").unwrap_or("unknown"),
        },
        "ts": now_ms(),
    }))
}

/// GET /actuator/mappings —— 全部 API 注册表（支持过滤）
#[derive(Debug, Deserialize)]
pub struct MappingsQuery {
    pub layer: Option<String>,
    pub domain: Option<String>,
    pub status: Option<String>,
    pub q: Option<String>,
    pub only_enabled: Option<String>,
}

async fn actuator_mappings(
    Query(q): Query<MappingsQuery>,
) -> Json<Value> {
    let only_enabled = q
        .only_enabled
        .as_deref()
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let mut list = Vec::new();
    for route in ROUTES.iter() {
        if let Some(layer) = &q.layer {
            if route.layer != layer {
                continue;
            }
        }
        if let Some(domain) = &q.domain {
            if route.domain != domain {
                continue;
            }
        }
        if let Some(status) = &q.status {
            if route.status != status {
                continue;
            }
        }
        if let Some(kw) = &q.q {
            let kw = kw.to_lowercase();
            if !(route.id.to_lowercase().contains(&kw)
                || route.path.to_lowercase().contains(&kw)
                || route.domain.to_lowercase().contains(&kw)
                || route.description.to_lowercase().contains(&kw))
            {
                continue;
            }
        }
        let enabled = route.enabled.load(Ordering::Relaxed);
        if only_enabled && !enabled {
            continue;
        }
        list.push(json!({
            "id": route.id,
            "method": route.method,
            "path": route.path,
            "layer": route.layer,
            "domain": route.domain,
            "status": route.status,
            "description": route.description,
            "enabled": enabled,
        }));
    }
    let disabled = ROUTES
        .iter()
        .filter(|r| !r.enabled.load(Ordering::Relaxed))
        .count();
    Json(json!({
        "ok": true,
        "total": ROUTES.len(),
        "filtered": list.len(),
        "disabled_total": disabled,
        "contexts": {
            "mox-gateway": {
                "dispatcher_servlet": "axum 0.7 Router",
                "routes": list,
            }
        }
    }))
}

/// GET /actuator/metrics
async fn actuator_metrics(State(state): State<GatewayState>) -> Json<Value> {
    let m = state.runtime.snapshot();
    Json(json!({
        "names": ["requests_total", "requests_2xx", "requests_4xx", "requests_5xx", "active_requests", "latency_avg_ms", "uptime_secs"],
        "measurements": m,
        "ts": now_ms(),
    }))
}

/// GET /actuator/env —— 网关配置（密钥脱敏）
async fn actuator_env(State(state): State<GatewayState>) -> Json<Value> {
    let cfg = &state.config;
    let jwt_secret = if cfg.auth.jwt_secret.is_empty() {
        "<empty>"
    } else if cfg.auth.jwt_secret == "change-me-in-production" {
        "change-me-in-production"
    } else {
        &cfg.auth.jwt_secret[..8.min(cfg.auth.jwt_secret.len())]
    };
    Json(json!({
        "config": {
            "host": cfg.host,
            "port": cfg.port,
            "auth": {
                "enabled": cfg.auth.enabled,
                "token_issuer": cfg.auth.token_issuer,
                "jwt_secret": format!("{jwt_secret}***"),
                "public_paths": cfg.auth.public_paths,
            },
            "rate_limit": {
                "enabled": cfg.rate_limit.enabled,
                "max_requests": cfg.rate_limit.max_requests,
                "window_secs": cfg.rate_limit.window_secs,
                "burst": cfg.rate_limit.burst,
            },
            "routing": {
                "path_routing": cfg.routing.path_routing,
                "header_routing": cfg.routing.header_routing,
            },
        },
        "ts": now_ms(),
    }))
}

/// GET /actuator/loggers —— 当前日志级别
async fn actuator_loggers(State(state): State<GatewayState>) -> Json<Value> {
    Json(json!({
        "levels": ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"],
        "configured_level": state.logs.min_level(),
        "effective_level": state.logs.min_level(),
        "buffered": state.logs.len(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct LoggerSetBody {
    pub level: String,
}

/// POST /actuator/loggers —— 动态调整日志级别
async fn actuator_loggers_set(
    State(state): State<GatewayState>,
    Json(body): Json<LoggerSetBody>,
) -> Response {
    let level = body.level.to_ascii_uppercase();
    if !matches!(level.as_str(), "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": "level 必须为 TRACE/DEBUG/INFO/WARN/ERROR",
                "got": level,
            })),
        )
            .into_response();
    }
    state.logs.set_min_level(&level);
    state
        .logs
        .push("INFO", "actuator", format!("日志级别调整为 {level}"));
    Json(json!({
        "ok": true,
        "configured_level": state.logs.min_level(),
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub level: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// GET /actuator/logs —— 在线查询日志
async fn actuator_logs(
    State(state): State<GatewayState>,
    Query(q): Query<LogsQuery>,
) -> Json<Value> {
    let limit = q.limit.unwrap_or(200).clamp(1, 2000);
    let offset = q.offset.unwrap_or(0);
    let (total, entries) = state
        .logs
        .query(q.level.as_deref(), q.search.as_deref(), limit, offset);
    Json(json!({
        "ok": true,
        "total": total,
        "returned": entries.len(),
        "level": q.level,
        "search": q.search,
        "min_level": state.logs.min_level(),
        "logs": entries,
        "ts": now_ms(),
    }))
}

/// DELETE /actuator/logs —— 清空日志缓冲
async fn actuator_logs_clear(State(state): State<GatewayState>) -> Json<Value> {
    let cleared = state.logs.clear();
    Json(json!({ "ok": true, "cleared": cleared }))
}

/// GET /actuator/logs/tail —— SSE 实时日志流（tail -f）
async fn actuator_logs_tail(
    State(state): State<GatewayState>,
    Query(q): Query<LogsQuery>,
) -> Response {
    let replay: usize = q.limit.unwrap_or(100).clamp(0, 1000);
    let recent: VecDeque<LogEntry> = state.logs.recent(replay).into_iter().collect();
    let rx = state.logs.subscribe();

    let stream: std::pin::Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> = Box::pin(
        stream::unfold((recent, rx), |(mut pending, mut rx)| async move {
            if let Some(e) = pending.pop_front() {
                return Some((
                    Ok(Event::default().data(json!(e).to_string())),
                    (pending, rx),
                ));
            }
            match rx.recv().await {
                Ok(e) => Some((
                    Ok(Event::default().data(json!(e).to_string())),
                    (pending, rx),
                )),
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    Some((Ok(Event::default().comment("consumer lagged, dropped older entries")), (pending, rx)))
                }
                Err(broadcast::error::RecvError::Closed) => None,
            }
        }),
    );

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

/// GET /actuator/api/:id —— 查询单个 API 状态
async fn actuator_api_get(Path(id): Path<String>) -> Json<Value> {
    match get_route(&id) {
        Some(route) => Json(json!({
            "ok": true,
            "id": route.id,
            "method": route.method,
            "path": route.path,
            "layer": route.layer,
            "domain": route.domain,
            "status": route.status,
            "description": route.description,
            "enabled": route.enabled.load(Ordering::Relaxed),
        })),
        None => Json(json!({
            "ok": false,
            "error": format!("未找到 API: {id}"),
            "hint": "可枚举 /actuator/mappings 获取 id",
        })),
    }
}

/// 启停状态变更（共用逻辑）
fn set_api_enabled(id: &str, enabled: bool, store: &LogStore) -> Json<Value> {
    match get_route(id) {
        Some(route) => {
            if is_management(route.path) {
                return Json(json!({
                    "ok": false,
                    "error": format!("管理面端点 `{id}` 不允许停用（防自锁）"),
                }));
            }
            route.enabled.store(enabled, Ordering::Relaxed);
            store.push(
                "INFO",
                "actuator",
                format!("API `{id}` ({}) 已{}", route.path, if enabled { "启用" } else { "停用" }),
            );
            Json(json!({
                "ok": true,
                "id": route.id,
                "path": route.path,
                "method": route.method,
                "enabled": enabled,
                "message": format!("API `{id}` 已{}，停用后请求将返回 403", if enabled { "启用" } else { "停用" }),
            }))
        }
        None => Json(json!({
            "ok": false,
            "error": format!("未找到 API: {id}"),
        })),
    }
}

/// POST /actuator/api/:id/enable
async fn actuator_api_enable(State(state): State<GatewayState>, Path(id): Path<String>) -> Json<Value> {
    set_api_enabled(&id, true, &state.logs)
}

/// POST /actuator/api/:id/disable
async fn actuator_api_disable(State(state): State<GatewayState>, Path(id): Path<String>) -> Json<Value> {
    set_api_enabled(&id, false, &state.logs)
}

// =====================================================================
// 单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_matching_params_and_wildcard() {
        assert!(path_matches("/kg/v1/neighborhood", "/kg/v1/neighborhood"));
        assert!(path_matches("/api/system/user/:id", "/api/system/user/u1"));
        assert!(!path_matches("/api/system/user/:id", "/api/system/user/u1/roles"));
        assert!(path_matches("/api/system/user/:id/roles", "/api/system/user/u1/roles"));
        assert!(path_matches("/api/{*path}", "/api/anything/here"));
        assert!(path_matches("/api/projects/{*path}", "/api/projects/foo"));
        assert!(!path_matches("/api/projects/{*path}", "/api/system/dept"));
    }

    #[test]
    fn test_match_best_prefers_specific() {
        // /api/system/user/u1/roles 应命中更具体的 roles 路由而非 user/:id
        let m = match_best("GET", "/api/system/user/u1/roles").expect("matched");
        assert_eq!(m.id, "sys-user-roles");
        // 普通 /api/system/user 命中系统路由而非代理通配
        let m2 = match_best("GET", "/api/system/user").expect("matched");
        assert_eq!(m2.id, "sys-user");
        // /api/others 落入代理
        let m3 = match_best("GET", "/api/others/foo").expect("matched");
        assert_eq!(m3.id, "proxy-orchestrator");
    }

    #[test]
    fn test_match_best_respects_method() {
        let m = match_best("POST", "/ai/engine/process").expect("matched");
        assert_eq!(m.id, "ai-process");
        // GET 不匹配 POST 路由 → 应落到别的（无匹配则 None）
        assert!(match_best("GET", "/ai/engine/process").is_none());
    }

    #[test]
    fn test_log_store_push_query_cap_and_level() {
        let store = LogStore::new(8);
        for i in 0..20 {
            store.push("INFO", "test", format!("msg-{i}"));
        }
        // 容量上限
        assert_eq!(store.len(), 8);
        let (total, entries) = store.query(None, None, 100, 0);
        assert_eq!(total, 8);
        // 最新在前
        assert!(entries[0].message.ends_with("msg-19"));
        // 关键词过滤
        let (t2, _) = store.query(None, Some("msg-1"), 100, 0);
        assert!(t2 >= 1);
        // 级别过滤：默认 INFO，WARN 也写入但 TRACE 被过滤
        store.set_min_level("WARN");
        store.push("INFO", "test", "should-be-filtered");
        let (t3, _) = store.query(None, None, 100, 0);
        assert!(t3 == 0 || !store.recent(1)[0].message.contains("should-be-filtered"));
        // 清除
        let cleared = store.clear();
        assert!(cleared > 0);
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_route_toggle() {
        let route = get_route("kg-stats").expect("route exists");
        assert!(route.enabled.load(Ordering::Relaxed));
        route.enabled.store(false, Ordering::Relaxed);
        // 停用后 match 仍命中（拦截在中间件层判定 enabled）
        let m = match_best("GET", "/kg/v1/stats").expect("matched");
        assert_eq!(m.id, "kg-stats");
        assert!(!m.enabled.load(Ordering::Relaxed));
        route.enabled.store(true, Ordering::Relaxed);
        assert!(route.enabled.load(Ordering::Relaxed));
    }

    #[test]
    fn test_management_path_exempt() {
        assert!(is_management("/actuator"));
        assert!(is_management("/actuator/mappings"));
        assert!(is_management("/health"));
        assert!(is_management("/metrics"));
        assert!(!is_management("/kg/v1/stats"));
        assert!(!is_management("/api/system/user"));
    }
}
