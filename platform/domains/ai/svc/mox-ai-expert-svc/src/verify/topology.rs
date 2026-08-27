// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 5a 拓扑守恒（语义级）
//!
//! 性能要点（T9）：
//!   · 调用方（`verify()`）会共享 `after` 的可达性；本模块提供 `_with_reach` 变体，
//!     避免一次调用里对同一图算 2~3 次传递闭包。
//!   · 对严格 DAG（深链）：用拓扑秩 `pos[u] < pos[v]` 先把 99% 的 writes×reads 对
//!     剪枝掉，再做真正的位图查询。

use crate::verify::Check;
use mox_ai_flow_svc::model::{FlowGraph, Reachability};
use std::collections::BTreeSet;

/// 5a 拓扑守恒（语义级）：公开入口（保持旧签名）
pub fn topology_invariant(before: &FlowGraph, after: &FlowGraph) -> Check {
    topology_invariant_with_reach(before, after, None)
}

/// 5a 拓扑守恒：允许调用方传入缓存的 `after` 可达性。
pub fn topology_invariant_with_reach(
    before: &FlowGraph,
    after: &FlowGraph,
    after_reach_cached: Option<&Reachability>,
) -> Check {
    // 1) 原始节点必须全部保留
    let b_ids: BTreeSet<&str> = before.nodes.iter().map(|n| n.id.as_str()).collect();
    let a_ids: BTreeSet<&str> = after.nodes.iter().map(|n| n.id.as_str()).collect();
    let missing: Vec<&str> = b_ids.difference(&a_ids).copied().collect();
    if !missing.is_empty() {
        return Check {
            name: "topology".into(),
            passed: false,
            blocking: true,
            detail: format!(
                "原始节点在优化后丢失: {:?}",
                &missing[..missing.len().min(5)]
            ),
        };
    }

    // 2) 真数据依赖对的语义可达性守恒：
    //    对 (u, v) 若 before 上 u 写 ∩ v 读 ≠ ∅ 且 before.u→v，则 after 上也必须 u→v。
    //
    // 性能：before 传闭包只用一次；after 传闭包优先复用调用方缓存。
    // 同时取「拓扑秩」——深链是严格 DAG，pos[u] >= pos[v] 时 reaches(u, v) = false，
    // 这能把 O(W*R) 的 n² 对几乎全部短-circuit。
    let (rb, b_pos) = before.reachability_with_topo_pos();
    let ra_owned;
    let (ra, a_pos_vec): (&Reachability, Vec<Option<usize>>) = match after_reach_cached {
        Some(r) => {
            // 复用缓存：仍要算一次 after 的 topo_pos（拓扑位置本身比传闭包便宜 ~100×）
            let order = after
                .topo_order()
                .unwrap_or_else(|_| (0..after.nodes.len()).collect());
            let mut pos = vec![None; after.nodes.len()];
            for (rank, &u) in order.iter().enumerate() {
                pos[u] = Some(rank);
            }
            (r, pos)
        }
        None => {
            let (r, pos) = after.reachability_with_topo_pos();
            ra_owned = r;
            (&ra_owned, pos)
        }
    };

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
        let ui = match before.index_of(u) {
            Some(i) => i,
            None => continue,
        };
        let ai = match after.index_of(u) {
            Some(i) => i,
            None => continue,
        };
        let bu_pos = match b_pos[ui] {
            Some(p) => p,
            None => continue,
        };
        let au_pos = match a_pos_vec[ai] {
            Some(p) => p,
            None => continue,
        };
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
                continue;
            }
            let vi = match before.index_of(v) {
                Some(i) => i,
                None => continue,
            };
            let avi = match after.index_of(v) {
                Some(i) => i,
                None => continue,
            };
            let bv_pos = match b_pos[vi] {
                Some(p) => p,
                None => continue,
            };
            // 拓扑秩短切：before 上 u 的秩 ≥ v，就不可能 u→* v
            let before_reach = if bu_pos < bv_pos {
                rb.reaches(ui, vi)
            } else {
                false
            };
            if !before_reach {
                continue;
            }
            // after 上同理：秩短切
            let av_pos = match a_pos_vec[avi] {
                Some(p) => p,
                None => continue,
            };
            let after_reach = if au_pos < av_pos {
                ra.reaches(ai, avi)
            } else {
                false
            };
            if after_reach {
                continue;
            }
            mismatch.push(format!(
                "{u}→{v}（写 {:?} 必须早于读，但可达性被破坏）",
                writes
            ));
            if mismatch.len() >= 5 {
                // 诊断只收集前 5 例，避免巨型反例
                break;
            }
        }
        if mismatch.len() >= 5 {
            break;
        }
    }
    if !mismatch.is_empty() {
        return Check {
            name: "topology".into(),
            passed: false,
            blocking: true,
            detail: format!(
                "真数据依赖可达性被破坏（≤5 例）: {:?}",
                &mismatch[..mismatch.len().min(5)]
            ),
        };
    }
    Check {
        name: "topology".into(),
        passed: true,
        blocking: true,
        detail: format!("原始节点 {} 全部保留，真数据依赖可达性守恒", b_ids.len()),
    }
}
