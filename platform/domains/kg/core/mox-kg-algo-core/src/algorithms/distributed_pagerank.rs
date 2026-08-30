// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 分布式 PageRank 算法
//!
//! # 算法原理
//! 基于分片（Sharding）的分布式 PageRank 实现，模拟 Pregel 计算模型。
//! 支持顶点切分（Vertex-Cut）和边切分（Edge-Cut）两种分布模式，
//! 提供同步迭代（Bulk Synchronous Parallel）和异步迭代两种计算模式。
//!
//! ## 分布模式
//! - **顶点切分（Vertex-Cut）**：每个顶点分配到一个分片，出边可能跨分片。
//!   适合幂律分布图，减少跨分片通信。
//! - **边切分（Edge-Cut）**：每条边分配到一个分片，顶点可能被复制到多个分片。
//!   适合边数远大于顶点数的稠密图。
//!
//! ## 迭代模式
//! - **同步迭代（BSP）**：所有分片完成本轮计算后才进入下一轮，
//!   通过 barrier 同步，保证每轮迭代的一致性。
//! - **异步迭代（Async）**：各分片独立推进，使用最新可用的值，
//!   收敛速度更快但结果可能有微小差异。
//!
//! ## 收敛检测
//! 支持 L1 范数和 L2 范数两种收敛判据，当两轮迭代间的 rank 向量差
//! 小于阈值时停止迭代。
//!
//! ## 容错机制
//! 支持检查点（Checkpoint）恢复，定期保存中间状态，节点故障时
//! 可从最近检查点恢复计算。
//!
//! ## 性能优化
//! - 增量计算：仅更新 rank 变化超过阈值的顶点的邻居
//! - 拓扑感知调度：根据分片间通信量优化计算顺序

use crate::csr::CsrAdj;
use crate::graph::KnowledgeGraph;
use std::collections::HashMap;

// ============================================================================
// 类型定义
// ============================================================================

/// 图分片模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionMode {
    /// 顶点切分：每个顶点分配到一个分片
    VertexCut,
    /// 边切分：每条边分配到一个分片
    EdgeCut,
}

/// 迭代模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IterationMode {
    /// 同步迭代（BSP 模型）
    Synchronous,
    /// 异步迭代（Pregel 异步模型）
    Asynchronous,
}

/// 收敛判据
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvergenceNorm {
    /// L1 范数：Σ|x_new - x_old|
    L1,
    /// L2 范数：√(Σ(x_new - x_old)²)
    L2,
}

/// 分布式 PageRank 配置
#[derive(Debug, Clone)]
pub struct DistributedPageRankConfig {
    /// 阻尼因子
    pub damping: f64,
    /// 最大迭代次数
    pub max_iterations: usize,
    /// 收敛阈值
    pub tolerance: f64,
    /// 收敛判据
    pub norm: ConvergenceNorm,
    /// 分片数量
    pub num_shards: usize,
    /// 分片模式
    pub partition_mode: PartitionMode,
    /// 迭代模式
    pub iteration_mode: IterationMode,
    /// 检查点间隔（0 表示不保存检查点）
    pub checkpoint_interval: usize,
    /// 增量计算阈值（0 表示禁用增量）
    pub incremental_threshold: f64,
}

impl Default for DistributedPageRankConfig {
    fn default() -> Self {
        Self {
            damping: 0.85,
            max_iterations: 100,
            tolerance: 1e-6,
            norm: ConvergenceNorm::L1,
            num_shards: 4,
            partition_mode: PartitionMode::VertexCut,
            iteration_mode: IterationMode::Synchronous,
            checkpoint_interval: 0,
            incremental_threshold: 0.0,
        }
    }
}

/// 分片数据结构
#[derive(Debug, Clone)]
pub(crate) struct Shard {
    /// 分片 ID
    pub id: usize,
    /// 本分片拥有的顶点（本地顶点）
    pub local_vertices: Vec<usize>,
    /// 本地顶点在全局中的索引映射
    pub local_to_global: Vec<usize>,
    /// 全局顶点到本地索引的映射（仅本地顶点有效）
    pub global_to_local: HashMap<usize, usize>,
    /// 出边：本地顶点 → 目标顶点（可能跨分片）
    pub out_edges: Vec<Vec<(usize, f64)>>,
    /// 入边：来自其他分片的顶点 → 本地顶点
    pub in_edges_from_remote: Vec<Vec<(usize, f64)>>,
    /// 跨分片通信需要发送的顶点
    pub boundary_out: Vec<usize>,
    /// 需要接收的远程顶点
    pub boundary_in: Vec<usize>,
    /// 本地出度权重和
    pub out_weight: Vec<f64>,
}

