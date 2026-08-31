// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! DAG 工具：循环检测 + 拓扑排序
//!
//! 从 flow_engine 的 detect_cycle 函数提取并增强，
//! 增加拓扑排序功能用于分层执行。

use crate::error::FlowResult;
use crate::types::UnifiedFlowGraph;

/// 检测图中是否存在循环
///
/// 使用 DFS 算法，时间复杂度 O(V + E)。
/// 返回 true 表示存在循环。
pub fn detect_cycle(graph: &UnifiedFlowGraph) -> FlowResult<bool> {
    let mut adj: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for edge in &graph.edges {
        adj.entry(edge.source.clone())
            .or_default()
            .push(edge.target.clone());
    }

    let mut visited = std::collections::HashSet::new();
    let mut stack = std::collections::HashSet::new();

    for node in &graph.nodes {
        if !visited.contains(&node.id) && dfs_cycle(&node.id, &adj, &mut visited, &mut stack) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// DFS 循环检测辅助函数
fn dfs_cycle(
    node: &str,
    adj: &std::collections::HashMap<String, Vec<String>>,
    visited: &mut std::collections::HashSet<String>,
    stack: &mut std::collections::HashSet<String>,
) -> bool {
    visited.insert(node.to_string());
    stack.insert(node.to_string());

    if let Some(neighbors) = adj.get(node) {
        for neighbor in neighbors {
            if stack.contains(neighbor) {
                return true;
            }
            if !visited.contains(neighbor) && dfs_cycle(neighbor, adj, visited, stack) {
                return true;
            }
        }
    }

    stack.remove(node);
    false
}

/// 拓扑排序（Kahn 算法）
///
/// 返回执行层（每层是一组可以并行执行的节点 ID）。
/// 如果图中存在循环，返回错误。
pub fn topo_sort(graph: &UnifiedFlowGraph) -> FlowResult<Vec<Vec<String>>> {
    let mut in_degree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut adj: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    // 初始化
    for node in &graph.nodes {
        in_degree.insert(node.id.clone(), 0);
        adj.insert(node.id.clone(), Vec::new());
    }

    // 构建邻接表和入度
    for edge in &graph.edges {
        adj.entry(edge.source.clone())
            .or_default()
            .push(edge.target.clone());
        *in_degree.entry(edge.target.clone()).or_insert(0) += 1;
    }

    // BFS 拓扑排序
    let mut layers: Vec<Vec<String>> = Vec::new();
    let mut current_layer: Vec<String> = in_degree
        .iter()
        .filter(|(_, &degree)| degree == 0)
        .map(|(id, _)| id.clone())
        .collect();

    let mut processed = 0;
    let total_nodes = graph.nodes.len();

    while !current_layer.is_empty() {
        let next_layer: Vec<String> = Vec::new();
        let mut next_in_degree = in_degree.clone();

        let mut next_layer_set = std::collections::HashSet::new();

        for node_id in &current_layer {
            processed += 1;
            if let Some(neighbors) = adj.get(node_id) {
                for neighbor in neighbors {
                    if let Some(degree) = next_in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            next_layer_set.insert(neighbor.clone());
                        }
                    }
                }
            }
        }

        in_degree = next_in_degree;
        layers.push(current_layer);
        current_layer = next_layer_set.into_iter().collect();
    }

    if processed != total_nodes {
        return Err(crate::error::UnifiedFlowError::CycleDetected(
            "拓扑排序失败，存在循环".into(),
        ));
    }

    Ok(layers)
}

