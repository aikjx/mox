// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

// ============================================================================
// CSR 稀疏邻接：O(N+E) 表示，避免 O(N²) dense 矩阵。(私有，对外零暴露)
// ============================================================================
/// 按出边组织的 CSR 邻接（出边表 i → j₁,j₂…）
#[derive(Debug, Clone)]
pub(crate) struct CsrAdj {
    pub(crate) n: usize,
    /// offsets[i+1] - offsets[i] == i 的出边数
    pub(crate) offsets: Vec<usize>,
    /// targets[offsets[i]..offsets[i+1]]：i 指向的邻居
    pub(crate) targets: Vec<usize>,
    /// weights[*]：与 targets 一一对应
    pub(crate) weights: Vec<f64>,
    /// out_weight[i] = Σ W(i,·)；0 表示 dangling
    pub(crate) out_weight: Vec<f64>,
    /// true ⟺ 所有边权 == 1.0（此时 closeness 可走 BFS，跳过二叉堆）
    pub(crate) all_unit_weight: bool,
}

impl CsrAdj {
    pub(crate) fn from_graph<N>(g: &DiGraph<N, f64>) -> Self {
        let n = g.node_count();
        let m = g.edge_count();

        let mut out_deg = vec![0usize; n];
        let mut out_weight = vec![0.0f64; n];
        let mut all_unit_weight = true;
        let mut edges: Vec<(usize, usize, f64)> = Vec::with_capacity(m);

        for e in g.edge_references() {
            let i = e.source().index();
            let j = e.target().index();
            let w = *e.weight();
            out_deg[i] += 1;
            out_weight[i] += w;
            if (w - 1.0).abs() > 1e-15 {
                all_unit_weight = false;
            }
            edges.push((i, j, w));
        }

        let mut offsets = vec![0usize; n + 1];
        for i in 0..n {
            offsets[i + 1] = offsets[i] + out_deg[i];
        }
        let mut targets = vec![0usize; m];
        let mut weights = vec![0.0f64; m];
        let mut curs = offsets[0..n].to_vec();
        for (i, j, w) in edges {
            let slot = curs[i];
            curs[i] += 1;
            targets[slot] = j;
            weights[slot] = w;
        }

        Self {
            n,
            offsets,
            targets,
            weights,
            out_weight,
            all_unit_weight,
        }
    }

    /// 标准 PageRank（CSR 推模型）
    pub(crate) fn pagerank(&self, alpha: f64, iterations: usize) -> Vec<f64> {
        let n = self.n;
        if n == 0 {
            return Vec::new();
        }
        let nf = n as f64;
        let mut rank = vec![1.0 / nf; n];
        let teleport = 1.0 / nf;
        let mut propagated = vec![0.0f64; n];
        let mut tmp_send = vec![0.0f64; n];

        for _ in 0..iterations {
            let mut dangling_mass = 0.0;
            for i in 0..n {
                let ow = self.out_weight[i];
                if ow > 1e-15 {
                    tmp_send[i] = rank[i] / ow;
                } else {
                    dangling_mass += rank[i];
                    tmp_send[i] = 0.0;
                }
            }

            for x in propagated.iter_mut() {
                *x = 0.0;
            }
            for (i, &ts) in tmp_send.iter().enumerate().take(n) {
                let rng = self.offsets[i]..self.offsets[i + 1];
                for k in rng {
                    let j = self.targets[k];
                    let w = self.weights[k];
                    propagated[j] += ts * w;
                }
            }

            let mut max_diff = 0.0;
            let dterm = alpha * dangling_mass * teleport;
            let tterm = (1.0 - alpha) * teleport;
            for j in 0..n {
                let new = tterm + alpha * propagated[j] + dterm;
                let d = (new - rank[j]).abs();
                if d > max_diff {
                    max_diff = d;
                }
                rank[j] = new;
            }
            if max_diff < 1e-6 {
                break;
            }
        }
        rank
    }

