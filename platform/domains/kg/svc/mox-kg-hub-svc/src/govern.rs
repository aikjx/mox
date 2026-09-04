// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 治理层：全域偏离检测 + 三重闸门（守恒 / 六维零孤儿 / GR-STD 合规）。
//!
//! 复用 `primiflow-fusion` 已认证的 `full_gate()`（R07 守恒残差、A4 零孤儿、GR-STD 8 闸门），
//! 并在其之上补齐**全域偏离检测**（GR-E6/E7）——原先偏离检测只存在于
//! `tools/info-graph` CLI，无法覆盖跨知识库的统一图。
//!
//! 治理结论一律可序列化：`primiflow-fusion` 的报告结构未实现 `Serialize`，
//! 本模块转成 DTO 后才能进 HTTP 与 CI，避免在 API 层做临时拼装。

use std::collections::{HashMap, HashSet, VecDeque};

use mox_platform_graph_core::{EntityKind, UnifiedGraph};
use serde::{Deserialize, Serialize};

use crate::ontology;

/// 偏离项
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Deviation {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub layer: String,
    pub path: String,
    /// `GR-E6`（核心实体无需求溯源）/ `GR-E7`（需求未分解）/ `GR-E3`（缺证据）
    pub code: String,
    pub reason: String,
}

/// 偏离检测报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviationReport {
    pub requirement_roots: usize,
    pub core_nodes: usize,
    pub aligned: usize,
    pub deviations: Vec<Deviation>,
    /// 需求对齐覆盖率 %
    pub coverage: f64,
    pub passed: bool,
}

/// 三重闸门 DTO（`mox_platform_graph_core::PlatformGate` 的可序列化投影）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateReport {
    pub passed: bool,
    pub error_count: usize,
    pub conservation_passed: bool,
    pub conservation_total_c: f64,
    pub conservation_errors: Vec<String>,
    pub binding_passed: bool,
    pub binding_six_dim_nodes: usize,
    pub binding_orphans: Vec<String>,
    pub governance_passed: bool,
    pub governance_errors: Vec<String>,
    pub governance_warnings: Vec<String>,
    /// 结构性错误：图本身不可信（悬空边 / 缺证据 / 守恒不自洽 / 空图）——必须拦停
    pub structural_errors: Vec<String>,
    /// 内容债务：图结构有效但存在待治理项（死代码孤儿 / 绑定链不全）——转复核
    pub debt_errors: Vec<String>,
    /// 结构性闸门是否通过。区别于 `passed`（含债务）
    pub structural_passed: bool,
}

/// 判定一条治理错误是「结构性」还是「内容债务」。
///
/// 这一分级至关重要：把债务当结构性错误会导致任何一个死代码文件
/// 就拒收整批知识接入，与本工程 `tools/guantu_gate.py` 的基线治理哲学相悖
/// （存量债务基线化、只阻断新增漂移）。
fn is_structural(err: &str) -> bool {
    // G5 核心孤儿 = 死代码/待挂接，属债务
    if err.starts_with("G5") {
        return false;
    }
    // 六维维度孤儿 = 绑定链不全，属债务
    if err.contains("维度孤儿") {
        return false;
    }
    // G1 空图 / G2 悬空边 / G4 缺证据 / 守恒违规 → 结构性
    true
}

/// 平台治理总报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceSummary {
    pub node_count: usize,
    pub edge_count: usize,
    pub gate: GateReport,
    pub deviation: DeviationReport,
    pub isolated: Vec<String>,
    pub six_dim_coverage: HashMap<String, usize>,
    pub acyclic: bool,
    /// 总体是否放行：闸门与偏离检测同时通过
    pub passed: bool,
}

/// 覆盖率绝对下限护栏（对齐 `tools/guantu_gate.py` 的 `COVERAGE_FLOOR`）
pub const COVERAGE_FLOOR: f64 = 90.0;

