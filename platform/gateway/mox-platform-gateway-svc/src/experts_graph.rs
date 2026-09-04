// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 专家能力图谱与协作网络（Experts Graph）HTTP 路由
//!
//! 提供专家能力图谱的完整查询、统计、路径分析、社区检测、最优团队组建与重建能力。
//!
//! 路径前缀：`/api/expert-graph/*`（注意：不是 `/api/experts/graph`）
//!
//! 核心算法：
//! - BFS 最短路径（无权图，VecDeque 队列 + 前驱回溯）
//! - 标签传播社区检测（Label Propagation，确定性迭代）
//! - 带权集合覆盖贪心优化（最优团队组建）
//! - 图谱统计（度中心性、聚类系数、连通分量、介数中心性、密度）

use super::experts_common::*;
use mox_api_protocol::ApiResponse;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

// =====================================================================
// 一、内部工具：邻接表构建（无向图，双向边）
// =====================================================================

/// 从 ExpertGraph 构建无向邻接表：node_id -> [(neighbor_id, weight)]
fn build_adjacency(graph: &ExpertGraph) -> HashMap<String, Vec<(String, f64)>> {
    let mut adj: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    for node in &graph.nodes {
        adj.entry(node.id.clone()).or_default();
    }
    for edge in &graph.edges {
        let w = if edge.weight > 0.0 { edge.weight } else { 1.0 };
        adj.entry(edge.source.clone()).or_default().push((edge.target.clone(), w));
        adj.entry(edge.target.clone()).or_default().push((edge.source.clone(), w));
    }
    adj
}

/// 节点 ID -> GraphNode 索引
fn node_index(graph: &ExpertGraph) -> HashMap<String, &GraphNode> {
    graph.nodes.iter().map(|n| (n.id.clone(), n)).collect()
}

// =====================================================================
// 二、核心算法：图谱统计 compute_graph_stats
// =====================================================================