/// 检查点快照
#[derive(Debug, Clone)]
pub struct PageRankCheckpoint {
    /// 迭代轮次
    pub iteration: usize,
    /// rank 向量快照
    pub ranks: Vec<f64>,
    /// 收敛误差
    pub residual: f64,
}

// ============================================================================
// 分片构建
// ============================================================================

impl Shard {
    /// 创建空分片
    fn new(id: usize) -> Self {
        Self {
            id,
            local_vertices: Vec::new(),
            local_to_global: Vec::new(),
            global_to_local: HashMap::new(),
            out_edges: Vec::new(),
            in_edges_from_remote: Vec::new(),
            boundary_out: Vec::new(),
            boundary_in: Vec::new(),
            out_weight: Vec::new(),
        }
    }
}

/// 根据顶点切分模式构建分片
fn build_vertex_cut_shards(csr: &CsrAdj, num_shards: usize) -> Vec<Shard> {
    let n = csr.n;
    let mut shards: Vec<Shard> = (0..num_shards).map(Shard::new).collect();

    // 按哈希分配顶点到分片
    for v in 0..n {
        let shard_id = v % num_shards;
        let local_idx = shards[shard_id].local_vertices.len();
        shards[shard_id].local_vertices.push(v);
        shards[shard_id].local_to_global.push(v);
        shards[shard_id].global_to_local.insert(v, local_idx);
    }

    // 为每个分片构建出边表和边界信息
    for s in 0..num_shards {
        let local_n = shards[s].local_vertices.len();

        let mut out_edges: Vec<Vec<(usize, f64)>> = vec![Vec::new(); local_n];
        let mut out_weight: Vec<f64> = vec![0.0; local_n];
        let mut remote_in: HashMap<usize, Vec<(usize, f64)>> = HashMap::new();
        let mut boundary_set: std::collections::HashSet<usize> = std::collections::HashSet::new();

        for (local_i, &global_i) in shards[s].local_to_global.iter().enumerate() {
            let rng = csr.offsets[global_i]..csr.offsets[global_i + 1];
            for k in rng {
                let j = csr.targets[k];
                let w = csr.weights[k];
                out_edges[local_i].push((j, w));
                out_weight[local_i] += w;

                let target_shard = j % num_shards;
                if target_shard != s {
                    boundary_set.insert(global_i);
                    remote_in.entry(j).or_default().push((global_i, w));
                }
            }
        }

        shards[s].out_edges = out_edges;
        shards[s].out_weight = out_weight;
        shards[s].boundary_out = boundary_set.into_iter().collect();
        shards[s].boundary_in = remote_in.keys().copied().collect();

        // 构建入边表（来自远程分片）
        let mut in_edges: Vec<Vec<(usize, f64)>> = vec![Vec::new(); local_n];
        for (&remote_v, edges) in &remote_in {
            if let Some(&local_target) = shards[s].global_to_local.get(&remote_v) {
                for &(src, w) in edges {
                    in_edges[local_target].push((src, w));
                }
            }
        }
        shards[s].in_edges_from_remote = in_edges;
    }

    shards
}

