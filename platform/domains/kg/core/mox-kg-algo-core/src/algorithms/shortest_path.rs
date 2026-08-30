// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 最短路径算法集
//!
//! # 算法概览
//!
//! ## Dijkstra 单源最短路径
//! - 适用：非负权边图
//! - 时间复杂度：O((V+E) log V)，使用二叉堆优化
//! - 实现：基于 CSR 稀疏结构，BinaryHeap 作为优先队列
//!
//! ## Bellman-Ford 算法
//! - 适用：含负权边的图，可检测负环
//! - 时间复杂度：O(V·E)
//! - 特性：第 V 轮仍可松弛则存在负环
//!
//! ## Floyd-Warshall 全源最短路径
//! - 适用：小规模图（V < 500），稠密图
//! - 时间复杂度：O(V³)
//! - 空间复杂度：O(V²)
//!
//! ## A* 启发式搜索
//! - 适用：有启发函数的单源单目标最短路径
//! - 时间复杂度：取决于启发函数的质量
//! - 启发函数：欧氏距离、曼哈顿距离、可采纳启发式
//!
//! ## 双向 BFS 最短路径
//! - 适用：无权图的单源单目标最短路径
//! - 时间复杂度：O(b^(d/2))，b 为分支因子，d 为最短路径长度
//! - 比单向 BFS 快得多（搜索空间从 b^d 降到 2·b^(d/2)）
//!
//! ## k 最短路径（Yen 算法）
//! - 适用：寻找从源到目标的前 k 条最短路径
//! - 时间复杂度：O(K·N·(M + N log N))
//! - 基于 Dijkstra 迭代寻找偏离路径

use crate::csr::CsrAdj;
use crate::graph::KnowledgeGraph;
use crate::types::PathResult;
use crate::Result;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

// ============================================================================
// Dijkstra 最短路径（基于 CSR + BinaryHeap）
// ============================================================================

/// Dijkstra 优先队列元素
#[derive(Debug, Clone, Copy)]
struct DijkstraState {
    cost: f64,
    node: usize,
}

impl PartialEq for DijkstraState {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.node == other.node
    }
}

impl Eq for DijkstraState {}

impl PartialOrd for DijkstraState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DijkstraState {
    fn cmp(&self, other: &Self) -> Ordering {
        // 反转：让 BinaryHeap 成为最小堆（cost 小的优先）
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.node.cmp(&other.node))
    }
}

/// Dijkstra 单源最短路径（CSR 实现）
///
/// 返回从 source 到所有可达节点的最短距离和前驱节点。
/// 适用于所有边权非负的图。
pub(crate) fn dijkstra_csr(
    csr: &CsrAdj,
    source: usize,
    target: Option<usize>,
) -> (Vec<Option<f64>>, Vec<Option<usize>>) {
    let n = csr.n;
    let mut dist = vec![None; n];
    let mut prev = vec![None; n];
    let mut heap = BinaryHeap::new();

    dist[source] = Some(0.0);
    heap.push(DijkstraState {
        cost: 0.0,
        node: source,
    });

    while let Some(DijkstraState { cost, node }) = heap.pop() {
        // 已找到更优路径，跳过
        if let Some(d) = dist[node] {
            if cost > d + 1e-12 {
                continue;
            }
        }

        // 到达目标，提前返回
        if Some(node) == target {
            break;
        }

        let rng = csr.offsets[node]..csr.offsets[node + 1];
        for k in rng {
            let neighbor = csr.targets[k];
            let weight = csr.weights[k];
            let new_cost = cost + weight;

            match dist[neighbor] {
                Some(d) if new_cost >= d - 1e-12 => continue,
                _ => {
                    dist[neighbor] = Some(new_cost);
                    prev[neighbor] = Some(node);
                    heap.push(DijkstraState {
                        cost: new_cost,
                        node: neighbor,
                    });
                }
            }
        }
    }

    (dist, prev)
}

/// 从前驱数组重建路径
fn reconstruct_path(prev: &[Option<usize>], source: usize, target: usize) -> Option<Vec<usize>> {
    let mut path = Vec::new();
    let mut current = target;

    if prev[target].is_none() && source != target {
        return None;
    }

    path.push(current);
    while current != source {
        match prev[current] {
            Some(p) => {
                current = p;
                path.push(current);
            }
            None => return None,
        }
    }
    path.reverse();
    Some(path)
}

// ============================================================================
// Bellman-Ford 算法
// ============================================================================