/// 全域偏离检测（GR-E6 / GR-E7 / GR-E3）。
///
/// 以 `Requirement` 为根做**无向**可达性 BFS：偏离检测与影响面分析不同，
/// 此处必须用无向——需求与实现的绑定方向在不同来源中并不一致，
/// 用有向会把大量已正确绑定的实体误报为偏离。
pub fn detect_deviation(graph: &UnifiedGraph) -> DeviationReport {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in &graph.edges {
        adj.entry(&e.from).or_default().push(&e.to);
        adj.entry(&e.to).or_default().push(&e.from);
    }

    let roots: Vec<&str> = graph
        .nodes
        .values()
        .filter(|n| n.kind == EntityKind::Requirement)
        .map(|n| n.id.as_str())
        .collect();

    // 自所有需求根出发的可达集
    let mut reachable: HashSet<&str> = HashSet::new();
    let mut q: VecDeque<&str> = VecDeque::new();
    for r in &roots {
        if reachable.insert(r) {
            q.push_back(r);
        }
    }
    while let Some(cur) = q.pop_front() {
        if let Some(neis) = adj.get(cur) {
            for nb in neis {
                if reachable.insert(nb) {
                    q.push_back(nb);
                }
            }
        }
    }

    let mut deviations: Vec<Deviation> = Vec::new();
    let mut core_nodes = 0usize;
    let mut aligned = 0usize;

    for n in graph.nodes.values() {
        // GR-E3：任何实体缺证据都不可信
        if n.evidence.trim().is_empty() {
            deviations.push(Deviation {
                id: n.id.clone(),
                name: n.name.clone(),
                kind: n.kind.zh().to_string(),
                layer: n.layer.code().to_string(),
                path: n.path.clone(),
                code: "GR-E3".to_string(),
                reason: "缺少可定位证据（evidence 为空）".to_string(),
            });
        }

        // GR-E7：需求根未分解（无任何关联）
        if n.kind == EntityKind::Requirement {
            let degree = adj.get(n.id.as_str()).map(|v| v.len()).unwrap_or(0);
            if degree == 0 {
                deviations.push(Deviation {
                    id: n.id.clone(),
                    name: n.name.clone(),
                    kind: n.kind.zh().to_string(),
                    layer: n.layer.code().to_string(),
                    path: n.path.clone(),
                    code: "GR-E7".to_string(),
                    reason: "需求未分解到任何实现（需求悬空）".to_string(),
                });
            }
            continue;
        }

        // GR-E6：核心实现实体必须可溯源到需求根；外部依赖豁免
        if ontology::is_core_impl(n.kind) && !n.external {
            core_nodes += 1;
            if reachable.contains(n.id.as_str()) {
                aligned += 1;
            } else {
                deviations.push(Deviation {
                    id: n.id.clone(),
                    name: n.name.clone(),
                    kind: n.kind.zh().to_string(),
                    layer: n.layer.code().to_string(),
                    path: n.path.clone(),
                    code: "GR-E6".to_string(),
                    reason: "核心实现无需求溯源（偏离/隐性依赖）".to_string(),
                });
            }
        }
    }

    deviations.sort_by(|a, b| a.code.cmp(&b.code).then_with(|| a.id.cmp(&b.id)));

    // 无核心节点时覆盖率视为 100%，避免空图判定为失败
    let coverage = if core_nodes == 0 {
        100.0
    } else {
        aligned as f64 * 100.0 / core_nodes as f64
    };

    DeviationReport {
        requirement_roots: roots.len(),
        core_nodes,
        aligned,
        // 放行条件：无 GR-E6/E7 硬偏离，且覆盖率不低于下限
        passed: !deviations.iter().any(|d| d.code != "GR-E3") && coverage >= COVERAGE_FLOOR,
        coverage: (coverage * 10.0).round() / 10.0,
        deviations,
    }
}

/// 把 fusion 的三重闸门转成可序列化 DTO
pub fn gate_report(graph: &UnifiedGraph) -> GateReport {
    let g = graph.full_gate();

    // 汇总全部错误后按性质分流
    let mut all: Vec<String> = g.conservation.errors.clone();
    all.extend(g.binding.orphans.iter().cloned());
    all.extend(g.governance.errors.iter().cloned());

    let (structural_errors, debt_errors): (Vec<String>, Vec<String>) =
        all.into_iter().partition(|e| is_structural(e));

    GateReport {
        passed: g.passed,
        error_count: g.error_count,
        conservation_passed: g.conservation.passed,
        conservation_total_c: g.conservation.total_c,
        conservation_errors: g.conservation.errors.clone(),
        binding_passed: g.binding.passed,
        binding_six_dim_nodes: g.binding.six_dim_nodes,
        binding_orphans: g.binding.orphans.clone(),
        governance_passed: g.governance.passed,
        governance_errors: g.governance.errors.clone(),
        governance_warnings: g.governance.warnings.clone(),
        structural_passed: structural_errors.is_empty(),
        structural_errors,
        debt_errors,
    }
}

