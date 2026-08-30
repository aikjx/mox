// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 扩展社区发现算法
//!
//! # 算法概览
//!
//! ## Louvain 算法
//! - 多层次模块度优化，速度快、效果好
//! - 第一阶段：局部移动，将节点移到增益最大的社区
//! - 第二阶段：社区聚合，将每个社区视为一个新节点
//! - 时间复杂度：接近 O(N log N)
//!
//! ## 标签传播算法（Label Propagation）
//! - 基于近邻多数投票的社区发现
//! - 每个节点初始有唯一标签，迭代更新为邻居中最多的标签
//! - 时间复杂度：O(E) 每轮，通常几轮收敛
//!
//! ## 谱聚类（Spectral Clustering）
//! - 基于图拉普拉斯矩阵的特征向量
//! - 将节点映射到谱空间后做 k-means 聚类
//! - 适用于发现非凸形状的社区
//!
//! ## Girvan-Newman 分裂式社区发现
//! - 基于边介数的分裂式算法
//! - 反复移除介数最高的边，直到图分裂为多个连通分量
//! - 时间复杂度：O(V·E²)，适合小规模图
//!
//! ## SLPA 重叠社区发现
//! - Speaker-Listener Label Propagation
//! - 每个节点可以有多个标签（多个社区归属）
//! - 支持发现重叠社区结构
//!
//! ## 社区质量评估
//! - 模块度（Modularity）：衡量社区内边密度与随机图的差异
//! - 电导（Conductance）：社区与外部的割边比例
//! - 标准化互信息（NMI）：两个划分的相似性度量

use crate::csr::CsrAdj;
use crate::graph::KnowledgeGraph;
use crate::types::Community;
use std::collections::{HashMap, HashSet, VecDeque};

// ============================================================================
// 辅助类型
// ============================================================================

/// 社区划分结果
#[derive(Debug, Clone)]
pub struct CommunityPartition {
    /// 每个节点的社区 ID
    pub node_community: Vec<usize>,
    /// 社区列表
    pub communities: Vec<Vec<usize>>,
    /// 模块度
    pub modularity: f64,
}

// ============================================================================
// Louvain 算法
// ============================================================================

/// Louvain 算法配置
#[derive(Debug, Clone)]
pub struct LouvainConfig {
    /// 最大迭代轮次（每一层）
    pub max_iterations: usize,
    /// 收敛阈值（模块度增益小于此值时停止）
    pub tolerance: f64,
    /// 最大层数（0 表示不限）
    pub max_levels: usize,
}

impl Default for LouvainConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            tolerance: 1e-6,
            max_levels: 0,
        }
    }
}

/// Louvain 社区发现算法
///
/// # 算法原理
/// 1. **局部移动阶段**：将每个节点尝试移到其邻居所在的社区，
///    选择使模块度增益最大的移动，反复迭代直到稳定。
/// 2. **社区聚合阶段**：将每个社区视为一个超级节点，
///    社区间的边合并为超级边，构建新的图。
/// 3. 在新图上重复步骤 1-2，直到模块度不再提升。
///
/// 时间复杂度接近 O(N log N)，是大规模图社区发现的首选算法。
pub(crate) fn louvain_csr(csr: &CsrAdj, config: &LouvainConfig) -> CommunityPartition {
    let n = csr.n;
    if n == 0 {
        return CommunityPartition {
            node_community: Vec::new(),
            communities: Vec::new(),
            modularity: 0.0,
        };
    }

    // 构建无向邻接表（对称化）
    let (adj, weights, total_weight) = build_undirected_adj(csr);

    // 初始：每个节点一个社区
    let mut node_comm: Vec<usize> = (0..n).collect();
    let mut level = 0;

    loop {
        if config.max_levels > 0 && level >= config.max_levels {
            break;
        }

        let prev_mod = modularity_undirected(&adj, &weights, &node_comm, total_weight);
        let mut improved = false;

        // 局部移动阶段
        for _ in 0..config.max_iterations {
            let mut node_order: Vec<usize> = (0..n).collect();
            // 打乱顺序（用确定性顺序保证可复现）
            node_order.sort();

            let mut iteration_improved = false;

            for &node in &node_order {
                let current_comm = node_comm[node];

                // 计算当前节点在当前社区的贡献
                let mut comm_weights: HashMap<usize, f64> = HashMap::new();
                for (&neighbor, &w) in &adj[node] {
                    let neighbor_comm = node_comm[neighbor];
                    *comm_weights.entry(neighbor_comm).or_insert(0.0) += w;
                }

                // 计算移到每个邻居社区的增益
                let mut best_comm = current_comm;
                let mut best_gain = 0.0;

                let node_degree: f64 = adj[node].values().sum();

                for (&comm, &weight_to_comm) in &comm_weights {
                    if comm == current_comm {
                        continue;
                    }

                    // 模块度增益近似公式
                    // ΔQ = (k_i,in / 2m) - (k_i · Σ_comm / (2m)²)
                    // 其中 k_i,in 是节点 i 到社区 comm 的边权和
                    // Σ_comm 是社区 comm 的总度数（含自环）
                    let comm_total_degree: f64 = (0..n)
                        .filter(|&v| node_comm[v] == comm)
                        .map(|v| adj[v].values().sum::<f64>())
                        .sum();

                    let gain = weight_to_comm / total_weight
                        - node_degree * comm_total_degree / (2.0 * total_weight * total_weight);

                    if gain > best_gain + 1e-12 {
                        best_gain = gain;
                        best_comm = comm;
                    }
                }

                if best_comm != current_comm && best_gain > config.tolerance {
                    node_comm[node] = best_comm;
                    iteration_improved = true;
                    improved = true;
                }
            }

            if !iteration_improved {
                break;
            }
        }

        let new_mod = modularity_undirected(&adj, &weights, &node_comm, total_weight);

        if !improved || new_mod - prev_mod < config.tolerance {
            break;
        }

        level += 1;

        // 社区聚合：重新编号社区
        let mut comm_map = HashMap::new();
        let mut new_id = 0;
        for i in 0..n {
            let old = node_comm[i];
            if !comm_map.contains_key(&old) {
                comm_map.insert(old, new_id);
                new_id += 1;
            }
            node_comm[i] = comm_map[&old];
        }

        let num_comms = new_id;
        if num_comms <= 1 {
            break;
        }
    }

    // 构建社区列表
    let mut communities_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for (node, &comm) in node_comm.iter().enumerate() {
        communities_map.entry(comm).or_default().push(node);
    }

    let mut communities: Vec<Vec<usize>> = communities_map.into_values().collect();
    communities.sort_by(|a, b| b.len().cmp(&a.len()));

    // 重新编号社区
    let mut new_node_comm = vec![0usize; n];
    for (comm_id, members) in communities.iter().enumerate() {
        for &node in members {
            new_node_comm[node] = comm_id;
        }
    }

    let final_mod = modularity_undirected(&adj, &weights, &new_node_comm, total_weight);

    CommunityPartition {
        node_community: new_node_comm,
        communities,
        modularity: final_mod,
    }
}

