// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! MOX Enterprise · 全功能 API 处理层
//!
//! 对接前端所有 /api/* 端点，内存存储 + 模拟响应，确保零 404。

pub mod handlers;
pub mod graph_algo;
pub mod kg_persist;

use axum::{
    body::Body,
    extract::{Path, Query, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Router,
};
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;

/// 全局共享状态（内存存储）
#[derive(Clone)]
pub struct AppState {
    pub projects: Arc<DashMap<String, Value>>,
    pub tasks: Arc<DashMap<String, Value>>,
    pub sessions: Arc<DashMap<String, Value>>,
    pub experts: Arc<DashMap<String, Value>>,
    pub llm_providers: Arc<DashMap<String, Value>>,
    pub kb_docs: Arc<DashMap<String, Value>>,
    pub market_items: Arc<DashMap<String, Value>>,
    pub flows: Arc<DashMap<String, Value>>,
    pub workflows: Arc<DashMap<String, Value>>,
    pub artifacts: Arc<DashMap<String, Value>>,
    pub api_keys: Arc<DashMap<String, Value>>,
    pub audit_logs: Arc<DashMap<String, Value>>,
    pub graph_nodes: Arc<DashMap<String, Value>>,
    pub graph_edges: Arc<DashMap<String, Value>>,
    /// M5.3：KG 图谱 JSON 快照持久化路径（None = 不持久化，仅内存）
    pub kg_file: Option<String>,
    pub browser_sessions: Arc<DashMap<String, Value>>,
    pub automation_runs: Arc<DashMap<String, Value>>,
    pub plugins: Arc<DashMap<String, Value>>,
    pub chat_history: Arc<DashMap<String, Value>>,
}

impl Default for AppState {
    fn default() -> Self {
        let state = Self {
            projects: Arc::new(DashMap::new()),
            tasks: Arc::new(DashMap::new()),
            sessions: Arc::new(DashMap::new()),
            experts: Arc::new(DashMap::new()),
            llm_providers: Arc::new(DashMap::new()),
            kb_docs: Arc::new(DashMap::new()),
            market_items: Arc::new(DashMap::new()),
            flows: Arc::new(DashMap::new()),
            workflows: Arc::new(DashMap::new()),
            artifacts: Arc::new(DashMap::new()),
            api_keys: Arc::new(DashMap::new()),
            audit_logs: Arc::new(DashMap::new()),
            graph_nodes: Arc::new(DashMap::new()),
            graph_edges: Arc::new(DashMap::new()),
            kg_file: std::env::var("MOX_KG_FILE").ok(),
            browser_sessions: Arc::new(DashMap::new()),
            automation_runs: Arc::new(DashMap::new()),
            plugins: Arc::new(DashMap::new()),
            chat_history: Arc::new(DashMap::new()),
        };
        state.seed_demo_data();
        state.load_kg_from_env();
        state
    }
}

impl AppState {
    /// 注入演示数据，确保前端首屏有内容
    fn seed_demo_data(&self) {
        // 演示项目
        let demo_projects = vec![
            serde_json::json!({
                "id": "proj-demo-001", "name": "璇玑全维数字孪生中台",
                "type": "platform", "status": "active",
                "description": "核心平台建设项目", "created_at": "2026-08-01T00:00:00Z"
            }),
            serde_json::json!({
                "id": "proj-demo-002", "name": "政务信创门户改造",
                "type": "government", "status": "active",
                "description": "清远市检察院信创改造", "created_at": "2026-08-10T00:00:00Z"
            }),
        ];
        for p in demo_projects {
            if let Some(id) = p.get("id").and_then(|v| v.as_str()) {
                self.projects.insert(id.to_string(), p);
            }
        }

        // 演示专家
        let demo_experts = vec![
            serde_json::json!({
                "id": "exp-arch", "name": "架构专家", "role": "architect",
                "capabilities": ["系统设计", "技术选型", "架构评审"], "status": "online"
            }),
            serde_json::json!({
                "id": "exp-dev", "name": "开发专家", "role": "developer",
                "capabilities": ["全栈开发", "代码审查", "性能优化"], "status": "online"
            }),
            serde_json::json!({
                "id": "exp-algo", "name": "算法专家", "role": "algorithm",
                "capabilities": ["算法分析", "数学建模", "拓扑优化"], "status": "online"
            }),
        ];
        for e in demo_experts {
            if let Some(id) = e.get("id").and_then(|v| v.as_str()) {
                self.experts.insert(id.to_string(), e);
            }
        }

        // 演示LLM提供商
        let demo_llms = vec![
            serde_json::json!({
                "id": "llm-doubao", "name": "豆包", "provider": "doubao",
                "base_url": "https://ark.cn-beijing.volces.com/api/v3",
                "models": ["doubao-pro-32k", "doubao-lite-128k"], "status": "active", "is_default": true
            }),
            serde_json::json!({
                "id": "llm-openai", "name": "OpenAI", "provider": "openai",
                "base_url": "https://api.openai.com/v1",
                "models": ["gpt-4o", "gpt-4o-mini"], "status": "configured"
            }),
        ];
        for l in demo_llms {
            if let Some(id) = l.get("id").and_then(|v| v.as_str()) {
                self.llm_providers.insert(id.to_string(), l);
            }
        }

        // 全维知识图谱：注入璇玑平台核心节点与关系，确保图谱页/搜索/路径分析首屏可用
        let demo_graph_nodes = vec![
            serde_json::json!({"id":"mox-core","label":"璇玑内核","node_type":"core","category":"平台底座","summary":"全维统一内核与运行时"}),
            serde_json::json!({"id":"dsql-engine","label":"DSQL引擎","node_type":"graph","category":"数据层","summary":"动态SQL低代码查询引擎"}),
            serde_json::json!({"id":"kg-engine","label":"知识图谱引擎","node_type":"graph","category":"数据层","summary":"自研图谱存储与图算法"}),
            serde_json::json!({"id":"kb-store","label":"云盘知识库","node_type":"data","category":"数据层","summary":"文档/知识沉淀与检索"}),
            serde_json::json!({"id":"ds-core","label":"数据源中心","node_type":"data","category":"数据层","summary":"多数据源适配与治理"}),
            serde_json::json!({"id":"llm-gateway","label":"大模型网关","node_type":"ai","category":"智能层","summary":"多Provider路由与统一接入"}),
            serde_json::json!({"id":"expert-alliance","label":"专家联盟","node_type":"ai","category":"智能层","summary":"多专家协同与编排"}),
            serde_json::json!({"id":"op-engine","label":"算子引擎","node_type":"graph","category":"智能层","summary":"算子统一系统(OUS)执行"}),
            serde_json::json!({"id":"flow-engine","label":"流程引擎","node_type":"core","category":"运行层","summary":"工作流编排与实例调度"}),
            serde_json::json!({"id":"mcp-gateway","label":"MCP网关","node_type":"core","category":"运行层","summary":"Model Context Protocol 兼容层"}),
            serde_json::json!({"id":"mox-fusion","label":"全维融合","node_type":"activation","category":"运行层","summary":"多源数据融合与编排"}),
            serde_json::json!({"id":"inf-optimizer","label":"无穷维度优化","node_type":"optimizer","category":"智能层","summary":"多维参数寻优引擎"}),
            serde_json::json!({"id":"algo-lab","label":"算法实验室","node_type":"math","category":"科研层","summary":"算法分析/仿真/归一化"}),
            serde_json::json!({"id":"monitor","label":"实时监控","node_type":"signal","category":"观测层","summary":"运行观测与AI诊断"}),
        ];
        for n in &demo_graph_nodes {
            if let Some(id) = n.get("id").and_then(|v| v.as_str()) {
                self.graph_nodes.insert(id.to_string(), n.clone());
            }
        }
        let demo_graph_edges = vec![
            serde_json::json!({"id":"e1","source":"mox-fusion","target":"kg-engine","relation":"integrate","weight":0.9}),
            serde_json::json!({"id":"e2","source":"mox-fusion","target":"dsql-engine","relation":"integrate","weight":0.8}),
            serde_json::json!({"id":"e3","source":"kg-engine","target":"kb-store","relation":"read","weight":0.7}),
            serde_json::json!({"id":"e4","source":"kg-engine","target":"dsql-engine","relation":"query","weight":0.6}),
            serde_json::json!({"id":"e5","source":"dsql-engine","target":"ds-core","relation":"use","weight":0.8}),
            serde_json::json!({"id":"e6","source":"op-engine","target":"mox-fusion","relation":"execute","weight":0.7}),
            serde_json::json!({"id":"e7","source":"flow-engine","target":"op-engine","relation":"schedule","weight":0.8}),
            serde_json::json!({"id":"e8","source":"flow-engine","target":"mcp-gateway","relation":"call","weight":0.6}),
            serde_json::json!({"id":"e9","source":"llm-gateway","target":"expert-alliance","relation":"route","weight":0.7}),
            serde_json::json!({"id":"e10","source":"expert-alliance","target":"kg-engine","relation":"reason","weight":0.6}),
            serde_json::json!({"id":"e11","source":"inf-optimizer","target":"flow-engine","relation":"optimize","weight":0.5}),
            serde_json::json!({"id":"e12","source":"inf-optimizer","target":"llm-gateway","relation":"tune","weight":0.5}),
            serde_json::json!({"id":"e13","source":"mox-core","target":"dsql-engine","relation":"host","weight":0.9}),
            serde_json::json!({"id":"e14","source":"mox-core","target":"flow-engine","relation":"host","weight":0.9}),
            serde_json::json!({"id":"e15","source":"algo-lab","target":"inf-optimizer","relation":"feed","weight":0.5}),
            serde_json::json!({"id":"e16","source":"monitor","target":"mox-core","relation":"observe","weight":0.4}),
        ];
        for e in &demo_graph_edges {
            if let Some(id) = e.get("id").and_then(|v| v.as_str()) {
                self.graph_edges.insert(id.to_string(), e.clone());
            }
        }
    }

    /// M5.3：KG 图谱持久化装配（重启自动恢复，无需重灌）。
    /// 优先级：MOX_KG_FILE 快照 > MOX_KG_SEED seed（并落快照）> 演示数据。
    fn load_kg_from_env(&self) {
        let kg_file = std::env::var("MOX_KG_FILE").ok();
        let kg_seed = std::env::var("MOX_KG_SEED").ok();
        if let Some(f) = &kg_file {
            if let Some((nodes, edges)) = kg_persist::load_snapshot(f) {
                self.graph_nodes.clear();
                for n in nodes {
                    if let Some(id) = n.get("id").and_then(|v| v.as_str()) {
                        self.graph_nodes.insert(id.to_string(), n);
                    }
                }
                self.graph_edges.clear();
                for e in edges {
                    if let Some(id) = e.get("id").and_then(|v| v.as_str()) {
                        self.graph_edges.insert(id.to_string(), e);
                    }
                }
                println!("[KG] 从快照恢复: {} 节点 / {} 边 ({})", self.graph_nodes.len(), self.graph_edges.len(), f);
                return;
            } else {
                println!("[KG] 快照不存在或解析失败，尝试 seed 冷启动: {}", f);
            }
        }
        if let Some(seed) = &kg_seed {
            match kg_persist::load_seed(seed, &self.graph_nodes, &self.graph_edges) {
                Ok(n) => {
                    println!("[KG] 从 seed 冷启动: 新增 {} 节点, 当前 {} 节点 / {} 边 ({})",
                        n, self.graph_nodes.len(), self.graph_edges.len(), seed);
                    if let Some(f) = &kg_file {
                        let _ = kg_persist::save_snapshot(f, &self.graph_nodes, &self.graph_edges);
                        println!("[KG] 已落快照: {}", f);
                    }
                }
                Err(e) => println!("[KG] seed 加载失败，使用演示数据: {}", e),
            }
        } else {
            println!("[KG] 未配置 MOX_KG_FILE/MOX_KG_SEED，使用演示数据");
        }
    }
}

/// 统一成功响应
pub fn ok<T: serde::Serialize>(data: T) -> Response<Body> {
    let body = serde_json::json!({ "success": true, "data": data });
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

/// 统一成功响应（带额外字段）
pub fn ok_with_extra<T: serde::Serialize>(data: T, extra: Value) -> Response<Body> {
    let mut body = serde_json::json!({ "success": true, "data": data });
    if let Value::Object(map) = &mut body {
        if let Value::Object(extra_map) = extra {
            for (k, v) in extra_map {
                map.insert(k, v);
            }
        }
    }
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

/// 直接返回 JSON（不包 success 信封，用于兼容前端直接取数据的场景）
pub fn json_raw<T: serde::Serialize>(data: T) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&data).unwrap()))
        .unwrap()
}

