//! 业务全景目录：把"系统所有业务"建模成流程图 + 六维关系网，
//! 并用开发专家联盟（expert-alliance）在运行中不断优化架构。
//!
//! 核心思想（与 Hermes / 璇玑架构一致）：
//! - **流程图是唯一需求源与开发产物**：每个业务 = 一张 `flow_ai::FlowGraph`，
//!   `tags` 携带 `dim:algo|perm|res|sec|data|obs` 做七维着色。
//! - **关系图是跨业务的六维知识网**：所有业务 flow 经 `TopologyGraph::ingest_flow`
//!   汇入同一张图，叠加 Skill/Rule/Memory/Model 实体与 Binds/Recalls/Constrains/Serves 关系。
//! - **使用中不断优化**：`record_hit`/`decay` 做动态权重学习；`impact_of` 做改一节点全链路
//!   同步；`route`/`shortest_path` 做跨业务复用最短路径（命中历史 Skill → 跳过完整 ReAct）。
//!
//! 本 crate 不重新发明并行化/冲突/验证算法，全部复用已验证的 flow-ai + expert-alliance 引擎。

use expert_alliance::context::{GovernContext, Principal, Tenant};
use expert_alliance::ir::auto_dimension;
use expert_alliance::pipeline::alliance_optimize;
use flow_ai::model::{
    Access, ExpertRule, FlowEdge, FlowGraph, FlowNode, NodeKind, Severity, ToolKind,
};
use flow_ai::topology::{Entity, EntityKind, Relation, RelationKind, TopologyGraph};

/// 空间光速螺旋模型分析算子（Frenet 螺旋运动学 + 量纲/数值诊断）
pub mod spiral;

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

/// 一条业务 = (id, 名称, 域, 受监管?, 流程图构造器)
pub struct Business {
    pub id: &'static str,
    pub name: &'static str,
    pub domain: &'static str,
    pub regulated: bool,
    pub build: fn() -> FlowGraph,
}

impl Business {
    /// 七维着色后交给开发专家联盟优化
    pub fn optimize(&self) -> expert_alliance::pipeline::GovernanceReport {
        let raw = (self.build)();
        // 七维着色：把 tags 中的 dim:* 映射到业务/算法/权限/资源/安全/数据/可观测
        let _df = auto_dimension(&raw);
        let tenant = Tenant::new(self.domain, "ns")
            .regulated(self.regulated)
            .with_pool("browser", 1);
        let principal = Principal::new("architect").with_roles(vec!["admin".to_string()]);
        let mut ctx = GovernContext::new(tenant, principal);
        // 真实租户配额：政务/金融等强合规场景允许更高算力预算与 SLA（产品按租户配置）
        ctx.quota = expert_alliance::context::ResourceQuota {
            max_parallel: 8,
            max_cost_budget: 100.0,
            sla_ms: 50_000,
        };
        alliance_optimize(&raw, &ctx)
    }
}