/// Bellman-Ford 算法结果
#[derive(Debug, Clone)]
pub struct BellmanFordResult {
    /// 各节点的最短距离（None 表示不可达）
    pub distances: Vec<Option<f64>>,
    /// 各节点的前驱节点
    pub predecessors: Vec<Option<usize>>,
    /// 是否存在负环
    pub has_negative_cycle: bool,
    /// 负环上的节点（如果存在）
    pub negative_cycle_nodes: Vec<usize>,
}

/// Bellman-Ford 单源最短路径
///
/// 支持负权边，并能检测负环。
/// 时间复杂度 O(V·E)，空间复杂度 O(V)。
pub(crate) fn bellman_ford_csr(csr: &CsrAdj, source: usize) -> BellmanFordResult {
    let n = csr.n;
    let m = csr.targets.len();
    let mut dist = vec![f64::INFINITY; n];
    let mut prev = vec![None; n];

    dist[source] = 0.0;

    // 收集所有边
    let mut edges = Vec::with_capacity(m);
    for i in 0..n {
        let rng = csr.offsets[i]..csr.offsets[i + 1];
        for k in rng {
            edges.push((i, csr.targets[k], csr.weights[k]));
        }
    }

    // V-1 轮松弛
    for _ in 0..n.saturating_sub(1) {
        let mut updated = false;
        for &(u, v, w) in &edges {
            if dist[u] + w < dist[v] - 1e-12 {
                dist[v] = dist[u] + w;
                prev[v] = Some(u);
                updated = true;
            }
        }
        if !updated {
            break; // 提前收敛
        }
    }

    // 第 V 轮检测负环
    let mut has_negative_cycle = false;
    let mut on_cycle = vec![false; n];
    for &(u, v, w) in &edges {
        if dist[u] + w < dist[v] - 1e-12 {
            has_negative_cycle = true;
            on_cycle[v] = true;
        }
    }

    // 找出所有能到达负环或被负环到达的节点
    let mut negative_cycle_nodes = Vec::new();
    if has_negative_cycle {
        // BFS 找出所有在负环上或能被负环到达的节点
        let mut queue = VecDeque::new();
        for v in 0..n {
            if on_cycle[v] {
                queue.push_back(v);
                negative_cycle_nodes.push(v);
            }
        }
        while let Some(u) = queue.pop_front() {
            let rng = csr.offsets[u]..csr.offsets[u + 1];
            for k in rng {
                let v = csr.targets[k];
                if !on_cycle[v] {
                    on_cycle[v] = true;
                    negative_cycle_nodes.push(v);
                    queue.push_back(v);
                }
            }
        }
    }

    // 将 INF 转为 None
    let distances: Vec<Option<f64>> = dist
        .iter()
        .map(|&d| if d.is_finite() { Some(d) } else { None })
        .collect();

    BellmanFordResult {
        distances,
        predecessors: prev,
        has_negative_cycle,
        negative_cycle_nodes,
    }
}

// ============================================================================
// Floyd-Warshall 全源最短路径
// ============================================================================

/// Floyd-Warshall 全源最短路径
///
/// 适用于小规模图（V < 500），支持负权边但不能有负环。
/// 时间复杂度 O(V³)，空间复杂度 O(V²)。
pub(crate) fn floyd_warshall_csr(csr: &CsrAdj) -> Vec<Vec<Option<f64>>> {
    let n = csr.n;
    let inf = f64::INFINITY;

    // 初始化距离矩阵
    let mut dist = vec![vec![inf; n]; n];
    for i in 0..n {
        dist[i][i] = 0.0;
    }

    // 填入边权
    for i in 0..n {
        let rng = csr.offsets[i]..csr.offsets[i + 1];
        for k in rng {
            let j = csr.targets[k];
            let w = csr.weights[k];
            if w < dist[i][j] {
                dist[i][j] = w;
            }
        }
    }

    // Floyd-Warshall 主循环
    for k in 0..n {
        for i in 0..n {
            if dist[i][k] == inf {
                continue;
            }
            for j in 0..n {
                if dist[k][j] == inf {
                    continue;
                }
                let new_dist = dist[i][k] + dist[k][j];
                if new_dist < dist[i][j] - 1e-12 {
                    dist[i][j] = new_dist;
                }
            }
        }
    }

    // 转为 Option<f64>
    dist.into_iter()
        .map(|row| {
            row.into_iter()
                .map(|d| if d.is_finite() { Some(d) } else { None })
                .collect()
        })
        .collect()
}