/// 错误响应
pub fn err(status: StatusCode, code: &str, message: &str) -> Response<Body> {
    let body = serde_json::json!({ "success": false, "code": code, "error": message });
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

/// 生成 UUID
pub fn new_id(prefix: &str) -> String {
    format!("{}-{}", prefix, uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"))
}

/// 专家联盟反代：转发到 mox-ai-expert-svc (:3002)，并把裸响应包装为 legacy 同款信封
const EXPERT_SVC_BASE: &str = "http://127.0.0.1:3002";

async fn proxy_forward(method: &str, path: &str, body: Option<String>) -> Response<Body> {
    let url = format!("{}{}", EXPERT_SVC_BASE, path);
    let client = reqwest::Client::new();
    let m = reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET);
    let mut rb = client.request(m, &url).timeout(std::time::Duration::from_secs(60));
    if let Some(b) = body {
        rb = rb.header("Content-Type", "application/json").body(b);
    }
    match rb.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            let wrapped = match serde_json::from_str::<Value>(&text) {
                Ok(v) => {
                    let data = if path == "/api/alliance/experts" {
                        v.get("experts").cloned().unwrap_or(v)
                    } else {
                        v
                    };
                    serde_json::json!({ "success": true, "data": data }).to_string()
                }
                Err(_) => text,
            };
            Response::builder()
                .status(status)
                .header("Content-Type", "application/json")
                .body(Body::from(wrapped))
                .unwrap()
        }
        Err(e) => err(StatusCode::BAD_GATEWAY, "proxy_error", &format!("专家服务转发失败: {}", e)),
    }
}

