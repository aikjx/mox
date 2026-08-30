// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::csr::CsrAdj;
use crate::graph::KnowledgeGraph;
use crate::Result;
use nalgebra::DMatrix;

impl KnowledgeGraph {
    /// 构建邻接矩阵
    pub fn adjacency_matrix(&self) -> DMatrix<f64> {
        let n = self.node_count();
        let mut adj = DMatrix::zeros(n, n);
        for edge in self.graph.edge_references() {
            let i = edge.source().index();
            let j = edge.target().index();
            adj[(i, j)] = *edge.weight();
        }
        adj
    }

    /// 构建度矩阵（CSR O(E)：行和 = Σ W(i,·)；仅最终对角矩阵分配一次 O(N²)）
    pub fn degree_matrix(&self) -> DMatrix<f64> {
        let n = self.node_count();
        if n == 0 {
            return DMatrix::zeros(0, 0);
        }
        let csr = CsrAdj::from_graph(&self.graph);
        let mut deg = DMatrix::zeros(n, n);
        for i in 0..n {
            deg[(i, i)] = csr.out_weight[i];
        }
        deg
    }

    /// 构建归一化拉普拉斯矩阵
    pub fn laplacian_matrix(&self) -> DMatrix<f64> {
        let deg = self.degree_matrix();
        let adj = self.adjacency_matrix();
        &deg - &adj
    }

    /// 构建对称归一化拉普拉斯矩阵（CSR O(E)，仅最终 N² 结果分配一次）
    pub fn normalized_laplacian(&self) -> DMatrix<f64> {
        let n = self.node_count();
        if n == 0 {
            return DMatrix::zeros(0, 0);
        }
        let csr = CsrAdj::from_graph(&self.graph);

        // 无向语义度 d[i]：行和 + 列和（对称归一化拉普拉斯一般基于对称邻接。
        // 历史实现取 row_sum(i) = Σ_j W(i,j) 作为 d[i]，故保持兼容）
        let mut d = vec![0.0f64; n];
        d[..n].copy_from_slice(&csr.out_weight[..n]);

        // L = I − D^-1/2 A D^-1/2
        #[allow(non_snake_case)]
        let mut L = DMatrix::identity(n, n);
        for i in 0..n {
            let di = d[i];
            let di_sqrt = if di > 1e-15 { 1.0 / di.sqrt() } else { 0.0 };
            let rng = csr.offsets[i]..csr.offsets[i + 1];
            for k in rng {
                let j = csr.targets[k];
                let w = csr.weights[k];
                let dj = d[j];
                let dj_sqrt = if dj > 1e-15 { 1.0 / dj.sqrt() } else { 0.0 };
                L[(i, j)] -= di_sqrt * w * dj_sqrt;
            }
        }
        L
    }

    /// 计算k步关联度
    pub fn k_step_relevance(&self, source: &str, target: &str, k: usize) -> Result<f64> {
        let source_idx = self
            .node_map
            .get(source)
            .ok_or_else(|| anyhow::anyhow!("源节点不存在: {}", source))?;
        let target_idx = self
            .node_map
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("目标节点不存在: {}", target))?;

        let adj = self.adjacency_matrix();
        let a_k = adj.pow(k as u32);
        let frobenius_norm = a_k.norm();

        if frobenius_norm < 1e-15 {
            return Ok(0.0);
        }

        Ok(a_k[(source_idx.index(), target_idx.index())] / frobenius_norm)
    }

    /// 计算全步关联度（带衰减）
    pub fn total_relevance(&self, source: &str, target: &str) -> Result<f64> {
        let source_idx = self
            .node_map
            .get(source)
            .ok_or_else(|| anyhow::anyhow!("源节点不存在: {}", source))?;
        let target_idx = self
            .node_map
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("目标节点不存在: {}", target))?;

        let n = self.node_count();
        let adj = self.adjacency_matrix();
        let alpha = self.damping_factor;

        let identity = DMatrix::identity(n, n);
        let matrix = &identity - &(&adj * alpha);
        let inv = matrix
            .try_inverse()
            .ok_or_else(|| anyhow::anyhow!("矩阵不可逆"))?;

        Ok(inv[(source_idx.index(), target_idx.index())])
    }
}
