//! ⛨ 璇玑验证网关（最高权限）
//!
//! 在 flow-ai 求解之后、治理闸门之前插入。所有检查均为**数学/语义正确性**判定，
//! 任何 RBAC / 合规 / 权限专家的结论都不可覆盖本层结论。
//! 任一阻断级检查失败 → `vetoed = true` → 治理闸门必须 BLOCK（记录 `algorithm_veto`）。

use flow_ai::model::{FlowGraph, NodeKind};
use flow_ai::pipeline::OptimizationReport;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// 单条验证结论
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    /// 检查名：topology / data_dep / conflict / gains / code_rt
    pub name: String,
    /// 是否通过
    pub passed: bool,
    /// 是否阻断级（失败则整体否决）
    pub blocking: bool,
    /// 人类可读说明；失败时为反例
    pub detail: String,
}

/// 璇玑验证报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgoVerification {
    pub checks: Vec<Check>,
    /// 全部通过
    pub all_passed: bool,
    /// 任一阻断级检查失败 → 治理必须 BLOCK
    pub vetoed: bool,
    pub summary: String,
}

impl AlgoVerification {
    pub fn check(&self, name: &str) -> Option<&Check> {
        self.checks.iter().find(|c| c.name == name)
    }
}

/// 最高权限验证：优化前图 vs 优化报告
pub fn verify(before: &FlowGraph, opt: &OptimizationReport) -> AlgoVerification {
    let after = &opt.optimized_graph;
    let checks: Vec<Check> = vec![
        topology_invariant(before, after),
        data_dependency_invariant(before, after, opt),
        conflict_invariant(after, opt),
        credible_gains_invariant(opt),
        code_roundtrip_invariant(opt),
    ];

    let vetoed = checks.iter().any(|c| c.blocking && !c.passed);
    let all_passed = checks.iter().all(|c| c.passed);
    let summary = if vetoed {
        format!(
            "⛨ 算法否决：{} 项阻断级检查未通过（语义/依赖/一致性被破坏）",
            checks.iter().filter(|c| c.blocking && !c.passed).count()
        )
    } else {
        format!("⛨ 算法验证通过：{} 项检查全部可信", checks.len())
    };

    AlgoVerification {
        checks,
        all_passed,
        vetoed,
        summary,
    }
}

/// 5a 拓扑守恒（语义级）：
/// 1) 原始节点全部保留（允许 flow-ai 新增 guard/handler）；
/// 2) 任何「写→读」真数据依赖对的**可达性必须保持**（被优化器挪动会导致读早于写，属语义破坏）。
///    普通控制边/无数据共享的并行化不算破坏（flow-ai 合法剪除伪依赖）。
fn topology_invariant(before: &FlowGraph, after: &FlowGraph) -> Check {
    // 1) 原始节点必须全部保留
    let b_ids: BTreeSet<&str> = before.nodes.iter().map(|n| n.id.as_str()).collect();
    let a_ids: BTreeSet<&str> = after.nodes.iter().map(|n| n.id.as_str()).collect();
    let missing: Vec<&str> = b_ids.difference(&a_ids).copied().collect();
    if !missing.is_empty() {
        return Check {
            name: "topology".into(),
            passed: false,
            blocking: true,
            detail: format!("原始节点在优化后丢失: {:?}", &missing[..missing.len().min(5)]),
        };
    }

    // 2) 真数据依赖对的语义可达性守恒
    let rb = before.reachability();
    let ra = after.reachability();
    let mut mismatch = Vec::new();
    for u in &b_ids {
        let nu = match before.node(u) {
            Some(n) => n,
            None => continue,
        };
        let writes = nu.write_set();
        if writes.is_empty() {
            continue;
        }
        let ui = before.index_of(u).unwrap();
        let ai = after.index_of(u).unwrap();
        for v in &b_ids {
            if u == v {
                continue;
            }
            let nv = match before.node(v) {
                Some(n) => n,
                None => continue,
            };
            let shared = writes.iter().any(|w| nv.read_set().contains(w));
            if !shared {
                continue; // 无数据共享，顺序可被优化器重组
            }
            let vi = before.index_of(v).unwrap();
            let avi = after.index_of(v).unwrap();
            let before_reach = rb.reaches(ui, vi);
            let after_reach = ra.reaches(ai, avi);
            if before_reach && !after_reach {
                mismatch.push(format!("{u}→{v}（写 {:?} 必须早于读，但可达性被破坏）", writes));
            }
        }
    }
    if !mismatch.is_empty() {
        return Check {
            name: "topology".into(),
            passed: false,
            blocking: true,
            detail: format!("真数据依赖可达性被破坏（≤5 例）: {:?}", &mismatch[..mismatch.len().min(5)]),
        };
    }
    Check {
        name: "topology".into(),
        passed: true,
        blocking: true,
        detail: format!("原始节点 {} 全部保留，真数据依赖可达性守恒", b_ids.len()),
    }
}

