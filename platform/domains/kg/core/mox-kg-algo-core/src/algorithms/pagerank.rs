// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::csr::{rank_vec_to_map, CsrAdj};
use crate::graph::KnowledgeGraph;
use nalgebra::DMatrix;
use std::collections::HashMap;

impl KnowledgeGraph {
    /// PageRank算法
    ///
    /// 修复 R-D2：悬挂节点（出度为 0）的质量此前直接丢失，导致 ΣPR < 1（不守恒）。
    /// 现将悬挂质量均匀回传全图，并加收敛提前终止（容差 1e-6）。
    ///
    /// CSR 新路径（默认）：O(E·iter) 避免 N² dense。
    /// 回滚开关：设环境变量 GRAPH_LEGACY_DENSE=1 则走原 dense 路径。
    pub fn pagerank(&self, iterations: usize) -> HashMap<String, f64> {
        let n = self.node_count();
        if n == 0 {
            return HashMap::new();
        }

        if std::env::var("GRAPH_LEGACY_DENSE").is_ok() {
            return self.pagerank_dense_legacy(iterations);
        }

        let alpha = self.damping_factor;
        let csr = CsrAdj::from_graph(&self.graph);
        let rank = csr.pagerank(alpha, iterations);
        rank_vec_to_map(&rank, &self.node_map)
    }