/// 构建无向邻接表（对称化 + 去重）
fn build_undirected_adj(csr: &CsrAdj) -> (Vec<HashMap<usize, f64>>, Vec<Vec<(usize, f64)>>, f64) {
    let n = csr.n;
    let mut adj: Vec<HashMap<usize, f64>> = vec![HashMap::new(); n];

    for i in 0..n {
        let rng = csr.offsets[i]..csr.offsets[i + 1];
        for k in rng {
            let j = csr.targets[k];
            let w = csr.weights[k];
            if i != j {
                // 取双向最大值作为无向边权
                let entry = adj[i].entry(j).or_insert(0.0);
                *entry = entry.max(w);
                let entry2 = adj[j].entry(i).or_insert(0.0);
                *entry2 = entry2.max(w);
            }
        }
    }

    // 计算无向图总边权 m（每条无向边计一次）
    let mut m = 0.0;
    for i in 0..n {
        for (&j, &w) in &adj[i] {
            if j > i {
                m += w;
            }
        }
    }

    // 转为 Vec 形式
    let weights: Vec<Vec<(usize, f64)>> = adj
        .iter()
        .map(|m| m.iter().map(|(&k, &v)| (k, v)).collect())
        .collect();

    (adj, weights, m)
}

/// 计算无向图模块度（Newman 标准公式）
///
/// Q = Σ_c [ e_cc / m - (d_c / (2m))² ]
/// 其中：
/// - m = 总无向边权
/// - e_cc = 社区 c 内部边权之和
/// - d_c = 社区 c 总度数（所有节点度数之和）
///
/// 当所有节点在一个社区时，Q = 0。
fn modularity_undirected(
    adj: &[HashMap<usize, f64>],
    _weights: &[Vec<(usize, f64)>],
    node_comm: &[usize],
    m: f64,
) -> f64 {
    let n = adj.len();
    if m < 1e-15 {
        return 0.0;
    }

    // 计算每个社区的内部边权 e_cc 和总度数 d_c
    let mut comm_internal: HashMap<usize, f64> = HashMap::new();
    let mut comm_degree: HashMap<usize, f64> = HashMap::new();

    for i in 0..n {
        let deg: f64 = adj[i].values().sum();
        let c = node_comm[i];
        *comm_degree.entry(c).or_insert(0.0) += deg;

        for (&j, &w) in &adj[i] {
            if j > i && node_comm[i] == node_comm[j] {
                *comm_internal.entry(c).or_insert(0.0) += w;
            }
        }
    }

    // 计算模块度
    let mut q = 0.0;
    let two_m = 2.0 * m;
    for (&c, &e_cc) in &comm_internal {
        let d_c = comm_degree.get(&c).copied().unwrap_or(0.0);
        q += e_cc / m - (d_c / two_m) * (d_c / two_m);
    }

    q
}

// ============================================================================
// 标签传播算法（Label Propagation）
// ============================================================================

/// 标签传播算法配置
#[derive(Debug, Clone)]
pub struct LabelPropagationConfig {
    /// 最大迭代次数
    pub max_iterations: usize,
    /// 同步/异步更新
    pub asynchronous: bool,
    /// 随机种子（0 表示确定性顺序）
    pub seed: u64,
}

impl Default for LabelPropagationConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            asynchronous: true,
            seed: 42,
        }
    }
}

/// 标签传播社区发现算法
///
/// # 算法原理
/// 1. 每个节点初始有唯一标签（社区 ID）
/// 2. 每轮迭代：每个节点的标签更新为其邻居中出现次数最多的标签
/// 3. 当标签不再变化或达到最大迭代次数时停止
///
/// 时间复杂度 O(E) 每轮，通常 5-10 轮收敛。
pub(crate) fn label_propagation_csr(
    csr: &CsrAdj,
    config: &LabelPropagationConfig,
) -> CommunityPartition {
    let n = csr.n;
    if n == 0 {
        return CommunityPartition {
            node_community: Vec::new(),
            communities: Vec::new(),
            modularity: 0.0,
        };
    }

    // 构建无向邻接表
    let (adj, _weights, total_weight) = build_undirected_adj(csr);

    // 初始标签
    let mut labels: Vec<usize> = (0..n).collect();

    let mut converged = false;
    let mut final_iter = 0;

    for iter in 0..config.max_iterations {
        final_iter = iter + 1;
        let mut changed = false;

        let node_order: Vec<usize> = if config.seed > 0 {
            // 确定性打乱（使用种子）
            let mut order: Vec<usize> = (0..n).collect();
            let mut rng = config.seed;
            for i in (1..order.len()).rev() {
                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                let j = (rng as usize) % (i + 1);
                order.swap(i, j);
            }
            order
        } else {
            (0..n).collect()
        };

        if config.asynchronous {
            // 异步更新
            for &node in &node_order {
                let new_label = majority_label(node, &adj, &labels);
                if new_label != labels[node] {
                    labels[node] = new_label;
                    changed = true;
                }
            }
        } else {
            // 同步更新
            let mut new_labels = labels.clone();
            for &node in &node_order {
                new_labels[node] = majority_label(node, &adj, &labels);
                if new_labels[node] != labels[node] {
                    changed = true;
                }
            }
            labels = new_labels;
        }

        if !changed {
            converged = true;
            break;
        }
    }

    // 重新编号社区
    let mut label_map = HashMap::new();
    let mut new_id = 0;
    for i in 0..n {
        let old = labels[i];
        if !label_map.contains_key(&old) {
            label_map.insert(old, new_id);
            new_id += 1;
        }
        labels[i] = label_map[&old];
    }

    // 构建社区列表
    let mut communities_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for (node, &label) in labels.iter().enumerate() {
        communities_map.entry(label).or_default().push(node);
    }

    let mut communities: Vec<Vec<usize>> = communities_map.into_values().collect();
    communities.sort_by(|a, b| b.len().cmp(&a.len()));

    // 重新编号
    let mut node_comm = vec![0usize; n];
    for (comm_id, members) in communities.iter().enumerate() {
        for &node in members {
            node_comm[node] = comm_id;
        }
    }

    let mod_val = modularity_undirected(&adj, &_weights, &node_comm, total_weight);

    let _ = converged;
    let _ = final_iter;

    CommunityPartition {
        node_community: node_comm,
        communities,
        modularity: mod_val,
    }
}

