// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 业务全景目录：把"系统所有业务"建模成流程图 + 六维关系网，
//! 并用璇玑（mox-expert）在运行中不断优化架构。
//!
//! 核心思想（与 Hermes / 璇玑架构一致）：
//! - **流程图是唯一需求源与开发产物**：每个业务 = 一张 `mox_ai_flow_svc::FlowGraph`，
//!   `tags` 携带 `dim:algo|perm|res|sec|data|obs` 做七维着色。
//! - **关系图是跨业务的六维知识网**：所有业务 flow 经 `TopologyGraph::ingest_flow`
//!   汇入同一张图，叠加 Skill/Rule/Memory/Model 实体与 Binds/Recalls/Constrains/Serves 关系。
//! - **使用中不断优化**：`record_hit`/`decay` 做动态权重学习；`impact_of` 做改一节点全链路
//!   同步；`route`/`shortest_path` 做跨业务复用最短路径（命中历史 Skill → 跳过完整 ReAct）。
//!
//! 【DIP 改造】本 crate 生产代码路径不再直接 `use mox_ai_expert_svc::pipeline`
//! （或 context/ir/... 等内部模块）。对外统一依赖：
//! - `mox_ai_expert_svc::expert_traits::{ExpertConsultant, ExpertRegistry, AllianceOrchestrator, ...}` 抽象 trait
//! - `mox_ai_expert_svc::types::{ConsultQuery, ConsultReport, ExpertMeta, ...}` 投影数据类型
//! - 需要「查询专家清单 / 注册专家」处统一用 `Arc<dyn ExpertRegistry>`。
//!
//! 从而实现依赖方向反转：`business-catalog → trait ← mox concrete`。

pub const CRATE_ID: &str = "62b2cca1-d98f-5e41-b26e-8d2a43966117";
pub const ENGINE_NAME: &str = "mox::business_catalog";
pub const CRATE_META: mox_platform_foundation::CrateMeta = mox_platform_foundation::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: mox_platform_foundation::AisLayer::L4Services,
    owner: "mox-core",
};

// ============================================================================
// DIP 依赖：仅引入 trait 与投影类型，不引入 mox_expert 内部 concrete struct。
// ============================================================================

use std::collections::HashMap;
use std::sync::Arc;
use mox_ai_expert_svc::expert_traits::{ExpertConsultant, ExpertRegistry};
use mox_ai_expert_svc::types::{ConsultQuery, ConsultReport, ExpertMeta};

use mox_ai_flow_svc::model::{
    Access, ExpertRule, FlowEdge, FlowGraph, FlowNode, NodeKind, Severity, ToolKind,
};
use mox_ai_flow_svc::topology::{Entity, EntityKind, Relation, RelationKind, TopologyGraph};

/// 空间光速螺旋模型分析算子（Frenet 螺旋运动学 + 量纲/数值诊断）
pub mod spiral;

/// 把业务配置（domain / regulated）编码成 ConsultQuery.ctx，供 ExpertServiceImpl 解析。
///
/// ctx 键与 `mox_ai_expert_svc::services::ExpertServiceImpl::consult_sync` 约定一致。
fn build_query(biz: &Business) -> ConsultQuery {
    let raw = (biz.build)();
    let mut ctx: HashMap<String, String> = HashMap::new();
    ctx.insert(
        "flow_json".into(),
        serde_json::to_string(&raw).unwrap_or_default(),
    );
    ctx.insert("tenant".into(), biz.domain.into());
    ctx.insert("namespace".into(), "ns".into());
    ctx.insert("principal".into(), "architect".into());
    ctx.insert("roles".into(), "admin".into());
    ctx.insert("pool_browser".into(), "1".into());
    ctx.insert(
        "regulated".into(),
        if biz.regulated {
            "true".into()
        } else {
            "false".into()
        },
    );
    ctx.insert("max_parallel".into(), "8".into());
    ctx.insert("max_cost_budget".into(), "100".into());
    ctx.insert("sla_ms".into(), "50000".into());
    ConsultQuery {
        id: biz.id.into(),
        query: biz.name.into(),
        ctx,
    }
}