// ============================================================================
// A* 启发式搜索
// ============================================================================

/// A* 算法状态
#[derive(Debug, Clone)]
struct AStarState {
    /// f = g + h，总估计代价
    f_score: f64,
    /// g，从起点到当前节点的实际代价
    g_score: f64,
    node: usize,
}

impl PartialEq for AStarState {
    fn eq(&self, other: &Self) -> bool {
        self.f_score == other.f_score && self.node == other.node
    }
}

impl Eq for AStarState {}

impl PartialOrd for AStarState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AStarState {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f_score
            .partial_cmp(&self.f_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.node.cmp(&other.node))
    }
}

/// 启发函数类型
#[derive(Debug, Clone, Copy)]
pub enum HeuristicType {
    /// 零启发式（退化为 Dijkstra）
    Zero,
    /// 常数启发式（h = 常数，可加速但不保证最优）
    Constant(f64),
    /// 基于度数的启发式（度数差的倒数，适合无权图近似）
    DegreeBased,
}

/// A* 启发式搜索最短路径
///
/// 使用启发函数指导搜索方向，减少搜索空间。
/// 当启发函数是可采纳的（admissible，即不高估真实距离）时，
/// 保证找到最优路径。
pub(crate) fn a_star_csr(
    csr: &CsrAdj,
    source: usize,
    target: usize,
    heuristic: HeuristicType,
) -> Option<(f64, Vec<usize>)> {
    let n = csr.n;
    if source >= n || target >= n {
        return None;
    }
    if source == target {
        return Some((0.0, vec![source]));
    }

    // 预计算启发值
    let h = compute_heuristic(csr, target, heuristic);

    let mut g_score = vec![f64::INFINITY; n];
    let mut prev = vec![None; n];
    let mut heap = BinaryHeap::new();
    let mut closed = HashSet::new();

    g_score[source] = 0.0;
    heap.push(AStarState {
        f_score: h[source],
        g_score: 0.0,
        node: source,
    });

    while let Some(AStarState {
        f_score: _,
        g_score: current_g,
        node,
    }) = heap.pop()
    {
        if node == target {
            let path = reconstruct_path(&prev, source, target)?;
            return Some((current_g, path));
        }

        if !closed.insert(node) {
            continue;
        }

        let rng = csr.offsets[node]..csr.offsets[node + 1];
        for k in rng {
            let neighbor = csr.targets[k];
            let weight = csr.weights[k];

            if closed.contains(&neighbor) {
                continue;
            }

            let tentative_g = current_g + weight;
            if tentative_g < g_score[neighbor] - 1e-12 {
                g_score[neighbor] = tentative_g;
                prev[neighbor] = Some(node);
                heap.push(AStarState {
                    f_score: tentative_g + h[neighbor],
                    g_score: tentative_g,
                    node: neighbor,
                });
            }
        }
    }

    None
}

/// 计算启发函数值
fn compute_heuristic(csr: &CsrAdj, target: usize, heuristic: HeuristicType) -> Vec<f64> {
    let n = csr.n;
    match heuristic {
        HeuristicType::Zero => vec![0.0; n],
        HeuristicType::Constant(c) => vec![c; n],
        HeuristicType::DegreeBased => {
            // 目标节点出度作为基准
            let target_degree = (csr.offsets[target + 1] - csr.offsets[target]) as f64;
            (0..n)
                .map(|v| {
                    let v_degree = (csr.offsets[v + 1] - csr.offsets[v]) as f64;
                    // 度数差的绝对值，归一化
                    (v_degree - target_degree).abs() / (target_degree + 1.0).max(1.0)
                })
                .collect()
        }
    }
}

// ============================================================================
// 双向 BFS 最短路径（无权图）
// ============================================================================