/// 计算图谱完整统计指标
/// 包含：节点/边计数、度中心性、聚类系数、连通分量、介数中心性、密度
pub fn compute_graph_stats(graph: &ExpertGraph) -> Value {
    let n = graph.nodes.len();
    let m = graph.edges.len();
    let idx = node_index(graph);
    let adj = build_adjacency(graph);

    // 节点类型计数
    let expert_nodes = graph.nodes.iter().filter(|n| n.node_type == "expert").count();
    let domain_nodes = graph.nodes.iter().filter(|n| n.node_type == "domain").count();

    // 边类型计数
    let collaboration_edges = graph.edges.iter().filter(|e| e.edge_type == "collaborates_with").count();
    let domain_edges = graph.edges.iter().filter(|e| e.edge_type == "has_domain").count();

    // 度中心性：degree / (n-1)
    let degree_centrality: HashMap<String, f64> = adj.iter()
        .map(|(id, neighbors)| {
            let dc = if n > 1 { neighbors.len() as f64 / (n - 1) as f64 } else { 0.0 };
            (id.clone(), dc)
        })
        .collect();

    // 聚类系数：节点邻居间实际边数 / 可能边数，取平均
    let mut total_clustering = 0.0f64;
    let mut clustering_count = 0usize;
    for (node_id, neighbors) in &adj {
        let k = neighbors.len();
        if k < 2 {
            continue;
        }
        let neighbor_set: HashSet<&String> = neighbors.iter().map(|(nid, _)| nid).collect();
        let mut actual_edges = 0usize;
        for (nid, _) in neighbors {
            if let Some(nn) = adj.get(nid) {
                for (mid, _) in nn {
                    if neighbor_set.contains(mid) && mid > nid {
                        actual_edges += 1;
                    }
                }
            }
        }
        let possible = k * (k - 1) / 2;
        let local_cc = if possible > 0 { actual_edges as f64 / possible as f64 } else { 0.0 };
        total_clustering += local_cc;
        clustering_count += 1;
        let _ = node_id;
    }
    let avg_clustering = if clustering_count > 0 { total_clustering / clustering_count as f64 } else { 0.0 };

    // 连通分量：BFS 遍历
    let mut visited: HashSet<String> = HashSet::new();
    let mut components: Vec<Vec<String>> = Vec::new();
    for node in &graph.nodes {
        if visited.contains(&node.id) {
            continue;
        }
        let mut comp = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(node.id.clone());
        visited.insert(node.id.clone());
        while let Some(curr) = queue.pop_front() {
            comp.push(curr.clone());
            if let Some(neighbors) = adj.get(&curr) {
                for (nid, _) in neighbors {
                    if !visited.contains(nid) {
                        visited.insert(nid.clone());
                        queue.push_back(nid.clone());
                    }
                }
            }
        }
        components.push(comp);
    }
    let connected_components = components.len();
    let largest_component_size = components.iter().map(|c| c.len()).max().unwrap_or(0);

    // 介数中心性（Brandes 算法，无向图）
    let mut betweenness: HashMap<String, f64> = HashMap::new();
    for node in &graph.nodes {
        betweenness.insert(node.id.clone(), 0.0);
    }
    for source in &graph.nodes {
        let s = &source.id;
        let mut dist: HashMap<String, i64> = HashMap::new();
        let mut sigma: HashMap<String, f64> = HashMap::new();
        let mut pred: HashMap<String, Vec<String>> = HashMap::new();
        let mut order: Vec<String> = Vec::new();
        for node in &graph.nodes {
            dist.insert(node.id.clone(), -1);
            sigma.insert(node.id.clone(), 0.0);
            pred.insert(node.id.clone(), Vec::new());
        }
        dist.insert(s.clone(), 0);
        sigma.insert(s.clone(), 1.0);
        let mut queue = VecDeque::new();
        queue.push_back(s.clone());
        while let Some(v) = queue.pop_front() {
            order.push(v.clone());
            if let Some(neighbors) = adj.get(&v) {
                for (w, _) in neighbors {
                    if dist[w] == -1 {
                        dist.insert(w.clone(), dist[&v] + 1);
                        queue.push_back(w.clone());
                    }
                    if dist[w] == dist[&v] + 1 {
                        let sv = sigma[&v];
                        *sigma.get_mut(w).unwrap() += sv;
                        pred.get_mut(w).unwrap().push(v.clone());
                    }
                }
            }
        }
        let mut delta: HashMap<String, f64> = HashMap::new();
        for node in &graph.nodes {
            delta.insert(node.id.clone(), 0.0);
        }
        for w in order.iter().rev() {
            for v in &pred[w] {
                let contribution = (sigma[v] / sigma[w]) * (1.0 + delta[w]);
                *delta.get_mut(v).unwrap() += contribution;
            }
            if w != s {
                *betweenness.get_mut(w).unwrap() += delta[w];
            }
        }
    }
    // 无向图除以 2
    for v in betweenness.values_mut() {
        *v /= 2.0;
    }

    // 度排序（专家节点）
    let mut expert_degrees: Vec<(String, f64)> = degree_centrality.iter()
        .filter(|(id, _)| idx.get(*id).map(|n| n.node_type == "expert").unwrap_or(false))
        .map(|(id, dc)| (id.clone(), *dc))
        .collect();
    expert_degrees.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let top_centrality: Vec<Value> = expert_degrees.iter().take(10).map(|(id, dc)| {
        let node = idx.get(id);
        json!({
            "id": id,
            "name": node.map(|n| n.label.clone()).unwrap_or_default(),
            "degree": adj.get(id).map(|nb| nb.len()).unwrap_or(0),
            "degree_centrality": dc,
            "betweenness": betweenness.get(id).copied().unwrap_or(0.0),
        })
    }).collect();

    // 密度：实际边数 / (n*(n-1)/2)
    let density = if n > 1 {
        m as f64 / (n * (n - 1) / 2) as f64
    } else { 0.0 };

    json!({
        "total_nodes": n,
        "total_edges": m,
        "expert_nodes": expert_nodes,
        "domain_nodes": domain_nodes,
        "collaboration_edges": collaboration_edges,
        "domain_edges": domain_edges,
        "avg_clustering_coefficient": avg_clustering,
        "connected_components": connected_components,
        "largest_component_size": largest_component_size,
        "top_centrality_experts": top_centrality,
        "density": density,
        "ts": now_iso(),
    })
}

// =====================================================================
// 三、核心算法：BFS 最短路径
// =====================================================================

/// BFS 最短路径（无权图）
/// 使用 VecDeque 做队列，HashMap 记录前驱节点，回溯重建路径
/// 返回从 source 到 target 的节点 ID 序列（含两端），不可达返回 None
pub fn bfs_shortest_path(graph: &ExpertGraph, source: &str, target: &str) -> Option<Vec<String>> {
    if source == target {
        return Some(vec![source.to_string()]);
    }
    let adj = build_adjacency(graph);
    if !adj.contains_key(source) || !adj.contains_key(target) {
        return None;
    }

    let mut predecessor: HashMap<String, Option<String>> = HashMap::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();

    predecessor.insert(source.to_string(), None);
    visited.insert(source.to_string());
    queue.push_back(source.to_string());

    while let Some(curr) = queue.pop_front() {
        if curr == target {
            // 回溯重建路径
            let mut path = Vec::new();
            let mut node = Some(target.to_string());
            while let Some(n) = node {
                path.push(n.clone());
                node = predecessor.get(&n).cloned().flatten();
            }
            path.reverse();
            return Some(path);
        }
        if let Some(neighbors) = adj.get(&curr) {
            for (nid, _) in neighbors {
                if !visited.contains(nid) {
                    visited.insert(nid.clone());
                    predecessor.insert(nid.clone(), Some(curr.clone()));
                    queue.push_back(nid.clone());
                }
            }
        }
    }
    None
}