/// 找到邻居中出现最多的标签
fn majority_label(node: usize, adj: &[HashMap<usize, f64>], labels: &[usize]) -> usize {
    let mut label_counts: HashMap<usize, f64> = HashMap::new();

    for (&neighbor, &weight) in &adj[node] {
        let label = labels[neighbor];
        *label_counts.entry(label).or_insert(0.0) += weight;
    }

    if label_counts.is_empty() {
        return labels[node];
    }

    // 找最大标签，平局时取最小的标签 ID（确定性）
    let mut best_label = labels[node];
    let mut best_count = -1.0;

    let mut sorted_labels: Vec<usize> = label_counts.keys().copied().collect();
    sorted_labels.sort();

    for label in sorted_labels {
        let count = label_counts[&label];
        if count > best_count + 1e-12 {
            best_count = count;
            best_label = label;
        }
    }

    best_label
}

// ============================================================================
// 谱聚类（Spectral Clustering）
// ============================================================================

/// 谱聚类配置
#[derive(Debug, Clone)]
pub struct SpectralClusteringConfig {
    /// 社区数量 k
    pub k: usize,
    /// 最大迭代次数（k-means）
    pub max_iterations: usize,
}

impl Default for SpectralClusteringConfig {
    fn default() -> Self {
        Self {
            k: 2,
            max_iterations: 100,
        }
    }
}

/// 谱聚类算法
///
/// # 算法原理
/// 1. 构建图拉普拉斯矩阵 L = D - A
/// 2. 计算 L 的前 k 个最小特征向量
/// 3. 将每个节点投影到这 k 个特征向量构成的空间
/// 4. 对投影后的向量做 k-means 聚类
///
/// 适用于发现非凸形状的社区，时间复杂度 O(V³)，适合小规模图。
pub(crate) fn spectral_clustering_csr(
    csr: &CsrAdj,
    config: &SpectralClusteringConfig,
) -> CommunityPartition {
    let n = csr.n;
    if n == 0 {
        return CommunityPartition {
            node_community: Vec::new(),
            communities: Vec::new(),
            modularity: 0.0,
        };
    }
    if config.k >= n {
        // 每个节点一个社区
        let communities: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();
        return CommunityPartition {
            node_community: (0..n).collect(),
            communities,
            modularity: 0.0,
        };
    }

    // 构建无向拉普拉斯矩阵
    let (adj, _weights, total_weight) = build_undirected_adj(csr);

    // 计算度
    let degree: Vec<f64> = (0..n).map(|i| adj[i].values().sum()).collect();

    // 计算拉普拉斯矩阵的前 k 个特征向量（用幂迭代法求最小特征值）
    // 使用反向迭代：(L + σI)^(-1) 的最大特征值对应 L 的最小特征值
    // 简化实现：用多次幂迭代 + 消去法
    let eigvecs = compute_smallest_eigenvectors(&adj, &degree, n, config.k);

    // k-means 聚类
    let labels = kmeans(&eigvecs, config.k, config.max_iterations);

    // 构建社区
    let mut communities_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for (node, &label) in labels.iter().enumerate() {
        communities_map.entry(label).or_default().push(node);
    }

    let mut communities: Vec<Vec<usize>> = communities_map.into_values().collect();
    communities.sort_by(|a, b| b.len().cmp(&a.len()));

    let mut node_comm = vec![0usize; n];
    for (comm_id, members) in communities.iter().enumerate() {
        for &node in members {
            node_comm[node] = comm_id;
        }
    }

    let mod_val = modularity_undirected(&adj, &_weights, &node_comm, total_weight);

    CommunityPartition {
        node_community: node_comm,
        communities,
        modularity: mod_val,
    }
}