/// 双向 BFS 最短路径
///
/// 从源和目标同时进行 BFS，在中间相遇。
/// 适用于无权图的单源单目标最短路径。
/// 比单向 BFS 搜索空间小得多（O(b^(d/2)) vs O(b^d)）。
pub(crate) fn bidirectional_bfs_csr(
    csr: &CsrAdj,
    source: usize,
    target: usize,
) -> Option<(usize, Vec<usize>)> {
    let n = csr.n;
    if source >= n || target >= n {
        return None;
    }
    if source == target {
        return Some((0, vec![source]));
    }

    // 构建反向邻接表（用于从目标反向搜索）
    let mut reverse_adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        let rng = csr.offsets[i]..csr.offsets[i + 1];
        for k in rng {
            let j = csr.targets[k];
            reverse_adj[j].push(i);
        }
    }

    let mut forward_visited = vec![None; n]; // 记录前驱
    let mut backward_visited = vec![None; n];
    let mut forward_queue = VecDeque::new();
    let mut backward_queue = VecDeque::new();

    forward_visited[source] = Some(source);
    backward_visited[target] = Some(target);
    forward_queue.push_back(source);
    backward_queue.push_back(target);

    let mut meeting_node = None;

    while !forward_queue.is_empty() && !backward_queue.is_empty() {
        // 从较小的一侧扩展（平衡搜索）
        if forward_queue.len() <= backward_queue.len() {
            // 扩展正向
            let level_size = forward_queue.len();
            for _ in 0..level_size {
                let u = forward_queue.pop_front().unwrap();
                let rng = csr.offsets[u]..csr.offsets[u + 1];
                for k in rng {
                    let v = csr.targets[k];
                    if forward_visited[v].is_some() {
                        continue;
                    }
                    forward_visited[v] = Some(u);
                    if backward_visited[v].is_some() {
                        meeting_node = Some(v);
                        break;
                    }
                    forward_queue.push_back(v);
                }
                if meeting_node.is_some() {
                    break;
                }
            }
        } else {
            // 扩展反向
            let level_size = backward_queue.len();
            for _ in 0..level_size {
                let u = backward_queue.pop_front().unwrap();
                for &v in &reverse_adj[u] {
                    if backward_visited[v].is_some() {
                        continue;
                    }
                    backward_visited[v] = Some(u);
                    if forward_visited[v].is_some() {
                        meeting_node = Some(v);
                        break;
                    }
                    backward_queue.push_back(v);
                }
                if meeting_node.is_some() {
                    break;
                }
            }
        }

        if meeting_node.is_some() {
            break;
        }
    }

    let meet = meeting_node?;

    // 重建正向路径
    let mut forward_path = Vec::new();
    let mut cur = meet;
    forward_path.push(cur);
    while cur != source {
        cur = forward_visited[cur]?;
        forward_path.push(cur);
    }
    forward_path.reverse();

    // 重建反向路径
    let mut cur = meet;
    while cur != target {
        cur = backward_visited[cur]?;
        forward_path.push(cur);
    }

    let distance = forward_path.len() - 1;
    Some((distance, forward_path))
}

// ============================================================================
// k 最短路径（Yen 算法）
// ============================================================================

/// Yen 算法 k 最短路径
///
/// 寻找从源到目标的前 k 条最短简单路径（无重复节点）。
/// 时间复杂度：O(K·N·(M + N log N))
///
/// # 算法原理
/// 1. 使用 Dijkstra 找到第 1 条最短路径
/// 2. 对每条已找到的路径，在每个节点处"偏离"，移除该边后找最短路径
/// 3. 使用候选堆维护偏离路径，按长度排序取前 k 条
pub(crate) fn yen_k_shortest_paths_csr(
    csr: &CsrAdj,
    source: usize,
    target: usize,
    k: usize,
) -> Vec<(f64, Vec<usize>)> {
    let n = csr.n;
    if k == 0 || source >= n || target >= n {
        return Vec::new();
    }
    if source == target {
        return vec![(0.0, vec![source])];
    }

    let mut results: Vec<(f64, Vec<usize>)> = Vec::new();
    let mut candidates: BinaryHeap<YenCandidate> = BinaryHeap::new();

    // 第 1 条最短路径
    let (dist, prev) = dijkstra_csr(csr, source, Some(target));
    match (dist[target], reconstruct_path(&prev, source, target)) {
        (Some(d), Some(p)) => {
            candidates.push(YenCandidate {
                cost: d,
                path: p,
                deviation_idx: 0,
            });
        }
        _ => return Vec::new(), // 无路径
    }

    // 已移除的边（按偏离节点分组）
    let mut removed_edges: HashSet<(usize, usize)> = HashSet::new();

    while results.len() < k && !candidates.is_empty() {
        let candidate = candidates.pop().unwrap();
        let (cost, path, dev_idx) = (candidate.cost, candidate.path, candidate.deviation_idx);

        // 检查是否与已选路径重复
        if results.iter().any(|(_, p)| p == &path) {
            continue;
        }

        results.push((cost, path.clone()));
        if results.len() >= k {
            break;
        }

        // 从偏离节点开始，依次尝试移除各边
        for i in dev_idx..path.len().saturating_sub(1) {
            let spur_node = path[i];
            let root_path = &path[..=i];

            // 收集需要临时移除的边
            let mut temp_removed = Vec::new();
            for (_, result_path) in &results {
                if result_path.len() > i && result_path[..=i] == *root_path {
                    let edge = (result_path[i], result_path[i + 1]);
                    if removed_edges.insert(edge) {
                        temp_removed.push(edge);
                    }
                }
            }

            // 临时移除 root_path 上除 spur_node 外的节点
            let mut removed_nodes = Vec::new();
            for &node in &path[..i] {
                removed_nodes.push(node);
            }

            // 构建临时 CSR（通过修改权重实现）
            // 简化实现：使用带屏蔽的 Dijkstra
            if let Some((spur_cost, spur_path)) =
                dijkstra_with_exclusions(csr, spur_node, target, &removed_edges, &removed_nodes)
            {
                if !spur_path.is_empty() && spur_path[0] == spur_node {
                    let mut full_path = root_path.to_vec();
                    full_path.extend_from_slice(&spur_path[1..]);

                    // 检查路径有效性（无重复节点）
                    let mut seen = HashSet::new();
                    let mut valid = true;
                    for &node in &full_path {
                        if !seen.insert(node) {
                            valid = false;
                            break;
                        }
                    }

                    if valid {
                        // 计算总代价
                        let total_cost = cost_from_path(csr, &full_path);
                        candidates.push(YenCandidate {
                            cost: total_cost,
                            path: full_path,
                            deviation_idx: i + 1,
                        });
                    }
                }
            }

            // 恢复移除的边
            for edge in temp_removed {
                removed_edges.remove(&edge);
            }
        }
    }

    results
}

