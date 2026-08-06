//! # 守恒律检查与残差监控
//!
//! 实现守恒律验证和残差计算，保证系统数学自洽性

use crate::state::StateVector;
use crate::OperatorError;

/// 守恒律trait
pub trait ConservationLaw {
    /// 守恒律名称
    fn name(&self) -> &str;

    /// 检查状态是否满足守恒律，返回残差
    fn check(&self, state: &StateVector) -> f64;

    /// 检查是否在阈值内
    fn is_satisfied(&self, state: &StateVector, threshold: f64) -> bool {
        self.check(state).abs() < threshold
    }

    /// 验证并在违反时返回错误
    fn verify(&self, state: &StateVector, threshold: f64) -> Result<(), OperatorError> {
        let residual = self.check(state);
        if residual.abs() >= threshold {
            Err(OperatorError::ConservationViolation {
                law: self.name().to_string(),
                residual,
                threshold,
            })
        } else {
            Ok(())
        }
    }
}

/// L1范数守恒（概率守恒）
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

    fn check(&self, state: &StateVector) -> f64 {
        (state.norm_l1() - self.expected_sum).abs()
    }
}

/// L2范数守恒（能量守恒）
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

    fn check(&self, state: &StateVector) -> f64 {
        (state.norm() - self.expected_norm).abs()
    }
}

/// 总和守恒
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

    fn check(&self, state: &StateVector) -> f64 {
        let sum: f64 = state.data.iter().sum();
        (sum - self.expected_sum).abs()
    }
}

/// 守恒律检查器
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

    pub fn check_all(&self, state: &StateVector) -> Result<(), OperatorError> {
        for law in &self.laws {
            law.verify(state, self.threshold)?;
        }
        Ok(())
    }

    pub fn check_all_residuals(&self, state: &StateVector) -> Vec<(&str, f64)> {
        self.laws
            .iter()
            .map(|law| (law.name(), law.check(state)))
            .collect()
    }
}

/// 残差监控器
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
        let max_residual = recent.iter().fold(0.0f64, |a, &b| a.max(b));
        max_residual < self.threshold
    }

    pub fn max_residual(&self) -> f64 {
        self.history.iter().fold(0.0f64, |a, &b| a.max(b))
    }

    pub fn mean_residual(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        self.history.iter().sum::<f64>() / self.history.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_l1_conservation() {
        let law = L1Conservation::probability();
        let state = StateVector::from_vec(vec![0.25, 0.25, 0.25, 0.25]);
        assert!(law.is_satisfied(&state, 1e-10));

        let modified = StateVector::from_vec(vec![0.5, 0.25, 0.25, 0.25]);
        assert!(!law.is_satisfied(&modified, 1e-10));
        assert_relative_eq!(law.check(&modified), 0.25);
    }

    #[test]
    fn test_l2_conservation() {
        let law = L2Conservation::unit_energy();
        let state = StateVector::from_vec(vec![1.0, 0.0, 0.0]);
        assert!(law.is_satisfied(&state, 1e-10));

        let modified = StateVector::from_vec(vec![2.0, 0.0, 0.0]);
        assert!(!law.is_satisfied(&modified, 1e-10));
    }

    #[test]
    fn test_conservation_checker() {
        // 使用L2归一化检查能量守恒
        let mut checker = ConservationChecker::new(1e-10);
        checker.add_law(L2Conservation::unit_energy());
        let mut state = StateVector::from_vec(vec![0.5, 0.5]);
        state.normalize();
        assert!(checker.check_all(&state).is_ok());
    }

    #[test]
    fn test_residual_monitor() {
        let mut monitor = ResidualMonitor::new(1e-6);
        monitor.record(0.1);
        monitor.record(0.01);
        monitor.record(0.001);
        monitor.record(0.0000001);
        assert!(monitor.is_converged(1));
        assert_relative_eq!(monitor.max_residual(), 0.1);
    }
}
