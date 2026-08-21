//! 关键路径法 (CPM) —— 完整前向/后向遍历 + 浮动时间
//!
//! 相比原 `optimizer` crate 的简化实现，本模块：
//! 1. 前向计算 ES/EF，后向计算 LS/LF，得到每个节点的总浮动 (Total Float)；
//! 2. 关键路径 = 浮动为 0 的节点链，支持**多条并列关键路径**；
//! 3. 输出浮动排名，直接指明「优化哪个节点能压缩总工期」。

use crate::dataflow::Dependency;
use crate::model::FlowGraph;
use serde::{Deserialize, Serialize};

/// 单节点时间分析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTiming {
    pub id: String,
    pub duration_ms: u64,
    /// 最早开始
    pub es: u64,
    /// 最早结束
    pub ef: u64,
    /// 最晚开始
    pub ls: u64,
    /// 最晚结束
    pub lf: u64,
    /// 总浮动 = ls - es，0 表示在关键路径上
    pub total_float: u64,
    pub critical: bool,
}

/// 关键路径分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalPathReport {
    pub timings: Vec<NodeTiming>,
    /// 所有并列关键路径（节点 id 序列）
    pub critical_paths: Vec<Vec<String>>,
    /// 工期（ms）
    pub makespan_ms: u64,
    /// 优化优先级：按 duration 降序的关键节点
    pub optimization_targets: Vec<String>,
}

impl CriticalPathReport {
    pub fn timing(&self, id: &str) -> Option<&NodeTiming> {
        self.timings.iter().find(|t| t.id == id)
    }
}

/// 基于依赖集合做 CPM 分析
pub fn analyze(graph: &FlowGraph, deps: &[Dependency]) -> CriticalPathReport {
    let n = graph.nodes.len();
    let mut succ: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut pred: Vec<Vec<usize>> = vec![Vec::new(); n];
    for d in deps {
        let (Some(u), Some(v)) = (graph.index_of(&d.from), graph.index_of(&d.to)) else {
            continue;
        };
        succ[u].push(v);
        pred[v].push(u);
    }

    // 拓扑序（在依赖图上）
    let order = topo(n, &succ, &pred);

    // --- 前向遍历: ES / EF ---
    let mut es = vec![0u64; n];
    let mut ef = vec![0u64; n];
    for &u in &order {
        es[u] = pred[u].iter().map(|&p| ef[p]).max().unwrap_or(0);
        ef[u] = es[u] + graph.nodes[u].duration_ms;
    }
    let makespan = ef.iter().copied().max().unwrap_or(0);

    // --- 后向遍历: LF / LS ---
    let mut lf = vec![makespan; n];
    let mut ls = vec![0u64; n];
    for &u in order.iter().rev() {
        lf[u] = succ[u]
            .iter()
            .map(|&s| ls[s])
            .min()
            .unwrap_or(makespan);
        ls[u] = lf[u].saturating_sub(graph.nodes[u].duration_ms);
    }

    let timings: Vec<NodeTiming> = (0..n)
        .map(|i| {
            let tf = ls[i].saturating_sub(es[i]);
            NodeTiming {
                id: graph.nodes[i].id.clone(),
                duration_ms: graph.nodes[i].duration_ms,
                es: es[i],
                ef: ef[i],
                ls: ls[i],
                lf: lf[i],
                total_float: tf,
                critical: tf == 0,
            }
        })
        .collect();

    // --- 枚举关键路径（浮动为 0 的子图上做 DFS） ---
    let critical_paths = enumerate_critical_paths(graph, &succ, &pred, &timings);

    let mut targets: Vec<(String, u64)> = timings
        .iter()
        .filter(|t| t.critical && t.duration_ms > 0)
        .map(|t| (t.id.clone(), t.duration_ms))
        .collect();
    targets.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    CriticalPathReport {
        timings,
        critical_paths,
        makespan_ms: makespan,
        optimization_targets: targets.into_iter().map(|(id, _)| id).collect(),
    }
}