/// Yen 算法候选堆元素
#[derive(Debug, Clone)]
struct YenCandidate {
    cost: f64,
    path: Vec<usize>,
    deviation_idx: usize,
}

impl PartialEq for YenCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}

impl Eq for YenCandidate {}

impl PartialOrd for YenCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for YenCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // 最小堆：cost 小的优先
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
    }
}

/// 带排除边和排除节点的 Dijkstra
fn dijkstra_with_exclusions(
    csr: &CsrAdj,
    source: usize,
    target: usize,
    excluded_edges: &HashSet<(usize, usize)>,
    excluded_nodes: &[usize],
) -> Option<(f64, Vec<usize>)> {
    let n = csr.n;
    let mut dist = vec![f64::INFINITY; n];
    let mut prev = vec![None; n];
    let mut heap = BinaryHeap::new();
    let excluded_set: HashSet<usize> = excluded_nodes.iter().copied().collect();

    if excluded_set.contains(&source) || excluded_set.contains(&target) {
        return None;
    }

    dist[source] = 0.0;
    heap.push(DijkstraState {
        cost: 0.0,
        node: source,
    });

    while let Some(DijkstraState { cost, node }) = heap.pop() {
        if cost > dist[node] + 1e-12 {
            continue;
        }
        if node == target {
            break;
        }

        let rng = csr.offsets[node]..csr.offsets[node + 1];
        for k in rng {
            let neighbor = csr.targets[k];
            let weight = csr.weights[k];

            if excluded_set.contains(&neighbor) {
                continue;
            }
            if excluded_edges.contains(&(node, neighbor)) {
                continue;
            }

            let new_cost = cost + weight;
            if new_cost < dist[neighbor] - 1e-12 {
                dist[neighbor] = new_cost;
                prev[neighbor] = Some(node);
                heap.push(DijkstraState {
                    cost: new_cost,
                    node: neighbor,
                });
            }
        }
    }

    if dist[target].is_infinite() {
        None
    } else {
        let path = reconstruct_path(&prev, source, target)?;
        Some((dist[target], path))
    }
}

/// 计算路径的总权重
fn cost_from_path(csr: &CsrAdj, path: &[usize]) -> f64 {
    let mut total = 0.0;
    for window in path.windows(2) {
        let u = window[0];
        let v = window[1];
        // 查找边权
        let rng = csr.offsets[u]..csr.offsets[u + 1];
        for k in rng {
            if csr.targets[k] == v {
                total += csr.weights[k];
                break;
            }
        }
    }
    total
}

// ============================================================================
// KnowledgeGraph 扩展方法
// ============================================================================

impl KnowledgeGraph {
    // --- Dijkstra ---