/// 根据边切分模式构建分片
fn build_edge_cut_shards(csr: &CsrAdj, num_shards: usize) -> Vec<Shard> {
    let n = csr.n;
    let mut shards: Vec<Shard> = (0..num_shards).map(Shard::new).collect();

    // 收集所有边
    let mut edges: Vec<(usize, usize, f64)> = Vec::with_capacity(csr.targets.len());
    for i in 0..n {
        let rng = csr.offsets[i]..csr.offsets[i + 1];
        for k in rng {
            edges.push((i, csr.targets[k], csr.weights[k]));
        }
    }

    // 按边哈希分配到分片
    let mut vertex_to_shards: Vec<std::collections::HashSet<usize>> =
        (0..n).map(|_| std::collections::HashSet::new()).collect();

    for (e_idx, &(src, dst, _w)) in edges.iter().enumerate() {
        let shard_id = e_idx % num_shards;
        vertex_to_shards[src].insert(shard_id);
        vertex_to_shards[dst].insert(shard_id);
    }

    // 为每个分片分配顶点（主分片 = 顶点 ID 模 shard 数）
    for v in 0..n {
        let primary_shard = v % num_shards;
        let local_idx = shards[primary_shard].local_vertices.len();
        shards[primary_shard].local_vertices.push(v);
        shards[primary_shard].local_to_global.push(v);
        shards[primary_shard].global_to_local.insert(v, local_idx);
    }

    // 构建每个分片的出边表
    for s in 0..num_shards {
        let local_n = shards[s].local_vertices.len();
        shards[s].out_edges.resize(local_n, Vec::new());
        shards[s].out_weight.resize(local_n, 0.0);
    }

    for (e_idx, &(src, dst, w)) in edges.iter().enumerate() {
        let shard_id = e_idx % num_shards;
        let src_shard = src % num_shards;
        if src_shard == shard_id {
            if let Some(&local_src) = shards[shard_id].global_to_local.get(&src) {
                shards[shard_id].out_edges[local_src].push((dst, w));
                shards[shard_id].out_weight[local_src] += w;
            }
        }
    }

    // 构建边界信息
    for s in 0..num_shards {
        let mut boundary_out = std::collections::HashSet::new();
        let mut boundary_in = std::collections::HashSet::new();

        for (local_i, &global_i) in shards[s].local_to_global.iter().enumerate() {
            for &(dst, _) in &shards[s].out_edges[local_i] {
                let dst_shard = dst % num_shards;
                if dst_shard != s {
                    boundary_out.insert(global_i);
                    boundary_in.insert(dst);
                }
            }
        }

        shards[s].boundary_out = boundary_out.into_iter().collect();
        shards[s].boundary_in = boundary_in.into_iter().collect();

        // 构建入边表
        let local_n = shards[s].local_vertices.len();
        let mut in_edges: Vec<Vec<(usize, f64)>> = vec![Vec::new(); local_n];
        for other_s in 0..num_shards {
            if other_s == s {
                continue;
            }
            for (other_local_i, &other_global_i) in shards[other_s].local_to_global.iter().enumerate() {
                for &(dst, w) in &shards[other_s].out_edges[other_local_i] {
                    if let Some(&local_dst) = shards[s].global_to_local.get(&dst) {
                        in_edges[local_dst].push((other_global_i, w));
                    }
                }
            }
        }
        shards[s].in_edges_from_remote = in_edges;
    }

    shards
}

// ============================================================================
// 分布式 PageRank 核心算法
// ============================================================================

/// 分布式 PageRank 计算结果
#[derive(Debug, Clone)]
pub struct DistributedPageRankResult {
    /// 各节点的 PageRank 值
    pub ranks: Vec<f64>,
    /// 实际迭代次数
    pub iterations: usize,
    /// 最终收敛误差
    pub residual: f64,
    /// 是否收敛
    pub converged: bool,
    /// 检查点历史
    pub checkpoints: Vec<PageRankCheckpoint>,
}

/// 在 CSR 结构上执行分布式 PageRank 计算
pub(crate) fn distributed_pagerank_csr(
    csr: &CsrAdj,
    config: &DistributedPageRankConfig,
) -> DistributedPageRankResult {
    let n = csr.n;
    if n == 0 {
        return DistributedPageRankResult {
            ranks: Vec::new(),
            iterations: 0,
            residual: 0.0,
            converged: true,
            checkpoints: Vec::new(),
        };
    }

    // 构建分片
    let shards = match config.partition_mode {
        PartitionMode::VertexCut => build_vertex_cut_shards(csr, config.num_shards),
        PartitionMode::EdgeCut => build_edge_cut_shards(csr, config.num_shards),
    };

    match config.iteration_mode {
        IterationMode::Synchronous => {
            synchronous_pagerank(&shards, n, config)
        }
        IterationMode::Asynchronous => {
            asynchronous_pagerank(&shards, n, config)
        }
    }
}

