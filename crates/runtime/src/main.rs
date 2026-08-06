//! # 算子统一系统运行时 v3.0 - AI驱动全维突破平台
//!
//! 集成五大核心能力：
//! 1. AI智能对话 - 自然语言交互、意图识别、算子推荐
//! 2. 算法分析归一化 - 最强算法流程图生成与标准化
//! 3. 全资源管理 - CPU/内存/插件/算子/工作流统一调度
//! 4. 插件互通总线 - 发布订阅/点对点/请求响应
//! 5. 业务流程自动化 - BPMN工作流驱动AI执行

use axum::{
    extract::{Query, State, Path},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
};
use operator_core::category::Workflow;
use operator_core::operator::{FunctionOperator, IdentityOperator, LinearOperator, Operator};
use operator_core::state::StateVector;
use operator_core::ExecutionContext;
use operator_graph::{
    CentralityMetrics, Community, GraphStats, KnowledgeEdge, KnowledgeGraph, KnowledgeGraphBuilder,
    KnowledgeNode, NodeRecommendation, PathResult,
};
use operator_wasm::WasmPluginManager;
use ai_agent::{
    AIAgent, ChatResponse, AlgorithmType,
    BusinessWorkflow, WorkflowResult, PluginInfo, PluginType, PluginStatus,
    ResourcePanorama, ResourceHealthReport, PluginTopology,
    BrowserAction,
    FlowDefinition, FlowExecutionResult, FlowNode, FlowEdge, NodeType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

/// 应用状态 - AI全维系统核心
struct AppState {
    // 原有组件
    operators: Mutex<Vec<OperatorInfo>>,
    knowledge_graph: Mutex<KnowledgeGraph>,
    plugin_manager: Mutex<WasmPluginManager>,
    execution_logs: Mutex<Vec<ExecutionLog>>,
    custom_operators: Mutex<HashMap<String, CustomOperatorDef>>,
    // AI智能体
    ai_agent: Arc<AIAgent>,
    // 会话存储
    chat_sessions: Mutex<HashMap<String, Vec<ai_agent::ChatMessage>>>,
    // 工作流存储
    saved_workflows: Mutex<HashMap<String, BusinessWorkflow>>,
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
    operator_core::init_logging();
    tracing::info!("🚀 启动算子统一系统 v3.0 - AI驱动全维突破平台...");

    // 初始化WASM插件管理器
    let mut plugin_manager = WasmPluginManager::new("./plugins");
    let _ = plugin_manager.load_all();

    // 构建超大规模知识图谱 - 算子关系网
    let kg = build_knowledge_graph();

    // 初始化AI智能体
    let ai_agent = Arc::new(AIAgent::new());

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
            capabilities: vec!["graph_query".to_string(), "recommend".to_string(), "centrality".to_string()],
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
        tracing::info!("已加载 {} 个工作流模板", engine_guard.list_templates().len());
    }

    // 初始化内置算子列表
    let operators = build_default_operators();

    let state = Arc::new(AppState {
        operators: Mutex::new(operators),
        knowledge_graph: Mutex::new(kg),
        plugin_manager: Mutex::new(plugin_manager),
        execution_logs: Mutex::new(Vec::new()),
        custom_operators: Mutex::new(HashMap::new()),
        ai_agent,
        chat_sessions: Mutex::new(HashMap::new()),
        saved_workflows: Mutex::new(HashMap::new()),
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
        // ========== AI智能对话API ==========
        .route("/api/ai/chat", post(ai_chat))
        .route("/api/ai/chat/history/:session", get(get_chat_history))
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
        .route("/api/ai/browser/execute-action", post(execute_browser_action))
        .route("/api/ai/browser/natural", post(browser_natural))
        .route("/api/ai/browser/sessions/:id", get(get_browser_session))
        .route("/api/ai/browser/sessions/:id", delete(close_browser_session))
        // ========== 流程图引擎API ==========
        .route("/api/ai/flows", get(list_flows))
        .route("/api/ai/flows", post(create_flow))
        .route("/api/ai/flows/:id", get(get_flow))
        .route("/api/ai/flows/:id", delete(delete_flow))
        .route("/api/ai/flows/validate", post(validate_flow))
        .route("/api/ai/flows/execute", post(execute_flow))
        .route("/api/ai/flows/node-types", get(list_flow_node_types))
        // ========== 系统API ==========
        .route("/api/plugins", get(list_plugins))
        .route("/api/logs", get(get_logs))
        .route("/api/status", get(get_status))
        .route("/api/status/full", get(get_full_status))
        // 静态前端
        .nest_service("/", ServeDir::new("./frontend/dist"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:3000";
    tracing::info!("📡 服务器监听在 http://{}", addr);
    println!("");
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  🚀 算子统一系统 v3.0 - AI驱动全维突破平台                    ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  🧠 AI智能对话  │ 意图识别 · 算子推荐 · 自然语言交互          ║");
    println!("║  📊 算法归一化  │ 9种算法模式 · 流程图生成 · 复杂度分析       ║");
    println!("║  💎 全资源管理  │ 8类资源 · LRU缓存 · 配额调度 · 健康监控     ║");
    println!("║  🔌 插件互通    │ 发布订阅 · 点对点 · 请求响应 · 内置4插件    ║");
    println!("║  🎯 流程自动化  │ BPMN引擎 · 5个模板 · 10种节点 · 并行分支   ║");
    println!("║  🤖 真实AI对接  │ OpenAI兼容API · LLM可配置 · 智能降级          ║");
    println!("║  🌐 浏览器自动化│ 自然语言驱动 · 5种预置任务 · 18种操作        ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  🕸️  知识图谱: 34+算子节点, 30+关系边                         ║");
    println!("║  🌐 访问地址: http://localhost:3000                            ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!("");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
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
        OperatorInfo { id: "identity".to_string(), name: "恒等算子".to_string(), description: "输出等于输入，用于测试和残差连接".to_string(), category: "core".to_string(), input_type: "StateVector".to_string(), output_type: "StateVector".to_string(), parameters: serde_json::json!({}) },
        OperatorInfo { id: "linear".to_string(), name: "线性变换算子".to_string(), description: "y = 2x，可配置缩放因子".to_string(), category: "core".to_string(), input_type: "StateVector".to_string(), output_type: "StateVector".to_string(), parameters: serde_json::json!({"scale": 2.0}) },
        OperatorInfo { id: "normalize".to_string(), name: "L2归一化算子".to_string(), description: "归一化到单位范数（欧几里得范数=1）".to_string(), category: "core".to_string(), input_type: "StateVector".to_string(), output_type: "StateVector".to_string(), parameters: serde_json::json!({}) },
        OperatorInfo { id: "normalize_l1".to_string(), name: "L1归一化算子".to_string(), description: "归一化到概率分布（L1范数=1）".to_string(), category: "core".to_string(), input_type: "StateVector".to_string(), output_type: "StateVector".to_string(), parameters: serde_json::json!({}) },
        OperatorInfo { id: "relu".to_string(), name: "ReLU激活算子".to_string(), description: "max(0, x)，整流线性单元".to_string(), category: "activation".to_string(), input_type: "StateVector".to_string(), output_type: "StateVector".to_string(), parameters: serde_json::json!({}) },
        OperatorInfo { id: "sigmoid".to_string(), name: "Sigmoid激活算子".to_string(), description: "1/(1+exp(-x))，S型激活函数".to_string(), category: "activation".to_string(), input_type: "StateVector".to_string(), output_type: "StateVector".to_string(), parameters: serde_json::json!({}) },
        OperatorInfo { id: "tanh".to_string(), name: "Tanh激活算子".to_string(), description: "双曲正切激活函数".to_string(), category: "activation".to_string(), input_type: "StateVector".to_string(), output_type: "StateVector".to_string(), parameters: serde_json::json!({}) },
        OperatorInfo { id: "softmax".to_string(), name: "Softmax算子".to_string(), description: "指数归一化，输出概率分布".to_string(), category: "activation".to_string(), input_type: "StateVector".to_string(), output_type: "StateVector".to_string(), parameters: serde_json::json!({}) },
        OperatorInfo { id: "scale".to_string(), name: "缩放算子".to_string(), description: "按指定因子缩放向量".to_string(), category: "math".to_string(), input_type: "StateVector".to_string(), output_type: "StateVector".to_string(), parameters: serde_json::json!({"factor": 1.0}) },
        OperatorInfo { id: "add_bias".to_string(), name: "偏置加算子".to_string(), description: "添加可学习偏置".to_string(), category: "math".to_string(), input_type: "StateVector".to_string(), output_type: "StateVector".to_string(), parameters: serde_json::json!({}) },
    ]
}

// ========== 基础API处理器 ==========

async fn health() -> &'static str {
    "OK - AI Operator System v3.0 Running - Full-Dimensional Breakthrough"
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

    (StatusCode::OK, Json(serde_json::json!({"success": true, "message": "算子注册成功", "operator": op_info})))
}

async fn execute_workflow(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecuteRequest>,
) -> Json<ExecuteResponse> {
    let start = std::time::Instant::now();
    let mut ctx = ExecutionContext::default();
    let input = StateVector::from_vec(req.input.clone());
    let input_norm = input.norm();
    let params = req.parameters.unwrap_or_default();

    let mut workflow = Workflow::new("ai-workflow");
    let mut all_logs = Vec::new();

    for op_id in &req.workflow {
        let result = match op_id.as_str() {
            "identity" => workflow.then(IdentityOperator::new(input.dimension)),
            "linear" => {
                let scale = params.get("scale").copied().unwrap_or(2.0);
                let n = input.dimension;
                let matrix = nalgebra::DMatrix::from_diagonal_element(n, n, scale);
                workflow.then(LinearOperator::new(matrix))
            }
            "normalize" => workflow.then(FunctionOperator::new("normalize", |s: &StateVector, _ctx| {
                let mut s = s.clone(); s.normalize(); Ok(s)
            })),
            "normalize_l1" => workflow.then(FunctionOperator::new("normalize_l1", |s: &StateVector, _ctx| {
                let mut s = s.clone(); s.normalize_probability(); Ok(s)
            })),
            "relu" => workflow.then(FunctionOperator::new("relu", |s: &StateVector, _ctx| {
                let mut result = s.clone();
                for i in 0..result.dimension { result[i] = result[i].max(0.0); }
                Ok(result)
            })),
            "sigmoid" => workflow.then(FunctionOperator::new("sigmoid", |s: &StateVector, _ctx| {
                let mut result = s.clone();
                for i in 0..result.dimension { result[i] = 1.0 / (1.0 + (-result[i]).exp()); }
                Ok(result)
            })),
            "tanh" => workflow.then(FunctionOperator::new("tanh", |s: &StateVector, _ctx| {
                let mut result = s.clone();
                for i in 0..result.dimension { result[i] = result[i].tanh(); }
                Ok(result)
            })),
            "softmax" => workflow.then(FunctionOperator::new("softmax", |s: &StateVector, _ctx| {
                let mut result = s.clone();
                let max_val = (0..result.dimension).map(|i| result[i]).fold(f64::NEG_INFINITY, f64::max);
                let sum_exp: f64 = (0..result.dimension).map(|i| (result[i] - max_val).exp()).sum();
                for i in 0..result.dimension { result[i] = (result[i] - max_val).exp() / sum_exp; }
                Ok(result)
            })),
            "scale" => {
                let factor = params.get("factor").copied().unwrap_or(1.0);
                workflow.then(FunctionOperator::new("scale", move |s: &StateVector, _ctx| {
                    let mut result = s.clone();
                    for i in 0..result.dimension { result[i] *= factor; }
                    Ok(result)
                }))
            }
            _ => {
                return Json(ExecuteResponse {
                    success: false, output: None,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    logs: vec![], error: Some(format!("未知算子: {}", op_id)), metrics: None,
                });
            }
        };

        match result {
            Ok(w) => workflow = w,
            Err(e) => {
                return Json(ExecuteResponse { success: false, output: None,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    logs: all_logs, error: Some(e.to_string()), metrics: None,
                });
            }
        }
    }

    match workflow.execute(&input, &mut ctx) {
        Ok(result) => {
            let output_norm = result.output_state.as_ref().map(|s| s.norm()).unwrap_or(0.0);
            let l1_residual = result.output_state.as_ref().map(|_| (input_norm - output_norm).abs()).unwrap_or(0.0);
            all_logs.extend(result.logs.clone());

            let mut logs = state.execution_logs.lock().await;
            logs.push(ExecutionLog {
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                operator_id: "workflow".to_string(), workflow: req.workflow.clone(),
                success: result.success, execution_time_ms: result.execution_time_ms,
                residual: result.residual, input_dim: input.dimension,
                output_dim: result.output_state.as_ref().map(|s| s.dimension).unwrap_or(0),
            });

            Json(ExecuteResponse {
                success: result.success, output: result.output_state.map(|s| s.to_vec()),
                execution_time_ms: result.execution_time_ms, logs: all_logs, error: result.error,
                metrics: Some(ExecutionMetrics { input_norm, output_norm, l1_residual }),
            })
        }
        Err(e) => Json(ExecuteResponse {
            success: false, output: None,
            execution_time_ms: start.elapsed().as_millis() as u64,
            logs: all_logs, error: Some(e.to_string()), metrics: None,
        }),
    }
}

// ========== 知识图谱API ==========

async fn get_graph(State(state): State<Arc<AppState>>) -> Json<GraphData> {
    let kg = state.knowledge_graph.lock().await;
    let centrality = kg.centrality_metrics();
    let stats = kg.stats();

    let type_colors: HashMap<&str, &str> = [
        ("core", "#6366f1"), ("activation", "#f59e0b"), ("math", "#10b981"),
        ("signal", "#ef4444"), ("data", "#8b5cf6"), ("ai", "#ec4899"),
        ("graph", "#06b6d4"), ("optimizer", "#84cc16"), ("loss", "#f97316"),
        ("regularization", "#a855f7"), ("normalization", "#14b8a6"), ("custom", "#64748b"),
    ].iter().cloned().collect();

    let nodes = kg.nodes().iter().map(|n| {
        let pr = centrality.pagerank.get(&n.id).copied().unwrap_or(0.0);
        let dc = centrality.degree_centrality.get(&n.id).copied().unwrap_or(0.0);
        NodeData {
            id: n.id.clone(), label: n.label.clone(), node_type: n.node_type.clone(),
            pagerank: pr, degree_centrality: dc, activation: n.activation,
            size: 20.0 + pr * 200.0,
            color: type_colors.get(n.node_type.as_str()).copied().unwrap_or("#64748b").to_string(),
        }
    }).collect();

    let edges = kg.edges().iter().map(|e| EdgeData {
        source: e.source.clone(), target: e.target.clone(),
        weight: e.weight, relation_type: e.relation_type.clone(),
    }).collect();

    Json(GraphData { nodes, edges, stats })
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

async fn get_shortest_path(State(state): State<Arc<AppState>>, Query(query): Query<PathQuery>) -> Json<Option<PathResult>> {
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

async fn get_neighbors(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Json<Vec<(String, f64, String)>> {
    let kg = state.knowledge_graph.lock().await;
    match kg.neighbors(&id) { Ok(neighbors) => Json(neighbors), Err(_) => Json(vec![]) }
}

async fn add_node(State(state): State<Arc<AppState>>, Json(req): Json<AddNodeRequest>) -> (StatusCode, Json<serde_json::Value>) {
    let mut kg = state.knowledge_graph.lock().await;
    kg.add_node(KnowledgeNode {
        id: req.id.clone(), label: req.label,
        node_type: req.node_type.unwrap_or_else(|| "custom".to_string()),
        properties: req.properties.unwrap_or(serde_json::json!({})),
        embedding: None, activation: 0.0, metadata: HashMap::new(),
    });
    (StatusCode::OK, Json(serde_json::json!({"success": true, "id": req.id})))
}

async fn add_edge(State(state): State<Arc<AppState>>, Json(req): Json<AddEdgeRequest>) -> StatusCode {
    let mut kg = state.knowledge_graph.lock().await;
    let _ = kg.add_edge(KnowledgeEdge {
        source: req.source, target: req.target, weight: req.weight,
        relation_type: req.relation_type.unwrap_or_else(|| "related".to_string()),
        properties: serde_json::json!({}),
    });
    StatusCode::OK
}

async fn propagate_activation(State(state): State<Arc<AppState>>, Json(req): Json<ActivationRequest>) -> Json<HashMap<String, f64>> {
    let mut kg = state.knowledge_graph.lock().await;
    Json(kg.propagate_activation(&req.start_nodes, req.iterations.unwrap_or(10)))
}

async fn recommend_nodes(State(state): State<Arc<AppState>>, Json(req): Json<RecommendRequest>) -> Json<Vec<NodeRecommendation>> {
    let kg = state.knowledge_graph.lock().await;
    Json(kg.recommend(&req.context_nodes, req.limit.unwrap_or(10)))
}

async fn list_plugins(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    let pm = state.plugin_manager.lock().await;
    Json(pm.list())
}

async fn get_logs(State(state): State<Arc<AppState>>) -> Json<Vec<ExecutionLog>> {
    let logs = state.execution_logs.lock().await;
    Json(logs.clone())
}

// ========== AI智能对话API ==========

async fn ai_chat(State(state): State<Arc<AppState>>, Json(req): Json<ChatRequest>) -> Json<ChatResponse> {
    let session_id = req.session_id.unwrap_or_else(|| format!("session-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()));

    // 调用AI对话
    let response = match state.ai_agent.chat(&session_id, &req.message).await {
        Ok(resp) => resp,
        Err(e) => ChatResponse {
            message: ai_agent::ChatMessage::assistant(format!("AI处理错误: {}", e)),
            suggestions: vec![], recommended_operators: vec![], actions: vec![], workflow_suggestion: None,
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

async fn get_chat_history(State(state): State<Arc<AppState>>, Path(session): Path<String>) -> Json<Vec<ai_agent::ChatMessage>> {
    let sessions = state.chat_sessions.lock().await;
    Json(sessions.get(&session).cloned().unwrap_or_default())
}

// ========== 算法分析归一化API ==========

async fn analyze_algorithm(State(state): State<Arc<AppState>>, Json(req): Json<AnalyzeAlgorithmRequest>) -> Json<serde_json::Value> {
    let algo_type = match req.algorithm_type.as_deref() {
        Some("sorting") | Some("排序") => AlgorithmType::Sorting,
        Some("search") | Some("搜索") => AlgorithmType::Search,
        Some("graph") | Some("图") => AlgorithmType::Graph,
        Some("ml") | Some("machine_learning") | Some("机器学习") => AlgorithmType::MachineLearning,
        Some("dl") | Some("deep_learning") | Some("深度学习") => AlgorithmType::DeepLearning,
        Some("optimization") | Some("优化") => AlgorithmType::Optimization,
        Some("signal") | Some("信号处理") => AlgorithmType::SignalProcessing,
        _ => AlgorithmType::Custom("general".to_string()),
    };

    match state.ai_agent.analyze_algorithm(&req.code, algo_type).await {
        Ok(flow) => Json(serde_json::to_value(flow).unwrap_or(serde_json::json!({"error": "序列化失败"}))),
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
            active_plugins: 0, active_workflows: 0, cached_operators: 0, total_allocations: 0,
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

async fn register_ai_plugin(State(state): State<Arc<AppState>>, Json(req): Json<RegisterPluginRequest>) -> Json<serde_json::Value> {
    let plugin_type = match req.plugin_type.as_deref() {
        Some("wasm") => PluginType::Wasm,
        Some("external") => PluginType::External,
        Some("ai_model") => PluginType::AiModel,
        Some("datasource") => PluginType::DataSource,
        _ => PluginType::Custom,
    };

    let plugin = PluginInfo {
        id: req.id.clone(), name: req.name, version: "1.0.0".to_string(),
        plugin_type, capabilities: req.capabilities,
        input_topics: req.input_topics, output_topics: req.output_topics,
        status: PluginStatus::Active, metadata: HashMap::new(),
    };

    let bus = state.ai_agent.plugin_bus();
    let mut bus_guard = bus.write().await;

    match bus_guard.register(plugin) {
        Ok(()) => Json(serde_json::json!({"success": true, "message": "插件注册成功", "plugin_id": req.id})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()})),
    }
}

async fn plugin_topology(State(state): State<Arc<AppState>>) -> Json<PluginTopology> {
    let bus = state.ai_agent.plugin_bus();
    let bus_guard = bus.read().await;
    Json(bus_guard.get_topology())
}

async fn send_plugin_message(State(state): State<Arc<AppState>>, Json(req): Json<PluginMessageRequest>) -> Json<serde_json::Value> {
    let mut msg = ai_agent::PluginMessage::new(&req.source, &req.topic, req.payload);
    if let Some(target) = req.target { msg = msg.to_target(&target); }
    if req.need_response.unwrap_or(false) { msg = msg.need_response(); }

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
    let templates: Vec<_> = engine_guard.list_templates().iter().map(|t| {
        serde_json::json!({
            "id": t.id, "name": t.name, "description": t.description, "category": t.category
        })
    }).collect();
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

async fn execute_business_workflow(State(state): State<Arc<AppState>>, Json(req): Json<WorkflowExecuteRequest>) -> Json<WorkflowResult> {
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
                        Err(e) => return Json(WorkflowResult {
                            instance: ai_agent::WorkflowInstance {
                                id: "error".to_string(), workflow_id: template_id,
                                status: ai_agent::WorkflowStatus::Failed,
                                current_nodes: vec![], variables: HashMap::new(),
                                node_executions: vec![], started_at: chrono::Utc::now(), completed_at: None,
                            },
                            final_output: None,
                            execution_log: vec![format!("执行错误: {}", e)],
                            metrics: Default::default(),
                        }),
                    }
                }
            }
            Err(e) => {
                return Json(WorkflowResult {
                    instance: ai_agent::WorkflowInstance {
                        id: "error".to_string(), workflow_id: template_id,
                        status: ai_agent::WorkflowStatus::Failed,
                        current_nodes: vec![], variables: HashMap::new(),
                        node_executions: vec![], started_at: chrono::Utc::now(), completed_at: None,
                    },
                    final_output: None, execution_log: vec![format!("模板错误: {}", e)], metrics: Default::default(),
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
                    id: "error".to_string(), workflow_id: "custom".to_string(),
                    status: ai_agent::WorkflowStatus::Failed,
                    current_nodes: vec![], variables: HashMap::new(),
                    node_executions: vec![], started_at: chrono::Utc::now(), completed_at: None,
                },
                final_output: None, execution_log: vec![format!("执行错误: {}", e)], metrics: Default::default(),
            }),
        }
    } else {
        Json(WorkflowResult {
            instance: ai_agent::WorkflowInstance {
                id: "error".to_string(), workflow_id: "none".to_string(),
                status: ai_agent::WorkflowStatus::Failed,
                current_nodes: vec![], variables: HashMap::new(),
                node_executions: vec![], started_at: chrono::Utc::now(), completed_at: None,
            },
            final_output: None,
            execution_log: vec!["请指定workflow或template_id".to_string()],
            metrics: Default::default(),
        })
    }
}

async fn save_workflow(State(state): State<Arc<AppState>>, Json(workflow): Json<BusinessWorkflow>) -> Json<serde_json::Value> {
    let mut saved = state.saved_workflows.lock().await;
    saved.insert(workflow.id.clone(), workflow.clone());
    Json(serde_json::json!({"success": true, "message": "工作流已保存", "id": workflow.id}))
}

async fn list_workflow_instances(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let engine = state.ai_agent.workflow_engine();
    let engine_guard = engine.read().await;
    let instances: Vec<_> = engine_guard.list_instances().iter().map(|i| {
        serde_json::json!({
            "id": i.id, "workflow_id": i.workflow_id, "status": format!("{:?}", i.status),
            "nodes_executed": i.node_executions.len(),
            "started_at": i.started_at.to_rfc3339(),
        })
    }).collect();
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

async fn update_llm_config(State(state): State<Arc<AppState>>, Json(req): Json<LLMConfigRequest>) -> Json<serde_json::Value> {
    let llm = state.ai_agent.llm_client();
    let mut client = llm.write().await;
    let mut config = client.get_config().clone();
    if let Some(v) = req.api_base { config.api_base = v; }
    if let Some(v) = req.api_key { config.api_key = v; }
    if let Some(v) = req.model { config.model = v; }
    if let Some(v) = req.temperature { config.temperature = v; }
    if let Some(v) = req.max_tokens { config.max_tokens = v; }
    if let Some(v) = req.enabled { config.enabled = v; }
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
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()}))
    }
}

// ========== 浏览器自动化Handlers ==========
async fn list_browser_templates(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let browser = state.ai_agent.browser();
    let b = browser.read().await;
    let templates: Vec<_> = b.list_task_templates().iter().map(|t| serde_json::json!({
        "id": t.id,
        "name": t.name,
        "description": t.description,
        "has_start_url": t.start_url.is_some(),
        "step_count": t.steps.len(),
        "variables": t.variables.keys().collect::<Vec<_>>()
    })).collect();
    Json(serde_json::json!({"templates": templates}))
}

async fn list_browser_sessions(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let browser = state.ai_agent.browser();
    let b = browser.read().await;
    let sessions: Vec<_> = b.list_sessions().iter().map(|s| serde_json::json!({
        "id": s.id,
        "current_url": s.current_url,
        "title": s.title,
        "status": s.status,
        "action_count": s.action_log.len(),
        "started_at": s.started_at
    })).collect();
    Json(serde_json::json!({"sessions": sessions}))
}

async fn get_browser_session(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Json<serde_json::Value> {
    let browser = state.ai_agent.browser();
    let b = browser.read().await;
    match b.get_session(&id) {
        Some(s) => Json(serde_json::json!(s)),
        None => Json(serde_json::json!({"error": "session not found"})),
    }
}

async fn close_browser_session(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Json<serde_json::Value> {
    let browser = state.ai_agent.browser();
    let mut b = browser.write().await;
    match b.close_session(&id) {
        Ok(()) => Json(serde_json::json!({"success": true})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()}))
    }
}

async fn execute_browser_task(State(state): State<Arc<AppState>>, Json(req): Json<BrowserTaskRequest>) -> Json<serde_json::Value> {
    let task_id = req.task_id.unwrap_or_else(|| "web-search".to_string());
    let browser = state.ai_agent.browser();
    let mut b = browser.write().await;
    match b.execute_task(&task_id, req.variables).await {
        Ok(result) => Json(serde_json::json!(result)),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()}))
    }
}

async fn execute_browser_steps(State(state): State<Arc<AppState>>, Json(req): Json<BrowserTaskRequest>) -> Json<serde_json::Value> {
    let steps = req.steps.unwrap_or_default();
    if steps.is_empty() {
        return Json(serde_json::json!({"success": false, "error": "no steps provided"}));
    }
    let browser = state.ai_agent.browser();
    let mut b = browser.write().await;
    match b.execute_custom_steps(steps, req.start_url).await {
        Ok(result) => Json(serde_json::json!(result)),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()}))
    }
}

