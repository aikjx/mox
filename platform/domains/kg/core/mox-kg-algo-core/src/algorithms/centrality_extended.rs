// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 扩展中心性算法
//!
//! # 算法概览
//!
//! ## 接近中心性（Closeness Centrality）
//! - 标准接近中心性：C(v) = (N-1) / Σ d(v,u)
//! - 调和中心性（Harmonic）：C(v) = Σ 1/d(v,u) / (N-1)，对不可达节点稳健
//! - 实现：无权图用 BFS，有权图用 Dijkstra
//!
//! ## 特征向量中心性（Eigenvector Centrality）
//! - 基于邻接矩阵主特征向量，幂迭代法求解
//! - C(v) ∝ Σ A(v,u)·C(u)，邻居越重要，节点越重要
//! - 时间复杂度：O((V+E)·iter)
//!
//! ## Katz 中心性（Katz Centrality）
//! - 特征向量中心性的推广，考虑所有长度的路径
//! - C(v) = α·Σ A(v,u)·C(u) + β，α 为衰减因子，β 为常数项
//! - 适用于有入度为 0 的节点的有向图
//!
//! ## HITS 算法（Hyperlink-Induced Topic Search）
//! - 权威度（Authority）：被许多高枢纽度节点指向的节点
//! - 枢纽度（Hub）：指向许多高权威度节点的节点
//! - 互增强迭代：a ← A^T·h, h ← A·a，每轮归一化
//!
//! ## 信息中心性（Information Centrality）
//! - 基于图拉普拉斯矩阵的伪逆
//! - 衡量节点在信息传播中的重要性
//! - C_I(v) = N / Σ_j (l_vv + l_jj - 2l_vj)
//!
//! ## 局部中心性（Local Centrality）
//! - 基于邻居数量和邻居的重要性
//! - 衡量节点的局部影响力
//! - 适合大规模图的快速近似中心性计算

use crate::csr::CsrAdj;
use crate::graph::KnowledgeGraph;
use std::collections::{HashMap, VecDeque};

// ============================================================================
// 接近中心性（增强版）
// ============================================================================

/// 接近中心性类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosenessType {
    /// 标准接近中心性：C(v) = (N-1) / Σ d(v,u)
    Standard,
    /// 调和中心性：C(v) = Σ 1/d(v,u) / (N-1)
    Harmonic,
}

/// 计算接近中心性（CSR 实现，支持标准和调和两种类型）
pub(crate) fn closeness_centrality_csr(
    csr: &CsrAdj,
    ctype: ClosenessType,
) -> Vec<f64> {
    let n = csr.n;
    if n == 0 {
        return Vec::new();
    }

    let mut result = vec![0.0f64; n];

    if csr.all_unit_weight {
        // 无权图：BFS
        let mut dist = vec![-1i32; n];
        let mut queue = VecDeque::with_capacity(n);

        for s in 0..n {
            for d in dist.iter_mut() {
                *d = -1;
            }
            dist[s] = 0;
            queue.clear();
            queue.push_back(s);

            while let Some(u) = queue.pop_front() {
                let rng = csr.offsets[u]..csr.offsets[u + 1];
                for k in rng {
                    let v = csr.targets[k];
                    if dist[v] < 0 {
                        dist[v] = dist[u] + 1;
                        queue.push_back(v);
                    }
                }
            }

            result[s] = match ctype {
                ClosenessType::Standard => {
                    let mut reachable = 0;
                    let mut sum_dist = 0.0;
                    for u in 0..n {
                        if u != s && dist[u] > 0 {
                            reachable += 1;
                            sum_dist += dist[u] as f64;
                        }
                    }
                    if reachable > 0 && sum_dist > 1e-15 {
                        reachable as f64 / sum_dist * (reachable as f64 / (n - 1) as f64)
                    } else {
                        0.0
                    }
                }
                ClosenessType::Harmonic => {
                    let mut harmonic = 0.0;
                    for u in 0..n {
                        if u != s && dist[u] > 0 {
                            harmonic += 1.0 / dist[u] as f64;
                        }
                    }
                    if n > 1 {
                        harmonic / (n as f64 - 1.0)
                    } else {
                        0.0
                    }
                }
            };
        }
    } else {
        // 有权图：Dijkstra
        // 使用 shortest_path 模块中的 dijkstra_csr
        for s in 0..n {
            let (dist, _) = super::shortest_path::dijkstra_csr(csr, s, None);

            result[s] = match ctype {
                ClosenessType::Standard => {
                    let mut reachable = 0;
                    let mut sum_dist = 0.0;
                    for u in 0..n {
                        if u != s {
                            if let Some(d) = dist[u] {
                                if d > 1e-15 {
                                    reachable += 1;
                                    sum_dist += d;
                                }
                            }
                        }
                    }
                    if reachable > 0 && sum_dist > 1e-15 {
                        reachable as f64 / sum_dist * (reachable as f64 / (n - 1) as f64)
                    } else {
                        0.0
                    }
                }
                ClosenessType::Harmonic => {
                    let mut harmonic = 0.0;
                    for u in 0..n {
                        if u != s {
                            if let Some(d) = dist[u] {
                                if d > 1e-15 {
                                    harmonic += 1.0 / d;
                                }
                            }
                        }
                    }
                    if n > 1 {
                        harmonic / (n as f64 - 1.0)
                    } else {
                        0.0
                    }
                }
            };
        }
    }

    result
}

