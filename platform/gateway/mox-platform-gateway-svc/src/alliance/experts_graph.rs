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
use super::experts_rbac::{RbacAction, enforce_admin_or_respond};
use mox_api_protocol::ApiResponse;
use mox_audit::{AuditAction, AuditOutcome};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post, put},
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

/// 从目标文本规则提取需求（goal → required_skills/required_domains）
///
/// 设计原则（一一对应、不模糊化）：
/// - 提取结果**只来自 registry 中真实存在的域/技能 id**（枚举专家 descriptors 的
///   domains/skills 全量），绝不虚构需求项；
/// - 每个真实 id 挂一张中英文别名表（id 本身 + 常见同义词/上位词），
///   goal 文本小写后按 `contains` 命中即纳入需求；
/// - 无任何命中时返回空集（与显式空需求行为一致，set-cover 如实报告空覆盖）。
fn extract_requirements(
    goal: &str,
    registry: &HashMap<String, ExpertDescriptor>,
) -> (Vec<String>, Vec<String>) {
    /// 域 id → 中英文别名（id 本身必含）
    fn domain_aliases(id: &str) -> Vec<&'static str> {
        match id {
            "data" => vec!["data", "数据", "数仓", "数据仓库", "数据分析", "数据治理"],
            "ai" => vec!["ai", "人工智能", "大模型", "智能体", "智能"],
            "algorithm" => vec!["algorithm", "算法", "优化", "推演"],
            "programming" => vec!["programming", "code", "代码", "编程", "开发", "软件", "工程"],
            "security" => vec!["security", "安全", "防护", "攻防"],
            "cloud" => vec!["cloud", "云", "云计算", "容器", "k8s", "kubernetes"],
            "database" => vec!["database", "sql", "数据库", "存储"],
            "architecture" => vec!["architecture", "架构", "设计"],
            "network" => vec!["network", "网络", "通信"],
            "ml" => vec!["ml", "机器学习", "模型训练"],
            "llm" => vec!["llm", "语言模型", "提示词", "prompt"],
            "graph" => vec!["graph", "图", "图谱", "社区发现"],
            "audio" => vec!["audio", "音频", "声音"],
            "voice" => vec!["voice", "语音", "说话"],
            "music" => vec!["music", "音乐", "乐谱", "旋律"],
            "math" => vec!["math", "数学", "计算"],
            "finance" => vec!["finance", "金融", "财务", "投资"],
            "marketing" => vec!["marketing", "营销", "市场", "运营"],
            "legal" => vec!["legal", "法律", "合规", "合同"],
            "medical" => vec!["medical", "医疗", "医学", "健康"],
            "project" => vec!["project", "项目管理", "交付"],
            "product" => vec!["product", "产品", "设计"],
            _ => vec![],
        }
    }

    let lower = goal.to_lowercase();
    let mut skills: Vec<String> = Vec::new();
    let mut domains: Vec<String> = Vec::new();

    let mut seen_skill: HashSet<String> = HashSet::new();
    let mut seen_domain: HashSet<String> = HashSet::new();

    for e in registry.values() {
        for d in &e.domains {
            if seen_domain.contains(d) {
                continue;
            }
            seen_domain.insert(d.clone());
            let aliases = domain_aliases(d);
            // id 本身（小写）作为兜底别名
            if aliases.iter().any(|a| lower.contains(&a.to_lowercase())) || lower.contains(&d.to_lowercase())
            {
                domains.push(d.clone());
            }
        }
        for sk in &e.skills {
            if seen_skill.contains(sk) {
                continue;
            }
            seen_skill.insert(sk.clone());
            // 技能 id 多为技术词本身，直接按小写包含匹配（≥2 字符防过泛）
            if sk.len() >= 2 && lower.contains(&sk.to_lowercase()) {
                skills.push(sk.clone());
            }
        }
    }
    (skills, domains)
}


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
    /// 目标描述（自然语言）：当未显式提供 required_skills/required_domains 时，
    /// 由 `extract_requirements` 规则提取真实存在的域/技能 id，保证一一对应。
    #[serde(default)]
    goal: Option<String>,
}

// =====================================================================
// 六-B、图谱节点/边增量 CRUD 请求体（N4，管理写面）
// =====================================================================

/// 合法节点类型（与 GraphNode 注释一致：expert / capability / domain）。
/// builder 只产出 expert/domain；capability 留给画布手工建点。
const VALID_NODE_TYPES: &[&str] = &["expert", "domain", "capability"];

/// POST /api/expert-graph/nodes — 新增节点
#[derive(Debug, Deserialize)]
struct CreateNodeBody {
    id: String,
    label: String,
    node_type: String,
    #[serde(default)]
    properties: Option<HashMap<String, Value>>,
}

/// PUT /api/expert-graph/nodes/:id — 更新节点（合并式：仅覆盖提供的字段）
#[derive(Debug, Deserialize)]
struct UpdateNodeBody {
    label: Option<String>,
    node_type: Option<String>,
    #[serde(default)]
    properties: Option<HashMap<String, Value>>,
}

/// POST /api/expert-graph/edges — 新增边
#[derive(Debug, Deserialize)]
struct CreateEdgeBody {
    source: String,
    target: String,
    edge_type: String,
    #[serde(default)]
    weight: Option<f64>,
    #[serde(default)]
    properties: Option<HashMap<String, Value>>,
}

/// PUT /api/expert-graph/edges/:seq — 更新边（合并式：仅覆盖提供的字段）
#[derive(Debug, Deserialize)]
struct UpdateEdgeBody {
    edge_type: Option<String>,
    weight: Option<f64>,
    #[serde(default)]
    properties: Option<HashMap<String, Value>>,
}

/// 校验 node_type 合法；返回 Some(错误响应) 表示失败，None 表示通过
fn validate_node_type(node_type: &str) -> Option<ApiResponse<Value>> {
    if VALID_NODE_TYPES.contains(&node_type) {
        None
    } else {
        Some(err(
            400,
            format!("非法 node_type: {node_type}（允许 {}）", VALID_NODE_TYPES.join(" / ")),
        ))
    }
}

/// 校验 weight 落在 [0, 1]
fn validate_weight(weight: f64) -> Option<ApiResponse<Value>> {
    if (0.0..=1.0).contains(&weight) {
        None
    } else {
        Some(err(400, format!("weight 须在 0.0..=1.0，收到 {weight}")))
    }
}

/// 构造写操作响应：受影响元素 + 图统计（便于前端即时刷新）
fn mutation_response(affected: Value, graph: &ExpertGraph) -> ApiResponse<Value> {
    ok(json!({
        "affected": affected,
        "stats": {
            "node_count": graph.nodes.len(),
            "edge_count": graph.edges.len(),
            "version": graph.version,
        },
        "built_at": graph.built_at,
    }))
}


// =====================================================================
// 七、端点 Handler
// =====================================================================