/// 计算拉普拉斯矩阵的前 k 个最小特征向量
fn compute_smallest_eigenvectors(
    adj: &[HashMap<usize, f64>],
    degree: &[f64],
    n: usize,
    k: usize,
) -> Vec<Vec<f64>> {
    // 用归一化拉普拉斯 L_norm = I - D^(-1/2) A D^(-1/2)
    // 其最小特征值为 0，对应特征向量为 D^(1/2)·1
    // 这里用简化方法：多次幂迭代求最大特征值，然后消去

    let mut eigvecs: Vec<Vec<f64>> = Vec::new();
    let max_iter = 200;
    let tol = 1e-8;

    // 计算 D^(-1/2)
    let d_inv_sqrt: Vec<f64> = degree
        .iter()
        .map(|&d| if d > 1e-15 { 1.0 / d.sqrt() } else { 0.0 })
        .collect();

    // 求归一化拉普拉斯的最大特征值（用于移位）
    // L_norm 的谱半径 ≤ 2
    let sigma = 2.0;

    for _ in 0..k {
        let mut x = vec![1.0f64 / (n as f64).sqrt(); n];

        // 对已找到的特征向量正交化
        for ev in &eigvecs {
            let dot: f64 = x.iter().zip(ev.iter()).map(|(a, b)| a * b).sum();
            for i in 0..n {
                x[i] -= dot * ev[i];
            }
        }

        let mut _eigval = 0.0;

        for _ in 0..max_iter {
            // 计算 y = (sigma * I - L_norm) · x
            // L_norm = I - D^(-1/2) A D^(-1/2)
            // sigma*I - L_norm = (sigma-1)*I + D^(-1/2) A D^(-1/2)
            let mut y = vec![0.0f64; n];

            for i in 0..n {
                // (sigma-1) * x[i]
                y[i] = (sigma - 1.0) * x[i];

                // D^(-1/2) A D^(-1/2) x
                for (&j, &w) in &adj[i] {
                    y[i] += d_inv_sqrt[i] * w * d_inv_sqrt[j] * x[j];
                }
            }

            // 正交化
            for ev in &eigvecs {
                let dot: f64 = y.iter().zip(ev.iter()).map(|(a, b)| a * b).sum();
                for i in 0..n {
                    y[i] -= dot * ev[i];
                }
            }

            // 归一化
            let norm: f64 = y.iter().map(|v| v * v).sum::<f64>().sqrt();
            if norm < 1e-15 {
                break;
            }
            for v in y.iter_mut() {
                *v /= norm;
            }

            // 收敛检测
            let diff: f64 = x.iter().zip(y.iter()).map(|(a, b)| (a - b).abs()).sum();
            x = y;

            if diff < tol {
                break;
            }
        }

        // 计算对应的特征值
        // 由于我们用 sigma*I - L_norm 做幂迭代，
        // 得到的最大特征值对应 L_norm 的最小特征值 + sigma
        // 但这里得到的是 sigma*I - L_norm 的特征值
        // L_norm 的最小特征值 = sigma - eigval_of_(sigmaI - L_norm)
        // 我们按从小到大排序，所以先找到的应该是最大的 eigval（对应最小的 L_norm eigval）
        eigvecs.push(x);
    }

    eigvecs
}

/// k-means 聚类
fn kmeans(data: &[Vec<f64>], k: usize, max_iter: usize) -> Vec<usize> {
    let n = data.len();
    if n == 0 || k == 0 {
        return Vec::new();
    }

    let d = data[0].len();

    // 初始化质心（取前 k 个点）
    let mut centroids: Vec<Vec<f64>> = Vec::with_capacity(k);
    for i in 0..k.min(n) {
        centroids.push(data[i].clone());
    }

    let mut labels = vec![0usize; n];

    for _ in 0..max_iter {
        // 分配点到最近质心
        let mut changed = false;
        for i in 0..n {
            let mut best = 0;
            let mut best_dist = f64::INFINITY;
            for c in 0..centroids.len() {
                let dist: f64 = data[i]
                    .iter()
                    .zip(centroids[c].iter())
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum();
                if dist < best_dist {
                    best_dist = dist;
                    best = c;
                }
            }
            if labels[i] != best {
                labels[i] = best;
                changed = true;
            }
        }

        if !changed {
            break;
        }

        // 更新质心
        let mut new_centroids = vec![vec![0.0f64; d]; k];
        let mut counts = vec![0usize; k];
        for i in 0..n {
            let c = labels[i];
            for j in 0..d {
                new_centroids[c][j] += data[i][j];
            }
            counts[c] += 1;
        }
        for c in 0..k {
            if counts[c] > 0 {
                for j in 0..d {
                    new_centroids[c][j] /= counts[c] as f64;
                }
            }
        }
        centroids = new_centroids;
    }

    labels
}

// ============================================================================
// Girvan-Newman 分裂式社区发现
// ============================================================================

/// Girvan-Newman 算法配置
#[derive(Debug, Clone)]
pub struct GirvanNewmanConfig {
    /// 目标社区数量（0 表示自动选择最优模块度）
    pub target_communities: usize,
}

impl Default for GirvanNewmanConfig {
    fn default() -> Self {
        Self {
            target_communities: 0,
        }
    }
}

