//! Rust 侧 7 条核心算法对账导出工具（对应 Node 侧 ai-flow-graph.js 的 7 算法 + 8 个标准数据集 T1..T8）
//!
//! 用法：
//! ```
//! cargo run -p graph-algorithms --bin export_formula -- iterations=20 datasets=T1..T8 > graph_algorithms_7x8_rust.json
//! ```
//!
//! 输出格式（JSON Array，每项 = 1 算法 × 1 数据集，共 7×8=56 项，用于 Node 侧 diff 对账 Δ≤1e-6）：
//! ```json
//! [
//!   {
//!     "dataset": "T1",
//!     "algorithm": "pagerank",
//!     "primary_impl": "RUST",
//!     "params": {"iterations": 20},
//!     "result": {"node_id_1": 0.0423, ..., "node_id_n": 0.0567}
//!   },
//!   ...
//! ]
//! ```

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use graph_algorithms::{Community, KnowledgeEdge, KnowledgeGraph, KnowledgeNode};
use serde::{Deserialize, Serialize};

/// 对账算法名（7 条，顺序固定，保证 56 项连续）
const ALGORITHMS: &[&str] = &[
    "pagerank",        // 推模型 PageRank
    "cnm",             // CNM 模块度贪心社区检测（同时输出 modularity_scalar 与社区列表）
    "betweenness",     // Brandes 介数中心性
    "harmonic",        // Harmonic 紧密中心性（closeness 语义，对齐 Node 端 F5 公式）
    "degree",          // 度中心性（无向 RAW 展开：in+out）
    "density",         // 图密度（有向归一：m/(n(n-1))）
    "modularity",      // 模块度：对 CNM 社区划分计算 Q 值；与 Node 端 modularity 算法对齐
];

