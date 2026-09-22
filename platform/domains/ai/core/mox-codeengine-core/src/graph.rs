// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Build 阶段：需求 IR → FlowGraph 流程中间表示。
//!
//! 自研建图规则：
//! 1. start → 各步骤顺序链 → end（伪依赖交给 Solve 阶段的传递归约自动剪除，
//!    因此线性描述也能挖出并行度）；
//! 2. 触碰敏感资源的步骤前自动注入 `desensitize` Guard（`auto_guard=true` 时），
//!    保证「合规护栏优先于业务写操作」；
//! 3. Shell / 浏览器等高危工具步骤前注入 `path_check` Guard。

use crate::ir::{Requirement, StepSpec};
use mox_ai_flow_core::model::{Access, EdgeKind, FlowEdge, FlowGraph, FlowNode, NodeKind};

/// IR → FlowGraph。节点 id 采用 `s{序号}`，护栏为 `g{序号}_*`，稳定可往返。
pub fn build_graph(req: &Requirement, auto_guard: bool) -> FlowGraph {
    let mut g = FlowGraph::new(req.id.clone(), req.name.clone());
    g.add_node(FlowNode::new("start", "开始", NodeKind::Start));
    g.add_node(FlowNode::new("end", "结束", NodeKind::End));

    let mut prev = "start".to_string();
    for (i, step) in req.steps.iter().enumerate() {
        for guard in guards_for(step, auto_guard) {
            let gid = format!("g{i}_{}", guard.0);
            let mut guard_node =
                FlowNode::new(&gid, guard.1, NodeKind::Guard).with_tag(guard.0);
            guard_node.duration_ms = 50;
            g.add_node(guard_node);
            g.add_edge(FlowEdge::seq(&prev, &gid));
            prev = gid;
        }
        let nid = format!("s{i}");
        let mut node = FlowNode::task(&nid, &step.name, step.tool, step.duration_ms);
        for r in &step.reads {
            node.accesses.push(Access::read(r));
        }
        for w in &step.writes {
            node.accesses.push(Access::write(w));
        }
        for s in &step.sensitive {
            node.props.insert(format!("sensitive:{s}"), "true".into());
        }
        if step.tool == mox_ai_flow_core::model::ToolKind::Database && !step.writes.is_empty() {
            node = node.transactional(true).idempotent(true);
        }
        g.add_node(node);
        g.add_edge(FlowEdge::seq(&prev, &nid));
        prev = nid;
    }
    g.add_edge(FlowEdge {
        from: prev,
        to: "end".into(),
        kind: EdgeKind::Sequence,
        condition: None,
    });
    g
}

/// (tag, 展示名) 护栏清单
fn guards_for(step: &StepSpec, auto_guard: bool) -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    if auto_guard && !step.sensitive.is_empty() {
        out.push(("desensitize", "脱敏护栏"));
    }
    if auto_guard && matches!(step.tool, mox_ai_flow_core::model::ToolKind::Shell) {
        out.push(("path_check", "路径与命令白名单护栏"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_flow_core::model::ToolKind;

    #[test]
    fn builds_chain_with_start_end() {
        let req = parse_requirement_fake(&[("处理 a_input.txt 文件", ToolKind::File)]);
        let g = build_graph(&req, true);
        assert_eq!(g.nodes.len(), 2 + 1);
        assert_eq!(g.edges.len(), 2);
    }

    #[test]
    fn injects_desensitize_guard_before_sensitive_step() {
        let req = parse_requirement_fake(&[("查询 db.citizen_idcard 表", ToolKind::Database)]);
        let g = build_graph(&req, true);
        let guard = g.nodes.iter().find(|n| n.tags.contains(&"desensitize".to_string()));
        assert!(guard.is_some());
        // 护栏必须先于业务步骤
        let gid = guard.unwrap().id.clone();
        assert!(g
            .edges
            .iter()
            .any(|e| e.from == gid && e.to == "s0"));
    }

    #[test]
    fn no_guard_when_auto_guard_off() {
        let req = parse_requirement_fake(&[("查询 db.citizen_idcard 表", ToolKind::Database)]);
        let g = build_graph(&req, false);
        assert!(g.nodes.iter().all(|n| n.kind != NodeKind::Guard));
    }

    fn parse_requirement_fake(steps: &[(&str, ToolKind)]) -> Requirement {
        use crate::ir::StepSpec;
        Requirement {
            id: "t".into(),
            name: "t".into(),
            text: String::new(),
            steps: steps
                .iter()
                .map(|(n, t)| StepSpec {
                    name: n.to_string(),
                    tool: *t,
                    duration_ms: 100,
                    reads: vec![n.split_whitespace().last().unwrap_or("x_y").to_string()],
                    writes: Vec::new(),
                    sensitive: if crate::ir::is_sensitive(n) {
                        vec![n.split_whitespace().last().unwrap_or("").to_string()]
                    } else {
                        Vec::new()
                    },
                })
                .collect(),
        }
    }
}