    /// Dijkstra 单源最短路径（优化版，基于 CSR + BinaryHeap）
    ///
    /// 适用于所有边权非负的图，时间复杂度 O((V+E) log V)。
    pub fn dijkstra_shortest_path(
        &self,
        source: &str,
        target: &str,
    ) -> Result<Option<PathResult>> {
        let source_idx = self
            .node_map
            .get(source)
            .ok_or_else(|| anyhow::anyhow!("源节点不存在: {}", source))?
            .index();
        let target_idx = self
            .node_map
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("目标节点不存在: {}", target))?
            .index();

        let csr = CsrAdj::from_graph(&self.graph);
        let (dist, prev) = dijkstra_csr(&csr, source_idx, Some(target_idx));

        match (dist[target_idx], reconstruct_path(&prev, source_idx, target_idx)) {
            (Some(d), Some(p)) => {
                let path_ids: Vec<String> = p
                    .iter()
                    .map(|&idx| self.graph[petgraph::graph::NodeIndex::new(idx)].id.clone())
                    .collect();
                Ok(Some(PathResult {
                    path: path_ids,
                    total_weight: d,
                    length: p.len() - 1,
                }))
            }
            _ => Ok(None),
        }
    }

    /// Dijkstra 单源到所有节点的最短距离
    pub fn dijkstra_all_distances(&self, source: &str) -> Result<HashMap<String, f64>> {
        let source_idx = self
            .node_map
            .get(source)
            .ok_or_else(|| anyhow::anyhow!("源节点不存在: {}", source))?
            .index();

        let csr = CsrAdj::from_graph(&self.graph);
        let (dist, _) = dijkstra_csr(&csr, source_idx, None);

        let mut result = HashMap::new();
        for (id, idx) in &self.node_map {
            if let Some(d) = dist[idx.index()] {
                result.insert(id.clone(), d);
            }
        }
        Ok(result)
    }

    // --- Bellman-Ford ---

    /// Bellman-Ford 最短路径（支持负权边，检测负环）
    ///
    /// 返回 (距离映射, 是否存在负环, 负环节点列表)
    pub fn bellman_ford(
        &self,
        source: &str,
    ) -> Result<(HashMap<String, f64>, bool, Vec<String>)> {
        let source_idx = self
            .node_map
            .get(source)
            .ok_or_else(|| anyhow::anyhow!("源节点不存在: {}", source))?
            .index();

        let csr = CsrAdj::from_graph(&self.graph);
        let result = bellman_ford_csr(&csr, source_idx);

        let mut distances = HashMap::new();
        for (id, idx) in &self.node_map {
            if let Some(d) = result.distances[idx.index()] {
                distances.insert(id.clone(), d);
            }
        }

        let neg_cycle_nodes: Vec<String> = result
            .negative_cycle_nodes
            .iter()
            .map(|&idx| self.graph[petgraph::graph::NodeIndex::new(idx)].id.clone())
            .collect();

        Ok((distances, result.has_negative_cycle, neg_cycle_nodes))
    }

    // --- Floyd-Warshall ---

    /// Floyd-Warshall 全源最短路径
    ///
    /// 适用于小规模图（节点数 < 500）。
    /// 返回距离矩阵：(source_id, target_id) -> distance
    pub fn floyd_warshall_all_pairs(&self) -> HashMap<(String, String), f64> {
        let csr = CsrAdj::from_graph(&self.graph);
        let dist_matrix = floyd_warshall_csr(&csr);

        let mut result = HashMap::new();
        let nodes: Vec<(String, usize)> = self
            .node_map
            .iter()
            .map(|(id, idx)| (id.clone(), idx.index()))
            .collect();

        for (id_i, idx_i) in &nodes {
            for (id_j, idx_j) in &nodes {
                if let Some(d) = dist_matrix[*idx_i][*idx_j] {
                    result.insert((id_i.clone(), id_j.clone()), d);
                }
            }
        }
        result
    }

    // --- A* ---

    /// A* 启发式搜索最短路径
    pub fn a_star_shortest_path(
        &self,
        source: &str,
        target: &str,
        heuristic: HeuristicType,
    ) -> Result<Option<PathResult>> {
        let source_idx = self
            .node_map
            .get(source)
            .ok_or_else(|| anyhow::anyhow!("源节点不存在: {}", source))?
            .index();
        let target_idx = self
            .node_map
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("目标节点不存在: {}", target))?
            .index();

        let csr = CsrAdj::from_graph(&self.graph);
        match a_star_csr(&csr, source_idx, target_idx, heuristic) {
            Some((cost, path)) => {
                let path_ids: Vec<String> = path
                    .iter()
                    .map(|&idx| self.graph[petgraph::graph::NodeIndex::new(idx)].id.clone())
                    .collect();
                Ok(Some(PathResult {
                    path: path_ids,
                    total_weight: cost,
                    length: path.len() - 1,
                }))
            }
            None => Ok(None),
        }
    }

    // --- 双向 BFS ---

    /// 双向 BFS 最短路径（无权图）
    ///
    /// 对于无权图，双向 BFS 比单向 BFS 快得多。
    pub fn bidirectional_bfs(&self, source: &str, target: &str) -> Result<Option<PathResult>> {
        let source_idx = self
            .node_map
            .get(source)
            .ok_or_else(|| anyhow::anyhow!("源节点不存在: {}", source))?
            .index();
        let target_idx = self
            .node_map
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("目标节点不存在: {}", target))?
            .index();

        let csr = CsrAdj::from_graph(&self.graph);
        match bidirectional_bfs_csr(&csr, source_idx, target_idx) {
            Some((distance, path)) => {
                let path_ids: Vec<String> = path
                    .iter()
                    .map(|&idx| self.graph[petgraph::graph::NodeIndex::new(idx)].id.clone())
                    .collect();
                Ok(Some(PathResult {
                    path: path_ids,
                    total_weight: distance as f64,
                    length: distance,
                }))
            }
            None => Ok(None),
        }
    }

    // --- k 最短路径 ---

    /// k 最短路径（Yen 算法）
    ///
    /// 返回从 source 到 target 的前 k 条最短简单路径。
    pub fn k_shortest_paths(
        &self,
        source: &str,
        target: &str,
        k: usize,
    ) -> Result<Vec<PathResult>> {
        let source_idx = self
            .node_map
            .get(source)
            .ok_or_else(|| anyhow::anyhow!("源节点不存在: {}", source))?
            .index();
        let target_idx = self
            .node_map
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("目标节点不存在: {}", target))?
            .index();

        let csr = CsrAdj::from_graph(&self.graph);
        let paths = yen_k_shortest_paths_csr(&csr, source_idx, target_idx, k);

        let results: Vec<PathResult> = paths
            .into_iter()
            .map(|(cost, path)| {
                let path_ids: Vec<String> = path
                    .iter()
                    .map(|&idx| self.graph[petgraph::graph::NodeIndex::new(idx)].id.clone())
                    .collect();
                PathResult {
                    path: path_ids,
                    total_weight: cost,
                    length: path.len() - 1,
                }
            })
            .collect();

        Ok(results)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::KnowledgeGraphBuilder;
    use approx::assert_relative_eq;

    fn build_weighted_graph() -> KnowledgeGraph {
        KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_node("c", "C", "test")
            .add_node("d", "D", "test")
            .add_node("e", "E", "test")
            .add_edge("a", "b", 4.0)
            .add_edge("a", "c", 2.0)
            .add_edge("b", "c", 1.0)
            .add_edge("b", "d", 5.0)
            .add_edge("c", "d", 8.0)
            .add_edge("c", "e", 10.0)
            .add_edge("d", "e", 2.0)
            .build()
    }

    fn build_unweighted_graph() -> KnowledgeGraph {
        KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_node("c", "C", "test")
            .add_node("d", "D", "test")
            .add_node("e", "E", "test")
            .add_node("f", "F", "test")
            .add_edge("a", "b", 1.0)
            .add_edge("a", "c", 1.0)
            .add_edge("b", "d", 1.0)
            .add_edge("c", "e", 1.0)
            .add_edge("d", "f", 1.0)
            .add_edge("e", "f", 1.0)
            .build()
    }

    #[test]
    fn test_dijkstra_basic() {
        let graph = build_weighted_graph();
        let result = graph.dijkstra_shortest_path("a", "e").unwrap();
        assert!(result.is_some());

        let path = result.unwrap();
        // 最短路径: a -> c -> d -> e? 不对, a->c->d->e = 2+8+2=12
        // a->b->c->d->e = 4+1+8+2=15
        // a->b->d->e = 4+5+2=11
        // a->c->e = 2+10=12
        // 最短: a->b->d->e = 11
        assert_relative_eq!(path.total_weight, 11.0, epsilon = 1e-6);
    }

    #[test]
    fn test_dijkstra_no_path() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .build();

        let result = graph.dijkstra_shortest_path("a", "b").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_dijkstra_all_distances() {
        let graph = build_weighted_graph();
        let dists = graph.dijkstra_all_distances("a").unwrap();
        assert_eq!(dists.len(), 5);
        assert_relative_eq!(dists["a"], 0.0);
        assert_relative_eq!(dists["b"], 4.0);
        assert_relative_eq!(dists["c"], 2.0);
    }

    #[test]
    fn test_bellman_ford_no_neg_cycle() {
        let graph = build_weighted_graph();
        let (dists, has_neg, _) = graph.bellman_ford("a").unwrap();
        assert!(!has_neg);
        assert_relative_eq!(dists["a"], 0.0);
        assert_relative_eq!(dists["b"], 4.0);
    }

    #[test]
    fn test_bellman_ford_with_neg_edges() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_node("c", "C", "test")
            .add_edge("a", "b", 1.0)
            .add_edge("b", "c", -2.0)
            .add_edge("a", "c", 5.0)
            .build();

        let (dists, has_neg, _) = graph.bellman_ford("a").unwrap();
        assert!(!has_neg);
        // a->b->c = 1 + (-2) = -1 < 5
        assert_relative_eq!(dists["c"], -1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_bellman_ford_negative_cycle() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_node("c", "C", "test")
            .add_edge("a", "b", 1.0)
            .add_edge("b", "c", -3.0)
            .add_edge("c", "a", 1.0) // 环 a->b->c->a = 1 + (-3) + 1 = -1
            .build();

        let (_, has_neg, neg_nodes) = graph.bellman_ford("a").unwrap();
        assert!(has_neg);
        assert!(!neg_nodes.is_empty());
    }

    #[test]
    fn test_floyd_warshall() {
        let graph = build_weighted_graph();
        let dists = graph.floyd_warshall_all_pairs();

        // a 到 e 的最短路径
        assert_relative_eq!(dists[&("a".to_string(), "e".to_string())], 11.0, epsilon = 1e-6);
        // 自身距离为 0
        assert_relative_eq!(dists[&("a".to_string(), "a".to_string())], 0.0, epsilon = 1e-6);
    }

    #[test]
    fn test_a_star_zero_heuristic() {
        let graph = build_weighted_graph();
        // 零启发式应与 Dijkstra 结果相同
        let result = graph
            .a_star_shortest_path("a", "e", HeuristicType::Zero)
            .unwrap();
        assert!(result.is_some());
        let path = result.unwrap();
        assert_relative_eq!(path.total_weight, 11.0, epsilon = 1e-6);
    }

    #[test]
    fn test_bidirectional_bfs() {
        let graph = build_unweighted_graph();
        let result = graph.bidirectional_bfs("a", "f").unwrap();
        assert!(result.is_some());
        let path = result.unwrap();
        // 最短路径长度为 3: a-b-d-f 或 a-c-e-f
        assert_eq!(path.length, 3);
    }

    #[test]
    fn test_bidirectional_bfs_no_path() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .build();

        let result = graph.bidirectional_bfs("a", "b").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_k_shortest_paths() {
        let graph = build_weighted_graph();
        let paths = graph.k_shortest_paths("a", "e", 3).unwrap();

        assert!(!paths.is_empty());
        assert!(paths.len() <= 3);

        // 第一条应该是最短路径
        assert_relative_eq!(paths[0].total_weight, 11.0, epsilon = 1e-6);

        // 路径应该按长度递增排序
        for i in 1..paths.len() {
            assert!(paths[i].total_weight >= paths[i - 1].total_weight - 1e-6);
        }
    }

    #[test]
    fn test_k_shortest_paths_single_node() {
        let graph = build_weighted_graph();
        let paths = graph.k_shortest_paths("a", "a", 1).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].length, 0);
        assert_relative_eq!(paths[0].total_weight, 0.0);
    }

    #[test]
    fn test_k_shortest_paths_no_path() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .build();

        let paths = graph.k_shortest_paths("a", "b", 3).unwrap();
        assert!(paths.is_empty());
    }

    #[test]
    fn test_reconstruct_path() {
        let prev = vec![None, Some(0), Some(0), Some(1)];
        let path = reconstruct_path(&prev, 0, 3);
        assert_eq!(path, Some(vec![0, 1, 3]));
    }

    #[test]
    fn test_reconstruct_path_no_path() {
        let prev = vec![None, None, Some(0)];
        let path = reconstruct_path(&prev, 0, 1);
        assert_eq!(path, None);
    }
}