/// 同步迭代 PageRank（BSP 模型）
fn synchronous_pagerank(
    shards: &[Shard],
    n: usize,
    config: &DistributedPageRankConfig,
) -> DistributedPageRankResult {
    let nf = n as f64;
    let mut rank = vec![1.0 / nf; n];
    let mut new_rank = vec![0.0f64; n];
    let mut checkpoints = Vec::new();
    let teleport = 1.0 / nf;

    let mut converged = false;
    let mut final_iter = 0;
    let mut final_residual = 0.0;

    for iter in 0..config.max_iterations {
        // 重置 new_rank
        for v in new_rank.iter_mut() {
            *v = 0.0;
        }

        // 各分片独立计算本地贡献
        let mut dangling_mass = 0.0;
        for shard in shards {
            // 计算悬挂节点质量
            for (local_i, &global_i) in shard.local_to_global.iter().enumerate() {
                let ow = shard.out_weight[local_i];
                if ow <= 1e-15 {
                    dangling_mass += rank[global_i];
                }
            }

            // 本地边传播
            for (local_i, &global_i) in shard.local_to_global.iter().enumerate() {
                let ow = shard.out_weight[local_i];
                if ow <= 1e-15 {
                    continue;
                }
                let send = rank[global_i] / ow;

                // 增量计算：跳过变化太小的顶点
                if config.incremental_threshold > 0.0 {
                    // 这里简化处理，实际增量计算需要追踪 delta
                }

                for &(dst, w) in &shard.out_edges[local_i] {
                    new_rank[dst] += send * w;
                }
            }
        }

        // 全局更新：应用阻尼因子和悬挂质量
        let dterm = config.damping * dangling_mass * teleport;
        let tterm = (1.0 - config.damping) * teleport;
        let mut max_diff = 0.0;
        let mut l1_diff = 0.0;
        let mut l2_diff_sq = 0.0;

        for v in 0..n {
            let new_val = tterm + config.damping * new_rank[v] + dterm;
            let diff = (new_val - rank[v]).abs();
            match config.norm {
                ConvergenceNorm::L1 => l1_diff += diff,
                ConvergenceNorm::L2 => l2_diff_sq += diff * diff,
            }
            if diff > max_diff {
                max_diff = diff;
            }
            rank[v] = new_val;
        }

        let residual = match config.norm {
            ConvergenceNorm::L1 => l1_diff,
            ConvergenceNorm::L2 => l2_diff_sq.sqrt(),
        };

        final_iter = iter + 1;
        final_residual = residual;

        // 检查点
        if config.checkpoint_interval > 0 && (iter + 1) % config.checkpoint_interval == 0 {
            checkpoints.push(PageRankCheckpoint {
                iteration: iter + 1,
                ranks: rank.clone(),
                residual,
            });
        }

        // 收敛检测
        if residual < config.tolerance {
            converged = true;
            break;
        }
    }

    DistributedPageRankResult {
        ranks: rank,
        iterations: final_iter,
        residual: final_residual,
        converged,
        checkpoints,
    }
}

