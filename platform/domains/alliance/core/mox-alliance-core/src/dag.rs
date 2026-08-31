// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! DAG（有向无环图）算法工具
//!
//! 提供 DAG 相关的纯算法实现：
//! - 拓扑排序
//! - 环检测
//! - 依赖分析
//! - 关键路径计算
//!
//! 所有函数都是纯函数，无 IO，可独立单测。

use std::collections::{HashMap, HashSet, VecDeque};

use mox_alliance_common_proto::{AllianceError, AllianceResult, Node, NodeStatus};

/// 拓扑排序结果
#[derive(Debug, Clone)]
pub struct TopoSortResult {
    /// 拓扑排序后的节点 ID 列表（按依赖顺序）
    pub order: Vec<String>,
    /// 每层的节点（按层级分组）
    pub layers: Vec<Vec<String>>,
    /// 每个节点的入度
    pub in_degree: HashMap<String, usize>,
}

/// Kahn 算法拓扑排序
///
/// 返回按依赖顺序排列的节点列表（无依赖的在前）。
/// 如果检测到环，返回错误。
pub fn topological_sort(nodes: &[Node]) -> AllianceResult<TopoSortResult> {
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    let mut node_ids: HashSet<String> = HashSet::new();

    // 初始化
    for node in nodes {
        node_ids.insert(node.node_id.clone());
        in_degree.entry(node.node_id.clone()).or_insert(0);
        adjacency.entry(node.node_id.clone()).or_default();
    }

    // 构建邻接表和入度
    for node in nodes {
        for dep in &node.dependencies {
            if !node_ids.contains(dep) {
                return Err(AllianceError::invalid_argument(format!(
                    "Node {} depends on non-existent node {}",
                    node.node_id, dep
                )));
            }
            // dep -> node（依赖项指向当前节点）
            adjacency
                .entry(dep.clone())
                .or_default()
                .push(node.node_id.clone());
            *in_degree.get_mut(&node.node_id).unwrap() += 1;
        }
    }

    // Kahn 算法
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut order: Vec<String> = Vec::new();
    let mut layers: Vec<Vec<String>> = Vec::new();

    // 入度为 0 的节点入队（第一层）
    for (id, &deg) in &in_degree {
        if deg == 0 {
            queue.push_back(id.clone());
        }
    }

    // 按层处理
    while !queue.is_empty() {
        let level_size = queue.len();
        let mut current_layer: Vec<String> = Vec::new();

        for _ in 0..level_size {
            let node_id = queue.pop_front().unwrap();
            current_layer.push(node_id.clone());
            order.push(node_id.clone());

            if let Some(neighbors) = adjacency.get(&node_id) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }

        layers.push(current_layer);
    }

    if order.len() != nodes.len() {
        return Err(AllianceError::invalid_argument(
            "Cycle detected in DAG",
        ));
    }

    // 重新计算入度（因为上面的 in_degree 已经被修改了）
    let mut final_in_degree: HashMap<String, usize> = HashMap::new();
    for node in nodes {
        final_in_degree.insert(node.node_id.clone(), node.dependencies.len());
    }

    Ok(TopoSortResult {
        order,
        layers,
        in_degree: final_in_degree,
    })
}

/// 查找可执行的节点（所有依赖都已完成的节点）
pub fn find_ready_nodes(nodes: &[Node]) -> Vec<String> {
    nodes
        .iter()
        .filter(|n| n.status == NodeStatus::Pending)
        .filter(|n| {
            n.dependencies.iter().all(|dep| {
                nodes
                    .iter()
                    .find(|m| m.node_id == *dep)
                    .map(|m| m.status == NodeStatus::Completed)
                    .unwrap_or(false)
            })
        })
        .map(|n| n.node_id.clone())
        .collect()
}

/// 计算 DAG 的关键路径（最长路径）
///
/// 返回关键路径上的节点 ID 列表和总权重（用节点数量估算）。
/// 这是一个简化版本，假设每个节点权重为 1。
pub fn critical_path(nodes: &[Node]) -> AllianceResult<(Vec<String>, usize)> {
    let topo = topological_sort(nodes)?;

    let mut dist: HashMap<String, usize> = HashMap::new();
    let mut prev: HashMap<String, Option<String>> = HashMap::new();

    // 初始化距离
    for node in nodes {
        dist.insert(node.node_id.clone(), 1); // 每个节点权重为 1
        prev.insert(node.node_id.clone(), None);
    }

    // 按拓扑顺序松弛
    for node_id in &topo.order {
        let node = nodes.iter().find(|n| n.node_id == *node_id).unwrap();
        let current_dist = *dist.get(node_id).unwrap();

        for dep in &node.dependencies {
            let new_dist = *dist.get(dep).unwrap() + 1;
            if new_dist > current_dist {
                dist.insert(node_id.clone(), new_dist);
                prev.insert(node_id.clone(), Some(dep.clone()));
            }
        }
    }

    // 找终点（距离最大的节点）
    let mut end_node: Option<String> = None;
    let mut max_dist = 0;
    for (id, &d) in &dist {
        if d > max_dist {
            max_dist = d;
            end_node = Some(id.clone());
        }
    }

    // 回溯关键路径
    let mut path: Vec<String> = Vec::new();
    let mut current = end_node;
    while let Some(node_id) = current {
        path.push(node_id.clone());
        current = prev.get(&node_id).cloned().flatten();
    }
    path.reverse();

    Ok((path, max_dist))
}