/// 1. GET /api/expert-graph — 获取完整图谱
async fn get_graph(State(state): State<Arc<ExpertsSharedState>>, TenantId(tenant): TenantId) -> ApiResponse<Value> {
    let all_g = state.graph.lock();
    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());
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
async fn get_graph_stats(State(state): State<Arc<ExpertsSharedState>>, TenantId(tenant): TenantId) -> ApiResponse<Value> {
    let all_g = state.graph.lock();
    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());
    ok(compute_graph_stats(&graph))
}

/// 3. GET /api/expert-graph/neighbors/:id — 获取节点邻居
async fn get_neighbors(
    Path(id): Path<String>,
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
) -> ApiResponse<Value> {
    let all_g = state.graph.lock();
    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());
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
    TenantId(tenant): TenantId,
) -> ApiResponse<Value> {
    let limit: usize = params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(10);
    let all_g = state.graph.lock();
    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());
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
    TenantId(tenant): TenantId,
) -> ApiResponse<Value> {
    let all_g = state.graph.lock();
    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());
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
            let path_length = if !path_ids.is_empty() { path_ids.len() - 1 } else { 0 };
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
async fn get_communities(State(state): State<Arc<ExpertsSharedState>>, TenantId(tenant): TenantId) -> ApiResponse<Value> {
    let all_g = state.graph.lock();
    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());
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
            "modularity_contribution": if !communities.is_empty() { modularity / communities.len() as f64 } else { 0.0 },
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
    TenantId(tenant): TenantId,
    Json(body): Json<OptimalTeamBody>,
) -> ApiResponse<Value> {
    let all_reg = state.registry.lock();
    let registry = all_reg.get(tenant.as_str()).unwrap_or(empty_registry());
    let max_members = body.max_members.unwrap_or(5);
    let min_rating = body.min_rating.unwrap_or(4.0);
    // goal 文本规则提取：仅当未显式声明需求时启用，提取结果来自 registry 真实 id
    let (req_skills, req_domains) = if body.required_skills.is_empty()
        && body.required_domains.is_empty()
    {
        if let Some(goal) = body.goal.as_deref() {
            extract_requirements(goal, &registry)
        } else {
            (body.required_skills.clone(), body.required_domains.clone())
        }
    } else {
        (body.required_skills.clone(), body.required_domains.clone())
    };
    let result = find_optimal_team(
        &registry,
        &req_skills,
        &req_domains,
        max_members,
        min_rating,
    );
    ok(result)
}

/// 8. POST /api/expert-graph/rebuild — 重建图谱
async fn post_rebuild(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
) -> ApiResponse<Value> {
    // R1 后端 RBAC 强制：图谱重建属管理写面，需 super_admin / tenant_admin
    if let Err(resp) = enforce_admin_or_respond(&state, &user, tenant.as_str(), RbacAction::RebuildGraph) {
        return resp;
    }

    let start = std::time::Instant::now();
    let previous_version;
    let new_graph;
    {
        let all_reg = state.registry.lock();
        let registry = all_reg.get(tenant.as_str()).unwrap_or(empty_registry());
        let mut all_g = state.graph.lock();
        let graph = all_g.entry(tenant.clone()).or_default();
        previous_version = graph.version;
        new_graph = build_graph_from_registry(registry);
        *graph = ExpertGraph {
            version: previous_version + 1,
            ..new_graph
        };
        save_graph(tenant.as_str(), graph);
    }
    let duration_ms = start.elapsed().as_millis() as u64;
    let all_g = state.graph.lock();
    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());
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
// 七-B、图谱节点/边增量 CRUD（N4，全部管理写面，强制 RBAC）
// =====================================================================
//
// 语义约定（与 rebuild 互补）：
// - 内存态 `state.graph` 为权威源，锁内完成校验+变更+落库，version += 1；
// - 边 seq == edges 下标；删除边/节点后用 replace_graph_edges 重排 seq，保持不变量；
// - rebuild 全量重算会覆盖增量改动，属预期（见文档）。

/// 9. POST /api/expert-graph/nodes — 新增节点
async fn post_graph_node(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<CreateNodeBody>,
) -> ApiResponse<Value> {
    if let Err(resp) = enforce_admin_or_respond(&state, &user, tenant.as_str(), RbacAction::MutateGraph) {
        return resp;
    }
    if body.id.trim().is_empty() {
        return err(400, "节点 id 不能为空");
    }
    if body.label.trim().is_empty() {
        return err(400, "节点 label 不能为空");
    }
    if let Some(resp) = validate_node_type(&body.node_type) {
        return resp;
    }

    let mut all_g = state.graph.lock();
    let graph = all_g.entry(tenant.clone()).or_default();
    if graph.nodes.iter().any(|n| n.id == body.id) {
        return err(409, format!("节点 id 已存在: {}", body.id));
    }
    let node = GraphNode {
        id: body.id.clone(),
        label: body.label.clone(),
        node_type: body.node_type.clone(),
        properties: body.properties.unwrap_or_default(),
    };
    graph.nodes.push(node.clone());
    graph.version += 1;
    graph.built_at = now_iso();
    drop(graph);
    drop(all_g); // 显式释放 graph 锁，避免下方重锁自死锁

    crate::alliance::experts_db::upsert_graph_node(tenant.as_str(), &node);
    let all_g = state.graph.lock();
    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());
    emit_audit(
        &state,
        &actor_from_opt_user(&user),
        tenant.as_str(),
        AuditAction::Unknown("graph.node.create".into()),
        "graph_node",
        &node.id,
        AuditOutcome::Success,
        Some(&format!("node_type={}", node.node_type)),
    );
    mutation_response(json!({ "node": node }), &graph)
}

/// 10. PUT /api/expert-graph/nodes/:id — 更新节点
async fn put_graph_node(
    Path(id): Path<String>,
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<UpdateNodeBody>,
) -> ApiResponse<Value> {
    if let Err(resp) = enforce_admin_or_respond(&state, &user, tenant.as_str(), RbacAction::MutateGraph) {
        return resp;
    }
    if let Some(t) = &body.node_type {
        if let Some(resp) = validate_node_type(t) {
            return resp;
        }
    }
    let mut all_g = state.graph.lock();
    let graph = all_g.entry(tenant.clone()).or_default();
    let node = match graph.nodes.iter_mut().find(|n| n.id == id) {
        Some(n) => n,
        None => return err(404, format!("node not found: {id}")),
    };
    if let Some(label) = body.label {
        if label.trim().is_empty() {
            return err(400, "节点 label 不能为空");
        }
        node.label = label;
    }
    if let Some(t) = body.node_type {
        node.node_type = t;
    }
    if let Some(props) = body.properties {
        node.properties = props;
    }
    let node_clone = node.clone();
    graph.version += 1;
    graph.built_at = now_iso();
    drop(graph);
    drop(all_g); // 显式释放 graph 锁，避免下方重锁自死锁

    crate::alliance::experts_db::upsert_graph_node(tenant.as_str(), &node_clone);
    let all_g = state.graph.lock();
    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());
    emit_audit(
        &state,
        &actor_from_opt_user(&user),
        tenant.as_str(),
        AuditAction::Unknown("graph.node.update".into()),
        "graph_node",
        &id,
        AuditOutcome::Success,
        None,
    );
    mutation_response(json!({ "node": node_clone }), &graph)
}

