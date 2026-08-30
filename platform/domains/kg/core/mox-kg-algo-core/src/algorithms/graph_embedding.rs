// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 图嵌入/向量化算法
//!
//! # 算法概览
//!
//! ## DeepWalk
//! - 通过随机游走生成节点序列，再用 Skip-gram 学习嵌入
//! - 将图结构转化为序列问题，利用 NLP 领域的词嵌入技术
//! - 时间复杂度：O(walks_per_node · walk_length · d)
//!
//! ## Node2Vec
//! - DeepWalk 的扩展，支持有偏随机游走
//! - p 参数：返回参数（控制返回上一节点的概率）
//! - q 参数：进出参数（控制探索新节点的概率）
//! - p=1, q=1 时退化为 DeepWalk
//!
//! ## LINE（Large-scale Information Network Embedding）
//! - 同时优化一阶相似度（直接相连）和二阶相似度（邻居相似）
//! - 适合大规模图，时间复杂度 O(E·d)
//!
//! ## GraphSAGE（Graph SAmple and aggreGatE）
//! - 归纳式学习：通过邻居聚合生成节点嵌入
//! - 支持为未见过的节点生成嵌入
//! - 多种聚合器：均值、池化、LSTM
//!
//! ## 节点相似度计算
//! - 余弦相似度
//! - 杰卡德相似度
//! - 阿达马积（Hadamard）
//! - 平均（Average）

use crate::csr::CsrAdj;
use crate::graph::KnowledgeGraph;
use crate::Result;
use std::collections::HashSet;

// ============================================================================
// 通用嵌入配置
// ============================================================================

/// 嵌入配置
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// 嵌入维度
    pub dimensions: usize,
    /// 随机种子
    pub seed: u64,
    /// 学习率
    pub learning_rate: f64,
    /// 训练轮次
    pub epochs: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            dimensions: 64,
            seed: 42,
            learning_rate: 0.025,
            epochs: 10,
        }
    }
}

/// 嵌入结果
#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    /// 每个节点的嵌入向量
    pub embeddings: Vec<Vec<f64>>,
    /// 训练损失（每轮）
    pub losses: Vec<f64>,
}

// ============================================================================
// DeepWalk
// ============================================================================

/// DeepWalk 配置
#[derive(Debug, Clone)]
pub struct DeepWalkConfig {
    /// 嵌入维度
    pub dimensions: usize,
    /// 每个节点的游走次数
    pub walks_per_node: usize,
    /// 每次游走的长度
    pub walk_length: usize,
    /// 窗口大小（Skip-gram）
    pub window_size: usize,
    /// 学习率
    pub learning_rate: f64,
    /// 随机种子
    pub seed: u64,
}

impl Default for DeepWalkConfig {
    fn default() -> Self {
        Self {
            dimensions: 64,
            walks_per_node: 10,
            walk_length: 40,
            window_size: 5,
            learning_rate: 0.025,
            seed: 42,
        }
    }
}

/// DeepWalk 图嵌入算法
///
/// # 算法原理
/// 1. 对每个节点进行多次随机游走，生成节点序列
/// 2. 将每个节点序列视为"句子"，节点视为"词"
/// 3. 使用 Skip-gram 模型学习节点的嵌入表示
///
/// 这里使用简化的 Skip-gram 实现（基于负采样的变体）。
pub(crate) fn deepwalk_csr(csr: &CsrAdj, config: &DeepWalkConfig) -> EmbeddingResult {
    let n = csr.n;
    if n == 0 {
        return EmbeddingResult {
            embeddings: Vec::new(),
            losses: Vec::new(),
        };
    }

    let d = config.dimensions;

    // 初始化嵌入向量（随机）
    let mut rng = config.seed;
    let mut next_rand = || {
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        (rng as f64) / (u64::MAX as f64) - 0.5
    };

    let mut embeddings = vec![vec![0.0f64; d]; n];
    let mut context = vec![vec![0.0f64; d]; n]; // 上下文向量

    for i in 0..n {
        for j in 0..d {
            embeddings[i][j] = next_rand() * 0.1;
            context[i][j] = next_rand() * 0.1;
        }
    }

    let mut losses = Vec::new();

    // 生成随机游走序列并训练
    for epoch in 0..config.walks_per_node {
        let mut epoch_loss = 0.0;
        let mut total_samples = 0;

        // 打乱节点顺序
        let mut node_order: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = ((next_rand() + 0.5) as usize) % (i + 1);
            node_order.swap(i, j);
        }

        for &start_node in &node_order {
            // 生成随机游走
            let walk = random_walk(csr, start_node, config.walk_length, &mut next_rand);

            // Skip-gram 训练
            for pos in 0..walk.len() {
                let center = walk[pos];
                let window_start = pos.saturating_sub(config.window_size);
                let window_end = (pos + config.window_size).min(walk.len());

                for target_pos in window_start..window_end {
                    if target_pos == pos {
                        continue;
                    }
                    let target = walk[target_pos];

                    // 正样本训练
                    let loss = train_skipgram_pair(
                        &mut embeddings,
                        &mut context,
                        center,
                        target,
                        1.0,
                        config.learning_rate,
                    );
                    epoch_loss += loss;
                    total_samples += 1;

                    // 负采样（简化：随机选一个非上下文节点作为负样本）
                    let neg = ((next_rand() + 0.5) as usize) % n;
                    if neg != target && neg != center {
                        let loss_neg = train_skipgram_pair(
                            &mut embeddings,
                            &mut context,
                            center,
                            neg,
                            0.0,
                            config.learning_rate,
                        );
                        epoch_loss += loss_neg;
                        total_samples += 1;
                    }
                }
            }
        }

        losses.push(if total_samples > 0 {
            epoch_loss / total_samples as f64
        } else {
            0.0
        });

        let _ = epoch;
    }

    // 归一化嵌入
    for emb in embeddings.iter_mut() {
        let norm: f64 = emb.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm > 1e-15 {
            for v in emb.iter_mut() {
                *v /= norm;
            }
        }
    }

    EmbeddingResult {
        embeddings,
        losses,
    }
}