// ---------------------------------------------------------------------------
// 构造辅助
// ---------------------------------------------------------------------------

/// Guard 节点（校验/脱敏/审计，无外部工具）
fn guard(id: &str, name: &str, ms: u64) -> FlowNode {
    FlowNode::new(id, name, NodeKind::Guard).with_duration(ms)
}
/// 节点构造辅助
fn start(id: &str) -> FlowNode {
    FlowNode::new(id, id, NodeKind::Start)
}
fn end(id: &str) -> FlowNode {
    FlowNode::new(id, id, NodeKind::End)
}
/// 给节点设耗时（FlowNode::new 默认 0）
trait WithDuration {
    fn with_duration(self, ms: u64) -> FlowNode;
}
impl WithDuration for FlowNode {
    fn with_duration(mut self, ms: u64) -> FlowNode {
        self.duration_ms = ms;
        self
    }
}
/// 合规规则构造辅助（基础版，无 required_guard_tags）
fn rule(id: &str, desc: &str, prefixes: &[&str]) -> ExpertRule {
    ExpertRule {
        id: id.into(),
        description: desc.into(),
        severity: Severity::Blocking,
        resource_prefixes: prefixes.iter().map(|s| s.to_string()).collect(),
        tool_kinds: Vec::new(),
        required_guard_tags: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// 业务实体
// ---------------------------------------------------------------------------

/// 一条业务 = (id, 名称, 域, 受监管?, 流程图构造器)
///
/// DIP：优化入口 `optimize` / `optimize_with` 不再直接依赖 `mox_ai_expert_svc::pipeline::mox_optimize`，
/// 而是走 `Arc<dyn ExpertConsultant>` trait；默认实现通过 `default_consultant()` 工厂注入。
pub struct Business {
    pub id: &'static str,
    pub name: &'static str,
    pub domain: &'static str,
    pub regulated: bool,
    pub build: fn() -> FlowGraph,
}

impl Business {
    /// 七维着色后交给璇玑优化（DIP 版：通过 ExpertConsultant trait，不出现 concrete struct）。
    ///
    /// 返回 `ConsultReport`（归一化投影报告：steps / score / vetoed），
    /// 替代此前直接暴露 `mox_ai_expert_svc::pipeline::GovernanceReport` 这一内部 concrete 类型。
    pub fn optimize(&self) -> ConsultReport {
        self.optimize_with(mox_ai_expert_svc::expert_traits::default_consultant())
    }
    /// 指定 consultant（DIP 证据：测试可替换 Mock 实现，无需真实璇玑引擎）。
    pub fn optimize_with(&self, consultant: Arc<dyn ExpertConsultant>) -> ConsultReport {
        let q = build_query(self);
        consultant
            .consult_blocking(&q)
            .unwrap_or_else(|e| ConsultReport {
                report_id: q.id.clone(),
                steps: vec![format!("[business-catalog] optimize 失败: {}", e)],
                score: 0.0,
                vetoed: true,
                reason: Some(format!("error: {}", e)),
            })
    }
}

/// 基于 Arc<dyn ExpertRegistry>（DIP）为每条业务注册其对应领域专家元信息。
///
/// 【归一化】架构层不再维护 "业务 ID → 专属关键词" 的硬编码 switch 表。
/// 专家元信息统一从 `Business` 自身字段（id / name / domain / regulated）泛化推导：
/// - 专家 id    → `biz-<id>`
/// - 专家名    → `<name>·领域专家`
/// - 能力集合  → `default_caps_for(&b)`（基于 域 + regulated flag 给出通用能力词，
///   不包含任何 政务/财务 等具体业务专属关键词）
///
/// 业务专属能力（政务的 pii/authz、财务的对账）
/// 由对应 `projects/business-*/` crate 自行 `registry.register(&custom_meta)` 外部注入，
/// 不再污染架构 business-catalog 源码。
pub async fn register_business_experts(
    registry: Arc<dyn ExpertRegistry>,
) -> mox_ai_expert_svc::types::Result<()> {
    for b in all_businesses() {
        let meta = ExpertMeta {
            id: format!("biz-{}", b.id),
            name: format!("{}·领域专家", b.name),
            domain: b.domain.into(),
            capabilities: default_caps_for(&b),
            description: format!("业务目录泛化注册 · 业务={}/{}", b.id, b.name),
            dimension: Some("Business".into()),
        };
        registry.register(&meta).await?;
    }
    Ok(())
}

/// 【归一化】架构级通用能力推导：禁止出现任何具体业务专属关键词
/// （pii / 对账 / 留痕 / 政务 … 一律不得写入此处）。
///
/// | 条件 | 注入能力（通用抽象） |
/// |---|---|
/// | `regulated=true` | compliance / permission / security（强监管三件套） |
/// | domain = data/gov    | data / governance / observability（数据治理类域） |
/// | domain = finance     | resource / data / reconciliation（资源 + 数据一致性） |
/// | domain = service     | knowledge / routing / observability（对话服务类域） |
/// | domain = integration/mcp | resource / plugin / permission（插件编排域） |
/// | domain = science/algo    | algorithm / validation / compliance（科学计算域） |
/// | 其它兜底             | business（通用业务） |
///
/// 业务专属能力在 `projects/business-*/src/lib.rs::expert_meta()` 中自声明并外部注入。
fn default_caps_for(b: &Business) -> Vec<String> {
    let mut caps: Vec<String> = Vec::new();

    if b.regulated {
        caps.extend(
            ["compliance", "permission", "security"]
                .iter()
                .map(|s| s.to_string()),
        );
    }

    match b.domain {
        "data" | "gov" => {
            caps.extend(
                ["data", "governance", "observability"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        "finance" => {
            caps.extend(
                ["resource", "data", "reconciliation"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        "service" => {
            caps.extend(
                ["knowledge", "routing", "observability"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        "integration" | "mcp" => {
            caps.extend(
                ["resource", "plugin", "permission"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        "science" | "algo" => {
            caps.extend(
                ["algorithm", "validation", "compliance"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        _ => {
            caps.push("business".into());
        }
    }

    caps.sort();
    caps.dedup();
    caps
}

// ---------------------------------------------------------------------------
// 业务一：政务数据归集（结构化脱敏 + 并行上报）
// ---------------------------------------------------------------------------
fn gov_pii() -> FlowGraph {
    let mut g = FlowGraph::new("gov-pii", "政务数据归集");
    g.add_node(start("s"));
    g.add_node(FlowNode::task("asr", "语音识别", ToolKind::Llm, 150));
    g.add_node(FlowNode::task("ic", "意图分类", ToolKind::Llm, 200).with_tag("dim:algo"));
    g.add_node(
        FlowNode::task("guard", "脱敏", ToolKind::Compute, 50)
            .with_tag("desensitize")
            .with_tag("dim:sec")
            .with_access(Access::read("var:citizen"))
            .with_access(Access::write("var:citizen_safe")),
    );
    g.add_node(
        guard("authz", "授权写库", 50)
            .with_tag("dim:perm")
            .with_tag("authz"),
    );
    g.add_node(
        FlowNode::task("db", "入库", ToolKind::Database, 300)
            .transactional(true)
            .with_tag("dim:data")
            .with_access(Access::read("var:citizen_safe"))
            .with_access(Access::write("db:citizen_info")),
    );
    g.add_node(
        FlowNode::task("web1", "门户上报", ToolKind::Browser, 400)
            .with_access(Access::read("var:citizen_safe")),
    );
    g.add_node(
        FlowNode::task("xls", "Excel 汇总", ToolKind::File, 200)
            .with_access(Access::read("db:citizen_info")),
    );
    g.add_node(
        FlowNode::task("merge", "合并", ToolKind::Compute, 100)
            .with_access(Access::read("var:citizen_safe")),
    );
    g.add_node(end("e"));
    g.rules.push(ExpertRule {
        id: "r-pii".into(),
        description: "政务 PII 必须脱敏并授权后入库".into(),
        severity: Severity::Blocking,
        resource_prefixes: vec!["var:".into(), "db:".into()],
        tool_kinds: vec![ToolKind::Database],
        required_guard_tags: vec!["desensitize".into(), "authorize".into()],
    });
    g.add_edge(FlowEdge::seq("s", "asr"));
    g.add_edge(FlowEdge::seq("asr", "ic"));
    g.add_edge(FlowEdge::seq("ic", "guard"));
    g.add_edge(FlowEdge::seq("guard", "authz"));
    g.add_edge(FlowEdge::seq("authz", "db"));
    g.add_edge(FlowEdge::seq("db", "web1"));
    g.add_edge(FlowEdge::seq("db", "xls"));
    g.add_edge(FlowEdge::seq("web1", "merge"));
    g.add_edge(FlowEdge::seq("xls", "merge"));
    g.add_edge(FlowEdge::seq("merge", "e"));
    g
}

// ---------------------------------------------------------------------------
// 业务三：财务对账（多源拉取 + 差异解释）
// ---------------------------------------------------------------------------
fn finance_reco() -> FlowGraph {
    let mut g = FlowGraph::new("finance", "财务对账");
    g.add_node(start("s"));
    g.add_node(FlowNode::task(
        "pull_a",
        "拉取A系统",
        ToolKind::Database,
        300,
    ));
    g.add_node(FlowNode::task(
        "pull_b",
        "拉取B系统",
        ToolKind::Database,
        300,
    ));
    g.add_node(
        FlowNode::task("diff", "比对差异", ToolKind::Compute, 200)
            .with_tag("dim:algo")
            .with_access(Access::read("var:a"))
            .with_access(Access::read("var:b")),
    );
    g.add_node(
        FlowNode::task("explain", "差异解释", ToolKind::Llm, 400)
            .with_tag("dim:algo")
            .with_access(Access::read("var:diff")),
    );
    g.add_node(
        guard("audit", "审计留痕", 80)
            .with_tag("dim:perm")
            .with_tag("transaction_check"),
    );
    g.add_node(end("e"));
    g.add_edge(FlowEdge::seq("s", "pull_a"));
    g.add_edge(FlowEdge::seq("s", "pull_b"));
    g.add_edge(FlowEdge::seq("pull_a", "diff"));
    g.add_edge(FlowEdge::seq("pull_b", "diff"));
    g.add_edge(FlowEdge::seq("diff", "explain"));
    g.add_edge(FlowEdge::seq("explain", "audit"));
    g.add_edge(FlowEdge::seq("audit", "e"));
    g
}

// ---------------------------------------------------------------------------
// 业务四：智能客服（意图路由 + 并行知识检索）
// ---------------------------------------------------------------------------
fn customer_bot() -> FlowGraph {
    let mut g = FlowGraph::new("bot", "智能客服");
    g.add_node(start("s"));
    g.add_node(FlowNode::task("asr", "语音识别", ToolKind::Llm, 150));
    g.add_node(FlowNode::task("ic", "意图分类", ToolKind::Llm, 120).with_tag("dim:algo"));
    g.add_node(
        FlowNode::task("kb", "知识库检索", ToolKind::Llm, 250)
            .with_tag("dim:data")
            .with_access(Access::read("mem:kb_vec")),
    );
    g.add_node(
        FlowNode::task("order", "下单", ToolKind::Http, 200)
            .with_tag("dim:res")
            .with_access(Access::write("var:order")),
    );
    g.add_node(
        FlowNode::task("reply", "生成回复", ToolKind::Llm, 300)
            .with_tag("dim:obs")
            .with_access(Access::read("mem:kb_vec")),
    );
    g.add_node(end("e"));
    g.add_edge(FlowEdge::seq("s", "asr"));
    g.add_edge(FlowEdge::seq("asr", "ic"));
    g.add_edge(FlowEdge::seq("ic", "kb"));
    g.add_edge(FlowEdge::seq("ic", "order"));
    g.add_edge(FlowEdge::seq("kb", "reply"));
    g.add_edge(FlowEdge::seq("order", "reply"));
    g.add_edge(FlowEdge::seq("reply", "e"));
    g
}

// ---------------------------------------------------------------------------
// 业务五：ETL 归集管道（多源抽取 + 字段映射）
// ---------------------------------------------------------------------------
fn etl() -> FlowGraph {
    let mut g = FlowGraph::new("etl", "ETL归集管道");
    g.add_node(start("s"));
    g.add_node(FlowNode::task(
        "ingest",
        "接入数据源",
        ToolKind::Database,
        200,
    ));
    g.add_node(FlowNode::task("map", "字段映射", ToolKind::Llm, 250).with_tag("dim:algo"));
    g.add_node(FlowNode::task("parse", "解析", ToolKind::Compute, 150));
    g.add_node(FlowNode::task("transform", "转换", ToolKind::Compute, 200).with_tag("dim:data"));
    g.add_node(guard("validate", "校验", 120).with_tag("compliance"));
    g.add_node(
        FlowNode::task("sink", "落库", ToolKind::Database, 220)
            .transactional(true)
            .with_access(Access::read("var:out")),
    );
    g.add_node(end("e"));
    g.add_edge(FlowEdge::seq("s", "ingest"));
    g.add_edge(FlowEdge::seq("ingest", "map"));
    g.add_edge(FlowEdge::seq("map", "parse"));
    g.add_edge(FlowEdge::seq("parse", "transform"));
    g.add_edge(FlowEdge::seq("transform", "validate"));
    g.add_edge(FlowEdge::seq("validate", "sink"));
    g.add_edge(FlowEdge::seq("sink", "e"));
    g
}

// ---------------------------------------------------------------------------
// 业务六：MCP 第三方插件编排（兼容任意第三方插件）
// ---------------------------------------------------------------------------
fn mcp_orchestration() -> FlowGraph {
    let mut g = FlowGraph::new("mcp", "MCP插件编排");
    g.add_node(start("s"));
    g.add_node(
        FlowNode::task("discover", "发现插件", ToolKind::Http, 150)
            .with_tag("dim:res")
            .with_tag("mcp"),
    );
    g.add_node(guard("authz", "鉴权", 60).with_tag("dim:perm"));
    g.add_node(
        FlowNode::task("call_a", "调用插件A", ToolKind::Http, 300)
            .with_tag("mcp")
            .with_access(Access::read("var:ctx")),
    );
    g.add_node(
        FlowNode::task("call_b", "调用插件B", ToolKind::Http, 300)
            .with_tag("mcp")
            .with_access(Access::read("var:ctx")),
    );
    g.add_node(FlowNode::task(
        "aggregate",
        "聚合结果",
        ToolKind::Compute,
        120,
    ));
    g.add_node(end("e"));
    g.add_edge(FlowEdge::seq("s", "discover"));
    g.add_edge(FlowEdge::seq("discover", "authz"));
    g.add_edge(FlowEdge::seq("authz", "call_a"));
    g.add_edge(FlowEdge::seq("authz", "call_b"));
    g.add_edge(FlowEdge::seq("call_a", "aggregate"));
    g.add_edge(FlowEdge::seq("call_b", "aggregate"));
    g.add_edge(FlowEdge::seq("aggregate", "e"));
    g
}

// ---------------------------------------------------------------------------
// 业务七：空间光速螺旋模型分析（科学计算 + 量纲/数值诊断）
// ---------------------------------------------------------------------------
fn spiral_analysis() -> FlowGraph {
    let mut g = FlowGraph::new("spiral", "空间光速螺旋模型分析");
    g.add_node(start("s"));
    g.add_node(
        FlowNode::task("input", "参数校验", ToolKind::Compute, 80)
            .with_tag("dim:algo")
            .with_access(Access::read("var:spiral_params")),
    );
    g.add_node(
        FlowNode::task("kinematics", "螺旋运动学计算", ToolKind::Compute, 200).with_tag("dim:algo"),
    );
    g.add_node(
        FlowNode::task("dimcheck", "量纲自洽诊断", ToolKind::Compute, 150)
            .with_tag("dim:algo")
            .with_tag("compliance"),
    );
    g.add_node(
        FlowNode::task("numcheck", "数值巧合标注", ToolKind::Compute, 120).with_tag("dim:algo"),
    );
    g.add_node(
        FlowNode::task("report", "生成诊断报告", ToolKind::Compute, 100)
            .with_tag("dim:obs")
            .with_access(Access::write("var:spiral_report")),
    );
    g.add_node(end("e"));
    g.rules.push(rule(
        "r-spiral",
        "螺旋模型物理推论须经量纲校验方可对外发布",
        &["var:"],
    ));
    g.add_edge(FlowEdge::seq("s", "input"));
    g.add_edge(FlowEdge::seq("input", "kinematics"));
    g.add_edge(FlowEdge::seq("kinematics", "dimcheck"));
    g.add_edge(FlowEdge::seq("kinematics", "numcheck"));
    g.add_edge(FlowEdge::seq("dimcheck", "report"));
    g.add_edge(FlowEdge::seq("numcheck", "report"));
    g.add_edge(FlowEdge::seq("report", "e"));
    g
}

/// 全部业务目录
pub fn all_businesses() -> Vec<Business> {
    vec![
        Business {
            id: "gov-pii",
            name: "政务数据归集",
            domain: "gov",
            regulated: true,
            build: gov_pii,
        },
        Business {
            id: "finance",
            name: "财务对账",
            domain: "finance",
            regulated: true,
            build: finance_reco,
        },
        Business {
            id: "bot",
            name: "智能客服",
            domain: "service",
            regulated: false,
            build: customer_bot,
        },
        Business {
            id: "etl",
            name: "ETL归集管道",
            domain: "data",
            regulated: false,
            build: etl,
        },
        Business {
            id: "mcp",
            name: "MCP插件编排",
            domain: "integration",
            regulated: false,
            build: mcp_orchestration,
        },
        Business {
            id: "spiral",
            name: "空间光速螺旋模型分析",
            domain: "science",
            regulated: false,
            build: spiral_analysis,
        },
    ]
}

/// 跨业务共享的六维关系网：注入 Skill / Rule / Memory / Model 实体与关系
pub fn build_topology() -> TopologyGraph {
    let mut topo = TopologyGraph::new();
    topo.add_entity(
        Entity::new("model:hermes3", EntityKind::Model, "Hermes3 重模型")
            .with_cost(800)
            .with_keywords(["流程图", "代码", "重推理"]),
    );
    topo.add_entity(
        Entity::new("model:light", EntityKind::Model, "轻量模型")
            .with_cost(120)
            .with_keywords(["分类", "意图", "摘要"]),
    );
    for t in [
        "database", "browser", "file", "http", "shell", "llm", "compute", "guard",
    ] {
        topo.add_entity(
            Entity::new(format!("tool:{}", t), EntityKind::Tool, t.to_string()).with_keywords([t]),
        );
    }
    topo.add_entity(
        Entity::new("skill:desensitize", EntityKind::Skill, "脱敏模板")
            .with_keywords(["脱敏", "pii", "政务"])
            .with_cost(50),
    );
    topo.add_entity(
        Entity::new("skill:intent-route", EntityKind::Skill, "意图路由模板")
            .with_keywords(["意图", "分类", "路由", "客服"])
            .with_cost(120),
    );
    topo.add_entity(
        Entity::new("skill:etl-map", EntityKind::Skill, "ETL字段映射模板")
            .with_keywords(["etl", "映射", "抽取"])
            .with_cost(250),
    );
    topo.add_entity(
        Entity::new("skill:db-pull", EntityKind::Skill, "数据库拉取模板")
            .with_keywords(["数据库", "拉取", "对账"])
            .with_cost(300),
    );
    topo.add_entity(
        Entity::new("mem:kb_vec", EntityKind::Memory, "知识库向量")
            .with_keywords(["知识", "检索", "客服"]),
    );
    topo.add_entity(
        Entity::new("rule:pii", EntityKind::Rule, "PII 必须脱敏")
            .with_keywords(["pii", "脱敏", "政务"]),
    );
    topo.add_entity(
        Entity::new("flownode:start", EntityKind::FlowNode, "开始节点")
            .with_keywords(["流程", "节点", "start"]),
    );
    topo.add_entity(
        Entity::new("flownode:end", EntityKind::FlowNode, "结束节点")
            .with_keywords(["流程", "节点", "end"]),
    );
    topo.add_relation(Relation::new(
        "model:hermes3",
        "flow:gov-pii:ic",
        RelationKind::Serves,
        0.9,
    ));
    topo.add_relation(Relation::new(
        "model:light",
        "flow:bot:ic",
        RelationKind::Serves,
        0.95,
    ));
    topo.add_relation(Relation::new(
        "skill:desensitize",
        "flow:gov-pii:guard",
        RelationKind::Implements,
        1.0,
    ));
    topo.add_relation(Relation::new(
        "skill:intent-route",
        "flow:bot:ic",
        RelationKind::Implements,
        1.0,
    ));
    topo.add_relation(Relation::new(
        "skill:etl-map",
        "flow:etl:map",
        RelationKind::Implements,
        1.0,
    ));
    topo.add_relation(Relation::new(
        "skill:db-pull",
        "flow:finance:pull_a",
        RelationKind::Implements,
        1.0,
    ));
    topo.add_relation(Relation::new(
        "skill:intent-route",
        "mem:kb_vec",
        RelationKind::Recalls,
        0.8,
    ));
    topo.add_relation(Relation::new(
        "rule:pii",
        "flow:gov-pii:db",
        RelationKind::Constrains,
        1.0,
    ));
    topo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_builds_all_businesses() {
        let biz = all_businesses();
        assert_eq!(biz.len(), 6, "架构内置业务数应为 6");
        for b in &biz {
            let g = (b.build)();
            assert!(!g.nodes.is_empty(), "{} 应有节点", b.id);
            assert!(g.topo_order().is_ok(), "{} 应为 DAG", b.id);
        }
    }

    #[test]
    fn topology_has_six_dimension_entities() {
        let topo = build_topology();
        let kinds: std::collections::HashSet<_> = topo
            .entities
            .iter()
            .map(|e| format!("{:?}", e.kind))
            .collect();
        for k in ["Model", "Tool", "Skill", "Memory", "Rule", "FlowNode"] {
            assert!(kinds.iter().any(|x| x == k), "关系网应含 {} 维", k);
        }
    }

    // —— DIP 证据：业务 optimize() 可换 MockExpert 运行（不依赖 mox-expert concrete）——
    #[test]
    fn business_optimize_uses_mock_consultant_via_trait() {
        use async_trait::async_trait;
        struct MockAlwaysApproved;
        #[async_trait]
        impl mox_ai_expert_svc::expert_traits::ExpertConsultant for MockAlwaysApproved {
            async fn consult(
                &self,
                _q: &ConsultQuery,
            ) -> mox_ai_expert_svc::types::Result<ConsultReport> {
                unreachable!("sync 路径不进入 async consult")
            }
            fn consult_blocking(
                &self,
                q: &ConsultQuery,
            ) -> mox_ai_expert_svc::types::Result<ConsultReport> {
                Ok(ConsultReport {
                    report_id: q.id.clone(),
                    steps: vec!["[Mock] 已批准（无璇玑引擎）".into()],
                    score: 0.85,
                    vetoed: false,
                    reason: None,
                })
            }
        }
        let biz = &all_businesses()[0]; // gov-pii
        let rep = biz.optimize_with(Arc::new(MockAlwaysApproved));
        assert_eq!(rep.report_id, biz.id);
        assert!((rep.score - 0.85).abs() < 1e-9);
        assert!(!rep.vetoed);
    }

    #[tokio::test]
    async fn register_business_experts_runs_via_registry_trait() {
        // DIP 证据：生产路径 register_business_experts 只依赖 Arc<dyn ExpertRegistry>，
        // 使用默认注册表工厂（default_registry），不出现任何 concrete struct 名字。
        let reg = mox_ai_expert_svc::expert_traits::default_registry();
        register_business_experts(reg.clone()).await.unwrap();
        let all = reg.list(Some("gov")).await.unwrap();
        assert!(!all.is_empty(), "应注册 gov 领域专家");
        assert!(
            reg.find("biz-gov-pii").await.unwrap().is_some(),
            "应注册 gov-pii 的领域专家"
        );
    }
}