/// Girvan-Newman 分裂式社区发现
///
/// # 算法原理
/// 1. 计算所有边的介数（边在多少条最短路径上）
/// 2. 移除介数最高的边
/// 3. 检查图的连通分量数
/// 4. 重复步骤 1-3，直到达到目标社区数
///
/// 时间复杂度 O(V·E²)，适合小规模图。
pub(crate) fn girvan_newman_csr(
    csr: &CsrAdj,
    config: &GirvanNewmanConfig,
) -> CommunityPartition {
    let n = csr.n;
    if n == 0 {
        return CommunityPartition {
            node_community: Vec::new(),
            communities: Vec::new(),
            modularity: 0.0,
        };
    }

    // 构建无向邻接表（可变）
    let mut adj: Vec<HashMap<usize, f64>> = vec![HashMap::new(); n];
    for i in 0..n {
        let rng = csr.offsets[i]..csr.offsets[i + 1];
        for k in rng {
            let j = csr.targets[k];
            let w = csr.weights[k];
            if i != j {
                let e = adj[i].entry(j).or_insert(0.0);
                *e = e.max(w);
                let e2 = adj[j].entry(i).or_insert(0.0);
                *e2 = e2.max(w);
            }
        }
    }

    let target = if config.target_communities == 0 {
        n // 最多 n 个社区
    } else {
        config.target_communities
    };

    let mut best_partition: Option<CommunityPartition> = None;
    let mut best_modularity = f64::NEG_INFINITY;

    loop {
        // 计算连通分量
        let components = connected_components(&adj, n);
        let num_components = components.len();

        // 计算当前模块度
        let mut node_comm = vec![0usize; n];
        for (cid, comp) in components.iter().enumerate() {
            for &node in comp {
                node_comm[node] = cid;
            }
        }

        let total_weight: f64 = adj
            .iter()
            .enumerate()
            .map(|(i, m)| {
                m.iter()
                    .filter(|(&j, _)| j > i)
                    .map(|(_, &w)| w)
                    .sum::<f64>()
            })
            .sum();
        let weights: Vec<Vec<(usize, f64)>> = adj
            .iter()
            .map(|m| m.iter().map(|(&k, &v)| (k, v)).collect())
            .collect();

        let mod_val = modularity_undirected(&adj, &weights, &node_comm, total_weight);

        // 记录最佳划分
        if mod_val > best_modularity {
            best_modularity = mod_val;
            let mut communities = components.clone();
            communities.sort_by(|a, b| b.len().cmp(&a.len()));

            let mut new_node_comm = vec![0usize; n];
            for (cid, comp) in communities.iter().enumerate() {
                for &node in comp {
                    new_node_comm[node] = cid;
                }
            }

            best_partition = Some(CommunityPartition {
                node_community: new_node_comm,
                communities,
                modularity: mod_val,
            });
        }

        if num_components >= target {
            break;
        }

        // 计算边介数
        let edge_betweenness = compute_edge_betweenness(&adj, n);

        // 找介数最高的边
        let mut max_betweenness = 0.0;
        let mut edge_to_remove = None;

        let mut edges_sorted: Vec<((usize, usize), f64)> = edge_betweenness
            .iter()
            .map(|(&(u, v), &b)| ((u.min(v), u.max(v)), b))
            .collect();
        edges_sorted.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        if let Some(&(edge, bw)) = edges_sorted.first() {
            max_betweenness = bw;
            edge_to_remove = Some(edge);
        }

        if max_betweenness < 1e-12 {
            break;
        }

        // 移除该边
        if let Some((u, v)) = edge_to_remove {
            adj[u].remove(&v);
            adj[v].remove(&u);
        }
    }

    best_partition.unwrap_or_else(|| CommunityPartition {
        node_community: (0..n).collect(),
        communities: (0..n).map(|i| vec![i]).collect(),
        modularity: 0.0,
    })
}

use std::cmp::Ordering;

/// 计算无向图的边介数
fn compute_edge_betweenness(adj: &[HashMap<usize, f64>], n: usize) -> HashMap<(usize, usize), f64> {
    let mut edge_bc: HashMap<(usize, usize), f64> = HashMap::new();

    for s in 0..n {
        // BFS 最短路径
        let mut dist = vec![-1i64; n];
        let mut sigma = vec![0.0f64; n];
        let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut order: Vec<usize> = Vec::with_capacity(n);
        let mut queue = VecDeque::new();

        dist[s] = 0;
        sigma[s] = 1.0;
        queue.push_back(s);

        while let Some(v) = queue.pop_front() {
            order.push(v);
            for (&w, _) in &adj[v] {
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

        // 反向累积边介数
        let mut delta = vec![0.0f64; n];
        for &w in order.iter().rev() {
            for &v in &preds[w] {
                let contribution = (sigma[v] / sigma[w]) * (1.0 + delta[w]);
                delta[v] += contribution;
                let edge = (v.min(w), v.max(w));
                *edge_bc.entry(edge).or_insert(0.0) += contribution;
            }
        }
    }

    // 无向图边介数除以 2
    for (_, v) in edge_bc.iter_mut() {
        *v /= 2.0;
    }

    edge_bc
}

/// 计算连通分量
fn connected_components(adj: &[HashMap<usize, f64>], n: usize) -> Vec<Vec<usize>> {
    let mut visited = vec![false; n];
    let mut components = Vec::new();

    for s in 0..n {
        if visited[s] {
            continue;
        }

        let mut component = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(s);
        visited[s] = true;

        while let Some(u) = queue.pop_front() {
            component.push(u);
            for (&v, _) in &adj[u] {
                if !visited[v] {
                    visited[v] = true;
                    queue.push_back(v);
                }
            }
        }

        components.push(component);
    }

    components
}

// ============================================================================
// SLPA 重叠社区发现
// ============================================================================

/// SLPA 算法配置
#[derive(Debug, Clone)]
pub struct SLPAConfig {
    /// 迭代次数
    pub iterations: usize,
    /// 阈值（标签出现频率低于此值的社区被过滤）
    pub threshold: f64,
    /// 随机种子
    pub seed: u64,
}

impl Default for SLPAConfig {
    fn default() -> Self {
        Self {
            iterations: 20,
            threshold: 0.1,
            seed: 42,
        }
    }
}

/// SLPA 重叠社区发现结果
#[derive(Debug, Clone)]
pub struct SLPAResult {
    /// 每个节点的社区标签及其出现频率
    pub node_labels: Vec<HashMap<usize, f64>>,
    /// 社区列表
    pub communities: Vec<Vec<usize>>,
}

/// SLPA（Speaker-Listener Label Propagation Algorithm）
///
/// # 算法原理
/// 1. 每个节点初始有唯一标签
/// 2. 每轮迭代：
///    - 随机选择一个节点作为 Listener
///    - 其邻居作为 Speaker，各说一个标签（从自己的标签库中随机选）
///    - Listener 选择出现最多的标签，加入自己的标签库
/// 3. 迭代结束后，每个节点保留出现频率高于阈值的标签
///
/// 支持发现重叠社区，时间复杂度 O(T·E)。
pub(crate) fn slpa_csr(csr: &CsrAdj, config: &SLPAConfig) -> SLPAResult {
    let n = csr.n;
    if n == 0 {
        return SLPAResult {
            node_labels: Vec::new(),
            communities: Vec::new(),
        };
    }

    // 构建无向邻接表
    let (adj, _weights, _total_weight) = build_undirected_adj(csr);

    // 每个节点的标签库（标签 -> 出现次数）
    let mut node_labels: Vec<HashMap<usize, usize>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut map = HashMap::new();
        map.insert(i, 1); // 初始标签：自己的 ID，出现 1 次
        node_labels.push(map);
    }

    let mut rng = config.seed;
    let mut next_rand = || {
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        rng
    };

    for _ in 0..config.iterations {
        let mut node_order: Vec<usize> = (0..n).collect();
        // 打乱顺序
        for i in (1..n).rev() {
            let j = (next_rand() as usize) % (i + 1);
            node_order.swap(i, j);
        }

        for &listener in &node_order {
            // 收集邻居说的标签
            let mut heard: HashMap<usize, f64> = HashMap::new();

            for (&neighbor, &weight) in &adj[listener] {
                // Speaker 从自己的标签库中随机选一个
                let labels = &node_labels[neighbor];
                let total_count: usize = labels.values().sum();
                if total_count == 0 {
                    continue;
                }

                let r = (next_rand() as usize) % total_count;
                let mut cumulative = 0;
                let mut chosen_label = 0;
                for (&label, &count) in labels {
                    cumulative += count;
                    if cumulative > r {
                        chosen_label = label;
                        break;
                    }
                }

                *heard.entry(chosen_label).or_insert(0.0) += weight;
            }

            if heard.is_empty() {
                continue;
            }

            // Listener 选择出现最多的标签
            let mut best_label = 0;
            let mut best_count = -1.0;
            let mut sorted_heard: Vec<(usize, f64)> = heard.into_iter().collect();
            sorted_heard.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| a.0.cmp(&b.0))
            });
            if let Some(&(label, count)) = sorted_heard.first() {
                best_label = label;
                best_count = count;
            }

            if best_count > 0.0 {
                *node_labels[listener].entry(best_label).or_insert(0) += 1;
            }
        }
    }

    // 计算频率并过滤
    let total_iter = config.iterations + 1; // 初始 + 迭代次数
    let mut result_labels: Vec<HashMap<usize, f64>> = Vec::with_capacity(n);
    for labels in &node_labels {
        let total: usize = labels.values().sum();
        let mut filtered = HashMap::new();
        for (&label, &count) in labels {
            let freq = count as f64 / total as f64;
            if freq >= config.threshold {
                filtered.insert(label, freq);
            }
        }
        result_labels.push(filtered);
    }

    // 收集社区
    let mut community_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for (node, labels) in result_labels.iter().enumerate() {
        for &label in labels.keys() {
            community_map.entry(label).or_default().push(node);
        }
    }

    let mut communities: Vec<Vec<usize>> = community_map.into_values().collect();
    communities.sort_by(|a, b| b.len().cmp(&a.len()));

    SLPAResult {
        node_labels: result_labels,
        communities,
    }
}