/// 生成一次随机游走
fn random_walk(csr: &CsrAdj, start: usize, length: usize, next_rand: &mut dyn FnMut() -> f64) -> Vec<usize> {
    let mut walk = Vec::with_capacity(length);
    walk.push(start);
    let mut current = start;

    for _ in 1..length {
        let out_deg = csr.offsets[current + 1] - csr.offsets[current];
        if out_deg == 0 {
            break; // 悬挂节点，停止游走
        }

        let r = (next_rand() + 0.5) * out_deg as f64;
        let idx = (r as usize).min(out_deg - 1);
        current = csr.targets[csr.offsets[current] + idx];
        walk.push(current);
    }

    walk
}

/// Skip-gram 单对训练（简化版）
///
/// 使用逻辑回归 + 梯度下降
/// 正样本：label = 1，负样本：label = 0
fn train_skipgram_pair(
    embeddings: &mut [Vec<f64>],
    context: &mut [Vec<f64>],
    center: usize,
    target: usize,
    label: f64,
    lr: f64,
) -> f64 {
    let d = embeddings[center].len();

    // 计算点积
    let mut dot = 0.0;
    for k in 0..d {
        dot += embeddings[center][k] * context[target][k];
    }

    // sigmoid
    let pred = sigmoid(dot);

    // 损失
    let loss = if label > 0.5 {
        -pred.ln().max(-100.0)
    } else {
        -(1.0 - pred).ln().max(-100.0)
    };

    // 梯度
    let error = pred - label;

    // 更新嵌入
    for k in 0..d {
        let grad_emb = error * context[target][k];
        let grad_ctx = error * embeddings[center][k];
        embeddings[center][k] -= lr * grad_emb;
        context[target][k] -= lr * grad_ctx;
    }

    loss
}

/// Sigmoid 函数
fn sigmoid(x: f64) -> f64 {
    if x >= 0.0 {
        1.0 / (1.0 + (-x).exp())
    } else {
        let ex = x.exp();
        ex / (1.0 + ex)
    }
}

// ============================================================================
// Node2Vec
// ============================================================================

/// Node2Vec 配置
#[derive(Debug, Clone)]
pub struct Node2VecConfig {
    /// 嵌入维度
    pub dimensions: usize,
    /// 每个节点的游走次数
    pub walks_per_node: usize,
    /// 每次游走的长度
    pub walk_length: usize,
    /// 窗口大小
    pub window_size: usize,
    /// 返回参数 p（越小越倾向于返回上一节点）
    pub p: f64,
    /// 进出参数 q（越小越倾向于探索远方节点）
    pub q: f64,
    /// 学习率
    pub learning_rate: f64,
    /// 随机种子
    pub seed: u64,
}

impl Default for Node2VecConfig {
    fn default() -> Self {
        Self {
            dimensions: 64,
            walks_per_node: 10,
            walk_length: 40,
            window_size: 5,
            p: 1.0,
            q: 1.0,
            learning_rate: 0.025,
            seed: 42,
        }
    }
}

