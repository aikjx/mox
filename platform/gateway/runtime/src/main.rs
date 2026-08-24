//! # 算子统一系统运行时 v3.0 - AI驱动全维突破平台
//!
//! 集成五大核心能力：
//! 1. AI智能对话 - 自然语言交互、意图识别、算子推荐
//! 2. 算法分析归一化 - 最强算法流程图生成与标准化
//! 3. 全资源管理 - CPU/内存/插件/算子/工作流统一调度
//! 4. 插件互通总线 - 发布订阅/点对点/请求响应
//! 5. 业务流程自动化 - BPMN工作流驱动AI执行

use axum::{
    extract::{Path, Query, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::Json,
    response::Response,
    routing::{delete, get, post, put},
    Router,
};
use graph_algorithms::{
    CentralityMetrics, Community, GraphStats, KnowledgeEdge, KnowledgeGraph, KnowledgeGraphBuilder,
    KnowledgeNode, NodeRecommendation, PathResult,
};
use operator_core::category::Workflow;
use operator_core::operator::{FunctionOperator, IdentityOperator, LinearOperator, Operator};
use operator_core::state::StateVector;
use operator_core::ExecutionContext;
use operator_wasm::WasmPluginManager;
// 璇玑全维治理内核：双璇玑十四维 → 治理报告
use xuanji_expert::context::GovernContext;
use xuanji_expert::pipeline::xuanji_optimize;
// OUS 前端治理台状态
use crate::handlers::governance::GovernanceState;
// HITL 人机协同审批状态
use crate::handlers::hitl::HitlState;
use ai_agent::{
    AIAgent, AlgorithmType, BrowserAction, BusinessWorkflow, ChatResponse, ExportBundle,
    FlowDefinition, PluginInfo, PluginStatus, PluginTopology, PluginType, ResourceHealthReport,
    ResourcePanorama, WorkflowResult,
};
use business_catalog::spiral::{analyze_spiral, SpiralParams};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing_subscriber::{prelude::*, EnvFilter};

// ========== 标准接口契约模块 ==========
// api_standard: RFC 9457 Problem+JSON 统一错误契约 + 响应标准化中间件
// openapi: OpenAPI 3.1 标准契约文档 + Swagger UI
// rbac_middleware: RBAC + 审计中间件
/// 统一 AI 查询：路由语义（静态→少参数→长路径优先）+ Node sidecar 客户端
mod ai_router;
mod api_standard;
/// AI 自动化中枢：需求对话 → 蓝图/流程图/代码/测试/RBAC → 沙箱实跑异常自动修复 → 回写
mod automation;
/// AI 自动化中枢共享资产模型 + 持久化（独立模块，避免与 market/automation 循环依赖）
mod automation_asset;
/// OUS 前端治理台 API：Dashboard / Audit / Config / WebSocket
mod handlers;
/// 算子商城：需求 + 可编辑业务流程图的资产市场
mod market;
/// 商城 DSL 转换：流程图 → FlowDefinition → BusinessWorkflow
mod market_dsl;
/// 商城路径迁移与存储 IO：$OUS_HOME/market/packages/ 归一化、备份、审计、ZIP、签名
mod market_migration;
/// 商城版本化管理：semver 快照 / 变更日志 / 回滚 / 差异对比
mod market_version;
mod openapi;
mod rbac_middleware;
mod routes;
mod sidecar;
/// 子服务聚合（Phase 1 收敛）：xuanji-expert / xuanji-system / primiflow / primiflow-fusion
/// 以库方式挂载，由 runtime 唯一对外暴露
mod subservers;

/// 应用状态 - AI全维系统核心
#[derive(Clone)]
struct AppState {
    // 原有组件
    operators: Arc<Mutex<Vec<OperatorInfo>>>,
    knowledge_graph: Arc<Mutex<KnowledgeGraph>>,
    plugin_manager: Arc<Mutex<WasmPluginManager>>,
    execution_logs: Arc<Mutex<Vec<ExecutionLog>>>,
    custom_operators: Arc<Mutex<HashMap<String, CustomOperatorDef>>>,
    // AI智能体
    ai_agent: Arc<AIAgent>,
    // 会话存储
    chat_sessions: Arc<Mutex<HashMap<String, Vec<ai_agent::ChatMessage>>>>,
    // 工作流存储
    saved_workflows: Arc<Mutex<HashMap<String, BusinessWorkflow>>>,
    // 算子商城
    market: market::MarketState,
    // 治理台状态
    governance: Arc<GovernanceState>,
    // HITL 人机协同审批状态
    hitl: Arc<HitlState>,
    // RBAC 访问审计接收器（放行/拒绝双写，供 /api/audit 查询）
    audit: Arc<rbac_middleware::MemoryAuditSink>,
    // 审计签名密钥（供 /api/audit 验签查询）
    audit_key: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OperatorInfo {
    id: String,
    name: String,
    description: String,
    category: String,
    input_type: String,
    output_type: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CustomOperatorDef {
    id: String,
    name: String,
    operator_type: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecutionLog {
    timestamp: u64,
    operator_id: String,
    workflow: Vec<String>,
    success: bool,
    execution_time_ms: u64,
    residual: f64,
    input_dim: usize,
    output_dim: usize,
}

// ========== 请求/响应结构 ==========

#[derive(Debug, Deserialize)]
struct ExecuteRequest {
    workflow: Vec<String>,
    input: Vec<f64>,
    parameters: Option<HashMap<String, f64>>,
}

#[derive(Debug, Serialize)]
struct ExecuteResponse {
    success: bool,
    output: Option<Vec<f64>>,
    execution_time_ms: u64,
    logs: Vec<String>,
    error: Option<String>,
    metrics: Option<ExecutionMetrics>,
}

#[derive(Debug, Serialize)]
struct ExecutionMetrics {
    input_norm: f64,
    output_norm: f64,
    l1_residual: f64,
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    session_id: Option<String>,
    message: String,
}

#[derive(Debug, Deserialize)]
struct AnalyzeAlgorithmRequest {
    code: String,
    algorithm_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorkflowExecuteRequest {
    workflow: Option<BusinessWorkflow>,
    template_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AddNodeRequest {
    id: String,
    label: String,
    node_type: Option<String>,
    properties: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct AddEdgeRequest {
    source: String,
    target: String,
    weight: f64,
    relation_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct GraphData {
    nodes: Vec<NodeData>,
    edges: Vec<EdgeData>,
    stats: GraphStats,
}

#[derive(Debug, Serialize)]
struct NodeData {
    id: String,
    label: String,
    node_type: String,
    pagerank: f64,
    degree_centrality: f64,
    activation: f64,
    size: f64,
    color: String,
}

#[derive(Debug, Serialize)]
struct EdgeData {
    source: String,
    target: String,
    weight: f64,
    relation_type: String,
}

#[derive(Debug, Deserialize)]
struct PathQuery {
    source: String,
    target: String,
}

#[derive(Debug, Deserialize)]
struct ActivationRequest {
    start_nodes: Vec<String>,
    iterations: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RecommendRequest {
    context_nodes: Vec<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RegisterOperatorRequest {
    id: String,
    name: String,
    operator_type: String,
    parameters: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct RegisterPluginRequest {
    id: String,
    name: String,
    plugin_type: Option<String>,
    capabilities: Vec<String>,
    input_topics: Vec<String>,
    output_topics: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PluginMessageRequest {
    source: String,
    target: Option<String>,
    topic: String,
    payload: serde_json::Value,
    need_response: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct LLMConfigRequest {
    api_base: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct BrowserTaskRequest {
    task_id: Option<String>,
    variables: Option<HashMap<String, String>>,
    steps: Option<Vec<BrowserAction>>,
    start_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BrowserActionRequest {
    session_id: String,
    action: BrowserAction,
}

#[derive(Debug, Deserialize)]
struct BrowserNaturalRequest {
    prompt: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 企业级可观测性：结构化日志 + 环境变量过滤（RUST_LOG，默认 info）
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_ansi(true),
        )
        .init();

    tracing::info!("🚀 启动算子统一系统 v3.0 - AI驱动全维突破平台...");

    // 生产级安全配置：API 访问令牌（缺失则仅开放只读/健康检查接口）
    let api_token = std::env::var("OUS_API_TOKEN").ok();
    if api_token.is_none() {
        tracing::warn!(
            "未设置环境变量 OUS_API_TOKEN，写操作/执行类接口将拒绝未授权访问；生产环境务必配置"
        );
    }

    // RBAC 令牌注册表：OUS_API_TOKEN 恒为 Admin（向后兼容）；
    // OUS_RBAC_TOKENS 格式 `令牌:角色[:租户]` 多组逗号分隔（admin/editor/viewer/operator/safety_approver/auditor）。
    // 配置了注册表即进入严格模式：仅注册表内令牌可认证，未知令牌一律 401。
    let (token_registry, rbac_skipped) = rbac_middleware::TokenRegistry::from_env("default");
    for note in &rbac_skipped {
        tracing::warn!(target: "auth", "{note}");
    }
    if token_registry.strict() {
        tracing::info!(
            "RBAC 令牌注册表已加载 {} 个条目（严格模式：仅注册表内令牌可认证）",
            token_registry.len()
        );
    } else {
        tracing::warn!(
            "未配置 OUS_RBAC_TOKENS，RBAC 处于兼容模式：OUS_API_TOKEN 恒为 Admin，其余令牌按前缀推断角色（仅建议开发环境使用，生产请显式配置）"
        );
    }
    let auth_ctx = AuthCtx {
        api_token: api_token.clone(),
        registry: Arc::new(token_registry),
    };

    // RBAC 上下文：租户 + 审计签名密钥 + 内存审计接收器（进程内可查）
    let audit_sink: Arc<rbac_middleware::MemoryAuditSink> =
        Arc::new(rbac_middleware::MemoryAuditSink::new());
    let audit_key = std::env::var("OUS_AUDIT_KEY")
        .map(|k| k.into_bytes())
        .unwrap_or_else(|_| b"ous-default-audit-key-2026".to_vec());
    let rbac_ctx = Arc::new(rbac_middleware::RbacContext {
        tenant_id: "default".to_string(),
        audit_key: audit_key.clone(),
        audit_sink: audit_sink.clone(),
    });

    // 初始化WASM插件管理器
    let mut plugin_manager = WasmPluginManager::new("./plugins");
    let _ = plugin_manager.load_all();

    // 构建超大规模知识图谱 - 算子关系网
    let kg = build_knowledge_graph();

    // 初始化AI智能体
    let ai_agent = Arc::new(AIAgent::new());

    // 启动时使用真实环境变量 DEEPSEEK_API_KEY 自动接入 DeepSeek LLM
    if let Ok(deepseek_key) = std::env::var("DEEPSEEK_API_KEY") {
        if !deepseek_key.is_empty() {
            ai_agent
                .configure_llm(ai_agent::LLMConfig {
                    api_base: "https://api.deepseek.com/v1".to_string(),
                    api_key: deepseek_key,
                    model: "deepseek-chat".to_string(),
                    temperature: 0.7,
                    max_tokens: 2048,
                    enabled: true,
                })
                .await;
            tracing::info!("已通过 DEEPSEEK_API_KEY 启用真实 LLM 接入 (model=deepseek-chat)");
        }
    } else {
        tracing::info!("未检测到 DEEPSEEK_API_KEY，AI 对话将使用内置规则引擎（离线降级）");
    }

    // 注册内置插件到AI插件总线
    {
        let bus = ai_agent.plugin_bus();
        let mut bus_guard = bus.write().await;
        // WASM插件管理器作为插件注册
        let _ = bus_guard.register(PluginInfo {
            id: "wasm-runtime".to_string(),
            name: "WASM插件运行时".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: PluginType::Builtin,
            capabilities: vec!["wasm_load".to_string(), "wasm_execute".to_string()],
            input_topics: vec!["wasm.load".to_string(), "wasm.execute".to_string()],
            output_topics: vec!["wasm.result".to_string()],
            status: PluginStatus::Active,
            metadata: HashMap::new(),
        });
        let _ = bus_guard.register(PluginInfo {
            id: "knowledge-graph".to_string(),
            name: "知识图谱引擎".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: PluginType::Builtin,
            capabilities: vec![
                "graph_query".to_string(),
                "recommend".to_string(),
                "centrality".to_string(),
            ],
            input_topics: vec!["graph.query".to_string(), "graph.recommend".to_string()],
            output_topics: vec!["graph.result".to_string()],
            status: PluginStatus::Active,
            metadata: HashMap::new(),
        });
    }

    // 注册预置工作流模板
    {
        let engine = ai_agent.workflow_engine();
        let engine_guard = engine.read().await;
        tracing::info!(
            "已加载 {} 个工作流模板",
            engine_guard.list_templates().len()
        );
    }

    // 初始化内置算子列表
    let operators = build_default_operators();

    // 初始化治理台状态（14维专家 + 审计链 + WebSocket广播）
    let governance_state = Arc::new(GovernanceState::default());
    governance_state.init_default_experts().await;
    tracing::info!("治理台状态已初始化（14维专家 + 审计链 + WebSocket广播）");

    // 初始化 HITL 人机协同审批状态
    let hitl_state = Arc::new(HitlState::default());
    tracing::info!("HITL 状态已初始化（待审批队列 + 广播通道）");

    let state = Arc::new(AppState {
        operators: Arc::new(Mutex::new(operators)),
        knowledge_graph: Arc::new(Mutex::new(kg)),
        plugin_manager: Arc::new(Mutex::new(plugin_manager)),
        execution_logs: Arc::new(Mutex::new(Vec::new())),
        custom_operators: Arc::new(Mutex::new(HashMap::new())),
        ai_agent,
        chat_sessions: Arc::new(Mutex::new(HashMap::new())),
        saved_workflows: Arc::new(Mutex::new(HashMap::new())),
        market: market::init_market_state().await,
        governance: governance_state,
        hitl: hitl_state,
        audit: audit_sink,
        audit_key,
    });

    // 创建路由 - 全维API
    let app = Router::new()
        // ========== 基础系统API ==========
        .route("/api/health", get(health))
        .route("/api/operators", get(list_operators))
        .route("/api/operators/register", post(register_operator))
        .route("/api/execute", post(execute_workflow))
        // ========== 知识图谱API ==========
        .route("/api/graph", get(get_graph))
        .route("/api/graph/stats", get(get_graph_stats))
        .route("/api/graph/node", post(add_node))
        .route("/api/graph/edge", post(add_edge))
        .route("/api/graph/neighbors/:id", get(get_neighbors))
        .route("/api/graph/centrality", get(get_centrality))
        .route("/api/graph/communities", get(get_communities))
        .route("/api/graph/path", get(get_shortest_path))
        .route("/api/graph/pagerank", get(get_pagerank))
        .route("/api/graph/activate", post(propagate_activation))
        .route("/api/graph/recommend", post(recommend_nodes))
        // ========== 对话自动→知识图谱 自动整理 ==========
        .route("/api/graph/search", get(graph_search))
        .route("/api/graph/auto-sync/toggle", post(toggle_auto_sync))
        .route("/api/graph/auto-sync/status", get(auto_sync_status))
        .route("/api/dialogue/sessions", get(list_dialogue_sessions))
        .route("/api/graph/export", get(graph_export))
        .route("/api/graph/import", post(graph_import))
        // ========== AI智能对话API ==========
        .route("/api/ai/chat", post(ai_chat))
        .route("/api/ai/chat/history/:session", get(get_chat_history))
        // ========== 草莓多平台：对话驱动全栈生成 ==========
        .route("/api/caomei/compile", post(caomei_compile))
        .route("/api/caomei/refine", post(caomei_refine))
        .route("/api/caomei/templates", get(caomei_templates))
        // ========== 算法分析归一化API ==========
        .route("/api/ai/analyze-algorithm", post(analyze_algorithm))
        .route("/api/ai/algorithm-types", get(list_algorithm_types))
        // ========== 全资源管理API ==========
        .route("/api/ai/resources", get(get_resources))
        .route("/api/ai/resources/health", get(resource_health))
        // ========== 插件互通API ==========
        .route("/api/ai/plugins", get(list_ai_plugins))
        .route("/api/ai/plugins/register", post(register_ai_plugin))
        .route("/api/ai/plugins/topology", get(plugin_topology))
        .route("/api/ai/plugins/send-message", post(send_plugin_message))
        // ========== 业务流程自动化API ==========
        .route("/api/ai/workflows/templates", get(list_workflow_templates))
        .route("/api/ai/workflows", get(list_workflows))
        .route("/api/ai/workflows/execute", post(execute_business_workflow))
        .route("/api/ai/workflows/save", post(save_workflow))
        .route("/api/ai/workflows/instances", get(list_workflow_instances))
        // ========== LLM配置API ==========
        .route("/api/ai/llm/config", get(get_llm_config))
        .route("/api/ai/llm/config", post(update_llm_config))
        .route("/api/ai/llm/test", post(test_llm_connection))
        // ========== 浏览器自动化API ==========
        .route("/api/ai/browser/templates", get(list_browser_templates))
        .route("/api/ai/browser/sessions", get(list_browser_sessions))
        .route("/api/ai/browser/execute-task", post(execute_browser_task))
        .route("/api/ai/browser/execute-steps", post(execute_browser_steps))
        .route(
            "/api/ai/browser/execute-action",
            post(execute_browser_action),
        )
        .route("/api/ai/browser/natural", post(browser_natural))
        .route("/api/ai/browser/sessions/:id", get(get_browser_session))
        .route(
            "/api/ai/browser/sessions/:id",
            delete(close_browser_session),
        )
        // ========== 流程图引擎API ==========
        .route("/api/ai/flows", get(list_flows))
        .route("/api/ai/flows", post(create_flow))
        .route("/api/ai/flows/:id", get(get_flow))
        .route("/api/ai/flows/:id", put(update_flow))
        .route("/api/ai/flows/:id", delete(delete_flow))
        .route("/api/ai/flows/validate", post(validate_flow))
        .route("/api/analyze/spiral", post(analyze_spiral_handler))
        .route("/api/ai/flows/execute", post(execute_flow))
        .route("/api/ai/flows/node-types", get(list_flow_node_types))
        // ========== 算子商城 API ==========
        // 需求 + 可编辑业务流程图的资产市场；挂载为独立 state 子路由
        .nest("/api/market", {
            let market_state = state.market.clone();
            // 基础路由 + 扩展路由（导入/导出/租户/所有者/下载），合并为完整商城 API
            market::market_routes()
                .with_state(market_state.clone())
                .merge(crate::routes::market::extra_routes().with_state(market_state))
        })
        // ========== AI 自动化中枢 API ==========
        // 需求驱动的端到端闭环：对话生成 → 自动代码/测试/RBAC → 沙箱实跑异常修复回写。
        // 与根路由共用 AppState（含 ai_agent / market）。
        .nest("/api/automation", automation::router())
        // ========== AI Agent 引擎任务 API ==========
        // AI Agent 引擎任务执行端点，共享 AppState（含 ai_agent）。
        .nest(
            "/api/agent",
            crate::routes::agent::agent_routes().with_state(state.ai_agent.clone()),
        )
        // ========== 统一 AI 查询：/ai/engine/* 四端点（T6）==========
        // 四端点：POST process / POST analyze / GET capabilities / GET metrics。
        // 统一语义网关：本地等价直调 sidecar → backend-node；AI 混合编排；返回 data 段 shape 与本地同。
        .nest("/ai/engine", {
            use crate::handlers::ai_engine::AiEngineState;
            use crate::sidecar::node_sidecar::NodeSidecarClient;
            let ai_state = AiEngineState::default()
                .with_agent(state.ai_agent.clone())
                .with_sidecar(NodeSidecarClient::new(
                    std::env::var("BACKEND_NODE_INTERNAL_BASE")
                        .unwrap_or_else(|_| "http://127.0.0.1:3010".to_string()),
                ));
            let r: Router =
                crate::routes::ai_engine::ai_engine_routes(std::sync::Arc::new(ai_state));
            r.with_state(())
        })
        // ========== MCP 兼容层（Model Context Protocol）==========
        // 把内置算子与 AI 插件统一暴露为标准 MCP tools，兼容任意开源 MCP 客户端
        .route("/api/mcp", post(handle_mcp_rpc))
        // ========== 系统API ==========
        // 标准接口契约：OpenAPI 3.1 规范 + Swagger UI
        .route("/api/openapi.yaml", get(openapi::serve_openapi_yaml))
        .route("/api/docs", get(openapi::serve_swagger_ui))
        .route("/api/plugins", get(list_plugins))
        .route("/api/logs", get(get_logs))
        .route("/api/audit", get(get_access_audit))
        .route("/api/status", get(get_status))
        .route("/api/status/full", get(get_full_status))
        // ========== 璇玑全维治理 API ==========
        .route("/api/xuanji/health", get(xuanji_health))
        .route("/api/xuanji/optimize", post(xuanji_optimize_handler))
        .route("/api/xuanji/publish", post(xuanji_publish_handler))
        // ========== OUS 前端治理台 API（/api/governance/*）==========
        // 全维治理：Dashboard / 专家状态 / 否决事件 / 审计日志 / RBAC 配置 / 专家配置 / WS 实时推送 / 治理评估。
        // 状态自包含于 GovernanceState（handlers/governance.rs），已适配 xuanji-expert 当前 API。
        .nest("/api/governance", {
            let gov_state = state.governance.clone();
            crate::routes::governance::governance_routes().with_state(gov_state)
        })
        // ========== HITL 人机协同审批 WebSocket ==========
        .route(
            "/ws/hitl",
            get(crate::handlers::hitl::hitl_ws_handler).with_state(state.hitl.clone()),
        )
        // ========== 静态前端
        .nest_service("/", ServeDir::new("./frontend/dist"))
        // 先固化状态为 Router<()>（axum nest 要求内外 state 一致），再挂载子服务
        .with_state(state);

    // ========== 子服务聚合（Phase 1 收敛）==========
    // 四套并行 server（xuanji-expert / xuanji-system / primiflow / primiflow-fusion）
    // 收敛为库，统一由 operator-server 对外暴露；可用 OUS_ENABLE_* 分别关闭。
    let subs = subservers::build().await;
    for note in &subs.notes {
        tracing::info!("{note}");
    }
    let mut app = app;
    for (prefix, router) in subs.routers {
        app = app.nest(prefix, router);
    }

    // 三层安全管线（从外到内）：
    //   ① CORS 受控来源
    //   ② auth_middleware：Bearer 令牌认证 → 写入 Principal 到请求扩展
    //   ③ rbac_audit_middleware：按角色授权 + 放行/拒绝双向审计（签名留痕）
    // 最内层 standardize_response：将 200+{success:false} 伪成功改写为 RFC 9457 错误。
    let app = app
        .layer(middleware::from_fn(api_standard::standardize_response))
        .layer(middleware::from_fn_with_state(
            rbac_ctx,
            rbac_middleware::rbac_audit_middleware,
        ))
        .layer(middleware::from_fn_with_state(auth_ctx, auth_middleware))
        .layer(build_cors()?);

    // 解析命令行参数：支持 `--port <NUM>`（默认 3001，Node 边缘入口占 3000）
    let mut port: u16 = 3001;
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            if let Ok(p) = args[i + 1].parse::<u16>() {
                port = p;
            }
        }
    }
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("📡 服务器监听在 http://{}", addr);
    tracing::info!("══════════════════════════════════════════════════════════");
    tracing::info!("  🚀 算子统一系统 v3.0 - AI驱动全维突破平台");
    tracing::info!("  🧠 AI智能对话 · 算法归一化 · 全资源管理 · 插件互通 · 流程自动化");
    tracing::info!("  🤖 真实AI对接(OpenAI兼容) · 浏览器自动化 · 知识图谱(34+节点)");
    tracing::info!("  🌐 访问地址: http://localhost:{}", port);
    tracing::info!("══════════════════════════════════════════════════════════");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    // 生产级优雅关闭：收到 SIGINT/SIGTERM 时停止接收新连接，等待在途请求完成
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("🛑 运行时已优雅关闭");
    Ok(())
}

/// 受控 CORS：默认仅允许同源/本地来源，可通过 `OUS_CORS_ORIGINS` 环境变量
/// 以逗号分隔配置额外受信来源（生产环境务必显式配置，禁止 '*'）。
fn build_cors() -> anyhow::Result<CorsLayer> {
    let allowed = std::env::var("OUS_CORS_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,http://127.0.0.1:3000".to_string());
    let origins: Vec<_> = allowed
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if origins.contains(&"*") {
        tracing::warn!("OUS_CORS_ORIGINS 包含 '*'，已退化为全开放 CORS（不推荐用于生产）");
        return Ok(CorsLayer::permissive());
    }
    let origins: Vec<axum::http::HeaderValue> =
        origins.into_iter().filter_map(|o| o.parse().ok()).collect();
    Ok(CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers(Any))
}

/// 认证上下文：网关主令牌（恒为 Admin）+ RBAC 令牌注册表。
#[derive(Clone)]
struct AuthCtx {
    api_token: Option<String>,
    registry: Arc<rbac_middleware::TokenRegistry>,
}

/// 令牌 → 认证主体解析。
///
/// 优先级（严格到宽松）：
/// 1. `OUS_API_TOKEN` 精确匹配 → Admin（向后兼容既有部署与前端调用）；
/// 2. 已配置 `OUS_RBAC_TOKENS` 注册表（严格模式）→ 仅注册表内令牌可认证；
/// 3. 兼容模式（未配置注册表）→ 按令牌前缀推断角色（仅建议开发环境）。
fn resolve_principal(auth: &AuthCtx, token: &str) -> Option<rbac_middleware::Principal> {
    // 1) 网关主令牌恒为 Admin
    if auth.api_token.as_deref() == Some(token) {
        return Some(rbac_middleware::Principal {
            token_id: token.chars().take(8).collect(),
            roles: vec![rbac_middleware::Role::Admin],
            tenant_id: "default".to_string(),
        });
    }
    // 2) 严格模式：已显式配置 OUS_RBAC_TOKENS 时，仅注册表内令牌可认证
    if auth.registry.strict() {
        return auth.registry.resolve(token);
    }
    // 3) 兼容模式（默认关闭）：仅当显式设置 OUS_AUTH_COMPAT=1 时才按令牌前缀推断角色。
    //    默认关闭后，未配置 OUS_API_TOKEN / OUS_RBAC_TOKENS 时任何令牌都无法认证（安全默认），
    //    杜绝「admin_* 任意令牌即 Admin」的越权默认。
    let compat = std::env::var("OUS_AUTH_COMPAT")
        .map(|v| v == "1")
        .unwrap_or(false);
    if compat {
        let roles = rbac_middleware::extract_roles_from_token(token);
        if roles.is_empty() {
            return None;
        }
        return Some(rbac_middleware::Principal {
            token_id: token.chars().take(8).collect(),
            roles,
            tenant_id: rbac_middleware::extract_tenant_from_token(token, "default"),
        });
    }
    None
}

/// 极简 URL 百分号解码（用于查询参数 token；非完整 RFC 3986，仅覆盖 `+` 与 `%XX`）。
fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = hex_val(bytes[i + 1]);
                let lo = hex_val(bytes[i + 2]);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push((h << 4) | l);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// 鉴权中间件：除公开端点（健康检查、静态资源、AI 对话、子服务透传）外，
/// 所有 API 必须携带 `Authorization: Bearer <令牌>`。
///
/// 认证通过后将 [`rbac_middleware::Principal`] 写入请求扩展，供内层
/// `rbac_audit_middleware` 直接读取，避免两层各自解析令牌导致口径不一致。
/// 未配置令牌时拒绝一切受保护接口。
async fn auth_middleware(
    State(auth): State<AuthCtx>,
    headers: HeaderMap,
    mut req: Request,
    next: Next,
) -> Result<Response, api_standard::ProblemDetail> {
    let path = req.uri().path().to_string();
    // 子服务聚合边界（见 subservers.rs）：
    // - /xuanji-system/* 由子服务自带成员令牌 RBAC 鉴权 → 网关透传
    // - /xuanji-viz /primiflow /fusion 无自带鉴权 → 网关令牌统一保护
    let is_passthrough = subservers::PASSTHROUGH_PREFIXES
        .iter()
        .any(|p| path.starts_with(p));
    let is_gateway = subservers::GATEWAY_PREFIXES
        .iter()
        .any(|p| path.starts_with(p));
    // 公开端点：健康检查、前端静态资源、AI对话、子服务透传前缀（无需网关token）。
    // 注意：/ai/engine/* 是统一 AI 能力编排入口（process/analyze/flow-graph/workflow 等），
    // 必须要求 Bearer 认证，防止匿名调用烧耗 LLM/浏览器自动化预算。前端经统一 fetcher（自动注入令牌）调用。
    if !is_gateway
        && (path == "/api/health"
            || path == "/healthz"
            || (!path.starts_with("/api/")
                && !path.starts_with("/ai/engine")
                && !path.starts_with("/ws/hitl"))
            || path == "/api/ai/chat"
            || path.starts_with("/api/ai/chat/history")
            || is_passthrough
            || (path.starts_with("/api/market") && req.method() == axum::http::Method::GET))
    {
        return Ok(next.run(req).await);
    }
    // 令牌来源：优先 Authorization: Bearer；WebSocket 升级(如 /ws/hitl)无法携带自定义头，
    // 前端经查询参数 ?token= 传入（见 hitl-ws.js），此处做 URL 解码兼容。
    let bearer = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t.to_string());
    let query_token = req.uri().query().and_then(|q| {
        q.split('&')
            .find_map(|kv| kv.strip_prefix("token="))
            .map(url_decode)
    });
    let token = bearer.or(query_token);
    match token {
        None => {
            tracing::warn!(target: "auth", "缺少 Authorization: Bearer 令牌: {}", path);
            Err(api_standard::ProblemDetail::new(
                StatusCode::UNAUTHORIZED,
                "缺少或无效的 Authorization: Bearer 令牌",
                Some("UNAUTHORIZED".into()),
            ))
        }
        Some(t) => match resolve_principal(&auth, &t) {
            Some(principal) => {
                req.extensions_mut().insert(principal);
                Ok(next.run(req).await)
            }
            None => {
                tracing::warn!(target: "auth", "未授权访问被拒绝: {}", path);
                Err(api_standard::ProblemDetail::new(
                    StatusCode::UNAUTHORIZED,
                    "缺少或无效的 Authorization: Bearer 令牌",
                    Some("UNAUTHORIZED".into()),
                ))
            }
        },
    }
}

/// 优雅关闭信号：监听 SIGINT / SIGTERM
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut sig) = signal(SignalKind::terminate()) {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("收到 SIGINT，准备优雅关闭..."); },
        _ = terminate => { tracing::info!("收到 SIGTERM，准备优雅关闭..."); },
    }
}

fn build_knowledge_graph() -> KnowledgeGraph {
    KnowledgeGraphBuilder::new()
        .add_node("identity", "恒等算子", "core")
        .add_node("linear", "线性变换", "core")
        .add_node("normalize", "归一化", "core")
        .add_node("relu", "ReLU激活", "activation")
        .add_node("sigmoid", "Sigmoid激活", "activation")
        .add_node("tanh", "Tanh激活", "activation")
        .add_node("softmax", "Softmax", "activation")
        .add_node("add", "加法", "math")
        .add_node("multiply", "乘法", "math")
        .add_node("matmul", "矩阵乘法", "math")
        .add_node("transpose", "转置", "math")
        .add_node("inverse", "矩阵求逆", "math")
        .add_node("conv2d", "2D卷积", "signal")
        .add_node("maxpool", "最大池化", "signal")
        .add_node("avgpool", "平均池化", "signal")
        .add_node("fft", "快速傅里叶", "signal")
        .add_node("dct", "离散余弦", "signal")
        .add_node("reshape", "维度重塑", "data")
        .add_node("concat", "张量拼接", "data")
        .add_node("split", "张量分割", "data")
        .add_node("dropout", "Dropout", "regularization")
        .add_node("batchnorm", "批归一化", "normalization")
        .add_node("attention", "注意力机制", "ai")
        .add_node("self_attention", "自注意力", "ai")
        .add_node("cross_attention", "交叉注意力", "ai")
        .add_node("feedforward", "前馈网络", "ai")
        .add_node("embedding", "词嵌入", "ai")
        .add_node("positional", "位置编码", "ai")
        .add_node("pagerank_op", "PageRank", "graph")
        .add_node("community", "社区发现", "graph")
        .add_node("shortest_path", "最短路径", "graph")
        .add_node("centrality", "中心性计算", "graph")
        .add_node("sgd", "SGD优化", "optimizer")
        .add_node("adam", "Adam优化", "optimizer")
        .add_node("adamw", "AdamW优化", "optimizer")
        .add_node("mse", "均方误差", "loss")
        .add_node("cross_entropy", "交叉熵", "loss")
        .add_edge_typed("identity", "linear", 0.9, "transforms")
        .add_edge_typed("linear", "relu", 0.95, "activation")
        .add_edge_typed("linear", "sigmoid", 0.9, "activation")
        .add_edge_typed("linear", "tanh", 0.85, "activation")
        .add_edge_typed("relu", "normalize", 0.8, "normalizes")
        .add_edge_typed("sigmoid", "softmax", 0.9, "compose")
        .add_edge_typed("matmul", "linear", 0.95, "implements")
        .add_edge_typed("conv2d", "relu", 0.9, "activation")
        .add_edge_typed("conv2d", "maxpool", 0.85, "pools")
        .add_edge_typed("conv2d", "batchnorm", 0.8, "normalizes")
        .add_edge_typed("attention", "self_attention", 0.95, "specializes")
        .add_edge_typed("attention", "cross_attention", 0.9, "specializes")
        .add_edge_typed("self_attention", "feedforward", 0.85, "feeds")
        .add_edge_typed("embedding", "positional", 0.9, "combines")
        .add_edge_typed("positional", "attention", 0.9, "prepares")
        .add_edge_typed("dropout", "relu", 0.7, "regularizes")
        .add_edge_typed("batchnorm", "dropout", 0.75, "regularizes")
        .add_edge_typed("adam", "sgd", 0.8, "extends")
        .add_edge_typed("adamw", "adam", 0.9, "extends")
        .add_edge_typed("mse", "cross_entropy", 0.7, "alternative")
        .add_edge_typed("pagerank_op", "centrality", 0.9, "related")
        .add_edge_typed("community", "shortest_path", 0.75, "analyzes")
        .add_edge_typed("normalize", "softmax", 0.85, "compose")
        .add_edge_typed("fft", "conv2d", 0.7, "accelerates")
        .add_edge_typed("reshape", "matmul", 0.6, "prepares")
        .add_edge_typed("concat", "reshape", 0.65, "prepares")
        .add_edge_typed("transpose", "matmul", 0.8, "required_by")
        .add_edge_typed("inverse", "linear", 0.7, "solves")
        .build()
}

fn build_default_operators() -> Vec<OperatorInfo> {
    vec![
        OperatorInfo {
            id: "identity".to_string(),
            name: "恒等算子".to_string(),
            description: "输出等于输入，用于测试和残差连接".to_string(),
            category: "core".to_string(),
            input_type: "StateVector".to_string(),
            output_type: "StateVector".to_string(),
            parameters: serde_json::json!({}),
        },
        OperatorInfo {
            id: "linear".to_string(),
            name: "线性变换算子".to_string(),
            description: "y = 2x，可配置缩放因子".to_string(),
            category: "core".to_string(),
            input_type: "StateVector".to_string(),
            output_type: "StateVector".to_string(),
            parameters: serde_json::json!({"scale": 2.0}),
        },
        OperatorInfo {
            id: "normalize".to_string(),
            name: "L2归一化算子".to_string(),
            description: "归一化到单位范数（欧几里得范数=1）".to_string(),
            category: "core".to_string(),
            input_type: "StateVector".to_string(),
            output_type: "StateVector".to_string(),
            parameters: serde_json::json!({}),
        },
        OperatorInfo {
            id: "normalize_l1".to_string(),
            name: "L1归一化算子".to_string(),
            description: "归一化到概率分布（L1范数=1）".to_string(),
            category: "core".to_string(),
            input_type: "StateVector".to_string(),
            output_type: "StateVector".to_string(),
            parameters: serde_json::json!({}),
        },
        OperatorInfo {
            id: "relu".to_string(),
            name: "ReLU激活算子".to_string(),
            description: "max(0, x)，整流线性单元".to_string(),
            category: "activation".to_string(),
            input_type: "StateVector".to_string(),
            output_type: "StateVector".to_string(),
            parameters: serde_json::json!({}),
        },
        OperatorInfo {
            id: "sigmoid".to_string(),
            name: "Sigmoid激活算子".to_string(),
            description: "1/(1+exp(-x))，S型激活函数".to_string(),
            category: "activation".to_string(),
            input_type: "StateVector".to_string(),
            output_type: "StateVector".to_string(),
            parameters: serde_json::json!({}),
        },
        OperatorInfo {
            id: "tanh".to_string(),
            name: "Tanh激活算子".to_string(),
            description: "双曲正切激活函数".to_string(),
            category: "activation".to_string(),
            input_type: "StateVector".to_string(),
            output_type: "StateVector".to_string(),
            parameters: serde_json::json!({}),
        },
        OperatorInfo {
            id: "softmax".to_string(),
            name: "Softmax算子".to_string(),
            description: "指数归一化，输出概率分布".to_string(),
            category: "activation".to_string(),
            input_type: "StateVector".to_string(),
            output_type: "StateVector".to_string(),
            parameters: serde_json::json!({}),
        },
        OperatorInfo {
            id: "scale".to_string(),
            name: "缩放算子".to_string(),
            description: "按指定因子缩放向量".to_string(),
            category: "math".to_string(),
            input_type: "StateVector".to_string(),
            output_type: "StateVector".to_string(),
            parameters: serde_json::json!({"factor": 1.0}),
        },
        OperatorInfo {
            id: "add_bias".to_string(),
            name: "偏置加算子".to_string(),
            description: "添加可学习偏置".to_string(),
            category: "math".to_string(),
            input_type: "StateVector".to_string(),
            output_type: "StateVector".to_string(),
            parameters: serde_json::json!({}),
        },
    ]
}

// ========== 基础API处理器 ==========

async fn health() -> &'static str {
    "OK - AI Operator System v3.0 Running - Full-Dimensional Breakthrough"
}

// ========== 璇玑全维治理 API ==========
// 把后端双璇玑十四维决策内核暴露给前端设计器：传入流程蓝图即可拿到
// 各维度健康分、治理闸门、璇玑校验、采纳建议，驱动"可视化治理闭环"。

/// 请求体：前端友好的任意流程蓝图（支持 {nodes,edges} 宽松结构，handler 内归一化为 FlowGraph）。
#[derive(Debug, Clone, Deserialize)]
struct XuanjiOptimizeRequest {
    flow: serde_json::Value,
    /// 租户策略分层（I-06）："gov"=强合规租户（政务/金融，强制脱敏/灾备闸门），
    /// 其它=普通商业租户。驱动治理 8 闸门按租户严格度差异化裁决。
    #[serde(default)]
    tenant: Option<String>,
}

/// 全维治理：返回 GovernanceReport（专家评分 + 优化 + 璇玑验证 + 闸门 + 审计 + 采纳建议）
fn normalize_flow_to_graph(v: &serde_json::Value) -> flow_ai::model::FlowGraph {
    let mut g = flow_ai::model::FlowGraph::new("unified", "unified-flow");
    if let Some(nodes) = v.get("nodes").and_then(|n| n.as_array()) {
        for n in nodes {
            let id = n
                .get("id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let name = n
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let t = n.get("type").and_then(|x| x.as_str()).unwrap_or("operator");
            let kind = match t {
                "start" => flow_ai::model::NodeKind::Start,
                "end" => flow_ai::model::NodeKind::End,
                "condition" | "decision" => flow_ai::model::NodeKind::Decision,
                "parallel" => flow_ai::model::NodeKind::ParallelFork,
                "guard" => flow_ai::model::NodeKind::Guard,
                "subflow" => flow_ai::model::NodeKind::SubFlow,
                _ => flow_ai::model::NodeKind::Task,
            };
            let mut node = flow_ai::model::FlowNode::new(id, name, kind);
            if let Some(tool) = n.get("tool").and_then(|x| x.as_str()) {
                node.tool =
                    serde_json::from_str::<flow_ai::model::ToolKind>(&format!("\"{}\"", tool)).ok();
            }
            g.add_node(node);
        }
    }
    if let Some(edges) = v.get("edges").and_then(|e| e.as_array()) {
        for e in edges {
            let from = e
                .get("from")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let to = e
                .get("to")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let kind = if e.get("condition").is_some() || e.get("label").is_some() {
                flow_ai::model::EdgeKind::Conditional
            } else {
                flow_ai::model::EdgeKind::Sequence
            };
            let condition = e
                .get("condition")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string());
            let edge = flow_ai::model::FlowEdge {
                from,
                to,
                kind,
                condition,
            };
            g.add_edge(edge);
        }
    }
    g
}
async fn xuanji_optimize_handler(
    Json(req): Json<XuanjiOptimizeRequest>,
) -> Json<serde_json::Value> {
    let tenant = match req.tenant.as_deref() {
        Some("gov") => xuanji_expert::context::Tenant::new("gov", "gov-ns").regulated(true),
        _ => xuanji_expert::context::Tenant::new("default", "default"),
    };
    let ctx = GovernContext::new(
        tenant,
        xuanji_expert::context::Principal::new("designer").with_roles(vec!["editor".into()]),
    );
    let report = xuanji_optimize(&normalize_flow_to_graph(&req.flow), &ctx);
    // 契约适配层：在原 GovernanceReport 基础上注入前端友好字段
    // （governance.score/gate、optimization.metric/algorithm），不改动治理内核。
    let score: f64 = if report.expert_scores.is_empty() {
        0.0
    } else {
        report.expert_scores.iter().map(|(_, s)| s).sum::<f64>() / report.expert_scores.len() as f64
    };
    let mut v = serde_json::to_value(&report).unwrap_or(json!({"error": "serialize"}));
    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "governance".to_string(),
            json!({
                "score": score,
                "gate": format!("{:?}", report.gate.status),
                // 完整治理闸门对象（含 8 闸门明细），供前端审计链/双验收展示
                "gate_detail": serde_json::to_value(&report.gate).unwrap_or(json!({})),
            }),
        );
        if let Some(opt) = obj.get_mut("optimization").and_then(|o| o.as_object_mut()) {
            let g = &report.optimization.gains;
            opt.insert(
                "metric".to_string(),
                json!({
                    "critical_path_ms": g.critical_path_ms,
                    "speedup": g.speedup,
                    "time_saved_pct": g.time_saved_pct,
                    "compute_saved_pct": g.compute_saved_pct,
                }),
            );
            opt.insert("algorithm".to_string(), json!(report.algo.summary));
        }
    }
    Json(v)
}

async fn xuanji_publish_handler(Json(req): Json<XuanjiPublishRequest>) -> Json<serde_json::Value> {
    use xuanji_expert::context::{GovernContext, Principal, Tenant};
    let ctx = GovernContext::new(
        Tenant::new("default", "default"),
        Principal::new("designer").with_roles(vec!["editor".into()]),
    );
    let report = xuanji_optimize(&normalize_flow_to_graph(&req.flow), &ctx);
    let optimized = &report.optimization.optimized_graph;
    let score: f64 = if report.expert_scores.is_empty() {
        0.0
    } else {
        report.expert_scores.iter().map(|(_, s)| s).sum::<f64>() / report.expert_scores.len() as f64
    };
    let name = req.name.clone().unwrap_or_else(|| "全维融合算子".into());
    let description = req
        .description
        .clone()
        .unwrap_or_else(|| format!("由璇玑双璇玑十四维归一化生成（治理评分 {:.2}）", score));
    let requirement = req.requirement.clone().unwrap_or_default();
    let tags = req
        .tags
        .clone()
        .unwrap_or_else(|| vec!["全维融合".into(), "璇玑".into(), "业务流程图".into()]);

    // ===== I-05 双验收联动门禁 =====
    // 需求侧任务 Done（req.task_done=true） ∧ 融合侧璇玑验证通过（algo 未否决且 gate 放行）
    // 二者同时满足才允许上架；任一方不达成则强制 blocked，原因写回前端审计链。
    let task_done = req.task_done.unwrap_or(false);
    let dual_acceptance = xuanji_expert::tenant_policy::dual_acceptance(task_done, &report);
    if !dual_acceptance {
        let mut reasons: Vec<String> = Vec::new();
        if !task_done {
            reasons.push("需求侧任务未标记 Done（task_done=false）".into());
        }
        if report.algo.vetoed {
            reasons.push("融合侧璇玑验证否决（⛨ 最高权限）".into());
        }
        if !report.gate.approved {
            reasons.push(format!("治理门禁未通过：{}", report.gate.reason));
        }
        return Json(json!({
            "published": false,
            "blocked": true,
            "dual_acceptance": false,
            "reason": reasons.join("；"),
            "governance": { "score": score, "gate": format!("{:?}", report.gate.status), "algo_veto": report.algo.vetoed },
            "gates": report.gate.gates.iter().map(|g| json!({ "id": g.id.code(), "name": g.id.name(), "passed": g.passed, "reason": g.reason })).collect::<Vec<_>>(),
        }));
    }

    match crate::market::publish_unified(
        name,
        description,
        requirement,
        optimized.nodes.clone(),
        optimized.edges.clone(),
        tags,
        Some(&report),
        req.task_id.clone(),
    ) {
        Ok(pkg) => Json(json!({
            "package": { "id": pkg.id, "name": pkg.name, "category": pkg.category, "nodes": pkg.nodes.len(), "edges": pkg.edges.len() },
            "published": true,
            "dual_acceptance": true,
            "provenance": pkg.provenance,
            "governance": { "score": score, "gate": format!("{:?}", report.gate.status) },
            "optimization": { "critical_path_ms": report.optimization.gains.critical_path_ms, "conflicts_found": report.optimization.gains.conflicts_found },
        })),
        Err(e) => Json(json!({ "published": false, "error": e.to_string() })),
    }
}

/// 治理内核健康度：列出双璇玑十四维与各专家状态（供前端雷达图坐标）
#[derive(Debug, Clone, Deserialize)]
struct XuanjiPublishRequest {
    /// 业务蓝图（支持前端友好的 {type,params} 风格，handler 内归一化为 FlowGraph）
    flow: serde_json::Value,
    name: Option<String>,
    description: Option<String>,
    requirement: Option<String>,
    tags: Option<Vec<String>>,
    /// I-05 双验收联动：需求侧任务是否 Done（与融合侧璇玑验证共同决定可否上架）
    #[serde(default)]
    task_done: Option<bool>,
    /// 来源任务 ID（双璇玑任务闭环，I-07 追溯）
    #[serde(default)]
    task_id: Option<String>,
}

async fn xuanji_health() -> Json<serde_json::Value> {
    let dims: Vec<&str> = vec![
        "Business",
        "Algorithm",
        "Permission",
        "Resource",
        "Security",
        "Data",
        "Observability",
        "ApiCompat",
        "Perf",
        "Maintain",
        "Test",
        "Style",
        "Cost",
        "Sensitive",
    ];
    Json(json!({
        "xuanji": "double-league-14-dim",
        "verification": "algo-verification-supreme",
        "business_league": ["Business","Algorithm","Permission","Resource","Security","Data","Observability"],
        "dev_league": ["ApiCompat","Perf","Maintain","Test","Style","Cost","Sensitive"],
        "dimensions": dims,
        "experts": [
            "business","algorithm","permission","resource","security","data","observability",
            "api_compat","perf","maintain","test","style","cost","sensitive"
        ],
    }))
}

async fn list_operators(State(state): State<Arc<AppState>>) -> Json<Vec<OperatorInfo>> {
    let ops = state.operators.lock().await;
    Json(ops.clone())
}

async fn register_operator(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterOperatorRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut ops = state.operators.lock().await;
    let mut custom = state.custom_operators.lock().await;

    let op_info = OperatorInfo {
        id: req.id.clone(),
        name: req.name.clone(),
        description: format!("自定义算子: {}", req.name),
        category: "custom".to_string(),
        input_type: "StateVector".to_string(),
        output_type: "StateVector".to_string(),
        parameters: req.parameters.unwrap_or(serde_json::json!({})),
    };

    ops.push(op_info.clone());
    custom.insert(
        req.id.clone(),
        CustomOperatorDef {
            id: req.id,
            name: req.name,
            operator_type: req.operator_type,
            parameters: op_info.parameters.clone(),
        },
    );

    (
        StatusCode::OK,
        Json(serde_json::json!({"success": true, "message": "算子注册成功", "operator": op_info})),
    )
}

// 内部复用：真正执行算子/工作流的核心逻辑，供 HTTP handler 与 MCP 兼容层共用
pub(crate) async fn run_workflow_inner(
    state: &Arc<AppState>,
    req: ExecuteRequest,
) -> ExecuteResponse {
    let start = std::time::Instant::now();
    let mut ctx = ExecutionContext::default();
    let input = StateVector::from_vec(req.input.clone());
    let input_norm = input.norm();
    let params = req.parameters.unwrap_or_default();

    // 安全加固：客户端可控 input 维度；linear 算子会构造 n×n 稠密矩阵（O(n²) 内存）。
    // 若不设上限，2MB 请求体（n≈25 万）即可触发数百 GB 分配 → OOM abort 进程。
    // 资源预检在算子构造之后才运行，无法拦截分配本身，故此处先行拒绝超限维度。
    const MAX_VECTOR_DIM: usize = 1024; // 1024² × 8B = 8MB，安全
    let max_dim = std::env::var("OUS_EXEC_MAX_DIM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(MAX_VECTOR_DIM);
    if input.dimension > max_dim {
        return ExecuteResponse {
            success: false,
            output: None,
            execution_time_ms: start.elapsed().as_millis() as u64,
            logs: vec![format!(
                "[scheduler] 输入维度 {} 超过上限 {}（阻止 O(n²) 矩阵分配导致 OOM）",
                input.dimension, max_dim
            )],
            error: Some(format!(
                "输入维度 {} 超过允许上限 {}，拒绝执行；可经 OUS_EXEC_MAX_DIM 调优",
                input.dimension, max_dim
            )),
            metrics: None,
        };
    }

    // 公理5（资源约束优化）接线：构造算子 DAG 做调度预检。
    // 每个算子按请求顺序建立串行依赖；预检通过后才进入真实执行。
    // 配额默认宽松（10^12 cycles / 10^12 B），可通过环境变量收紧：
    //   OUS_EXEC_MAX_CPU / OUS_EXEC_MAX_MEM（企业部署建议显式配置）。
    let mut dag_ops: Vec<std::sync::Arc<dyn Operator>> = Vec::new();
    for op_id in &req.workflow {
        let arc: std::sync::Arc<dyn Operator> = match op_id.as_str() {
            "identity" => std::sync::Arc::new(IdentityOperator::new(input.dimension)),
            "linear" => {
                let scale = params.get("scale").copied().unwrap_or(2.0);
                let n = input.dimension;
                std::sync::Arc::new(LinearOperator::new(
                    nalgebra::DMatrix::from_diagonal_element(n, n, scale),
                ))
            }
            "normalize" => std::sync::Arc::new(FunctionOperator::new(
                "normalize",
                |s: &StateVector, _ctx| {
                    let mut s = s.clone();
                    s.normalize();
                    Ok(s)
                },
            )),
            "normalize_l1" => std::sync::Arc::new(FunctionOperator::new(
                "normalize_l1",
                |s: &StateVector, _ctx| {
                    let mut s = s.clone();
                    s.normalize_probability();
                    Ok(s)
                },
            )),
            "relu" => {
                std::sync::Arc::new(FunctionOperator::new("relu", |s: &StateVector, _ctx| {
                    let mut result = s.clone();
                    for i in 0..result.dimension {
                        result[i] = result[i].max(0.0);
                    }
                    Ok(result)
                }))
            }
            "sigmoid" => {
                std::sync::Arc::new(FunctionOperator::new("sigmoid", |s: &StateVector, _ctx| {
                    let mut result = s.clone();
                    for i in 0..result.dimension {
                        result[i] = 1.0 / (1.0 + (-result[i]).exp());
                    }
                    Ok(result)
                }))
            }
            "tanh" => {
                std::sync::Arc::new(FunctionOperator::new("tanh", |s: &StateVector, _ctx| {
                    let mut result = s.clone();
                    for i in 0..result.dimension {
                        result[i] = result[i].tanh();
                    }
                    Ok(result)
                }))
            }
            "softmax" => {
                std::sync::Arc::new(FunctionOperator::new("softmax", |s: &StateVector, _ctx| {
                    let mut result = s.clone();
                    let max_val = (0..result.dimension)
                        .map(|i| result[i])
                        .fold(f64::NEG_INFINITY, f64::max);
                    let sum_exp: f64 = (0..result.dimension)
                        .map(|i| (result[i] - max_val).exp())
                        .sum();
                    for i in 0..result.dimension {
                        result[i] = (result[i] - max_val).exp() / sum_exp;
                    }
                    Ok(result)
                }))
            }
            "scale" => {
                let factor = params.get("factor").copied().unwrap_or(1.0);
                std::sync::Arc::new(FunctionOperator::new(
                    "scale",
                    move |s: &StateVector, _ctx| {
                        let mut result = s.clone();
                        for i in 0..result.dimension {
                            result[i] *= factor;
                        }
                        Ok(result)
                    },
                ))
            }
            _ => {
                return ExecuteResponse {
                    success: false,
                    output: None,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    logs: vec![],
                    error: Some(format!("未知算子: {}", op_id)),
                    metrics: None,
                };
            }
        };
        dag_ops.push(arc);
    }

    // 构建串行 DAG 并执行公理5 资源约束预检（拓扑有效 + 配额内）
    let mut dag = optimizer::OperatorDag::new();
    for (i, op) in dag_ops.iter().enumerate() {
        dag.add_operator(&req.workflow[i], op.clone());
        if i > 0 {
            if let Err(e) = dag.add_dependency(&req.workflow[i - 1], &req.workflow[i]) {
                return ExecuteResponse {
                    success: false,
                    output: None,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    logs: vec![],
                    error: Some(e),
                    metrics: None,
                };
            }
        }
    }
    if let Err(e) = dag.topological_order() {
        return ExecuteResponse {
            success: false,
            output: None,
            execution_time_ms: start.elapsed().as_millis() as u64,
            logs: vec![],
            error: Some(format!("调度预检失败（DAG 含环）: {}", e)),
            metrics: None,
        };
    }
    let max_cpu = std::env::var("OUS_EXEC_MAX_CPU")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000_000_000_000_u64);
    let max_mem = std::env::var("OUS_EXEC_MAX_MEM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000_000_000_000_u64);
    let scheduler = optimizer::ResourceOptimizer::new(max_cpu, max_mem);
    if !scheduler.check_resources(&dag_ops) {
        let cost = dag.estimated_resource_cost();
        return ExecuteResponse {
            success: false, output: None,
            execution_time_ms: start.elapsed().as_millis() as u64,
            logs: vec![], error: Some(format!(
                "公理5 资源约束预检失败：估算 CPU={} cycles / MEM={} B 超出配额 CPU={} / MEM={}（可用 OUS_EXEC_MAX_CPU/OUS_EXEC_MAX_MEM 调优）",
                cost.cpu_cycles, cost.memory_bytes, max_cpu, max_mem
            )), metrics: None,
        };
    }
    let est_ms = dag.estimated_execution_time();
    let est_cost = dag.estimated_resource_cost();
    let mut all_logs =
        vec![format!(
        "[scheduler] 公理5 预检通过: 关键路径={:?} 预估执行时间={}ms 资源成本=CPU {} / MEM {} B",
        dag.critical_path(), est_ms, est_cost.cpu_cycles, est_cost.memory_bytes
    )];

    let mut workflow = Workflow::new("ai-workflow");
    for op_id in &req.workflow {
        let result = match op_id.as_str() {
            "identity" => workflow.then(IdentityOperator::new(input.dimension)),
            "linear" => {
                let scale = params.get("scale").copied().unwrap_or(2.0);
                let n = input.dimension;
                let matrix = nalgebra::DMatrix::from_diagonal_element(n, n, scale);
                workflow.then(LinearOperator::new(matrix))
            }
            "normalize" => workflow.then(FunctionOperator::new(
                "normalize",
                |s: &StateVector, _ctx| {
                    let mut s = s.clone();
                    s.normalize();
                    Ok(s)
                },
            )),
            "normalize_l1" => workflow.then(FunctionOperator::new(
                "normalize_l1",
                |s: &StateVector, _ctx| {
                    let mut s = s.clone();
                    s.normalize_probability();
                    Ok(s)
                },
            )),
            "relu" => workflow.then(FunctionOperator::new("relu", |s: &StateVector, _ctx| {
                let mut result = s.clone();
                for i in 0..result.dimension {
                    result[i] = result[i].max(0.0);
                }
                Ok(result)
            })),
            "sigmoid" => {
                workflow.then(FunctionOperator::new("sigmoid", |s: &StateVector, _ctx| {
                    let mut result = s.clone();
                    for i in 0..result.dimension {
                        result[i] = 1.0 / (1.0 + (-result[i]).exp());
                    }
                    Ok(result)
                }))
            }
            "tanh" => workflow.then(FunctionOperator::new("tanh", |s: &StateVector, _ctx| {
                let mut result = s.clone();
                for i in 0..result.dimension {
                    result[i] = result[i].tanh();
                }
                Ok(result)
            })),
            "softmax" => {
                workflow.then(FunctionOperator::new("softmax", |s: &StateVector, _ctx| {
                    let mut result = s.clone();
                    let max_val = (0..result.dimension)
                        .map(|i| result[i])
                        .fold(f64::NEG_INFINITY, f64::max);
                    let sum_exp: f64 = (0..result.dimension)
                        .map(|i| (result[i] - max_val).exp())
                        .sum();
                    for i in 0..result.dimension {
                        result[i] = (result[i] - max_val).exp() / sum_exp;
                    }
                    Ok(result)
                }))
            }
            "scale" => {
                let factor = params.get("factor").copied().unwrap_or(1.0);
                workflow.then(FunctionOperator::new(
                    "scale",
                    move |s: &StateVector, _ctx| {
                        let mut result = s.clone();
                        for i in 0..result.dimension {
                            result[i] *= factor;
                        }
                        Ok(result)
                    },
                ))
            }
            _ => {
                return ExecuteResponse {
                    success: false,
                    output: None,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    logs: vec![],
                    error: Some(format!("未知算子: {}", op_id)),
                    metrics: None,
                };
            }
        };

        match result {
            Ok(w) => workflow = w,
            Err(e) => {
                return ExecuteResponse {
                    success: false,
                    output: None,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    logs: all_logs,
                    error: Some(e.to_string()),
                    metrics: None,
                };
            }
        }
    }

    match workflow.execute(&input, &mut ctx) {
        Ok(result) => {
            let output_norm = result
                .output_state
                .as_ref()
                .map(|s| s.norm())
                .unwrap_or(0.0);
            let l1_residual = result
                .output_state
                .as_ref()
                .map(|_| (input_norm - output_norm).abs())
                .unwrap_or(0.0);
            all_logs.extend(result.logs.clone());

            let mut logs = state.execution_logs.lock().await;
            logs.push(ExecutionLog {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
                operator_id: "workflow".to_string(),
                workflow: req.workflow.clone(),
                success: result.success,
                execution_time_ms: result.execution_time_ms,
                residual: result.residual,
                input_dim: input.dimension,
                output_dim: result
                    .output_state
                    .as_ref()
                    .map(|s| s.dimension)
                    .unwrap_or(0),
            });

            ExecuteResponse {
                success: result.success,
                output: result.output_state.map(|s| s.to_vec()),
                execution_time_ms: result.execution_time_ms,
                logs: all_logs,
                error: result.error,
                metrics: Some(ExecutionMetrics {
                    input_norm,
                    output_norm,
                    l1_residual,
                }),
            }
        }
        Err(e) => ExecuteResponse {
            success: false,
            output: None,
            execution_time_ms: start.elapsed().as_millis() as u64,
            logs: all_logs,
            error: Some(e.to_string()),
            metrics: None,
        },
    }
}

async fn execute_workflow(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecuteRequest>,
) -> Json<ExecuteResponse> {
    Json(run_workflow_inner(&state, req).await)
}

// ========== 知识图谱API ==========

async fn get_graph(State(state): State<Arc<AppState>>) -> Json<GraphData> {
    let kg = state.knowledge_graph.lock().await;
    let centrality = kg.centrality_metrics();
    let stats = kg.stats();

    let type_colors: HashMap<&str, &str> = [
        ("core", "#6366f1"),
        ("activation", "#f59e0b"),
        ("math", "#10b981"),
        ("signal", "#ef4444"),
        ("data", "#8b5cf6"),
        ("ai", "#ec4899"),
        ("graph", "#06b6d4"),
        ("optimizer", "#84cc16"),
        ("loss", "#f97316"),
        ("regularization", "#a855f7"),
        ("normalization", "#14b8a6"),
        ("custom", "#64748b"),
    ]
    .iter()
    .cloned()
    .collect();

    let nodes = kg
        .nodes()
        .iter()
        .map(|n| {
            let pr = centrality.pagerank.get(&n.id).copied().unwrap_or(0.0);
            let dc = centrality
                .degree_centrality
                .get(&n.id)
                .copied()
                .unwrap_or(0.0);
            NodeData {
                id: n.id.clone(),
                label: n.label.clone(),
                node_type: n.node_type.clone(),
                pagerank: pr,
                degree_centrality: dc,
                activation: n.activation,
                size: 20.0 + pr * 200.0,
                color: type_colors
                    .get(n.node_type.as_str())
                    .copied()
                    .unwrap_or("#64748b")
                    .to_string(),
            }
        })
        .collect();

    let edges = kg
        .edges()
        .iter()
        .map(|e| EdgeData {
            source: e.source.clone(),
            target: e.target.clone(),
            weight: e.weight,
            relation_type: e.relation_type.clone(),
        })
        .collect();

    Json(GraphData {
        nodes,
        edges,
        stats,
    })
}

async fn get_graph_stats(State(state): State<Arc<AppState>>) -> Json<GraphStats> {
    let kg = state.knowledge_graph.lock().await;
    Json(kg.stats())
}

async fn get_centrality(State(state): State<Arc<AppState>>) -> Json<CentralityMetrics> {
    let kg = state.knowledge_graph.lock().await;
    Json(kg.centrality_metrics())
}

async fn get_communities(State(state): State<Arc<AppState>>) -> Json<Vec<Community>> {
    let kg = state.knowledge_graph.lock().await;
    Json(kg.detect_communities(20))
}

async fn get_shortest_path(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PathQuery>,
) -> Json<Option<PathResult>> {
    let kg = state.knowledge_graph.lock().await;
    match kg.shortest_path(&query.source, &query.target) {
        Ok(path) => Json(path),
        Err(_) => Json(None),
    }
}

async fn get_pagerank(State(state): State<Arc<AppState>>) -> Json<HashMap<String, f64>> {
    let kg = state.knowledge_graph.lock().await;
    Json(kg.pagerank(30))
}

async fn get_neighbors(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<Vec<(String, f64, String)>> {
    let kg = state.knowledge_graph.lock().await;
    match kg.neighbors(&id) {
        Ok(neighbors) => Json(neighbors),
        Err(_) => Json(vec![]),
    }
}

async fn add_node(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddNodeRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut kg = state.knowledge_graph.lock().await;
    kg.add_node(KnowledgeNode {
        id: req.id.clone(),
        label: req.label,
        node_type: req.node_type.unwrap_or_else(|| "custom".to_string()),
        properties: req.properties.unwrap_or(serde_json::json!({})),
        embedding: None,
        activation: 0.0,
        metadata: HashMap::new(),
    });
    (
        StatusCode::OK,
        Json(serde_json::json!({"success": true, "id": req.id})),
    )
}

async fn add_edge(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddEdgeRequest>,
) -> StatusCode {
    let mut kg = state.knowledge_graph.lock().await;
    let _ = kg.add_edge(KnowledgeEdge {
        source: req.source,
        target: req.target,
        weight: req.weight,
        relation_type: req.relation_type.unwrap_or_else(|| "related".to_string()),
        properties: serde_json::json!({}),
    });
    StatusCode::OK
}

async fn propagate_activation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ActivationRequest>,
) -> Json<HashMap<String, f64>> {
    let mut kg = state.knowledge_graph.lock().await;
    Json(kg.propagate_activation(&req.start_nodes, req.iterations.unwrap_or(10)))
}

async fn recommend_nodes(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RecommendRequest>,
) -> Json<Vec<NodeRecommendation>> {
    let kg = state.knowledge_graph.lock().await;
    Json(kg.recommend(&req.context_nodes, req.limit.unwrap_or(10)))
}

// ============ 对话自动→知识图谱 自动整理 API ============

/// 统一搜索：对话内容 + 知识图谱节点
async fn graph_search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let q = match params.get("q") {
        Some(q) if !q.trim().is_empty() => q.clone(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"success": false, "error": "缺少查询参数 q"})),
            )
        }
    };
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(20);

    match state.ai_agent.dialogue_graph().search(&q, limit).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::to_value(&result).unwrap())),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": format!("搜索失败: {e}")})),
        ),
    }
}