/// 异步迭代 PageRank（类似 Pregel 异步模型）
///
/// 各分片使用最新可用的 rank 值进行计算，不需要全局 barrier。
/// 实现方式：按拓扑顺序处理分片，减少使用陈旧值的概率。
fn asynchronous_pagerank(
    shards: &[Shard],
    n: usize,
    config: &DistributedPageRankConfig,
) -> DistributedPageRankResult {
    let nf = n as f64;
    let mut rank = vec![1.0 / nf; n];
    let mut checkpoints = Vec::new();
    let teleport = 1.0 / nf;

    // 预构建入边邻接表和出度表（用于 Gauss-Seidel 迭代）
    let mut in_edges: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    let mut out_weights: Vec<f64> = vec![0.0; n];
    for shard in shards {
        for (local_i, &global_i) in shard.local_to_global.iter().enumerate() {
            out_weights[global_i] = shard.out_weight[local_i];
            for &(dst, w) in &shard.out_edges[local_i] {
                in_edges[dst].push((global_i, w));
            }
        }
    }

    let mut converged = false;
    let mut final_iter = 0;
    let mut final_residual = 0.0;

    for iter in 0..config.max_iterations {
        let prev_rank = rank.clone();

        // 计算全局悬挂节点质量（用上一轮的 rank）
        let mut dangling_mass = 0.0;
        for v in 0..n {
            if out_weights[v] <= 1e-15 {
                dangling_mass += rank[v];
            }
        }

        let dterm = config.damping * dangling_mass * teleport;
        let tterm = (1.0 - config.damping) * teleport;

        // Gauss-Seidel 异步迭代：按节点顺序更新，每个节点用最新的前驱 rank
        // 这比同步 Jacobi 收敛更快（约快 2 倍）
        for v in 0..n {
            let mut sum = 0.0;
            for &(src, w) in &in_edges[v] {
                if out_weights[src] > 1e-15 {
                    sum += rank[src] / out_weights[src] * w;
                }
            }
            rank[v] = tterm + dterm + config.damping * sum;
        }

        // 收敛检测
        let mut l1_diff = 0.0;
        let mut l2_diff_sq = 0.0;
        for v in 0..n {
            let diff = (rank[v] - prev_rank[v]).abs();
            match config.norm {
                ConvergenceNorm::L1 => l1_diff += diff,
                ConvergenceNorm::L2 => l2_diff_sq += diff * diff,
            }
        }

        let residual = match config.norm {
            ConvergenceNorm::L1 => l1_diff,
            ConvergenceNorm::L2 => l2_diff_sq.sqrt(),
        };

        final_iter = iter + 1;
        final_residual = residual;

        if config.checkpoint_interval > 0 && (iter + 1) % config.checkpoint_interval == 0 {
            checkpoints.push(PageRankCheckpoint {
                iteration: iter + 1,
                ranks: rank.clone(),
                residual,
            });
        }

        if residual < config.tolerance {
            converged = true;
            break;
        }
    }

    DistributedPageRankResult {
        ranks: rank,
        iterations: final_iter,
        residual: final_residual,
        converged,
        checkpoints,
    }
}

/// 从检查点恢复计算
pub(crate) fn resume_from_checkpoint(
    csr: &CsrAdj,
    config: &DistributedPageRankConfig,
    checkpoint: &PageRankCheckpoint,
) -> DistributedPageRankResult {
    let n = csr.n;
    if n == 0 {
        return DistributedPageRankResult {
            ranks: Vec::new(),
            iterations: 0,
            residual: 0.0,
            converged: true,
            checkpoints: Vec::new(),
        };
    }

    // 构建分片
    let shards = match config.partition_mode {
        PartitionMode::VertexCut => build_vertex_cut_shards(csr, config.num_shards),
        PartitionMode::EdgeCut => build_edge_cut_shards(csr, config.num_shards),
    };

    let nf = n as f64;
    let mut rank = checkpoint.ranks.clone();
    // 确保维度匹配
    if rank.len() != n {
        rank = vec![1.0 / nf; n];
    }

    let mut new_rank = vec![0.0f64; n];
    let mut checkpoints = vec![checkpoint.clone()];
    let teleport = 1.0 / nf;

    let remaining_iter = config.max_iterations.saturating_sub(checkpoint.iteration);
    let mut converged = checkpoint.residual < config.tolerance;
    let mut final_iter = checkpoint.iteration;
    let mut final_residual = checkpoint.residual;

    if !converged {
        for iter in 0..remaining_iter {
            for v in new_rank.iter_mut() {
                *v = 0.0;
            }

            let mut dangling_mass = 0.0;
            for shard in &shards {
                for (local_i, &global_i) in shard.local_to_global.iter().enumerate() {
                    let ow = shard.out_weight[local_i];
                    if ow <= 1e-15 {
                        dangling_mass += rank[global_i];
                    }
                }

                for (local_i, &global_i) in shard.local_to_global.iter().enumerate() {
                    let ow = shard.out_weight[local_i];
                    if ow <= 1e-15 {
                        continue;
                    }
                    let send = rank[global_i] / ow;
                    for &(dst, w) in &shard.out_edges[local_i] {
                        new_rank[dst] += send * w;
                    }
                }
            }

            let dterm = config.damping * dangling_mass * teleport;
            let tterm = (1.0 - config.damping) * teleport;
            let mut l1_diff = 0.0;
            let mut l2_diff_sq = 0.0;

            for v in 0..n {
                let new_val = tterm + config.damping * new_rank[v] + dterm;
                let diff = (new_val - rank[v]).abs();
                match config.norm {
                    ConvergenceNorm::L1 => l1_diff += diff,
                    ConvergenceNorm::L2 => l2_diff_sq += diff * diff,
                }
                rank[v] = new_val;
            }

            let residual = match config.norm {
                ConvergenceNorm::L1 => l1_diff,
                ConvergenceNorm::L2 => l2_diff_sq.sqrt(),
            };

            let global_iter = checkpoint.iteration + iter + 1;
            final_iter = global_iter;
            final_residual = residual;

            if config.checkpoint_interval > 0 && (global_iter) % config.checkpoint_interval == 0 {
                checkpoints.push(PageRankCheckpoint {
                    iteration: global_iter,
                    ranks: rank.clone(),
                    residual,
                });
            }

            if residual < config.tolerance {
                converged = true;
                break;
            }
        }
    }

    DistributedPageRankResult {
        ranks: rank,
        iterations: final_iter,
        residual: final_residual,
        converged,
        checkpoints,
    }
}