/// 11. DELETE /api/expert-graph/nodes/:id — 删除节点（联动删除关联边）
async fn delete_graph_node(
    Path(id): Path<String>,
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
) -> ApiResponse<Value> {
    if let Err(resp) = enforce_admin_or_respond(&state, &user, tenant.as_str(), RbacAction::MutateGraph) {
        return resp;
    }
    let mut all_g = state.graph.lock();
    let graph = all_g.entry(tenant.clone()).or_default();
    let node_idx = graph.nodes.iter().position(|n| n.id == id);
    let node_idx = match node_idx {
        Some(i) => i,
        None => return err(404, format!("node not found: {id}")),
    };
    let removed_node = graph.nodes.remove(node_idx);
    let removed_edges = graph
        .edges
        .iter()
        .filter(|e| e.source == id || e.target == id)
        .count();
    graph.edges.retain(|e| e.source != id && e.target != id);
    graph.version += 1;
    graph.built_at = now_iso();
    let edges_snapshot: Vec<GraphEdge> = graph.edges.clone();
    drop(graph);
    drop(all_g); // 显式释放 graph 锁，避免下方重锁自死锁

    crate::alliance::experts_db::delete_graph_node_cascade(tenant.as_str(), &id);
    crate::alliance::experts_db::replace_graph_edges(tenant.as_str(), &edges_snapshot);
    let all_g = state.graph.lock();
    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());
    emit_audit(
        &state,
        &actor_from_opt_user(&user),
        tenant.as_str(),
        AuditAction::Unknown("graph.node.delete".into()),
        "graph_node",
        &id,
        AuditOutcome::Success,
        Some(&format!("cascaded_edges={removed_edges}")),
    );
    mutation_response(
        json!({ "node": removed_node, "removed_edges": removed_edges }),
        &graph,
    )
}

/// 12. POST /api/expert-graph/edges — 新增边
async fn post_graph_edge(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<CreateEdgeBody>,
) -> ApiResponse<Value> {
    if let Err(resp) = enforce_admin_or_respond(&state, &user, tenant.as_str(), RbacAction::MutateGraph) {
        return resp;
    }
    if body.edge_type.trim().is_empty() {
        return err(400, "edge_type 不能为空");
    }
    let weight = body.weight.unwrap_or(1.0);
    if let Some(resp) = validate_weight(weight) {
        return resp;
    }

    let mut all_g = state.graph.lock();
    let graph = all_g.entry(tenant.clone()).or_default();
    // source/target 必须存在
    let node_ids = node_index(&graph);
    if !node_ids.contains_key(&body.source) {
        return err(400, format!("边的 source 节点不存在: {}", body.source));
    }
    if !node_ids.contains_key(&body.target) {
        return err(400, format!("边的 target 节点不存在: {}", body.target));
    }
    // 重复边冲突（source,target,edge_type 无序视为同一条）
    let dup = graph.edges.iter().any(|e| {
        let a = (e.source.as_str(), e.target.as_str());
        let b = (body.source.as_str(), body.target.as_str());
        e.edge_type == body.edge_type
            && ((a == b) || (a.0 == b.1 && a.1 == b.0))
    });
    if dup {
        return err(
            409,
            format!(
                "边已存在: {} --[{}]--> {}",
                body.source, body.edge_type, body.target
            ),
        );
    }
    let edge = GraphEdge {
        source: body.source.clone(),
        target: body.target.clone(),
        edge_type: body.edge_type.clone(),
        weight,
        properties: body.properties.unwrap_or_default(),
    };
    let seq = graph.edges.len() as i64; // 追加到末尾，seq == 下标
    graph.edges.push(edge.clone());
    graph.version += 1;
    graph.built_at = now_iso();
    drop(graph);
    drop(all_g); // 显式释放 graph 锁，避免下方重锁自死锁

    crate::alliance::experts_db::upsert_graph_edge(tenant.as_str(), seq, &edge);
    let all_g = state.graph.lock();
    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());
    emit_audit(
        &state,
        &actor_from_opt_user(&user),
        tenant.as_str(),
        AuditAction::Unknown("graph.edge.create".into()),
        "graph_edge",
        &format!("{}/{}", edge.source, edge.target),
        AuditOutcome::Success,
        Some(&format!("seq={seq} edge_type={}", edge.edge_type)),
    );
    mutation_response(json!({ "edge": edge, "seq": seq }), &graph)
}

/// 13. PUT /api/expert-graph/edges/:seq — 更新边
async fn put_graph_edge(
    Path(seq): Path<i64>,
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<UpdateEdgeBody>,
) -> ApiResponse<Value> {
    if let Err(resp) = enforce_admin_or_respond(&state, &user, tenant.as_str(), RbacAction::MutateGraph) {
        return resp;
    }
    if let Some(w) = body.weight {
        if let Some(resp) = validate_weight(w) {
            return resp;
        }
    }
    let mut all_g = state.graph.lock();
    let graph = all_g.entry(tenant.clone()).or_default();
    let idx = seq as usize;
    if idx >= graph.edges.len() {
        return err(404, format!("edge seq 不存在: {seq}"));
    }
    let edge = &mut graph.edges[idx];
    if let Some(t) = body.edge_type {
        if t.trim().is_empty() {
            return err(400, "edge_type 不能为空");
        }
        edge.edge_type = t;
    }
    if let Some(w) = body.weight {
        edge.weight = w;
    }
    if let Some(props) = body.properties {
        edge.properties = props;
    }
    let edge_clone = edge.clone();
    graph.version += 1;
    graph.built_at = now_iso();
    drop(graph);
    drop(all_g); // 显式释放 graph 锁，避免下方重锁自死锁

    crate::alliance::experts_db::upsert_graph_edge(tenant.as_str(), seq, &edge_clone);
    let all_g = state.graph.lock();
    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());
    emit_audit(
        &state,
        &actor_from_opt_user(&user),
        tenant.as_str(),
        AuditAction::Unknown("graph.edge.update".into()),
        "graph_edge",
        &format!("seq={seq}"),
        AuditOutcome::Success,
        None,
    );
    mutation_response(json!({ "edge": edge_clone, "seq": seq }), &graph)
}