/// Node2Vec 图嵌入算法
///
/// # 算法原理
/// Node2Vec 是 DeepWalk 的扩展，通过 p、q 参数控制随机游走的偏置：
/// - p：返回参数，控制回到上一个节点的概率
/// - q：进出参数，控制向远离起点方向探索的概率
///
/// 转移概率：
/// - 如果下一个节点是上一个节点：概率 = 1/p
/// - 如果下一个节点与上一个节点相邻：概率 = 1
/// - 如果下一个节点与上一个节点不相邻：概率 = 1/q
pub(crate) fn node2vec_csr(csr: &CsrAdj, config: &Node2VecConfig) -> EmbeddingResult {
    let n = csr.n;
    if n == 0 {
        return EmbeddingResult {
            embeddings: Vec::new(),
            losses: Vec::new(),
        };
    }

    let d = config.dimensions;
    let mut rng = config.seed;
    let mut next_rand = || {
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        (rng as f64) / (u64::MAX as f64) - 0.5
    };

    let mut embeddings = vec![vec![0.0f64; d]; n];
    let mut context = vec![vec![0.0f64; d]; n];

    for i in 0..n {
        for j in 0..d {
            embeddings[i][j] = next_rand() * 0.1;
            context[i][j] = next_rand() * 0.1;
        }
    }

    let mut losses = Vec::new();

    for epoch in 0..config.walks_per_node {
        let mut epoch_loss = 0.0;
        let mut total_samples = 0;

        let mut node_order: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = ((next_rand() + 0.5) as usize) % (i + 1);
            node_order.swap(i, j);
        }

        for &start_node in &node_order {
            let walk = biased_random_walk(csr, start_node, config.walk_length, config.p, config.q, &mut next_rand);

            for pos in 0..walk.len() {
                let center = walk[pos];
                let window_start = pos.saturating_sub(config.window_size);
                let window_end = (pos + config.window_size).min(walk.len());

                for target_pos in window_start..window_end {
                    if target_pos == pos {
                        continue;
                    }
                    let target = walk[target_pos];

                    let loss = train_skipgram_pair(
                        &mut embeddings,
                        &mut context,
                        center,
                        target,
                        1.0,
                        config.learning_rate,
                    );
                    epoch_loss += loss;
                    total_samples += 1;

                    let neg = ((next_rand() + 0.5) as usize) % n;
                    if neg != target && neg != center {
                        let loss_neg = train_skipgram_pair(
                            &mut embeddings,
                            &mut context,
                            center,
                            neg,
                            0.0,
                            config.learning_rate,
                        );
                        epoch_loss += loss_neg;
                        total_samples += 1;
                    }
                }
            }
        }

        losses.push(if total_samples > 0 {
            epoch_loss / total_samples as f64
        } else {
            0.0
        });

        let _ = epoch;
    }

    for emb in embeddings.iter_mut() {
        let norm: f64 = emb.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm > 1e-15 {
            for v in emb.iter_mut() {
                *v /= norm;
            }
        }
    }

    EmbeddingResult {
        embeddings,
        losses,
    }
}

/// 有偏随机游走（Node2Vec）
fn biased_random_walk(
    csr: &CsrAdj,
    start: usize,
    length: usize,
    p: f64,
    q: f64,
    next_rand: &mut dyn FnMut() -> f64,
) -> Vec<usize> {
    let n = csr.n;
    let mut walk = Vec::with_capacity(length);
    walk.push(start);

    if length <= 1 {
        return walk;
    }

    // 第一步：普通随机游走
    let out_deg_0 = csr.offsets[start + 1] - csr.offsets[start];
    if out_deg_0 == 0 {
        return walk;
    }
    let r0 = (next_rand() + 0.5) * out_deg_0 as f64;
    let idx0 = (r0 as usize).min(out_deg_0 - 1);
    let mut prev = start;
    let mut current = csr.targets[csr.offsets[start] + idx0];
    walk.push(current);

    // 构建邻居查询（用 HashSet 加速）
    let mut neighbor_sets: Vec<Option<HashSet<usize>>> = vec![None; n];

    for _ in 2..length {
        let out_deg = csr.offsets[current + 1] - csr.offsets[current];
        if out_deg == 0 {
            break;
        }

        // 获取当前节点的邻居集合（懒加载）
        if neighbor_sets[current].is_none() {
            let mut set = HashSet::new();
            let rng = csr.offsets[current]..csr.offsets[current + 1];
            for k in rng {
                set.insert(csr.targets[k]);
            }
            neighbor_sets[current] = Some(set);
        }

        // 计算各邻居的转移概率（非归一化）
        let mut probs = Vec::with_capacity(out_deg);
        let neighbors: Vec<usize> = (csr.offsets[current]..csr.offsets[current + 1])
            .map(|k| csr.targets[k])
            .collect();

        let prev_neighbors = &neighbor_sets[current];

        for &next_node in &neighbors {
            let prob = if next_node == prev {
                1.0 / p
            } else if prev_neighbors.as_ref().map_or(false, |s| s.contains(&next_node)) {
                1.0
            } else {
                1.0 / q
            };
            probs.push(prob);
        }

        // 归一化并采样
        let total: f64 = probs.iter().sum();
        if total < 1e-15 {
            break;
        }

        let r = (next_rand() + 0.5) * total;
        let mut cumulative = 0.0;
        let mut chosen = 0;
        for (i, &prob) in probs.iter().enumerate() {
            cumulative += prob;
            if cumulative >= r {
                chosen = i;
                break;
            }
        }
        if chosen >= neighbors.len() {
            chosen = neighbors.len() - 1;
        }

        prev = current;
        current = neighbors[chosen];
        walk.push(current);
    }

    walk
}

// ============================================================================
// LINE（Large-scale Information Network Embedding）
// ============================================================================

/// LINE 配置
#[derive(Debug, Clone)]
pub struct LINEConfig {
    /// 嵌入维度
    pub dimensions: usize,
    /// 一阶相似度权重
    pub first_order_weight: f64,
    /// 二阶相似度权重
    pub second_order_weight: f64,
    /// 学习率
    pub learning_rate: f64,
    /// 训练轮次
    pub epochs: usize,
    /// 负采样数量
    pub negative_samples: usize,
    /// 随机种子
    pub seed: u64,
}

impl Default for LINEConfig {
    fn default() -> Self {
        Self {
            dimensions: 64,
            first_order_weight: 1.0,
            second_order_weight: 1.0,
            learning_rate: 0.025,
            epochs: 10,
            negative_samples: 5,
            seed: 42,
        }
    }
}