    /// dense 回滚路径（保留原语义）。
    pub(crate) fn pagerank_dense_legacy(&self, iterations: usize) -> HashMap<String, f64> {
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

        let transition = &deg * &adj;
        let mut rank = DMatrix::from_element(n, 1, 1.0 / n as f64);
        let teleport = 1.0 / n as f64;

        for _ in 0..iterations {
            let dangling_mass: f64 = (0..n).filter(|&i| dangling[i]).map(|i| rank[(i, 0)]).sum();

            let propagated = transition.transpose() * &rank;
            let mut new_rank = propagated * alpha;
            for i in 0..n {
                new_rank[(i, 0)] += alpha * dangling_mass / n as f64 + (1.0 - alpha) * teleport;
            }

            let max_diff: f64 = (0..n)
                .map(|i| (new_rank[(i, 0)] - rank[(i, 0)]).abs())
                .fold(0.0, f64::max);
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
    /// a_i = (1-d)·p_i + d·(Σ_{j→i} a_j·W(j,i)/outW(j) + dangling_mass·p[i])
    /// p 为个性化向量（命中关键词按权重归一），和为 1。
    ///
    /// 默认走 CSR；设 GRAPH_LEGACY_DENSE=1 走原 dense 路径。
    pub fn pagerank_personalized(
        &self,
        personalization: &HashMap<String, f64>,
        iterations: usize,
    ) -> HashMap<String, f64> {
        let n = self.node_count();
        if n == 0 {
            return HashMap::new();
        }
        if std::env::var("GRAPH_LEGACY_DENSE").is_ok() {
            return self.ppr_dense_legacy(personalization, iterations);
        }

        let alpha = self.damping_factor;

        // 个性化向量
        let mut p = vec![0.0f64; n];
        let total: f64 = personalization.values().sum();
        if total > 1e-15 {
            for (id, w) in personalization {
                if let Some(&idx) = self.node_map.get(id) {
                    p[idx.index()] = w / total;
                }
            }
        } else {
            for v in p.iter_mut() {
                *v = 1.0 / n as f64;
            }
        }

        let csr = CsrAdj::from_graph(&self.graph);
        let rank = csr.pagerank_personalized(alpha, iterations, &p);
        rank_vec_to_map(&rank, &self.node_map)
    }

    pub(crate) fn ppr_dense_legacy(
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

        let mut p = vec![0.0f64; n];
        let total: f64 = personalization.values().sum();
        if total > 1e-15 {
            for (id, w) in personalization {
                if let Some(&idx) = self.node_map.get(id) {
                    p[idx.index()] = w / total;
                }
            }
        } else {
            for v in p.iter_mut() {
                *v = 1.0 / n as f64;
            }
        }

        let transition = &deg * &adj;
        let mut rank: DMatrix<f64> = DMatrix::from_column_slice(n, 1, &p);

        for _ in 0..iterations {
            let dangling_mass: f64 = (0..n).filter(|&i| dangling[i]).map(|i| rank[(i, 0)]).sum();

            let propagated = transition.transpose() * &rank;
            let mut new_rank = propagated * alpha;
            for i in 0..n {
                new_rank[(i, 0)] += alpha * dangling_mass * p[i] + (1.0 - alpha) * p[i];
            }

            let max_diff: f64 = (0..n)
                .map(|i| (new_rank[(i, 0)] - rank[(i, 0)]).abs())
                .fold(0.0, f64::max);
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

    // ========================================================================
    // 增强 PageRank：死端 / 蜘蛛陷阱 / 幂法加速
    // ========================================================================

    /// 增强型 PageRank（带死端处理、蜘蛛陷阱检测、幂法加速）
    ///
    /// # 死端（Dead-end）处理
    /// 出度为 0 的悬挂节点会吸收 PageRank 质量。
    /// 已通过阻尼因子 + 悬挂质量回传机制处理，保证 PR 守恒。
    ///
    /// # 蜘蛛陷阱（Spider-trap）检测
    /// 自环或小环结构会导致 PR 质量被困。
    /// 可通过 `detect_spider_trap_nodes` 方法检测疑似陷阱节点。
    ///
    /// # 幂法加速（Anderson Acceleration）
    /// 使用 Anderson 混合加速技术，结合历史迭代信息构造最优外推，
    /// 可显著加速收敛（通常 2-5 倍迭代减少）。
    ///
    /// # 参数
    /// - `tolerance`: 收敛阈值（默认 1e-6）
    /// - `max_iterations`: 最大迭代次数（默认 100）
    /// - `use_acceleration`: 是否启用 Anderson 加速
    pub fn pagerank_enhanced(
        &self,
        tolerance: f64,
        max_iterations: usize,
        use_acceleration: bool,
    ) -> HashMap<String, f64> {
        let n = self.node_count();
        if n == 0 {
            return HashMap::new();
        }

        let alpha = self.damping_factor;
        let csr = CsrAdj::from_graph(&self.graph);
        let (rank, _, _) = csr.pagerank_enhanced(alpha, tolerance, max_iterations, use_acceleration);
        rank_vec_to_map(&rank, &self.node_map)
    }

    /// 增强型 PageRank（返回完整信息：rank、迭代次数、是否收敛）
    pub fn pagerank_enhanced_full(
        &self,
        tolerance: f64,
        max_iterations: usize,
        use_acceleration: bool,
    ) -> (HashMap<String, f64>, usize, bool) {
        let n = self.node_count();
        if n == 0 {
            return (HashMap::new(), 0, true);
        }

        let alpha = self.damping_factor;
        let csr = CsrAdj::from_graph(&self.graph);
        let (rank, iters, converged) =
            csr.pagerank_enhanced(alpha, tolerance, max_iterations, use_acceleration);
        let result = rank_vec_to_map(&rank, &self.node_map);
        (result, iters, converged)
    }

    /// 增强型个性化 PageRank
    ///
    /// 支持 Anderson 加速和更灵活的收敛控制。
    pub fn pagerank_personalized_enhanced(
        &self,
        personalization: &HashMap<String, f64>,
        tolerance: f64,
        max_iterations: usize,
        use_acceleration: bool,
    ) -> HashMap<String, f64> {
        let n = self.node_count();
        if n == 0 {
            return HashMap::new();
        }

        let alpha = self.damping_factor;

        let mut p = vec![0.0f64; n];
        let total: f64 = personalization.values().sum();
        if total > 1e-15 {
            for (id, w) in personalization {
                if let Some(&idx) = self.node_map.get(id) {
                    p[idx.index()] = w / total;
                }
            }
        } else {
            for v in p.iter_mut() {
                *v = 1.0 / n as f64;
            }
        }

        let csr = CsrAdj::from_graph(&self.graph);
        let (rank, _, _) = csr.pagerank_personalized_enhanced(
            alpha,
            &p,
            tolerance,
            max_iterations,
            use_acceleration,
        );
        rank_vec_to_map(&rank, &self.node_map)
    }

    /// 检测悬挂节点（死端节点，出度为 0）
    ///
    /// 悬挂节点没有出边，会吸收所有到达的 PageRank 质量。
    /// 标准 PageRank 通过阻尼因子和悬挂质量回传处理此问题。
    pub fn dangling_nodes(&self) -> Vec<String> {
        let csr = CsrAdj::from_graph(&self.graph);
        let dangling = csr.dangling_nodes();
        dangling
            .into_iter()
            .map(|idx| self.graph[petgraph::graph::NodeIndex::new(idx)].id.clone())
            .collect()
    }

    /// 检测蜘蛛陷阱节点
    ///
    /// 蜘蛛陷阱是指出度很少但入度很多的强连通分量（SCC），
    /// PageRank 质量进入后难以流出，导致收敛缓慢。
    ///
    /// 检测标准：
    /// - 多节点 SCC：外部出边占比 < 10%
    /// - 单节点自环：出度 = 1（自环）且入度 > 1
    pub fn detect_spider_trap_nodes(&self) -> Vec<String> {
        let csr = CsrAdj::from_graph(&self.graph);
        let traps = csr.detect_spider_traps();
        traps
            .into_iter()
            .map(|idx| self.graph[petgraph::graph::NodeIndex::new(idx)].id.clone())
            .collect()
    }

    /// PageRank 收敛速度对比（加速 vs 标准）
    ///
    /// 返回 (标准迭代数, 加速迭代数, 加速比)
    pub fn pagerank_acceleration_benchmark(&self, tolerance: f64) -> (usize, usize, f64) {
        let n = self.node_count();
        if n == 0 {
            return (0, 0, 1.0);
        }

        let alpha = self.damping_factor;
        let csr = CsrAdj::from_graph(&self.graph);
        let max_iter = 1000;

        let (_, iters_std, _) = csr.pagerank_enhanced(alpha, tolerance, max_iter, false);
        let (_, iters_acc, _) = csr.pagerank_enhanced(alpha, tolerance, max_iter, true);

        let speedup = if iters_acc > 0 {
            iters_std as f64 / iters_acc as f64
        } else {
            1.0
        };

        (iters_std, iters_acc, speedup)
    }
}
