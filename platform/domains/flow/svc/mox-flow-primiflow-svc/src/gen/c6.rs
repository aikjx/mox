// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 代码骨架 · 由关联图谱自动生成（mox_flow_primiflow_svc::assoc::primiflow_seed）
//! 溯源链路: R2 → F2 → B3 → A2 → T7 → C6
//! 数据设计: S5(Artifact)
//! 说明: schema 校验 + 冒烟执行（幻觉兜底，失败回写对话重生成，绝不静默放行）。
//! 规格: primiflow/SPEC.md（§9 风险缓解 / §10 DoD）

/// 依赖模块: C4
use mox_ai_flow_sdk::model::{FlowGraph, NodeKind};

/// 单条 schema 校验结果
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

/// 单节点冒烟执行记录
#[derive(Debug, Clone)]
pub struct ExecRecord {
    pub node_id: String,
    pub output: String,
    pub ok: bool,
}

/// 冒烟报告（schema 校验 + 拓扑序执行）
#[derive(Debug, Clone)]
pub struct SmokeReport {
    pub ok: bool,
    pub checks: Vec<CheckResult>,
    pub executed: Vec<ExecRecord>,
}

/// 冒烟测试器：schema 校验 + 拓扑序冒烟执行
#[derive(Debug, Default)]
pub struct SmokeTester;

impl SmokeTester {
    pub fn new() -> Self {
        Self
    }

    /// 冒烟测试：先做 schema 合法性校验，再按拓扑序模拟执行每个节点，
    /// 任一环节失败即 `ok=false`，由上层回写对话重生成（绝不静默放行）。
    pub fn smoke_test(&self, g: &FlowGraph) -> SmokeReport {
        let mut checks = Vec::new();

        // 1) 边端点必须指向存在的节点
        let ids: std::collections::HashSet<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
        let mut dangling = Vec::new();
        for e in &g.edges {
            if !ids.contains(e.from.as_str()) {
                dangling.push(format!("{}→{}", e.from, e.to));
            }
            if !ids.contains(e.to.as_str()) {
                dangling.push(format!("{}→{}", e.from, e.to));
            }
        }
        checks.push(CheckResult {
            name: "边端点存在性".into(),
            passed: dangling.is_empty(),
            detail: if dangling.is_empty() {
                "所有边端点均指向存在的节点".into()
            } else {
                format!("悬空边: {:?}", dangling)
            },
        });

        // 2) 拓扑必须为 DAG（无环）
        let topo = g.topo_order();
        checks.push(CheckResult {
            name: "无环(DAG)".into(),
            passed: topo.is_ok(),
            detail: match &topo {
                Ok(_) => "拓扑可线性化".into(),
                Err(cyc) => format!("检测到环: {:?}", cyc),
            },
        });

        // 3) 起止节点齐全
        let has_start = g.nodes.iter().any(|n| n.kind == NodeKind::Start);
        let has_end = g.nodes.iter().any(|n| n.kind == NodeKind::End);
        checks.push(CheckResult {
            name: "起止节点".into(),
            passed: has_start && has_end,
            detail: format!("start={} end={}", has_start, has_end),
        });

        // 4) 每个可执行节点必须绑定工具
        let missing_tool: Vec<&str> = g
            .nodes
            .iter()
            .filter(|n| n.kind.is_executable() && n.tool.is_none())
            .map(|n| n.id.as_str())
            .collect();
        checks.push(CheckResult {
            name: "可执行节点绑定工具".into(),
            passed: missing_tool.is_empty(),
            detail: if missing_tool.is_empty() {
                "全部可执行节点已绑定工具".into()
            } else {
                format!("缺工具的节点: {:?}", missing_tool)
            },
        });

        let schema_ok = checks.iter().all(|c| c.passed);

        // 5) 拓扑序冒烟执行（仅当 DAG 合法）
        let mut executed = Vec::new();
        if let Ok(order) = topo {
            for idx in order {
                let n = &g.nodes[idx];
                // 模拟执行：依据工具类型产出确定性结果
                let output = match n.tool {
                    Some(_) => format!("OK:{}", n.name),
                    None => format!("CTRL:{}", n.name),
                };
                executed.push(ExecRecord {
                    node_id: n.id.clone(),
                    output,
                    ok: true,
                });
            }
        }

        let ok = schema_ok && !g.nodes.is_empty();
        SmokeReport {
            ok,
            checks,
            executed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_flow_sdk::model::{FlowEdge, FlowNode, ToolKind};

    #[test]
    fn clean_graph_passes_smoke() {
        let mut g = FlowGraph::new("g", "demo");
        g.add_node(FlowNode::new("start", "入口", NodeKind::Start));
        g.add_node(FlowNode::task("a", "抓取", ToolKind::Http, 300));
        g.add_node(FlowNode::task("b", "清洗", ToolKind::Compute, 200));
        g.add_node(FlowNode::new("end", "出口", NodeKind::End));
        g.add_edge(FlowEdge::seq("start", "a"));
        g.add_edge(FlowEdge::seq("a", "b"));
        g.add_edge(FlowEdge::seq("b", "end"));
        let r = SmokeTester::new().smoke_test(&g);
        assert!(r.ok, "合法拓扑应冒烟通过: {:?}", r.checks);
        assert_eq!(r.executed.len(), 4);
    }

    #[test]
    fn dangling_edge_fails() {
        let mut g = FlowGraph::new("g", "demo");
        g.add_node(FlowNode::task("a", "A", ToolKind::Http, 100));
        g.add_edge(FlowEdge::seq("a", "ghost"));
        let r = SmokeTester::new().smoke_test(&g);
        assert!(!r.ok);
        assert!(r
            .checks
            .iter()
            .any(|c| c.name == "边端点存在性" && !c.passed));
    }

    #[test]
    fn cycle_fails_and_no_execution() {
        let mut g = FlowGraph::new("g", "demo");
        g.add_node(FlowNode::new("start", "入口", NodeKind::Start));
        g.add_node(FlowNode::task("a", "A", ToolKind::Http, 100));
        g.add_node(FlowNode::new("end", "出口", NodeKind::End));
        g.add_edge(FlowEdge::seq("start", "a"));
        g.add_edge(FlowEdge::seq("a", "start")); // 环
        let r = SmokeTester::new().smoke_test(&g);
        assert!(!r.ok);
        assert!(r.checks.iter().any(|c| c.name == "无环(DAG)" && !c.passed));
        assert!(r.executed.is_empty(), "有环不应执行");
    }

    #[test]
    fn missing_tool_fails() {
        let mut g = FlowGraph::new("g", "demo");
        g.add_node(FlowNode::new("start", "入口", NodeKind::Start));
        g.add_node(FlowNode::new("a", "未绑定工具的任务", NodeKind::Task)); // 无工具
        g.add_node(FlowNode::new("end", "出口", NodeKind::End));
        g.add_edge(FlowEdge::seq("start", "a"));
        g.add_edge(FlowEdge::seq("a", "end"));
        let r = SmokeTester::new().smoke_test(&g);
        assert!(!r.ok);
        assert!(r
            .checks
            .iter()
            .any(|c| c.name == "可执行节点绑定工具" && !c.passed));
    }
}