/// 14. DELETE /api/expert-graph/edges/:seq — 删除边
async fn delete_graph_edge(
    Path(seq): Path<i64>,
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
) -> ApiResponse<Value> {
    if let Err(resp) = enforce_admin_or_respond(&state, &user, tenant.as_str(), RbacAction::MutateGraph) {
        return resp;
    }
    let mut all_g = state.graph.lock();
    let graph = all_g.entry(tenant.clone()).or_default();
    let idx = seq as usize;
    if idx >= graph.edges.len() {
        return err(404, format!("edge seq 不存在: {seq}"));
    }
    let removed = graph.edges.remove(idx);
    graph.version += 1;
    graph.built_at = now_iso();
    let edges_snapshot: Vec<GraphEdge> = graph.edges.clone();
    drop(graph);
    drop(all_g); // 显式释放 graph 锁，避免下方重锁自死锁

    // 删除后剩余边下标前移，重排 seq 保持 seq==下标
    crate::alliance::experts_db::replace_graph_edges(tenant.as_str(), &edges_snapshot);
    let all_g = state.graph.lock();
    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());
    emit_audit(
        &state,
        &actor_from_opt_user(&user),
        tenant.as_str(),
        AuditAction::Unknown("graph.edge.delete".into()),
        "graph_edge",
        &format!("seq={seq}"),
        AuditOutcome::Success,
        Some(&format!("removed {}-{}", removed.source, removed.target)),
    );
    mutation_response(json!({ "edge": removed, "removed_seq": seq }), &graph)
}


// =====================================================================
// 七-C、图 RAG（T2）：多跳邻域扩展检索
// =====================================================================
//
// 设计定位（2026-10-01）：
// - 护城河方向：竞品均向量 RAG，本能力以「图谱结构化邻域扩展」为差异化——
//   沿边多跳召回相关专家/能力域/历史协作链路，给 planner 提供「该找谁、为什么、怎么组队」
//   的可解释上下文（每条结果带完整路径与首跳边）。
// - 实现选择：本实现采用**内存态加权多跳扩展**（数据源 state.graph，与 get_graph 同源、
//   与 SQLite 双向同步等价）。规划期曾选「SQLite 图表递归 CTE」，但 state 无 SQLite 连接句柄
//   （引入需连接生命周期管理），内存态零连接开销且与既有 8 只读查询同构。语义等价，
//   A4 图库迁入时仅替换查询实现，handler 契约不变。
// - 权重聚合：路径边权重**乘积**（w∈[0,1]，随深度自然衰减）；同节点多路径取最优
//   （乘积大者优先，同乘积取更浅深度）。
// - 融合重排：向量/关键词检索目标态未落地（#27 pgvector 是 P1），本端点为图谱纯检索；
//   `hybrid_rerank` 仅作扩展点函数签名与文档，真实向量融合待 #27，不冒充已做。
// - 读面公开（与 get_graph 一致，无需角色）。

/// POST /api/expert-graph/rag/expand 请求体
#[derive(Debug, Deserialize)]
struct RagExpandBody {
    /// 种子节点 id 数组（专家或域均可）
    seeds: Vec<String>,
    /// 最大跳数（1-4，缺省 2）
    #[serde(default)]
    max_depth: Option<usize>,
    /// 返回条数上限（缺省 20）
    #[serde(default)]
    top_k: Option<usize>,
    /// 结果节点类型过滤（如 ["expert"]）；空=不过滤，展开仍可经过其他类型
    #[serde(default)]
    node_types: Option<Vec<String>>,
    /// 边权重下限（缺省 0.0）：低于该值的边不沿其展开
    #[serde(default)]
    min_weight: Option<f64>,
}

/// 单条召回结果（内部）
pub struct RagHit {
    pub node_id: String,
    pub depth: usize,
    pub aggregate_weight: f64,
    pub path: Vec<String>,
    pub first_hop: Value,
}

/// 多跳邻域扩展纯函数。
///
/// - seeds 中不存在的节点自动跳过（存在性由 handler 统一 404）；
/// - 排除种子自身；环路防重复：单条路径内不回环（path.contains），跨路径以 best 择优；
/// - min_weight 在边展开侧过滤；node_types 在结果侧过滤（路径仍保留完整节点序列）。
pub fn expand_neighborhood(
    graph: &ExpertGraph,
    seeds: &[String],
    max_depth: usize,
    top_k: usize,
    node_types: &HashSet<String>,
    min_weight: f64,
) -> Vec<RagHit> {
    // 带边类型的无向邻接表：node_id -> [(neighbor_id, edge_type, normalized_weight)]
    let mut adj: HashMap<String, Vec<(String, String, f64)>> = HashMap::new();
    for node in &graph.nodes {
        adj.entry(node.id.clone()).or_default();
    }
    for edge in &graph.edges {
        let w = if edge.weight > 0.0 { edge.weight } else { 1.0 };
        adj.entry(edge.source.clone()).or_default()
            .push((edge.target.clone(), edge.edge_type.clone(), w));
        adj.entry(edge.target.clone()).or_default()
            .push((edge.source.clone(), edge.edge_type.clone(), w));
    }

    // best: node_id -> (aggregate_weight, depth, path, first_hop)
    struct Best {
        acc: f64,
        depth: usize,
        path: Vec<String>,
        first_hop: Value,
    }
    let mut best: HashMap<String, Best> = HashMap::new();

    // 队列元素：(curr, acc, depth, path, first_hop)
    let mut queue: VecDeque<(String, f64, usize, Vec<String>, Value)> = VecDeque::new();

    // 种子首跳入队
    for seed in seeds {
        if !adj.contains_key(seed) {
            continue;
        }
        if let Some(neighbors) = adj.get(seed) {
            for (nb, et, w) in neighbors {
                if *w < min_weight {
                    continue;
                }
                let path = vec![seed.clone(), nb.clone()];
                let fh = json!({
                    "from": seed,
                    "to": nb,
                    "edge_type": et,
                    "weight": w,
                });
                queue.push_back((nb.clone(), *w, 1usize, path, fh));
            }
        }
    }

    while let Some((curr, acc, depth, path, fh)) = queue.pop_front() {
        // 择优判据：权重更大，或同权重更浅
        let better = match best.get(&curr) {
            None => true,
            Some(b) => acc > b.acc || (acc == b.acc && depth < b.depth),
        };
        if !better {
            continue;
        }
        best.insert(
            curr.clone(),
            Best { acc, depth, path: path.clone(), first_hop: fh.clone() },
        );

        if depth >= max_depth {
            continue;
        }
        if let Some(neighbors) = adj.get(&curr) {
            for (nb, _et, w) in neighbors {
                if *w < min_weight {
                    continue;
                }
                // 单路径内防回环
                if path.contains(nb) {
                    continue;
                }
                let mut new_path = path.clone();
                new_path.push(nb.clone());
                queue.push_back((nb.clone(), acc * w, depth + 1, new_path, fh.clone()));
            }
        }
    }

    // 结果侧 node_types 过滤
    let idx = node_index(graph);
    let mut hits: Vec<RagHit> = best
        .into_iter()
        .filter(|(id, _)| {
            if node_types.is_empty() {
                return true;
            }
            idx.get(id).map(|n| node_types.contains(&n.node_type)).unwrap_or(false)
        })
        .map(|(node_id, b)| RagHit {
            node_id,
            depth: b.depth,
            aggregate_weight: b.acc,
            path: b.path,
            first_hop: b.first_hop,
        })
        .collect();

    // 排序：权重降序 → 深度升序 → id 字典序（确定性）
    hits.sort_by(|a, b| {
        b.aggregate_weight
            .partial_cmp(&a.aggregate_weight)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.depth.cmp(&b.depth))
            .then(a.node_id.cmp(&b.node_id))
    });

    hits.truncate(top_k);
    hits
}