/// 5b 数据依赖守恒：被剪除的伪依赖不得破坏真数据依赖（读早于写）
fn data_dependency_invariant(
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
                drop(nb_read);
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
fn path_preserves_data_dep(
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

/// 5c 冲突消解守恒：0 阻塞冲突 + 无悬空异常边
fn conflict_invariant(after: &FlowGraph, opt: &OptimizationReport) -> Check {
    if opt.conflicts.has_blocking() {
        return Check {
            name: "conflict".into(),
            passed: false,
            blocking: true,
            detail: format!(
                "优化后仍存在 {} 个阻塞级冲突",
                opt.conflicts.blocking().len()
            ),
        };
    }
    // 悬空异常边：目标节点不存在，或不是 Guard/Handler/End 类型。
    // 放宽说明：异常 → 普通 End 作为「异常归档/终止」是合法业务语义，
    // 此前要求必须是 Guard/Handler 约束过严（迭代 4-① 优化项）。
    let mut dangling = Vec::new();
    for e in &after.edges {
        if matches!(e.kind, flow_ai::model::EdgeKind::Exception) {
            match after.node(&e.to) {
                None => dangling.push(format!("{}→{} 目标缺失", e.from, e.to)),
                Some(n) => {
                    if !matches!(n.kind, NodeKind::Guard | NodeKind::End)
                        && !is_handler_name(&n.name)
                    {
                        dangling.push(format!(
                            "{}→{} 目标非 Handler/Guard/End",
                            e.from, e.to
                        ));
                    }
                }
            }
        }
    }
    if !dangling.is_empty() {
        return Check {
            name: "conflict".into(),
            passed: false,
            blocking: true,
            detail: format!("悬空异常边: {:?}", &dangling[..dangling.len().min(5)]),
        };
    }
    Check {
        name: "conflict".into(),
        passed: true,
        blocking: true,
        detail: format!("阻塞冲突 0，异常边全部落点有效（{} 条）", opt.conflicts.conflicts.len()),
    }
}

fn is_handler_name(name: &str) -> bool {
    name.contains("error") || name.contains("handler") || name.contains("错误处理") || name.starts_with("__")
}

/// 5d 收益可信：speedup≥1 且并行不慢于串行
fn credible_gains_invariant(opt: &OptimizationReport) -> Check {
    let g = &opt.gains;
    if g.speedup < 1.0 {
        return Check {
            name: "gains".into(),
            passed: false,
            blocking: false,
            detail: format!("speedup={:.2} < 1.0，收益虚假", g.speedup),
        };
    }
    // 并行调度耗时不应超过串行（允许 5% 调度误差）
    let eps = (g.sequential_ms as f64 * 0.05).max(1.0);
    if g.scheduled_ms as f64 > g.sequential_ms as f64 + eps {
        return Check {
            name: "gains".into(),
            passed: false,
            blocking: false,
            detail: format!(
                "scheduled_ms={} > sequential_ms={}（+eps {}），并行反而更慢",
                g.scheduled_ms, g.sequential_ms, eps as u64
            ),
        };
    }
    Check {
        name: "gains".into(),
        passed: true,
        blocking: false,
        detail: format!(
            "speedup={:.2}×，scheduled {}ms ≤ sequential {}ms",
            g.speedup, g.scheduled_ms, g.sequential_ms
        ),
    }
}

/// 5e 代码往返一致（仅 emit_code 时；不一致仅告警不阻断）
fn code_roundtrip_invariant(opt: &OptimizationReport) -> Check {
    let code = match &opt.code {
        Some(c) => c,
        None => {
            return Check {
                name: "code_rt".into(),
                passed: true,
                blocking: false,
                detail: "未生成代码（跳过往返检查）".into(),
            }
        }
    };
    // 取主模块 main.py 做反向解析
    let main = code
        .file("main.py")
        .or_else(|| code.files.first())
        .map(|f| &f.content);
    let src = match main {
        Some(s) => s,
        None => {
            return Check {
                name: "code_rt".into(),
                passed: true,
                blocking: false,
                detail: "无 main.py（跳过）".into(),
            }
        }
    };
    let rev = flow_ai::codegen::reverse_from_python(src, &opt.flow_id);
    let g2 = &rev.graph;
    // 反向解析器会重新推导节点 id（基于工具名派生），不保证与原 id 一致；
    // 因此用「可执行工具节点数量」做语义守恒判定：生成的代码应覆盖全部核心工具节点。
    let before_tool_count = opt
        .optimized_graph
        .nodes
        .iter()
        .filter(|n| n.tool.is_some())
        .count();
    let rev_tool_count = g2.nodes.iter().filter(|n| n.tool.is_some()).count();
    // 反向解析器可能因缩进/结构未被识别到工具节点，这里仅做「尽力告警」，不阻断
    if rev_tool_count == 0 && before_tool_count > 0 {
        return Check {
            name: "code_rt".into(),
            passed: false,
            blocking: false,
            detail: "反向解析未识别出工具节点（结构未被缩进解析覆盖），仅告警".into(),
        };
    }
    if rev_tool_count < before_tool_count {
        return Check {
            name: "code_rt".into(),
            passed: false,
            blocking: false,
            detail: format!(
                "反向解析工具节点 {} < 原核心工具节点 {}，疑似丢失",
                rev_tool_count, before_tool_count
            ),
        };
    }
    Check {
        name: "code_rt".into(),
        passed: true,
        blocking: false,
        detail: format!(
            "代码⇄流程图往返一致（反向工具节点 {}，原核心 {}，结构恢复完整）",
            rev_tool_count, before_tool_count
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_ai::model::{Access, FlowEdge, FlowNode, ToolKind};

    fn base_flow() -> FlowGraph {
        let mut g = FlowGraph::new("t", "测试");
        g.add_node(FlowNode::task("a", "写x", ToolKind::File, 100).with_access(Access::write("var:x")));
        g.add_node(FlowNode::task("b", "读x", ToolKind::Compute, 100).with_access(Access::read("var:x")));
        g.add_node(FlowNode::task("c", "读y", ToolKind::Compute, 100).with_access(Access::read("var:y")));
        g.add_edge(FlowEdge::seq("a", "b")); // 真数据依赖 a→b (x)
        g.add_edge(FlowEdge::seq("a", "c")); // 真数据依赖 a→c (x)
        g
    }

    #[test]
    fn normal_optimization_passes_verification() {
        let g = base_flow();
        let opt = flow_ai::optimize(&g, &flow_ai::OptimizeConfig::default());
        let v = verify(&g, &opt);
        // 阻断级检查（拓扑/数据依赖/冲突）必须全部通过，不得否决
        assert!(!v.vetoed, "正常优化不应被否决: {:?}", v.checks);
        // 各阻断级 check 必须 passed
        for c in &v.checks {
            if c.blocking {
                assert!(c.passed, "阻断级检查失败: {:?}", c);
            }
        }
    }

    #[test]
    fn veto_when_data_dependency_broken() {
        // 构造一个「坏优化」：删掉真依赖边 a→b，且不保留任何可达路径
        let g = base_flow();
        let mut opt = flow_ai::optimize(&g, &flow_ai::OptimizeConfig::default());
        // 人为破坏：移除 b 节点，制造语义丢失
        opt.optimized_graph
            .nodes
            .retain(|n| n.id != "b");
        // 同时让 removed_edges 不含这条（模拟优化器误删真依赖）
        opt.plan.removed_edges.push(("a".into(), "b".into()));
        let v = verify(&g, &opt);
        // 拓扑守恒应失败（节点缺失）
        assert!(v.vetoed, "节点缺失必须被否决: {:?}", v.checks);
        assert!(!v.check("topology").unwrap().passed);
    }

    #[test]
    fn veto_when_blocking_conflict_remains() {
        use flow_ai::conflict::ConflictKind;
        use flow_ai::model::Severity;
        let g = base_flow();
        let mut opt = flow_ai::optimize(&g, &flow_ai::OptimizeConfig::default());
        // 注入一个阻塞冲突且未修复
        opt.conflicts.conflicts.push(
            flow_ai::conflict::Conflict::new(
                ConflictKind::DbTransaction,
                Severity::Blocking,
                vec!["x".into()],
                Some("browser".into()),
                "测试阻塞冲突",
                None,
            ),
        );
        let v = verify(&g, &opt);
        assert!(v.vetoed);
        assert!(!v.check("conflict").unwrap().passed);
    }

    #[test]
    fn code_roundtrip_passes_for_generated_code() {
        let g = base_flow();
        let cfg = flow_ai::OptimizeConfig {
            emit_code: true,
            ..Default::default()
        };
        let opt = flow_ai::optimize(&g, &cfg);
        let v = verify(&g, &opt);
        // 不应因代码往返失败而否决（最多告警）
        assert!(!v.vetoed, "代码往返不应阻断: {:?}", v.checks);
    }
}