async fn proxy_experts_list(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("GET", "/api/alliance/experts", None).await
}
async fn proxy_experts_get(State(_st): State<AppState>, Path(id): Path<String>) -> Response<Body> {
    proxy_forward("GET", &format!("/api/alliance/experts/{}", id), None).await
}
async fn proxy_experts_overview(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("GET", "/api/alliance/overview", None).await
}
async fn proxy_experts_metrics(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("GET", "/api/alliance/metrics", None).await
}
async fn proxy_experts_multi_consult(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/multi-consult", Some(body)).await
}
async fn proxy_experts_debate(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/debate", Some(body)).await
}
async fn proxy_experts_route(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/route", Some(body)).await
}
async fn proxy_experts_algorithm_analysis(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/algorithm-analysis", Some(body)).await
}
async fn proxy_experts_orchestrate(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/orchestrate", Some(body)).await
}
fn proxy_query_string(q: &std::collections::HashMap<String, String>) -> String {
    if q.is_empty() {
        return String::new();
    }
    let mut parts: Vec<String> = q
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();
    parts.sort();
    format!("?{}", parts.join("&"))
}

async fn proxy_experts_register(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/experts/register", Some(body)).await
}

async fn proxy_experts_update(State(_st): State<AppState>, Path(id): Path<String>, body: String) -> Response<Body> {
    proxy_forward("PUT", &format!("/api/alliance/experts/{}", id), Some(body)).await
}