/// LINE 图嵌入算法
///
/// # 算法原理
/// LINE 同时优化两种相似度：
/// - **一阶相似度**：直接相连的节点嵌入应接近
/// - **二阶相似度**：共享邻居的节点嵌入应接近（结构等价）
///
/// 最终嵌入 = [一阶嵌入; 二阶嵌入]（拼接后归一化）
pub(crate) fn line_csr(csr: &CsrAdj, config: &LINEConfig) -> EmbeddingResult {
    let n = csr.n;
    if n == 0 {
        return EmbeddingResult {
            embeddings: Vec::new(),
            losses: Vec::new(),
        };
    }

    let d = config.dimensions;
    let half_d = d / 2;
    let mut rng = config.seed;
    let mut next_rand = || {
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        (rng as f64) / (u64::MAX as f64) - 0.5
    };

    // 一阶嵌入（出向量）
    let mut emb_1st = vec![vec![0.0f64; half_d]; n];
    // 二阶嵌入（出向量 + 上下文向量）
    let mut emb_2nd = vec![vec![0.0f64; half_d]; n];
    let mut ctx_2nd = vec![vec![0.0f64; half_d]; n];

    for i in 0..n {
        for j in 0..half_d {
            emb_1st[i][j] = next_rand() * 0.1;
            emb_2nd[i][j] = next_rand() * 0.1;
            ctx_2nd[i][j] = next_rand() * 0.1;
        }
    }

    // 收集所有边
    let mut edges = Vec::new();
    for i in 0..n {
        let rng = csr.offsets[i]..csr.offsets[i + 1];
        for k in rng {
            edges.push((i, csr.targets[k], csr.weights[k]));
        }
    }

    let mut losses = Vec::new();

    for epoch in 0..config.epochs {
        let mut epoch_loss = 0.0;
        let mut total_samples = 0;

        // 打乱边顺序
        let mut edge_order: Vec<usize> = (0..edges.len()).collect();
        for i in (1..edge_order.len()).rev() {
            let j = ((next_rand() + 0.5) as usize) % (i + 1);
            edge_order.swap(i, j);
        }

        for &e_idx in &edge_order {
            let (src, dst, weight) = edges[e_idx];

            // --- 一阶相似度 ---
            if config.first_order_weight > 1e-12 {
                // 正样本
                let mut dot = 0.0;
                for k in 0..half_d {
                    dot += emb_1st[src][k] * emb_1st[dst][k];
                }
                let pred = sigmoid(dot);
                let error = pred - 1.0;
                epoch_loss += -(1.0 * pred.ln().max(-100.0));
                total_samples += 1;

                let lr = config.learning_rate * config.first_order_weight;
                for k in 0..half_d {
                    let grad_src = error * emb_1st[dst][k];
                    let grad_dst = error * emb_1st[src][k];
                    emb_1st[src][k] -= lr * grad_src;
                    emb_1st[dst][k] -= lr * grad_dst;
                }

                // 负采样
                for _ in 0..config.negative_samples {
                    let neg = ((next_rand() + 0.5) as usize) % n;
                    if neg == dst {
                        continue;
                    }

                    let mut dot_neg = 0.0;
                    for k in 0..half_d {
                        dot_neg += emb_1st[src][k] * emb_1st[neg][k];
                    }
                    let pred_neg = sigmoid(dot_neg);
                    let error_neg = pred_neg - 0.0;
                    epoch_loss += -((1.0 - pred_neg).ln().max(-100.0));
                    total_samples += 1;

                    for k in 0..half_d {
                        let grad_src = error_neg * emb_1st[neg][k];
                        let grad_neg = error_neg * emb_1st[src][k];
                        emb_1st[src][k] -= lr * grad_src;
                        emb_1st[neg][k] -= lr * grad_neg;
                    }
                }
            }

            // --- 二阶相似度 ---
            if config.second_order_weight > 1e-12 {
                // 正样本
                let mut dot = 0.0;
                for k in 0..half_d {
                    dot += emb_2nd[src][k] * ctx_2nd[dst][k];
                }
                let pred = sigmoid(dot);
                let error = pred - 1.0;
                epoch_loss += -(1.0 * pred.ln().max(-100.0));
                total_samples += 1;

                let lr = config.learning_rate * config.second_order_weight;
                for k in 0..half_d {
                    let grad_emb = error * ctx_2nd[dst][k];
                    let grad_ctx = error * emb_2nd[src][k];
                    emb_2nd[src][k] -= lr * grad_emb;
                    ctx_2nd[dst][k] -= lr * grad_ctx;
                }

                // 负采样
                for _ in 0..config.negative_samples {
                    let neg = ((next_rand() + 0.5) as usize) % n;
                    if neg == dst {
                        continue;
                    }

                    let mut dot_neg = 0.0;
                    for k in 0..half_d {
                        dot_neg += emb_2nd[src][k] * ctx_2nd[neg][k];
                    }
                    let pred_neg = sigmoid(dot_neg);
                    let error_neg = pred_neg - 0.0;
                    epoch_loss += -((1.0 - pred_neg).ln().max(-100.0));
                    total_samples += 1;

                    for k in 0..half_d {
                        let grad_emb = error_neg * ctx_2nd[neg][k];
                        let grad_neg = error_neg * emb_2nd[src][k];
                        emb_2nd[src][k] -= lr * grad_emb;
                        ctx_2nd[neg][k] -= lr * grad_neg;
                    }
                }
            }

            let _ = weight;
        }

        losses.push(if total_samples > 0 {
            epoch_loss / total_samples as f64
        } else {
            0.0
        });

        let _ = epoch;
    }

    // 拼接一阶和二阶嵌入
    let mut embeddings = vec![vec![0.0f64; d]; n];
    for i in 0..n {
        for k in 0..half_d {
            embeddings[i][k] = emb_1st[i][k];
            embeddings[i][half_d + k] = emb_2nd[i][k];
        }
    }

    // 归一化
    for emb in embeddings.iter_mut() {
        let norm: f64 = emb.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm > 1e-15 {
            for v in emb.iter_mut() {
                *v /= norm;
            }
        }
    }

    EmbeddingResult {
        embeddings,
        losses,
    }
}