    /// 个性化 PageRank（CSR）：悬挂质量按 p 分配。
    pub(crate) fn pagerank_personalized(&self, alpha: f64, iterations: usize, p: &[f64]) -> Vec<f64> {
        let n = self.n;
        if n == 0 {
            return Vec::new();
        }
        let mut rank = p.to_vec();
        let mut propagated = vec![0.0f64; n];
        let mut tmp_send = vec![0.0f64; n];

        for _ in 0..iterations {
            let mut dangling_mass = 0.0;
            for i in 0..n {
                let ow = self.out_weight[i];
                if ow > 1e-15 {
                    tmp_send[i] = rank[i] / ow;
                } else {
                    dangling_mass += rank[i];
                    tmp_send[i] = 0.0;
                }
            }

            for x in propagated.iter_mut() {
                *x = 0.0;
            }
            for (i, &ts) in tmp_send.iter().enumerate().take(n) {
                let rng = self.offsets[i]..self.offsets[i + 1];
                for k in rng {
                    let j = self.targets[k];
                    let w = self.weights[k];
                    propagated[j] += ts * w;
                }
            }

            let mut max_diff = 0.0;
            for j in 0..n {
                let pj = p[j];
                let new = alpha * propagated[j] + alpha * dangling_mass * pj + (1.0 - alpha) * pj;
                let d = (new - rank[j]).abs();
                if d > max_diff {
                    max_diff = d;
                }
                rank[j] = new;
            }
            if max_diff < 1e-6 {
                break;
            }
        }
        rank
    }

    // ========================================================================
    // 增强 PageRank：死端处理 / 蜘蛛陷阱 / 幂法加速
    // ========================================================================

    /// 增强型 PageRank（带死端和蜘蛛陷阱处理 + 幂法加速）
    ///
    /// # 死端（Dead-end）处理
    /// 出度为 0 的节点（悬挂节点）会吸收所有 PageRank 质量。
    /// 处理方式：将悬挂节点的质量按 teleport 向量均匀回传。
    /// （标准 PageRank 已包含此处理，此处优化了收敛速度）
    ///
    /// # 蜘蛛陷阱（Spider-trap）处理
    /// 自环或小环结构会导致 PageRank 质量被困在局部。
    /// 处理方式：通过阻尼因子 α 保证全局随机跳转，
    /// 并使用自适应收敛检测避免在陷阱附近振荡。
    ///
    /// # 幂法加速（Anderson 混合加速）
    /// 使用 Anderson Acceleration 技术加速幂迭代收敛：
    /// 结合前 m 步的历史信息，构造最优线性组合，
    /// 可将收敛速度提升 2-5 倍。
    pub(crate) fn pagerank_enhanced(
        &self,
        alpha: f64,
        tol: f64,
        max_iter: usize,
        use_acceleration: bool,
    ) -> (Vec<f64>, usize, bool) {
        let n = self.n;
        if n == 0 {
            return (Vec::new(), 0, true);
        }
        let nf = n as f64;
        let teleport = 1.0 / nf;

        if !use_acceleration {
            // 标准幂法（优化版，带自适应收敛检测）
            return self.pagerank_standard(alpha, tol, max_iter);
        }

        // Anderson Acceleration（m = 5 历史步）
        let m = 5;
        let mut rank = vec![1.0 / nf; n];
        let mut history: Vec<Vec<f64>> = Vec::with_capacity(m + 1);
        let mut residuals: Vec<Vec<f64>> = Vec::with_capacity(m + 1);

        let mut converged = false;
        let mut final_iter = 0;

        for iter in 0..max_iter {
            final_iter = iter + 1;

            // 一步幂迭代
            let (new_rank, residual) = self.pagerank_step(&rank, alpha, teleport);

            // 收敛检测
            let max_diff = residual.iter().cloned().fold(0.0, f64::max);
            if max_diff < tol {
                rank = new_rank;
                converged = true;
                break;
            }

            // Anderson 加速
            if iter >= m {
                // 构造并求解最小二乘问题
                let accelerated = self.anderson_extrapolate(
                    &new_rank,
                    &residual,
                    &history,
                    &residuals,
                    m,
                );

                // 验证加速后的结果是否更优
                let (_, acc_residual_vec) = self.pagerank_step(&accelerated, alpha, teleport);
                let acc_max_diff = acc_residual_vec.iter().cloned().fold(0.0, f64::max);

                if acc_max_diff < max_diff {
                    rank = accelerated;
                } else {
                    rank = new_rank;
                }
            } else {
                rank = new_rank;
            }

            // 更新历史
            history.push(rank.clone());
            residuals.push(residual);
            if history.len() > m {
                history.remove(0);
                residuals.remove(0);
            }
        }

        (rank, final_iter, converged)
    }

