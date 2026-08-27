// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 根因分析器
//!
//! 核心能力：
//! - 基于依赖图的故障传播路径分析
//! - 多指标相关性分析
//! - 根因节点排序（PageRank 风格）
//! - 故障传播时间线重建
//! - 根因置信度计算

use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

/// 根因分析结果
#[derive(Debug, Clone, Serialize)]
pub struct RootCauseResult {
    pub id: String,
    pub anomaly_metric: String,
    pub timestamp: String,
    pub root_causes: Vec<RankedRootCause>,
    pub propagation_paths: Vec<FaultPropagationPath>,
    pub confidence: f64,
    pub analysis_duration_ms: u64,
    pub recommendations: Vec<String>,
}

/// 排序后的根因
#[derive(Debug, Clone, Serialize)]
pub struct RankedRootCause {
    pub node_id: String,
    pub node_name: String,
    pub score: f64,
    pub rank: usize,
    pub evidence: Vec<String>,
    pub affected_downstream: Vec<String>,
}

/// 故障传播路径
#[derive(Debug, Clone, Serialize)]
pub struct FaultPropagationPath {
    pub path: Vec<String>,
    pub total_delay_ms: u64,
    pub hop_count: usize,
    pub confidence: f64,
}

/// 依赖图节点
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct DependencyNode {
    id: String,
    name: String,
    node_type: String,
    anomaly_score: f64,
    metadata: HashMap<String, String>,
}

/// 根因分析器
pub struct RootCauseAnalyzer {
    nodes: HashMap<String, DependencyNode>,
    edges: Vec<(String, String, f64)>, // (source, target, weight)
    adjacency: HashMap<String, Vec<(String, f64)>>,
    reverse_adjacency: HashMap<String, Vec<(String, f64)>>,
    total_analyses: std::sync::atomic::AtomicU64,
}

impl RootCauseAnalyzer {
    /// 创建根因分析器
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            adjacency: HashMap::new(),
            reverse_adjacency: HashMap::new(),
            total_analyses: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 添加节点
    pub fn add_node(&mut self, id: &str, name: &str, node_type: &str) {
        self.nodes.insert(id.to_string(), DependencyNode {
            id: id.to_string(),
            name: name.to_string(),
            node_type: node_type.to_string(),
            anomaly_score: 0.0,
            metadata: HashMap::new(),
        });
        self.adjacency.entry(id.to_string()).or_default();
        self.reverse_adjacency.entry(id.to_string()).or_default();
    }

    /// 添加依赖边
    pub fn add_edge(&mut self, source: &str, target: &str, weight: f64) {
        self.edges.push((source.to_string(), target.to_string(), weight));
        self.adjacency.entry(source.to_string()).or_default().push((target.to_string(), weight));
        self.reverse_adjacency.entry(target.to_string()).or_default().push((source.to_string(), weight));
    }

