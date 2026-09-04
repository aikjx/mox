// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! KG 共享图谱地基：从 seed JSON 加载真实mox 模块化系统架构需求图谱，构建 mox-kg-algo-core
//! 的 [`KnowledgeGraph`]，并以全局单例供 http_adapter 10 个端点共享。
//!
//! # 数据来源
//! seed 文件：`platform/domains/kg/seed/functional-requirements-graph-seed.json`
//! （687 节点 / 776 边，mox 模块化系统架构需求图谱真实数据）。
//!
//! # 加载顺序
//! 1. 环境变量 `MOX_KG_SEED_PATH`（若设置且文件存在）
//! 2. 相对当前工作目录：`platform/domains/kg/seed/functional-requirements-graph-seed.json`
//! 3. 相对 crate 目录：`../seed/functional-requirements-graph-seed.json`
//! 4. 均失败时回退到内置最小 demo 图（保证服务不崩溃，但 `meta.fallback=true` 标记）

use mox_kg_algo_core::{KnowledgeEdge, KnowledgeGraph, KnowledgeNode};
use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, OnceLock};

// ============================================================================
// Seed JSON 反序列化结构
// ============================================================================

#[derive(Debug, Deserialize)]
struct SeedNode {
    id: String,
    label: String,
    node_type: String,
    #[serde(default)]
    properties: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct SeedEdge {
    source: String,
    target: String,
    #[serde(default = "default_weight")]
    weight: f64,
    #[serde(default = "default_rel")]
    relation_type: String,
}

fn default_weight() -> f64 {
    1.0
}
fn default_rel() -> String {
    "related".to_string()
}

#[derive(Debug, Deserialize)]
struct SeedData {
    nodes: Vec<SeedNode>,
    edges: Vec<SeedEdge>,
}

// ============================================================================
// 邻域 BFS 结果
// ============================================================================

/// 邻域子图（BFS 双向扩展结果）
#[derive(Debug, Clone)]
pub struct NeighborhoodSubgraph {
    pub nodes: Vec<String>,
    pub edges: Vec<(String, String, f64, String)>,
}

// ============================================================================
// KgGraph：共享图谱包装
// ============================================================================

#[derive(Debug)]
pub struct KgGraph {
    /// algo-core 有向加权图（所有算法的计算载体）
    pub graph: KnowledgeGraph,
    /// 节点 id → (label, node_type) 元数据快查表（避免反复遍历 graph）
    pub node_meta: HashMap<String, (String, String)>,
    /// 边 id → (source, target, weight, relation_type) 快查表
    pub edge_meta: HashMap<String, (String, String, f64, String)>,
    /// 是否为回退 demo 图（true 表示 seed 加载失败）
    pub fallback: bool,
    /// seed 来源路径（用于响应中的 note 字段）
    pub source_path: String,
}

impl KgGraph {
    /// 从 seed JSON 文件加载
    pub fn load_from_path(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("读取 seed 文件失败 {}: {}", path, e))?;
        let data: SeedData = serde_json::from_str(&content)
            .map_err(|e| format!("解析 seed JSON 失败 {}: {}", path, e))?;
        Self::from_seed_data(&data, path, false)
    }

    /// 从 SeedData 构建 KgGraph
    fn from_seed_data(data: &SeedData, source_path: &str, fallback: bool) -> Result<Self, String> {
        let mut graph = KnowledgeGraph::new();
        let mut node_meta = HashMap::new();
        let mut edge_meta = HashMap::new();

        // 1. 添加所有节点
        for sn in &data.nodes {
            node_meta.insert(sn.id.clone(), (sn.label.clone(), sn.node_type.clone()));
            graph.add_node(KnowledgeNode {
                id: sn.id.clone(),
                label: sn.label.clone(),
                node_type: sn.node_type.clone(),
                properties: sn.properties.clone(),
                embedding: None,
                activation: 0.0,
                metadata: HashMap::new(),
            });
        }

        // 2. 添加所有边（跳过引用不存在节点的边）
        let mut edge_idx = 0usize;
        for se in &data.edges {
            if !node_meta.contains_key(&se.source) || !node_meta.contains_key(&se.target) {
                continue;
            }
            let edge_id = format!("e{}", edge_idx);
            edge_meta.insert(
                edge_id.clone(),
                (se.source.clone(), se.target.clone(), se.weight, se.relation_type.clone()),
            );
            let _ = graph.add_edge(KnowledgeEdge {
                source: se.source.clone(),
                target: se.target.clone(),
                weight: se.weight,
                relation_type: se.relation_type.clone(),
                properties: serde_json::json!({}),
            });
            edge_idx += 1;
        }

        Ok(Self {
            graph,
            node_meta,
            edge_meta,
            fallback,
            source_path: source_path.to_string(),
        })
    }