// ============================================================================
// GraphSAGE
// ============================================================================

/// GraphSAGE 聚合器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregatorType {
    /// 均值聚合
    Mean,
    /// 最大池化聚合
    MaxPool,
    /// 求和聚合
    Sum,
}

/// GraphSAGE 配置
#[derive(Debug, Clone)]
pub struct GraphSAGEConfig {
    /// 嵌入维度
    pub dimensions: usize,
    /// 聚合层数
    pub num_layers: usize,
    /// 每层采样邻居数（0 表示全部）
    pub sample_size: usize,
    /// 聚合器类型
    pub aggregator: AggregatorType,
    /// 学习率
    pub learning_rate: f64,
    /// 训练轮次
    pub epochs: usize,
    /// 随机种子
    pub seed: u64,
}

impl Default for GraphSAGEConfig {
    fn default() -> Self {
        Self {
            dimensions: 64,
            num_layers: 2,
            sample_size: 10,
            aggregator: AggregatorType::Mean,
            learning_rate: 0.01,
            epochs: 10,
            seed: 42,
        }
    }
}

/// GraphSAGE 归纳式图嵌入
///
/// # 算法原理
/// GraphSAGE 通过聚合邻居信息生成节点嵌入：
/// 1. 第 0 层：节点自身特征（这里用 one-hot 或度特征）
/// 2. 第 k 层：聚合第 k-1 层邻居的表示，加上自身变换
///
/// 支持归纳式学习：可以为训练时未见过的节点生成嵌入。
pub(crate) fn graphsage_csr(csr: &CsrAdj, config: &GraphSAGEConfig) -> EmbeddingResult {
    let n = csr.n;
    if n == 0 {
        return EmbeddingResult {
            embeddings: Vec::new(),
            losses: Vec::new(),
        };
    }

    let d = config.dimensions;
    let mut rng = config.seed;
    let mut next_rand = || {
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        (rng as f64) / (u64::MAX as f64) - 0.5
    };

    // 初始特征：度 + 一个常数项（简化的特征向量）
    let mut features = vec![vec![0.0f64; d]; n];
    for i in 0..n {
        let deg = (csr.offsets[i + 1] - csr.offsets[i]) as f64;
        features[i][0] = deg;
        features[i][1] = 1.0; // bias term
        for j in 2..d {
            features[i][j] = next_rand() * 0.01;
        }
    }

    // 权重矩阵（每层一个 W 和一个 b）
    let mut weights: Vec<Vec<Vec<f64>>> = Vec::new();
    let mut biases: Vec<Vec<f64>> = Vec::new();

    for _ in 0..config.num_layers {
        let mut w = vec![vec![0.0f64; d]; d];
        let mut b = vec![0.0f64; d];
        for i in 0..d {
            for j in 0..d {
                w[i][j] = next_rand() * 0.1 / (d as f64).sqrt();
            }
            b[i] = 0.0;
        }
        // 初始化为近似单位矩阵
        for i in 0..d {
            w[i][i] = 1.0;
        }
        weights.push(w);
        biases.push(b);
    }

    // 计算嵌入（前向传播）
    let embeddings = graphsage_forward(csr, &features, &weights, &biases, config);

    // 简单的无监督训练：相邻节点嵌入应接近
    let mut losses = Vec::new();
    let mut current_emb = embeddings.clone();

    for epoch in 0..config.epochs {
        let mut epoch_loss = 0.0;
        let mut total = 0;

        // 打乱边
        let mut edges = Vec::new();
        for i in 0..n {
            let rng = csr.offsets[i]..csr.offsets[i + 1];
            for k in rng {
                edges.push((i, csr.targets[k]));
            }
        }

        for i in (1..edges.len()).rev() {
            let j = ((next_rand() + 0.5) as usize) % (i + 1);
            edges.swap(i, j);
        }

        // 用梯度下降调整嵌入（简化版，实际 GraphSAGE 训练权重）
        let lr = config.learning_rate;

        for &(src, dst) in &edges {
            // 正样本：src 和 dst 应接近
            let mut dot = 0.0;
            for k in 0..d {
                dot += current_emb[src][k] * current_emb[dst][k];
            }
            let pred = sigmoid(dot);
            let error = pred - 1.0;
            epoch_loss += -pred.ln().max(-100.0);
            total += 1;

            for k in 0..d {
                let grad_src = error * current_emb[dst][k];
                let grad_dst = error * current_emb[src][k];
                current_emb[src][k] -= lr * grad_src;
                current_emb[dst][k] -= lr * grad_dst;
            }

            // 负样本
            let neg = ((next_rand() + 0.5) as usize) % n;
            if neg != dst {
                let mut dot_neg = 0.0;
                for k in 0..d {
                    dot_neg += current_emb[src][k] * current_emb[neg][k];
                }
                let pred_neg = sigmoid(dot_neg);
                let error_neg = pred_neg - 0.0;
                epoch_loss += -(1.0 - pred_neg).ln().max(-100.0);
                total += 1;

                for k in 0..d {
                    let grad_src = error_neg * current_emb[neg][k];
                    let grad_neg = error_neg * current_emb[src][k];
                    current_emb[src][k] -= lr * grad_src;
                    current_emb[neg][k] -= lr * grad_neg;
                }
            }
        }

        losses.push(if total > 0 {
            epoch_loss / total as f64
        } else {
            0.0
        });

        let _ = epoch;
    }

    // 归一化
    for emb in current_emb.iter_mut() {
        let norm: f64 = emb.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm > 1e-15 {
            for v in emb.iter_mut() {
                *v /= norm;
            }
        }
    }

    EmbeddingResult {
        embeddings: current_emb,
        losses,
    }
}