// ---------------------------------------------------------------------------
// 业务一：政务数据归集（结构化脱敏 + 并行上报）
// ---------------------------------------------------------------------------
fn gov_pii() -> FlowGraph {
    let mut g = FlowGraph::new("gov-pii", "政务数据归集");
    g.add_node(start("s"));
    g.add_node(FlowNode::task("asr", "语音识别", ToolKind::Llm, 150));
    g.add_node(
        FlowNode::task("ic", "意图分类", ToolKind::Llm, 200)
            .with_tag("dim:algo"),
    );
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
// 业务二：法院文书生成（卷宗 OCR + 要素抽取 + 文书起草）
// ---------------------------------------------------------------------------
fn court_doc() -> FlowGraph {
    let mut g = FlowGraph::new("court", "法院文书生成");
    g.add_node(start("s"));
    g.add_node(FlowNode::task("fetch", "调取卷宗", ToolKind::Database, 400).with_tag("dim:data"));
    g.add_node(FlowNode::task("ocr", "卷宗OCR", ToolKind::Llm, 600).with_tag("dim:algo"));
    g.add_node(
        FlowNode::task("extract", "要素抽取", ToolKind::Llm, 300)
            .with_tag("dim:algo")
            .with_access(Access::read("var:case")),
    );
    g.add_node(
        FlowNode::task("draft", "文书起草", ToolKind::Llm, 800)
            .with_tag("dim:sec")
            .with_access(Access::read("var:case")),
    );
    g.add_node(
        FlowNode::task("review", "合规复核", ToolKind::Llm, 300)
            .with_tag("dim:perm")
            .with_tag("dim:sec"),
    );
    g.add_node(end("e"));
    g.rules.push(rule("r-court", "文书须双人复核且留痕", &["var:"]));
    g.add_edge(FlowEdge::seq("s", "fetch"));
    g.add_edge(FlowEdge::seq("fetch", "ocr"));
    g.add_edge(FlowEdge::seq("ocr", "extract"));
    g.add_edge(FlowEdge::seq("extract", "draft"));
    g.add_edge(FlowEdge::seq("draft", "review"));
    g.add_edge(FlowEdge::seq("review", "e"));
    g
}

// ---------------------------------------------------------------------------
// 业务三：财务对账（多源拉取 + 差异解释）
// ---------------------------------------------------------------------------
fn finance_reco() -> FlowGraph {
    let mut g = FlowGraph::new("finance", "财务对账");
    g.add_node(start("s"));
    g.add_node(FlowNode::task("pull_a", "拉取A系统", ToolKind::Database, 300));
    g.add_node(FlowNode::task("pull_b", "拉取B系统", ToolKind::Database, 300));
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
    g.add_node(
        FlowNode::task("ic", "意图分类", ToolKind::Llm, 120)
            .with_tag("dim:algo"),
    );
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
    g.add_node(FlowNode::task("ingest", "接入数据源", ToolKind::Database, 200));
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
    g.add_node(FlowNode::task("aggregate", "聚合结果", ToolKind::Compute, 120));
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
    // 1. 输入参数 / 校验
    g.add_node(
        FlowNode::task("input", "参数校验", ToolKind::Compute, 80)
            .with_tag("dim:algo")
            .with_access(Access::read("var:spiral_params")),
    );
    // 2. Frenet 螺旋运动学（数学内核，干净）
    g.add_node(
        FlowNode::task("kinematics", "螺旋运动学计算", ToolKind::Compute, 200)
            .with_tag("dim:algo"),
    );
    // 3. 量纲诊断（修正原报告错误）
    g.add_node(
        FlowNode::task("dimcheck", "量纲自洽诊断", ToolKind::Compute, 150)
            .with_tag("dim:algo")
            .with_tag("compliance"),
    );
    // 4. 数值巧合标注
    g.add_node(
        FlowNode::task("numcheck", "数值巧合标注", ToolKind::Compute, 120)
            .with_tag("dim:algo"),
    );
    // 5. 生成报告
    g.add_node(
        FlowNode::task("report", "生成诊断报告", ToolKind::Compute, 100)
            .with_tag("dim:obs")
            .with_access(Access::write("var:spiral_report")),
    );
    g.add_node(end("e"));
    g.rules.push(rule("r-spiral", "螺旋模型物理推论须经量纲校验方可对外发布", &["var:"]));
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
        Business { id: "gov-pii", name: "政务数据归集", domain: "gov", regulated: true, build: gov_pii },
        Business { id: "court", name: "法院文书生成", domain: "court", regulated: true, build: court_doc },
        Business { id: "finance", name: "财务对账", domain: "finance", regulated: true, build: finance_reco },
        Business { id: "bot", name: "智能客服", domain: "service", regulated: false, build: customer_bot },
        Business { id: "etl", name: "ETL归集管道", domain: "data", regulated: false, build: etl },
        Business { id: "mcp", name: "MCP插件编排", domain: "integration", regulated: false, build: mcp_orchestration },
        Business { id: "spiral", name: "空间光速螺旋模型分析", domain: "science", regulated: false, build: spiral_analysis },
    ]
}