/// 平台治理总评：一次拿到全部结论，供治理台与 CI 共用同一判定口径。
pub fn summarize(graph: &UnifiedGraph) -> GovernanceSummary {
    let gate = gate_report(graph);
    let deviation = detect_deviation(graph);
    let passed = gate.passed && deviation.passed;
    GovernanceSummary {
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
        isolated: crate::reason::isolated(graph),
        six_dim_coverage: crate::reason::six_dim_coverage(graph),
        acyclic: graph.is_acyclic(),
        gate,
        deviation,
        passed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_platform_graph_core::{PrimitiveCoords, RelKind, UnifiedEdge, UnifiedNode};

    fn n(id: &str, kind: EntityKind, evidence: &str) -> UnifiedNode {
        UnifiedNode {
            id: id.into(),
            kind,
            layer: ontology::default_layer(kind),
            name: id.into(),
            path: id.into(),
            summary: String::new(),
            evidence: evidence.into(),
            primitive: PrimitiveCoords::zero(),
            bind_id: None,
            external: false,
        }
    }

    fn e(from: &str, to: &str) -> UnifiedEdge {
        UnifiedEdge {
            id: format!("{from}->{to}"),
            from: from.into(),
            to: to.into(),
            kind: RelKind::Bind,
            label: String::new(),
            evidence: "t".into(),
        }
    }

    #[test]
    fn fully_aligned_graph_has_full_coverage() {
        let mut g = UnifiedGraph::new();
        g.add_node(n("REQ", EntityKind::Requirement, "spec.md:1"));
        g.add_node(n("COD", EntityKind::Code, "a.rs:1"));
        g.add_edge(e("REQ", "COD"));

        let r = detect_deviation(&g);
        assert_eq!(r.core_nodes, 1);
        assert_eq!(r.aligned, 1);
        assert_eq!(r.coverage, 100.0);
        assert!(r.passed);
        assert!(r.deviations.is_empty());
    }

    #[test]
    fn ungrounded_core_node_reported_as_e6() {
        let mut g = UnifiedGraph::new();
        g.add_node(n("REQ", EntityKind::Requirement, "spec.md:1"));
        g.add_node(n("COD", EntityKind::Code, "a.rs:1"));
        g.add_node(n("LOOSE", EntityKind::Code, "b.rs:1"));
        g.add_edge(e("REQ", "COD"));

        let r = detect_deviation(&g);
        assert_eq!(r.core_nodes, 2);
        assert_eq!(r.aligned, 1);
        assert_eq!(r.coverage, 50.0);
        assert!(!r.passed, "50% 覆盖率低于下限必须拦截");
        let e6: Vec<&Deviation> = r.deviations.iter().filter(|d| d.code == "GR-E6").collect();
        assert_eq!(e6.len(), 1);
        assert_eq!(e6[0].id, "LOOSE");
    }

    #[test]
    fn dangling_requirement_reported_as_e7() {
        let mut g = UnifiedGraph::new();
        g.add_node(n("REQ", EntityKind::Requirement, "spec.md:1"));
        let r = detect_deviation(&g);
        let e7: Vec<&Deviation> = r.deviations.iter().filter(|d| d.code == "GR-E7").collect();
        assert_eq!(e7.len(), 1, "无实现的需求必须报 GR-E7");
        assert!(!r.passed);
    }

    #[test]
    fn missing_evidence_reported_as_e3_but_not_blocking_coverage() {
        let mut g = UnifiedGraph::new();
        g.add_node(n("REQ", EntityKind::Requirement, "spec.md:1"));
        g.add_node(n("COD", EntityKind::Code, "")); // 缺证据
        g.add_edge(e("REQ", "COD"));

        let r = detect_deviation(&g);
        assert_eq!(r.coverage, 100.0);
        let e3: Vec<&Deviation> = r.deviations.iter().filter(|d| d.code == "GR-E3").collect();
        assert_eq!(e3.len(), 1);
        // GR-E3 是告警级：不拉低覆盖率，但仍需暴露
        assert!(r.passed, "仅缺证据不应阻断放行，但必须可见");
    }

    #[test]
    fn docs_and_external_deps_are_exempt_from_e6() {
        let mut g = UnifiedGraph::new();
        g.add_node(n("REQ", EntityKind::Requirement, "spec.md:1"));
        g.add_node(n("COD", EntityKind::Code, "a.rs:1"));
        g.add_node(n("DOC", EntityKind::Doc, "r.md:1")); // 文档不强制溯源
        let mut ext = n("SERDE", EntityKind::Code, "Cargo.toml:1");
        ext.external = true; // 外部实体豁免
        g.add_node(ext);
        g.add_edge(e("REQ", "COD"));

        let r = detect_deviation(&g);
        assert_eq!(r.core_nodes, 1, "文档与外部依赖不计入核心实体");
        assert_eq!(r.coverage, 100.0);
        assert!(r.passed);
    }

    #[test]
    fn empty_graph_does_not_divide_by_zero() {
        let g = UnifiedGraph::new();
        let r = detect_deviation(&g);
        assert_eq!(r.core_nodes, 0);
        assert_eq!(r.coverage, 100.0);
        assert!(r.passed);
    }

    #[test]
    fn summary_aggregates_gate_and_deviation() {
        let mut g = UnifiedGraph::new();
        g.add_node(n("REQ", EntityKind::Requirement, "spec.md:1"));
        g.add_node(n("COD", EntityKind::Code, "a.rs:1"));
        g.add_edge(e("REQ", "COD"));

        let s = summarize(&g);
        assert_eq!(s.node_count, 2);
        assert_eq!(s.edge_count, 1);
        assert!(s.acyclic);
        assert_eq!(s.six_dim_coverage.len(), 6);
        // 总评必须是闸门与偏离的合取
        assert_eq!(s.passed, s.gate.passed && s.deviation.passed);
    }

    #[test]
    fn deviation_output_is_deterministic() {
        let mut g = UnifiedGraph::new();
        g.add_node(n("REQ", EntityKind::Requirement, "s:1"));
        for i in 0..8 {
            g.add_node(n(&format!("L{i}"), EntityKind::Code, "x:1"));
        }
        let a = detect_deviation(&g).deviations;
        for _ in 0..5 {
            assert_eq!(detect_deviation(&g).deviations, a, "偏离清单必须稳定可比对");
        }
    }
}