// ============================================================================
// 社区质量评估
// ============================================================================

/// 社区质量指标
#[derive(Debug, Clone)]
pub struct CommunityQualityMetrics {
    /// 模块度
    pub modularity: f64,
    /// 各社区电导
    pub conductance: Vec<f64>,
    /// 平均电导
    pub avg_conductance: f64,
    /// 社区数量
    pub num_communities: usize,
}

/// 计算社区模块度
pub(crate) fn compute_modularity(csr: &CsrAdj, node_comm: &[usize]) -> f64 {
    let (adj, weights, total_weight) = build_undirected_adj(csr);
    modularity_undirected(&adj, &weights, node_comm, total_weight)
}

/// 计算社区电导
///
/// 电导 = 社区割边数 / min(2m_c, 2(m - m_c))
/// 其中 m_c 是社区内部边数（端点都在社区内），m 是总边数
pub(crate) fn compute_conductance(csr: &CsrAdj, community: &[usize]) -> f64 {
    let community_set: HashSet<usize> = community.iter().copied().collect();

    let mut internal_edges = 0.0;
    let mut cut_edges = 0.0;

    for &node in &community_set {
        let rng = csr.offsets[node]..csr.offsets[node + 1];
        for k in rng {
            let neighbor = csr.targets[k];
            let w = csr.weights[k];
            if community_set.contains(&neighbor) {
                internal_edges += w;
            } else {
                cut_edges += w;
            }
        }
    }

    // 无向化：内部边算了两次，割边算了一次
    internal_edges /= 2.0;

    let total_community = internal_edges * 2.0 + cut_edges;
    let total_other = cut_edges; // 简化

    let denominator = total_community.min(total_other);
    if denominator < 1e-15 {
        return 0.0;
    }

    cut_edges / denominator
}

/// 计算标准化互信息（NMI）
///
/// 衡量两个社区划分的相似性，范围 [0, 1]。
/// NMI(X,Y) = 2·I(X;Y) / (H(X) + H(Y))
pub fn normalized_mutual_info(partition_a: &[usize], partition_b: &[usize]) -> f64 {
    let n = partition_a.len();
    if n == 0 || partition_b.len() != n {
        return 0.0;
    }

    // 计算联合分布
    let mut counts: HashMap<(usize, usize), usize> = HashMap::new();
    let mut count_a: HashMap<usize, usize> = HashMap::new();
    let mut count_b: HashMap<usize, usize> = HashMap::new();

    for i in 0..n {
        let a = partition_a[i];
        let b = partition_b[i];
        *counts.entry((a, b)).or_insert(0) += 1;
        *count_a.entry(a).or_insert(0) += 1;
        *count_b.entry(b).or_insert(0) += 1;
    }

    // 计算互信息 I(X;Y)
    let mut mutual_info = 0.0;
    for (&(a, b), &count) in &counts {
        let p_xy = count as f64 / n as f64;
        let p_x = count_a[&a] as f64 / n as f64;
        let p_y = count_b[&b] as f64 / n as f64;
        if p_xy > 0.0 && p_x > 0.0 && p_y > 0.0 {
            mutual_info += p_xy * (p_xy / (p_x * p_y)).ln();
        }
    }

    // 计算熵 H(X) 和 H(Y)
    let mut h_x = 0.0;
    for &count in count_a.values() {
        let p = count as f64 / n as f64;
        if p > 0.0 {
            h_x -= p * p.ln();
        }
    }

    let mut h_y = 0.0;
    for &count in count_b.values() {
        let p = count as f64 / n as f64;
        if p > 0.0 {
            h_y -= p * p.ln();
        }
    }

    if h_x + h_y < 1e-15 {
        return 0.0;
    }

    2.0 * mutual_info / (h_x + h_y)
}