async fn execute_browser_action(State(state): State<Arc<AppState>>, Json(req): Json<BrowserActionRequest>) -> Json<serde_json::Value> {
    let browser = state.ai_agent.browser();
    let mut b = browser.write().await;
    match b.execute_action(&req.session_id, req.action).await {
        Ok(result) => Json(serde_json::json!(result)),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()}))
    }
}

async fn browser_natural(State(state): State<Arc<AppState>>, Json(req): Json<BrowserNaturalRequest>) -> Json<serde_json::Value> {
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
        Ok(result) => Json(serde_json::json!({"success": true, "result": result, "ai_parsed": true})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()}))
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

#[derive(Debug, Deserialize)]
struct UpdateFlowRequest {
    flow: FlowDefinition,
}

async fn list_flows(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match state.ai_agent.list_flows().await {
        Ok(flows) => Json(serde_json::json!({"success": true, "flows": flows})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()}))
    }
}

async fn create_flow(State(state): State<Arc<AppState>>, Json(flow): Json<FlowDefinition>) -> Json<serde_json::Value> {
    // 先验证
    if let Err(e) = AIAgent::validate_flow(&flow) {
        return Json(serde_json::json!({"success": false, "error": format!("验证失败: {}", e)}));
    }
    match state.ai_agent.create_flow(flow).await {
        Ok(created) => Json(serde_json::json!({"success": true, "flow": created})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()}))
    }
}