/// 标准 8 个测试数据集（企业级算法对账基准：稀疏、稠密、悬挂、不连通、树、网格、长尾、加权环）
fn datasets() -> HashMap<&'static str, KnowledgeGraph> {
    let mut m: HashMap<&'static str, KnowledgeGraph> = HashMap::new();
    // 权重工具：全部边权重 1.0（7 条核心算法均为无权；对账的数值定义不涉及权重加权）
    let w = 1.0;
    let mk = |vs: &[&str], es: &[(&str, &str)]| -> KnowledgeGraph {
        let mut g = KnowledgeGraph::new();
        for v in vs {
            g.add_node(KnowledgeNode {
                id: (*v).to_string(),
                label: (*v).to_string(),
                node_type: "vertex".to_string(),
                properties: serde_json::Value::Object(serde_json::Map::new()),
                embedding: None,
                activation: 0.0,
                metadata: HashMap::new(),
            });
        }
        for (a, b) in es {
            let edge = KnowledgeEdge {
                source: (*a).to_string(),
                target: (*b).to_string(),
                weight: w,
                relation_type: "e".to_string(),
                properties: serde_json::Value::Object(serde_json::Map::new()),
            };
            let _ = g.add_edge(edge);
        }
        g
    };

    // T1 稀疏图：6 节点 7 边链 + 1 条回边（典型业务拓扑）
    m.insert("T1", mk(
        &["a", "b", "c", "d", "e", "f"],
        &[("a", "b"), ("b", "c"), ("c", "d"), ("d", "e"), ("e", "f"), ("f", "c"), ("a", "d")],
    ));
    // T2 稠密图（5 节点完全图，K5）
    let nodes5 = &["p", "q", "r", "s", "t"];
    let mut k5_edges = vec![];
    for i in 0..5 { for j in 0..5 { if i != j { k5_edges.push((nodes5[i], nodes5[j])); }}}
    m.insert("T2", mk(nodes5, &k5_edges));
    // T3 悬挂节点（星 + 1 叶子无出边，用于验证 PR dangling mass 回传）
    m.insert("T3", mk(
        &["hub", "s1", "s2", "s3", "s4", "leaf"],
        &[("hub", "s1"), ("hub", "s2"), ("hub", "s3"), ("hub", "s4"),
          ("s1", "hub"), ("s2", "hub"), ("s3", "hub"), ("s4", "leaf")],
    ));
    // T4 不连通图（两个独立连通分支，用于验证 harmonic 对不可达 0 贡献）
    m.insert("T4", mk(
        &["A1", "A2", "A3", "B1", "B2"],
        &[("A1", "A2"), ("A2", "A3"), ("A3", "A1"), ("B1", "B2"), ("B2", "B1")],
    ));
    // T5 树（7 节点二叉树，用于验证介数 Brandes）
    m.insert("T5", mk(
        &["root", "l", "r", "ll", "lr", "rl", "rr"],
        &[("root", "l"), ("root", "r"), ("l", "root"), ("r", "root"),
          ("l", "ll"), ("l", "lr"), ("r", "rl"), ("r", "rr"),
          ("ll", "l"), ("lr", "l"), ("rl", "r"), ("rr", "r")],
    ));
    // T6 网格（3x3 栅格，9 节点，用于密度）
    let grid = &["n11", "n12", "n13", "n21", "n22", "n23", "n31", "n32", "n33"];
    let mut grid_edges = vec![];
    let g = |i: usize, j: usize| -> &str { grid[(i - 1) * 3 + (j - 1)] };
    for i in 1..=3 {
        for j in 1..=3 {
            if i < 3 { grid_edges.push((g(i, j), g(i + 1, j))); grid_edges.push((g(i + 1, j), g(i, j))); }
            if j < 3 { grid_edges.push((g(i, j), g(i, j + 1))); grid_edges.push((g(i, j + 1), g(i, j))); }
        }
    }
    m.insert("T6", mk(grid, &grid_edges));
    // T7 长尾幂律图（hub + 10 叶子，幂律分布验证度中心性）
    let mut ln = vec!["hub"];
    for i in 1..=10 { ln.push(Box::leak(format!("u{i}").into_boxed_str())); }
    let mut tail = vec![];
    for i in 1..=10 {
        tail.push(("hub", ln[i]));
        tail.push((ln[i], "hub"));
        if i > 1 { // 幂律：部分 hubs 之间还有 1-2 连接，长尾不互连
            tail.push((ln[i - 1], ln[i]));
        }
    }
    m.insert("T7", mk(&ln, &tail));
    // T8 加权环（8 节点双向环；权重仍 1.0，用于社区 4 vs 4 划分的 modularity 测试）
    let ring: Vec<&str> = (1..=8).map(|i| Box::leak(format!("r{i}").into_boxed_str()) as &str).collect();
    let mut ring_edges = vec![];
    for i in 0..8 {
        let a = ring[i]; let b = ring[(i + 1) % 8];
        ring_edges.push((a, b)); ring_edges.push((b, a));
    }
    // 额外连接 r1-r3, r5-r7 制造两个环簇（使 CNM 倾向分成 4+4 社区）
    ring_edges.push(("r1", "r3")); ring_edges.push(("r3", "r1"));
    ring_edges.push(("r5", "r7")); ring_edges.push(("r7", "r5"));
    m.insert("T8", {
        let mut g = KnowledgeGraph::new();
        for v in &ring {
            g.add_node(KnowledgeNode {
                id: (*v).to_string(),
                label: (*v).to_string(),
                node_type: "vertex".to_string(),
                properties: serde_json::Value::Object(serde_json::Map::new()),
                embedding: None,
                activation: 0.0,
                metadata: HashMap::new(),
            });
        }
        for (a, b) in ring_edges {
            let edge = KnowledgeEdge {
                source: a.to_string(), target: b.to_string(),
                weight: 1.0, relation_type: "e".to_string(),
                properties: serde_json::Value::Object(serde_json::Map::new()),
            };
            let _ = g.add_edge(edge);
        }
        g
    });

    m
}

#[derive(Serialize, Deserialize)]
struct Record {
    dataset: String,
    algorithm: String,
    primary_impl: &'static str,
    params: serde_json::Value,
    result: serde_json::Value,
}

fn map_to_object(map: &HashMap<String, f64>) -> serde_json::Value {
    let mut m = serde_json::Map::new();
    for (k, v) in map { m.insert(k.clone(), serde_json::json!(v)); }
    serde_json::Value::Object(m)
}

/// 模块化度 Q = Σ_c (Σ_in/2m − (Σ_tot/2m)²)
///
/// communities: 每个社区的节点 id 列表（CNM 返回 vec![]）；公式与 Node 端 ai-flow-graph.js
/// 的 modularity 算法完全一致（无向语义：边双向对称 → m 按无向边数计算）。
fn compute_modularity(g: &KnowledgeGraph, communities: &[Community]) -> f64 {
    let ids: HashMap<String, usize> = g.node_ids().into_iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();
    let n = ids.len();
    if n == 0 { return 0.0; }
    // 无向边集（基于字符串 source/target → 用 ids 映射 idx；去重 s<t）
    let mut edge_set: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    for e in g.edges() {
        let si = match ids.get(&e.source) { Some(i) => *i, None => continue };
        let ti = match ids.get(&e.target) { Some(i) => *i, None => continue };
        if si != ti { edge_set.insert((si.min(ti), si.max(ti))); }
    }
    let m = edge_set.len() as f64;
    if m <= 0.0 { return 0.0; }

    // 节点 idx → comm_idx
    let mut node_comm: Vec<Option<usize>> = vec![None; n];
    for (ci, c) in communities.iter().enumerate() {
        for v in &c.nodes { if let Some(&i) = ids.get(v) { node_comm[i] = Some(ci); } }
    }

    let k = communities.len().max(1);
    let mut sum_in = vec![0.0f64; k];
    let mut sum_tot = vec![0.0f64; k];
    let mut deg = vec![0f64; n];
    for &(s, t) in &edge_set { deg[s] += 1.0; deg[t] += 1.0; }
    for (i, d) in deg.iter().enumerate() {
        if let Some(c) = node_comm[i] { sum_tot[c] += *d; }
    }
    for &(s, t) in &edge_set {
        if let (Some(c1), Some(c2)) = (node_comm[s], node_comm[t]) {
            if c1 == c2 { sum_in[c1] += 1.0; }
        }
    }

    let two_m = 2.0 * m;
    let mut q = 0.0f64;
    for c in 0..k {
        q += (sum_in[c] / two_m) - (sum_tot[c] / two_m).powi(2);
    }
    q
}