/// 切换全自动同步开关
async fn toggle_auto_sync(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let enabled = payload
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    state.ai_agent.dialogue_graph().set_auto_sync(enabled).await;
    (
        StatusCode::OK,
        Json(serde_json::json!({ "auto_sync": enabled, "message": "已更新对话自动同步设置" })),
    )
}

/// 查询全自动同步状态
async fn auto_sync_status(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let enabled = state.ai_agent.dialogue_graph().is_auto_sync().await;
    (
        StatusCode::OK,
        Json(serde_json::json!({ "auto_sync": enabled })),
    )
}

/// 列出对话会话
async fn list_dialogue_sessions(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.ai_agent.dialogue_graph().list_sessions().await {
        Ok(sessions) => (
            StatusCode::OK,
            Json(serde_json::json!({ "sessions": sessions })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": format!("列出会话失败: {e}")})),
        ),
    }
}

/// 导出：对话 + 知识图谱 打包为单文件迁移包
async fn graph_export(State(state): State<Arc<AppState>>) -> (StatusCode, Json<serde_json::Value>) {
    match state.ai_agent.dialogue_graph().export_bundle().await {
        Ok(bundle) => (
            StatusCode::OK,
            Json(serde_json::to_value(&bundle).unwrap_or(serde_json::json!({}))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": format!("导出失败: {e}")})),
        ),
    }
}

/// 导入：从迁移包恢复对话 + 知识图谱（幂等合并）
async fn graph_import(
    State(state): State<Arc<AppState>>,
    Json(bundle): Json<ExportBundle>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.ai_agent.dialogue_graph().import_bundle(bundle).await {
        Ok(report) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "imported": {
                    "sessions": report.sessions,
                    "messages": report.messages,
                    "nodes": report.nodes,
                    "edges": report.edges,
                },
                "message": "导入完成，已自动优化布局"
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": format!("导入失败: {e}")})),
        ),
    }
}

async fn list_plugins(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    let pm = state.plugin_manager.lock().await;
    Json(pm.list())
}

async fn get_logs(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let logs = state.execution_logs.lock().await;
    Json(serde_json::json!({ "logs": *logs }))
}

/// GET /api/audit — RBAC 访问审计查询（admin / auditor 专属）。
///
/// 返回认证+授权两层产生的全部审计事件（放行 allowed / 拒绝 forbidden），
/// 每条均带 HMAC 签名，可独立验真（见 rbac_middleware::AuditEvent::verify_signature）。
async fn get_access_audit(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let events = state.audit.events();
    let total = state.audit.count();
    // 逐条验签（签名密钥与写入时一致；验真失败标记 tampered，供合规审计确认完整性）
    let verified: Vec<serde_json::Value> = events
        .iter()
        .map(|ev| {
            let ok = ev.verify_signature(&state.audit_key);
            let mut v = serde_json::to_value(ev).unwrap_or(serde_json::Value::Null);
            if let serde_json::Value::Object(ref mut m) = v {
                m.insert("signature_valid".to_string(), serde_json::Value::Bool(ok));
            }
            v
        })
        .collect();
    let tampered = verified
        .iter()
        .filter(|v| v.get("signature_valid").and_then(|b| b.as_bool()) == Some(false))
        .count();
    Json(serde_json::json!({ "audit": verified, "total": total, "tampered": tampered }))
}

// ========== AI智能对话API ==========

async fn ai_chat(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Json<ChatResponse> {
    let session_id = req
        .session_id
        .unwrap_or_else(|| format!("session-{}", &uuid::Uuid::new_v4().to_string()[..8]));

    // 调用AI对话
    let response = match state.ai_agent.chat(&session_id, &req.message).await {
        Ok(resp) => resp,
        Err(e) => ChatResponse {
            message: ai_agent::ChatMessage::assistant(format!("AI处理错误: {}", e)),
            suggestions: vec![],
            recommended_operators: vec![],
            actions: vec![],
            workflow_suggestion: None,
        },
    };

    // 保存会话历史
    {
        let mut sessions = state.chat_sessions.lock().await;
        let history = sessions.entry(session_id.clone()).or_insert_with(Vec::new);
        history.push(ai_agent::ChatMessage::user(&req.message));
        history.push(response.message.clone());
    }

    // 如果有工作流建议且包含可执行算子，记录
    if let Some(ref wf) = response.workflow_suggestion {
        tracing::info!("AI推荐工作流: {:?}", wf);
    }

    Json(response)
}

async fn get_chat_history(
    State(state): State<Arc<AppState>>,
    Path(session): Path<String>,
) -> Json<Vec<ai_agent::ChatMessage>> {
    let sessions = state.chat_sessions.lock().await;
    Json(sessions.get(&session).cloned().unwrap_or_default())
}

// ========== 草莓多平台：对话驱动系统生成 API ==========

/// 请求：把一句话需求编译成系统蓝图
#[derive(serde::Deserialize)]
struct CaomeiCompileRequest {
    requirement: String,
    name: Option<String>,
    tags: Option<Vec<String>>,
}

/// 请求：在已有蓝图基础上增量追加功能
#[derive(serde::Deserialize)]
struct CaomeiRefineRequest {
    blueprint_id: String,
    addition: String,
}

/// 对话 → 系统蓝图（功能点 + 关联关系 + 流程图）
async fn caomei_compile(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CaomeiCompileRequest>,
) -> Json<serde_json::Value> {
    let name = req.name.unwrap_or_else(|| "未命名系统".to_string());
    let tags = req.tags.unwrap_or_default();
    match state
        .ai_agent
        .compile_requirement(&req.requirement, &name, tags)
        .await
    {
        Ok(bp) => Json(serde_json::json!({
            "success": true,
            "blueprint_id": bp.id,
            "name": bp.name,
            "feature_count": bp.features.len(),
            "entities": bp.entities.keys().collect::<Vec<_>>(),
            "flow": bp.flow,
        })),
        Err(e) => Json(serde_json::json!({ "success": false, "error": e.to_string() })),
    }
}

/// 继续对话迭代：追加功能（"再加一个退货"）
async fn caomei_refine(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CaomeiRefineRequest>,
) -> Json<serde_json::Value> {
    match state
        .ai_agent
        .refine_blueprint(&req.blueprint_id, &req.addition)
        .await
    {
        Ok(bp) => Json(serde_json::json!({
            "success": true,
            "blueprint_id": bp.id,
            "feature_count": bp.features.len(),
            "flow": bp.flow,
        })),
        Err(e) => Json(serde_json::json!({ "success": false, "error": e.to_string() })),
    }
}

/// 列出系统模板市场（按域/关键词检索，支持"通用模块"复用）
async fn caomei_templates(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let domain = params.get("domain").cloned();
    let keyword = params.get("keyword").cloned();
    let market = state.market.index.lock().await;
    // MarketState 以 HashMap 存模板元信息，这里直接返回概览
    let total = market.len();
    Json(serde_json::json!({
        "success": true,
        "total": total,
        "domain_filter": domain,
        "keyword_filter": keyword,
        "hint": "系统模板市场索引可用，发布模板请用 POST /api/caomei/publish",
    }))
}

// ========== 算法分析归一化API ==========

// ========== 空间光速螺旋模型分析 API ==========

#[derive(Debug, Deserialize)]
struct SpiralAnalysisRequest {
    /// 曲率 κ
    curvature: f64,
    /// 挠率 τ
    torsion: f64,
    /// 「一周步长」h
    step_h: f64,
    /// 螺旋半径（可选）
    radius: Option<f64>,
    /// 切向速率，默认取真空光速 c
    speed: Option<f64>,
}

async fn analyze_spiral_handler(Json(req): Json<SpiralAnalysisRequest>) -> Json<serde_json::Value> {
    let consts = business_catalog::spiral::PhysicalConstants::default();
    let speed = req.speed.unwrap_or(consts.c);
    let params = SpiralParams {
        curvature: req.curvature,
        torsion: req.torsion,
        step_h: req.step_h,
        radius: req.radius,
    };
    let report = analyze_spiral(&params, speed, &consts);
    Json(serde_json::to_value(report).unwrap_or(serde_json::json!({"error": "序列化失败"})))
}

// ========== 算法分析 API ==========

async fn analyze_algorithm(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AnalyzeAlgorithmRequest>,
) -> Json<serde_json::Value> {
    let algo_type = match req.algorithm_type.as_deref() {
        Some("sorting") | Some("排序") => AlgorithmType::Sorting,
        Some("search") | Some("搜索") => AlgorithmType::Search,
        Some("graph") | Some("图") => AlgorithmType::Graph,
        Some("ml") | Some("machine_learning") | Some("机器学习") => {
            AlgorithmType::MachineLearning
        }
        Some("dl") | Some("deep_learning") | Some("深度学习") => AlgorithmType::DeepLearning,
        Some("optimization") | Some("优化") => AlgorithmType::Optimization,
        Some("signal") | Some("信号处理") => AlgorithmType::SignalProcessing,
        _ => AlgorithmType::Custom("general".to_string()),
    };

    match state.ai_agent.analyze_algorithm(&req.code, algo_type).await {
        Ok(flow) => {
            Json(serde_json::to_value(flow).unwrap_or(serde_json::json!({"error": "序列化失败"})))
        }
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

async fn list_algorithm_types() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "types": [
            {"id": "sorting", "name": "排序算法", "algorithms": ["快速排序", "归并排序", "堆排序", "冒泡排序"]},
            {"id": "search", "name": "搜索算法", "algorithms": ["二分查找", "广度优先", "深度优先", "A*"]},
            {"id": "graph", "name": "图算法", "algorithms": ["PageRank", "最短路径Dijkstra", "社区发现", "中心性"]},
            {"id": "machine_learning", "name": "机器学习", "algorithms": ["梯度下降", "线性回归", "随机森林"]},
            {"id": "deep_learning", "name": "深度学习", "algorithms": ["神经网络前向传播", "卷积", "自注意力", "Transformer"]},
            {"id": "optimization", "name": "优化算法", "algorithms": ["SGD", "Adam", "AdamW", "动量"]},
            {"id": "signal_processing", "name": "信号处理", "algorithms": ["FFT", "卷积", "小波变换", "DCT"]},
        ]
    }))
}