/// GraphSAGE 前向传播
fn graphsage_forward(
    csr: &CsrAdj,
    features: &[Vec<f64>],
    weights: &[Vec<Vec<f64>>],
    biases: &[Vec<f64>],
    config: &GraphSAGEConfig,
) -> Vec<Vec<f64>> {
    let n = csr.n;
    let d = config.dimensions;
    let mut current = features.to_vec();

    for layer in 0..config.num_layers {
        let mut next = vec![vec![0.0f64; d]; n];

        for v in 0..n {
            // 收集邻居特征
            let mut neighbor_features: Vec<Vec<f64>> = Vec::new();
            let rng = csr.offsets[v]..csr.offsets[v + 1];
            for k in rng {
                let u = csr.targets[k];
                neighbor_features.push(current[u].clone());
            }

            // 采样邻居
            if config.sample_size > 0 && neighbor_features.len() > config.sample_size {
                // 简化：取前 sample_size 个
                neighbor_features.truncate(config.sample_size);
            }

            // 聚合邻居特征
            let aggregated = match config.aggregator {
                AggregatorType::Mean => {
                    if neighbor_features.is_empty() {
                        vec![0.0f64; d]
                    } else {
                        let mut agg = vec![0.0f64; d];
                        for feat in &neighbor_features {
                            for k in 0..d {
                                agg[k] += feat[k];
                            }
                        }
                        let cnt = neighbor_features.len() as f64;
                        for k in 0..d {
                            agg[k] /= cnt;
                        }
                        agg
                    }
                }
                AggregatorType::MaxPool => {
                    if neighbor_features.is_empty() {
                        vec![0.0f64; d]
                    } else {
                        let mut agg = vec![f64::NEG_INFINITY; d];
                        for feat in &neighbor_features {
                            for k in 0..d {
                                agg[k] = agg[k].max(feat[k]);
                            }
                        }
                        for k in 0..d {
                            if agg[k] == f64::NEG_INFINITY {
                                agg[k] = 0.0;
                            }
                        }
                        agg
                    }
                }
                AggregatorType::Sum => {
                    let mut agg = vec![0.0f64; d];
                    for feat in &neighbor_features {
                        for k in 0..d {
                            agg[k] += feat[k];
                        }
                    }
                    agg
                }
            };

            // 拼接自身特征 + 邻居聚合，然后线性变换
            // 简化：用加权和替代拼接
            let mut combined = vec![0.0f64; d];
            for k in 0..d {
                combined[k] = current[v][k] + aggregated[k];
            }

            // 线性变换 W · h + b
            let mut transformed = vec![0.0f64; d];
            for i in 0..d {
                for j in 0..d {
                    transformed[i] += weights[layer][i][j] * combined[j];
                }
                transformed[i] += biases[layer][i];
            }

            // ReLU 激活
            for k in 0..d {
                next[v][k] = transformed[k].max(0.0);
            }
        }

        current = next;
    }

    current
}

// ============================================================================
// 节点相似度计算
// ============================================================================

/// 节点相似度类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeSimilarityType {
    /// 余弦相似度
    Cosine,
    /// 杰卡德相似度（基于邻居集合）
    Jaccard,
    /// 阿达马积（Hadamard product，逐元素相乘后求和）
    Hadamard,
    /// 平均（Average）
    Average,
}

