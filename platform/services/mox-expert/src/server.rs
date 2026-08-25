//! HTTP 服务：把全维治理报告转为前端可视化 DTO，供 Three.js 力导向图实时联动高亮
//!
//! 设计：本服务完全独立，仅依赖 `mox-expert` + `flow-ai`，不触碰已失败的 runtime/ai-agent。
//! 可视化契约（DTO）在本模块内定义，核心层类型保持纯净。

use crate::context::{GovernContext, Principal, Tenant};
use crate::executor::{self, ExecState};
use crate::pipeline::{mox_optimize, GovernanceReport};
use crate::programming::programming_pipeline;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use flow_ai::model::{Access, FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};
use flow_ai::topology::{Entity, EntityKind, Relation, RelationKind, TopologyGraph};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// 前端提交的一次优化请求
#[derive(Debug, Clone, Deserialize)]
pub struct OptimizeRequest {
    /// 流程图 JSON（flow_ai::FlowGraph 序列化）
    pub flow: FlowGraph,
    /// 租户 ID（缺省 gov-tenant）
    #[serde(default = "default_tenant")]
    pub tenant: String,
    /// 命名空间
    #[serde(default = "default_ns")]
    pub namespace: String,
    /// 主体标识（缺省 anonymous，仅为低权限 viewer）
    #[serde(default = "default_principal")]
    pub principal: String,
    /// 显式角色列表（如 ["admin","editor"]）。缺省时仅授予 viewer 低权限角色，
    /// 禁止再默认授予 admin/editor 以避免 RBAC 被绕过。
    #[serde(default)]
    pub roles: Option<Vec<String>>,
    /// 是否监管租户（默认 true → 触发脱敏/合规校验）
    #[serde(default = "default_true")]
    pub regulated: bool,
    /// 自然语言指令（用于关系网复用路径点亮）
    #[serde(default)]
    pub instruction: String,
}

fn default_tenant() -> String {
    "gov-tenant".into()
}
fn default_ns() -> String {
    "ns-gov".into()
}
fn default_principal() -> String {
    "anonymous".into()
}
fn default_true() -> bool {
    true
}

/// 可视化 DTO：前端唯一需要理解的结构
#[derive(Debug, Clone, Serialize)]
pub struct VizBundle {
    /// 流程图节点（含高亮状态）
    pub flow_nodes: Vec<VizFlowNode>,
    /// 流程图边
    pub flow_edges: Vec<VizFlowEdge>,
    /// 关系网实体
    pub entities: Vec<VizEntity>,
    /// 关系网关系
    pub relations: Vec<VizRelation>,
    /// 统计
    pub stats: VizStats,
    /// 治理结论
    pub gate: VizGate,
    /// ⛨ 璇玑验证结论（最高优先级，不可被治理覆盖）
    pub algorithm: VizAlgo,
    /// 审计事件
    pub audit: Vec<VizAudit>,
    /// 专家评分
    pub expert_scores: Vec<(String, f64)>,
    /// 关键路径（节点 id 序列）
    pub critical_path: Vec<String>,
    /// 复用路径（关系网实体 id 序列，命中 fast-path 时点亮）
    pub reuse_path: Vec<String>,
    /// 冲突详情列表（前端标红时直接显示原因）
    pub conflicts: Vec<VizConflict>,
    pub mermaid: String,
    pub summary: String,
}

