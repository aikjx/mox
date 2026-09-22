// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 自修复层（参考 2026 code agent 的 self-repair loop）：
//! 把专家诊断意见（Finding）映射为对流程图的**确定性结构突变**，
//! 引擎在主循环中「诊断 → 修复 → 重诊断」直至收敛或用尽轮次。
//!
//! 规则表（只修复结构缺陷，绝不放宽审计标准）：
//! - `CE-D-001`：敏感节点缺前置脱敏护栏 → 在目标节点前注入 `desensitize` Guard；
//! - `CE-D-003`：Shell 节点缺命令白名单护栏 → 注入 `path_check` Guard；
//! - `CE-A-003`：孤立不可达节点 → 以 start/end 顺序边接回主链。

use crate::experts::ExpertOpinion;
use mox_ai_flow_core::model::{EdgeKind, FlowEdge, FlowGraph, FlowNode, NodeKind, Severity};
use serde::Serialize;

/// 一轮修复的结果
#[derive(Debug, Clone, Default, Serialize)]
pub struct RepairOutcome {
    /// 人类可读的修复动作清单
    pub applied: Vec<String>,
}

impl RepairOutcome {
    pub fn changed(&self) -> bool {
        !self.applied.is_empty()
    }
}

/// 依据专家意见对图执行突变；返回应用的修复动作。
pub fn repair_graph(graph: &mut FlowGraph, opinions: &[ExpertOpinion]) -> RepairOutcome {
    let mut out = RepairOutcome::default();
    for op in opinions {
        for f in &op.findings {
            if f.severity == Severity::Info {
                continue;
            }
            match f.code {
                "CE-D-001" => {
                    if let Some(t) = &f.target {
                        if insert_guard_before(graph, t, "desensitize", "脱敏护栏(自修复)") {
                            out.applied.push(format!("为敏感节点 {t} 前置注入 desensitize 护栏"));
                        }
                    }
                }
                "CE-D-003" => {
                    if let Some(t) = &f.target {
                        if insert_guard_before(graph, t, "path_check", "命令白名单护栏(自修复)") {
                            out.applied.push(format!("为 Shell 节点 {t} 前置注入 path_check 护栏"));
                        }
                    }
                }
                "CE-A-003" => {
                    if let Some(list) = &f.target {
                        for id in list.split(", ") {
                            if reconnect_orphan(graph, id) {
                                out.applied.push(format!("将孤立节点 {id} 接回主链"));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

fn unique_guard_id(graph: &FlowGraph, target: &str, tag: &str) -> String {
    let base = format!("fix_{target}_{tag}");
    if graph.node(&base).is_none() {
        return base;
    }
    let mut i = 1;
    loop {
        let id = format!("{base}{i}");
        if graph.node(&id).is_none() {
            return id;
        }
        i += 1;
    }
}

/// 在 `target` 前插入带 tag 的 Guard；若目标已有该 tag 的前置护栏则不动（幂等）。
fn insert_guard_before(graph: &mut FlowGraph, target: &str, tag: &str, label: &str) -> bool {
    let already = graph.edges.iter().any(|e| {
        e.to == target
            && graph
                .node(&e.from)
                .map(|p| p.kind == NodeKind::Guard && p.tags.iter().any(|x| x == tag))
                .unwrap_or(false)
    });
    if already {
        return false;
    }
    let gid = unique_guard_id(graph, target, tag);
    let mut guard = FlowNode::new(&gid, label, NodeKind::Guard).with_tag(tag);
    guard.duration_ms = 50;
    // 入边重指到护栏
    for e in graph.edges.iter_mut() {
        if e.to == target {
            e.to = gid.clone();
        }
    }
    graph.add_node(guard);
    graph.add_edge(FlowEdge {
        from: gid,
        to: target.to_string(),
        kind: EdgeKind::Sequence,
        condition: None,
    });
    true
}

/// 将零入边的孤立节点接入 start→…→end 主链（幂等）。
fn reconnect_orphan(graph: &mut FlowGraph, id: &str) -> bool {
    if graph.node(id).is_none() {
        return false;
    }
    let has_in = graph.edges.iter().any(|e| e.to == id);
    if has_in {
        return false;
    }
    graph.add_edge(FlowEdge::seq("start", id));
    if !graph.edges.iter().any(|e| e.from == *id) {
        graph.add_edge(FlowEdge::seq(id, "end"));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experts::Finding;
    use crate::graph::build_graph;
    use crate::ir::parse_requirement;

    fn finding(code: &'static str, sev: Severity, target: Option<&str>) -> Finding {
        Finding {
            code,
            expert: "auditor",
            severity: sev,
            message: "m".into(),
            suggestion: "s".into(),
            confidence: 1.0,
            target: target.map(Into::into),
        }
    }

    #[test]
    fn inserts_guard_before_sensitive_node() {
        let req = parse_requirement("t", "t", "查询 db.citizen_idcard 表");
        let mut g = build_graph(&req, false);
        let opinions = vec![ExpertOpinion {
            expert: "auditor",
            role: "r",
            score: 0.0,
            findings: vec![finding("CE-D-001", Severity::Blocking, Some("s0"))],
        }];
        let out = repair_graph(&mut g, &opinions);
        assert!(out.changed());
        let guarded = g.edges.iter().any(|e| {
            e.to == "s0"
                && g.node(&e.from)
                    .map(|p| p.tags.iter().any(|t| t == "desensitize"))
                    .unwrap_or(false)
        });
        assert!(guarded);
        // 幂等：再来一轮不得重复注入
        let out2 = repair_graph(&mut g, &opinions);
        assert!(!out2.changed());
    }

    #[test]
    fn reconnects_orphan_nodes() {
        let mut g = build_graph(&parse_requirement("t", "t", "读取 input.csv 文件"), true);
        g.add_node(FlowNode::task(
            "orphan",
            "孤儿",
            mox_ai_flow_core::model::ToolKind::Compute,
            10,
        ));
        let opinions = vec![ExpertOpinion {
            expert: "analyst",
            role: "r",
            score: 0.8,
            findings: vec![finding("CE-A-003", Severity::Warning, Some("orphan"))],
        }];
        let out = repair_graph(&mut g, &opinions);
        assert!(out.changed());
        assert!(g.edges.iter().any(|e| e.from == "start" && e.to == "orphan"));
        assert!(g.edges.iter().any(|e| e.from == "orphan" && e.to == "end"));
    }

    #[test]
    fn ignores_unrepairable_codes() {
        let mut g = build_graph(&parse_requirement("t", "t", "读取 input.csv 文件"), true);
        let before = g.edges.len();
        let opinions = vec![ExpertOpinion {
            expert: "builder",
            role: "r",
            score: 0.8,
            findings: vec![finding("CE-B-001", Severity::Warning, None)],
        }];
        let out = repair_graph(&mut g, &opinions);
        assert!(!out.changed());
        assert_eq!(g.edges.len(), before);
        let _ = g.node_mut("s0"); // 编译性使用检查
    }
}
