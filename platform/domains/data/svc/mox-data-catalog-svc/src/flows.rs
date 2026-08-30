// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_ai_flow_svc::model::{
    Access, ExpertRule, FlowEdge, FlowGraph, FlowNode, Severity, ToolKind,
};

use crate::builders::{end, guard, rule, start};
use crate::business::Business;

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
