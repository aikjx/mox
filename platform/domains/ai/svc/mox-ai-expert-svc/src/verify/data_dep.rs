//! 5b 数据依赖守恒：被剪除的伪依赖不得破坏真数据依赖（读早于写）
//!
//! 性能要点（T9）：
//!   · 避免在每个并行层 pair 内重新计算传闭包。
//!   · 若并行层的 RAW 对在拓扑秩上 already 有序（秩小的节点是写者、秩大的是读者，
//!     且 after 上写者→读者可达），则该并行层 pair 不构成 RAW 冒险，跳过位图查询。

use crate::verify::Check;
use mox_ai_flow_svc::model::{FlowGraph, Reachability};
use mox_ai_flow_svc::pipeline::OptimizationReport;
use std::collections::BTreeSet;

/// 5b 数据依赖守恒 —— 公开入口（保持旧签名）
pub fn data_dependency_invariant(
    before: &FlowGraph,
    after: &FlowGraph,
    opt: &OptimizationReport,
) -> Check {
    data_dependency_invariant_with_reach(before, after, opt, None)
}

/// 5b 数据依赖守恒（允许传入已缓存的 after 可达性）
pub fn data_dependency_invariant_with_reach(
    before: &FlowGraph,
    after: &FlowGraph,
    opt: &OptimizationReport,
    after_reach_cached: Option<&Reachability>,
) -> Check {
    // Phase 1：真依赖边被剪除时，after 上仍须存在「写者 →* 读者」路径
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
            continue;
        }
        if !path_preserves_data_dep_inner(after, u, v, after_reach_cached) {
            violated.push(format!("{u}→{v} (写 {:?} 被读)", writes));
        }
    }

    // Phase 2：同一并行层中的写-读对不应并行（RAW 冒险）
    // 性能：
    //   · 只对有访问集的节点做检查；大多数节点 write_set/read_set 为空。
    //   · 拓扑秩短切：层内对 (a,b) 若 after.topo_order 上 a 严格先于 b 且 a→b 可达，
    //     则该对实际有序，不会被并行。
    let layers = &opt.plan.layers;
    let mut raw_risk = Vec::new();

    let (ra_cache, a_pos) = build_reach_and_pos(after, after_reach_cached);
    // id_index 用于快速把 node id 翻译成索引
    let n = after.nodes.len();
    let mut id_index: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::with_capacity(n);
    for (i, nd) in after.nodes.iter().enumerate() {
        id_index.insert(nd.id.as_str(), i);
    }

    for layer in layers {
        // 预取出层内每个节点的 (idx, write_set, read_set, pos)
        let mut members: Vec<(usize, Vec<&str>, Vec<&str>, usize)> =
            Vec::with_capacity(layer.len());
        for id in layer {
            let Some(&idx) = id_index.get(id.as_str()) else {
                continue;
            };
            let Some(node) = after.node(id) else { continue };
            let Some(pos) = a_pos[idx] else { continue };
            let ws: Vec<&str> = node.write_set().into_iter().collect();
            let rs: Vec<&str> = node.read_set().into_iter().collect();
            if ws.is_empty() && rs.is_empty() {
                continue;
            }
            members.push((idx, ws, rs, pos));
        }
        if members.len() < 2 {
            continue;
        }
        for i in 0..members.len() {
            let (ai, aws, _, apos) = &members[i];
            let ai = *ai;
            let apos = *apos;
            if aws.is_empty() {
                continue;
            }
            for (_, (bj, _, brs, bpos)) in members.iter().enumerate().skip(i + 1) {
                let bj = *bj;
                let bpos = *bpos;
                if brs.is_empty() {
                    continue;
                }
                // 共享写-读交集？
                let mut shared_exists = false;
                for w in aws {
                    if brs.contains(w) {
                        shared_exists = true;
                        break;
                    }
                }
                if !shared_exists {
                    continue;
                }
                // 秩短切 + 位图确认
                let reach_ab = apos < bpos && ra_cache.reaches(ai, bj);
                let reach_ba = bpos < apos && ra_cache.reaches(bj, ai);
                if reach_ab || reach_ba {
                    // 有依赖序：该 pair 实际串行，不冒险
                    continue;
                }
                let a_id = after.nodes[ai].id.as_str();
                let b_id = after.nodes[bj].id.as_str();
                raw_risk.push(format!("{a_id}|{b_id} 共享写-读并行"));
                if raw_risk.len() >= 5 {
                    break;
                }
            }
            if raw_risk.len() >= 5 {
                break;
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
            detail: format!(
                "并行层存在 RAW 冒险风险: {:?}",
                &raw_risk[..raw_risk.len().min(5)]
            ),
        };
    }
    Check {
        name: "data_dep".into(),
        passed: true,
        blocking: true,
        detail: format!(
            "剪除 {} 条伪依赖，真数据依赖全部保留",
            opt.plan.removed_edges.len()
        ),
    }
}

/// 构建 (reachability, topo_pos)，优先复用调用方缓存的可达性。
fn build_reach_and_pos<'a>(
    g: &FlowGraph,
    cached: Option<&'a Reachability>,
) -> (std::borrow::Cow<'a, Reachability>, Vec<Option<usize>>) {
    match cached {
        Some(r) => {
            // 缓存命中：重算 topo_pos（便宜）
            let order = g
                .topo_order()
                .unwrap_or_else(|_| (0..g.nodes.len()).collect());
            let mut pos = vec![None; g.nodes.len()];
            for (rank, &u) in order.iter().enumerate() {
                pos[u] = Some(rank);
            }
            (std::borrow::Cow::Borrowed(r), pos)
        }
        None => {
            let (r, pos) = g.reachability_with_topo_pos();
            (std::borrow::Cow::Owned(r), pos)
        }
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
    path_preserves_data_dep_inner(after, u, v, None)
}

fn path_preserves_data_dep_inner(
    after: &FlowGraph,
    u: &str,
    v: &str,
    after_reach_cached: Option<&Reachability>,
) -> bool {
    let ui = match after.index_of(u) {
        Some(i) => i,
        None => return false,
    };
    let vi = match after.index_of(v) {
        Some(i) => i,
        None => return false,
    };
    if let Some(r) = after_reach_cached {
        return r.reaches(ui, vi);
    }
    after.reachability().reaches(ui, vi)
}