// ============================================================================
// 特征向量中心性（幂迭代法）
// ============================================================================

/// 特征向量中心性配置
#[derive(Debug, Clone)]
pub struct EigenvectorConfig {
    /// 最大迭代次数
    pub max_iterations: usize,
    /// 收敛阈值
    pub tolerance: f64,
}

impl Default for EigenvectorConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-6,
        }
    }
}

/// 特征向量中心性（幂迭代法）
///
/// 使用幂迭代法求邻接矩阵的主特征向量。
/// 时间复杂度 O((V+E)·iter)，空间复杂度 O(V)。
///
/// # 算法原理
/// 1. 初始化中心性向量为全 1
/// 2. 迭代：x_new = A · x
/// 3. 归一化：x_new /= ||x_new||
/// 4. 检查收敛：||x_new - x|| < tolerance
pub(crate) fn eigenvector_centrality_csr(
    csr: &CsrAdj,
    config: &EigenvectorConfig,
) -> (Vec<f64>, bool, usize) {
    let n = csr.n;
    if n == 0 {
        return (Vec::new(), true, 0);
    }

    let mut x = vec![1.0f64 / (n as f64).sqrt(); n];
    let mut converged = false;
    let mut iterations = 0;

    for iter in 0..config.max_iterations {
        iterations = iter + 1;

        // x_new = A^T · x（入边加权，与 PageRank 方向一致）
        let mut x_new = vec![0.0f64; n];
        for i in 0..n {
            let rng = csr.offsets[i]..csr.offsets[i + 1];
            for k in rng {
                let j = csr.targets[k];
                let w = csr.weights[k];
                x_new[j] += x[i] * w;
            }
        }

        // L2 归一化
        let norm: f64 = x_new.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm < 1e-15 {
            // 零向量，无法归一化
            converged = true;
            break;
        }
        for v in x_new.iter_mut() {
            *v /= norm;
        }

        // 收敛检测
        let mut diff = 0.0;
        for i in 0..n {
            diff += (x_new[i] - x[i]).abs();
        }

        x = x_new;

        if diff < config.tolerance {
            converged = true;
            break;
        }
    }

    (x, converged, iterations)
}

// ============================================================================
// Katz 中心性
// ============================================================================