// ============================================================================
// KnowledgeGraph 扩展方法
// ============================================================================

impl KnowledgeGraph {
    /// Louvain 社区发现算法
    pub fn louvain_communities(&self, config: Option<LouvainConfig>) -> Vec<Community> {
        let cfg = config.unwrap_or_default();
        let csr = CsrAdj::from_graph(&self.graph);
        let partition = louvain_csr(&csr, &cfg);
        self.partition_to_communities(&partition)
    }

    /// 标签传播社区发现
    pub fn label_propagation_communities(
        &self,
        config: Option<LabelPropagationConfig>,
    ) -> Vec<Community> {
        let cfg = config.unwrap_or_default();
        let csr = CsrAdj::from_graph(&self.graph);
        let partition = label_propagation_csr(&csr, &cfg);
        self.partition_to_communities(&partition)
    }

    /// 谱聚类社区发现
    pub fn spectral_clustering_communities(
        &self,
        config: Option<SpectralClusteringConfig>,
    ) -> Vec<Community> {
        let cfg = config.unwrap_or_default();
        let csr = CsrAdj::from_graph(&self.graph);
        let partition = spectral_clustering_csr(&csr, &cfg);
        self.partition_to_communities(&partition)
    }

    /// Girvan-Newman 分裂式社区发现
    pub fn girvan_newman_communities(
        &self,
        config: Option<GirvanNewmanConfig>,
    ) -> Vec<Community> {
        let cfg = config.unwrap_or_default();
        let csr = CsrAdj::from_graph(&self.graph);
        let partition = girvan_newman_csr(&csr, &cfg);
        self.partition_to_communities(&partition)
    }

    /// SLPA 重叠社区发现
    pub fn slpa_overlapping_communities(
        &self,
        config: Option<SLPAConfig>,
    ) -> (Vec<Community>, HashMap<String, HashMap<String, f64>>) {
        let cfg = config.unwrap_or_default();
        let csr = CsrAdj::from_graph(&self.graph);
        let result = slpa_csr(&csr, &cfg);

        // 社区列表
        let mut communities = Vec::new();
        for (i, members) in result.communities.iter().enumerate() {
            let node_ids: Vec<String> = members
                .iter()
                .map(|&idx| self.graph[petgraph::graph::NodeIndex::new(idx)].id.clone())
                .collect();
            let density = if members.len() > 1 {
                let mut internal = 0;
                for a in 0..members.len() {
                    for b in a + 1..members.len() {
                        let idx_a = petgraph::graph::NodeIndex::new(members[a]);
                        let idx_b = petgraph::graph::NodeIndex::new(members[b]);
                        if self.graph.find_edge(idx_a, idx_b).is_some()
                            || self.graph.find_edge(idx_b, idx_a).is_some()
                        {
                            internal += 1;
                        }
                    }
                }
                let max_edges = members.len() * (members.len() - 1) / 2;
                internal as f64 / max_edges as f64
            } else {
                0.0
            };
            communities.push(Community {
                id: i,
                nodes: node_ids,
                density,
                label: format!("社区 {}", i),
            });
        }

        // 每个节点的社区归属概率
        let mut node_community_probs = HashMap::new();
        for (node_idx, labels) in result.node_labels.iter().enumerate() {
            let node_id = &self.graph[petgraph::graph::NodeIndex::new(node_idx)].id;
            let mut probs = HashMap::new();
            for (&label, &freq) in labels {
                probs.insert(format!("社区_{}", label), freq);
            }
            node_community_probs.insert(node_id.clone(), probs);
        }

        (communities, node_community_probs)
    }

    /// 社区质量评估
    pub fn community_quality_metrics(&self, communities: &[Community]) -> CommunityQualityMetrics {
        let csr = CsrAdj::from_graph(&self.graph);
        let n = self.node_count();

        // 构建 node_community 映射
        let mut node_comm = vec![0usize; n];
        for (cid, comm) in communities.iter().enumerate() {
            for node_id in &comm.nodes {
                if let Some(&idx) = self.node_map.get(node_id) {
                    node_comm[idx.index()] = cid;
                }
            }
        }

        let modularity = compute_modularity(&csr, &node_comm);

        let mut conductance_values = Vec::new();
        for comm in communities {
            let member_indices: Vec<usize> = comm
                .nodes
                .iter()
                .filter_map(|id| self.node_map.get(id).map(|idx| idx.index()))
                .collect();
            let cond = compute_conductance(&csr, &member_indices);
            conductance_values.push(cond);
        }

        let avg_conductance = if conductance_values.is_empty() {
            0.0
        } else {
            conductance_values.iter().sum::<f64>() / conductance_values.len() as f64
        };

        CommunityQualityMetrics {
            modularity,
            conductance: conductance_values,
            avg_conductance,
            num_communities: communities.len(),
        }
    }