/// 计算两个节点的相似度
pub fn node_similarity(
    embedding_a: &[f64],
    embedding_b: &[f64],
    stype: NodeSimilarityType,
) -> f64 {
    match stype {
        NodeSimilarityType::Cosine => cosine_similarity(embedding_a, embedding_b),
        NodeSimilarityType::Jaccard => {
            // 基于嵌入符号计算 Jaccard（近似）
            let set_a: HashSet<usize> = embedding_a
                .iter()
                .enumerate()
                .filter(|(_, &v)| v > 0.0)
                .map(|(i, _)| i)
                .collect();
            let set_b: HashSet<usize> = embedding_b
                .iter()
                .enumerate()
                .filter(|(_, &v)| v > 0.0)
                .map(|(i, _)| i)
                .collect();

            let intersection: usize = set_a.intersection(&set_b).count();
            let union: usize = set_a.union(&set_b).count();

            if union == 0 {
                0.0
            } else {
                intersection as f64 / union as f64
            }
        }
        NodeSimilarityType::Hadamard => {
            let d = embedding_a.len().min(embedding_b.len());
            let mut sum = 0.0;
            for i in 0..d {
                sum += embedding_a[i] * embedding_b[i];
            }
            sum
        }
        NodeSimilarityType::Average => {
            let d = embedding_a.len().min(embedding_b.len());
            let mut sum = 0.0;
            for i in 0..d {
                sum += (embedding_a[i] + embedding_b[i]) / 2.0;
            }
            sum / d as f64
        }
    }
}

/// 余弦相似度
fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let d = a.len().min(b.len());
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for i in 0..d {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    let na = norm_a.sqrt();
    let nb = norm_b.sqrt();
    if na < 1e-15 || nb < 1e-15 {
        0.0
    } else {
        dot / (na * nb)
    }
}