async fn proxy_experts_remove(State(_st): State<AppState>, Path(id): Path<String>) -> Response<Body> {
    proxy_forward("DELETE", &format!("/api/alliance/experts/{}", id), None).await
}

async fn proxy_experts_consult(State(_st): State<AppState>, Path(id): Path<String>, body: String) -> Response<Body> {
    proxy_forward("POST", &format!("/api/alliance/experts/{}/consult", id), Some(body)).await
}

async fn proxy_experts_capabilities(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("GET", "/api/alliance/capabilities", None).await
}

async fn proxy_experts_intelligent_consult(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/intelligent-consult", Some(body)).await
}

async fn proxy_experts_single_metrics(State(_st): State<AppState>, Path(id): Path<String>) -> Response<Body> {
    proxy_forward("GET", &format!("/api/alliance/experts/{}/metrics", id), None).await
}

async fn proxy_experts_sessions_list(State(_st): State<AppState>, Query(q): Query<std::collections::HashMap<String, String>>) -> Response<Body> {
    let qs = proxy_query_string(&q);
    proxy_forward("GET", &format!("/api/alliance/sessions{}", qs), None).await
}

async fn proxy_experts_session_create(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/sessions", Some(body)).await
}

async fn proxy_experts_sessions_stats(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("GET", "/api/alliance/sessions/stats", None).await
}

async fn proxy_experts_session_get(State(_st): State<AppState>, Path(id): Path<String>) -> Response<Body> {
    proxy_forward("GET", &format!("/api/alliance/sessions/{}", id), None).await
}

async fn proxy_experts_session_update(State(_st): State<AppState>, Path(id): Path<String>, body: String) -> Response<Body> {
    proxy_forward("PUT", &format!("/api/alliance/sessions/{}", id), Some(body)).await
}

async fn proxy_experts_session_delete(State(_st): State<AppState>, Path(id): Path<String>) -> Response<Body> {
    proxy_forward("DELETE", &format!("/api/alliance/sessions/{}", id), None).await
}

async fn proxy_experts_session_append_message(State(_st): State<AppState>, Path(id): Path<String>, body: String) -> Response<Body> {
    proxy_forward("POST", &format!("/api/alliance/sessions/{}/messages", id), Some(body)).await
}

async fn proxy_experts_session_similar_search(State(_st): State<AppState>, Path(id): Path<String>, body: String) -> Response<Body> {
    proxy_forward("POST", &format!("/api/alliance/sessions/{}/similar-search", id), Some(body)).await
}

async fn proxy_experts_semantic_search(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/semantic-search", Some(body)).await
}

async fn proxy_experts_session_export(State(_st): State<AppState>, Path(id): Path<String>) -> Response<Body> {
    proxy_forward("GET", &format!("/api/alliance/sessions/{}/export", id), None).await
}

async fn proxy_experts_session_archive(State(_st): State<AppState>, Path(id): Path<String>) -> Response<Body> {
    proxy_forward("POST", &format!("/api/alliance/sessions/{}/archive", id), None).await
}

async fn proxy_dispatcher_config(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("GET", "/api/alliance/dispatcher/config", None).await
}

async fn proxy_dispatcher_config_update(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("PUT", "/api/alliance/dispatcher/config", Some(body)).await
}

async fn proxy_dispatcher_status(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("GET", "/api/alliance/dispatcher/status", None).await
}

async fn proxy_dispatcher_dispatch(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/dispatcher/dispatch", Some(body)).await
}

async fn proxy_dispatcher_consult(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/dispatcher/consult", Some(body)).await
}

async fn proxy_dispatcher_multi_consult(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/dispatcher/multi-consult", Some(body)).await
}

async fn proxy_dispatcher_reset_expert(State(_st): State<AppState>, Path(id): Path<String>) -> Response<Body> {
    proxy_forward("POST", &format!("/api/alliance/dispatcher/reset/{}", id), None).await
}