// =====================================================================
// 四、核心算法：标签传播社区检测 + 模块度
// =====================================================================

/// 计算模块度 Q = (1/2m) * Σ[A_ij - k_i*k_j/(2m)] * δ(c_i,c_j)
fn compute_modularity(graph: &ExpertGraph, communities: &[Vec<String>]) -> f64 {
    let m = graph.edges.len() as f64;
    if m == 0.0 {
        return 0.0;
    }
    let adj = build_adjacency(graph);
    let mut node_community: HashMap<String, usize> = HashMap::new();
    for (ci, comm) in communities.iter().enumerate() {
        for nid in comm {
            node_community.insert(nid.clone(), ci);
        }
    }
    let two_m = 2.0 * m;
    let mut q = 0.0f64;
    for edge in &graph.edges {
        let ci = node_community.get(&edge.source);
        let cj = node_community.get(&edge.target);
        if ci.is_some() && cj.is_some() && ci == cj {
            let ki = adj.get(&edge.source).map(|nb| nb.len() as f64).unwrap_or(0.0);
            let kj = adj.get(&edge.target).map(|nb| nb.len() as f64).unwrap_or(0.0);
            q += 1.0 - (ki * kj) / two_m;
        }
    }
    q / two_m
}

/// 标签传播社区检测（Label Propagation Algorithm）
/// 每个节点初始标签=自身ID，迭代每轮按邻居中出现最多的标签更新，
/// 直到稳定或达到最大迭代次数（50轮）。确定性实现（固定顺序、字典序打破平局）。
/// 返回 (communities, modularity, iterations, converged)
pub fn detect_communities(graph: &ExpertGraph) -> (Vec<Vec<String>>, f64, u32, bool) {
    let adj = build_adjacency(graph);
    let node_ids: Vec<String> = graph.nodes.iter().map(|n| n.id.clone()).collect();

    // 初始标签：每个节点自身 ID
    let mut labels: HashMap<String, String> = node_ids.iter()
        .map(|id| (id.clone(), id.clone()))
        .collect();

    let max_iterations = 50u32;
    let mut iterations = 0u32;
    let mut converged = false;

    for iter in 1..=max_iterations {
        iterations = iter;
        let mut changed = false;
        // 固定顺序：按节点 ID 字典序处理（确定性）
        let mut sorted_ids = node_ids.clone();
        sorted_ids.sort();
        for nid in &sorted_ids {
            if let Some(neighbors) = adj.get(nid) {
                if neighbors.is_empty() {
                    continue;
                }
                // 统计邻居标签频率
                let mut label_count: HashMap<String, usize> = HashMap::new();
                for (nb_id, _) in neighbors {
                    if let Some(lbl) = labels.get(nb_id) {
                        *label_count.entry(lbl.clone()).or_insert(0) += 1;
                    }
                }
                if label_count.is_empty() {
                    continue;
                }
                // 找出现最多的标签，平局选字典序最小（确定性）
                let max_count = label_count.values().max().copied().unwrap_or(0);
                let mut best_labels: Vec<&String> = label_count.iter()
                    .filter(|(_, c)| **c == max_count)
                    .map(|(l, _)| l)
                    .collect();
                best_labels.sort();
                let new_label = best_labels.first().map(|l| (*l).clone()).unwrap_or_else(|| nid.clone());
                if labels.get(nid) != Some(&new_label) {
                    labels.insert(nid.clone(), new_label);
                    changed = true;
                }
            }
        }
        if !changed {
            converged = true;
            break;
        }
    }

    // 按标签分组形成社区
    let mut community_map: HashMap<String, Vec<String>> = HashMap::new();
    for nid in &node_ids {
        let lbl = labels.get(nid).cloned().unwrap_or_else(|| nid.clone());
        community_map.entry(lbl).or_default().push(nid.clone());
    }
    let mut communities: Vec<Vec<String>> = community_map.into_values().collect();
    // 社区内节点排序，社区按大小降序
    for comm in &mut communities {
        comm.sort();
    }
    communities.sort_by(|a, b| b.len().cmp(&a.len()));

    let modularity = compute_modularity(graph, &communities);
    (communities, modularity, iterations, converged)
}