/// Katz 中心性配置
#[derive(Debug, Clone)]
pub struct KatzConfig {
    /// 衰减因子 α（必须小于邻接矩阵谱半径的倒数）
    pub alpha: f64,
    /// 常数项 β
    pub beta: f64,
    /// 最大迭代次数
    pub max_iterations: usize,
    /// 收敛阈值
    pub tolerance: f64,
}

impl Default for KatzConfig {
    fn default() -> Self {
        Self {
            alpha: 0.1,
            beta: 1.0,
            max_iterations: 100,
            tolerance: 1e-6,
        }
    }
}

/// Katz 中心性
///
/// Katz 中心性是特征向量中心性的推广，考虑所有长度的路径，
/// 路径长度 l 的贡献按 α^l 衰减。
///
/// 公式：c = α·A·c + β·1
/// 展开：c(v) = β·Σ_{l=1}^∞ Σ_u (α^l · A^l(v,u))
///
/// # 收敛条件
/// α 必须小于邻接矩阵谱半径的倒数，否则迭代发散。
pub(crate) fn katz_centrality_csr(
    csr: &CsrAdj,
    config: &KatzConfig,
) -> (Vec<f64>, bool, usize) {
    let n = csr.n;
    if n == 0 {
        return (Vec::new(), true, 0);
    }

    let mut x = vec![config.beta; n];
    let mut converged = false;
    let mut iterations = 0;

    for iter in 0..config.max_iterations {
        iterations = iter + 1;

        // x_new = α · A^T · x + β · 1
        let mut x_new = vec![config.beta; n];
        for i in 0..n {
            let rng = csr.offsets[i]..csr.offsets[i + 1];
            for k in rng {
                let j = csr.targets[k];
                let w = csr.weights[k];
                x_new[j] += config.alpha * x[i] * w;
            }
        }

        // 收敛检测
        let mut diff = 0.0;
        for i in 0..n {
            diff += (x_new[i] - x[i]).abs();
        }

        x = x_new;

        if diff < config.tolerance {
            converged = true;
            break;
        }
    }

    // 归一化（L2 范数）
    let norm: f64 = x.iter().map(|v| v * v).sum::<f64>().sqrt();
    if norm > 1e-15 {
        for v in x.iter_mut() {
            *v /= norm;
        }
    }

    (x, converged, iterations)
}

// ============================================================================
// HITS 算法（权威度/枢纽度）
// ============================================================================

/// HITS 算法结果
#[derive(Debug, Clone)]
pub struct HITSResult {
    /// 权威度
    pub authority: Vec<f64>,
    /// 枢纽度
    pub hub: Vec<f64>,
    /// 是否收敛
    pub converged: bool,
    /// 迭代次数
    pub iterations: usize,
}

/// HITS 算法配置
#[derive(Debug, Clone)]
pub struct HITSConfig {
    /// 最大迭代次数
    pub max_iterations: usize,
    /// 收敛阈值
    pub tolerance: f64,
}

impl Default for HITSConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-6,
        }
    }
}

