// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Gate 阶段：出码前三证守恒 + 出码后往返校验（全自研）。
//!
//! 三证（缺一不可，任一失败禁止交付——对应 mox-ai-expert-svc 的 G-A~G-E 护栏语义）：
//! 1. **拓扑守恒**：优化图保留原始图全部可执行节点；
//! 2. **数据守恒**：优化前后读/写资源集合并不变（剪的是伪边，不是语义）；
//! 3. **往返守恒**：生成的 tasks.py 覆盖每个可执行节点，且逆向解析器可无恐慌重建流程图。

use mox_ai_flow_core::codegen::{self, CodeBundle};
use mox_ai_flow_core::model::{AccessMode, FlowGraph};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GateCheck {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GateReport {
    pub checks: Vec<GateCheck>,
    pub passed: bool,
}

impl GateReport {
    pub fn summary(&self) -> String {
        self.checks
            .iter()
            .map(|c| format!("[{}] {}", if c.passed { "PASS" } else { "FAIL" }, c.name))
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

fn access_set(graph: &FlowGraph, mode: AccessMode) -> Vec<String> {
    let mut set: Vec<String> = Vec::new();
    for n in &graph.nodes {
        for a in &n.accesses {
            let hit = match mode {
                AccessMode::Read => a.mode.reads(),
                AccessMode::Write => a.mode.writes(),
                AccessMode::ReadWrite => true,
            };
            if hit && !set.contains(&a.resource) {
                set.push(a.resource.clone());
            }
        }
    }
    set.sort();
    set
}

/// 执行三证闸门核查。`bundle` 为 None 表示裁决前尚未出码（视为闸门未通过）。
pub fn run_gates(raw: &FlowGraph, optimized: &FlowGraph, bundle: Option<&CodeBundle>) -> GateReport {
    let mut checks: Vec<GateCheck> = Vec::new();

    // ① 拓扑守恒
    let lost: Vec<&str> = raw
        .nodes
        .iter()
        .filter(|n| n.kind.is_executable())
        .map(|n| n.id.as_str())
        .filter(|id| {
            !optimized
                .nodes
                .iter()
                .any(|o| o.kind.is_executable() && (&o.id == id || codegen::py_ident(&o.id) == codegen::py_ident(id)))
        })
        .collect();
    checks.push(GateCheck {
        name: "拓扑守恒（优化图保留全部可执行节点）",
        passed: lost.is_empty(),
        detail: if lost.is_empty() {
            format!("{} 个可执行节点全保留", raw.nodes.iter().filter(|n| n.kind.is_executable()).count())
        } else {
            format!("丢失节点: {}", lost.join(", "))
        },
    });

    // ② 数据守恒
    let mut data_ok = true;
    let mut detail = String::new();
    for mode in [AccessMode::Read, AccessMode::Write] {
        let before = access_set(raw, mode);
        let after = access_set(optimized, mode);
        let missing: Vec<&String> = before.iter().filter(|r| !after.contains(*r)).collect();
        if !missing.is_empty() {
            data_ok = false;
            detail.push_str(&format!(
                "{mode:?} 丢失资源: {}; ",
                missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
            ));
        }
    }
    if data_ok {
        detail = format!(
            "读集合 {} 项 / 写集合 {} 项不变",
            access_set(raw, AccessMode::Read).len(),
            access_set(raw, AccessMode::Write).len()
        );
    }
    checks.push(GateCheck {
        name: "数据守恒（读写资源集合前后不变）",
        passed: data_ok,
        detail,
    });

    // ③ 往返守恒 + 交付完整性
    let bundle = match bundle {
        Some(b) if !b.rejected => b,
        Some(b) => {
            checks.push(GateCheck {
                name: "往返守恒（出码覆盖全部节点）",
                passed: false,
                detail: format!("代码被拒绝生成: {:?}", b.reject_reasons),
            });
            let passed = checks.iter().all(|c| c.passed);
            return GateReport { checks, passed };
        }
        None => {
            checks.push(GateCheck {
                name: "往返守恒（出码覆盖全部节点）",
                passed: false,
                detail: "尚未产出代码包".into(),
            });
            let passed = checks.iter().all(|c| c.passed);
            return GateReport { checks, passed };
        }
    };
    let tasks: String = bundle
        .files
        .iter()
        .filter(|f| f.path.ends_with(".py"))
        .map(|f| f.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let uncovered: Vec<&str> = optimized
        .nodes
        .iter()
        .filter(|n| n.kind.is_executable())
        .map(|n| n.id.as_str())
        .filter(|id| !tasks.contains(&codegen::py_ident(id)))
        .collect();
    let roundtrip = codegen::reverse_from_python(&tasks, "gate_roundtrip");
    checks.push(GateCheck {
        name: "往返守恒（出码覆盖全部节点）",
        passed: uncovered.is_empty() && !roundtrip.graph.nodes.is_empty(),
        detail: if uncovered.is_empty() {
            format!("{} 行代码全部覆盖，逆向解析重建 {} 节点", bundle.total_lines(), roundtrip.graph.nodes.len())
        } else {
            format!("未覆盖节点: {}", uncovered.join(", "))
        },
    });

    let passed = checks.iter().all(|c| c.passed);
    GateReport { checks, passed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::build_graph;
    use crate::ir::parse_requirement;
    use mox_ai_flow_core::{optimize, OptimizeConfig};

    #[test]
    fn gates_pass_for_auto_graph() {
        let req = parse_requirement("t", "t", "读取 input.csv 文件；把结果写入 db.orders 表；导出报表 output.xlsx");
        let raw = build_graph(&req, true);
        let cfg = OptimizeConfig::default(); // emit_code=true
        let report = optimize(&raw, &cfg);
        let gr = run_gates(&raw, &report.optimized_graph, report.code.as_ref());
        assert!(gr.passed, "三证应全过: {}", gr.summary());
    }

    #[test]
    fn gates_fail_without_bundle() {
        let req = parse_requirement("t", "t", "读取 input.csv 文件");
        let raw = build_graph(&req, true);
        let cfg = OptimizeConfig {
            emit_code: false,
            ..Default::default()
        };
        let report = optimize(&raw, &cfg);
        let gr = run_gates(&raw, &report.optimized_graph, None);
        assert!(!gr.passed);
    }
}