    /// 标准幂法 PageRank（优化收敛检测）
    fn pagerank_standard(
        &self,
        alpha: f64,
        tol: f64,
        max_iter: usize,
    ) -> (Vec<f64>, usize, bool) {
        let n = self.n;
        let nf = n as f64;
        let mut rank = vec![1.0 / nf; n];
        let teleport = 1.0 / nf;
        let mut propagated = vec![0.0f64; n];
        let mut tmp_send = vec![0.0f64; n];

        let mut converged = false;
        let mut final_iter = 0;

        for iter in 0..max_iter {
            final_iter = iter + 1;

            let mut dangling_mass = 0.0;
            for i in 0..n {
                let ow = self.out_weight[i];
                if ow > 1e-15 {
                    tmp_send[i] = rank[i] / ow;
                } else {
                    dangling_mass += rank[i];
                    tmp_send[i] = 0.0;
                }
            }

            for x in propagated.iter_mut() {
                *x = 0.0;
            }
            for (i, &ts) in tmp_send.iter().enumerate().take(n) {
                let rng = self.offsets[i]..self.offsets[i + 1];
                for k in rng {
                    let j = self.targets[k];
                    let w = self.weights[k];
                    propagated[j] += ts * w;
                }
            }

            let mut max_diff = 0.0;
            let dterm = alpha * dangling_mass * teleport;
            let tterm = (1.0 - alpha) * teleport;
            for j in 0..n {
                let new = tterm + alpha * propagated[j] + dterm;
                let d = (new - rank[j]).abs();
                if d > max_diff {
                    max_diff = d;
                }
                rank[j] = new;
            }

            if max_diff < tol {
                converged = true;
                break;
            }
        }

        (rank, final_iter, converged)
    }

    /// 执行单步 PageRank 迭代，返回新 rank 和残差向量
    fn pagerank_step(&self, rank: &[f64], alpha: f64, teleport: f64) -> (Vec<f64>, Vec<f64>) {
        let n = self.n;
        let mut new_rank = vec![0.0f64; n];
        let mut tmp_send = vec![0.0f64; n];
        let mut dangling_mass = 0.0;

        for i in 0..n {
            let ow = self.out_weight[i];
            if ow > 1e-15 {
                tmp_send[i] = rank[i] / ow;
            } else {
                dangling_mass += rank[i];
                tmp_send[i] = 0.0;
            }
        }

        for (i, &ts) in tmp_send.iter().enumerate().take(n) {
            let rng = self.offsets[i]..self.offsets[i + 1];
            for k in rng {
                let j = self.targets[k];
                let w = self.weights[k];
                new_rank[j] += ts * w;
            }
        }

        let dterm = alpha * dangling_mass * teleport;
        let tterm = (1.0 - alpha) * teleport;
        let mut residual = vec![0.0f64; n];
        for j in 0..n {
            let new_val = tterm + alpha * new_rank[j] + dterm;
            residual[j] = new_val - rank[j];
            new_rank[j] = new_val;
        }

        (new_rank, residual)
    }