/// HITS（Hyperlink-Induced Topic Search）算法
///
/// 同时计算每个节点的权威度（Authority）和枢纽度（Hub）：
/// - 权威度高：被许多高枢纽度节点指向
/// - 枢纽度高：指向许多高权威度节点
///
/// # 算法原理
/// 1. 初始化 a = 1, h = 1
/// 2. 迭代：
///    - a ← A^T · h（权威度 = 入邻居的枢纽度之和）
///    - h ← A · a（枢纽度 = 出邻居的权威度之和）
/// 3. 每轮归一化 a 和 h
/// 4. 收敛后输出
pub(crate) fn hits_centrality_csr(csr: &CsrAdj, config: &HITSConfig) -> HITSResult {
    let n = csr.n;
    if n == 0 {
        return HITSResult {
            authority: Vec::new(),
            hub: Vec::new(),
            converged: true,
            iterations: 0,
        };
    }

    let mut authority = vec![1.0f64 / (n as f64).sqrt(); n];
    let mut hub = vec![1.0f64 / (n as f64).sqrt(); n];
    let mut converged = false;
    let mut iterations = 0;

    // 构建入边 CSR（用于高效计算 A^T · x）
    let mut in_offsets = vec![0usize; n + 1];
    let mut in_sources = Vec::with_capacity(csr.targets.len());
    let mut in_weights = Vec::with_capacity(csr.targets.len());
    let mut in_deg = vec![0usize; n];
    for i in 0..n {
        let rng = csr.offsets[i]..csr.offsets[i + 1];
        for k in rng {
            let j = csr.targets[k];
            in_deg[j] += 1;
        }
    }
    for i in 0..n {
        in_offsets[i + 1] = in_offsets[i] + in_deg[i];
    }
    let mut curs = in_offsets[0..n].to_vec();
    for i in 0..n {
        let rng = csr.offsets[i]..csr.offsets[i + 1];
        for k in rng {
            let j = csr.targets[k];
            let slot = curs[j];
            curs[j] += 1;
            in_sources.push(i);
            in_weights.push(csr.weights[k]);
        }
    }

    for iter in 0..config.max_iterations {
        iterations = iter + 1;

        // 计算权威度: a = A^T · h（入边加权和）
        let mut new_authority = vec![0.0f64; n];
        for j in 0..n {
            let rng = in_offsets[j]..in_offsets[j + 1];
            for k in rng {
                let i = in_sources[k];
                let w = in_weights[k];
                new_authority[j] += hub[i] * w;
            }
        }

        // 归一化权威度
        let a_norm: f64 = new_authority.iter().map(|v| v * v).sum::<f64>().sqrt();
        if a_norm > 1e-15 {
            for v in new_authority.iter_mut() {
                *v /= a_norm;
            }
        }

        // 计算枢纽度: h = A · a（出边加权和）
        let mut new_hub = vec![0.0f64; n];
        for i in 0..n {
            let rng = csr.offsets[i]..csr.offsets[i + 1];
            for k in rng {
                let j = csr.targets[k];
                let w = csr.weights[k];
                new_hub[i] += new_authority[j] * w;
            }
        }

        // 归一化枢纽度
        let h_norm: f64 = new_hub.iter().map(|v| v * v).sum::<f64>().sqrt();
        if h_norm > 1e-15 {
            for v in new_hub.iter_mut() {
                *v /= h_norm;
            }
        }

        // 收敛检测
        let mut diff = 0.0;
        for i in 0..n {
            diff += (new_authority[i] - authority[i]).abs();
            diff += (new_hub[i] - hub[i]).abs();
        }

        authority = new_authority;
        hub = new_hub;

        if diff < config.tolerance {
            converged = true;
            break;
        }
    }

    HITSResult {
        authority,
        hub,
        converged,
        iterations,
    }
}

// ============================================================================
// 信息中心性（Information Centrality）
// ============================================================================