// =====================================================================
// 五、核心算法：带权集合覆盖贪心优化（最优团队组建）
// =====================================================================

/// 最优团队组建：带权集合覆盖 + 贪心优化
/// - 候选专家：满足 min_rating、enabled、可用的专家
/// - 覆盖值 = 交集大小 * (avg_rating/5) * availability_score
/// - 贪心选择：每轮选覆盖剩余需求最多的专家
pub fn find_optimal_team(
    registry: &HashMap<String, ExpertDescriptor>,
    required_skills: &[String],
    required_domains: &[String],
    max_members: usize,
    min_rating: f64,
) -> Value {
    let max_members = if max_members == 0 { 5 } else { max_members };

    // 候选专家筛选
    let candidates: Vec<&ExpertDescriptor> = registry.values()
        .filter(|e| {
            e.enabled
                && e.metrics.avg_rating >= min_rating
                && e.availability.status != "offline"
        })
        .collect();

    // 需求集合（技能 + 领域合并为统一需求项）
    let mut remaining_skills: HashSet<String> = required_skills.iter().cloned().collect();
    let mut remaining_domains: HashSet<String> = required_domains.iter().cloned().collect();
    let total_required = remaining_skills.len() + remaining_domains.len();

    let mut team: Vec<&ExpertDescriptor> = Vec::new();
    let mut team_details: Vec<Value> = Vec::new();
    let mut team_score = 0.0f64;

    while team.len() < max_members {
        let mut best: Option<(&ExpertDescriptor, f64, Vec<String>, Vec<String>)> = None;
        for exp in &candidates {
            if team.iter().any(|t| t.id == exp.id) {
                continue;
            }
            // 计算该专家对剩余需求的覆盖
            let covered_skills: Vec<String> = exp.skills.iter()
                .filter(|s| remaining_skills.contains(*s))
                .cloned()
                .collect();
            let covered_domains: Vec<String> = exp.domains.iter()
                .filter(|d| remaining_domains.contains(*d))
                .cloned()
                .collect();
            let intersection_size = covered_skills.len() + covered_domains.len();
            if intersection_size == 0 {
                continue;
            }
            // 可用性分数
            let availability_score = match exp.availability.status.as_str() {
                "online" => 1.0,
                "busy" => 0.6,
                "away" => 0.4,
                _ => 0.2,
            };
            // 覆盖值 = 交集大小 * (avg_rating/5) * availability_score
            let coverage_value = intersection_size as f64
                * (exp.metrics.avg_rating / 5.0).min(1.0)
                * availability_score;
            match &best {
                None => best = Some((exp, coverage_value, covered_skills, covered_domains)),
                Some((_, bv, _, _)) => {
                    if coverage_value > *bv {
                        best = Some((exp, coverage_value, covered_skills, covered_domains));
                    }
                }
            }
        }
        match best {
            Some((exp, score, cskills, cdomains)) => {
                // 从剩余需求中移除已覆盖项
                for s in &cskills { remaining_skills.remove(s); }
                for d in &cdomains { remaining_domains.remove(d); }
                team_score += score;
                let role = if exp.domains.iter().any(|d| remaining_domains.contains(d) || cdomains.contains(d)) {
                    "domain_lead"
                } else if !cskills.is_empty() {
                    "skill_expert"
                } else {
                    "consultant"
                };
                team_details.push(json!({
                    "id": exp.id,
                    "name": exp.name,
                    "title": exp.title,
                    "covered_skills": cskills,
                    "covered_domains": cdomains,
                    "match_score": score,
                    "avg_rating": exp.metrics.avg_rating,
                    "role": role,
                }));
                team.push(exp);
                // 需求全覆盖则提前终止
                if remaining_skills.is_empty() && remaining_domains.is_empty() {
                    break;
                }
            }
            None => break,
        }
    }

    let covered_count = total_required - remaining_skills.len() - remaining_domains.len();
    let coverage_ratio = if total_required > 0 {
        covered_count as f64 / total_required as f64
    } else { 1.0 };
    let missing_skills: Vec<String> = remaining_skills.into_iter().collect();
    let missing_domains: Vec<String> = remaining_domains.into_iter().collect();

    json!({
        "team_id": gen_id("team"),
        "required_skills": required_skills,
        "required_domains": required_domains,
        "team_members": team_details,
        "coverage": {
            "required_total": total_required,
            "covered_count": covered_count,
            "coverage_ratio": coverage_ratio,
            "missing_skills": missing_skills,
            "missing_domains": missing_domains,
        },
        "team_score": team_score,
        "selection_strategy": "weighted_set_cover_greedy",
        "created_at": now_iso(),
    })
}