    /// Anderson 外推加速（简化版，m = 历史窗口大小）
    ///
    /// 求解 min_θ || Σ θ_i · F_i ||，s.t. Σ θ_i = 1
    /// 然后 x_new = Σ θ_i · x_i
    fn anderson_extrapolate(
        &self,
        x_current: &[f64],
        f_current: &[f64],
        history_x: &[Vec<f64>],
        history_f: &[Vec<f64>],
        m: usize,
    ) -> Vec<f64> {
        let n = self.n;
        let k = history_x.len().min(m);

        if k == 0 {
            return x_current.to_vec();
        }

        // 使用最近的 k 个残差构造差向量
        // ΔF_i = F_k - F_i
        // 求解 (ΔF^T ΔF) θ = ΔF^T F_k 的最小范数解
        // 这里用简化方法：直接用残差差的 Gram 矩阵

        let mut df: Vec<Vec<f64>> = Vec::with_capacity(k);
        for i in 0..k {
            let mut diff = vec![0.0f64; n];
            for j in 0..n {
                diff[j] = f_current[j] - history_f[i][j];
            }
            df.push(diff);
        }

        // 构造 Gram 矩阵 G（k x k）
        let mut g = vec![vec![0.0f64; k]; k];
        for i in 0..k {
            for j in 0..k {
                let mut dot = 0.0;
                for t in 0..n {
                    dot += df[i][t] * df[j][t];
                }
                g[i][j] = dot;
            }
        }

        // 正则化
        let reg = 1e-8;
        for i in 0..k {
            g[i][i] += reg;
        }

        // 求解 G · γ = df^T · f_current
        let mut rhs = vec![0.0f64; k];
        for i in 0..k {
            let mut dot = 0.0;
            for t in 0..n {
                dot += df[i][t] * f_current[t];
            }
            rhs[i] = dot;
        }

        // 用高斯消去求解
        let gamma = solve_linear_system(&g, &rhs);

        // 计算组合系数
        // x_acc = x_k - Σ γ_i · (x_k - x_i)
        //       = (1 - Σ γ_i) · x_k + Σ γ_i · x_i
        let mut result = x_current.to_vec();
        let mut sum_gamma = 0.0;
        for i in 0..k {
            sum_gamma += gamma[i];
        }

        if sum_gamma.abs() > 1e-12 {
            // 安全检查：系数不要太大
            let max_gamma = gamma.iter().map(|g| g.abs()).fold(0.0, f64::max);
            if max_gamma < 10.0 {
                for i in 0..k {
                    for t in 0..n {
                        result[t] -= gamma[i] * (x_current[t] - history_x[i][t]);
                    }
                }
            }
        }

        result
    }

    /// 检测蜘蛛陷阱节点（入度高、出度低且形成强连通分量的节点组）
    ///
    /// 返回疑似蜘蛛陷阱的节点索引列表。
    pub(crate) fn detect_spider_traps(&self) -> Vec<usize> {
        let n = self.n;
        if n == 0 {
            return Vec::new();
        }

        // 计算入度
        let mut in_deg = vec![0usize; n];
        for i in 0..n {
            let rng = self.offsets[i]..self.offsets[i + 1];
            for k in rng {
                let j = self.targets[k];
                in_deg[j] += 1;
            }
        }

        // 计算强连通分量（Kosaraju 算法简化版）
        let sccs = self.strongly_connected_components();

        // 找出出边很少且入边很多的 SCC（可能是蜘蛛陷阱）
        let mut traps = Vec::new();
        for scc in &sccs {
            if scc.len() < 2 {
                continue; // 太小的 SCC 不算（自环单独处理）
            }

            let scc_set: HashSet<usize> = scc.iter().copied().collect();
            let mut external_out = 0;
            let mut internal_edges = 0;

            for &node in scc {
                let rng = self.offsets[node]..self.offsets[node + 1];
                for k in rng {
                    let t = self.targets[k];
                    if scc_set.contains(&t) {
                        internal_edges += 1;
                    } else {
                        external_out += 1;
                    }
                }
            }

            // 如果外部出边很少，内部边很多，可能是蜘蛛陷阱
            let total_out = external_out + internal_edges;
            if total_out > 0 && external_out as f64 / (total_out as f64) < 0.1 {
                traps.extend(scc.iter().copied());
            }
        }

        // 检测自环节点（单节点蜘蛛陷阱）
        for i in 0..n {
            let rng = self.offsets[i]..self.offsets[i + 1];
            for k in rng {
                if self.targets[k] == i {
                    // 有自环，且出度低
                    let out_deg = self.offsets[i + 1] - self.offsets[i];
                    if out_deg == 1 && in_deg[i] > 1 {
                        traps.push(i);
                    }
                    break;
                }
            }
        }

        traps
    }

