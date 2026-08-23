//! # flow-ai —— 业务流程图优化 AI 核心算法库
//!
//! 面向「流程图 + 关系网」双载体的 Agent 内核，用 Rust 实现全部核心算法：
//!
//! | 模块 | 解决的问题 | 核心算法 |
//! |------|-----------|---------|
//! | [`model`]    | 流程图统一 IR | 位图传递闭包、Kahn 拓扑排序 |
//! | [`dataflow`] | 串行流程自动并行化 | RAW/WAR/WAW 冒险分析 + 传递归约 |
//! | [`critpath`] | 找瓶颈、算工期 | CPM 双向遍历 + 浮动时间 |
//! | [`conflict`] | 异常/合规前置拦截 | 并发资源冲突检测 + 自动修复 |
//! | [`schedule`] | 真实资源下的排程 | RCPSP 列表调度 (upward rank) |
//! | [`topology`] | 六维实体关系网 | Dijkstra 最短路径 + 权重衰减 |
//! | [`codegen`]  | 流程 ⇄ 代码双向映射 | 分层代码生成 + 缩进结构反解析 |
//! | [`pipeline`] | 端到端编排 | 六阶段流水线 |
//!
//! ## 快速使用
//!
//! ```
//! use flow_ai::prelude::*;
//!
//! let mut g = FlowGraph::new("demo", "示例流程");
//! g.add_node(FlowNode::task("a", "读文件", ToolKind::File, 300)
//!     .with_access(Access::write("var:x")));
//! g.add_node(FlowNode::task("b", "查数据库", ToolKind::Database, 400)
//!     .with_access(Access::write("var:y")));
//! g.add_node(FlowNode::task("c", "汇总", ToolKind::Compute, 100)
//!     .with_access(Access::read("var:x"))
//!     .with_access(Access::read("var:y")));
//! g.add_edge(FlowEdge::seq("a", "b"));
//! g.add_edge(FlowEdge::seq("b", "c"));
//!
//! let report = optimize(&g, &OptimizeConfig::default());
//! // a 与 b 之间是伪依赖，被剪掉后可并行
//! assert!(report.gains.speedup > 1.0);
//! ```

/// 璇玑系统 Crate 注册常量（图谱自同步契约：Rust 端显式声明 crate 身份）。
pub const CRATE_ID: &str = "flow-ai";

/// 璇玑系统 Crate 结构化元数据。
#[derive(Debug, Clone, Copy)]
pub struct CrateMeta {
    pub uuid: &'static str,
    pub ais_layers: &'static [&'static str],
    pub owner_project: &'static str,
    pub capabilities: &'static [&'static str],
    pub data_tables_read: &'static [&'static str],
    pub data_tables_write: &'static [&'static str],
}

pub const CRATE_META: CrateMeta = CrateMeta {
    uuid: "e5b1a7c6-2f8b-4e9c-3a5b-6c7d8e9fa0b1",
    ais_layers: &["L4-Core", "L3-Service", "L7-Tool"],
    owner_project: "proj-graph-infra",
    capabilities: &[
        "数据流 RAW/WAR/WAW 冒险检测",
        "传递归约位并行自动并行化",
        "CPM 关键路径与浮动时间",
        "并发资源冲突检测与自动修复",
        "RCPSP 资源约束列表调度",
        "六维拓扑图权重衰减与最短路径",
        "流程<->代码双向生成与反解析",
        "六阶段优化流水线编排",
    ],
    data_tables_read: &["flows.json"],
    data_tables_write: &[],
};

pub mod automation;
pub mod codegen;
pub mod conflict;
pub mod critpath;
pub mod dataflow;
pub mod model;
pub mod pipeline;
pub mod primitive;
pub mod schedule;
pub mod topology;

pub use pipeline::{optimize, optimize_with_topology, Gains, OptimizationReport, OptimizeConfig};