/// 信息中心性
///
/// 基于图拉普拉斯矩阵的 Moore-Penrose 伪逆。
/// 衡量节点在信息传播中的重要性。
///
/// 公式：C_I(v) = N / Σ_{u≠v} (l_vv + l_uu - 2l_vu)
/// 其中 L+ 是拉普拉斯矩阵的伪逆，l_ij 是其第 (i,j) 个元素。
///
/// 注意：适用于小规模连通图，时间复杂度 O(V³)。
pub(crate) fn information_centrality_csr(csr: &CsrAdj) -> Vec<f64> {
    let n = csr.n;
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![0.0];
    }

    // 构建无向拉普拉斯矩阵（对称化）
    // L = D - A，其中 D 是度数矩阵，A 是无向邻接矩阵
    let mut laplacian = vec![vec![0.0f64; n]; n];

    // 构建无向邻接矩阵（对称化）
    let mut adj_undir = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        let rng = csr.offsets[i]..csr.offsets[i + 1];
        for k in rng {
            let j = csr.targets[k];
            let w = csr.weights[k];
            adj_undir[i][j] = adj_undir[i][j].max(w);
            adj_undir[j][i] = adj_undir[j][i].max(w);
        }
    }

    // 构建拉普拉斯矩阵
    for i in 0..n {
        let mut deg = 0.0;
        for j in 0..n {
            deg += adj_undir[i][j];
        }
        laplacian[i][i] = deg;
        for j in 0..n {
            if i != j {
                laplacian[i][j] = -adj_undir[i][j];
            }
        }
    }

    // 计算 Moore-Penrose 伪逆
    // 使用特征分解：L = Q·Λ·Q^T，L+ = Q·Λ+·Q^T
    // 简化实现：用高斯-若尔当消去 + 正则化
    let l_plus = pseudo_inverse(&laplacian, n);

    // 计算信息中心性
    let mut result = vec![0.0f64; n];
    let nf = n as f64;

    for v in 0..n {
        let mut sum = 0.0;
        for u in 0..n {
            if u != v {
                let distance = l_plus[v][v] + l_plus[u][u] - 2.0 * l_plus[v][u];
                if distance > 1e-15 {
                    sum += 1.0 / distance;
                }
            }
        }
        // 调和平均
        result[v] = sum / nf;
    }

    result
}

/// 对称矩阵的 Moore-Penrose 伪逆（简化实现）
///
/// 通过特征分解计算：L+ = Q · Λ+ · Q^T
/// 对于小规模矩阵（n < 100）可接受。
fn pseudo_inverse(matrix: &[Vec<f64>], n: usize) -> Vec<Vec<f64>> {
    // 使用幂迭代 + 消去法求前 n-1 个特征对（拉普拉斯有一个零特征值）
    // 简化实现：用雅可比法或迭代法
    // 这里用一个更简单的近似：正则化逆

    // 添加小正则化项，使矩阵可逆
    let eps = 1e-10;
    let mut reg = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in 0..n {
            reg[i][j] = matrix[i][j];
        }
        reg[i][i] += eps;
    }

    // 高斯消去求逆
    let mut aug = vec![vec![0.0f64; 2 * n]; n];
    for i in 0..n {
        for j in 0..n {
            aug[i][j] = reg[i][j];
        }
        aug[i][n + i] = 1.0;
    }

    // 前向消去
    for i in 0..n {
        // 选主元
        let mut max_row = i;
        let mut max_val = aug[i][i].abs();
        for k in i + 1..n {
            if aug[k][i].abs() > max_val {
                max_val = aug[k][i].abs();
                max_row = k;
            }
        }
        if max_val < 1e-15 {
            continue;
        }
        aug.swap(i, max_row);

        // 消去
        let pivot = aug[i][i];
        for j in i..2 * n {
            aug[i][j] /= pivot;
        }
        for k in 0..n {
            if k != i {
                let factor = aug[k][i];
                if factor.abs() > 1e-15 {
                    for j in i..2 * n {
                        aug[k][j] -= factor * aug[i][j];
                    }
                }
            }
        }
    }

    // 提取逆矩阵
    let mut inv = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in 0..n {
            inv[i][j] = aug[i][n + j];
        }
    }

    // 减去零空间投影（修正伪逆）
    // 零空间向量是全 1 向量（归一化）
    let ones_norm = (n as f64).sqrt();
    for i in 0..n {
        for j in 0..n {
            inv[i][j] -= 1.0 / (ones_norm * ones_norm);
        }
    }

    inv
}

// ============================================================================
// 局部中心性（Local Centrality）
// ============================================================================

/// 局部中心性类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalCentralityType {
    /// 邻居数（度）
    Degree,
    /// 邻居的度之和（二阶度）
    NeighborDegreeSum,
    /// 邻居的平均度
    NeighborDegreeAvg,
    /// 局部聚类系数
    ClusteringCoefficient,
}