    /// 计算强连通分量（Kosaraju 算法）
    fn strongly_connected_components(&self) -> Vec<Vec<usize>> {
        let n = self.n;

        // 构建反向图
        let mut rev_offsets = vec![0usize; n + 1];
        let mut rev_sources = Vec::with_capacity(self.targets.len());
        let mut rev_weights = Vec::with_capacity(self.targets.len());
        let mut in_deg = vec![0usize; n];

        for i in 0..n {
            let rng = self.offsets[i]..self.offsets[i + 1];
            for k in rng {
                let j = self.targets[k];
                in_deg[j] += 1;
            }
        }

        for i in 0..n {
            rev_offsets[i + 1] = rev_offsets[i] + in_deg[i];
        }

        let mut curs = rev_offsets[0..n].to_vec();
        for i in 0..n {
            let rng = self.offsets[i]..self.offsets[i + 1];
            for k in rng {
                let j = self.targets[k];
                let slot = curs[j];
                curs[j] += 1;
                rev_sources.push(i);
                rev_weights.push(self.weights[k]);
            }
        }

        let _ = rev_weights;

        // 第一步：正向 DFS，按完成时间排序
        let mut visited = vec![false; n];
        let mut order = Vec::with_capacity(n);

        for s in 0..n {
            if !visited[s] {
                let mut stack = vec![(s, false)];
                while let Some((u, processed)) = stack.pop() {
                    if processed {
                        order.push(u);
                        continue;
                    }
                    if visited[u] {
                        continue;
                    }
                    visited[u] = true;
                    stack.push((u, true));

                    let rng = self.offsets[u]..self.offsets[u + 1];
                    // 反向迭代以保持顺序一致
                    let mut neighbors: Vec<usize> = Vec::new();
                    for k in rng {
                        neighbors.push(self.targets[k]);
                    }
                    neighbors.sort();
                    for &v in neighbors.iter().rev() {
                        if !visited[v] {
                            stack.push((v, false));
                        }
                    }
                }
            }
        }

        // 第二步：反向 DFS
        let mut visited2 = vec![false; n];
        let mut sccs = Vec::new();

        for &s in order.iter().rev() {
            if !visited2[s] {
                let mut component = Vec::new();
                let mut stack = vec![s];
                visited2[s] = true;

                while let Some(u) = stack.pop() {
                    component.push(u);
                    let rng = rev_offsets[u]..rev_offsets[u + 1];
                    for k in rng {
                        let v = rev_sources[k];
                        if !visited2[v] {
                            visited2[v] = true;
                            stack.push(v);
                        }
                    }
                }

                component.sort();
                sccs.push(component);
            }
        }

        sccs
    }

    /// 检测悬挂节点（死端）
    pub(crate) fn dangling_nodes(&self) -> Vec<usize> {
        (0..self.n)
            .filter(|&i| self.out_weight[i] <= 1e-15)
            .collect()
    }

    /// 增强型个性化 PageRank
    ///
    /// 支持 Anderson 加速和自适应收敛检测。
    pub(crate) fn pagerank_personalized_enhanced(
        &self,
        alpha: f64,
        p: &[f64],
        tol: f64,
        max_iter: usize,
        use_acceleration: bool,
    ) -> (Vec<f64>, usize, bool) {
        let n = self.n;
        if n == 0 {
            return (Vec::new(), 0, true);
        }

        if !use_acceleration {
            return self.ppr_standard(p, alpha, tol, max_iter);
        }

        // Anderson 加速版 PPR
        let m = 5;
        let mut rank = p.to_vec();
        let mut history: Vec<Vec<f64>> = Vec::with_capacity(m + 1);
        let mut residuals: Vec<Vec<f64>> = Vec::with_capacity(m + 1);

        let mut converged = false;
        let mut final_iter = 0;

        for iter in 0..max_iter {
            final_iter = iter + 1;

            let (new_rank, residual) = self.ppr_step(&rank, p, alpha);

            let max_diff = residual.iter().cloned().fold(0.0, f64::max);
            if max_diff < tol {
                rank = new_rank;
                converged = true;
                break;
            }

            if iter >= m {
                let accelerated = self.anderson_extrapolate(
                    &new_rank,
                    &residual,
                    &history,
                    &residuals,
                    m,
                );

                let (_, acc_res) = self.ppr_step(&accelerated, p, alpha);
                let acc_max = acc_res.iter().cloned().fold(0.0, f64::max);

                if acc_max < max_diff {
                    rank = accelerated;
                } else {
                    rank = new_rank;
                }
            } else {
                rank = new_rank;
            }

            history.push(rank.clone());
            residuals.push(residual);
            if history.len() > m {
                history.remove(0);
                residuals.remove(0);
            }
        }

        (rank, final_iter, converged)
    }