/// 计算关键路径长度（基于节点 duration_ms）
pub fn critical_path_ms(graph: &UnifiedFlowGraph) -> FlowResult<u64> {
    let layers = topo_sort(graph)?;

    // 动态规划：earliest_finish[node] = 节点最早完成时间
    let mut earliest_finish: std::collections::HashMap<String, u64> =
        std::collections::HashMap::new();

    for layer in &layers {
        for node_id in layer {
            let node = graph
                .node(node_id)
                .ok_or_else(|| crate::error::UnifiedFlowError::NodeNotFound(node_id.clone()))?;

            // 找所有前驱节点的最早完成时间
            let max_prev = graph
                .incoming_edges(node_id)
                .iter()
                .map(|e| earliest_finish.get(&e.source).copied().unwrap_or(0))
                .max()
                .unwrap_or(0);

            earliest_finish.insert(node_id.clone(), max_prev + node.duration_ms);
        }
    }

    // 关键路径长度 = 所有 End 节点的最大最早完成时间
    let cp = graph
        .end_nodes()
        .iter()
        .map(|n| earliest_finish.get(&n.id).copied().unwrap_or(0))
        .max()
        .unwrap_or(0);

    Ok(cp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn linear_graph() -> UnifiedFlowGraph {
        let mut g = UnifiedFlowGraph::new("test", "线性图");
        g.add_node(UnifiedFlowNode::new("a", "A", UnifiedNodeKind::Start));
        g.add_node(
            UnifiedFlowNode::task("b", "B", UnifiedToolKind::Compute, 100)
        );
        g.add_node(UnifiedFlowNode::new("c", "C", UnifiedNodeKind::End));
        g.add_edge(UnifiedFlowEdge::seq("e1", "a", "b"));
        g.add_edge(UnifiedFlowEdge::seq("e2", "b", "c"));
        g
    }

    fn parallel_graph() -> UnifiedFlowGraph {
        let mut g = UnifiedFlowGraph::new("test", "并行图");
        g.add_node(UnifiedFlowNode::new("start", "Start", UnifiedNodeKind::Start));
        g.add_node(
            UnifiedFlowNode::task("a", "A", UnifiedToolKind::Compute, 100),
        );
        g.add_node(
            UnifiedFlowNode::task("b", "B", UnifiedToolKind::Compute, 200),
        );
        g.add_node(UnifiedFlowNode::new("end", "End", UnifiedNodeKind::End));
        g.add_edge(UnifiedFlowEdge::seq("e1", "start", "a"));
        g.add_edge(UnifiedFlowEdge::seq("e2", "start", "b"));
        g.add_edge(UnifiedFlowEdge::seq("e3", "a", "end"));
        g.add_edge(UnifiedFlowEdge::seq("e4", "b", "end"));
        g
    }

    fn cyclic_graph() -> UnifiedFlowGraph {
        let mut g = UnifiedFlowGraph::new("test", "循环图");
        g.add_node(UnifiedFlowNode::new("a", "A", UnifiedNodeKind::Start));
        g.add_node(
            UnifiedFlowNode::task("b", "B", UnifiedToolKind::Compute, 100),
        );
        g.add_node(UnifiedFlowNode::new("c", "C", UnifiedNodeKind::End));
        g.add_edge(UnifiedFlowEdge::seq("e1", "a", "b"));
        g.add_edge(UnifiedFlowEdge::seq("e2", "b", "c"));
        g.add_edge(UnifiedFlowEdge::seq("e3", "c", "a")); // 回边
        g
    }

    #[test]
    fn test_detect_cycle_linear() {
        let g = linear_graph();
        assert!(!detect_cycle(&g).unwrap());
    }

    #[test]
    fn test_detect_cycle_parallel() {
        let g = parallel_graph();
        assert!(!detect_cycle(&g).unwrap());
    }

    #[test]
    fn test_detect_cycle_cyclic() {
        let g = cyclic_graph();
        assert!(detect_cycle(&g).unwrap());
    }

    #[test]
    fn test_topo_sort_linear() {
        let g = linear_graph();
        let layers = topo_sort(&g).unwrap();
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0], vec!["a"]);
        assert_eq!(layers[1], vec!["b"]);
        assert_eq!(layers[2], vec!["c"]);
    }

    #[test]
    fn test_topo_sort_parallel() {
        let g = parallel_graph();
        let layers = topo_sort(&g).unwrap();
        // start 在第一层，a+b 在第二层，end 在第三层
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].len(), 1); // start
        assert_eq!(layers[1].len(), 2); // a, b
        assert_eq!(layers[2].len(), 1); // end
    }

    #[test]
    fn test_topo_sort_cyclic() {
        let g = cyclic_graph();
        assert!(topo_sort(&g).is_err());
    }

    #[test]
    fn test_critical_path() {
        let g = parallel_graph();
        let cp = critical_path_ms(&g).unwrap();
        // 关键路径：start(0) -> b(200) -> end(0) = 200
        assert_eq!(cp, 200);
    }
}