fn topo(n: usize, succ: &[Vec<usize>], pred: &[Vec<usize>]) -> Vec<usize> {
    let mut indeg: Vec<usize> = (0..n).map(|i| pred[i].len()).collect();
    let mut q: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    let mut out = Vec::with_capacity(n);
    let mut head = 0;
    while head < q.len() {
        let u = q[head];
        head += 1;
        out.push(u);
        for &v in &succ[u] {
            indeg[v] -= 1;
            if indeg[v] == 0 {
                q.push(v);
            }
        }
    }
    // 有环时补齐剩余（防御性）
    for i in 0..n {
        if !out.contains(&i) {
            out.push(i);
        }
    }
    out
}

fn enumerate_critical_paths(
    graph: &FlowGraph,
    succ: &[Vec<usize>],
    pred: &[Vec<usize>],
    timings: &[NodeTiming],
) -> Vec<Vec<String>> {
    let n = graph.nodes.len();
    let crit = |i: usize| timings[i].critical;
    let starts: Vec<usize> = (0..n).filter(|&i| crit(i) && pred[i].iter().all(|&p| !crit(p))).collect();

    let mut paths = Vec::new();
    let mut stack: Vec<(usize, Vec<usize>)> = starts.into_iter().map(|s| (s, vec![s])).collect();
    // 限制枚举数量，避免组合爆炸
    const MAX_PATHS: usize = 32;
    while let Some((u, path)) = stack.pop() {
        if paths.len() >= MAX_PATHS {
            break;
        }
        // 关键后继：必须紧邻（ef[u] == es[v]）且自身关键
        let nexts: Vec<usize> = succ[u]
            .iter()
            .copied()
            .filter(|&v| crit(v) && timings[u].ef == timings[v].es)
            .collect();
        if nexts.is_empty() {
            paths.push(path.iter().map(|&i| graph.nodes[i].id.clone()).collect());
        } else {
            for v in nexts {
                let mut p = path.clone();
                p.push(v);
                stack.push((v, p));
            }
        }
    }
    paths.sort();
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow;
    use crate::model::{Access, FlowEdge, FlowNode, ToolKind};

    fn diamond() -> FlowGraph {
        let mut g = FlowGraph::new("d", "diamond");
        g.add_node(FlowNode::task("a", "A", ToolKind::Compute, 100).with_access(Access::write("x")));
        g.add_node(
            FlowNode::task("b", "B", ToolKind::Compute, 300)
                .with_access(Access::read("x"))
                .with_access(Access::write("b_out")),
        );
        g.add_node(
            FlowNode::task("c", "C", ToolKind::Compute, 50)
                .with_access(Access::read("x"))
                .with_access(Access::write("c_out")),
        );
        g.add_node(
            FlowNode::task("d", "D", ToolKind::Compute, 100)
                .with_access(Access::read("b_out"))
                .with_access(Access::read("c_out")),
        );
        g.add_edge(FlowEdge::seq("a", "b"));
        g.add_edge(FlowEdge::seq("a", "c"));
        g.add_edge(FlowEdge::seq("b", "d"));
        g.add_edge(FlowEdge::seq("c", "d"));
        g
    }

    #[test]
    fn critical_path_is_longest() {
        let g = diamond();
        let plan = dataflow::analyze(&g);
        let rep = analyze(&g, &plan.dependencies);
        assert_eq!(rep.makespan_ms, 500); // 100 + 300 + 100
        assert!(rep.critical_paths.iter().any(|p| p == &vec![
            "a".to_string(),
            "b".to_string(),
            "d".to_string()
        ]));
    }

    #[test]
    fn float_identifies_slack() {
        let g = diamond();
        let plan = dataflow::analyze(&g);
        let rep = analyze(&g, &plan.dependencies);
        let c = rep.timing("c").unwrap();
        assert_eq!(c.total_float, 250, "C 有 250ms 浮动");
        assert!(!c.critical);
        let b = rep.timing("b").unwrap();
        assert_eq!(b.total_float, 0);
    }

    #[test]
    fn optimization_target_is_biggest_critical_node() {
        let g = diamond();
        let plan = dataflow::analyze(&g);
        let rep = analyze(&g, &plan.dependencies);
        assert_eq!(rep.optimization_targets.first().map(|s| s.as_str()), Some("b"));
    }
}