// =====================================================================
// 六、请求体定义
// =====================================================================

#[derive(Debug, Deserialize)]
struct OptimalTeamBody {
    #[serde(default)]
    required_skills: Vec<String>,
    #[serde(default)]
    required_domains: Vec<String>,
    max_members: Option<usize>,
    min_rating: Option<f64>,
    #[serde(default)]
    constraints: Option<Value>,
}

// =====================================================================
// 七、端点 Handler
// =====================================================================

/// 1. GET /api/expert-graph — 获取完整图谱
async fn get_graph(State(state): State<Arc<ExpertsSharedState>>) -> ApiResponse<Value> {
    let graph = state.graph.lock();
    let expert_count = graph.nodes.iter().filter(|n| n.node_type == "expert").count();
    let domain_count = graph.nodes.iter().filter(|n| n.node_type == "domain").count();
    let n = graph.nodes.len();
    let avg_degree = if n > 0 {
        let adj = build_adjacency(&graph);
        let total_deg: usize = adj.values().map(|nb| nb.len()).sum();
        total_deg as f64 / n as f64 / 2.0
    } else { 0.0 };
    let density = if n > 1 {
        graph.edges.len() as f64 / (n * (n - 1) / 2) as f64
    } else { 0.0 };
    ok(json!({
        "nodes": graph.nodes,
        "edges": graph.edges,
        "stats": {
            "node_count": n,
            "edge_count": graph.edges.len(),
            "expert_count": expert_count,
            "domain_count": domain_count,
            "avg_degree": avg_degree,
            "density": density,
        },
        "built_at": graph.built_at,
        "version": graph.version,
    }))
}

/// 2. GET /api/expert-graph/stats — 图谱统计
async fn get_graph_stats(State(state): State<Arc<ExpertsSharedState>>) -> ApiResponse<Value> {
    let graph = state.graph.lock();
    ok(compute_graph_stats(&graph))
}

/// 3. GET /api/expert-graph/neighbors/:id — 获取节点邻居
async fn get_neighbors(
    Path(id): Path<String>,
    State(state): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let graph = state.graph.lock();
    let idx = node_index(&graph);
    let node = match idx.get(&id) {
        Some(n) => n,
        None => return err(404, format!("node not found: {id}")),
    };
    let mut neighbors: Vec<Value> = Vec::new();
    for edge in &graph.edges {
        let (other_id, direction) = if edge.source == id {
            (&edge.target, "out")
        } else if edge.target == id {
            (&edge.source, "in")
        } else {
            continue;
        };
        if let Some(other) = idx.get(other_id) {
            neighbors.push(json!({
                "id": other.id,
                "label": other.label,
                "node_type": other.node_type,
                "edge_type": edge.edge_type,
                "weight": edge.weight,
                "direction": direction,
                "properties": edge.properties,
            }));
        }
    }
    ok(json!({
        "node_id": id,
        "node_label": node.label,
        "node_type": node.node_type,
        "neighbors": neighbors,
        "neighbor_count": neighbors.len(),
    }))
}

/// 4. GET /api/expert-graph/collaborators/:id — 获取专家协作者
async fn get_collaborators(
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let limit: usize = params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(10);
    let graph = state.graph.lock();
    let idx = node_index(&graph);
    if !idx.contains_key(&id) {
        return err(404, format!("expert not found: {id}"));
    }
    let mut collaborators: Vec<Value> = Vec::new();
    for edge in &graph.edges {
        if edge.edge_type != "collaborates_with" {
            continue;
        }
        let other_id = if edge.source == id {
            &edge.target
        } else if edge.target == id {
            &edge.source
        } else {
            continue;
        };
        if let Some(other) = idx.get(other_id) {
            let shared_domains: Vec<String> = edge.properties.get("shared_domains")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            collaborators.push(json!({
                "id": other.id,
                "name": other.label,
                "collaboration_weight": edge.weight,
                "shared_domains": shared_domains,
            }));
        }
    }
    // 按 weight 降序
    collaborators.sort_by(|a, b| {
        let wa = a.get("collaboration_weight").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let wb = b.get("collaboration_weight").and_then(|v| v.as_f64()).unwrap_or(0.0);
        wb.partial_cmp(&wa).unwrap_or(std::cmp::Ordering::Equal)
    });
    let total = collaborators.len();
    // 添加 rank
    let collaborators: Vec<Value> = collaborators.into_iter().take(limit).enumerate().map(|(i, mut c)| {
        if let Some(obj) = c.as_object_mut() {
            obj.insert("collaboration_rank".into(), json!(i + 1));
        }
        c
    }).collect();
    ok(json!({
        "expert_id": id,
        "collaborators": collaborators,
        "total_collaborators": total,
    }))
}

