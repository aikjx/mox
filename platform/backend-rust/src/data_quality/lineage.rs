// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 数据血缘
//!
//! 追踪数据资产的上下游关系，支持：
//! - 血缘 DAG 构建
//! - 上下游递归查询
//! - 影响分析（修改一个资产影响哪些下游）
//! - 血缘路径查找
//! - 循环依赖检测
//! - Mermaid/DOT 可视化导出

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

/// 血缘节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub id: String,
    pub name: String,
    pub asset_type: String,
    pub metadata: HashMap<String, String>,
}

/// 血缘边（数据流向）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub transform_type: TransformType,
    pub description: String,
    pub metadata: HashMap<String, String>,
}

/// 转换类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TransformType {
    Copy,
    Filter,
    Aggregate,
    Join,
    Transform,
    Load,
    Extract,
    ApiCall,
    ModelTraining,
    ModelInference,
    Unknown,
}

/// 影响分析结果
#[derive(Debug, Clone, Serialize)]
pub struct LineageImpact {
    pub source_asset: String,
    pub downstream_assets: Vec<String>,
    pub total_impacted: usize,
    pub max_depth: usize,
    pub paths: Vec<Vec<String>>,
}

/// 数据血缘
pub struct DataLineage {
    nodes: HashMap<String, LineageNode>,
    edges: Vec<LineageEdge>,
    adjacency: HashMap<String, Vec<String>>,  // source -> targets
    reverse_adjacency: HashMap<String, Vec<String>>, // target -> sources
}