/// 跨业务共享的六维关系网：注入 Skill / Rule / Memory / Model 实体与关系
pub fn build_topology() -> TopologyGraph {
    let mut topo = TopologyGraph::new();
    // —— 模型实体（算力分级路由）——
    topo.add_entity(Entity::new("model:hermes3", EntityKind::Model, "Hermes3 重模型").with_cost(800).with_keywords(["流程图", "代码", "重推理"]));
    topo.add_entity(Entity::new("model:light", EntityKind::Model, "轻量模型").with_cost(120).with_keywords(["分类", "意图", "摘要"]));
    // —— 工具实体（流程节点经 ingest_flow 自动 Binds）——
    for t in ["database", "browser", "file", "http", "shell", "llm", "compute", "guard"] {
        topo.add_entity(Entity::new(format!("tool:{}", t), EntityKind::Tool, t.to_string()).with_keywords([t]));
    }
    // —— Skill 实体（可复用模板，跨业务命中即跳过 ReAct）——
    topo.add_entity(Entity::new("skill:desensitize", EntityKind::Skill, "脱敏模板").with_keywords(["脱敏", "pii", "政务"]).with_cost(50));
    topo.add_entity(Entity::new("skill:intent-route", EntityKind::Skill, "意图路由模板").with_keywords(["意图", "分类", "路由", "客服"]).with_cost(120));
    topo.add_entity(Entity::new("skill:etl-map", EntityKind::Skill, "ETL字段映射模板").with_keywords(["etl", "映射", "抽取"]).with_cost(250));
    topo.add_entity(Entity::new("skill:db-pull", EntityKind::Skill, "数据库拉取模板").with_keywords(["数据库", "拉取", "对账"]).with_cost(300));
    // —— Memory 实体（语义检索块）——
    topo.add_entity(Entity::new("mem:kb_vec", EntityKind::Memory, "知识库向量").with_keywords(["知识", "检索", "客服"]));
    topo.add_entity(Entity::new("mem:case_vec", EntityKind::Memory, "卷宗向量").with_keywords(["卷宗", "法院", "检索"]));
    // —— Rule 实体（合规约束，ingest_flow 自动 Constrains 命中节点）——
    topo.add_entity(Entity::new("rule:pii", EntityKind::Rule, "PII 必须脱敏").with_keywords(["pii", "脱敏", "政务"]));
    topo.add_entity(Entity::new("rule:dual-review", EntityKind::Rule, "文书双人复核").with_keywords(["复核", "法院", "留痕"]));
    // —— 关系：模型服务任务类型 ——
    topo.add_relation(Relation::new("model:hermes3", "flow:gov-pii:ic", RelationKind::Serves, 0.9));
    topo.add_relation(Relation::new("model:light", "flow:bot:ic", RelationKind::Serves, 0.95));
    // —— 关系：Skill 实现(Implements)具体流程节点，构成最短路径终点 ——
    topo.add_relation(Relation::new("skill:desensitize", "flow:gov-pii:guard", RelationKind::Implements, 1.0));
    topo.add_relation(Relation::new("skill:intent-route", "flow:bot:ic", RelationKind::Implements, 1.0));
    topo.add_relation(Relation::new("skill:etl-map", "flow:etl:map", RelationKind::Implements, 1.0));
    topo.add_relation(Relation::new("skill:db-pull", "flow:finance:pull_a", RelationKind::Implements, 1.0));
    // —— 关系：Skill 召回记忆 ——
    topo.add_relation(Relation::new("skill:intent-route", "mem:kb_vec", RelationKind::Recalls, 0.8));
    topo.add_relation(Relation::new("skill:etl-map", "mem:case_vec", RelationKind::Recalls, 0.6));
    // —— 关系：规则约束流程分支 ——
    topo.add_relation(Relation::new("rule:pii", "flow:gov-pii:db", RelationKind::Constrains, 1.0));
    topo.add_relation(Relation::new("rule:dual-review", "flow:court:review", RelationKind::Constrains, 1.0));
    topo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_builds_all_businesses() {
        let biz = all_businesses();
        assert_eq!(biz.len(), 7);
        for b in &biz {
            let g = (b.build)();
            assert!(!g.nodes.is_empty(), "{} 应有节点", b.id);
            assert!(g.topo_order().is_ok(), "{} 应为 DAG", b.id);
        }
    }

    #[test]
    fn topology_has_six_dimension_entities() {
        let topo = build_topology();
        let kinds: std::collections::HashSet<_> =
            topo.entities.iter().map(|e| format!("{:?}", e.kind)).collect();
        for k in ["Model", "Tool", "Skill", "Memory", "Rule", "FlowNode"] {
            assert!(kinds.iter().any(|x| x == k), "关系网应含 {} 维", k);
        }
    }
}