    /// 构建回退用最小 demo 图（6 节点交付链）
    fn fallback_demo() -> Self {
        let data = SeedData {
            nodes: vec![
                SeedNode { id: "P0-REQ-001".into(), label: "需求·考勤系统".into(), node_type: "Requirement".into(), properties: serde_json::json!({}) },
                SeedNode { id: "P2-ARCH-001".into(), label: "架构·微服务".into(), node_type: "Design".into(), properties: serde_json::json!({}) },
                SeedNode { id: "P3-UI-001".into(), label: "UI·考勤页".into(), node_type: "UIDesign".into(), properties: serde_json::json!({}) },
                SeedNode { id: "P4-CODE-001".into(), label: "代码·考勤svc".into(), node_type: "Code".into(), properties: serde_json::json!({}) },
                SeedNode { id: "P8-TEST-001".into(), label: "测试·SIT报告".into(), node_type: "TestReport".into(), properties: serde_json::json!({}) },
                SeedNode { id: "P10-RUN-001".into(), label: "运行·生产v1.2".into(), node_type: "Deployment".into(), properties: serde_json::json!({}) },
            ],
            edges: vec![
                SeedEdge { source: "P0-REQ-001".into(), target: "P2-ARCH-001".into(), weight: 1.0, relation_type: "derive".into() },
                SeedEdge { source: "P2-ARCH-001".into(), target: "P3-UI-001".into(), weight: 1.0, relation_type: "derive".into() },
                SeedEdge { source: "P3-UI-001".into(), target: "P4-CODE-001".into(), weight: 1.0, relation_type: "derive".into() },
                SeedEdge { source: "P4-CODE-001".into(), target: "P8-TEST-001".into(), weight: 1.0, relation_type: "verify".into() },
                SeedEdge { source: "P8-TEST-001".into(), target: "P10-RUN-001".into(), weight: 1.0, relation_type: "promote".into() },
                SeedEdge { source: "P8-TEST-001".into(), target: "P4-CODE-001".into(), weight: 0.8, relation_type: "bug_fix".into() },
            ],
        };
        Self::from_seed_data(&data, "fallback://demo-6node", true).unwrap()
    }

    /// 自动探测并加载 seed（按优先级尝试多个路径）
    pub fn auto_load() -> Self {
        // 1. 环境变量
        if let Ok(env_path) = std::env::var("MOX_KG_SEED_PATH") {
            if std::path::Path::new(&env_path).exists() {
                if let Ok(g) = Self::load_from_path(&env_path) {
                    return g;
                }
            }
        }

        // 2. 相对工作目录（cargo run / 二进制从 workspace root 启动）
        let candidates = [
            "platform/domains/kg/seed/functional-requirements-graph-seed.json",
            "../seed/functional-requirements-graph-seed.json",
            "../../seed/functional-requirements-graph-seed.json",
            "domains/kg/seed/functional-requirements-graph-seed.json",
        ];
        for c in &candidates {
            if std::path::Path::new(c).exists() {
                if let Ok(g) = Self::load_from_path(c) {
                    return g;
                }
            }
        }

        // 3. 回退 demo
        tracing::warn!("KG seed 图谱加载失败，使用回退 demo 图（6 节点）。设置 MOX_KG_SEED_PATH 可指定真实 seed 路径。");
        Self::fallback_demo()
    }

    // ========================================================================
    // 邻域 BFS（双向：入边 + 出边）
    // ========================================================================

    /// 以 center 为中心做 depth 层双向 BFS，返回子图节点和边。
    /// 双向含义：同时沿出边（source→target）和入边（target←source）扩展。
    pub fn neighborhood_bfs(
        &self,
        center: &str,
        depth: usize,
        limit: usize,
    ) -> NeighborhoodSubgraph {
        let mut visited: HashSet<String> = HashSet::new();
        let mut nodes: Vec<String> = Vec::new();
        let mut edges: Vec<(String, String, f64, String)> = Vec::new();
        let mut edge_set: HashSet<(String, String)> = HashSet::new();

        if !self.node_meta.contains_key(center) {
            return NeighborhoodSubgraph { nodes, edges };
        }

        visited.insert(center.to_string());
        nodes.push(center.to_string());

        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        queue.push_back((center.to_string(), 0));

        while let Some((node, d)) = queue.pop_front() {
            if d >= depth || nodes.len() >= limit {
                continue;
            }

            // 获取该节点的所有邻居（出边目标 + 入边源）
            let neighbors = self.graph.neighbors(&node).unwrap_or_default();
            for (nb_id, weight, _ntype) in neighbors {
                if nodes.len() >= limit {
                    break;
                }
                // 记录边（去重，无向语义）
                let edge_key = if node < nb_id {
                    (node.clone(), nb_id.clone())
                } else {
                    (nb_id.clone(), node.clone())
                };
                if edge_set.insert(edge_key) {
                    // 查找 relation_type
                    let rel = self
                        .edge_meta
                        .values()
                        .find(|(s, t, _, _)| {
                            (*s == node && *t == nb_id) || (*s == nb_id && *t == node)
                        })
                        .map(|(_, _, _, r)| r.clone())
                        .unwrap_or_else(|| "related".to_string());
                    edges.push((node.clone(), nb_id.clone(), weight, rel));
                }
                if visited.insert(nb_id.clone()) {
                    nodes.push(nb_id.clone());
                    queue.push_back((nb_id.clone(), d + 1));
                }
            }
        }

        NeighborhoodSubgraph { nodes, edges }
    }

