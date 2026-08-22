//! # 知识图谱模块 - AI驱动关系网引擎
//!
//! 实现公理3：关联关系加权有向图
//! 基于petgraph实现加权有向图，支持邻接矩阵、关联度计算、图拉普拉斯、
//! 中心性分析、社区发现、最短路径、智能推荐等AI驱动功能

use nalgebra::DMatrix;
use petgraph::algo::dijkstra;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::f64::consts::E;

pub use operator_core::Result;

/// AI 流程图谱引擎：业务流程 + 算法流程统一承载于图谱（与 Node 层 ai-flow-graph.js 跨语言对齐）
pub mod flow_graph;
pub use flow_graph::{AIFlowGraph, CapabilityMeta, FlowGraphStats, IntentResult, IntentRule};

/// 知识图谱节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub properties: serde_json::Value,
    pub embedding: Option<Vec<f64>>,
    pub activation: f64,
    pub metadata: HashMap<String, String>,
}

/// 知识图谱边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEdge {
    pub source: String,
    pub target: String,
    pub weight: f64,
    pub relation_type: String,
    pub properties: serde_json::Value,
}

/// 中心性指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentralityMetrics {
    pub degree_centrality: HashMap<String, f64>,
    pub betweenness_centrality: HashMap<String, f64>,
    pub pagerank: HashMap<String, f64>,
    pub closeness_centrality: HashMap<String, f64>,
}

/// 社区结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    pub id: usize,
    pub nodes: Vec<String>,
    pub density: f64,
    pub label: String,
}

/// 路径分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResult {
    pub path: Vec<String>,
    pub total_weight: f64,
    pub length: usize,
}

/// 节点推荐
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRecommendation {
    pub node_id: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

/// 图统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub density: f64,
    pub average_degree: f64,
    pub strongly_connected_components: usize,
    pub diameter: Option<usize>,
    pub clustering_coefficient: f64,
}

