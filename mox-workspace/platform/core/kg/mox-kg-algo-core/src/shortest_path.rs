//! 最短路径算法
//!
//! BFS（无权图）/ Dijkstra（带权图）

use std::collections::{HashMap, VecDeque};
use mox_kg_meta_core::{Graph, NodeId};

/// BFS 最短路径（无权图）
///
/// 返回从源点到各节点的最短距离。
pub fn bfs_shortest_path(graph: &Graph, source: &NodeId) -> HashMap<NodeId, u32> {
    let mut distances: HashMap<NodeId, u32> = graph
        .node_ids()
        .iter()
        .map(|id| ((*id).clone(), u32::MAX))
        .collect();

    let mut queue = VecDeque::new();
    distances.insert(source.clone(), 0);
    queue.push_back(source.clone());

    while let Some(node) = queue.pop_front() {
        let current_dist = *distances.get(&node).unwrap();

        for neighbor in graph.neighbors(&node) {
            if *distances.get(neighbor).unwrap() == u32::MAX {
                distances.insert(neighbor.clone(), current_dist + 1);
                queue.push_back(neighbor.clone());
            }
        }
    }

    distances
}

/// 获取两点间的最短路径长度
pub fn shortest_path_length(graph: &Graph, source: &NodeId, target: &NodeId) -> Option<u32> {
    let distances = bfs_shortest_path(graph, source);
    distances.get(target).copied().filter(|&d| d != u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_kg_meta_core::{GraphNode, GraphEdge};
    use std::collections::HashMap;

    fn build_linear_graph() -> Graph {
        let mut g = Graph::new();
        for i in 0..4 {
            g.add_node(GraphNode {
                id: format!("n{}", i),
                label: "n".into(),
                properties: HashMap::new(),
            });
        }
        for i in 0..3 {
            g.add_edge(GraphEdge {
                id: format!("e{}", i),
                from: format!("n{}", i),
                to: format!("n{}", i + 1),
                label: "l".into(),
                properties: HashMap::new(),
                directed: false,
            });
        }
        g
    }

    #[test]
    fn test_bfs_linear() {
        let g = build_linear_graph();
        let distances = bfs_shortest_path(&g, &"n0".into());
        assert_eq!(*distances.get("n0").unwrap(), 0);
        assert_eq!(*distances.get("n1").unwrap(), 1);
        assert_eq!(*distances.get("n2").unwrap(), 2);
        assert_eq!(*distances.get("n3").unwrap(), 3);
    }

    #[test]
    fn test_shortest_path_length() {
        let g = build_linear_graph();
        assert_eq!(shortest_path_length(&g, &"n0".into(), &"n3".into()), Some(3));
    }
}