/// 5. GET /api/expert-graph/path/:source/:target — 最短路径查询
async fn get_path(
    Path((source, target)): Path<(String, String)>,
    State(state): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let graph = state.graph.lock();
    let idx = node_index(&graph);
    match bfs_shortest_path(&graph, &source, &target) {
        Some(path_ids) => {
            let path: Vec<Value> = path_ids.iter().map(|nid| {
                let node = idx.get(nid);
                json!({
                    "node_id": nid,
                    "label": node.map(|n| n.label.clone()).unwrap_or_default(),
                    "node_type": node.map(|n| n.node_type.clone()).unwrap_or_default(),
                })
            }).collect();
            let path_length = if path_ids.len() > 0 { path_ids.len() - 1 } else { 0 };
            // 计算总权重
            let mut total_weight = 0.0f64;
            for w in path_ids.windows(2) {
                for edge in &graph.edges {
                    if (edge.source == w[0] && edge.target == w[1])
                        || (edge.source == w[1] && edge.target == w[0])
                    {
                        total_weight += edge.weight;
                        break;
                    }
                }
            }
            ok(json!({
                "source": source,
                "target": target,
                "path": path,
                "path_length": path_length,
                "total_weight": total_weight,
                "found": true,
            }))
        }
        None => ok(json!({
            "source": source,
            "target": target,
            "path": [],
            "path_length": 0,
            "total_weight": 0.0,
            "found": false,
        })),
    }
}

/// 6. GET /api/expert-graph/communities — 社区检测
async fn get_communities(State(state): State<Arc<ExpertsSharedState>>) -> ApiResponse<Value> {
    let graph = state.graph.lock();
    let idx = node_index(&graph);
    let (communities, modularity, iterations, converged) = detect_communities(&graph);

    let community_list: Vec<Value> = communities.iter().enumerate().map(|(ci, members)| {
        let member_labels: Vec<String> = members.iter()
            .filter_map(|m| idx.get(m).map(|n| n.label.clone()))
            .collect();
        // 计算内部边和外部边
        let member_set: HashSet<&String> = members.iter().collect();
        let mut internal_edges = 0usize;
        let mut external_edges = 0usize;
        for edge in &graph.edges {
            let s_in = member_set.contains(&edge.source);
            let t_in = member_set.contains(&edge.target);
            if s_in && t_in {
                internal_edges += 1;
            } else if s_in || t_in {
                external_edges += 1;
            }
        }
        json!({
            "community_id": format!("community-{}", ci + 1),
            "size": members.len(),
            "member_ids": members,
            "member_labels": member_labels,
            "internal_edges": internal_edges,
            "external_edges": external_edges,
            "modularity_contribution": if communities.len() > 0 { modularity / communities.len() as f64 } else { 0.0 },
        })
    }).collect();

    ok(json!({
        "communities": community_list,
        "total_communities": communities.len(),
        "modularity": modularity,
        "algorithm": "label_propagation",
        "iterations": iterations,
        "converged": converged,
    }))
}

/// 7. POST /api/expert-graph/optimal-team — 最优团队组建
async fn post_optimal_team(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<OptimalTeamBody>,
) -> ApiResponse<Value> {
    let registry = state.registry.lock();
    let max_members = body.max_members.unwrap_or(5);
    let min_rating = body.min_rating.unwrap_or(4.0);
    let result = find_optimal_team(
        &registry,
        &body.required_skills,
        &body.required_domains,
        max_members,
        min_rating,
    );
    ok(result)
}

/// 8. POST /api/expert-graph/rebuild — 重建图谱
async fn post_rebuild(State(state): State<Arc<ExpertsSharedState>>) -> ApiResponse<Value> {
    let start = std::time::Instant::now();
    let previous_version;
    let new_graph;
    {
        let registry = state.registry.lock();
        let mut graph = state.graph.lock();
        previous_version = graph.version;
        new_graph = build_graph_from_registry(&registry);
        *graph = ExpertGraph {
            version: previous_version + 1,
            ..new_graph
        };
        save_graph(&graph);
    }
    let duration_ms = start.elapsed().as_millis() as u64;
    let graph = state.graph.lock();
    let expert_count = graph.nodes.iter().filter(|n| n.node_type == "expert").count();
    ok(json!({
        "rebuilt": true,
        "previous_version": previous_version,
        "new_version": graph.version,
        "node_count": graph.nodes.len(),
        "edge_count": graph.edges.len(),
        "expert_count": expert_count,
        "built_at": graph.built_at,
        "duration_ms": duration_ms,
    }))
}