// ========== 全资源管理API ==========

async fn get_resources(State(state): State<Arc<AppState>>) -> Json<ResourcePanorama> {
    match state.ai_agent.get_resource_status().await {
        Ok(panorama) => Json(panorama),
        Err(_) => Json(ResourcePanorama {
            timestamp: chrono::Utc::now(),
            resources: HashMap::new(),
            active_plugins: 0,
            active_workflows: 0,
            cached_operators: 0,
            total_allocations: 0,
        }),
    }
}

async fn resource_health(State(state): State<Arc<AppState>>) -> Json<ResourceHealthReport> {
    let rm = state.ai_agent.resource_manager();
    let rm_guard = rm.read().await;
    Json(rm_guard.health_check())
}

// ========== 插件互通API ==========

async fn list_ai_plugins(State(state): State<Arc<AppState>>) -> Json<Vec<PluginInfo>> {
    let bus = state.ai_agent.plugin_bus();
    let bus_guard = bus.read().await;
    Json(bus_guard.list_plugins())
}

async fn register_ai_plugin(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterPluginRequest>,
) -> Json<serde_json::Value> {
    let plugin_type = match req.plugin_type.as_deref() {
        Some("wasm") => PluginType::Wasm,
        Some("external") => PluginType::External,
        Some("ai_model") => PluginType::AiModel,
        Some("datasource") => PluginType::DataSource,
        _ => PluginType::Custom,
    };

    let plugin = PluginInfo {
        id: req.id.clone(),
        name: req.name,
        version: "1.0.0".to_string(),
        plugin_type,
        capabilities: req.capabilities,
        input_topics: req.input_topics,
        output_topics: req.output_topics,
        status: PluginStatus::Active,
        metadata: HashMap::new(),
    };

    let bus = state.ai_agent.plugin_bus();
    let mut bus_guard = bus.write().await;

    match bus_guard.register(plugin) {
        Ok(()) => Json(
            serde_json::json!({"success": true, "message": "插件注册成功", "plugin_id": req.id}),
        ),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

async fn plugin_topology(State(state): State<Arc<AppState>>) -> Json<PluginTopology> {
    let bus = state.ai_agent.plugin_bus();
    let bus_guard = bus.read().await;
    Json(bus_guard.get_topology())
}

async fn send_plugin_message(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PluginMessageRequest>,
) -> Json<serde_json::Value> {
    let mut msg = ai_agent::PluginMessage::new(&req.source, &req.topic, req.payload);
    if let Some(target) = req.target {
        msg = msg.to_target(&target);
    }
    if req.need_response.unwrap_or(false) {
        msg = msg.need_response();
    }

    let bus = state.ai_agent.plugin_bus();
    let bus_guard = bus.read().await;

    match bus_guard.route_message(msg).await {
        Ok(Some(response)) => Json(serde_json::json!({"success": true, "response": response})),
        Ok(None) => Json(serde_json::json!({"success": true, "message": "消息已投递"})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

// ========== 业务流程自动化API ==========

async fn list_workflow_templates(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let engine = state.ai_agent.workflow_engine();
    let engine_guard = engine.read().await;
    let templates: Vec<_> = engine_guard
        .list_templates()
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id, "name": t.name, "description": t.description, "category": t.category
            })
        })
        .collect();
    Json(serde_json::json!({"templates": templates}))
}

async fn list_workflows(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let engine = state.ai_agent.workflow_engine();
    let engine_guard = engine.read().await;
    let saved = state.saved_workflows.lock().await;

    let mut wfs: Vec<_> = engine_guard.list_workflows().iter().map(|w| {
        serde_json::json!({"id": w.id, "name": w.name, "description": w.description, "nodes_count": w.nodes.len(), "is_template": false})
    }).collect();

    for w in saved.values() {
        wfs.push(serde_json::json!({"id": w.id, "name": w.name, "description": w.description, "nodes_count": w.nodes.len(), "is_saved": true}));
    }

    Json(serde_json::json!({"workflows": wfs}))
}

async fn execute_business_workflow(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WorkflowExecuteRequest>,
) -> Json<WorkflowResult> {
    // 如果指定了模板ID，从模板创建
    if let Some(template_id) = req.template_id {
        let engine = state.ai_agent.workflow_engine();
        let mut engine_guard = engine.write().await;
        match engine_guard.create_from_template(&template_id) {
            Ok(instance_id) => {
                if let Some(wf) = engine_guard.get_workflow(&instance_id).cloned() {
                    drop(engine_guard);
                    match state.ai_agent.execute_workflow(wf).await {
                        Ok(result) => return Json(result),
                        Err(e) => {
                            return Json(WorkflowResult {
                                instance: ai_agent::WorkflowInstance {
                                    id: "error".to_string(),
                                    workflow_id: template_id,
                                    status: ai_agent::WorkflowStatus::Failed,
                                    current_nodes: vec![],
                                    variables: HashMap::new(),
                                    node_executions: vec![],
                                    started_at: chrono::Utc::now(),
                                    completed_at: None,
                                },
                                final_output: None,
                                execution_log: vec![format!("执行错误: {}", e)],
                                metrics: Default::default(),
                            })
                        }
                    }
                }
            }
            Err(e) => {
                return Json(WorkflowResult {
                    instance: ai_agent::WorkflowInstance {
                        id: "error".to_string(),
                        workflow_id: template_id,
                        status: ai_agent::WorkflowStatus::Failed,
                        current_nodes: vec![],
                        variables: HashMap::new(),
                        node_executions: vec![],
                        started_at: chrono::Utc::now(),
                        completed_at: None,
                    },
                    final_output: None,
                    execution_log: vec![format!("模板错误: {}", e)],
                    metrics: Default::default(),
                });
            }
        }
    }

    // 执行自定义工作流
    if let Some(workflow) = req.workflow {
        match state.ai_agent.execute_workflow(workflow).await {
            Ok(result) => Json(result),
            Err(e) => Json(WorkflowResult {
                instance: ai_agent::WorkflowInstance {
                    id: "error".to_string(),
                    workflow_id: "custom".to_string(),
                    status: ai_agent::WorkflowStatus::Failed,
                    current_nodes: vec![],
                    variables: HashMap::new(),
                    node_executions: vec![],
                    started_at: chrono::Utc::now(),
                    completed_at: None,
                },
                final_output: None,
                execution_log: vec![format!("执行错误: {}", e)],
                metrics: Default::default(),
            }),
        }
    } else {
        Json(WorkflowResult {
            instance: ai_agent::WorkflowInstance {
                id: "error".to_string(),
                workflow_id: "none".to_string(),
                status: ai_agent::WorkflowStatus::Failed,
                current_nodes: vec![],
                variables: HashMap::new(),
                node_executions: vec![],
                started_at: chrono::Utc::now(),
                completed_at: None,
            },
            final_output: None,
            execution_log: vec!["请指定workflow或template_id".to_string()],
            metrics: Default::default(),
        })
    }
}

async fn save_workflow(
    State(state): State<Arc<AppState>>,
    Json(workflow): Json<BusinessWorkflow>,
) -> Json<serde_json::Value> {
    let mut saved = state.saved_workflows.lock().await;
    saved.insert(workflow.id.clone(), workflow.clone());
    Json(serde_json::json!({"success": true, "message": "工作流已保存", "id": workflow.id}))
}

async fn list_workflow_instances(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let engine = state.ai_agent.workflow_engine();
    let engine_guard = engine.read().await;
    let instances: Vec<_> = engine_guard
        .list_instances()
        .iter()
        .map(|i| {
            serde_json::json!({
                "id": i.id, "workflow_id": i.workflow_id, "status": format!("{:?}", i.status),
                "nodes_executed": i.node_executions.len(),
                "started_at": i.started_at.to_rfc3339(),
            })
        })
        .collect();
    Json(serde_json::json!({"instances": instances}))
}

// ========== 系统状态API ==========

async fn get_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let kg = state.knowledge_graph.lock().await;
    let pm = state.plugin_manager.lock().await;
    let ops = state.operators.lock().await;
    let logs = state.execution_logs.lock().await;
    let custom = state.custom_operators.lock().await;
    let stats = kg.stats();

    Json(serde_json::json!({
        "status": "running",
        "version": "3.0.0-ai-full-dimensional",
        "operators_count": ops.len(),
        "custom_operators_count": custom.len(),
        "plugins_count": pm.list().len(),
        "graph": {
            "nodes": kg.node_count(), "edges": kg.edge_count(),
            "density": stats.density, "clustering_coefficient": stats.clustering_coefficient,
            "communities": kg.detect_communities(10).len()
        },
        "executions_count": logs.len(),
        "success_rate": if logs.is_empty() { 100.0 } else {
            logs.iter().filter(|l| l.success).count() as f64 / logs.len() as f64 * 100.0
        },
        "ai_capabilities": [
            "ai_chat", "intent_recognition", "operator_recommendation",
            "algorithm_analysis", "flow_normalization", "complexity_analysis",
            "resource_management", "plugin_bus", "workflow_automation",
            "parallel_execution", "bpmn_engine"
        ]
    }))
}