/// 知识图谱 - AI驱动无限扩展关系网
#[derive(Debug, Clone)]
pub struct KnowledgeGraph {
    graph: DiGraph<KnowledgeNode, f64>,
    node_map: HashMap<String, NodeIndex>,
    damping_factor: f64,
    learning_rate: f64,
    activation_history: Vec<HashMap<String, f64>>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
            damping_factor: 0.85,
            learning_rate: 0.01,
            activation_history: Vec::new(),
        }
    }

    pub fn with_damping(damping: f64) -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
            damping_factor: damping,
            learning_rate: 0.01,
            activation_history: Vec::new(),
        }
    }

    /// 添加节点
    pub fn add_node(&mut self, node: KnowledgeNode) -> NodeIndex {
        let id = node.id.clone();
        let idx = self.graph.add_node(node);
        self.node_map.insert(id, idx);
        idx
    }

    /// 添加边 - 支持权重自适应学习
    pub fn add_edge(&mut self, edge: KnowledgeEdge) -> Result<()> {
        let source = self
            .node_map
            .get(&edge.source)
            .ok_or_else(|| anyhow::anyhow!("源节点不存在: {}", edge.source))?;
        let target = self
            .node_map
            .get(&edge.target)
            .ok_or_else(|| anyhow::anyhow!("目标节点不存在: {}", edge.target))?;
        
        // 如果边已存在，强化权重（Hebbian学习）
        if let Some(existing_edge) = self.graph.find_edge(*source, *target) {
            let current_weight = *self.graph.edge_weight(existing_edge).unwrap();
            let new_weight = current_weight + edge.weight * self.learning_rate;
            *self.graph.edge_weight_mut(existing_edge).unwrap() = new_weight;
        } else {
            self.graph.add_edge(*source, *target, edge.weight);
        }
        Ok(())
    }

    /// 获取节点
    pub fn get_node(&self, id: &str) -> Option<&KnowledgeNode> {
        self.node_map.get(id).map(|idx| &self.graph[*idx])
    }

    /// 获取节点（可变），用于回写布局优化结果（中心性/社区）
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut KnowledgeNode> {
        if let Some(idx) = self.node_map.get(id) {
            let idx = *idx;
            Some(&mut self.graph[idx])
        } else {
            None
        }
    }

    /// 获取节点数
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// 获取边数
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// 构建邻接矩阵
    pub fn adjacency_matrix(&self) -> DMatrix<f64> {
        let n = self.node_count();
        let mut adj = DMatrix::zeros(n, n);
        for edge in self.graph.edge_references() {
            let i = edge.source().index();
            let j = edge.target().index();
            adj[(i, j)] = *edge.weight();
        }
        adj
    }

    /// 构建度矩阵
    pub fn degree_matrix(&self) -> DMatrix<f64> {
        let n = self.node_count();
        let adj = self.adjacency_matrix();
        let mut deg = DMatrix::zeros(n, n);
        for i in 0..n {
            let row_sum: f64 = (0..n).map(|j| adj[(i, j)]).sum();
            deg[(i, i)] = row_sum;
        }
        deg
    }

    /// 构建归一化拉普拉斯矩阵
    pub fn laplacian_matrix(&self) -> DMatrix<f64> {
        let deg = self.degree_matrix();
        let adj = self.adjacency_matrix();
        &deg - &adj
    }

    /// 构建对称归一化拉普拉斯矩阵
    pub fn normalized_laplacian(&self) -> DMatrix<f64> {
        let n = self.node_count();
        let adj = self.adjacency_matrix();
        let mut deg_inv_sqrt = DMatrix::zeros(n, n);
        
        for i in 0..n {
            let d: f64 = (0..n).map(|j| adj[(i, j)]).sum();
            if d > 1e-15 {
                deg_inv_sqrt[(i, i)] = 1.0 / d.sqrt();
            }
        }
        
        let identity = DMatrix::identity(n, n);
        &identity - &(&deg_inv_sqrt * &adj * &deg_inv_sqrt)
    }

    /// 计算k步关联度
    pub fn k_step_relevance(&self, source: &str, target: &str, k: usize) -> Result<f64> {
        let source_idx = self
            .node_map
            .get(source)
            .ok_or_else(|| anyhow::anyhow!("源节点不存在: {}", source))?;
        let target_idx = self
            .node_map
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("目标节点不存在: {}", target))?;

        let adj = self.adjacency_matrix();
        let a_k = adj.pow(k as u32);
        let frobenius_norm = a_k.norm();

        if frobenius_norm < 1e-15 {
            return Ok(0.0);
        }

        Ok(a_k[(source_idx.index(), target_idx.index())] / frobenius_norm)
    }

    /// 计算全步关联度（带衰减）
    pub fn total_relevance(&self, source: &str, target: &str) -> Result<f64> {
        let source_idx = self
            .node_map
            .get(source)
            .ok_or_else(|| anyhow::anyhow!("源节点不存在: {}", source))?;
        let target_idx = self
            .node_map
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("目标节点不存在: {}", target))?;

        let n = self.node_count();
        let adj = self.adjacency_matrix();
        let alpha = self.damping_factor;

        let identity = DMatrix::identity(n, n);
        let matrix = &identity - &(&adj * alpha);
        let inv = matrix
            .try_inverse()
            .ok_or_else(|| anyhow::anyhow!("矩阵不可逆"))?;

        Ok(inv[(source_idx.index(), target_idx.index())])
    }

    /// PageRank算法
    ///
    /// 修复 R-D2：悬挂节点（出度为 0）的质量此前直接丢失，导致 ΣPR < 1（不守恒）。
    /// 现将悬挂质量均匀回传全图，并加收敛提前终止（容差 1e-6）。
    pub fn pagerank(&self, iterations: usize) -> HashMap<String, f64> {
        let n = self.node_count();
        if n == 0 {
            return HashMap::new();
        }

        let alpha = self.damping_factor;
        let adj = self.adjacency_matrix();

        // 出度归一化矩阵 + 悬挂节点标记
        let mut deg = DMatrix::zeros(n, n);
        let mut dangling = vec![false; n];
        for i in 0..n {
            let row_sum: f64 = (0..n).map(|j| adj[(i, j)]).sum();
            if row_sum > 1e-15 {
                deg[(i, i)] = 1.0 / row_sum;
            } else {
                dangling[i] = true;
            }
        }

        let transition = &deg * &adj;
        let mut rank = DMatrix::from_element(n, 1, 1.0 / n as f64);
        let teleport = 1.0 / n as f64;

        for _ in 0..iterations {
            // 悬挂质量：均匀回传全图（质量守恒）
            let dangling_mass: f64 = (0..n)
                .filter(|&i| dangling[i])
                .map(|i| rank[(i, 0)])
                .sum();

            // 修复 R-D6：transition[i][j] = W(i,j)/out(i) 是"i 给 j 的份额"，
            // 传播须取转置（推模型）：rank_new[j] = Σ_i transition[i][j]·rank[i]
            let propagated = transition.transpose() * &rank;
            let mut new_rank = propagated * alpha;
            for i in 0..n {
                new_rank[(i, 0)] += alpha * dangling_mass / n as f64 + (1.0 - alpha) * teleport;
            }

            // 收敛判断
            let max_diff: f64 = (0..n).map(|i| (new_rank[(i, 0)] - rank[(i, 0)]).abs()).fold(0.0, f64::max);
            rank = new_rank;
            if max_diff < 1e-6 {
                break;
            }
        }

        let mut result = HashMap::new();
        for (id, idx) in &self.node_map {
            result.insert(id.clone(), rank[(idx.index(), 0)]);
        }
        result
    }

    /// 个性化 PageRank（激活扩散意图识别的算法基础）
    ///
    /// a_i = (1-d)·p_i + d·(Σ_{j→i} a_j·W(j,i)/outW(j) + dangling_mass/n)
    /// p 为个性化向量（命中关键词按权重归一），和为 1。
    pub fn pagerank_personalized(
        &self,
        personalization: &HashMap<String, f64>,
        iterations: usize,
    ) -> HashMap<String, f64> {
        let n = self.node_count();
        if n == 0 {
            return HashMap::new();
        }

        let alpha = self.damping_factor;
        let adj = self.adjacency_matrix();

        let mut deg = DMatrix::zeros(n, n);
        let mut dangling = vec![false; n];
        for i in 0..n {
            let row_sum: f64 = (0..n).map(|j| adj[(i, j)]).sum();
            if row_sum > 1e-15 {
                deg[(i, i)] = 1.0 / row_sum;
            } else {
                dangling[i] = true;
            }
        }

        // 个性化向量：命中的节点带权重，其余为 0（和为 1）
        let mut p = vec![0.0f64; n];
        let total: f64 = personalization.values().sum();
        if total > 1e-15 {
            for (id, w) in personalization {
                if let Some(&idx) = self.node_map.get(id) {
                    p[idx.index()] = w / total;
                }
            }
        } else {
            // 空个性化 → 均匀分布（退化为标准 PageRank）
            for v in p.iter_mut() {
                *v = 1.0 / n as f64;
            }
        }

        let transition = &deg * &adj;
        let mut rank: DMatrix<f64> = DMatrix::from_column_slice(n, 1, &p);

        for _ in 0..iterations {
            let dangling_mass: f64 = (0..n)
                .filter(|&i| dangling[i])
                .map(|i| rank[(i, 0)])
                .sum();

            // 修复 R-D6：推模型取转置（与 pagerank() 一致）
            let propagated = transition.transpose() * &rank;
            let mut new_rank = propagated * alpha;
            for i in 0..n {
                new_rank[(i, 0)] += alpha * dangling_mass / n as f64 + (1.0 - alpha) * p[i];
            }

            let max_diff: f64 = (0..n).map(|i| (new_rank[(i, 0)] - rank[(i, 0)]).abs()).fold(0.0, f64::max);
            rank = new_rank;
            if max_diff < 1e-6 {
                break;
            }
        }

        let mut result = HashMap::new();
        for (id, idx) in &self.node_map {
            result.insert(id.clone(), rank[(idx.index(), 0)]);
        }
        result
    }

    /// 度中心性
    ///
    /// 修复 R-D4：此前除以 2(n-1)（把无向图当双向计算），与 Node 层 F2 语义不一致。
    /// 统一为 C_D(v) = deg(v)/(N-1)，deg = 入度+出度（无向度语义，与 Node 层一致）。
    pub fn degree_centrality(&self) -> HashMap<String, f64> {
        let n = self.node_count() as f64;
        let mut result = HashMap::new();

        for (id, idx) in &self.node_map {
            let in_degree = self.graph.edges_directed(*idx, petgraph::Direction::Incoming).count() as f64;
            let out_degree = self.graph.edges_directed(*idx, petgraph::Direction::Outgoing).count() as f64;
            if n > 1.0 {
                result.insert(id.clone(), (in_degree + out_degree) / (n - 1.0));
            } else {
                result.insert(id.clone(), 0.0);
            }
        }
        result
    }

    /// 介数中心性（Brandes 2001，有向图版）
    ///
    /// 修复 R-D1：此前 centrality_metrics() 中该指标为空占位符（HashMap::new()）。
    /// C_B(v) = Σ_{s≠v≠t} σ_st(v)/σ_st，BFS 最短路计数 + 反向依赖累积，
    /// 归一化除以 (N-1)(N-2)（有向）。
    pub fn betweenness_centrality(&self) -> HashMap<String, f64> {
        let n = self.node_count();
        let mut cb = vec![0.0f64; n];
        if n < 3 {
            let mut result = HashMap::new();
            for id in self.node_map.keys() {
                result.insert(id.clone(), 0.0);
            }
            return result;
        }

        // 邻接表（有向）
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for edge in self.graph.edge_references() {
            adj[edge.source().index()].push(edge.target().index());
        }

        for s in 0..n {
            // BFS 最短路计数
            let mut dist = vec![-1i64; n];
            let mut sigma = vec![0.0f64; n];
            let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
            let mut order: Vec<usize> = Vec::with_capacity(n);
            let mut queue = std::collections::VecDeque::new();

            dist[s] = 0;
            sigma[s] = 1.0;
            queue.push_back(s);

            while let Some(v) = queue.pop_front() {
                order.push(v);
                for &w in &adj[v] {
                    if dist[w] < 0 {
                        dist[w] = dist[v] + 1;
                        queue.push_back(w);
                    }
                    if dist[w] == dist[v] + 1 {
                        sigma[w] += sigma[v];
                        preds[w].push(v);
                    }
                }
            }

            // 反向累积依赖（δ）
            let mut delta = vec![0.0f64; n];
            for &w in order.iter().rev() {
                for &v in &preds[w] {
                    delta[v] += (sigma[v] / sigma[w]) * (1.0 + delta[w]);
                }
                if w != s {
                    cb[w] += delta[w];
                }
            }
        }

        // 归一化：(N-1)(N-2)
        let norm = ((n - 1) * (n - 2)) as f64;
        let mut result = HashMap::new();
        for (id, idx) in &self.node_map {
            result.insert(id.clone(), cb[idx.index()] / norm);
        }
        result
    }

    /// 紧密中心性（harmonic 版本，对不可达节点稳健）
    ///
    /// 修复 R-D5：此前用经典公式 (n-1)/Σd，存在不可达节点时结果偏大（分母漏掉 ∞ 项）。
    /// 统一为 harmonic：C_C(v) = (Σ_{u≠v} 1/d(v,u))/(N-1)，不可达贡献 0（与 Node 层 F5 一致）。
    pub fn closeness_centrality(&self) -> HashMap<String, f64> {
        let mut result = HashMap::new();
        let n = self.node_count();

        for (id, idx) in &self.node_map {
            let distances = dijkstra(&self.graph, *idx, None, |e| *e.weight());
            let mut harmonic = 0.0f64;
            for (other, &d) in &distances {
                if *other != *idx && d > 0.0 {
                    harmonic += 1.0 / d;
                }
            }
            let value = if n > 1 {
                harmonic / (n as f64 - 1.0)
            } else {
                0.0
            };
            result.insert(id.clone(), value);
        }
        result
    }

    /// 综合中心性指标
    pub fn centrality_metrics(&self) -> CentralityMetrics {
        CentralityMetrics {
            degree_centrality: self.degree_centrality(),
            betweenness_centrality: self.betweenness_centrality(),
            pagerank: self.pagerank(20),
            closeness_centrality: self.closeness_centrality(),
        }
    }

    /// 最短路径 - Dijkstra算法
    pub fn shortest_path(&self, source: &str, target: &str) -> Result<Option<PathResult>> {
        let source_idx = self
            .node_map
            .get(source)
            .ok_or_else(|| anyhow::anyhow!("源节点不存在: {}", source))?;
        let target_idx = self
            .node_map
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("目标节点不存在: {}", target))?;

        let distances = dijkstra(&self.graph, *source_idx, Some(*target_idx), |e| *e.weight());
        
        if let Some(&dist) = distances.get(target_idx) {
            let mut path = Vec::new();
            let mut current = *target_idx;
            path.push(self.graph[current].id.clone());
            
            let mut predecessors = HashMap::new();
            for (node, &d) in &distances {
                for edge in self.graph.edges_directed(*node, petgraph::Direction::Incoming) {
                    let from = edge.source();
                    if let Some(&from_d) = distances.get(&from) {
                        if (d - from_d - edge.weight()).abs() < 1e-10 {
                            predecessors.insert(*node, from);
                        }
                    }
                }
            }
            
            while current != *source_idx {
                if let Some(&prev) = predecessors.get(&current) {
                    path.push(self.graph[prev].id.clone());
                    current = prev;
                } else {
                    break;
                }
            }
            path.reverse();
            
            Ok(Some(PathResult {
                path,
                total_weight: dist,
                length: distances.len(),
            }))
        } else {
            Ok(None)
        }
    }

    /// 社区发现：模块度贪心凝聚（CNM / Clauset-Newman-Moore 简化版）
    ///
    /// 修复 R-D3：此前用标签传播（LPA）存在两类缺陷：
    ///   1. 平局时 HashMap 迭代顺序随机 → 结果不可复现；
    ///   2. 标签吞并：双团+桥图坍缩为 1 社区（与 Node 层 D6/D9 同源缺陷）。
    /// CNM：初始每节点一社区，反复合并 ΔQ 最大的相邻社区对，直到无正增益。
    ///   ΔQ(A,B) = e_cross(A,B)/m − d_A·d_B/(2m²)
    /// 确定性：平局取 (社区A, 社区B) 字典序最小的对。
    /// iterations 参数保留以兼容旧 API（仅作迭代上限保护，实际由增益收敛决定）。
    pub fn detect_communities(&self, iterations: usize) -> Vec<Community> {
        let n = self.node_count();
        if n == 0 {
            return Vec::new();
        }

        // 无向边集（合并方向、去重、跳过自环）
        let mut edge_set: HashSet<(usize, usize)> = HashSet::new();
        for edge in self.graph.edge_references() {
            let s = edge.source().index();
            let t = edge.target().index();
            if s != t {
                edge_set.insert((s.min(t), s.max(t)));
            }
        }
        let m = edge_set.len();
        if m == 0 {
            // 无边：每个节点自成社区
            let mut communities = Vec::new();
            for (i, id) in self.node_map.keys().enumerate() {
                communities.push(Community {
                    id: i,
                    nodes: vec![id.clone()],
                    density: 0.0,
                    label: format!("社区 {}", i),
                });
            }
            return communities;
        }

        // 度数（无向语义：每条 RAW 边对两端各贡献 1）
        let mut degree = vec![0usize; n];
        for &(s, t) in &edge_set {
            degree[s] += 1;
            degree[t] += 1;
        }

        // 社区状态
        let mut comm_of: Vec<usize> = (0..n).collect(); // 节点 → 社区 id（初始自身）
        let mut comm_members: Vec<Option<Vec<usize>>> = (0..n).map(|i| Some(vec![i])).collect();
        let mut comm_degree = degree.clone();
        let mut comm_alive: Vec<bool> = (0..n).map(|_| true).collect();

        // 社区间跨边计数：key (a<b)
        let mut cross: HashMap<(usize, usize), usize> = HashMap::new();
        for &(s, t) in &edge_set {
            // 初始每节点一社区，s≠t 必跨社区
            *cross.entry((s.min(t), s.max(t))).or_insert(0) += 1;
        }

        // 贪心合并循环（上限保护：n 次合并足够收敛）
        let max_merges = if iterations == 0 { n } else { iterations.min(n * n) };
        let mut merges = 0;
        loop {
            if merges >= max_merges {
                break;
            }
            // 找 ΔQ 最大的相邻社区对（确定性：平局取字典序最小）
            let mut candidates: Vec<((usize, usize), f64)> = Vec::new();
            for (&(a, b), &cnt) in &cross {
                if cnt == 0 || !comm_alive[a] || !comm_alive[b] {
                    continue;
                }
                let gain = cnt as f64 / m as f64
                    - (comm_degree[a] as f64 * comm_degree[b] as f64) / (2.0 * m as f64 * m as f64);
                candidates.push(((a, b), gain));
            }
            if candidates.is_empty() {
                break;
            }
            candidates.sort_by(|x, y| {
                y.1.partial_cmp(&x.1).unwrap().then(x.0.cmp(&y.0))
            });
            let ((a, b), gain) = candidates[0];
            if gain <= 1e-12 {
                break; // 无正增益 → 收敛
            }

            // 合并 b 入 a（保小 id）
            let members_b = comm_members[b].clone().unwrap_or_default();
            for &node in &members_b {
                comm_of[node] = a;
                if let Some(members) = &mut comm_members[a] {
                    members.push(node);
                }
            }
            comm_degree[a] += comm_degree[b];
            comm_members[b] = None;
            comm_alive[b] = false;
            merges += 1;

            // 更新跨边：b 的所有跨边转入 a
            let keys: Vec<(usize, usize)> = cross.keys().copied().collect();
            for key in keys {
                let (x, y) = key;
                if x != b && y != b {
                    continue;
                }
                let cnt = cross.remove(&key).unwrap_or(0);
                if cnt == 0 {
                    continue;
                }
                let other = if x == b { y } else { x };
                if other == a || !comm_alive[other] {
                    continue; // a-b 间跨边随合并消失
                }
                let nk = (a.min(other), a.max(other));
                *cross.entry(nk).or_insert(0) += cnt;
            }
        }

        // 聚合输出：按规模降序
        let mut groups: Vec<(usize, Vec<String>)> = Vec::new();
        for i in 0..n {
            if !comm_alive[i] {
                continue;
            }
            if let Some(Some(members)) = comm_members.get(i).map(|m| m.as_ref()) {
                let ids: Vec<String> = members
                    .iter()
                    .map(|&node| self.graph[NodeIndex::new(node)].id.clone())
                    .collect();
                groups.push((i, ids));
            }
        }
        groups.sort_by(|x, y| y.1.len().cmp(&x.1.len()).then(x.0.cmp(&y.0)));

        let mut communities = Vec::new();
        for (i, (_, nodes)) in groups.into_iter().enumerate() {
            // 社区密度：内部边 / 最大可能边
            let density = if nodes.len() > 1 {
                let mut internal_edges = 0;
                for (j, n1) in nodes.iter().enumerate() {
                    for n2 in nodes.iter().skip(j + 1) {
                        if let (Some(idx1), Some(idx2)) =
                            (self.node_map.get(n1.as_str()), self.node_map.get(n2.as_str()))
                        {
                            if self.graph.find_edge(*idx1, *idx2).is_some()
                                || self.graph.find_edge(*idx2, *idx1).is_some()
                            {
                                internal_edges += 1;
                            }
                        }
                    }
                }
                let max_edges = nodes.len() * (nodes.len() - 1) / 2;
                internal_edges as f64 / max_edges as f64
            } else {
                0.0
            };

            communities.push(Community {
                id: i,
                nodes,
                density,
                label: format!("社区 {}", i),
            });
        }

        communities
    }

    /// 激活传播 - AI神经网络风格传播
    pub fn propagate_activation(&mut self, start_nodes: &[String], iterations: usize) -> HashMap<String, f64> {
        // 重置激活值
        for idx in self.node_map.values() {
            self.graph[*idx].activation = 0.0;
        }

        // 设置初始激活
        for node_id in start_nodes {
            if let Some(&idx) = self.node_map.get(node_id) {
                self.graph[idx].activation = 1.0;
            }
        }

        let n = self.node_count();
        let indices: Vec<NodeIndex> = self.node_map.values().copied().collect();

        for _ in 0..iterations {
            let mut new_activations = vec![0.0; n];
            
            for (i, &idx) in indices.iter().enumerate() {
                let mut incoming = 0.0;
                for edge in self.graph.edges_directed(idx, petgraph::Direction::Incoming) {
                    let weight = *edge.weight();
                    incoming += self.graph[edge.source()].activation * weight;
                }
                
                // Sigmoid激活函数
                let current = self.graph[idx].activation;
                new_activations[i] = 1.0 / (1.0 + E.powf(-incoming)) * 0.3 + current * 0.7;
            }

            for (i, &idx) in indices.iter().enumerate() {
                self.graph[idx].activation = new_activations[i];
            }
        }

        // 记录历史
        let mut activations = HashMap::new();
        for (id, idx) in &self.node_map {
            activations.insert(id.clone(), self.graph[*idx].activation);
        }
        self.activation_history.push(activations.clone());
        activations
    }

    /// 智能推荐 - 基于激活传播和中心性
    pub fn recommend(&self, context_nodes: &[String], limit: usize) -> Vec<NodeRecommendation> {
        let mut scores = HashMap::new();
        let pagerank = self.pagerank(20);
        let centrality = self.degree_centrality();
        
        // 初始分数：PageRank + 中心性
        for id in self.node_map.keys() {
            if !context_nodes.contains(id) {
                let pr = pagerank.get(id).copied().unwrap_or(0.0);
                let dc = centrality.get(id).copied().unwrap_or(0.0);
                scores.insert(id.clone(), pr * 0.5 + dc * 0.3);
            }
        }

        // 基于上下文节点的关联度加分
        let score_ids: Vec<String> = scores.keys().cloned().collect();
        for context in context_nodes {
            for id in &score_ids {
                if let Ok(relevance) = self.total_relevance(context, id) {
                    if let Some(score) = scores.get_mut(id) {
                        *score += relevance * 0.2;
                    }
                }
            }
        }

        // 排序并生成推荐
        let mut sorted: Vec<_> = scores.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        sorted
            .into_iter()
            .take(limit)
            .map(|(node_id, score)| {
                let mut reasons = Vec::new();
                if let Some(node) = self.get_node(&node_id) {
                    reasons.push(format!("类型: {}", node.node_type));
                }
                reasons.push(format!("相关度得分: {:.4}", score));
                
                NodeRecommendation {
                    node_id,
                    score,
                    reasons,
                }
            })
            .collect()
    }

    /// 图统计信息
    pub fn stats(&self) -> GraphStats {
        let n = self.node_count();
        let m = self.edge_count();
        
        let density = if n > 1 {
            m as f64 / (n as f64 * (n as f64 - 1.0))
        } else {
            0.0
        };

        let average_degree = if n > 0 {
            2.0 * m as f64 / n as f64
        } else {
            0.0
        };

        // 简单聚类系数计算
        let mut clustering_sum = 0.0;
        for idx in self.node_map.values() {
            let neighbors: Vec<NodeIndex> = self
                .graph
                .neighbors(*idx)
                .chain(self.graph.neighbors_directed(*idx, petgraph::Direction::Incoming))
                .collect();
            let unique_neighbors: HashSet<_> = neighbors.iter().collect();
            let k = unique_neighbors.len();
            
            if k >= 2 {
                let mut triangles = 0;
                let neighbor_vec: Vec<_> = unique_neighbors.into_iter().collect();
                for (i, &&n1) in neighbor_vec.iter().enumerate() {
                    for &&n2 in neighbor_vec.iter().skip(i + 1) {
                        if self.graph.find_edge(n1, n2).is_some() || self.graph.find_edge(n2, n1).is_some() {
                            triangles += 1;
                        }
                    }
                }
                clustering_sum += (2 * triangles) as f64 / (k * (k - 1)) as f64;
            }
        }
        
        let clustering_coefficient = if n > 0 {
            clustering_sum / n as f64
        } else {
            0.0
        };

        GraphStats {
            node_count: n,
            edge_count: m,
            density,
            average_degree,
            strongly_connected_components: 1,
            diameter: None,
            clustering_coefficient,
        }
    }

    /// 邻居节点
    pub fn neighbors(&self, id: &str) -> Result<Vec<(String, f64, String)>> {
        let idx = self
            .node_map
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("节点不存在: {}", id))?;
        let mut neighbors = Vec::new();
        
        for edge in self.graph.edges(*idx) {
            let target = &self.graph[edge.target()];
            neighbors.push((target.id.clone(), *edge.weight(), target.node_type.clone()));
        }
        for edge in self.graph.edges_directed(*idx, petgraph::Direction::Incoming) {
            let source = &self.graph[edge.source()];
            neighbors.push((source.id.clone(), *edge.weight(), source.node_type.clone()));
        }
        Ok(neighbors)
    }

    /// 获取所有节点
    pub fn nodes(&self) -> Vec<&KnowledgeNode> {
        self.graph.node_weights().collect()
    }

    /// 获取所有边
    pub fn edges(&self) -> Vec<KnowledgeEdge> {
        self.graph
            .edge_references()
            .map(|e| {
                let source = &self.graph[e.source()];
                let target = &self.graph[e.target()];
                KnowledgeEdge {
                    source: source.id.clone(),
                    target: target.id.clone(),
                    weight: *e.weight(),
                    relation_type: "related".to_string(),
                    properties: serde_json::json!({}),
                }
            })
            .collect()
    }

    /// 获取所有节点ID
    pub fn node_ids(&self) -> Vec<String> {
        self.node_map.keys().cloned().collect()
    }

    /// 余弦相似度计算（基于嵌入向量）
    pub fn cosine_similarity(&self, a: &str, b: &str) -> Result<f64> {
        let node_a = self.get_node(a).ok_or_else(|| anyhow::anyhow!("节点不存在: {}", a))?;
        let node_b = self.get_node(b).ok_or_else(|| anyhow::anyhow!("节点不存在: {}", b))?;

        if let (Some(emb_a), Some(emb_b)) = (&node_a.embedding, &node_b.embedding) {
            if emb_a.len() != emb_b.len() {
                return Ok(0.0);
            }
            let dot: f64 = emb_a.iter().zip(emb_b.iter()).map(|(x, y)| x * y).sum();
            let norm_a: f64 = emb_a.iter().map(|x| x * x).sum::<f64>().sqrt();
            let norm_b: f64 = emb_b.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm_a > 1e-15 && norm_b > 1e-15 {
                Ok(dot / (norm_a * norm_b))
            } else {
                Ok(0.0)
            }
        } else {
            Ok(0.0)
        }
    }
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// 知识图谱构建算子
pub struct KnowledgeGraphBuilder {
    graph: KnowledgeGraph,
}