async fn get_flow(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Json<serde_json::Value> {
    match state.ai_agent.get_flow(&id).await {
        Ok(Some(flow)) => Json(serde_json::json!({"success": true, "flow": flow})),
        Ok(None) => Json(serde_json::json!({"success": false, "error": "流程图不存在"})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()}))
    }
}

async fn delete_flow(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Json<serde_json::Value> {
    match state.ai_agent.delete_flow(&id).await {
        Ok(true) => Json(serde_json::json!({"success": true})),
        Ok(false) => Json(serde_json::json!({"success": false, "error": "流程图不存在"})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()}))
    }
}

async fn validate_flow(State(_state): State<Arc<AppState>>, Json(req): Json<ValidateFlowRequest>) -> Json<serde_json::Value> {
    match AIAgent::validate_flow(&req.flow) {
        Ok(()) => Json(serde_json::json!({"success": true, "valid": true, "message": "流程图验证通过"})),
        Err(e) => Json(serde_json::json!({"success": true, "valid": false, "error": e.to_string()}))
    }
}

async fn execute_flow(State(state): State<Arc<AppState>>, Json(req): Json<ExecuteFlowRequest>) -> Json<serde_json::Value> {
    let input = req.input.unwrap_or_default();
    match state.ai_agent.execute_flow(&req.flow_id, input).await {
        Ok(result) => Json(serde_json::json!({
            "success": result.success,
            "result": result
        })),
        Err(e) => Json(serde_json::json!({"success": false, "error": e.to_string()}))
    }
}

async fn list_flow_node_types() -> Json<serde_json::Value> {
    let node_types = vec![
        serde_json::json!({"type": "Start", "name": "开始节点", "description": "流程图起始点", "config_fields": []}),
        serde_json::json!({"type": "End", "name": "结束节点", "description": "流程图结束点", "config_fields": []}),
        serde_json::json!({"type": "LLM", "name": "AI大模型", "description": "调用LLM处理", "config_fields": [
            {"name": "prompt", "label": "提示词模板", "type": "text", "placeholder": "{{input}} 会被替换"},
            {"name": "model", "label": "模型名称", "type": "select", "options": ["gpt-3.5-turbo", "gpt-4", "deepseek-chat"]}
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