async fn proxy_dispatcher_reset_all(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/dispatcher/reset-all", None).await
}

async fn proxy_expert_graph_get(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("GET", "/api/alliance/graph", None).await
}

async fn proxy_expert_graph_stats(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("GET", "/api/alliance/graph/stats", None).await
}

async fn proxy_expert_graph_neighbors(State(_st): State<AppState>, Path(id): Path<String>) -> Response<Body> {
    proxy_forward("GET", &format!("/api/alliance/graph/neighbors/{}", id), None).await
}

async fn proxy_expert_graph_collaborators(State(_st): State<AppState>, Path(id): Path<String>, Query(q): Query<std::collections::HashMap<String, String>>) -> Response<Body> {
    let qs = proxy_query_string(&q);
    proxy_forward("GET", &format!("/api/alliance/graph/collaborators/{}{}", id, qs), None).await
}

async fn proxy_expert_graph_path(State(_st): State<AppState>, Path((source, target)): Path<(String, String)>) -> Response<Body> {
    proxy_forward("GET", &format!("/api/alliance/graph/path/{}/{}", source, target), None).await
}

async fn proxy_expert_graph_communities(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("GET", "/api/alliance/graph/communities", None).await
}

async fn proxy_expert_graph_optimal_team(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/graph/optimal-team", Some(body)).await
}

async fn proxy_expert_graph_rebuild(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/graph/rebuild", None).await
}

async fn proxy_experts_enterprise_consult(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/enterprise/consult", Some(body)).await
}

async fn proxy_experts_enterprise_analyze(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/enterprise/analyze", Some(body)).await
}

async fn proxy_experts_plan_generate(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/plan/generate", Some(body)).await
}

async fn proxy_experts_plan_execute(State(_st): State<AppState>, body: String) -> Response<Body> {
    proxy_forward("POST", "/api/alliance/plan/execute", Some(body)).await
}

async fn proxy_orchestration_stats(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("GET", "/api/alliance/orchestration/stats", None).await
}

async fn proxy_orchestration_plugins(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("GET", "/api/alliance/orchestration/plugins", None).await
}

async fn proxy_orchestration_history(State(_st): State<AppState>) -> Response<Body> {
    proxy_forward("GET", "/api/alliance/orchestration/history", None).await
}