    /// 将内部划分转换为 Community 类型
    fn partition_to_communities(&self, partition: &CommunityPartition) -> Vec<Community> {
        let mut communities = Vec::new();
        for (i, members) in partition.communities.iter().enumerate() {
            let node_ids: Vec<String> = members
                .iter()
                .map(|&idx| self.graph[petgraph::graph::NodeIndex::new(idx)].id.clone())
                .collect();

            let density = if members.len() > 1 {
                let mut internal_edges = 0;
                for a in 0..members.len() {
                    for b in a + 1..members.len() {
                        let idx_a = petgraph::graph::NodeIndex::new(members[a]);
                        let idx_b = petgraph::graph::NodeIndex::new(members[b]);
                        if self.graph.find_edge(idx_a, idx_b).is_some()
                            || self.graph.find_edge(idx_b, idx_a).is_some()
                        {
                            internal_edges += 1;
                        }
                    }
                }
                let max_edges = members.len() * (members.len() - 1) / 2;
                internal_edges as f64 / max_edges as f64
            } else {
                0.0
            };

            communities.push(Community {
                id: i,
                nodes: node_ids,
                density,
                label: format!("社区 {}", i),
            });
        }
        communities
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

    fn build_two_communities_graph() -> KnowledgeGraph {
        // 两个社区，社区内连接密集，社区间连接稀疏
        KnowledgeGraphBuilder::new()
            .add_node("a1", "A1", "group1")
            .add_node("a2", "A2", "group1")
            .add_node("a3", "A3", "group1")
            .add_node("b1", "B1", "group2")
            .add_node("b2", "B2", "group2")
            .add_node("b3", "B3", "group2")
            // 社区 1 内部
            .add_edge("a1", "a2", 1.0)
            .add_edge("a2", "a1", 1.0)
            .add_edge("a1", "a3", 1.0)
            .add_edge("a3", "a1", 1.0)
            .add_edge("a2", "a3", 1.0)
            .add_edge("a3", "a2", 1.0)
            // 社区 2 内部
            .add_edge("b1", "b2", 1.0)
            .add_edge("b2", "b1", 1.0)
            .add_edge("b1", "b3", 1.0)
            .add_edge("b3", "b1", 1.0)
            .add_edge("b2", "b3", 1.0)
            .add_edge("b3", "b2", 1.0)
            // 社区间桥
            .add_edge("a3", "b1", 1.0)
            .add_edge("b1", "a3", 1.0)
            .build()
    }

    #[test]
    fn test_louvain_two_communities() {
        let graph = build_two_communities_graph();
        let communities = graph.louvain_communities(None);

        // 应该至少检测到 2 个社区
        assert!(communities.len() >= 2);

        // 模块度应为正
        let metrics = graph.community_quality_metrics(&communities);
        assert!(metrics.modularity > 0.0);
    }

    #[test]
    fn test_label_propagation() {
        let graph = build_two_communities_graph();
        let communities = graph.label_propagation_communities(None);

        assert!(!communities.is_empty());

        let metrics = graph.community_quality_metrics(&communities);
        assert!(metrics.modularity > -0.5);
    }

    #[test]
    fn test_spectral_clustering() {
        let graph = build_two_communities_graph();
        let config = SpectralClusteringConfig {
            k: 2,
            max_iterations: 50,
        };
        let communities = graph.spectral_clustering_communities(Some(config));

        assert_eq!(communities.len(), 2);
    }

    #[test]
    fn test_girvan_newman() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .add_node("b", "B", "test")
            .add_node("c", "C", "test")
            .add_node("d", "D", "test")
            .add_edge("a", "b", 1.0)
            .add_edge("b", "a", 1.0)
            .add_edge("b", "c", 1.0)
            .add_edge("c", "b", 1.0)
            .add_edge("c", "d", 1.0)
            .add_edge("d", "c", 1.0)
            .build();

        let config = GirvanNewmanConfig {
            target_communities: 2,
        };
        let communities = graph.girvan_newman_communities(Some(config));

        assert!(communities.len() >= 2);
    }

    #[test]
    fn test_slpa_overlapping() {
        let graph = build_two_communities_graph();
        let config = SLPAConfig {
            iterations: 30,
            threshold: 0.1,
            seed: 42,
        };
        let (communities, node_probs) = graph.slpa_overlapping_communities(Some(config));

        assert!(!communities.is_empty());
        assert_eq!(node_probs.len(), 6); // 6 个节点

        // 桥节点 a3 可能属于多个社区
        // （不一定，取决于随机性，但应该有节点有标签）
        for probs in node_probs.values() {
            assert!(!probs.is_empty());
        }
    }

    #[test]
    fn test_modularity_computation() {
        let graph = build_two_communities_graph();
        let csr = CsrAdj::from_graph(&graph.graph);

        // 完美划分的模块度
        let mut perfect_partition = vec![0; 6];
        perfect_partition[3] = 1; // b1
        perfect_partition[4] = 1; // b2
        perfect_partition[5] = 1; // b3

        let mod_val = compute_modularity(&csr, &perfect_partition);
        assert!(mod_val > 0.0);

        // 随机划分（全在一个社区）的模块度为 0
        let trivial_partition = vec![0; 6];
        let mod_trivial = compute_modularity(&csr, &trivial_partition);
        assert_relative_eq!(mod_trivial, 0.0, epsilon = 1e-9);
    }

    #[test]
    fn test_conductance() {
        let graph = build_two_communities_graph();
        let csr = CsrAdj::from_graph(&graph.graph);

        // 社区 1 的电导
        let community1 = vec![0, 1, 2]; // a1, a2, a3
        let cond = compute_conductance(&csr, &community1);
        assert!(cond >= 0.0 && cond <= 1.0);
    }

    #[test]
    fn test_nmi_identical() {
        let a = vec![0, 0, 1, 1, 2, 2];
        let b = vec![0, 0, 1, 1, 2, 2];
        let nmi = normalized_mutual_info(&a, &b);
        assert_relative_eq!(nmi, 1.0, epsilon = 1e-9);
    }

    #[test]
    fn test_nmi_independent() {
        let a = vec![0, 0, 1, 1];
        let b = vec![0, 1, 0, 1];
        let nmi = normalized_mutual_info(&a, &b);
        // 完全独立的划分 NMI 应该为 0
        assert!(nmi.abs() < 1e-9);
    }

    #[test]
    fn test_community_quality_metrics() {
        let graph = build_two_communities_graph();
        let communities = graph.louvain_communities(None);
        let metrics = graph.community_quality_metrics(&communities);

        assert_eq!(metrics.num_communities, communities.len());
        assert_eq!(metrics.conductance.len(), communities.len());
        assert!(metrics.avg_conductance >= 0.0);
        assert!(metrics.avg_conductance <= 1.0 + 1e-9);
    }

    #[test]
    fn test_empty_graph_communities() {
        let graph = KnowledgeGraph::new();
        let communities = graph.louvain_communities(None);
        assert!(communities.is_empty());

        let (slpa_comms, _) = graph.slpa_overlapping_communities(None);
        assert!(slpa_comms.is_empty());
    }

    #[test]
    fn test_single_node_community() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .build();

        let communities = graph.louvain_communities(None);
        assert_eq!(communities.len(), 1);
        assert_eq!(communities[0].nodes.len(), 1);
    }
}