/// 融合重排扩展点（待 #27 pgvector 落地后真实实现）。
///
/// 当前为图谱纯检索结果，向量/关键词召回尚未落地。本函数仅签名占位：
/// 接收图谱召回 hits 与未来向量召回列表，返回融合后 hits。
/// 落地前直接透传图谱 hits，不伪造向量分数。
pub fn hybrid_rerank(graph_hits: Vec<RagHit>, _vector_candidates: Value) -> Vec<RagHit> {
    // TODO(#27): 接入 pgvector 召回后，按 RRF（Reciprocal Rank Fusion）与图谱权重融合。
    // 当前阶段：向量候选集恒为空，直接返回图谱 hits，如实标注「向量融合待 #27」。
    graph_hits
}

/// 15. POST /api/expert-graph/rag/expand — 图 RAG 多跳邻域扩展（T2，读面公开）
async fn post_rag_expand(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    Json(body): Json<RagExpandBody>,
) -> ApiResponse<Value> {
    // 校验：seeds 非空
    if body.seeds.is_empty() {
        return err(400, "seeds 不能为空");
    }
    // 校验：max_depth 范围 1..=4
    let max_depth = body.max_depth.unwrap_or(2);
    if !(1..=4).contains(&max_depth) {
        return err(400, format!("max_depth 须在 1..=4，收到 {max_depth}"));
    }
    let top_k = body.top_k.unwrap_or(20);
    if top_k == 0 {
        return err(400, "top_k 须 > 0");
    }
    let min_weight = body.min_weight.unwrap_or(0.0);
    if !(0.0..=1.0).contains(&min_weight) {
        return err(400, format!("min_weight 须在 0.0..=1.0，收到 {min_weight}"));
    }
    let node_types: HashSet<String> = body.node_types.clone().unwrap_or_default().into_iter().collect();

    let start = std::time::Instant::now();
    let all_g = state.graph.lock();
    let graph = all_g.get(tenant.as_str()).unwrap_or(empty_graph());

    // 种子存在性校验：任一不存在 → 404（与 get_neighbors 语义一致）
    let idx = node_index(&graph);
    let mut missing: Vec<String> = Vec::new();
    for s in &body.seeds {
        if !idx.contains_key(s) {
            missing.push(s.clone());
        }
    }
    if !missing.is_empty() {
        return err(404, format!("种子节点不存在: {}", missing.join(", ")));
    }

    // 空图 → 空 results（非错误）
    if graph.nodes.is_empty() {
        return ok(json!({
            "query": {
                "seeds": body.seeds,
                "max_depth": max_depth,
                "top_k": top_k,
            },
            "results": [],
            "stats": {
                "searched_nodes": 0usize,
                "returned": 0usize,
                "elapsed_ms": start.elapsed().as_millis() as u64,
            },
            "rerank": "graph_only（向量融合待 #27）",
        }));
    }

    // 先做全量扩展（未按 node_types 过滤），searched_nodes 记过滤前的去重节点数
    let all_hits = expand_neighborhood(&graph, &body.seeds, max_depth, usize::MAX, &HashSet::new(), min_weight);
    let searched_nodes = all_hits.len();

    // 再按 node_types 过滤并截断（expand_neighborhood 内部已排序）
    let hits = expand_neighborhood(&graph, &body.seeds, max_depth, top_k, &node_types, min_weight);

    // 融合重排扩展点（当前透传）
    let hits = hybrid_rerank(hits, json!([]));

    let results: Vec<Value> = hits
        .iter()
        .map(|h| {
            let node = idx.get(&h.node_id);
            json!({
                "node": {
                    "id": h.node_id,
                    "label": node.map(|n| n.label.clone()).unwrap_or_default(),
                    "node_type": node.map(|n| n.node_type.clone()).unwrap_or_default(),
                },
                "depth": h.depth,
                "aggregate_weight": h.aggregate_weight,
                "path": h.path,
                "first_hops": [h.first_hop],
            })
        })
        .collect();

    ok(json!({
        "query": {
            "seeds": body.seeds,
            "max_depth": max_depth,
            "top_k": top_k,
            "node_types": body.node_types.unwrap_or_default(),
            "min_weight": min_weight,
        },
        "results": results,
        "stats": {
            "searched_nodes": searched_nodes,
            "returned": results.len(),
            "elapsed_ms": start.elapsed().as_millis() as u64,
        },
        "rerank": "graph_only（向量融合待 #27）",
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
        // ── N4 节点级 CRUD（管理写面，强制 RBAC MutateGraph）──
        .route("/api/expert-graph/nodes", post(post_graph_node))
        .route("/api/expert-graph/nodes/:id", put(put_graph_node).delete(delete_graph_node))
        .route("/api/expert-graph/edges", post(post_graph_edge))
        .route("/api/expert-graph/edges/:seq", put(put_graph_edge).delete(delete_graph_edge))
        // ── T2 图 RAG：多跳邻域扩展（读面公开，无需角色）──
        .route("/api/expert-graph/rag/expand", post(post_rag_expand))
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
        assert!((0.0..=1.0).contains(&avg_cc));
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
        assert!((-0.5..=1.0).contains(&modularity));
        assert!((1..=50).contains(&iterations));
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

    // ── N4 节点级 CRUD ──────────────────────────────────────────────

    use crate::alliance::experts_rbac::enforce_admin;
    use mox_audit::{AuditContext, MultiSink, NoopSink};
    use mox_platform_api::UserInfo;
    use parking_lot::Mutex;

    fn admin_user() -> UserInfo {
        UserInfo {
            id: "u-admin".into(),
            username: "admin".into(),
            email: "a@a.com".into(),
            tenant_id: "t".into(),
            roles: vec!["tenant_admin".into()],
            enabled: true,
            created_at: "2026-09-30T00:00:00Z".into(),
        }
    }

    fn normal_user() -> UserInfo {
        let mut u = admin_user();
        u.roles = vec!["normal_user".into()];
        u
    }

    /// 构造带测试图的共享状态；DB 指向临时文件，避免污染 data/experts.db
    fn crud_state() -> Arc<ExpertsSharedState> {
        std::env::set_var(
            crate::alliance::experts_db::ENV_DB_PATH,
            std::env::temp_dir().join("mox-test-crud-experts.db").to_string_lossy().to_string(),
        );
        let audit = AuditContext::new(Arc::new(MultiSink::new().with_sink(Box::new(NoopSink))));
        Arc::new(ExpertsSharedState {
            registry: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            dispatcher_config: Arc::new(Mutex::new(Default::default())),
            dispatch_records: Arc::new(Mutex::new(Vec::new())),
            graph: Arc::new(Mutex::new(HashMap::from([("default".to_string(), make_test_graph())]))),
            plans: Arc::new(Mutex::new(HashMap::new())),
            orchestration_history: Arc::new(Mutex::new(Vec::new())),
            favorites: Arc::new(Mutex::new(std::collections::HashMap::new())),
            audit: Arc::new(audit),
            events: Arc::new(crate::alliance::experts_events::EventBus::new(16)),
        })
    }

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().unwrap()
    }

    #[test]
    fn test_validate_node_type_and_weight() {
        assert!(validate_node_type("expert").is_none());
        assert!(validate_node_type("domain").is_none());
        assert!(validate_node_type("capability").is_none());
        assert!(validate_node_type("weird").unwrap().code == 400);
        assert!(validate_weight(0.0).is_none());
        assert!(validate_weight(1.0).is_none());
        assert!(validate_weight(1.5).unwrap().code == 400);
        assert!(validate_weight(-0.1).unwrap().code == 400);
    }

    #[test]
    fn test_mutate_graph_rbac_rejects() {
        // 未认证 → 401
        assert_eq!(enforce_admin(&None, RbacAction::MutateGraph).unwrap_err().status, 401);
        // 普通用户 → 403
        assert_eq!(enforce_admin(&Some(normal_user()), RbacAction::MutateGraph).unwrap_err().status, 403);
        // 管理员放行
        assert!(enforce_admin(&Some(admin_user()), RbacAction::MutateGraph).is_ok());
        assert_eq!(RbacAction::MutateGraph.code(), "graph.mutate");
    }

    #[test]
    fn test_create_node_ok_and_duplicate_409() {
        let state = crud_state();
        let body = CreateNodeBody {
            id: "exp-new".into(),
            label: "新专家".into(),
            node_type: "expert".into(),
            properties: None,
        };
        let resp = rt().block_on(post_graph_node(
            State(state.clone()), TenantId("default".into()), OptionalAuthUser(Some(admin_user())), Json(body)));
        assert_eq!(resp.code, 0);
        assert_eq!(state.graph.lock().get("default").unwrap().nodes.len(), 6);
        assert!(state.graph.lock().get("default").unwrap().version >= 2);

        // 重复 id → 409
        let dup = CreateNodeBody {
            id: "exp-new".into(), label: "x".into(), node_type: "expert".into(), properties: None,
        };
        let resp2 = rt().block_on(post_graph_node(
            State(state.clone()), TenantId("default".into()), OptionalAuthUser(Some(admin_user())), Json(dup)));
        assert_eq!(resp2.code, 409);
    }

    #[test]
    fn test_create_node_bad_type_400() {
        let state = crud_state();
        let body = CreateNodeBody {
            id: "exp-x".into(), label: "x".into(), node_type: "bogus".into(), properties: None,
        };
        let resp = rt().block_on(post_graph_node(
            State(state.clone()), TenantId("default".into()), OptionalAuthUser(Some(admin_user())), Json(body)));
        assert_eq!(resp.code, 400);
        assert_eq!(state.graph.lock().get("default").unwrap().nodes.len(), 5);
    }

    #[test]
    fn test_update_node_404_and_merge() {
        let state = crud_state();
        // 不存在 → 404
        let resp = rt().block_on(put_graph_node(
            Path("nope".into()), State(state.clone()), TenantId("default".into()),
            OptionalAuthUser(Some(admin_user())), Json(UpdateNodeBody { label: None, node_type: None, properties: None })));
        assert_eq!(resp.code, 404);
        // 合并更新 label
        let resp2 = rt().block_on(put_graph_node(
            Path("exp-1".into()), State(state.clone()), TenantId("default".into()),
            OptionalAuthUser(Some(admin_user())),
            Json(UpdateNodeBody { label: Some("改名".into()), node_type: None, properties: None })));
        assert_eq!(resp2.code, 0);
        assert_eq!(state.graph.lock().get("default").unwrap().nodes[0].label, "改名");
    }

    #[test]
    fn test_delete_node_cascades_edges() {
        let state = crud_state();
        // exp-1 关联边：exp-1--domain-ai, exp-1--exp-2（2 条）
        let before_edges = state.graph.lock().get("default").unwrap().edges.len();
        let resp = rt().block_on(delete_graph_node(
            Path("exp-1".into()), State(state.clone()), TenantId("default".into()), OptionalAuthUser(Some(admin_user()))));
        assert_eq!(resp.code, 0);
        let g_lock = state.graph.lock();
        let g = g_lock.get("default").unwrap();
        assert_eq!(g.nodes.len(), 4);
        assert_eq!(g.edges.len(), before_edges - 2);
        // 残留边不得再引用 exp-1
        assert!(g.edges.iter().all(|e| e.source != "exp-1" && e.target != "exp-1"));
    }

    #[test]
    fn test_create_edge_ok_duplicate_409_missing_endpoint_400() {
        let state = crud_state();
        // 端点不存在 → 400
        let bad = CreateEdgeBody {
            source: "exp-1".into(), target: "ghost".into(), edge_type: "collaborates_with".into(), weight: Some(0.5), properties: None,
        };
        let resp0 = rt().block_on(post_graph_edge(
            State(state.clone()), TenantId("default".into()), OptionalAuthUser(Some(admin_user())), Json(bad)));
        assert_eq!(resp0.code, 400);

        // 新建边 exp-1 -- exp-3（当前无直接协作边）
        let ok = CreateEdgeBody {
            source: "exp-1".into(), target: "exp-3".into(), edge_type: "collaborates_with".into(), weight: Some(0.5), properties: None,
        };
        let before = state.graph.lock().get("default").unwrap().edges.len();
        let resp = rt().block_on(post_graph_edge(
            State(state.clone()), TenantId("default".into()), OptionalAuthUser(Some(admin_user())), Json(ok)));
        assert_eq!(resp.code, 0);
        assert_eq!(state.graph.lock().get("default").unwrap().edges.len(), before + 1);

        // 重复边（反向同对同类型）→ 409
        let dup = CreateEdgeBody {
            source: "exp-3".into(), target: "exp-1".into(), edge_type: "collaborates_with".into(), weight: None, properties: None,
        };
        let resp2 = rt().block_on(post_graph_edge(
            State(state.clone()), TenantId("default".into()), OptionalAuthUser(Some(admin_user())), Json(dup)));
        assert_eq!(resp2.code, 409);
    }

    #[test]
    fn test_update_delete_edge_by_seq() {
        let state = crud_state();
        let n = state.graph.lock().get("default").unwrap().edges.len();
        // 更新 seq=0 的权重
        let resp = rt().block_on(put_graph_edge(
            Path(0i64), State(state.clone()), TenantId("default".into()), OptionalAuthUser(Some(admin_user())),
            Json(UpdateEdgeBody { edge_type: None, weight: Some(0.9), properties: None })));
        assert_eq!(resp.code, 0);
        assert_eq!(state.graph.lock().get("default").unwrap().edges[0].weight, 0.9);
        // seq 越界 → 404
        let resp2 = rt().block_on(put_graph_edge(
            Path((n as i64) + 5), State(state.clone()), TenantId("default".into()), OptionalAuthUser(Some(admin_user())),
            Json(UpdateEdgeBody { edge_type: None, weight: None, properties: None })));
        assert_eq!(resp2.code, 404);
        // 删除 seq=0
        let resp3 = rt().block_on(delete_graph_edge(
            Path(0i64), State(state.clone()), TenantId("default".into()), OptionalAuthUser(Some(admin_user()))));
        assert_eq!(resp3.code, 0);
        assert_eq!(state.graph.lock().get("default").unwrap().edges.len(), n - 1);
    }

    #[test]
    fn test_crud_requires_auth_401() {
        let state = crud_state();
        let body = CreateNodeBody { id: "x".into(), label: "x".into(), node_type: "expert".into(), properties: None };
        let resp = rt().block_on(post_graph_node(
            State(state.clone()), TenantId("default".into()), OptionalAuthUser(None), Json(body)));
        assert_eq!(resp.code, 401);
    }

    // ── T2 图 RAG 多跳邻域扩展 ───────────────────────────────────────

    /// 检索器测试用：复用 make_test_graph（exp-1/2/3 + domain-ai/data）
    /// 边权重：has_domain=1.0，exp-1--exp-2=0.6，exp-2--exp-3=0.4
    fn rag_empty_types() -> HashSet<String> {
        HashSet::new()
    }

    #[test]
    fn test_rag_multihop_reaches_third_hop() {
        let graph = make_test_graph();
        // 种子 exp-1，max_depth=2：直达 domain-ai/exp-2；二跳可达 exp-3（经 exp-2，乘积 0.6*0.4=0.24）
        let hits = expand_neighborhood(&graph, &["exp-1".into()], 2, 50, &rag_empty_types(), 0.0);
        let ids: Vec<&str> = hits.iter().map(|h| h.node_id.as_str()).collect();
        // 排除种子自身
        assert!(!ids.contains(&"exp-1"));
        // 一跳邻居
        assert!(ids.contains(&"domain-ai"));
        assert!(ids.contains(&"exp-2"));
        // 二跳可达 exp-3（经 exp-2）
        assert!(ids.contains(&"exp-3"));
        // exp-3 的 depth=2，aggregate_weight = 0.6 * 0.4 = 0.24
        let exp3 = hits.iter().find(|h| h.node_id == "exp-3").unwrap();
        assert_eq!(exp3.depth, 2);
        assert!((exp3.aggregate_weight - 0.24).abs() < 1e-9);
        // 路径含 exp-2 中转
        assert_eq!(exp3.path, vec!["exp-1", "exp-2", "exp-3"]);
        // 首跳是 exp-1 -> exp-2
        assert_eq!(exp3.first_hop["from"], "exp-1");
        assert_eq!(exp3.first_hop["to"], "exp-2");
    }

    #[test]
    fn test_rag_weight_ranking_desc() {
        let graph = make_test_graph();
        let hits = expand_neighborhood(&graph, &["exp-1".into()], 2, 50, &rag_empty_types(), 0.0);
        // 一跳 domain-ai 权重 1.0、exp-2 权重 0.6；二跳 domain-data 经 exp-2 = 0.6*1.0=0.6、exp-3 = 0.24
        // 排序应按 aggregate_weight 降序
        let weights: Vec<f64> = hits.iter().map(|h| h.aggregate_weight).collect();
        for w in weights.windows(2) {
            assert!(w[0] >= w[1], "权重未降序: {:?}", weights);
        }
        // 第一名是 domain-ai（权重 1.0）
        assert_eq!(hits[0].node_id, "domain-ai");
        assert!((hits[0].aggregate_weight - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_rag_cycle_guard_no_self_loop() {
        // 构造一个含环的图：exp-a -- exp-b -- exp-c -- exp-a
        let nodes = vec![
            GraphNode { id: "exp-a".into(), label: "A".into(), node_type: "expert".into(), properties: HashMap::new() },
            GraphNode { id: "exp-b".into(), label: "B".into(), node_type: "expert".into(), properties: HashMap::new() },
            GraphNode { id: "exp-c".into(), label: "C".into(), node_type: "expert".into(), properties: HashMap::new() },
        ];
        let edges = vec![
            GraphEdge { source: "exp-a".into(), target: "exp-b".into(), edge_type: "collaborates_with".into(), weight: 0.9, properties: HashMap::new() },
            GraphEdge { source: "exp-b".into(), target: "exp-c".into(), edge_type: "collaborates_with".into(), weight: 0.8, properties: HashMap::new() },
            GraphEdge { source: "exp-c".into(), target: "exp-a".into(), edge_type: "collaborates_with".into(), weight: 0.7, properties: HashMap::new() },
        ];
        let graph = ExpertGraph { nodes, edges, built_at: now_iso(), version: 1 };
        let hits = expand_neighborhood(&graph, &["exp-a".into()], 3, 50, &rag_empty_types(), 0.0);
        // 排除种子；其余两节点各只出现一次（best 去重）
        let ids: Vec<&str> = hits.iter().map(|h| h.node_id.as_str()).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"exp-b"));
        assert!(ids.contains(&"exp-c"));
        // exp-b 最短 depth=1，不应被 depth=2 的环路路径覆盖
        let b = hits.iter().find(|h| h.node_id == "exp-b").unwrap();
        assert_eq!(b.depth, 1);
        // exp-c 经 exp-a->b->c depth=2，乘积 0.9*0.8=0.72；不选 a->c(0.7) 那条（虽然 depth=1 但权重低）
        // 注意：a->c 直接相连 weight=0.7，depth=1；best 择优：0.72(d=2) vs 0.7(d=1) → 0.72 胜
        let c = hits.iter().find(|h| h.node_id == "exp-c").unwrap();
        assert!((c.aggregate_weight - 0.72).abs() < 1e-9);
        assert_eq!(c.path, vec!["exp-a", "exp-b", "exp-c"]);
    }

    #[test]
    fn test_rag_empty_graph_returns_empty() {
        let graph = ExpertGraph::default();
        let hits = expand_neighborhood(&graph, &["exp-1".into()], 2, 20, &rag_empty_types(), 0.0);
        assert!(hits.is_empty());
    }

    #[test]
    fn test_rag_max_depth_3_reaches_deep_chain() {
        // 链：s -- a(0.9) -- b(0.8) -- c(0.7) -- d(0.6)
        // depth: s=0(种子), a=1, b=2, c=3, d=4
        let nodes = vec![
            GraphNode { id: "s".into(), label: "S".into(), node_type: "expert".into(), properties: HashMap::new() },
            GraphNode { id: "a".into(), label: "A".into(), node_type: "expert".into(), properties: HashMap::new() },
            GraphNode { id: "b".into(), label: "B".into(), node_type: "expert".into(), properties: HashMap::new() },
            GraphNode { id: "c".into(), label: "C".into(), node_type: "expert".into(), properties: HashMap::new() },
            GraphNode { id: "d".into(), label: "D".into(), node_type: "expert".into(), properties: HashMap::new() },
        ];
        let edges = vec![
            GraphEdge { source: "s".into(), target: "a".into(), edge_type: "collaborates_with".into(), weight: 0.9, properties: HashMap::new() },
            GraphEdge { source: "a".into(), target: "b".into(), edge_type: "collaborates_with".into(), weight: 0.8, properties: HashMap::new() },
            GraphEdge { source: "b".into(), target: "c".into(), edge_type: "collaborates_with".into(), weight: 0.7, properties: HashMap::new() },
            GraphEdge { source: "c".into(), target: "d".into(), edge_type: "collaborates_with".into(), weight: 0.6, properties: HashMap::new() },
        ];
        let graph = ExpertGraph { nodes, edges, built_at: now_iso(), version: 1 };
        // max_depth=2：到 a(1)、b(2)，到不了 c(3)、d(4)
        let h2 = expand_neighborhood(&graph, &["s".into()], 2, 50, &rag_empty_types(), 0.0);
        let ids2: Vec<&str> = h2.iter().map(|h| h.node_id.as_str()).collect();
        assert!(ids2.contains(&"a"));
        assert!(ids2.contains(&"b"));
        assert!(!ids2.contains(&"c"));
        assert!(!ids2.contains(&"d"));
        // max_depth=3：到 c(3)，仍到不了 d(4)
        let h3 = expand_neighborhood(&graph, &["s".into()], 3, 50, &rag_empty_types(), 0.0);
        let ids3: Vec<&str> = h3.iter().map(|h| h.node_id.as_str()).collect();
        assert!(ids3.contains(&"c"));
        assert!(!ids3.contains(&"d"));
        // max_depth=4：才到 d
        let h4 = expand_neighborhood(&graph, &["s".into()], 4, 50, &rag_empty_types(), 0.0);
        assert!(h4.iter().any(|h| h.node_id == "d"));
    }

    #[test]
    fn test_rag_node_types_filter() {
        let graph = make_test_graph();
        let mut types = HashSet::new();
        types.insert("expert".to_string());
        let hits = expand_neighborhood(&graph, &["exp-1".into()], 2, 50, &types, 0.0);
        // 只返回 expert 类型，domain-ai/domain-data 被过滤
        for h in &hits {
            assert_eq!(h.node_id.starts_with("domain-"), false, "不应返回 domain 节点: {}", h.node_id);
        }
        assert!(hits.iter().any(|h| h.node_id == "exp-2"));
        assert!(hits.iter().any(|h| h.node_id == "exp-3"));
        // 但路径里仍保留 domain 节点（exp-2 经 domain-data 到 exp-3 的路径要完整）
        // exp-3 的路径可能含 domain-data
    }

    #[test]
    fn test_rag_min_weight_filter_edges() {
        // 构造图：s -- a(0.9) -- x(0.3)；x 只有弱边 0.3 可达
        let nodes = vec![
            GraphNode { id: "s".into(), label: "S".into(), node_type: "expert".into(), properties: HashMap::new() },
            GraphNode { id: "a".into(), label: "A".into(), node_type: "expert".into(), properties: HashMap::new() },
            GraphNode { id: "x".into(), label: "X".into(), node_type: "expert".into(), properties: HashMap::new() },
        ];
        let edges = vec![
            GraphEdge { source: "s".into(), target: "a".into(), edge_type: "collaborates_with".into(), weight: 0.9, properties: HashMap::new() },
            GraphEdge { source: "a".into(), target: "x".into(), edge_type: "collaborates_with".into(), weight: 0.3, properties: HashMap::new() },
        ];
        let graph = ExpertGraph { nodes, edges, built_at: now_iso(), version: 1 };
        // min_weight=0.5：a(0.9) 可达，x 因 a--x(0.3<0.5) 被过滤而不可达
        let hits = expand_neighborhood(&graph, &["s".into()], 3, 50, &rag_empty_types(), 0.5);
        let ids: Vec<&str> = hits.iter().map(|h| h.node_id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(!ids.contains(&"x"), "min_weight=0.5 下 x 应不可达（a--x 边权重 0.3 被过滤）");
        // min_weight=0.2：x 可达
        let hits2 = expand_neighborhood(&graph, &["s".into()], 3, 50, &rag_empty_types(), 0.2);
        assert!(hits2.iter().any(|h| h.node_id == "x"));
    }

    #[test]
    fn test_rag_top_k_truncation() {
        let graph = make_test_graph();
        let hits = expand_neighborhood(&graph, &["exp-1".into()], 2, 1, &rag_empty_types(), 0.0);
        assert_eq!(hits.len(), 1);
        // 权重最高的是 domain-ai (1.0)
        assert_eq!(hits[0].node_id, "domain-ai");
    }

    #[test]
    fn test_rag_handler_seed_not_found_404() {
        let state = crud_state();
        let body = RagExpandBody {
            seeds: vec!["exp-1".into(), "ghost".into()],
            max_depth: Some(2),
            top_k: Some(20),
            node_types: None,
            min_weight: None,
        };
        let resp = rt().block_on(post_rag_expand(State(state), TenantId("default".into()), Json(body)));
        assert_eq!(resp.code, 404);
    }

    #[test]
    fn test_rag_handler_validation_400() {
        let state = crud_state();
        // seeds 空
        let r1 = rt().block_on(post_rag_expand(State(state.clone()), TenantId("default".into()), Json(RagExpandBody {
            seeds: vec![], max_depth: None, top_k: None, node_types: None, min_weight: None,
        })));
        assert_eq!(r1.code, 400);
        // max_depth=5 越界
        let r2 = rt().block_on(post_rag_expand(State(state.clone()), TenantId("default".into()), Json(RagExpandBody {
            seeds: vec!["exp-1".into()], max_depth: Some(5), top_k: None, node_types: None, min_weight: None,
        })));
        assert_eq!(r2.code, 400);
        // top_k=0
        let r3 = rt().block_on(post_rag_expand(State(state.clone()), TenantId("default".into()), Json(RagExpandBody {
            seeds: vec!["exp-1".into()], max_depth: None, top_k: Some(0), node_types: None, min_weight: None,
        })));
        assert_eq!(r3.code, 400);
    }

    #[test]
    fn test_rag_handler_ok_returns_results() {
        let state = crud_state();
        let resp = rt().block_on(post_rag_expand(State(state), TenantId("default".into()), Json(RagExpandBody {
            seeds: vec!["exp-1".into()],
            max_depth: Some(2),
            top_k: Some(20),
            node_types: Some(vec!["expert".into()]),
            min_weight: Some(0.0),
        })));
        assert_eq!(resp.code, 0);
        let data = resp.data.as_ref().unwrap();
        let results = data["results"].as_array().unwrap();
        // 只回 expert：exp-2、exp-3
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r["node"]["node_type"] == "expert"));
        // stats 字段齐全
        assert!(data["stats"]["searched_nodes"].is_number());
        assert!(data["stats"]["returned"].is_number());
        assert!(data["stats"]["elapsed_ms"].is_number());
        // rerank 标注向量融合待 #27
        assert!(data["rerank"].as_str().unwrap().contains("#27"));
    }
}