async fn get_full_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let basic = get_status(State(state.clone())).await;
    let resources = get_resources(State(state.clone())).await;
    let health = resource_health(State(state.clone())).await;
    let plugins = list_ai_plugins(State(state.clone())).await;

    let engine = state.ai_agent.workflow_engine();
    let engine_guard = engine.read().await;
    let templates_count = engine_guard.list_templates().len();

    Json(serde_json::json!({
        "system": basic.0,
        "resources": resources.0,
        "health": health.0,
        "ai_plugins": plugins.0,
        "workflow_templates": templates_count,
        "modules": {
            "conversation": "active",
            "algorithm_analyzer": "active",
            "resource_manager": "active",
            "plugin_bus": "active",
            "workflow_engine": "active",
            "knowledge_graph": "active",
            "wasm_runtime": "active",
        }
    }))
}

// ========== LLM配置Handlers ==========
async fn get_llm_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let llm = state.ai_agent.llm_client();
    let client = llm.read().await;
    let config = client.get_config();
    Json(serde_json::json!({
        "api_base": config.api_base,
        "model": config.model,
        "temperature": config.temperature,
        "max_tokens": config.max_tokens,
        "enabled": config.enabled,
        "has_api_key": !config.api_key.is_empty()
    }))
}

async fn update_llm_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LLMConfigRequest>,
) -> Json<serde_json::Value> {
    let llm = state.ai_agent.llm_client();
    let mut client = llm.write().await;
    let mut config = client.get_config().clone();
    if let Some(v) = req.api_base {
        config.api_base = v;
    }
    if let Some(v) = req.api_key {
        config.api_key = v;
    }
    if let Some(v) = req.model {
        config.model = v;
    }
    if let Some(v) = req.temperature {
        config.temperature = v;
    }
    if let Some(v) = req.max_tokens {
        config.max_tokens = v;
    }
    if let Some(v) = req.enabled {
        config.enabled = v;
    }
    client.update_config(config.clone());
    Json(serde_json::json!({"success": true, "config": {
        "api_base": config.api_base,
        "model": config.model,
        "temperature": config.temperature,
        "enabled": config.enabled,
        "has_api_key": !config.api_key.is_empty()
    }}))
}