impl DataLineage {
    /// 创建数据血缘
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            adjacency: HashMap::new(),
            reverse_adjacency: HashMap::new(),
        }
    }

    /// 添加节点
    pub fn add_node(&mut self, node: LineageNode) {
        self.adjacency.entry(node.id.clone()).or_default();
        self.reverse_adjacency.entry(node.id.clone()).or_default();
        self.nodes.insert(node.id.clone(), node);
    }

    /// 添加边（source -> target 表示数据从 source 流向 target）
    pub fn add_edge(&mut self, source: &str, target: &str, transform_type: TransformType, description: &str) {
        // 确保节点存在
        if !self.nodes.contains_key(source) {
            self.add_node(LineageNode {
                id: source.to_string(),
                name: source.to_string(),
                asset_type: "unknown".to_string(),
                metadata: HashMap::new(),
            });
        }
        if !self.nodes.contains_key(target) {
            self.add_node(LineageNode {
                id: target.to_string(),
                name: target.to_string(),
                asset_type: "unknown".to_string(),
                metadata: HashMap::new(),
            });
        }

        let edge = LineageEdge {
            id: Uuid::new_v4().to_string(),
            source: source.to_string(),
            target: target.to_string(),
            transform_type,
            description: description.to_string(),
            metadata: HashMap::new(),
        };

        self.adjacency.get_mut(source).unwrap().push(target.to_string());
        self.reverse_adjacency.get_mut(target).unwrap().push(source.to_string());
        self.edges.push(edge);
    }

    /// 获取直接上游
    pub fn get_upstream(&self, asset_id: &str) -> Vec<&LineageNode> {
        self.reverse_adjacency.get(asset_id)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    /// 获取直接下游
    pub fn get_downstream(&self, asset_id: &str) -> Vec<&LineageNode> {
        self.adjacency.get(asset_id)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    /// 获取所有上游（递归）
    pub fn get_all_upstream(&self, asset_id: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(asset_id.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(upstreams) = self.reverse_adjacency.get(&current) {
                for upstream in upstreams {
                    if visited.insert(upstream.clone()) {
                        result.push(upstream.clone());
                        queue.push_back(upstream.clone());
                    }
                }
            }
        }

        result
    }

    /// 获取所有下游（递归）
    pub fn get_all_downstream(&self, asset_id: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(asset_id.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(downstreams) = self.adjacency.get(&current) {
                for downstream in downstreams {
                    if visited.insert(downstream.clone()) {
                        result.push(downstream.clone());
                        queue.push_back(downstream.clone());
                    }
                }
            }
        }

        result
    }

    /// 影响分析
    pub fn analyze_impact(&self, asset_id: &str) -> LineageImpact {
        let downstream = self.get_all_downstream(asset_id);
        let paths = self.find_all_paths(asset_id);
        let max_depth = paths.iter().map(|p| p.len()).max().unwrap_or(0);

        LineageImpact {
            source_asset: asset_id.to_string(),
            downstream_assets: downstream.clone(),
            total_impacted: downstream.len(),
            max_depth,
            paths,
        }
    }

    /// 查找所有血缘路径（DFS）
    pub fn find_all_paths(&self, start: &str) -> Vec<Vec<String>> {
        let mut paths = Vec::new();
        let mut current_path = vec![start.to_string()];
        let mut visited = HashSet::new();
        visited.insert(start.to_string());

        self.dfs_paths(start, &mut current_path, &mut visited, &mut paths);
        paths
    }

    fn dfs_paths(&self, node: &str, current: &mut Vec<String>, visited: &mut HashSet<String>, paths: &mut Vec<Vec<String>>) {
        let mut has_downstream = false;
        if let Some(downstreams) = self.adjacency.get(node) {
            for downstream in downstreams {
                if !visited.contains(downstream) {
                    has_downstream = true;
                    visited.insert(downstream.clone());
                    current.push(downstream.clone());
                    self.dfs_paths(downstream, current, visited, paths);
                    current.pop();
                    visited.remove(downstream);
                }
            }
        }
        if !has_downstream && current.len() > 1 {
            paths.push(current.clone());
        }
    }

    /// 检测循环依赖
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node in self.nodes.keys() {
            if !visited.contains(node) {
                self.dfs_cycle(node, &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }

        cycles
    }

    fn dfs_cycle(&self, node: &str, visited: &mut HashSet<String>, rec_stack: &mut HashSet<String>, path: &mut Vec<String>, cycles: &mut Vec<Vec<String>>) {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = self.adjacency.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.dfs_cycle(neighbor, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(neighbor) {
                    // 发现循环
                    if let Some(start) = path.iter().position(|n| n == neighbor) {
                        cycles.push(path[start..].to_vec());
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
    }

    /// 获取节点
    pub fn get_node(&self, id: &str) -> Option<&LineageNode> {
        self.nodes.get(id)
    }

    /// 获取节点间的边
    pub fn get_edges_between(&self, source: &str, target: &str) -> Vec<&LineageEdge> {
        self.edges.iter()
            .filter(|e| e.source == source && e.target == target)
            .collect()
    }

    /// 导出为 Mermaid
    pub fn to_mermaid(&self) -> String {
        let mut mermaid = "graph LR\n".to_string();
        for node in self.nodes.values() {
            mermaid.push_str(&format!("    {}[\"{}\"]\n", node.id.replace('.', "_"), node.name));
        }
        for edge in &self.edges {
            let s = edge.source.replace('.', "_");
            let t = edge.target.replace('.', "_");
            let label = format!("{:?}", edge.transform_type);
            mermaid.push_str(&format!("    {} -->|{}| {}\n", s, label, t));
        }
        mermaid
    }

    /// 导出为 DOT
    pub fn to_dot(&self) -> String {
        let mut dot = "digraph lineage {\n".to_string();
        dot.push_str("    rankdir=LR;\n");
        dot.push_str("    node [shape=box, style=rounded];\n");
        for node in self.nodes.values() {
            dot.push_str(&format!("    \"{}\" [label=\"{}\"];\n", node.id, node.name));
        }
        for edge in &self.edges {
            dot.push_str(&format!("    \"{}\" -> \"{}\" [label=\"{:?}\"];\n", edge.source, edge.target, edge.transform_type));
        }
        dot.push_str("}\n");
        dot
    }

    /// 获取统计
    pub fn stats(&self) -> LineageStats {
        LineageStats {
            total_nodes: self.nodes.len(),
            total_edges: self.edges.len(),
            cycles: self.detect_cycles().len(),
            by_transform: self.edges.iter().fold(HashMap::new(), |mut acc, e| {
                *acc.entry(format!("{:?}", e.transform_type)).or_insert(0) += 1;
                acc
            }),
        }
    }
}

impl Default for DataLineage {
    fn default() -> Self {
        Self::new()
    }
}

/// 血缘统计
#[derive(Debug, Clone, Serialize)]
pub struct LineageStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub cycles: usize,
    pub by_transform: HashMap<String, usize>,
}