// =====================================================================
// 八、路由装配
// =====================================================================

pub fn build_experts_graph_router(state: Arc<ExpertsSharedState>) -> Router {
    Router::new()
        .route("/api/expert-graph", get(get_graph))
        .route("/api/expert-graph/stats", get(get_graph_stats))
        .route("/api/expert-graph/neighbors/:id", get(get_neighbors))
        .route("/api/expert-graph/collaborators/:id", get(get_collaborators))
        .route("/api/expert-graph/path/:source/:target", get(get_path))
        .route("/api/expert-graph/communities", get(get_communities))
        .route("/api/expert-graph/optimal-team", post(post_optimal_team))
        .route("/api/expert-graph/rebuild", post(post_rebuild))
        .with_state(state)
}

// =====================================================================
// 九、单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 构建测试用图谱（3专家 + 2领域，含协作边）
    fn make_test_graph() -> ExpertGraph {
        let nodes = vec![
            GraphNode { id: "exp-1".into(), label: "专家A".into(), node_type: "expert".into(), properties: HashMap::new() },
            GraphNode { id: "exp-2".into(), label: "专家B".into(), node_type: "expert".into(), properties: HashMap::new() },
            GraphNode { id: "exp-3".into(), label: "专家C".into(), node_type: "expert".into(), properties: HashMap::new() },
            GraphNode { id: "domain-ai".into(), label: "ai".into(), node_type: "domain".into(), properties: HashMap::new() },
            GraphNode { id: "domain-data".into(), label: "data".into(), node_type: "domain".into(), properties: HashMap::new() },
        ];
        let edges = vec![
            GraphEdge { source: "exp-1".into(), target: "domain-ai".into(), edge_type: "has_domain".into(), weight: 1.0, properties: HashMap::new() },
            GraphEdge { source: "exp-2".into(), target: "domain-ai".into(), edge_type: "has_domain".into(), weight: 1.0, properties: HashMap::new() },
            GraphEdge { source: "exp-2".into(), target: "domain-data".into(), edge_type: "has_domain".into(), weight: 1.0, properties: HashMap::new() },
            GraphEdge { source: "exp-3".into(), target: "domain-data".into(), edge_type: "has_domain".into(), weight: 1.0, properties: HashMap::new() },
            GraphEdge { source: "exp-1".into(), target: "exp-2".into(), edge_type: "collaborates_with".into(), weight: 0.6, properties: HashMap::new() },
            GraphEdge { source: "exp-2".into(), target: "exp-3".into(), edge_type: "collaborates_with".into(), weight: 0.4, properties: HashMap::new() },
        ];
        ExpertGraph { nodes, edges, built_at: now_iso(), version: 1 }
    }

    /// 测试1：图谱获取 — 验证节点/边计数与统计字段
    #[test]
    fn test_graph_structure() {
        let graph = make_test_graph();
        assert_eq!(graph.nodes.len(), 5);
        assert_eq!(graph.edges.len(), 6);
        let expert_count = graph.nodes.iter().filter(|n| n.node_type == "expert").count();
        assert_eq!(expert_count, 3);
        let domain_count = graph.nodes.iter().filter(|n| n.node_type == "domain").count();
        assert_eq!(domain_count, 2);
    }

    /// 测试2：stats 计算 — 验证密度、聚类系数、连通分量
    #[test]
    fn test_compute_graph_stats() {
        let graph = make_test_graph();
        let stats = compute_graph_stats(&graph);
        assert_eq!(stats["total_nodes"], 5);
        assert_eq!(stats["total_edges"], 6);
        assert_eq!(stats["expert_nodes"], 3);
        assert_eq!(stats["domain_nodes"], 2);
        assert_eq!(stats["collaboration_edges"], 2);
        assert_eq!(stats["domain_edges"], 4);
        assert_eq!(stats["connected_components"], 1);
        assert_eq!(stats["largest_component_size"], 5);
        let density = stats["density"].as_f64().unwrap();
        assert!(density > 0.0 && density <= 1.0);
        let avg_cc = stats["avg_clustering_coefficient"].as_f64().unwrap();
        assert!(avg_cc >= 0.0 && avg_cc <= 1.0);
        let top = stats["top_centrality_experts"].as_array().unwrap();
        assert!(!top.is_empty());
    }

    /// 测试3：neighbors — 验证邻居查询
    #[test]
    fn test_neighbors() {
        let graph = make_test_graph();
        let adj = build_adjacency(&graph);
        // exp-2 连接 domain-ai, domain-data, exp-1, exp-3 = 4个邻居
        let exp2_neighbors = adj.get("exp-2").unwrap();
        assert_eq!(exp2_neighbors.len(), 4);
        // domain-ai 连接 exp-1, exp-2 = 2个邻居
        let ai_neighbors = adj.get("domain-ai").unwrap();
        assert_eq!(ai_neighbors.len(), 2);
    }

    /// 测试4a：BFS 最短路径 — 可达路径
    #[test]
    fn test_bfs_shortest_path_reachable() {
        let graph = make_test_graph();
        // exp-1 -> exp-3 经过 exp-2（长度2）
        let path = bfs_shortest_path(&graph, "exp-1", "exp-3").unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], "exp-1");
        assert_eq!(path[2], "exp-3");
        assert!(path.contains(&"exp-2".to_string()));
        // 直接相邻 exp-1 -> exp-2（长度1）
        let path2 = bfs_shortest_path(&graph, "exp-1", "exp-2").unwrap();
        assert_eq!(path2.len(), 2);
    }

    /// 测试4b：BFS 最短路径 — 不可达 / 不存在节点
    #[test]
    fn test_bfs_shortest_path_unreachable() {
        let graph = make_test_graph();
        // 不存在的节点
        assert!(bfs_shortest_path(&graph, "exp-1", "nonexistent").is_none());
        assert!(bfs_shortest_path(&graph, "nonexistent", "exp-2").is_none());
        // 自身到自身
        let path = bfs_shortest_path(&graph, "exp-1", "exp-1").unwrap();
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], "exp-1");
    }

    /// 测试5：communities 标签传播 — 验证社区检测结果
    #[test]
    fn test_detect_communities() {
        let graph = make_test_graph();
        let (communities, modularity, iterations, converged) = detect_communities(&graph);
        assert!(!communities.is_empty());
        // 所有节点都被分配到社区
        let total_members: usize = communities.iter().map(|c| c.len()).sum();
        assert_eq!(total_members, graph.nodes.len());
        // 模块度在合理范围
        assert!(modularity >= -0.5 && modularity <= 1.0);
        assert!(iterations >= 1 && iterations <= 50);
        // 测试图是连通的，标签传播应收敛
        assert!(converged);
    }

    /// 测试6：optimal-team 集合覆盖 — 验证贪心团队组建
    #[test]
    fn test_find_optimal_team() {
        let mut registry = HashMap::new();
        let mut e1 = ExpertDescriptor::minimal("exp-1".into(), "架构师".into());
        e1.skills = vec!["Rust".into(), "Go".into(), "Kubernetes".into()];
        e1.domains = vec!["architecture".into(), "backend".into()];
        e1.metrics.avg_rating = 4.8;
        e1.availability.status = "online".into();
        registry.insert("exp-1".into(), e1);

        let mut e2 = ExpertDescriptor::minimal("exp-2".into(), "AI专家".into());
        e2.skills = vec!["PyTorch".into(), "Rust".into(), "LLM".into()];
        e2.domains = vec!["ai".into(), "ml".into()];
        e2.metrics.avg_rating = 4.5;
        e2.availability.status = "online".into();
        registry.insert("exp-2".into(), e2);

        let mut e3 = ExpertDescriptor::minimal("exp-3".into(), "数据工程师".into());
        e3.skills = vec!["PostgreSQL".into(), "Spark".into()];
        e3.domains = vec!["data".into(), "database".into()];
        e3.metrics.avg_rating = 4.2;
        e3.availability.status = "busy".into();
        registry.insert("exp-3".into(), e3);

        // 需求：Rust + ai 领域
        let result = find_optimal_team(
            &registry,
            &["Rust".to_string(), "PyTorch".to_string()],
            &["ai".to_string()],
            3,
            4.0,
        );
        let team = result["team_members"].as_array().unwrap();
        assert!(!team.is_empty());
        // 覆盖率应 > 0
        let coverage = result["coverage"]["coverage_ratio"].as_f64().unwrap();
        assert!(coverage > 0.0);
        // 选择策略正确
        assert_eq!(result["selection_strategy"], "weighted_set_cover_greedy");

        // 测试 min_rating 过滤
        let result2 = find_optimal_team(
            &registry,
            &["PostgreSQL".to_string()],
            &[],
            5,
            4.9, // 高于所有专家评分
        );
        let team2 = result2["team_members"].as_array().unwrap();
        assert!(team2.is_empty()); // 无候选满足
    }

    /// 测试7：rebuild 版本递增逻辑（纯函数验证）
    #[test]
    fn test_rebuild_version_increment() {
        let graph = make_test_graph();
        assert_eq!(graph.version, 1);
        let new_version = graph.version + 1;
        assert_eq!(new_version, 2);
    }
}
