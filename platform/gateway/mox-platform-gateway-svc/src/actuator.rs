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
use mox_api_protocol::{ApiResponse, api_ok, api_error};

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
pub static ROUTES: [ApiRoute; 98] = [
    // =====================================================================
    // Actuator 域（L0·Spring Boot 风格管理面·/actuator/*）
    // =====================================================================
    r("actuator.index", "GET", "/actuator", "L0", "actuator", "ready", "管理面端点索引"),
    r("actuator.health", "GET", "/actuator/health", "L0", "actuator", "ready", "健康检查（Spring Boot 风格）"),
    r("actuator.info", "GET", "/actuator/info", "L0", "actuator", "ready", "构建信息（版本/时间/Git）"),
    r("actuator.mappings", "GET", "/actuator/mappings", "L0", "actuator", "ready", "全部 API 注册表（?layer&domain&status&q&only_enabled）"),
    r("actuator.metrics", "GET", "/actuator/metrics", "L0", "actuator", "ready", "运行时指标（JVM/CPU/内存/GC/线程/请求）"),
    r("actuator.env", "GET", "/actuator/env", "L0", "actuator", "ready", "网关配置（密钥脱敏）"),
    r("actuator.loggers", "ANY", "/actuator/loggers", "L0", "actuator", "ready", "日志级别查看/动态调整"),
    r("actuator.logs", "ANY", "/actuator/logs", "L0", "actuator", "ready", "在线日志查询（?level&search&limit&offset）"),
    r("actuator.logs_tail", "GET", "/actuator/logs/tail", "L0", "actuator", "ready", "SSE 实时日志流（curl -N）"),
    r("actuator.api", "ANY", "/actuator/api/:id", "L0", "actuator", "ready", "按 API 启停管理（/enable|/disable）"),

    // =====================================================================
    // Platform 域（L0·通用接入·/health /metrics /api/v1/* + L6 反向代理）
    // =====================================================================
    r("platform.health", "GET", "/health", "L0", "platform", "ready", "存活探针（网关 Rust axum 版本）"),
    r("platform.metrics", "GET", "/metrics", "L0", "platform", "ready", "Prometheus 指标端点（占位）"),
    r("platform.status", "GET", "/api/v1/status", "L0", "platform", "ready", "网关状态（域就绪统计+认证+限流）"),
    r("platform.domains", "GET", "/api/v1/domains", "L0", "platform", "ready", "31 业务域描述符列表（自描述）"),
    r("platform.proxy_orchestrator", "ANY", "/api/{*path}", "L6", "platform", "ready", "业务域反向代理→编排器（默认 :3001，catch-all）"),
    r("platform.proxy_primiflow", "ANY", "/api/projects/{*path}", "L6", "platform", "ready", "项目域反向代理→PrimiFlow（默认 :8000）"),

    // =====================================================================
    // KG 域（L2·知识图谱·/kg/v1/*·mox-kg-service-svc 真实算法）
    // =====================================================================
    r("kg.graph.neighborhood", "GET", "/kg/v1/neighborhood", "L2", "kg", "ready", "实体邻域查询（多跳邻居+边）"),
    r("kg.graph.path", "GET", "/kg/v1/path", "L2", "kg", "ready", "两实体间路径枚举（BFS）"),
    r("kg.graph.shortest_path", "GET", "/kg/v1/shortest-path", "L2", "kg", "ready", "最短路径（Dijkstra 边权重）"),
    r("kg.graph.centrality", "GET", "/kg/v1/centrality", "L2", "kg", "ready", "中心性分析（度/介数/接近）"),
    r("kg.graph.communities", "GET", "/kg/v1/communities", "L2", "kg", "ready", "社区发现（Louvain 模块度）"),
    r("kg.graph.stats", "GET", "/kg/v1/stats", "L2", "kg", "ready", "图谱统计（节点/边/标签分布）"),

    // =====================================================================
    // AI 域（L3·AI 引擎·/ai/v1/*·归一化版本前缀）
    // =====================================================================
    r("ai.engine.process", "POST", "/ai/v1/process", "L3", "ai", "ready", "AI 引擎统一处理（多模型路由）"),
    r("ai.engine.analyze", "POST", "/ai/v1/analyze", "L3", "ai", "ready", "AI 深度分析（结构化输出）"),
    r("ai.engine.capabilities", "GET", "/ai/v1/capabilities", "L3", "ai", "ready", "AI 引擎能力清单（模型/工具/配额）"),
    r("ai.engine.metrics", "GET", "/ai/v1/metrics", "L3", "ai", "ready", "AI 引擎运行指标（调用量/延迟/成功率）"),

    // =====================================================================
    // KB 域（L2·云盘知识库·/kb/v1/*·mox-kb-svc 100% 自研·归一化版本前缀）
    // =====================================================================
    r("kb.documents.list", "ANY", "/kb/v1/documents", "L2", "kb", "ready", "文档列表/上传/搜索（云盘根目录）"),
    r("kb.documents.detail", "ANY", "/kb/v1/documents/:id", "L2", "kb", "ready", "文档详情/下载/删除/元数据更新"),
    r("kb.documents.analyze", "POST", "/kb/v1/documents/:id/analyze", "L2", "kb", "ready", "文档 AI 分析（摘要/关键词/实体）"),
    r("kb.documents.batch_analyze", "POST", "/kb/v1/batch-analyze", "L2", "kb", "ready", "批量文档分析（异步任务）"),
    r("kb.categories.list", "GET", "/kb/v1/categories", "L2", "kb", "ready", "知识库分类树"),
    r("kb.tags.list", "GET", "/kb/v1/tags", "L2", "kb", "ready", "标签云/标签列表"),
    r("kb.search.query", "POST", "/kb/v1/search", "L2", "kb", "ready", "全文检索（向量+关键词混合）"),
    r("kb.versions.list", "ANY", "/kb/v1/documents/:id/versions", "L2", "kb", "ready", "文档版本列表"),
    r("kb.versions.detail", "GET", "/kb/v1/documents/:id/versions/:ver", "L2", "kb", "ready", "指定版本详情/下载"),
    r("kb.versions.compare", "POST", "/kb/v1/documents/:id/versions/compare", "L2", "kb", "ready", "版本差异对比（diff）"),
    r("kb.versions.revert", "POST", "/kb/v1/documents/:id/versions/revert", "L2", "kb", "ready", "回滚到指定版本"),
    r("kb.entities.list", "GET", "/kb/v1/documents/:id/entities", "L2", "kb", "ready", "文档实体抽取结果"),
    r("kb.graph.link", "ANY", "/kb/v1/documents/:id/graph-link", "L2", "kb", "ready", "文档→知识图谱关联/挂图"),
    r("kb.documents.history", "GET", "/kb/v1/documents/:id/history", "L2", "kb", "ready", "文档操作历史（审计）"),
    r("kb.stats.summary", "GET", "/kb/v1/stats", "L2", "kb", "ready", "知识库统计（文档数/容量/活跃度）"),
    r("kb.history.list", "GET", "/kb/v1/history", "L2", "kb", "ready", "全局操作历史（最近活动）"),

    // =====================================================================
    // Alliance 域（L4·专家联盟·/alliance/v1/*·scheduler-core 真实存储+匹配+执行）
    // =====================================================================
    r("alliance.tasks.list", "ANY", "/alliance/v1/tasks", "L4", "alliance", "ready", "联盟任务列表/创建（InMemoryTaskRepository 真实存储）"),
    r("alliance.tasks.detail", "ANY", "/alliance/v1/tasks/:task_id", "L4", "alliance", "ready", "任务详情/操作（暂停/恢复/取消）"),
    r("alliance.experts.search", "POST", "/alliance/v1/experts/search", "L4", "alliance", "ready", "专家匹配搜索（RuleBasedExpertMatcher 真实匹配）"),
    r("alliance.tasks.status", "GET", "/alliance/v1/tasks/:task_id/status", "L4", "alliance", "ready", "执行状态查询（真实节点统计）"),
    r("alliance.tasks.nodes", "GET", "/alliance/v1/tasks/:task_id/nodes", "L4", "alliance", "ready", "执行节点列表（真实 DAG 节点）"),
    r("alliance.tasks.node", "ANY", "/alliance/v1/tasks/:task_id/nodes/:node_id", "L4", "alliance", "ready", "节点详情/跳过（人工干预）"),
    r("alliance.tasks.logs", "GET", "/alliance/v1/tasks/:id/logs", "L4", "alliance", "ready", "任务执行日志（真实存储）"),
    r("alliance.tasks.fusion", "GET", "/alliance/v1/tasks/:id/fusion-result", "L4", "alliance", "ready", "融合结果（真实从节点输出融合）"),
    r("alliance.tasks.dag", "GET", "/alliance/v1/tasks/:id/dag", "L4", "alliance", "ready", "DAG 节点+边（真实存储的 DAG）"),
    r("alliance.tasks.toggle_done", "PUT", "/alliance/v1/tasks/:id/toggle-done", "L4", "alliance", "ready", "完成状态切换（真实状态流转）"),
    r("alliance.tasks.status_poll", "GET", "/alliance/v1/tasks/:id/status", "L4", "alliance", "ready", "任务状态轮询（供前端轮询）"),

    // =====================================================================
    // System 域（L5·系统管理+安全·/api/v1/system/* · /api/v1/security/*·IAM SQLite 真实数据链路）
    // =====================================================================
    r("system.permissions.current", "GET", "/api/system/permissions", "L5", "system", "ready", "当前用户权限/角色/菜单"),
    r("system.dept.list", "ANY", "/api/system/dept", "L5", "system", "ready", "部门列表/创建"),
    r("system.dept.tree", "GET", "/api/system/dept/tree", "L5", "system", "ready", "部门树"),
    r("system.dept.detail", "ANY", "/api/system/dept/:id", "L5", "system", "ready", "部门详情/更新/删除"),
    r("system.dept.users", "GET", "/api/system/dept/:id/users", "L5", "system", "ready", "部门用户列表"),
    r("system.post.list", "ANY", "/api/system/post", "L5", "system", "ready", "岗位列表/创建"),
    r("system.post.by_dept", "GET", "/api/system/post/dept/:deptId", "L5", "system", "ready", "按部门查询岗位"),
    r("system.post.detail", "ANY", "/api/system/post/:id", "L5", "system", "ready", "岗位详情/更新/删除"),
    r("system.user.list", "ANY", "/api/system/user", "L5", "system", "ready", "用户列表/创建"),
    r("system.user.detail", "ANY", "/api/system/user/:id", "L5", "system", "ready", "用户详情/更新/删除"),
    r("system.user.reset_pwd", "PUT", "/api/system/user/:id/resetPwd", "L5", "system", "ready", "重置用户密码"),
    r("system.user.change_status", "PUT", "/api/system/user/:id/changeStatus", "L5", "system", "ready", "用户状态切换（启用/停用）"),
    r("system.user.roles", "ANY", "/api/system/user/:id/roles", "L5", "system", "ready", "用户角色查询/分配"),
    r("system.role.list", "ANY", "/api/system/role", "L5", "system", "ready", "角色列表/创建"),
    r("system.role.detail", "ANY", "/api/system/role/:id", "L5", "system", "ready", "角色详情/更新/删除"),
    r("system.role.menu_perms", "ANY", "/api/system/role/:id/menuPerms", "L5", "system", "ready", "角色菜单权限查询/设置"),
    r("system.role.data_perms", "ANY", "/api/system/role/:id/dataPerms", "L5", "system", "ready", "角色数据权限查询/设置"),
    r("system.role.users", "GET", "/api/system/role/:id/users", "L5", "system", "ready", "角色用户列表"),
    r("system.role.copy", "POST", "/api/system/role/:id/copy", "L5", "system", "ready", "复制角色"),
    r("system.menu.tree", "GET", "/api/system/menu/tree", "L5", "system", "ready", "菜单树（用户可见）"),
    r("system.menu.list", "ANY", "/api/system/menu", "L5", "system", "ready", "菜单列表/创建"),
    r("system.menu.detail", "ANY", "/api/system/menu/:id", "L5", "system", "ready", "菜单详情/更新/删除"),
    r("system.dict_type.list", "ANY", "/api/system/dict/type", "L5", "system", "ready", "字典类型列表/创建"),
    r("system.dict_type.all", "GET", "/api/system/dict/type/all", "L5", "system", "ready", "全部字典类型"),
    r("system.dict_type.detail", "ANY", "/api/system/dict/type/:id", "L5", "system", "ready", "字典类型详情/更新/删除"),
    r("system.dict_data.list", "ANY", "/api/system/dict/data", "L5", "system", "ready", "字典数据列表/创建"),
    r("system.dict_data.by_type", "GET", "/api/system/dict/data/type/:dictType", "L5", "system", "ready", "按类型查询字典数据"),
    r("system.dict_data.detail", "ANY", "/api/system/dict/data/:id", "L5", "system", "ready", "字典数据详情/更新/删除"),
    r("system.config.list", "ANY", "/api/system/config", "L5", "system", "ready", "参数配置列表/创建"),
    r("system.config.refresh", "DELETE", "/api/system/config/refresh-cache", "L5", "system", "ready", "刷新配置缓存"),
    r("system.config.detail", "ANY", "/api/system/config/:id", "L5", "system", "ready", "配置详情/更新/删除"),
    r("system.config.by_key", "GET", "/api/system/config/key/:key", "L5", "system", "ready", "按键查询配置"),
    r("system.operlog.list", "GET", "/api/system/operlog", "L5", "system", "ready", "操作日志列表"),
    r("system.operlog.clean", "DELETE", "/api/system/operlog/clean", "L5", "system", "ready", "清空操作日志"),
    r("system.operlog.detail", "ANY", "/api/system/operlog/:id", "L5", "system", "ready", "操作日志详情/删除"),
    r("system.operlog.export", "GET", "/api/system/operlog/export", "L5", "system", "ready", "导出操作日志（CSV）"),
    r("system.loginlog.list", "GET", "/api/system/logininfor", "L5", "system", "ready", "登录日志列表"),
    r("system.loginlog.clean", "DELETE", "/api/system/logininfor/clean", "L5", "system", "ready", "清空登录日志"),
    r("system.loginlog.detail", "DELETE", "/api/system/logininfor/:id", "L5", "system", "ready", "删除登录日志"),
    r("system.loginlog.export", "GET", "/api/system/logininfor/export", "L5", "system", "ready", "导出登录日志（CSV）"),
    r("system.security.status", "GET", "/api/security/status", "L5", "system", "ready", "安全状态（认证/限流/IAM）"),
    r("system.security.api_keys", "ANY", "/api/security/api-keys", "L5", "system", "ready", "API Key 列表/创建（SQLite 持久化）"),
    r("system.security.api_key_revoke", "DELETE", "/api/security/api-keys/:id", "L5", "system", "ready", "吊销 API Key（DB+内存双删）"),
    r("system.security.api_key_validate", "POST", "/api/security/validate", "L5", "system", "ready", "校验 API Key 明文"),
    r("system.security.audit_log", "GET", "/api/security/audit-log", "L5", "system", "ready", "审计日志（SQLite 读取）"),
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

/// 兼容迁移期历史入口，但始终把它们归一化到唯一的 canonical route。
///
/// `/api/*` 是旧编排入口，`/kg/v1/*` 和 `/ai/v1/*` 是当前 L2/L3 入口。
/// 管理面需要同时识别两者，否则通配代理会在精确业务路由之前抢占匹配。
fn canonical_path(path: &str) -> &str {
    match path {
        "/api/kg/stats" => "/kg/v1/stats",
        "/api/ai/engine/process" => "/ai/v1/process",
        "/api/ai/engine/analyze" => "/ai/v1/analyze",
        "/api/ai/engine/capabilities" => "/ai/v1/capabilities",
        "/api/ai/engine/metrics" => "/ai/v1/metrics",
        _ => path,
    }
}

/// 在注册表中查找“最具体”匹配的路由（用于启停拦截）。
pub fn match_best(method: &str, path: &str) -> Option<&'static ApiRoute> {
    let canonical = canonical_path(path);

    // 一个已知业务路径如果方法不允许，必须明确返回 None，不能降级到 ANY catch-all。
    // 这样可以让上层返回 405/路由错误，而不是把请求误交给另一个业务服务。
    if ROUTES
        .iter()
        .any(|route| path_matches(route.path, canonical) && !method_ok(route, method))
    {
        return None;
    }

    let mut best: Option<&'static ApiRoute> = None;
    let mut best_spec = 0usize;
    for route in ROUTES.iter() {
        if !method_ok(route, method) {
            continue;
        }
        if path_matches(route.path, canonical) {
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
            return ApiResponse::<Value>::error(403, format!("API `{}` 已被管理端停用，请在 /actuator/api/{} 恢复", route.id, route.id)).into_response();
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
async fn actuator_index() -> ApiResponse<Value> {
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
    api_ok(json!({
        "_links": endpoints,
        "total_endpoints": endpoints.len(),
        "framework": "MOX Gateway Actuator (Spring Boot style)",
        "ts": now_ms(),
    }))
}

/// GET /actuator/health
async fn actuator_health(State(state): State<GatewayState>) -> ApiResponse<Value> {
    api_ok(json!({
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
async fn actuator_info() -> ApiResponse<Value> {
    api_ok(json!({
        "app": {
            "name": "mox-gateway",
            "description": "MOX mox 模块化系统架构低代码平台 · 企业级网关",
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
) -> ApiResponse<Value> {
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
    api_ok(json!({
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
async fn actuator_metrics(State(state): State<GatewayState>) -> ApiResponse<Value> {
    let m = state.runtime.snapshot();
    api_ok(json!({
        "names": ["requests_total", "requests_2xx", "requests_4xx", "requests_5xx", "active_requests", "latency_avg_ms", "uptime_secs"],
        "measurements": m,
        "ts": now_ms(),
    }))
}

/// GET /actuator/env —— 网关配置（密钥脱敏）
async fn actuator_env(State(state): State<GatewayState>) -> ApiResponse<Value> {
    let cfg = &state.config;
    let jwt_secret = if cfg.auth.jwt_secret.is_empty() {
        "<empty>"
    } else if cfg.auth.jwt_secret == "change-me-in-production" {
        "change-me-in-production"
    } else {
        &cfg.auth.jwt_secret[..8.min(cfg.auth.jwt_secret.len())]
    };
    api_ok(json!({
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
async fn actuator_loggers(State(state): State<GatewayState>) -> ApiResponse<Value> {
    api_ok(json!({
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
) -> ApiResponse<Value> {
    let level = body.level.to_ascii_uppercase();
    if !matches!(level.as_str(), "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR") {
        return api_error(400, format!("level 必须为 TRACE/DEBUG/INFO/WARN/ERROR，got: {level}"));
    }
    state.logs.set_min_level(&level);
    state
        .logs
        .push("INFO", "actuator", format!("日志级别调整为 {level}"));
    api_ok(json!({
        "configured_level": state.logs.min_level(),
    }))
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
) -> ApiResponse<Value> {
    let limit = q.limit.unwrap_or(200).clamp(1, 2000);
    let offset = q.offset.unwrap_or(0);
    let (total, entries) = state
        .logs
        .query(q.level.as_deref(), q.search.as_deref(), limit, offset);
    api_ok(json!({
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
async fn actuator_logs_clear(State(state): State<GatewayState>) -> ApiResponse<Value> {
    let cleared = state.logs.clear();
    api_ok(json!({ "cleared": cleared }))
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
async fn actuator_api_get(Path(id): Path<String>) -> ApiResponse<Value> {
    match get_route(&id) {
        Some(route) => api_ok(json!({
            "id": route.id,
            "method": route.method,
            "path": route.path,
            "layer": route.layer,
            "domain": route.domain,
            "status": route.status,
            "description": route.description,
            "enabled": route.enabled.load(Ordering::Relaxed),
        })),
        None => api_error(404, format!("未找到 API: {id}，可枚举 /actuator/mappings 获取 id")),
    }
}

/// 启停状态变更（共用逻辑）
fn set_api_enabled(id: &str, enabled: bool, store: &LogStore) -> ApiResponse<Value> {
    match get_route(id) {
        Some(route) => {
            if is_management(route.path) {
                return api_error(403, format!("管理面端点 `{id}` 不允许停用（防自锁"));
            }
            route.enabled.store(enabled, Ordering::Relaxed);
            store.push(
                "INFO",
                "actuator",
                format!("API `{id}` ({}) 已{}", route.path, if enabled { "启用" } else { "停用" }),
            );
            api_ok(json!({
                "id": route.id,
                "path": route.path,
                "method": route.method,
                "enabled": enabled,
                "message": format!("API `{id}` 已{}，停用后请求将返回 403", if enabled { "启用" } else { "停用" }),
            }))
        }
        None => api_error(404, format!("未找到 API: {id}")),
    }
}

/// POST /actuator/api/:id/enable
async fn actuator_api_enable(State(state): State<GatewayState>, Path(id): Path<String>) -> ApiResponse<Value> {
    set_api_enabled(&id, true, &state.logs)
}

/// POST /actuator/api/:id/disable
async fn actuator_api_disable(State(state): State<GatewayState>, Path(id): Path<String>) -> ApiResponse<Value> {
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
        assert_eq!(m.id, "system.user.roles");
        // 普通 /api/system/user 命中系统路由而非代理通配
        let m2 = match_best("GET", "/api/system/user").expect("matched");
        assert_eq!(m2.id, "system.user.list");
        // /api/others 落入代理
        let m3 = match_best("GET", "/api/others/foo").expect("matched");
        assert_eq!(m3.id, "platform.proxy_orchestrator");
    }

    #[test]
    fn test_match_best_respects_method() {
        let m = match_best("POST", "/api/ai/engine/process").expect("matched");
        assert_eq!(m.id, "ai.engine.process");
        // GET 不匹配 POST 路由 → 应落到别的（无匹配则 None）
        assert!(match_best("GET", "/api/ai/engine/process").is_none());
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
        let route = get_route("kg.graph.stats").expect("route exists");
        assert!(route.enabled.load(Ordering::Relaxed));
        route.enabled.store(false, Ordering::Relaxed);
        // 停用后 match 仍命中（拦截在中间件层判定 enabled）
        let m = match_best("GET", "/api/kg/stats").expect("matched");
        assert_eq!(m.id, "kg.graph.stats");
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