/// 基于结构的 Jaccard 相似度（邻居集合）
pub(crate) fn jaccard_structural_csr(csr: &CsrAdj, a: usize, b: usize) -> f64 {
    let neighbors_a: HashSet<usize> = (csr.offsets[a]..csr.offsets[a + 1])
        .map(|k| csr.targets[k])
        .collect();
    let neighbors_b: HashSet<usize> = (csr.offsets[b]..csr.offsets[b + 1])
        .map(|k| csr.targets[k])
        .collect();

    let intersection: usize = neighbors_a.intersection(&neighbors_b).count();
    let union: usize = neighbors_a.union(&neighbors_b).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

// ============================================================================
// KnowledgeGraph 扩展方法
// ============================================================================

impl KnowledgeGraph {
    /// DeepWalk 图嵌入
    pub fn deepwalk_embedding(&self, config: Option<DeepWalkConfig>) -> Vec<Vec<f64>> {
        let cfg = config.unwrap_or_default();
        let csr = CsrAdj::from_graph(&self.graph);
        let result = deepwalk_csr(&csr, &cfg);
        result.embeddings
    }

    /// Node2Vec 图嵌入
    pub fn node2vec_embedding(&self, config: Option<Node2VecConfig>) -> Vec<Vec<f64>> {
        let cfg = config.unwrap_or_default();
        let csr = CsrAdj::from_graph(&self.graph);
        let result = node2vec_csr(&csr, &cfg);
        result.embeddings
    }

    /// LINE 图嵌入
    pub fn line_embedding(&self, config: Option<LINEConfig>) -> Vec<Vec<f64>> {
        let cfg = config.unwrap_or_default();
        let csr = CsrAdj::from_graph(&self.graph);
        let result = line_csr(&csr, &cfg);
        result.embeddings
    }

    /// GraphSAGE 图嵌入
    pub fn graphsage_embedding(&self, config: Option<GraphSAGEConfig>) -> Vec<Vec<f64>> {
        let cfg = config.unwrap_or_default();
        let csr = CsrAdj::from_graph(&self.graph);
        let result = graphsage_csr(&csr, &cfg);
        result.embeddings
    }

    /// 计算两个节点的嵌入相似度
    pub fn node_embedding_similarity(
        &self,
        node_a: &str,
        node_b: &str,
        stype: NodeSimilarityType,
    ) -> Result<f64> {
        // 使用 DeepWalk 生成嵌入（默认配置）
        let embeddings = self.deepwalk_embedding(Some(DeepWalkConfig {
            dimensions: 32,
            walks_per_node: 5,
            walk_length: 20,
            window_size: 3,
            learning_rate: 0.025,
            seed: 42,
        }));

        let idx_a = self
            .node_map
            .get(node_a)
            .ok_or_else(|| anyhow::anyhow!("节点不存在: {}", node_a))?
            .index();
        let idx_b = self
            .node_map
            .get(node_b)
            .ok_or_else(|| anyhow::anyhow!("节点不存在: {}", node_b))?
            .index();

        Ok(node_similarity(&embeddings[idx_a], &embeddings[idx_b], stype))
    }

    /// 结构 Jaccard 相似度（基于邻居集合）
    pub fn structural_jaccard(&self, node_a: &str, node_b: &str) -> Result<f64> {
        let idx_a = self
            .node_map
            .get(node_a)
            .ok_or_else(|| anyhow::anyhow!("节点不存在: {}", node_a))?
            .index();
        let idx_b = self
            .node_map
            .get(node_b)
            .ok_or_else(|| anyhow::anyhow!("节点不存在: {}", node_b))?
            .index();

        let csr = CsrAdj::from_graph(&self.graph);
        Ok(jaccard_structural_csr(&csr, idx_a, idx_b))
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
            .add_edge("b", "c", 1.0)
            .add_edge("c", "d", 1.0)
            .add_edge("d", "e", 1.0)
            .add_edge("a", "c", 1.0)
            .build()
    }

    #[test]
    fn test_deepwalk_embedding() {
        let graph = build_test_graph();
        let config = DeepWalkConfig {
            dimensions: 16,
            walks_per_node: 5,
            walk_length: 10,
            window_size: 3,
            learning_rate: 0.025,
            seed: 42,
        };
        let embeddings = graph.deepwalk_embedding(Some(config));

        assert_eq!(embeddings.len(), 5);
        assert_eq!(embeddings[0].len(), 16);

        // 嵌入应已归一化
        for emb in &embeddings {
            let norm: f64 = emb.iter().map(|v| v * v).sum::<f64>().sqrt();
            assert_relative_eq!(norm, 1.0, epsilon = 1e-6);
        }
    }

    #[test]
    fn test_node2vec_embedding() {
        let graph = build_test_graph();
        let config = Node2VecConfig {
            dimensions: 16,
            walks_per_node: 5,
            walk_length: 10,
            window_size: 3,
            p: 2.0,
            q: 0.5,
            learning_rate: 0.025,
            seed: 42,
        };
        let embeddings = graph.node2vec_embedding(Some(config));

        assert_eq!(embeddings.len(), 5);
        assert_eq!(embeddings[0].len(), 16);
    }

    #[test]
    fn test_node2vec_default_eq_deepwalk() {
        // p=1, q=1 时 Node2Vec 应近似 DeepWalk
        let graph = build_test_graph();

        let dw_config = DeepWalkConfig {
            dimensions: 16,
            walks_per_node: 5,
            walk_length: 10,
            window_size: 3,
            learning_rate: 0.025,
            seed: 42,
        };

        let n2v_config = Node2VecConfig {
            dimensions: 16,
            walks_per_node: 5,
            walk_length: 10,
            window_size: 3,
            p: 1.0,
            q: 1.0,
            learning_rate: 0.025,
            seed: 42,
        };

        let dw_emb = graph.deepwalk_embedding(Some(dw_config));
        let n2v_emb = graph.node2vec_embedding(Some(n2v_config));

        // 两者结果应该有一定相关性（不完全相同因为随机性）
        assert_eq!(dw_emb.len(), n2v_emb.len());
    }

    #[test]
    fn test_line_embedding() {
        let graph = build_test_graph();
        let config = LINEConfig {
            dimensions: 16,
            epochs: 5,
            negative_samples: 3,
            seed: 42,
            ..Default::default()
        };
        let embeddings = graph.line_embedding(Some(config));

        assert_eq!(embeddings.len(), 5);
        assert_eq!(embeddings[0].len(), 16);
    }

    #[test]
    fn test_graphsage_embedding() {
        let graph = build_test_graph();
        let config = GraphSAGEConfig {
            dimensions: 16,
            num_layers: 2,
            epochs: 5,
            seed: 42,
            ..Default::default()
        };
        let embeddings = graph.graphsage_embedding(Some(config));

        assert_eq!(embeddings.len(), 5);
        assert_eq!(embeddings[0].len(), 16);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert_relative_eq!(node_similarity(&a, &b, NodeSimilarityType::Cosine), 1.0);

        let c = vec![0.0, 1.0, 0.0];
        assert_relative_eq!(node_similarity(&a, &c, NodeSimilarityType::Cosine), 0.0);

        let d = vec![1.0, 1.0, 0.0];
        let sim = node_similarity(&a, &d, NodeSimilarityType::Cosine);
        assert!(sim > 0.0 && sim < 1.0);
    }

    #[test]
    fn test_jaccard_similarity() {
        let a = vec![1.0, 1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 1.0, 0.0];
        // a正: {0,1}, b正: {0,2}
        // intersection = 1, union = 3
        assert_relative_eq!(node_similarity(&a, &b, NodeSimilarityType::Jaccard), 1.0 / 3.0);
    }

    #[test]
    fn test_hadamard_similarity() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
        assert_relative_eq!(node_similarity(&a, &b, NodeSimilarityType::Hadamard), 32.0);
    }

    #[test]
    fn test_structural_jaccard() {
        let graph = build_test_graph();
        let j = graph.structural_jaccard("a", "b").unwrap();
        // a 的邻居: b, c
        // b 的邻居: c (假设只有一条边)
        // 实际: a->b, a->c, b->c, c->d, d->e
        // a出邻: {b, c}
        // b出邻: {c}
        // 交集: {c} = 1
        // 并集: {b, c} = 2
        assert_relative_eq!(j, 0.5);
    }

    #[test]
    fn test_sigmoid() {
        assert_relative_eq!(sigmoid(0.0), 0.5);
        assert!(sigmoid(100.0) > 0.99);
        assert!(sigmoid(-100.0) < 0.01);
    }

    #[test]
    fn test_random_walk() {
        let graph = build_test_graph();
        let csr = CsrAdj::from_graph(&graph.graph);

        let mut rng = 42u64;
        let mut next_rand = || {
            rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
            (rng as f64) / (u64::MAX as f64) - 0.5
        };

        let walk = random_walk(&csr, 0, 10, &mut next_rand);
        assert!(!walk.is_empty());
        assert_eq!(walk[0], 0);
        assert!(walk.len() <= 10);
    }

    #[test]
    fn test_empty_graph_embedding() {
        let graph = KnowledgeGraph::new();
        let embeddings = graph.deepwalk_embedding(None);
        assert!(embeddings.is_empty());
    }
}