async fn test_llm_connection(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match state.ai_agent.test_llm_connection().await {
        Ok(result) => Json(serde_json::json!({"success": true, "result": result})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

// ========== 浏览器自动化Handlers ==========
async fn list_browser_templates(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let browser = state.ai_agent.browser();
    let b = browser.read().await;
    let templates: Vec<_> = b
        .list_task_templates()
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "name": t.name,
                "description": t.description,
                "has_start_url": t.start_url.is_some(),
                "step_count": t.steps.len(),
                "variables": t.variables.keys().collect::<Vec<_>>()
            })
        })
        .collect();
    Json(serde_json::json!({"templates": templates}))
}

async fn list_browser_sessions(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let browser = state.ai_agent.browser();
    let b = browser.read().await;
    let sessions: Vec<_> = b
        .list_sessions()
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "current_url": s.current_url,
                "title": s.title,
                "status": s.status,
                "action_count": s.action_log.len(),
                "started_at": s.started_at
            })
        })
        .collect();
    Json(serde_json::json!({"sessions": sessions}))
}

async fn get_browser_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let browser = state.ai_agent.browser();
    let b = browser.read().await;
    match b.get_session(&id) {
        Some(s) => Json(serde_json::json!(s)),
        None => Json(serde_json::json!({"error": "session not found"})),
    }
}