    /// 设置节点异常分数
    pub fn set_anomaly_score(&mut self, node_id: &str, score: f64) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.anomaly_score = score;
        }
    }

    /// 分析根因
    pub fn analyze(&self, anomaly_node: &str) -> RootCauseResult {
        let start = std::time::Instant::now();
        self.total_analyses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // 1. 从异常节点向上游遍历，找到所有可能的根因
        let upstream_nodes = self.get_all_upstream(anomaly_node);

        // 2. 计算每个节点的根因分数（PageRank 风格）
        let mut scores: HashMap<String, f64> = HashMap::new();
        for node_id in &upstream_nodes {
            let score = self.calculate_root_cause_score(node_id, anomaly_node);
            scores.insert(node_id.clone(), score);
        }

        // 3. 排序
        let mut ranked: Vec<(String, f64)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 4. 构建根因列表
        let root_causes: Vec<RankedRootCause> = ranked.iter().enumerate().map(|(rank, (node_id, score))| {
            let node = self.nodes.get(node_id);
            RankedRootCause {
                node_id: node_id.clone(),
                node_name: node.map(|n| n.name.clone()).unwrap_or_else(|| node_id.clone()),
                score: *score,
                rank: rank + 1,
                evidence: self.collect_evidence(node_id, anomaly_node),
                affected_downstream: self.get_all_downstream(node_id),
            }
        }).collect();

        // 5. 找到故障传播路径
        let propagation_paths = self.find_propagation_paths(anomaly_node);

        // 6. 计算置信度
        let confidence = if !root_causes.is_empty() {
            root_causes[0].score.min(1.0)
        } else {
            0.0
        };

        // 7. 生成建议
        let recommendations = self.generate_recommendations(&root_causes);

        RootCauseResult {
            id: Uuid::new_v4().to_string(),
            anomaly_metric: anomaly_node.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            root_causes,
            propagation_paths,
            confidence,
            analysis_duration_ms: start.elapsed().as_millis() as u64,
            recommendations,
        }
    }

    fn calculate_root_cause_score(&self, candidate: &str, anomaly: &str) -> f64 {
        let node = match self.nodes.get(candidate) {
            Some(n) => n,
            None => return 0.0,
        };

        // 基础分：节点自身的异常分数
        let mut score = node.anomaly_score * 0.4;

        // 传播分：到异常节点的路径权重
        let paths = self.find_paths(candidate, anomaly);
        if !paths.is_empty() {
            let avg_path_weight: f64 = paths.iter().map(|p| {
                p.iter().skip(1).map(|(_, w)| w).sum::<f64>() / p.len().max(1) as f64
            }).sum::<f64>() / paths.len() as f64;
            score += avg_path_weight * 0.3;
        }

        // 影响分：下游受影响节点数
        let downstream = self.get_all_downstream(candidate);
        score += (downstream.len() as f64 / self.nodes.len().max(1) as f64) * 0.2;

        // 中心性分：节点在图中的重要性
        let degree = self.adjacency.get(candidate).map(|e| e.len()).unwrap_or(0)
            + self.reverse_adjacency.get(candidate).map(|e| e.len()).unwrap_or(0);
        score += (degree as f64 / (self.edges.len().max(1) as f64 * 2.0)) * 0.1;

        score.min(1.0)
    }

    fn collect_evidence(&self, candidate: &str, anomaly: &str) -> Vec<String> {
        let mut evidence = Vec::new();

        if let Some(node) = self.nodes.get(candidate) {
            if node.anomaly_score > 0.5 {
                evidence.push(format!("节点 {} 异常分数 {:.2}", node.name, node.anomaly_score));
            }
        }

        let paths = self.find_paths(candidate, anomaly);
        if !paths.is_empty() {
            evidence.push(format!("存在 {} 条到异常节点的传播路径", paths.len()));
        }

        let downstream = self.get_all_downstream(candidate);
        if downstream.contains(&anomaly.to_string()) {
            evidence.push("异常节点在该节点的下游影响范围内".to_string());
        }

        evidence
    }

    fn find_propagation_paths(&self, anomaly: &str) -> Vec<FaultPropagationPath> {
        let mut paths = Vec::new();
        let upstream = self.get_all_upstream(anomaly);

        for source in &upstream {
            let source_paths = self.find_paths(source, anomaly);
            for path in source_paths {
                let total_weight: f64 = path.iter().skip(1).map(|(_, w)| w).sum();
                paths.push(FaultPropagationPath {
                    path: path.iter().map(|(n, _)| n.clone()).collect(),
                    total_delay_ms: (total_weight * 1000.0) as u64,
                    hop_count: path.len().saturating_sub(1),
                    confidence: (1.0 / path.len().max(1) as f64).min(1.0),
                });
            }
        }

        paths.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        paths.truncate(10);
        paths
    }

    fn find_paths(&self, start: &str, end: &str) -> Vec<Vec<(String, f64)>> {
        let mut paths = Vec::new();
        let mut current = vec![(start.to_string(), 0.0)];
        let mut visited = HashSet::new();
        visited.insert(start.to_string());

        self.dfs_paths(start, end, &mut current, &mut visited, &mut paths);
        paths
    }

    fn dfs_paths(&self, current: &str, end: &str, path: &mut Vec<(String, f64)>, visited: &mut HashSet<String>, paths: &mut Vec<Vec<(String, f64)>>) {
        if current == end {
            paths.push(path.clone());
            return;
        }

        if let Some(neighbors) = self.adjacency.get(current) {
            for (next, weight) in neighbors {
                if !visited.contains(next) {
                    visited.insert(next.clone());
                    path.push((next.clone(), *weight));
                    self.dfs_paths(next, end, path, visited, paths);
                    path.pop();
                    visited.remove(next);
                }
            }
        }
    }

    fn get_all_upstream(&self, node: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(node.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(upstreams) = self.reverse_adjacency.get(&current) {
                for (upstream, _) in upstreams {
                    if visited.insert(upstream.clone()) {
                        result.push(upstream.clone());
                        queue.push_back(upstream.clone());
                    }
                }
            }
        }

        result
    }

    fn get_all_downstream(&self, node: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(node.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(downstreams) = self.adjacency.get(&current) {
                for (downstream, _) in downstreams {
                    if visited.insert(downstream.clone()) {
                        result.push(downstream.clone());
                        queue.push_back(downstream.clone());
                    }
                }
            }
        }

        result
    }

    fn generate_recommendations(&self, root_causes: &[RankedRootCause]) -> Vec<String> {
        let mut recs = Vec::new();

        if let Some(top) = root_causes.first() {
            recs.push(format!(
                "优先排查根因节点 '{}'（排名第 {}, 分数 {:.2}）",
                top.node_name, top.rank, top.score
            ));
            recs.push(format!(
                "检查节点 '{}' 的日志、指标和最近变更",
                top.node_name
            ));
        }

        if root_causes.len() > 1 {
            recs.push(format!(
                "同时关注前 {} 个可疑节点，排除级联故障",
                root_causes.len().min(3)
            ));
        }

        recs.push("检查依赖链路中的网络延迟和超时配置".to_string());
        recs.push("验证相关服务的最近部署和配置变更".to_string());

        recs
    }

    /// 获取统计
    pub fn stats(&self) -> RootCauseStats {
        RootCauseStats {
            total_nodes: self.nodes.len(),
            total_edges: self.edges.len(),
            total_analyses: self.total_analyses.load(std::sync::atomic::Ordering::Relaxed),
            node_types: self.nodes.values().fold(HashMap::new(), |mut acc, n| {
                *acc.entry(n.node_type.clone()).or_insert(0) += 1;
                acc
            }),
        }
    }
}

impl Default for RootCauseAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// 根因分析统计
#[derive(Debug, Clone, Serialize)]
pub struct RootCauseStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub total_analyses: u64,
    pub node_types: HashMap<String, usize>,
}