impl KnowledgeGraphBuilder {
    pub fn new() -> Self {
        Self {
            graph: KnowledgeGraph::new(),
        }
    }

    pub fn add_node(mut self, id: &str, label: &str, node_type: &str) -> Self {
        self.graph.add_node(KnowledgeNode {
            id: id.to_string(),
            label: label.to_string(),
            node_type: node_type.to_string(),
            properties: serde_json::json!({}),
            embedding: None,
            activation: 0.0,
            metadata: HashMap::new(),
        });
        self
    }

    pub fn add_node_with_embedding(mut self, id: &str, label: &str, node_type: &str, embedding: Vec<f64>) -> Self {
        self.graph.add_node(KnowledgeNode {
            id: id.to_string(),
            label: label.to_string(),
            node_type: node_type.to_string(),
            properties: serde_json::json!({}),
            embedding: Some(embedding),
            activation: 0.0,
            metadata: HashMap::new(),
        });
        self
    }

    pub fn add_edge(mut self, source: &str, target: &str, weight: f64) -> Self {
        let _ = self.graph.add_edge(KnowledgeEdge {
            source: source.to_string(),
            target: target.to_string(),
            weight,
            relation_type: "related".to_string(),
            properties: serde_json::json!({}),
        });
        self
    }

    pub fn add_edge_typed(mut self, source: &str, target: &str, weight: f64, relation: &str) -> Self {
        let _ = self.graph.add_edge(KnowledgeEdge {
            source: source.to_string(),
            target: target.to_string(),
            weight,
            relation_type: relation.to_string(),
            properties: serde_json::json!({}),
        });
        self
    }