async fn close_browser_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let browser = state.ai_agent.browser();
    let mut b = browser.write().await;
    match b.close_session(&id) {
        Ok(()) => Json(serde_json::json!({"success": true})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

async fn execute_browser_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BrowserTaskRequest>,
) -> Json<serde_json::Value> {
    let task_id = req.task_id.unwrap_or_else(|| "web-search".to_string());
    let browser = state.ai_agent.browser();
    let mut b = browser.write().await;
    match b.execute_task(&task_id, req.variables).await {
        Ok(result) => Json(serde_json::json!(result)),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

async fn execute_browser_steps(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BrowserTaskRequest>,
) -> Json<serde_json::Value> {
    let steps = req.steps.unwrap_or_default();
    if steps.is_empty() {
        return Json(serde_json::json!({"success": false, "error": "no steps provided"}));
    }
    let browser = state.ai_agent.browser();
    let mut b = browser.write().await;
    match b.execute_custom_steps(steps, req.start_url).await {
        Ok(result) => Json(serde_json::json!(result)),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

async fn execute_browser_action(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BrowserActionRequest>,
) -> Json<serde_json::Value> {
    let browser = state.ai_agent.browser();
    let mut b = browser.write().await;
    match b.execute_action(&req.session_id, req.action).await {
        Ok(result) => Json(serde_json::json!(result)),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

async fn browser_natural(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BrowserNaturalRequest>,
) -> Json<serde_json::Value> {
    let (url, steps) = ai_agent::BrowserAutomationEngine::parse_natural_language(&req.prompt);
    if steps.is_empty() {
        return Json(serde_json::json!({
            "success": false,
            "error": "无法解析浏览器操作指令，请提供URL或明确的操作描述",
            "parsed_url": url
        }));
    }
    let browser = state.ai_agent.browser();
    let mut b = browser.write().await;
    match b.execute_custom_steps(steps, url).await {
        Ok(result) => {
            Json(serde_json::json!({"success": true, "result": result, "ai_parsed": true}))
        }
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

// ============ 流程图引擎API ============

#[derive(Debug, Deserialize)]
struct ValidateFlowRequest {
    flow: FlowDefinition,
}

#[derive(Debug, Deserialize)]
struct ExecuteFlowRequest {
    flow_id: String,
    input: Option<HashMap<String, serde_json::Value>>,
}

/// PUT /api/ai/flows/:id — 更新流程图（目标须已存在；校验规则与创建一致）
async fn update_flow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(flow): Json<FlowDefinition>,
) -> Json<serde_json::Value> {
    if flow.id != id {
        return Json(serde_json::json!({
            "success": false,
            "error": format!("路径 id（{}）与请求体 flow.id（{}）不一致", id, flow.id),
        }));
    }
    if let Err(e) = AIAgent::validate_flow(&flow) {
        return Json(serde_json::json!({"success": false, "error": format!("验证失败: {}", e)}));
    }
    match state.ai_agent.update_flow(flow).await {
        Ok(updated) => Json(serde_json::json!({"success": true, "flow": updated})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

async fn list_flows(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match state.ai_agent.list_flows().await {
        Ok(flows) => Json(serde_json::json!({"success": true, "flows": flows})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

async fn create_flow(
    State(state): State<Arc<AppState>>,
    Json(flow): Json<FlowDefinition>,
) -> Json<serde_json::Value> {
    // 先验证
    if let Err(e) = AIAgent::validate_flow(&flow) {
        return Json(serde_json::json!({"success": false, "error": format!("验证失败: {}", e)}));
    }
    match state.ai_agent.create_flow(flow).await {
        Ok(created) => Json(serde_json::json!({"success": true, "flow": created})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

async fn get_flow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.ai_agent.get_flow(&id).await {
        Ok(Some(flow)) => Json(serde_json::json!({"success": true, "flow": flow})),
        Ok(None) => Json(serde_json::json!({"success": false, "error": "流程图不存在"})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

async fn delete_flow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.ai_agent.delete_flow(&id).await {
        Ok(true) => Json(serde_json::json!({"success": true})),
        Ok(false) => Json(serde_json::json!({"success": false, "error": "流程图不存在"})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

async fn validate_flow(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ValidateFlowRequest>,
) -> Json<serde_json::Value> {
    match AIAgent::validate_flow(&req.flow) {
        Ok(()) => {
            Json(serde_json::json!({"success": true, "valid": true, "message": "流程图验证通过"}))
        }
        Err(e) => {
            Json(serde_json::json!({"success": true, "valid": false, "error": e.to_string()}))
        }
    }
}

async fn execute_flow(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecuteFlowRequest>,
) -> Json<serde_json::Value> {
    let input = req.input.unwrap_or_default();
    match state.ai_agent.execute_flow(&req.flow_id, input).await {
        Ok(result) => Json(serde_json::json!({
            "success": result.success,
            "result": result
        })),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

async fn list_flow_node_types() -> Json<serde_json::Value> {
    let node_types = vec![
        serde_json::json!({"type": "Start", "name": "开始节点", "description": "流程图起始点", "config_fields": []}),
        serde_json::json!({"type": "End", "name": "结束节点", "description": "流程图结束点", "config_fields": []}),
        serde_json::json!({"type": "LLM", "name": "AI大模型(智能节点)", "description": "调用真实LLM处理，支持指定Provider与fallback链", "config_fields": [
            {"name": "prompt", "label": "提示词模板", "type": "text", "placeholder": "{{input}} 会被替换为上游变量"},
            {"name": "provider", "label": "模型供应商(可选)", "type": "select", "options": ["", "deepseek", "openai", "qwen", "glm", "ollama"], "hint": "留空则走AI Gateway默认fallback链(deepseek→openai→qwen→glm→ollama)"},
            {"name": "model", "label": "模型名称(可选)", "type": "text", "placeholder": "如 deepseek-chat / gpt-4o / qwen-plus", "hint": "留空使用provider默认模型"},
            {"name": "temperature", "label": "温度(可选)", "type": "number", "placeholder": "0.7", "hint": "0=精确,1=发散"}
        ]}),
        serde_json::json!({"type": "Browser", "name": "浏览器操作", "description": "访问网页并获取内容", "config_fields": [
            {"name": "url", "label": "URL地址", "type": "text", "placeholder": "https://example.com"},
            {"name": "action", "label": "操作类型", "type": "select", "options": ["navigate", "search", "extract"]}
        ]}),
        serde_json::json!({"type": "HttpRequest", "name": "HTTP请求", "description": "发送HTTP请求", "config_fields": [
            {"name": "url", "label": "请求URL", "type": "text"},
            {"name": "method", "label": "请求方法", "type": "select", "options": ["GET", "POST"]},
            {"name": "body", "label": "请求体", "type": "textarea"}
        ]}),
        serde_json::json!({"type": "Condition", "name": "条件判断", "description": "根据条件分支执行", "config_fields": [
            {"name": "condition", "label": "条件表达式", "type": "text", "placeholder": "{{value}} > 10"}
        ]}),
        serde_json::json!({"type": "Transform", "name": "数据转换", "description": "格式化/转换数据", "config_fields": [
            {"name": "template", "label": "转换模板", "type": "textarea", "placeholder": "结果: {{input}}"}
        ]}),
        serde_json::json!({"type": "Script", "name": "自定义脚本", "description": "执行简单脚本", "config_fields": [
            {"name": "code", "label": "脚本代码", "type": "textarea", "placeholder": "print(\"hello\")"}
        ]}),
        serde_json::json!({"type": "DataInput", "name": "数据输入", "description": "定义/输出数据", "config_fields": [
            {"name": "value", "label": "数据值", "type": "textarea"}
        ]}),
        serde_json::json!({"type": "DataOutput", "name": "数据输出", "description": "输出数据节点", "config_fields": []}),
        serde_json::json!({"type": "Operator", "name": "算子执行", "description": "调用算子", "config_fields": [
            {"name": "operator", "label": "算子ID", "type": "text"}
        ]}),
        serde_json::json!({"type": "Parallel", "name": "并行执行", "description": "并行执行多个分支", "config_fields": [
            {"name": "branches", "label": "并行分支", "type": "textarea"}
        ]}),
    ];
    Json(serde_json::json!({"success": true, "node_types": node_types}))
}

// ============ MCP 兼容层实现（Model Context Protocol over JSON-RPC 2.0）============
// 把系统内「算子」与「AI 插件」统一暴露为标准 MCP tools，使任意开源 MCP 客户端
// （Claude Desktop / Cursor / Cline / 自研 Agent）可零改造调用本系统能力。

#[derive(Debug, Deserialize)]
struct McpRpcReq {
    #[serde(default)]
    #[allow(dead_code)] // 预留：JSON-RPC 2.0 协议版本字段，当前 MCP 层未读取
    jsonrpc: String,
    #[serde(default)]
    id: serde_json::Value,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct McpRpcRes {
    jsonrpc: String,
    id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpRpcErr>,
}

#[derive(Debug, Serialize)]
struct McpRpcErr {
    code: i32,
    message: String,
}

fn mcp_ok(id: serde_json::Value, result: serde_json::Value) -> Json<McpRpcRes> {
    Json(McpRpcRes {
        jsonrpc: "2.0".into(),
        id,
        result: Some(result),
        error: None,
    })
}
fn mcp_err(id: serde_json::Value, code: i32, message: String) -> Json<McpRpcRes> {
    Json(McpRpcRes {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(McpRpcErr { code, message }),
    })
}

/// 把算子元数据收敛为 JSON-Schema 风格的 inputSchema
fn operator_input_schema(op: &OperatorInfo) -> serde_json::Value {
    if op.parameters.is_object() && op.parameters.get("properties").is_some() {
        op.parameters.clone()
    } else {
        serde_json::json!({
            "type": "object",
            "properties": {
                "input": { "type": "array", "items": { "type": "number" }, "description": "算子输入向量（数字数组）" }
            },
            "required": ["input"]
        })
    }
}

fn operator_to_mcp_tool(op: &OperatorInfo) -> serde_json::Value {
    serde_json::json!({
        "name": format!("operator_{}", op.id.replace(['-', ' ', '/', '.'], "_")),
        "description": format!("[{}] {}", op.category, op.description),
        "inputSchema": operator_input_schema(op),
        "annotations": { "title": op.name, "source": "ous-operator", "operatorId": op.id }
    })
}

async fn handle_mcp_rpc(
    State(state): State<Arc<AppState>>,
    Json(req): Json<McpRpcReq>,
) -> Json<McpRpcRes> {
    let id = req.id.clone();
    match req.method.as_str() {
        "initialize" => mcp_ok(
            id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "operator-unified-system", "version": env!("CARGO_PKG_VERSION") }
            }),
        ),
        "ping" => mcp_ok(id, serde_json::json!({})),
        "tools/list" => {
            let ops = state.operators.lock().await;
            let mut tools: Vec<serde_json::Value> = ops.iter().map(operator_to_mcp_tool).collect();
            drop(ops);
            // AI 插件总线里的插件同样暴露为 MCP tool（兼容开源插件生态）
            let bus = state.ai_agent.plugin_bus();
            let bus_guard = bus.read().await;
            for p in bus_guard.list_plugins() {
                tools.push(serde_json::json!({
                    "name": format!("plugin_{}", p.id.replace(['-', ' ', '/', '.'], "_")),
                    "description": format!("[插件 {:?}] {}", p.plugin_type, p.name),
                    "inputSchema": serde_json::json!({
                        "type": "object",
                        "properties": { "message": { "type": "string", "description": "发送给插件的消息/topic" }, "payload": { "type": "object", "description": "消息载荷" } },
                        "required": ["message"]
                    }),
                    "annotations": { "title": p.name, "source": "ous-plugin", "pluginId": p.id }
                }));
            }
            drop(bus_guard);
            mcp_ok(id, serde_json::json!({ "tools": tools }))
        }
        "tools/call" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let arguments = req
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            if let Some(op_id) = name.strip_prefix("operator_") {
                let real_id = op_id.replace('_', "-");
                // 解析输入向量：支持 arguments.input 或 arguments 本身为数组
                let input: Vec<f64> =
                    if let Some(arr) = arguments.get("input").and_then(|v| v.as_array()) {
                        arr.iter().filter_map(|x| x.as_f64()).collect()
                    } else if let Some(arr) = arguments.as_array() {
                        arr.iter().filter_map(|x| x.as_f64()).collect()
                    } else {
                        vec![1.0, 2.0, 3.0]
                    };
                let params: Option<HashMap<String, f64>> =
                    arguments.get("input").and(None).or_else(|| {
                        // 提取顶层数字参数作为算子参数
                        let mut m = HashMap::new();
                        for (k, v) in arguments.as_object()? {
                            if let Some(f) = v.as_f64() {
                                m.insert(k.clone(), f);
                            }
                        }
                        Some(m)
                    });
                let exec_req = ExecuteRequest {
                    workflow: vec![real_id],
                    input,
                    parameters: params,
                };
                let resp = run_workflow_inner(&state, exec_req).await;
                if resp.success {
                    mcp_ok(
                        id,
                        serde_json::json!({
                            "content": [ { "type": "text", "text": serde_json::to_string_pretty(&resp).unwrap_or_default() } ],
                            "isError": false
                        }),
                    )
                } else {
                    mcp_ok(
                        id,
                        serde_json::json!({
                            "content": [ { "type": "text", "text": format!("执行失败: {}", resp.error.unwrap_or_default()) } ],
                            "isError": true
                        }),
                    )
                }
            } else if let Some(plg) = name.strip_prefix("plugin_") {
                let _real_id = plg.replace('_', "-");
                let message = arguments
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let payload = arguments
                    .get("payload")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));
                let bus = state.ai_agent.plugin_bus();
                let bus_guard = bus.read().await;
                let msg = ai_agent::PluginMessage::new("mcp-client", &message, payload);
                match bus_guard.route_message(msg).await {
                    Ok(Some(r)) => mcp_ok(
                        id,
                        serde_json::json!({
                            "content": [ { "type": "text", "text": serde_json::to_string_pretty(&r).unwrap_or_default() } ],
                            "isError": false
                        }),
                    ),
                    Ok(None) => mcp_ok(
                        id,
                        serde_json::json!({
                            "content": [ { "type": "text", "text": "消息已投递（无响应）" } ],
                            "isError": false
                        }),
                    ),
                    Err(e) => mcp_ok(
                        id,
                        serde_json::json!({
                            "content": [ { "type": "text", "text": format!("插件调用失败: {}", e) } ],
                            "isError": true
                        }),
                    ),
                }
            } else {
                mcp_err(id, -32602, format!("未知 tool: {}", name))
            }
        }
        _ => mcp_err(id, -32601, format!("方法不存在: {}", req.method)),
    }
}