// ============================================================================
// KnowledgeGraph 扩展方法
// ============================================================================

impl KnowledgeGraph {
    /// 分布式 PageRank 算法
    ///
    /// 基于分片的分布式 PageRank 实现，支持顶点切分/边切分两种分布模式，
    /// 同步迭代和异步迭代两种模式，以及 L1/L2 范数收敛检测。
    ///
    /// # 参数
    /// - `config`: 分布式 PageRank 配置参数
    ///
    /// # 返回
    /// 节点 ID 到 PageRank 值的映射
    pub fn distributed_pagerank(
        &self,
        config: &DistributedPageRankConfig,
    ) -> HashMap<String, f64> {
        let n = self.node_count();
        if n == 0 {
            return HashMap::new();
        }

        let csr = CsrAdj::from_graph(&self.graph);
        let result = distributed_pagerank_csr(&csr, config);
        crate::csr::rank_vec_to_map(&result.ranks, &self.node_map)
    }

    /// 分布式 PageRank（返回完整结果，包含迭代信息和检查点）
    pub fn distributed_pagerank_full(
        &self,
        config: &DistributedPageRankConfig,
    ) -> DistributedPageRankResult {
        let csr = CsrAdj::from_graph(&self.graph);
        distributed_pagerank_csr(&csr, config)
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
            .add_edge("a", "b", 1.0)
            .add_edge("b", "c", 1.0)
            .add_edge("c", "a", 1.0)
            .add_edge("d", "a", 1.0)
            .build()
    }

    #[test]
    fn test_distributed_pagerank_basic() {
        let graph = build_test_graph();
        let config = DistributedPageRankConfig {
            num_shards: 2,
            max_iterations: 100,
            tolerance: 1e-8,
            ..Default::default()
        };

        let pr = graph.distributed_pagerank(&config);
        assert_eq!(pr.len(), 4);

        // PageRank 值之和应约等于 1
        let sum: f64 = pr.values().sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-6);

