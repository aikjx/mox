//! MOX 统一基座 · 图遍历引擎层
//!
//! 定义图遍历的统一契约（多跳 / 路径 / 可达性 / 最短路径），
//! 供 kg 域 mox-kg-algo-core 等实现（算法上移到基座）。
//!
//! ## 设计原则
//! - 只定义 trait 契约 + 内存参考实现，不内置具体图数据库后端。
//! - 图遍历算法收敛到统一核心库（mox-unified-algo-core 为算法基座）。

use std::collections::{HashMap, HashSet, VecDeque};

use async_trait::async_trait;
use mox_base_model_core::{Edge, Id};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 图遍历错误
#[derive(Debug, Error)]
pub enum GraphError {
    #[error("节点不存在: {0}")]
    NodeNotFound(String),
    #[error("不支持的边类型: {0}")]
    UnsupportedEdgeType(String),
    #[error("其他错误: {0}")]
    Other(String),
}

/// 图遍历结果
pub type GraphResult<T> = Result<T, GraphError>;

/// 路径（节点序列）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Path {
    /// 途经节点 ID（含起点终点）
    pub nodes: Vec<String>,
    /// 途经边 ID
    pub edges: Vec<String>,
    /// 路径长度（跳数）
    pub hops: usize,
}

impl Path {
    /// 构造路径
    pub fn new(nodes: Vec<String>, edges: Vec<String>) -> Self {
        let hops = nodes.len().saturating_sub(1);
        Self {
            nodes,
            edges,
            hops,
        }
    }
}

/// 图遍历 trait（多跳 / 路径）
///
/// kg 域 mox-kg-algo-core 实现此 trait，算法上移到基座。
#[async_trait]
pub trait GraphTraversal: Send + Sync {
    /// 从起点出发，沿指定边类型遍历 k 跳，返回可达节点集合
    async fn traverse(&self, from: &Id, edge_type: &str, hops: usize) -> GraphResult<Vec<String>>;

    /// 查找两节点间的最短路径（BFS）
    async fn shortest_path(&self, from: &Id, to: &Id) -> GraphResult<Option<Path>>;

    /// 返回某节点的直接邻居（按边类型过滤）
    async fn neighbors(&self, node: &Id, edge_type: &str) -> GraphResult<Vec<String>>;

    /// 可达性判断（是否存在路径）
    async fn reachable(&self, from: &Id, to: &Id, max_hops: usize) -> GraphResult<bool>;
}

/// 内存图存储（参考实现 / 测试用；生产由 kg 域实现）
pub struct InMemoryGraph {
    /// node_id -> [(edge_type, from, to)]
    edges: HashMap<String, Vec<(String, String, String)>>,
}

impl Default for InMemoryGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryGraph {
    /// 新建内存图
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
        }
    }

    /// 添加边（统一用字符串 ID 简化）
    pub fn add_edge(&mut self, edge: &Edge) {
        let from = edge.from.to_string();
        let to = edge.to.to_string();
        self.edges.entry(from.clone()).or_default().push((
            edge.edge_type.clone(),
            from,
            to,
        ));
    }

    /// 添加原始边（字符串三元组，便于测试）
    pub fn add_raw_edge(&mut self, edge_type: &str, from: &str, to: &str) {
        self.edges
            .entry(from.to_string())
            .or_default()
            .push((edge_type.to_string(), from.to_string(), to.to_string()));
    }
}

#[async_trait]
impl GraphTraversal for InMemoryGraph {
    async fn traverse(&self, from: &Id, edge_type: &str, hops: usize) -> GraphResult<Vec<String>> {
        let start = from.to_string();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((start.clone(), 0usize));
        visited.insert(start.clone());

        while let Some((node, depth)) = queue.pop_front() {
            if depth >= hops {
                continue;
            }
            if let Some(adj) = self.edges.get(&node) {
                for (et, _, to) in adj {
                    if et == edge_type && visited.insert(to.clone()) {
                        queue.push_back((to.clone(), depth + 1));
                    }
                }
            }
        }

        // 移除起点自身
        visited.remove(&start);
        Ok(visited.into_iter().collect())
    }