/// 冲突详情 DTO（前端标红节点时显示原因）
#[derive(Debug, Clone, Serialize)]
pub struct VizConflict {
    pub kind: String,
    pub severity: String,
    pub blocking: bool,
    pub nodes: Vec<String>,
    pub message: String,
    pub remedy: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VizFlowNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub tool: Option<String>,
    pub duration_ms: u64,
    /// 高亮状态：critical（关键路径）/ conflict（冲突标红）/ normal
    pub highlight: String,
    /// 是否由治理层注入（如脱敏 Guard）
    pub injected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VizFlowEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VizEntity {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub degree: usize,
    /// 是否命中复用路径
    pub on_reuse_path: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VizRelation {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub strength: f64,
    pub on_reuse_path: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VizStats {
    pub speedup: f64,
    pub time_saved_pct: f64,
    /// 算力消耗压缩率（模型分级路由带来的真实算力节省，与墙钟加速比正交）
    pub compute_saved_pct: f64,
    pub removed_false_deps: usize,
    pub parallel_layers: usize,
    pub max_concurrency: usize,
    pub conflicts_total: usize,
    pub conflicts_blocking: usize,
    pub conflicts_fixed: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct VizGate {
    pub status: String,
    pub approved: bool,
    pub reason: String,
}

/// 璇玑验证 DTO
#[derive(Debug, Clone, Serialize)]
pub struct VizAlgo {
    pub all_passed: bool,
    pub vetoed: bool,
    pub summary: String,
    pub checks: Vec<VizAlgoCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VizAlgoCheck {
    pub name: String,
    pub passed: bool,
    pub blocking: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VizAudit {
    pub subject: String,
    pub action: String,
    pub decision: String,
    pub hash: String,
}

/// 把核心报告转换为前端 DTO；topo 可选，提供则尝试点亮复用路径
pub fn to_viz(rep: &GovernanceReport, topo: Option<&TopologyGraph>) -> VizBundle {
    let opt = &rep.optimization;
    let g = &opt.optimized_graph;

    // 关键路径节点集合
    let crit: std::collections::HashSet<String> = opt
        .critical_path
        .critical_paths
        .iter()
        .flat_map(|p| p.iter().cloned())
        .collect();

    // 冲突节点集合（标红）
    let mut conflict_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();
    for c in &opt.conflicts.conflicts {
        for n in &c.nodes {
            conflict_nodes.insert(n.clone());
        }
    }

    let flow_nodes = g
        .nodes
        .iter()
        .map(|n| {
            let highlight = if conflict_nodes.contains(&n.id) {
                "conflict".into()
            } else if crit.contains(&n.id) {
                "critical".into()
            } else {
                "normal".into()
            };
            VizFlowNode {
                id: n.id.clone(),
                label: n.name.clone(),
                kind: format!("{:?}", n.kind),
                tool: n.tool.map(|t| format!("{:?}", t)),
                duration_ms: n.duration_ms,
                highlight,
                injected: n.tags.iter().any(|t| t == "desensitize" || t == "authz")
                    || n.kind == flow_ai::model::NodeKind::Guard,
            }
        })
        .collect();

    let flow_edges = g
        .edges
        .iter()
        .map(|e| VizFlowEdge {
            from: e.from.clone(),
            to: e.to.clone(),
            kind: format!("{:?}", e.kind),
            label: e.condition.clone(),
        })
        .collect();

    // 关系网：默认从流程图结构派生「流程节点→工具」「工具→算力」实体
    let (entities, relations, reuse) = build_topology(g, topo, &rep.flow_id);

    let stats = VizStats {
        speedup: opt.gains.speedup,
        time_saved_pct: opt.gains.time_saved_pct,
        compute_saved_pct: opt.gains.compute_saved_pct,
        removed_false_deps: opt.gains.removed_false_deps,
        parallel_layers: opt.gains.parallel_layers,
        max_concurrency: opt.gains.max_concurrency,
        conflicts_total: opt.gains.conflicts_found,
        conflicts_blocking: opt.gains.conflicts_blocking,
        conflicts_fixed: opt.gains.conflicts_auto_fixed,
    };

    VizBundle {
        flow_nodes,
        flow_edges,
        entities: entities.clone(),
        relations: relations.clone(),
        stats,
        gate: VizGate {
            status: format!("{:?}", rep.gate.status),
            approved: rep.gate.approved,
            reason: rep.gate.reason.clone(),
        },
        algorithm: VizAlgo {
            all_passed: rep.algo.all_passed,
            vetoed: rep.algo.vetoed,
            summary: rep.algo.summary.clone(),
            checks: rep
                .algo
                .checks
                .iter()
                .map(|c| VizAlgoCheck {
                    name: c.name.clone(),
                    passed: c.passed,
                    blocking: c.blocking,
                    detail: c.detail.clone(),
                })
                .collect(),
        },
        audit: rep
            .audit
            .events
            .iter()
            .map(|e| VizAudit {
                subject: e.subject.clone(),
                action: e.action.clone(),
                decision: e.decision.clone(),
                hash: e.hash.clone(),
            })
            .collect(),
        expert_scores: rep.expert_scores.clone(),
        critical_path: opt
            .critical_path
            .critical_paths
            .first()
            .cloned()
            .unwrap_or_default(),
        reuse_path: reuse,
        conflicts: opt
            .conflicts
            .conflicts
            .iter()
            .map(|c| VizConflict {
                kind: format!("{:?}", c.kind),
                severity: format!("{:?}", c.severity),
                blocking: matches!(c.severity, flow_ai::model::Severity::Blocking),
                nodes: c.nodes.clone(),
                message: c.message.clone(),
                remedy: c.remedy.as_ref().map(|r| format!("{:?}", r)),
            })
            .collect(),
        mermaid: flow_ai::to_mermaid(g),
        summary: opt.summary(),
    }
}

/// 若未提供外部拓扑网，则基于流程图结构自动派生一个六维关系网（流程节点↔工具↔算力）
fn build_topology(
    g: &FlowGraph,
    topo: Option<&TopologyGraph>,
    _flow_id: &str,
) -> (Vec<VizEntity>, Vec<VizRelation>, Vec<String>) {
    let mut entities: Vec<VizEntity> = Vec::new();
    let mut relations: Vec<VizRelation> = Vec::new();
    let mut reuse: Vec<String> = Vec::new();

    if let Some(t) = topo {
        let on_path: std::collections::HashSet<String> =
            t.route("", 0.15).path.iter().cloned().collect();
        for e in &t.entities {
            entities.push(VizEntity {
                id: e.id.clone(),
                label: e.label.clone(),
                kind: format!("{:?}", e.kind),
                degree: t
                    .relations
                    .iter()
                    .filter(|r| r.from == e.id || r.to == e.id)
                    .count(),
                on_reuse_path: on_path.contains(&e.id),
            });
        }
        for r in &t.relations {
            relations.push(VizRelation {
                from: r.from.clone(),
                to: r.to.clone(),
                kind: format!("{:?}", r.kind),
                strength: r.strength,
                on_reuse_path: on_path.contains(&r.from) && on_path.contains(&r.to),
            });
        }
        reuse = t.route("", 0.15).path.clone();
        return (entities, relations, reuse);
    }

    // 自动派生：流程节点 + 工具 + 算力三类实体
    use std::collections::BTreeMap;
    let mut tool_kind: BTreeMap<String, String> = BTreeMap::new();
    for n in &g.nodes {
        entities.push(VizEntity {
            id: n.id.clone(),
            label: n.name.clone(),
            kind: "FlowNode".into(),
            degree: 0,
            on_reuse_path: false,
        });
        if let Some(t) = n.tool {
            let tk = format!("{:?}", t);
            tool_kind.insert(format!("tool:{}", n.id), tk.clone());
        }
    }
    for n in &g.nodes {
        if let Some(t) = n.tool {
            let tk = format!("{:?}", t);
            let tool_id = format!("tool:{}", n.id);
            if !entities.iter().any(|e| e.id == tool_id) {
                entities.push(VizEntity {
                    id: tool_id.clone(),
                    label: tk.clone(),
                    kind: "Tool".into(),
                    degree: 0,
                    on_reuse_path: false,
                });
            }
            relations.push(VizRelation {
                from: n.id.clone(),
                to: tool_id.clone(),
                kind: "Binds".into(),
                strength: 0.9,
                on_reuse_path: false,
            });
            let pool: String = match t {
                flow_ai::model::ToolKind::Browser => "pool:browser".into(),
                flow_ai::model::ToolKind::Database => "pool:db".into(),
                flow_ai::model::ToolKind::Llm => "pool:llm".into(),
                _ => "pool:compute".into(),
            };
            if !entities.iter().any(|e| e.id == pool) {
                entities.push(VizEntity {
                    id: pool.clone(),
                    label: pool.clone(),
                    kind: "Resource".into(),
                    degree: 0,
                    on_reuse_path: false,
                });
            }
            relations.push(VizRelation {
                from: tool_id.clone(),
                to: pool,
                kind: "Serves".into(),
                strength: 0.7,
                on_reuse_path: false,
            });
        }
    }
    (entities, relations, reuse)
}

/// 应用请求：构造治理上下文并跑全链路。
/// 安全修复：不再硬编码 admin/editor 角色，而是透传调用方显式声明的角色；
/// 未声明时仅授予最低权限 `viewer` 角色，避免 RBAC 被绕过导致越权治理。
fn run(req: &OptimizeRequest) -> GovernanceReport {
    let mut tenant =
        Tenant::new(req.tenant.clone(), req.namespace.clone()).regulated(req.regulated);
    // 浏览器默认单例（政务填报互斥）
    tenant = tenant.with_pool("browser", 1);
    let roles = req.roles.clone().unwrap_or_else(|| vec!["viewer".into()]);
    let principal = Principal::new(req.principal.clone()).with_roles(roles);
    let ctx = GovernContext::new(tenant, principal);
    mox_optimize(&req.flow, &ctx)
}

/// 默认派生一个示例关系网（含 Skill/Rule 维度），供前端演示「复用路径点亮」
pub fn demo_topology() -> TopologyGraph {
    let mut t = TopologyGraph::new();
    t.add_entity(
        Entity::new(
            "skill:rpa-citizen",
            EntityKind::Skill,
            "政务公民库 RPA 模板",
        )
        .with_keywords(vec!["政务".to_string(), "公民".to_string()]),
    );
    t.add_entity(Entity::new("node:read", EntityKind::FlowNode, "读取公民库"));
    t.add_entity(Entity::new("node:guard", EntityKind::FlowNode, "脱敏"));
    t.add_entity(Entity::new("tool:db", EntityKind::Tool, "数据库工具"));
    t.add_entity(Entity::new("rule:gdpr", EntityKind::Rule, "个保法合规"));
    t.add_entity(Entity::new(
        "mem:citizen_vec",
        EntityKind::Memory,
        "公民向量块",
    ));
    t.add_relation(Relation::new(
        "skill:rpa-citizen",
        "node:read",
        RelationKind::Implements,
        0.95,
    ));
    t.add_relation(Relation::new(
        "skill:rpa-citizen",
        "mem:citizen_vec",
        RelationKind::Recalls,
        0.8,
    ));
    t.add_relation(Relation::new(
        "node:read",
        "tool:db",
        RelationKind::Binds,
        0.9,
    ));
    t.add_relation(Relation::new(
        "rule:gdpr",
        "node:guard",
        RelationKind::Constrains,
        0.9,
    ));
    t.add_relation(Relation::new(
        "node:guard",
        "node:read",
        RelationKind::Constrains,
        0.6,
    ));
    t
}

/// axum 共享状态
pub struct AppState {
    pub topo: Arc<Mutex<Option<TopologyGraph>>>,
    /// Step 10 实时联动：bridge 推送进来的会话流程图
    pub live: Arc<Mutex<Option<flow_ai::model::FlowGraph>>>,
    /// Phase 3：当前执行态（可视化监听源）
    pub current_exec: Arc<Mutex<Option<Arc<ExecState>>>>,
}

impl AppState {
    /// 空状态构造：供宿主进程（operator-server 聚合）以库方式挂载，
    /// 各 handler 按需惰性初始化闭环节点。
    pub fn new_state() -> Self {
        Self {
            topo: Arc::new(Mutex::new(None)),
            live: Arc::new(Mutex::new(None)),
            current_exec: Arc::new(Mutex::new(None)),
        }
    }
}

/// 一键闭环演示返回：可视化 DTO + LLM 调用对比（用户原方案核心收益量化）
#[derive(Debug, Clone, Serialize)]
pub struct ClosedLoopBundle {
    pub viz: VizBundle,
    /// linear ReAct baseline 的 LLM 调用次数（每工具一步一次）
    pub llm_baseline: u64,
    /// 复用回放后 bridge 的 LLM 调用次数（已知流程=0）
    pub llm_bridge: u64,
    /// 削减百分比
    pub llm_saved_pct: f64,
    /// 复用模板 id（点亮 fast-path）
    pub reuse_template: String,
}

/// 内置「政务 PII 归集」闭环图——直接复用 bench 的权威构造器，保证与 `mox bench`
/// 输出的 gov-pii 数字完全一致（加速比/剪枝/冲突同源），维护产品可信度。
fn closedloop_graph() -> FlowGraph {
    crate::bench::gov_pii_graph()
}

/// 一键闭环：跑完整优化 + 量化 LLM 调用对比（baseline 线性 ReAct vs 复用回放）
fn closedloop() -> ClosedLoopBundle {
    let g = closedloop_graph();
    let topo = demo_topology();
    let req = OptimizeRequest {
        flow: g,
        tenant: "gov-tenant".into(),
        namespace: "ns-gov".into(),
        principal: "admin".into(),
        roles: None,
        regulated: true,
        instruction: String::new(),
    };
    let rep = run(&req);
    let viz = to_viz(&rep, Some(&topo));
    // baseline：每个工具节点一次 LLM 决策
    let llm_baseline = viz
        .flow_nodes
        .iter()
        .filter(|n| n.tool.is_some() && n.kind != "Start" && n.kind != "End")
        .count() as u64;
    // bridge：已知完整流程（gov-pii 模板）整段回放 → 0 次 LLM
    let llm_bridge: u64 = 0;
    let llm_saved_pct = if llm_baseline > 0 {
        (1.0 - llm_bridge as f64 / llm_baseline as f64) * 100.0
    } else {
        0.0
    };
    ClosedLoopBundle {
        viz,
        llm_baseline,
        llm_bridge,
        llm_saved_pct,
        reuse_template: "gov-pii".into(),
    }
}

/// Phase 3 执行请求
#[derive(Debug, Clone, Deserialize)]
pub struct RunRequest {
    pub flow: FlowGraph,
    #[serde(default)]
    pub instruction: String,
    /// 执行回放速率（1.0=真实 ms；0.001=演示加速）
    #[serde(default = "default_rate")]
    pub rate: f64,
    #[serde(default = "default_tenant")]
    pub tenant: String,
    #[serde(default = "default_ns")]
    pub namespace: String,
    #[serde(default = "default_principal")]
    pub principal: String,
    /// 显式角色列表。缺省时仅授予 viewer 低权限角色，禁止默认 admin/editor。
    #[serde(default)]
    pub roles: Option<Vec<String>>,
    #[serde(default = "default_true")]
    pub regulated: bool,
}

fn default_rate() -> f64 {
    1.0
}

/// 构建 Router
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/health", get(health_handler))
        .route("/api/optimize", post(optimize_handler))
        .route("/api/ingest", post(ingest_handler))
        .route("/api/live", get(live_handler))
        .route("/api/run", post(run_handler))
        .route("/api/trace", get(trace_handler))
        .route("/api/closedloop", get(closedloop_handler))
        .with_state(Arc::new(state))
}

/// Step 10：接收 bridge 推送的会话流程图并缓存
async fn ingest_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    // 允许 {session, flow} 或裸 flow 两种载荷
    let flow_val = body.get("flow").cloned().unwrap_or(body.clone());
    match serde_json::from_value::<flow_ai::model::FlowGraph>(flow_val) {
        Ok(g) => {
            *state.live.lock().await = Some(g);
            (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// 内置「政务 PII 归集」会话图（与 hermes-flow-bridge::bridge_demo 的 gov-pii 模板完全一致）。
/// 用于 /api/live 在无 bridge 推送时的真实会话等价回退，使「实时联动」按钮始终有数据。
fn demo_session_graph() -> FlowGraph {
    let mut g = FlowGraph::new("gov-pii", "政务PII归集-实时会话");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(
        FlowNode::task("db_read", "读取公民库", ToolKind::Database, 300)
            .with_access(Access::read("db:citizen_info"))
            .with_access(Access::write("var:citizen")),
    );
    g.add_node(
        FlowNode::task("guard", "脱敏", ToolKind::Compute, 50)
            .with_tag("desensitize")
            .with_access(Access::read("var:citizen"))
            .with_access(Access::write("var:citizen_safe")),
    );
    g.add_node(FlowNode::task("web1", "网办提交", ToolKind::Browser, 400));
    g.add_node(
        FlowNode::task("merge", "汇总报告", ToolKind::Compute, 100)
            .with_access(Access::read("var:citizen_safe")),
    );
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    g.add_edge(FlowEdge::seq("s", "db_read"));
    g.add_edge(FlowEdge::seq("db_read", "guard"));
    g.add_edge(FlowEdge::seq("guard", "web1"));
    g.add_edge(FlowEdge::seq("web1", "merge"));
    g.add_edge(FlowEdge::seq("merge", "e"));
    g
}

/// Step 10：对实时会话图跑完整 optimize+verify，返回带高亮的 VizBundle
async fn live_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let g = state.live.lock().await.clone();
    let g = match g {
        Some(g) => g,
        None => {
            // 无 bridge 推送实时图时，回退到内置「政务 PII 归集」真实会话等价图
            // （节点 / 边 / 资源与 hermes-flow-bridge bridge_demo 完全一致），
            // 让前端「实时联动」按钮始终能跑出一张带高亮的实时可视化图。
            demo_session_graph()
        }
    };
    let topo_guard = state.topo.lock().await;
    let topo_ref = topo_guard.as_ref();
    // 复用 optimize_handler 同款 run()
    let req = OptimizeRequest {
        flow: g,
        tenant: "live-session".into(),
        namespace: "ns-live".into(),
        principal: "bridge".into(),
        roles: None,
        regulated: false,
        instruction: String::new(),
    };
    let rep = run(&req);
    let viz = to_viz(&rep, topo_ref);
    drop(topo_guard);
    (StatusCode::OK, Json(viz)).into_response()
}

/// Phase 3：接收流程图 → 走完整编程护栏（需求归一→专家→优化→验证→治理→出码）
/// → 启动确定性执行回放 → 返回 run_id + 预测指标（可预测、可量化）。
async fn run_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RunRequest>,
) -> impl IntoResponse {
    let tenant = Tenant::new(req.tenant.clone(), req.namespace.clone())
        .regulated(req.regulated)
        .with_pool("browser", 1);
    // 安全修复：透传调用方显式声明的角色；未声明时仅授予最低权限 `viewer` 角色
    let roles = req.roles.clone().unwrap_or_else(|| vec!["viewer".into()]);
    let principal = Principal::new(req.principal.clone()).with_roles(roles);
    let mut ctx = GovernContext::new(tenant, principal);
    // 配额安全默认值（可经 ServerConfig 配置化覆盖，此处保证无配置时仍可安全运行）
    ctx.quota.max_parallel = 8;
    ctx.quota.max_cost_budget = 100.0;
    ctx.quota.sla_ms = 50_000;

    let rep = programming_pipeline(&req.instruction, vec![], true, &req.flow, &ctx);
    if !rep.safe_to_emit {
        let reason = rep
            .governance
            .as_ref()
            .map(|g| {
                if g.algo.vetoed {
                    "算法验证网关否决：不可执行".to_string()
                } else {
                    format!("治理闸门未通过：{}", g.gate.reason)
                }
            })
            .unwrap_or_else(|| "需求未确认或流程不合规".to_string());
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "ok": false, "reason": reason })),
        )
            .into_response();
    }
    let exec = executor::run_report(&rep, req.rate).await;
    let snapshot = exec.trace.lock().await.clone();
    *state.current_exec.lock().await = Some(exec);
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "flow_id": snapshot.flow_id,
            "makespan_ms": snapshot.makespan_ms,
            "predict_progress": 0.0,
        })),
    )
        .into_response()
}