/// 局部中心性计算
///
/// 快速计算节点的局部影响力，不需要全局遍历。
/// 适用于大规模图的近似中心性估计。
pub(crate) fn local_centrality_csr(
    csr: &CsrAdj,
    ltype: LocalCentralityType,
) -> Vec<f64> {
    let n = csr.n;
    if n == 0 {
        return Vec::new();
    }

    // 计算每个节点的出度
    let out_deg: Vec<usize> = (0..n)
        .map(|i| csr.offsets[i + 1] - csr.offsets[i])
        .collect();

    match ltype {
        LocalCentralityType::Degree => {
            out_deg.iter().map(|&d| d as f64).collect()
        }
        LocalCentralityType::NeighborDegreeSum => {
            let mut result = vec![0.0f64; n];
            for i in 0..n {
                let rng = csr.offsets[i]..csr.offsets[i + 1];
                for k in rng {
                    let j = csr.targets[k];
                    result[i] += out_deg[j] as f64;
                }
            }
            result
        }
        LocalCentralityType::NeighborDegreeAvg => {
            let mut result = vec![0.0f64; n];
            for i in 0..n {
                let deg = out_deg[i];
                if deg == 0 {
                    result[i] = 0.0;
                    continue;
                }
                let rng = csr.offsets[i]..csr.offsets[i + 1];
                let mut sum = 0.0;
                for k in rng {
                    let j = csr.targets[k];
                    sum += out_deg[j] as f64;
                }
                result[i] = sum / deg as f64;
            }
            result
        }
        LocalCentralityType::ClusteringCoefficient => {
            // 局部聚类系数：实际三角形数 / 可能三角形数
            // 对于有向图，使用无向化版本
            let mut result = vec![0.0f64; n];

            // 构建无向邻居集合
            let mut neighbors: Vec<std::collections::HashSet<usize>> =
                (0..n).map(|_| std::collections::HashSet::new()).collect();
            for i in 0..n {
                let rng = csr.offsets[i]..csr.offsets[i + 1];
                for k in rng {
                    let j = csr.targets[k];
                    if i != j {
                        neighbors[i].insert(j);
                        neighbors[j].insert(i);
                    }
                }
            }

            for i in 0..n {
                let deg = neighbors[i].len();
                if deg < 2 {
                    result[i] = 0.0;
                    continue;
                }

                // 计算邻居之间的边数
                let mut triangles = 0;
                let neighbor_list: Vec<usize> = neighbors[i].iter().copied().collect();
                for a in 0..neighbor_list.len() {
                    for b in a + 1..neighbor_list.len() {
                        if neighbors[neighbor_list[a]].contains(&neighbor_list[b]) {
                            triangles += 1;
                        }
                    }
                }

                let max_triangles = deg * (deg - 1) / 2;
                result[i] = if max_triangles > 0 {
                    triangles as f64 / max_triangles as f64
                } else {
                    0.0
                };
            }

            result
        }
    }
}

// ============================================================================
// KnowledgeGraph 扩展方法
// ============================================================================

impl KnowledgeGraph {
    /// 接近中心性（支持标准和调和两种类型）
    pub fn closeness_centrality_extended(
        &self,
        ctype: ClosenessType,
    ) -> HashMap<String, f64> {
        let csr = CsrAdj::from_graph(&self.graph);
        let values = closeness_centrality_csr(&csr, ctype);
        crate::csr::rank_vec_to_map(&values, &self.node_map)
    }

    /// 特征向量中心性（幂迭代法）
    pub fn eigenvector_centrality(&self, config: Option<EigenvectorConfig>) -> HashMap<String, f64> {
        let cfg = config.unwrap_or_default();
        let csr = CsrAdj::from_graph(&self.graph);
        let (values, _, _) = eigenvector_centrality_csr(&csr, &cfg);
        crate::csr::rank_vec_to_map(&values, &self.node_map)
    }