/// 检查两个节点之间是否存在路径
pub fn has_path(nodes: &[Node], from: &str, to: &str) -> AllianceResult<bool> {
    use std::collections::VecDeque;

    // 构建邻接表（from -> to，即依赖方向）
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for node in nodes {
        for dep in &node.dependencies {
            // node depends on dep, so path goes dep -> node
            adjacency
                .entry(dep.clone())
                .or_default()
                .push(node.node_id.clone());
        }
    }

    // BFS
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(from.to_string());
    visited.insert(from.to_string());

    while let Some(current) = queue.pop_front() {
        if current == to {
            return Ok(true);
        }
        if let Some(neighbors) = adjacency.get(&current) {
            for neighbor in neighbors {
                if visited.insert(neighbor.clone()) {
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_node(id: &str, deps: Vec<&str>) -> Node {
        Node {
            node_id: id.to_string(),
            task_id: uuid::Uuid::new_v4(),
            expert_id: "test-expert".to_string(),
            name: id.to_string(),
            description: None,
            status: NodeStatus::Pending,
            retry_count: 0,
            dependencies: deps.into_iter().map(|s| s.to_string()).collect(),
            input_refs: vec![],
            output_ref: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            error_message: None,
        }
    }

    #[test]
    fn test_topological_sort_linear() {
        let nodes = vec![
            make_node("A", vec![]),
            make_node("B", vec!["A"]),
            make_node("C", vec!["B"]),
        ];
        let result = topological_sort(&nodes).unwrap();
        assert_eq!(result.order, vec!["A", "B", "C"]);
        assert_eq!(result.layers.len(), 3);
    }

    #[test]
    fn test_topological_sort_parallel() {
        let nodes = vec![
            make_node("A", vec![]),
            make_node("B", vec!["A"]),
            make_node("C", vec!["A"]),
            make_node("D", vec!["B", "C"]),
        ];
        let result = topological_sort(&nodes).unwrap();
        assert_eq!(result.order[0], "A");
        assert_eq!(result.order.last().unwrap(), "D");
        assert_eq!(result.layers.len(), 3);
    }

    #[test]
    fn test_topological_sort_cycle() {
        let nodes = vec![
            make_node("A", vec!["B"]),
            make_node("B", vec!["A"]),
        ];
        assert!(topological_sort(&nodes).is_err());
    }

    #[test]
    fn test_find_ready_nodes() {
        let mut nodes = vec![
            make_node("A", vec![]),
            make_node("B", vec!["A"]),
            make_node("C", vec!["A"]),
            make_node("D", vec!["B", "C"]),
        ];
        // 初始状态：A 是 ready
        let ready = find_ready_nodes(&nodes);
        assert_eq!(ready, vec!["A"]);

        // A 完成后：B 和 C 是 ready
        nodes[0].status = NodeStatus::Completed;
        let ready = find_ready_nodes(&nodes);
        assert_eq!(ready.len(), 2);
        assert!(ready.contains(&"B".to_string()));
        assert!(ready.contains(&"C".to_string()));
    }

    #[test]
    fn test_critical_path() {
        let nodes = vec![
            make_node("A", vec![]),
            make_node("B", vec!["A"]),
            make_node("C", vec!["A"]),
            make_node("D", vec!["B"]),
            make_node("E", vec!["C", "D"]),
        ];
        let (path, length) = critical_path(&nodes).unwrap();
        assert_eq!(length, 4); // A -> B -> D -> E
        assert_eq!(path, vec!["A", "B", "D", "E"]);
    }

    #[test]
    fn test_has_path() {
        let nodes = vec![
            make_node("A", vec![]),
            make_node("B", vec!["A"]),
            make_node("C", vec!["B"]),
        ];
        assert!(has_path(&nodes, "A", "C").unwrap());
        assert!(!has_path(&nodes, "C", "A").unwrap());
    }
}