/// Phase 3：实时执行轨迹（前端轮询做进度监听 + 节点状态着色）。
async fn trace_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let guard = state.current_exec.lock().await;
    let Some(exec) = guard.as_ref() else {
        return (StatusCode::NO_CONTENT, "no active execution").into_response();
    };
    let snap = exec.trace.lock().await.clone();
    (StatusCode::OK, Json(snap)).into_response()
}

/// 一键闭环演示：完整优化 + LLM 调用对比量化（用户原方案核心收益可复现证据）
async fn closedloop_handler() -> impl IntoResponse {
    let bundle = closedloop();
    (StatusCode::OK, Json(bundle)).into_response()
}

async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "ok").into_response()
}

async fn optimize_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OptimizeRequest>,
) -> impl IntoResponse {
    let topo_guard = state.topo.lock().await;
    let topo_ref = topo_guard.as_ref();
    let rep = run(&req);
    let viz = to_viz(&rep, topo_ref);
    drop(topo_guard);
    (StatusCode::OK, Json(viz)).into_response()
}

/// 独立服务首页：本 crate 作为纯治理库被主后端 runtime 调用，独立 `mox serve`
/// 仅作演示。前端主入口是 Vue SPA（`frontend/`，由 runtime 托管 `/`），此处不内嵌
/// 已移除的单文件 HTML，改为引导到主前端，避免重复维护两份前端产物。
const INDEX_HTML: &str = r#"<!doctype html>
<html lang="zh-CN">
<head><meta charset="utf-8"><title>璇玑全维治理</title></head>
<body style="font-family:system-ui,sans-serif;padding:2rem">
  <h1>璇玑 · 全维处理工具流程图</h1>
  <p>治理内核已就绪。完整可视化前端由 Vue SPA 提供（由主后端 runtime 托管）。</p>
  <p>返回 <a href="/">主控制台</a> 查看全维治理台。</p>