    /// Katz 中心性
    pub fn katz_centrality(&self, config: Option<KatzConfig>) -> HashMap<String, f64> {
        let cfg = config.unwrap_or_default();
        let csr = CsrAdj::from_graph(&self.graph);
        let (values, _, _) = katz_centrality_csr(&csr, &cfg);
        crate::csr::rank_vec_to_map(&values, &self.node_map)
    }

    /// HITS 算法（权威度 + 枢纽度）
    pub fn hits_centrality(
        &self,
        config: Option<HITSConfig>,
    ) -> (HashMap<String, f64>, HashMap<String, f64>) {
        let cfg = config.unwrap_or_default();
        let csr = CsrAdj::from_graph(&self.graph);
        let result = hits_centrality_csr(&csr, &cfg);
        let authority = crate::csr::rank_vec_to_map(&result.authority, &self.node_map);
        let hub = crate::csr::rank_vec_to_map(&result.hub, &self.node_map);
        (authority, hub)
    }

    /// 信息中心性
    ///
    /// 适用于小规模图（节点数 < 100），时间复杂度 O(V³)。
    pub fn information_centrality(&self) -> HashMap<String, f64> {
        let csr = CsrAdj::from_graph(&self.graph);
        let values = information_centrality_csr(&csr);
        crate::csr::rank_vec_to_map(&values, &self.node_map)
    }