/// 构建全功能 API 路由
pub fn api_router(state: AppState) -> Router {
    use handlers::*;

    Router::new()
        // ===== 系统 =====
        .route("/health", get(system_health))
        .route("/status", get(system_status))
        .route("/status/full", get(system_status_full))
        .route("/logs", get(system_logs))
        .route("/plugins", get(system_plugins))
        .route("/config", get(system_config))
        .route("/modules", get(system_modules))

        // ===== 算子 =====
        .route("/operators", get(operators_list))
        .route("/operators/register", post(operators_register))
        .route("/operators/ai-recommend", post(operators_ai_recommend))
        .route("/execute", post(execute_workflow))

        // ===== 知识图谱 =====
        .route("/graph", get(graph_get))
        .route("/graph/stats", get(graph_stats))
        .route("/graph/centrality", get(graph_centrality))
        .route("/graph/communities", get(graph_communities))
        .route("/graph/pagerank", get(graph_pagerank))
        .route("/graph/neighbors/:id", get(graph_neighbors))
        .route("/graph/path", get(graph_shortest_path))
        .route("/graph/recommend", post(graph_recommend))
        .route("/graph/node", post(graph_add_node))
        .route("/graph/edge", post(graph_add_edge))
        .route("/graph/activate", post(graph_activate))
        .route("/graph/search", get(graph_search))
        .route("/graph/auto-sync/toggle", post(graph_auto_sync_toggle))
        .route("/graph/auto-sync/status", get(graph_auto_sync_status))
        .route("/graph/export", get(graph_export))
        .route("/graph/import", post(graph_import))
        .route("/graph/ai-insights", post(graph_ai_insights))

        // ===== 对话会话 =====
        .route("/dialogue/sessions", get(dialogue_sessions))

        // ===== AI 对话 =====
        .route("/ai/chat", post(ai_chat))
        .route("/ai/chat/history/:session", get(ai_chat_history))
        .route("/ai/analyze-algorithm", post(ai_analyze_algorithm))
        .route("/ai/algorithm-types", get(ai_algorithm_types))
        .route("/ai/expert-chat", post(ai_expert_chat))
        .route("/ai/resources", get(ai_resources))
        .route("/ai/resources/health", get(ai_resources_health))

        // ===== 全维智能分析 =====
        .route("/ai/full-analysis", post(ai_full_analysis))
        .route("/ai/generate-doc", post(ai_generate_doc))
        .route("/ai/generate-flow-diagram", post(ai_generate_flow_diagram))
        .route("/ai/dev-test-fix", post(ai_dev_test_fix))
        .route("/ai/full-complete", post(ai_full_complete))
        .route("/ai/optimize-doc", post(ai_optimize_doc))
        .route("/ai/project-from-chat", post(ai_project_from_chat))
        .route("/ai/project-graph", post(ai_generate_project_graph))
        .route("/ai/req-db-link", post(ai_link_req_to_db))
        .route("/ai/alliance-pipeline", post(ai_alliance_pipeline))
        .route("/ai/publish-kb", post(ai_publish_artifacts_to_kb))
        .route("/ai/generate-erd", post(ai_generate_erd))
        .route("/ai/engine/flow-graph", get(ai_engine_flow_graph))

        // ===== 无穷维度优化 =====
        .route("/ai/infinite-optimize/benchmarks", get(infinite_benchmarks))
        .route("/ai/infinite-optimize/start", post(infinite_start))
        .route("/ai/infinite-optimize/stop", post(infinite_stop))
        .route("/ai/infinite-optimize/status", get(infinite_status))
        .route("/ai/infinite-optimize/results", get(infinite_results))
        .route("/ai/infinite-optimize/compare", post(infinite_compare))
        .route("/ai/infinite-optimize/comparison", get(infinite_comparison))
        .route("/ai/infinite-optimize/apply", post(infinite_apply))

        // ===== 本地制品引擎 =====
        .route("/ai/artifact/config", get(artifact_config))
        .route("/ai/artifact/list", get(artifact_list))
        .route("/ai/artifact/create", post(artifact_create))

        // ===== AI 插件 =====
        .route("/ai/plugins", get(ai_plugins_list))
        .route("/ai/plugins/register", post(ai_plugins_register))
        .route("/ai/plugins/send-message", post(ai_plugins_send_message))
        .route("/ai/plugins/topology", get(ai_plugins_topology))

        // ===== 工作流 =====
        .route("/ai/workflows/templates", get(workflow_templates))
        .route("/ai/workflows", get(workflows_list))
        .route("/ai/workflows/save", post(workflow_save))
        .route("/ai/workflows/execute", post(workflow_execute))
        .route("/ai/workflows/instances", get(workflow_instances))

        // ===== 流程图 =====
        .route("/ai/flows", get(flows_list))
        .route("/ai/flows", post(flow_create))
        .route("/ai/flows/:id", get(flow_get))
        .route("/ai/flows/:id", delete(flow_delete))
        .route("/ai/flows/validate", post(flow_validate))
        .route("/ai/flows/execute", post(flow_execute))
        .route("/ai/flows/node-types", get(flow_node_types))

        // ===== LLM 配置 =====
        .route("/ai/llm/config", get(llm_config_get))
        .route("/ai/llm/config", post(llm_config_update))
        .route("/ai/llm/test", post(llm_test))

        // ===== 浏览器自动化 =====
        .route("/ai/browser/templates", get(browser_templates))
        .route("/ai/browser/sessions", get(browser_sessions))
        .route("/ai/browser/sessions/:id", get(browser_session_get))
        .route("/ai/browser/sessions/:id", delete(browser_session_close))
        .route("/ai/browser/execute-task", post(browser_execute_task))
        .route("/ai/browser/execute-steps", post(browser_execute_steps))
        .route("/ai/browser/execute-action", post(browser_execute_action))
        .route("/ai/browser/natural", post(browser_natural))

        // ===== 联网搜索 =====
        .route("/web-search/config", get(web_search_config))
        .route("/web-search/config", post(web_search_config_update))
        .route("/web-search/test", post(web_search_test))
        .route("/web-search", post(web_search_do))

        // ===== 算子商城 =====
        .route("/market", get(market_list))
        .route("/market/random", get(market_random))
        .route("/market/:id", get(market_get))
        .route("/market/upload", post(market_upload))
        .route("/market/:id", post(market_update))
        .route("/market/:id", delete(market_delete))
        .route("/market/:id/clone", post(market_clone))
        .route("/market/:id/export", get(market_export))
        .route("/market/ai-search", post(market_ai_search))

        // ===== Caomei =====
        .route("/caomei/compile", post(caomei_compile))
        .route("/caomei/refine", post(caomei_refine))
        .route("/caomei/templates", get(caomei_templates))
        .route("/caomei/ai-parse", post(caomei_ai_parse))

        // ===== MCP =====
        .route("/mcp", post(mcp_handle))
        .route("/mcp/ai-map", post(mcp_ai_map))

        // ===== AI 自动化中枢 =====
        .route("/automation", get(automation_list))
        .route("/automation/chat", post(automation_chat))
        .route("/automation/:id/refine", post(automation_refine))
        .route("/automation/:id/run", post(automation_run))
        .route("/automation/:id/permissions", get(automation_permissions))
        .route("/automation/:id", put(automation_update))
        .route("/automation/ai-execute", post(automation_ai_execute))

        // ===== 璇玑全维治理 =====
        .route("/mox/health", get(mox_health))
        .route("/mox/optimize", post(mox_optimize))
        .route("/mox/publish", post(mox_publish))

        // ===== LLM 网关 =====
        .route("/llm/providers", get(llm_providers_list))
        .route("/llm/providers/presets", get(llm_provider_presets))
        .route("/llm/providers/:id", get(llm_provider_get))
        .route("/llm/providers/active", post(llm_set_active))
        .route("/llm/providers", post(llm_provider_add))
        .route("/llm/providers/:id", put(llm_provider_update))
        .route("/llm/providers/:id", delete(llm_provider_remove))
        .route("/llm/providers/:id/enable", post(llm_provider_enable))
        .route("/llm/providers/:id/disable", post(llm_provider_disable))
        .route("/llm/providers/:id/test", post(llm_provider_test))
        .route("/llm/providers/:id/discover", post(llm_provider_discover))
        .route("/llm/health", get(llm_health))
        .route("/llm/routing", get(llm_routing_get))
        .route("/llm/routing", put(llm_routing_update))
        .route("/llm/usage", get(llm_usage))
        .route("/llm/logs", get(llm_logs))
        .route("/llm/stats", get(llm_stats))

        // ===== 专家联盟 =====
        .route("/experts", get(proxy_experts_list))
        .route("/experts/capabilities", get(proxy_experts_capabilities))
        .route("/experts/metrics", get(proxy_experts_metrics))
        .route("/experts/overview", get(proxy_experts_overview))
        .route("/experts/multi-consult", post(proxy_experts_multi_consult))
        .route("/experts/debate", post(proxy_experts_debate))
        .route("/experts/route", post(proxy_experts_route))
        .route("/experts/intelligent-consult", post(proxy_experts_intelligent_consult))
        .route("/experts/algorithm-analysis", post(proxy_experts_algorithm_analysis))
        .route("/experts/enterprise/consult", post(proxy_experts_enterprise_consult))
        .route("/experts/enterprise/analyze", post(proxy_experts_enterprise_analyze))
        .route("/experts/orchestrate", post(proxy_experts_orchestrate))
        .route("/experts/plan/generate", post(proxy_experts_plan_generate))
        .route("/experts/plan/execute", post(proxy_experts_plan_execute))
        .route("/experts/orchestration/stats", get(proxy_orchestration_stats))
        .route("/experts/orchestration/plugins", get(proxy_orchestration_plugins))
        .route("/experts/orchestration/history", get(proxy_orchestration_history))
        .route("/experts/:id", get(proxy_experts_get))
        .route("/experts", post(proxy_experts_register))
        .route("/experts/:id", put(proxy_experts_update))
        .route("/experts/:id", delete(proxy_experts_remove))
        .route("/experts/:id/consult", post(proxy_experts_consult))
        .route("/experts/:id/metrics", get(proxy_experts_single_metrics))

        // ===== 专家会话 =====
        .route("/experts/sessions", get(proxy_experts_sessions_list))
        .route("/experts/sessions/stats", get(proxy_experts_sessions_stats))
        .route("/experts/sessions", post(proxy_experts_session_create))
        .route("/experts/sessions/:id", get(proxy_experts_session_get))
        .route("/experts/sessions/:id", put(proxy_experts_session_update))
        .route("/experts/sessions/:id", delete(proxy_experts_session_delete))
        .route("/experts/sessions/:id/messages", post(proxy_experts_session_append_message))
        .route("/experts/sessions/:id/similar-search", post(proxy_experts_session_similar_search))
        .route("/experts/sessions/:id/export", get(proxy_experts_session_export))
        .route("/experts/sessions/:id/archive", post(proxy_experts_session_archive))
        .route("/experts/semantic-search", post(proxy_experts_semantic_search))

        // ===== 调度策略 =====
        .route("/experts/dispatcher/config", get(proxy_dispatcher_config))
        .route("/experts/dispatcher/config", put(proxy_dispatcher_config_update))
        .route("/experts/dispatcher/status", get(proxy_dispatcher_status))
        .route("/experts/dispatcher/dispatch", post(proxy_dispatcher_dispatch))
        .route("/experts/dispatcher/consult", post(proxy_dispatcher_consult))
        .route("/experts/dispatcher/multi-consult", post(proxy_dispatcher_multi_consult))
        .route("/experts/dispatcher/reset/:id", post(proxy_dispatcher_reset_expert))
        .route("/experts/dispatcher/reset-all", post(proxy_dispatcher_reset_all))

        // ===== 专家图谱 =====
        .route("/expert-graph", get(proxy_expert_graph_get))
        .route("/expert-graph/stats", get(proxy_expert_graph_stats))
        .route("/expert-graph/neighbors/:id", get(proxy_expert_graph_neighbors))
        .route("/expert-graph/collaborators/:id", get(proxy_expert_graph_collaborators))
        .route("/expert-graph/path/:source/:target", get(proxy_expert_graph_path))
        .route("/expert-graph/communities", get(proxy_expert_graph_communities))
        .route("/expert-graph/optimal-team", post(proxy_expert_graph_optimal_team))
        .route("/expert-graph/rebuild", post(proxy_expert_graph_rebuild))

        // ===== 任务管理 =====
        .route("/tasks", get(tasks_list))
        .route("/tasks/auto", post(tasks_auto_create))
        .route("/tasks/from-chat", post(tasks_from_chat))
        .route("/tasks/:id", get(tasks_get))
        .route("/tasks", post(tasks_create))
        .route("/tasks/:id", put(tasks_update))
        .route("/tasks/:id", delete(tasks_delete))
        .route("/tasks/:id/to-chat", post(tasks_to_chat))
        .route("/tasks/:id/execute", post(tasks_execute))

        // ===== 项目中心 =====
        .route("/projects", get(projects_list))
        .route("/projects/types", get(projects_types))
        .route("/projects/catalog", get(projects_catalog))
        .route("/projects/stats", get(projects_stats))
        .route("/projects/by-resource", get(projects_by_resource))
        .route("/projects/:id", get(projects_get))
        .route("/projects", post(projects_create))
        .route("/projects/:id", put(projects_update))
        .route("/projects/:id", delete(projects_delete))
        .route("/projects/:id/resources", post(projects_bind_resources))
        .route("/projects/:id/resources/:rid", delete(projects_unbind_resource))
        .route("/projects/:id/resources/:rid", put(projects_update_resource_note))

        // ===== 16模块 AI 增强 =====
        .route("/workbench/ai-overview", get(workbench_ai_overview))
        .route("/resources/ai-analysis", post(resources_ai_analysis))
        .route("/workflow/ai-generate", post(workflow_ai_generate))
        .route("/plugins/ai-route", post(plugins_ai_route))
        .route("/browser/ai-instruct", post(browser_ai_instruct))
        .route("/monitor/ai-diagnose", post(monitor_ai_diagnose))
        .route("/docs/ai-explain", post(docs_ai_explain))
        .route("/algolab/ai-analyze", post(algolab_ai_analyze))
        .route("/fusion/ai-govern", post(fusion_ai_govern))

        // ===== 云盘知识库 =====
        .route("/kb/documents", get(kb_documents_list))
        .route("/kb/documents", post(kb_document_create))
        .route("/kb/documents/:id", get(kb_document_get))
        .route("/kb/documents/:id", put(kb_document_update))
        .route("/kb/documents/:id", delete(kb_document_delete))
        .route("/kb/documents/:id/analyze", post(kb_document_analyze))
        .route("/kb/batch-analyze", post(kb_batch_analyze))
        .route("/kb/categories", get(kb_categories))
        .route("/kb/tags", get(kb_tags))
        .route("/kb/search", post(kb_search))
        .route("/kb/documents/:id/versions", get(kb_doc_versions))
        .route("/kb/documents/:id/versions/:ver", get(kb_doc_version))
        .route("/kb/documents/:id/versions", post(kb_doc_create_version))
        .route("/kb/documents/:id/versions/compare", post(kb_doc_compare_versions))
        .route("/kb/documents/:id/versions/revert", post(kb_doc_revert_version))
        .route("/kb/documents/:id/entities", get(kb_doc_entities))
        .route("/kb/documents/:id/graph-link", post(kb_doc_graph_link))
        .route("/kb/documents/:id/history", get(kb_doc_history))
        .route("/kb/stats", get(kb_stats))
        .route("/kb/history", get(kb_history))

        // ===== Melody2Score =====
        .route("/melody2score/health", get(melody_health))
        .route("/melody2score/status", get(melody_status))
        .route("/melody2score/samples", get(melody_samples))
        .route("/melody2score/recognize", post(melody_recognize))
        .route("/melody2score/recognize-sample", post(melody_recognize_sample))
        .route("/melody2score/recognize-record", post(melody_recognize_record))
        .route("/melody2score/export-sheet", post(melody_export_sheet))
        .route("/melody2score/save-report", post(melody_save_report))

        // ===== 安全管理 =====
        .route("/security/status", get(security_status))
        .route("/security/api-keys", get(security_api_keys))
        .route("/security/api-keys", post(security_create_api_key))
        .route("/security/api-keys/:id", delete(security_revoke_api_key))
        .route("/security/validate", post(security_validate))
        .route("/security/audit-log", get(security_audit_log))

        // ===== 存储管理 =====
        .route("/storage/providers", get(storage_providers))
        .route("/storage/switch", post(storage_switch))
        .route("/storage/status", get(storage_status))

        // ===== 分析螺旋 =====
        .route("/analyze/spiral", post(analyze_spiral))

        // 兜底：未匹配路由返回 404（带 success 信封）
        .fallback(api_fallback)
        .with_state(state)
}

/// API 兜底：未匹配路由
async fn api_fallback(req: Request) -> impl IntoResponse {
    let path = req.uri().path().to_string();
    err(
        StatusCode::NOT_FOUND,
        "not_found",
        &format!("API 端点未实现: {}", path),
    )
}