fn main() -> Result<()> {
    let mut iterations: usize = 20;
    // 最小 CLI：支持 args key=value
    for a in std::env::args().skip(1) {
        if let Some(v) = a.strip_prefix("iterations=") { iterations = v.parse().unwrap_or(20); }
        if a.starts_with("datasets=") { /* 未来可过滤；当前固定 T1..T8 */ }
    }

    let ds = datasets();
    let dataset_order: &[&str] = &["T1", "T2", "T3", "T4", "T5", "T6", "T7", "T8"];
    let mut out: Vec<Record> = Vec::with_capacity(ALGORITHMS.len() * dataset_order.len());

    for d in dataset_order {
        let g = ds.get(d).ok_or_else(|| anyhow!("缺失数据集 {d}"))?;
        for &algo in ALGORITHMS {
            let rec = match algo {
                "pagerank" => {
                    let pr = g.pagerank(iterations);
                    Record {
                        dataset: d.to_string(), algorithm: "pagerank".to_string(),
                        primary_impl: "RUST",
                        params: serde_json::json!({"iterations": iterations}),
                        result: map_to_object(&pr),
                    }
                }
                "cnm" => {
                    let cs = g.detect_communities(iterations.max(1000));
                    let q = compute_modularity(g, &cs);
                    let communities_json: Vec<serde_json::Value> = cs.iter()
                        .map(|c| serde_json::json!({
                            "id": c.id, "nodes": c.nodes, "density": c.density, "label": c.label, "size": c.nodes.len()
                        })).collect();
                    Record {
                        dataset: d.to_string(), algorithm: "cnm".to_string(),
                        primary_impl: "RUST",
                        params: serde_json::json!({"iterations": iterations.max(1000)}),
                        result: serde_json::json!({"communities": communities_json, "community_count": cs.len(), "modularity": q}),
                    }
                }
                "betweenness" => {
                    let b = g.betweenness_centrality();
                    Record {
                        dataset: d.to_string(), algorithm: "betweenness".to_string(),
                        primary_impl: "RUST",
                        params: serde_json::json!({}),
                        result: map_to_object(&b),
                    }
                }
                "harmonic" => {
                    let h = g.closeness_centrality(); // 已经是 harmonic (Σ 1/d) / (N-1)
                    Record {
                        dataset: d.to_string(), algorithm: "harmonic".to_string(),
                        primary_impl: "RUST",
                        params: serde_json::json!({}),
                        result: map_to_object(&h),
                    }
                }
                "degree" => {
                    let dmap = g.degree_centrality();
                    Record {
                        dataset: d.to_string(), algorithm: "degree".to_string(),
                        primary_impl: "RUST",
                        params: serde_json::json!({}),
                        result: map_to_object(&dmap),
                    }
                }
                "density" => {
                    let s = g.stats();
                    Record {
                        dataset: d.to_string(), algorithm: "density".to_string(),
                        primary_impl: "RUST",
                        params: serde_json::json!({}),
                        result: serde_json::json!({
                            "density": s.density,
                            "node_count": s.node_count,
                            "edge_count": s.edge_count,
                            "average_degree": s.average_degree,
                        }),
                    }
                }
                "modularity" => {
                    let cs = g.detect_communities(iterations.max(1000));
                    let q = compute_modularity(g, &cs);
                    Record {
                        dataset: d.to_string(), algorithm: "modularity".to_string(),
                        primary_impl: "RUST",
                        params: serde_json::json!({}),
                        result: serde_json::json!({
                            "modularity": q,
                            "community_count": cs.len(),
                            "communities": cs.iter().map(|c| c.nodes.clone()).collect::<Vec<_>>(),
                        }),
                    }
                }
                _ => unreachable!(),
            };
            out.push(rec);
        }
    }

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
