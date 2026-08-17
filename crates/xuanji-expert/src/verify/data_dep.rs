//! 5b 数据依赖守恒：被剪除的伪依赖不得破坏真数据依赖（读早于写）

use crate::verify::Check;
use flow_ai::model::FlowGraph;
use flow_ai::pipeline::OptimizationReport;
use std::collections::BTreeSet;

/// 5b 数据依赖守恒：被剪除的伪依赖不得破坏真数据依赖（读早于写）
pub fn data_dependency_invariant(
    before: &FlowGraph,
    after: &FlowGraph,
    opt: &OptimizationReport,
) -> Check {
    // 对每条被 removed_edges 删除的边 (u,v)：检查 after 中是否仍存在满足数据依赖的路径
    // 真依赖判定：u.write_set ∩ v.read_set ≠ ∅ 意味着 v 必须读到 u 的写
    let mut violated = Vec::new();
    for (u, v) in &opt.plan.removed_edges {
        let nu = match before.node(u) {
            Some(n) => n,
            None => continue,
        };
        let nv = match before.node(v) {
            Some(n) => n,
            None => continue,
        };
        let writes = nu.write_set();
        let reads = nv.read_set();
        let true_dep = writes.iter().any(|w| reads.contains(w));
        if !true_dep {
            // 仅是伪依赖（无共享变量），安全剪除
            continue;
        }
        // 真数据依赖被剪：必须存在 after 中 u →* v 的路径使依赖链完整
        if !path_preserves_data_dep(after, u, v, &writes, &reads) {
            violated.push(format!("{u}→{v} (写 {:?} 被读)", writes));
        }
    }
    // 额外：所有保留边的 RAW 冒险（读早于写）不违规 —— 用 after 的并行层检查
    // 若同一并行层内存在 "u 写一个变量，v 读同一变量" 且两者无先后边，则危险
    let layers = &opt.plan.layers;
    let mut raw_risk = Vec::new();
    for layer in layers {
        for i in 0..layer.len() {
            for j in (i + 1)..layer.len() {
                let a = &layer[i];
                let b = &layer[j];
                let na = match after.node(a) {
                    Some(n) => n,
                    None => continue,
                };
                let nb = match after.node(b) {
                    Some(n) => n,
                    None => continue,
                };
                let nb_read = nb.read_set();
                let shared: Vec<&str> =
                    na.write_set().iter().filter(|w| nb_read.contains(*w)).copied().collect();
                if !shared.is_empty() && !after.reachability().reaches(
                    after.index_of(a).unwrap(),
                    after.index_of(b).unwrap(),
                ) {
                    raw_risk.push(format!("{a}|{b} 共享写-读 {:?} 却同层并行", shared));
                }
            }
        }
    }

    if !violated.is_empty() {
        return Check {
            name: "data_dep".into(),
            passed: false,
            blocking: true,
            detail: format!("真数据依赖被破坏: {:?}", &violated[..violated.len().min(5)]),
        };
    }
    if !raw_risk.is_empty() {
        return Check {
            name: "data_dep".into(),
            passed: false,
            blocking: false,
            detail: format!("并行层存在 RAW 冒险风险: {:?}", &raw_risk[..raw_risk.len().min(5)]),
        };
    }
    Check {
        name: "data_dep".into(),
        passed: true,
        blocking: true,
        detail: format!("剪除 {} 条伪依赖，真数据依赖全部保留", opt.plan.removed_edges.len()),
    }
}

/// 在 after 图中检查 u→*v 路径上是否仍然存在「u 写 → 某中间读 → v 读」的连贯依赖
pub fn path_preserves_data_dep(
    after: &FlowGraph,
    u: &str,
    v: &str,
    _writes: &BTreeSet<&str>,
    _reads: &BTreeSet<&str>,
) -> bool {
    // 简化：只要 after 中 u 仍可达 v（存在任意路径），即认为依赖链未断
    // （更严格可检查路径上是否有 Guard/中间节点透传变量，此处以可达性为保底）
    let ui = match after.index_of(u) {
        Some(i) => i,
        None => return false,
    };
    let vi = match after.index_of(v) {
        Some(i) => i,
        None => return false,
    };
    after.reachability().reaches(ui, vi)
}
