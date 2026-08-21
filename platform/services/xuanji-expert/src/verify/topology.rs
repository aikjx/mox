//! 5a 拓扑守恒（语义级）

use crate::verify::Check;
use flow_ai::model::FlowGraph;
use std::collections::BTreeSet;

/// 5a 拓扑守恒（语义级）：
/// 1) 原始节点全部保留（允许 flow-ai 新增 guard/handler）；
/// 2) 任何「写→读」真数据依赖对的**可达性必须保持**（被优化器挪动会导致读早于写，属语义破坏）。
///    普通控制边/无数据共享的并行化不算破坏（flow-ai 合法剪除伪依赖）。
pub fn topology_invariant(before: &FlowGraph, after: &FlowGraph) -> Check {
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