    /// 获取节点标签（节点不存在时返回 id 本身）
    pub fn node_label(&self, id: &str) -> String {
        self.node_meta
            .get(id)
            .map(|(l, _)| l.clone())
            .unwrap_or_else(|| id.to_string())
    }

    /// 获取节点类型（节点不存在时返回 "unknown"）
    pub fn node_type(&self, id: &str) -> String {
        self.node_meta
            .get(id)
            .map(|(_, t)| t.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

// ============================================================================
// 全局单例
// ============================================================================

static KG_GRAPH: OnceLock<Arc<KgGraph>> = OnceLock::new();

/// 获取全局共享图谱实例（首次调用时自动加载 seed）
pub fn global() -> Arc<KgGraph> {
    KG_GRAPH
        .get_or_init(|| Arc::new(KgGraph::auto_load()))
        .clone()
}

/// 强制重新加载（测试用，或运行时热更新 seed）
pub fn reload() -> Arc<KgGraph> {
    let g = Arc::new(KgGraph::auto_load());
    // OnceLock 无法覆盖，这里通过内部可变性模式暂不支持运行时替换；
    // 测试场景使用独立 KgGraph 实例即可。
    let _ = KG_GRAPH.set(g.clone());
    g
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_demo_graph() {
        let g = KgGraph::fallback_demo();
        assert_eq!(g.graph.node_count(), 6);
        assert_eq!(g.graph.edge_count(), 6);
        assert!(g.fallback);
    }

    #[test]
    fn test_neighborhood_bfs_center_exists() {
        let g = KgGraph::fallback_demo();
        let sub = g.neighborhood_bfs("P0-REQ-001", 2, 50);
        assert!(sub.nodes.contains(&"P0-REQ-001".to_string()));
        assert!(sub.nodes.len() >= 2); // center + at least one neighbor
        assert!(!sub.edges.is_empty());
    }

    #[test]
    fn test_neighborhood_bfs_center_not_exists() {
        let g = KgGraph::fallback_demo();
        let sub = g.neighborhood_bfs("NONEXISTENT", 2, 50);
        assert!(sub.nodes.is_empty());
        assert!(sub.edges.is_empty());
    }

    #[test]
    fn test_node_label_and_type() {
        let g = KgGraph::fallback_demo();
        assert_eq!(g.node_label("P0-REQ-001"), "需求·考勤系统");
        assert_eq!(g.node_type("P0-REQ-001"), "Requirement");
        assert_eq!(g.node_label("UNKNOWN"), "UNKNOWN");
        assert_eq!(g.node_type("UNKNOWN"), "unknown");
    }

    #[test]
    fn test_graph_stats_on_demo() {
        let g = KgGraph::fallback_demo();
        let stats = g.graph.stats();
        assert_eq!(stats.node_count, 6);
        assert_eq!(stats.edge_count, 6);
        assert!(stats.density > 0.0);
    }

    #[test]
    fn test_pagerank_on_demo() {
        let g = KgGraph::fallback_demo();
        let pr = g.graph.pagerank(50);
        assert_eq!(pr.len(), 6);
        let sum: f64 = pr.values().sum();
        assert!((sum - 1.0).abs() < 0.01, "PR sum = {}, expected ~1.0", sum);
    }

    #[test]
    fn test_centrality_on_demo() {
        let g = KgGraph::fallback_demo();
        let metrics = g.graph.centrality_metrics();
        assert_eq!(metrics.degree_centrality.len(), 6);
        assert_eq!(metrics.betweenness_centrality.len(), 6);
        assert_eq!(metrics.pagerank.len(), 6);
        assert_eq!(metrics.closeness_centrality.len(), 6);
    }

    #[test]
    fn test_communities_on_demo() {
        let g = KgGraph::fallback_demo();
        let comms = g.graph.detect_communities(20);
        assert!(!comms.is_empty());
        let total_members: usize = comms.iter().map(|c| c.nodes.len()).sum();
        assert_eq!(total_members, 6);
    }

    #[test]
    fn test_shortest_path_on_demo() {
        let g = KgGraph::fallback_demo();
        let result = g.graph.dijkstra_shortest_path("P0-REQ-001", "P10-RUN-001").unwrap();
        assert!(result.is_some());
        let path = result.unwrap();
        assert_eq!(path.path[0], "P0-REQ-001");
        assert_eq!(path.path.last().unwrap(), "P10-RUN-001");
        assert!(path.length >= 4);
    }

    #[test]
    fn test_k_shortest_paths_on_demo() {
        let g = KgGraph::fallback_demo();
        let paths = g.graph.k_shortest_paths("P0-REQ-001", "P10-RUN-001", 3).unwrap();
        assert!(!paths.is_empty());
        assert!(paths.len() <= 3);
    }
}
