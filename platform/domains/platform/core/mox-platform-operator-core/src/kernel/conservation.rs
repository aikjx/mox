// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 纯守恒律系统：基于 VectorOps 抽象，不绑定具体实现。

use crate::kernel::types::KernelError;
use crate::kernel::vector::VectorOps;

/// 守恒律 trait：通过 `dyn VectorOps` 接收任意向量实现。
pub trait ConservationLaw {
    fn name(&self) -> &str;
    fn check(&self, state: &dyn VectorOps) -> f64;

    fn is_satisfied(&self, state: &dyn VectorOps, threshold: f64) -> bool {
        self.check(state).abs() < threshold
    }
}

pub struct L1Conservation {
    expected_sum: f64,
}

impl L1Conservation {
    pub fn new(expected_sum: f64) -> Self {
        Self { expected_sum }
    }

    pub fn probability() -> Self {
        Self::new(1.0)
    }
}

impl ConservationLaw for L1Conservation {
    fn name(&self) -> &str {
        "L1范数守恒（概率守恒）"
    }

    fn check(&self, state: &dyn VectorOps) -> f64 {
        (state.norm_l1() - self.expected_sum).abs()
    }
}

pub struct L2Conservation {
    expected_norm: f64,
}

impl L2Conservation {
    pub fn new(expected_norm: f64) -> Self {
        Self { expected_norm }
    }

    pub fn unit_energy() -> Self {
        Self::new(1.0)
    }
}

impl ConservationLaw for L2Conservation {
    fn name(&self) -> &str {
        "L2范数守恒（能量守恒）"
    }

    fn check(&self, state: &dyn VectorOps) -> f64 {
        (state.norm_l2() - self.expected_norm).abs()
    }
}

pub struct SumConservation {
    expected_sum: f64,
}

impl SumConservation {
    pub fn new(expected_sum: f64) -> Self {
        Self { expected_sum }
    }
}

impl ConservationLaw for SumConservation {
    fn name(&self) -> &str {
        "元素总和守恒"
    }

    fn check(&self, state: &dyn VectorOps) -> f64 {
        (state.sum() - self.expected_sum).abs()
    }
}

pub struct ConservationChecker {
    laws: Vec<Box<dyn ConservationLaw>>,
    threshold: f64,
}

impl ConservationChecker {
    pub fn new(threshold: f64) -> Self {
        Self {
            laws: Vec::new(),
            threshold,
        }
    }

    pub fn with_default_laws(threshold: f64) -> Self {
        let mut checker = Self::new(threshold);
        checker.add_law(L1Conservation::probability());
        checker.add_law(L2Conservation::unit_energy());
        checker
    }

    pub fn add_law<L: ConservationLaw + 'static>(&mut self, law: L) {
        self.laws.push(Box::new(law));
    }

    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    pub fn check_all(&self, state: &dyn VectorOps) -> Result<(), KernelError> {
        for law in &self.laws {
            let residual = law.check(state);
            if residual.abs() >= self.threshold {
                return Err(KernelError::Other(format!(
                    "守恒律违反: {}, residual={}",
                    law.name(),
                    residual
                )));
            }
        }
        Ok(())
    }

    pub fn check_all_residuals(&self, state: &dyn VectorOps) -> Vec<(&str, f64)> {
        self.laws
            .iter()
            .map(|law| (law.name(), law.check(state)))
            .collect()
    }
}

pub struct ResidualMonitor {
    history: Vec<f64>,
    threshold: f64,
}

impl ResidualMonitor {
    pub fn new(threshold: f64) -> Self {
        Self {
            history: Vec::new(),
            threshold,
        }
    }

    pub fn record(&mut self, residual: f64) {
        self.history.push(residual);
    }

    pub fn is_converged(&self, window: usize) -> bool {
        if self.history.len() < window {
            return false;
        }
        let recent = &self.history[self.history.len() - window..];
        let max = recent.iter().fold(0.0f64, |a, &b| a.max(b.abs()));
        max < self.threshold
    }

    pub fn max_residual(&self) -> f64 {
        self.history.iter().fold(0.0f64, |a, &b| a.max(b.abs()))
    }

    pub fn mean_residual(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        self.history.iter().sum::<f64>() / self.history.len() as f64
    }
}

/// 通用图谱节点：任何携带状态向量的结构均可实现。
pub trait GraphNode {
    fn state_vector_dyn(&self) -> &dyn VectorOps;
}