    /// 局部中心性
    pub fn local_centrality(&self, ltype: LocalCentralityType) -> HashMap<String, f64> {
        let csr = CsrAdj::from_graph(&self.graph);
        let values = local_centrality_csr(&csr, ltype);
        crate::csr::rank_vec_to_map(&values, &self.node_map)
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

    fn build_test_graph() -> KnowledgeGraph {
        KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_node("c", "C", "test")
            .add_node("d", "D", "test")
            .add_node("e", "E", "test")
            .add_edge("a", "b", 1.0)
            .add_edge("a", "c", 1.0)
            .add_edge("b", "c", 1.0)
            .add_edge("c", "d", 1.0)
            .add_edge("d", "e", 1.0)
            .build()
    }

    #[test]
    fn test_closeness_standard() {
        let graph = build_test_graph();
        let result = graph.closeness_centrality_extended(ClosenessType::Standard);
        assert_eq!(result.len(), 5);

        // a 能到达更多节点（b, c, d, e），接近中心性应高于 c（只能到达 d, e）
        assert!(result["a"] > result["c"]);
        assert!(result["c"] > result["e"]);
    }

    #[test]
    fn test_closeness_harmonic() {
        let graph = build_test_graph();
        let result = graph.closeness_centrality_extended(ClosenessType::Harmonic);
        assert_eq!(result.len(), 5);

        // 所有值应在 [0, 1] 范围内
        for &v in result.values() {
            assert!(v >= 0.0 && v <= 1.0 + 1e-9);
        }
    }

    #[test]
    fn test_eigenvector_centrality() {
        // 使用有环的图（特征向量中心性在强连通图上有意义）
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_node("c", "C", "test")
            .add_node("d", "D", "test")
            .add_edge("a", "b", 1.0)
            .add_edge("b", "c", 1.0)
            .add_edge("c", "a", 1.0) // 形成环
            .add_edge("c", "d", 1.0)
            .add_edge("d", "c", 1.0) // c-d 双向
            .build();

        let result = graph.eigenvector_centrality(None);
        assert_eq!(result.len(), 4);

        // c 连接最多（入边来自 b 和 d，以及 a 通过环），应较高
        assert!(result["c"] > result["d"]);

        // L2 范数应为 1（已归一化）
        let norm: f64 = result.values().map(|v| v * v).sum::<f64>().sqrt();
        assert_relative_eq!(norm, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_katz_centrality() {
        let graph = build_test_graph();
        let config = KatzConfig {
            alpha: 0.1,
            beta: 1.0,
            ..Default::default()
        };
        let result = graph.katz_centrality(Some(config));
        assert_eq!(result.len(), 5);

        // 所有值应为正
        for &v in result.values() {
            assert!(v >= 0.0);
        }
    }

    #[test]
    fn test_hits_centrality() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_node("c", "C", "test")
            .add_node("d", "D", "test")
            .add_edge("a", "c", 1.0) // a 指向 c
            .add_edge("a", "d", 1.0) // a 指向 d
            .add_edge("b", "c", 1.0) // b 指向 c
            .add_edge("b", "d", 1.0) // b 指向 d
            .build();

        let (authority, hub) = graph.hits_centrality(None);
        assert_eq!(authority.len(), 4);
        assert_eq!(hub.len(), 4);

        // c 和 d 应该是高权威度节点
        assert!(authority["c"] > authority["a"]);
        assert!(authority["d"] > authority["b"]);

        // a 和 b 应该是高枢纽度节点
        assert!(hub["a"] > hub["c"]);
        assert!(hub["b"] > hub["d"]);
    }

    #[test]
    fn test_information_centrality() {
        // 小规模图测试
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_node("c", "C", "test")
            .add_edge("a", "b", 1.0)
            .add_edge("b", "c", 1.0)
            .add_edge("a", "c", 1.0)
            .build();

        let result = graph.information_centrality();
        assert_eq!(result.len(), 3);

        // 所有值应为非负
        for &v in result.values() {
            assert!(v >= 0.0 || v.abs() < 1e-9);
        }
    }

    #[test]
    fn test_local_centrality_degree() {
        let graph = build_test_graph();
        let result = graph.local_centrality(LocalCentralityType::Degree);
        assert_eq!(result["a"], 2.0); // a -> b, a -> c
        assert_eq!(result["c"], 1.0); // c -> d
    }

    #[test]
    fn test_local_centrality_neighbor_degree_sum() {
        let graph = build_test_graph();
        let result = graph.local_centrality(LocalCentralityType::NeighborDegreeSum);
        // a 的邻居：b(1) + c(1) = 2
        assert_eq!(result["a"], 2.0);
    }

    #[test]
    fn test_local_centrality_clustering() {
        let graph = build_test_graph();
        let result = graph.local_centrality(LocalCentralityType::ClusteringCoefficient);

        // a 的邻居：b, c；b 和 c 之间有边 → 聚类系数 = 1
        // 但这是有向图，无向化后 b-c 是否有边？
        // 边: a->b, a->c, b->c, c->d, d->e
        // 无向化后: a-b, a-c, b-c, c-d, d-e
        // a 的邻居: b, c → b-c 有边 → 1个三角形 / 1个可能 = 1.0
        assert_relative_eq!(result["a"], 1.0, epsilon = 1e-9);
    }

    #[test]
    fn test_eigenvector_convergence() {
        let graph = build_test_graph();
        let csr = CsrAdj::from_graph(&graph.graph);
        let config = EigenvectorConfig {
            max_iterations: 100,
            tolerance: 1e-8,
        };
        let (_, converged, iters) = eigenvector_centrality_csr(&csr, &config);
        assert!(converged);
        assert!(iters < 100);
    }

    #[test]
    fn test_hits_convergence() {
        let graph = build_test_graph();
        let csr = CsrAdj::from_graph(&graph.graph);
        let config = HITSConfig {
            max_iterations: 100,
            tolerance: 1e-8,
        };
        let result = hits_centrality_csr(&csr, &config);
        assert!(result.converged);
        assert!(result.iterations < 100);
    }

    #[test]
    fn test_empty_graph_centrality() {
        let graph = KnowledgeGraph::new();
        let result = graph.eigenvector_centrality(None);
        assert!(result.is_empty());

        let (auth, hub) = graph.hits_centrality(None);
        assert!(auth.is_empty());
        assert!(hub.is_empty());
    }
}