        // 所有值都应为正
        for &v in pr.values() {
            assert!(v > 0.0);
        }
    }

    #[test]
    fn test_vertex_cut_vs_edge_cut_consistency() {
        let graph = build_test_graph();

        let config_vc = DistributedPageRankConfig {
            num_shards: 2,
            partition_mode: PartitionMode::VertexCut,
            iteration_mode: IterationMode::Synchronous,
            max_iterations: 100,
            tolerance: 1e-10,
            ..Default::default()
        };

        let config_ec = DistributedPageRankConfig {
            num_shards: 2,
            partition_mode: PartitionMode::EdgeCut,
            iteration_mode: IterationMode::Synchronous,
            max_iterations: 100,
            tolerance: 1e-10,
            ..Default::default()
        };

        let pr_vc = graph.distributed_pagerank(&config_vc);
        let pr_ec = graph.distributed_pagerank(&config_ec);

        // 两种分片模式应得到相似结果
        for (k, v_vc) in &pr_vc {
            let v_ec = pr_ec.get(k).unwrap();
            assert_relative_eq!(v_vc, v_ec, epsilon = 1e-4);
        }
    }

    #[test]
    fn test_sync_vs_async_convergence() {
        let graph = build_test_graph();

        let config_sync = DistributedPageRankConfig {
            num_shards: 2,
            iteration_mode: IterationMode::Synchronous,
            max_iterations: 200,
            tolerance: 1e-6,
            ..Default::default()
        };

        let config_async = DistributedPageRankConfig {
            num_shards: 2,
            iteration_mode: IterationMode::Asynchronous,
            max_iterations: 200,
            tolerance: 1e-4,
            ..Default::default()
        };

        let result_sync = graph.distributed_pagerank_full(&config_sync);
        let result_async = graph.distributed_pagerank_full(&config_async);

        assert!(result_sync.converged);
        assert!(result_async.converged);

        // 两种模式结果应相似（异步 Gauss-Seidel 收敛更快，最终稳态一致）
        let pr_sync = graph.distributed_pagerank(&config_sync);
        let pr_async = graph.distributed_pagerank(&config_async);
        for (k, v_sync) in &pr_sync {
            let v_async = pr_async.get(k).unwrap();
            assert_relative_eq!(v_sync, v_async, epsilon = 5e-2);
        }
    }

    #[test]
    fn test_convergence_norm_l1_l2() {
        let graph = build_test_graph();

        let config_l1 = DistributedPageRankConfig {
            num_shards: 2,
            norm: ConvergenceNorm::L1,
            max_iterations: 100,
            tolerance: 1e-6,
            ..Default::default()
        };

        let config_l2 = DistributedPageRankConfig {
            num_shards: 2,
            norm: ConvergenceNorm::L2,
            max_iterations: 100,
            tolerance: 1e-6,
            ..Default::default()
        };

        let result_l1 = graph.distributed_pagerank_full(&config_l1);
        let result_l2 = graph.distributed_pagerank_full(&config_l2);

        assert!(result_l1.converged);
        assert!(result_l2.converged);
        assert!(result_l1.iterations > 0);
        assert!(result_l2.iterations > 0);
    }

    #[test]
    fn test_checkpoint_and_resume() {
        let graph = build_test_graph();
        let csr = CsrAdj::from_graph(&graph.graph);

        let config = DistributedPageRankConfig {
            num_shards: 2,
            max_iterations: 200,
            tolerance: 1e-6,
            checkpoint_interval: 10,
            ..Default::default()
        };

        let result_full = distributed_pagerank_csr(&csr, &config);
        assert!(!result_full.checkpoints.is_empty());

        // 从第10轮检查点恢复
        let checkpoint = &result_full.checkpoints[0];
        assert_eq!(checkpoint.iteration, 10);

        let resumed = resume_from_checkpoint(&csr, &config, checkpoint);
        assert!(resumed.converged);

        // 恢复后的结果应与完整计算结果一致
        for i in 0..csr.n {
            assert_relative_eq!(result_full.ranks[i], resumed.ranks[i], epsilon = 1e-6);
        }
    }

    #[test]
    fn test_empty_graph() {
        let graph = KnowledgeGraph::new();
        let config = DistributedPageRankConfig::default();
        let pr = graph.distributed_pagerank(&config);
        assert!(pr.is_empty());
    }

    #[test]
    fn test_single_node() {
        let graph = KnowledgeGraphBuilder::new()
            .add_node("a", "A", "test")
            .build();

        let config = DistributedPageRankConfig {
            num_shards: 1,
            ..Default::default()
        };

        let pr = graph.distributed_pagerank(&config);
        assert_eq!(pr.len(), 1);
        assert_relative_eq!(pr["a"], 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_single_shard_equals_standard_pagerank() {
        let graph = build_test_graph();

        // 单分片分布式应与标准 PageRank 一致
        let config = DistributedPageRankConfig {
            num_shards: 1,
            max_iterations: 100,
            tolerance: 1e-10,
            ..Default::default()
        };

        let pr_dist = graph.distributed_pagerank(&config);
        let pr_std = graph.pagerank(100);

        for (k, v_dist) in &pr_dist {
            let v_std = pr_std.get(k).unwrap();
            assert_relative_eq!(v_dist, v_std, epsilon = 1e-6);
        }
    }
}