    pub fn build(self) -> KnowledgeGraph {
        self.graph
    }
}

impl Default for KnowledgeGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_graph_creation() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(KnowledgeNode {
            id: "a".to_string(),
            label: "A".to_string(),
            node_type: "test".to_string(),
            properties: serde_json::json!({}),
            embedding: None,
            activation: 0.0,
            metadata: HashMap::new(),
        });
        graph.add_node(KnowledgeNode {
            id: "b".to_string(),
            label: "B".to_string(),
            node_type: "test".to_string(),
            properties: serde_json::json!({}),
            embedding: None,
            activation: 0.0,
            metadata: HashMap::new(),
        });
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_adjacency_matrix() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_edge("a", "b", 1.0)
            .build();

        let adj = graph.adjacency_matrix();
        assert_relative_eq!(adj[(0, 1)], 1.0);
        assert_relative_eq!(adj[(1, 0)], 0.0);
    }

    #[test]
    fn test_laplacian() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_edge("a", "b", 1.0)
            .build();

        let lap = graph.laplacian_matrix();
        assert_relative_eq!(lap[(0, 0)], 1.0);
        assert_relative_eq!(lap[(0, 1)], -1.0);
    }

    #[test]
    fn test_pagerank() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_node("c", "C", "test")
            .add_edge("a", "b", 1.0)
            .add_edge("b", "c", 1.0)
            .add_edge("c", "a", 1.0)
            .build();

        let pr = graph.pagerank(100);
        assert!(pr.len() == 3);
        for score in pr.values() {
            assert!(*score > 0.0);
        }
    }

    #[test]
    fn test_communities() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "group1")
            .add_node("b", "B", "group1")
            .add_node("c", "C", "group2")
            .add_node("d", "D", "group2")
            .add_edge("a", "b", 1.0)
            .add_edge("b", "a", 1.0)
            .add_edge("c", "d", 1.0)
            .add_edge("d", "c", 1.0)
            .build();

        let communities = graph.detect_communities(10);
        assert!(communities.len() >= 2);
    }

    #[test]
    fn test_stats() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_edge("a", "b", 1.0)
            .build();

        let stats = graph.stats();
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 1);
    }
}