    /// 标准个性化 PageRank
    fn ppr_standard(&self, p: &[f64], alpha: f64, tol: f64, max_iter: usize) -> (Vec<f64>, usize, bool) {
        let n = self.n;
        let mut rank = p.to_vec();
        let mut propagated = vec![0.0f64; n];
        let mut tmp_send = vec![0.0f64; n];

        let mut converged = false;
        let mut final_iter = 0;

        for iter in 0..max_iter {
            final_iter = iter + 1;

            let mut dangling_mass = 0.0;
            for i in 0..n {
                let ow = self.out_weight[i];
                if ow > 1e-15 {
                    tmp_send[i] = rank[i] / ow;
                } else {
                    dangling_mass += rank[i];
                    tmp_send[i] = 0.0;
                }
            }

            for x in propagated.iter_mut() {
                *x = 0.0;
            }
            for (i, &ts) in tmp_send.iter().enumerate().take(n) {
                let rng = self.offsets[i]..self.offsets[i + 1];
                for k in rng {
                    let j = self.targets[k];
                    let w = self.weights[k];
                    propagated[j] += ts * w;
                }
            }

            let mut max_diff = 0.0;
            for j in 0..n {
                let pj = p[j];
                let new = alpha * propagated[j] + alpha * dangling_mass * pj + (1.0 - alpha) * pj;
                let d = (new - rank[j]).abs();
                if d > max_diff {
                    max_diff = d;
                }
                rank[j] = new;
            }

            if max_diff < tol {
                converged = true;
                break;
            }
        }

        (rank, final_iter, converged)
    }

    /// 单步个性化 PageRank 迭代
    fn ppr_step(&self, rank: &[f64], p: &[f64], alpha: f64) -> (Vec<f64>, Vec<f64>) {
        let n = self.n;
        let mut new_rank = vec![0.0f64; n];
        let mut tmp_send = vec![0.0f64; n];
        let mut dangling_mass = 0.0;

        for i in 0..n {
            let ow = self.out_weight[i];
            if ow > 1e-15 {
                tmp_send[i] = rank[i] / ow;
            } else {
                dangling_mass += rank[i];
                tmp_send[i] = 0.0;
            }
        }

        for (i, &ts) in tmp_send.iter().enumerate().take(n) {
            let rng = self.offsets[i]..self.offsets[i + 1];
            for k in rng {
                let j = self.targets[k];
                let w = self.weights[k];
                new_rank[j] += ts * w;
            }
        }

        let mut residual = vec![0.0f64; n];
        for j in 0..n {
            let pj = p[j];
            let new_val = alpha * new_rank[j] + alpha * dangling_mass * pj + (1.0 - alpha) * pj;
            residual[j] = new_val - rank[j];
            new_rank[j] = new_val;
        }

        (new_rank, residual)
    }
}

use std::collections::HashSet;

/// 求解线性方程组 Ax = b（高斯消去法，小规模）
fn solve_linear_system(a: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = a.len();
    if n == 0 {
        return Vec::new();
    }

    // 构造增广矩阵
    let mut aug = vec![vec![0.0f64; n + 1]; n];
    for i in 0..n {
        for j in 0..n {
            aug[i][j] = a[i][j];
        }
        aug[i][n] = b[i];
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

        let pivot = aug[i][i];
        for j in i..=n {
            aug[i][j] /= pivot;
        }

        for k in 0..n {
            if k != i {
                let factor = aug[k][i];
                if factor.abs() > 1e-15 {
                    for j in i..=n {
                        aug[k][j] -= factor * aug[i][j];
                    }
                }
            }
        }
    }

    // 提取解
    (0..n).map(|i| aug[i][n]).collect()
}

pub(crate) fn rank_vec_to_map(rank: &[f64], node_map: &HashMap<String, NodeIndex>) -> HashMap<String, f64> {
    let mut result = HashMap::with_capacity(rank.len());
    for (id, idx) in node_map {
        result.insert(id.clone(), rank[idx.index()]);
    }
    result
}