    async fn shortest_path(&self, from: &Id, to: &Id) -> GraphResult<Option<Path>> {
        let start = from.to_string();
        let goal = to.to_string();
        if start == goal {
            return Ok(Some(Path::new(vec![start], vec![])));
        }

        let mut prev: HashMap<String, (String, String)> = HashMap::new(); // node -> (prev, edge)
        let mut queue = VecDeque::new();
        queue.push_back(start.clone());
        let mut visited = HashSet::new();
        visited.insert(start.clone());

        while let Some(node) = queue.pop_front() {
            if let Some(adj) = self.edges.get(&node) {
                for (et, _, next) in adj {
                    if visited.insert(next.clone()) {
                        prev.insert(next.clone(), (node.clone(), et.clone()));
                        if next == &goal {
                            // 回溯路径
                            let mut nodes = vec![goal.clone()];
                            let mut edges = Vec::new();
                            let mut cur = goal.clone();
                            while let Some((p, e)) = prev.get(&cur) {
                                edges.push(e.clone());
                                nodes.push(p.clone());
                                cur = p.clone();
                            }
                            nodes.reverse();
                            edges.reverse();
                            return Ok(Some(Path::new(nodes, edges)));
                        }
                        queue.push_back(next.clone());
                    }
                }
            }
        }
        Ok(None)
    }

    async fn neighbors(&self, node: &Id, edge_type: &str) -> GraphResult<Vec<String>> {
        let key = node.to_string();
        Ok(self
            .edges
            .get(&key)
            .map(|adj| {
                adj.iter()
                    .filter(|(et, _, _)| et == edge_type)
                    .map(|(_, _, to)| to.clone())
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn reachable(&self, from: &Id, to: &Id, max_hops: usize) -> GraphResult<bool> {
        let reached = self.traverse(from, "contains", max_hops).await?;
        Ok(reached.contains(&to.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_base_model_core::EntityKind;
    use uuid::Uuid;

    /// 由 label 派生确定性 Id，使 to_string() 可复现（图 key 与 Id 一致）
    fn id_from(label: &str) -> Id {
        // 确定性：基于 label 哈希映射到固定 uuid（不依赖 v5 feature）
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        label.hash(&mut h);
        let bytes = h.finish().to_be_bytes();
        let mut arr = [0u8; 16];
        arr[0..8].copy_from_slice(&bytes);
        arr[15] = 0x01; // 固定 namespace 标志
        Id {
            domain: "kg".into(),
            kind: EntityKind::Node,
            uuid: Uuid::from_bytes(arr),
        }
    }

    #[tokio::test]
    async fn traverse_multi_hop_works() {
        let mut g = InMemoryGraph::new();
        let a = id_from("a");
        let b = id_from("b");
        let c = id_from("c");
        let d = id_from("d");
        g.add_raw_edge("contains", &a.to_string(), &b.to_string());
        g.add_raw_edge("contains", &b.to_string(), &c.to_string());
        g.add_raw_edge("contains", &a.to_string(), &d.to_string());
        let reached = g.traverse(&a, "contains", 2).await.unwrap();
        assert_eq!(reached.len(), 3); // b, c, d
    }

    #[tokio::test]
    async fn shortest_path_found() {
        let mut g = InMemoryGraph::new();
        let a = id_from("a");
        let b = id_from("b");
        let c = id_from("c");
        g.add_raw_edge("contains", &a.to_string(), &b.to_string());
        g.add_raw_edge("contains", &b.to_string(), &c.to_string());
        g.add_raw_edge("contains", &a.to_string(), &c.to_string()); // 直达边
        let path = g.shortest_path(&a, &c).await.unwrap().unwrap();
        assert_eq!(path.hops, 1); // 直达是最短
        assert_eq!(path.nodes, vec![a.to_string(), c.to_string()]);
    }

    #[tokio::test]
    async fn no_path_returns_none() {
        let mut g = InMemoryGraph::new();
        let a = id_from("a");
        let b = id_from("b");
        let x = id_from("x");
        g.add_raw_edge("contains", &a.to_string(), &b.to_string());
        assert!(g.shortest_path(&a, &x).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn neighbors_by_edge_type() {
        let mut g = InMemoryGraph::new();
        let a = id_from("a");
        let b = id_from("b");
        let c = id_from("c");
        g.add_raw_edge("contains", &a.to_string(), &b.to_string());
        g.add_raw_edge("references", &a.to_string(), &c.to_string());
        let nb = g.neighbors(&a, "contains").await.unwrap();
        assert_eq!(nb, vec![b.to_string()]);
    }
}