/// 常用类型一次导入
pub mod prelude {
    pub use crate::codegen::{reverse_from_python, CodeBundle, ReverseResult};
    pub use crate::conflict::{Conflict, ConflictKind, ConflictReport, Remedy};
    pub use crate::critpath::{CriticalPathReport, NodeTiming};
    pub use crate::dataflow::{DepKind, Dependency, ParallelPlan};
    pub use crate::model::{
        Access, AccessMode, EdgeKind, ExpertRule, FlowEdge, FlowGraph, FlowNode, NodeKind,
        ResourcePool, Severity, ToolKind,
    };
    pub use crate::pipeline::{optimize, optimize_with_topology, OptimizationReport, OptimizeConfig};
    pub use crate::schedule::{ModelTier, Schedule, Slot};
    pub use crate::topology::{Entity, EntityKind, Relation, RelationKind, TopologyGraph};
    pub use crate::primitive::{
        adjust_after_failure, adjust_after_success, CandidateTopology, DeliveryPolicy, EmergeStatus,
        EmergenceResult, KnowledgeBase, Outcome, PrimitiveState, PrimiEngine, Requirement, ResourceBudget,
        StoredTopology, SubTask, ValidationReport, Violation, ViolationKind, generate, regularize, validate,
    };
}

/// 从 JSON 载入流程图
pub fn load_flow(json: &str) -> anyhow::Result<model::FlowGraph> {
    Ok(serde_json::from_str(json)?)
}

/// 导出流程图为 JSON
pub fn dump_flow(g: &model::FlowGraph) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(g)?)
}

/// 导出为 Mermaid 流程图（前端可视化 / 文档直用）
pub fn to_mermaid(g: &model::FlowGraph) -> String {
    use model::{EdgeKind, NodeKind};
    let mut s = String::from("flowchart TD\n");
    for n in &g.nodes {
        let id = sanitize(&n.id);
        let label = n.name.replace('"', "'");
        let shape = match n.kind {
            NodeKind::Start | NodeKind::End => format!("{}([\"{}\"])", id, label),
            NodeKind::Decision => format!("{}{{\"{}\"}}", id, label),
            NodeKind::ParallelFork | NodeKind::ParallelJoin => format!("{}[/\"{}\"/]", id, label),
            NodeKind::Guard => format!("{}[[\"{}\"]]", id, label),
            NodeKind::SubFlow => format!("{}[(\"{}\")]", id, label),
            _ => {
                let dur = if n.duration_ms > 0 {
                    format!("<br/>{}ms", n.duration_ms)
                } else {
                    String::new()
                };
                format!("{}[\"{}{}\"]", id, label, dur)
            }
        };
        s.push_str(&format!("    {}\n", shape));
    }
    for e in &g.edges {
        let (a, b) = (sanitize(&e.from), sanitize(&e.to));
        let arrow = match e.kind {
            EdgeKind::Exception => "-.->",
            EdgeKind::InferredData => "==>",
            _ => "-->",
        };
        match &e.condition {
            Some(c) => s.push_str(&format!("    {} {}|{}| {}\n", a, arrow, c.replace('|', "/"), b)),
            None => s.push_str(&format!("    {} {} {}\n", a, arrow, b)),
        }
    }
    s
}

fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use prelude::*;

    fn sample() -> FlowGraph {
        let mut g = FlowGraph::new("t", "测试");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(FlowNode::task("a", "任务A", ToolKind::File, 100));
        g.add_node(FlowNode::new("d", "判断", NodeKind::Decision));
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "a"));
        g.add_edge(FlowEdge::seq("a", "d"));
        g.add_edge(FlowEdge::cond("d", "e", "ok"));
        g.add_edge(FlowEdge::exception("a", "e"));
        g
    }

    #[test]
    fn json_roundtrip() {
        let g = sample();
        let j = dump_flow(&g).unwrap();
        let back = load_flow(&j).unwrap();
        assert_eq!(back.nodes.len(), g.nodes.len());
        assert_eq!(back.edges.len(), g.edges.len());
        assert_eq!(back.name, g.name);
    }

    #[test]
    fn mermaid_has_shapes_and_arrows() {
        let m = to_mermaid(&sample());
        assert!(m.starts_with("flowchart TD"));
        assert!(m.contains("([\"开始\"])"));
        assert!(m.contains("{\"判断\"}"));
        assert!(m.contains("-.->"), "异常边应为虚线: {}", m);
        assert!(m.contains("|ok|"));
    }

    #[test]
    fn optimize_reexport_works() {
        let g = sample();
        let r = optimize(&g, &OptimizeConfig::default());
        assert_eq!(r.flow_id, "t");
    }
}