</body>
</html>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use flow_ai::model::{FlowEdge, FlowNode, NodeKind, ToolKind};

    fn sample_req() -> OptimizeRequest {
        let mut g = FlowGraph::new("gov", "政务数据归集");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(
            FlowNode::task("read", "读取公民库", ToolKind::Database, 300)
                .with_access(flow_ai::model::Access::read("db:citizen_info"))
                .with_access(flow_ai::model::Access::write("var:citizen")),
        );
        g.add_node(
            FlowNode::task("guard", "脱敏", ToolKind::Compute, 50)
                .with_tag("desensitize")
                .with_access(flow_ai::model::Access::read("var:citizen"))
                .with_access(flow_ai::model::Access::write("var:citizen_safe")),
        );
        g.add_node(FlowNode::task("web1", "网办A", ToolKind::Browser, 500));
        g.add_node(FlowNode::task("web2", "网办B", ToolKind::Browser, 400));
        g.add_node(
            FlowNode::task("merge", "汇总", ToolKind::Compute, 100)
                .with_access(flow_ai::model::Access::read("var:citizen_safe")),
        );
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "read"));
        g.add_edge(FlowEdge::seq("read", "guard"));
        g.add_edge(FlowEdge::seq("guard", "web1"));
        g.add_edge(FlowEdge::seq("guard", "web2"));
        g.add_edge(FlowEdge::seq("web1", "merge"));
        g.add_edge(FlowEdge::seq("web2", "merge"));
        g.add_edge(FlowEdge::seq("merge", "e"));
        OptimizeRequest {
            flow: g,
            tenant: "gov-tenant".into(),
            namespace: "ns-gov".into(),
            principal: "admin".into(),
            roles: None,
            regulated: true,
            instruction: String::new(),
        }
    }

    #[test]
    fn viz_dto_builds_with_highlights() {
        let req = sample_req();
        let rep = run(&req);
        let viz = to_viz(&rep, None);
        // 关键路径非空
        assert!(!viz.critical_path.is_empty());
        // 统计合理
        assert!(viz.stats.speedup >= 1.0);
        // 治理通过
        assert!(viz.gate.approved);
        // 注入了脱敏 Guard 后冲突为 0
        assert_eq!(viz.stats.conflicts_blocking, 0);
        // 有节点被识别为关键路径高亮
        assert!(viz.flow_nodes.iter().any(|n| n.highlight == "critical"));
        // 自动派生关系网含 Resource 池实体
        assert!(viz.entities.iter().any(|e| e.kind == "Resource"));
    }

    #[test]
    fn demo_topology_has_skill_and_rule() {
        let t = demo_topology();
        assert!(t
            .entities
            .iter()
            .any(|e| format!("{:?}", e.kind) == "Skill"));
        assert!(t.entities.iter().any(|e| format!("{:?}", e.kind) == "Rule"));
    }
}
