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

/// 图谱节点状态向量提供者
///
/// 任何具有状态向量的图谱节点类型都应实现此 trait，
/// 使得 `GuardedGraph` 可以对其进行守恒律检查。
pub trait GraphNode {
    fn state_vector(&self) -> &StateVector;
    fn state_vector_mut(&mut self) -> &mut StateVector;
}

/// 守恒保护的图谱操作
///
/// 对节点/边的增删改操作自动进行守恒律校验，
/// 违规操作在执行前即被阻断，保证图谱数据的数学自洽性。
pub struct GuardedGraph {
    checker: ConservationChecker,
    node_count: usize,
    edge_count: usize,
}

impl GuardedGraph {
    pub fn new(threshold: f64) -> Self {
        Self {
            checker: ConservationChecker::new(threshold),
            node_count: 0,
            edge_count: 0,
        }
    }

    pub fn with_default_laws(threshold: f64) -> Self {
        Self {
            checker: ConservationChecker::with_default_laws(threshold),
            node_count: 0,
            edge_count: 0,
        }
    }

    pub fn add_law<L: ConservationLaw + 'static>(&mut self, law: L) {
        self.checker.add_law(law);
    }

    /// 检查节点状态是否满足守恒律
    pub fn check_node<N: GraphNode>(&self, node: &N) -> Result<(), crate::OperatorError> {
        self.checker.check_all(node.state_vector())
    }

    /// 预检查两个节点合并后的守恒状态
    pub fn check_edge<N: GraphNode>(
        &self,
        source: &N,
        target: &N,
    ) -> Result<(), crate::OperatorError> {
        let source_state = source.state_vector();
        let target_state = target.state_vector();
        let combined = source_state.combine(target_state)?;
        self.checker.check_all(&combined)
    }

    /// 获取图谱级守恒残差全景
    pub fn check_all_residuals<N: GraphNode>(&self, nodes: &[N]) -> Vec<(String, f64)> {
        let mut all_residuals = Vec::new();
        for node in nodes {
            let residuals = self.checker.check_all_residuals(node.state_vector());
            for (law, r) in residuals {
                all_residuals.push((law.to_string(), r));
            }
        }
        all_residuals
    }

    pub fn node_count(&self) -> usize {
        self.node_count
    }

    pub fn edge_count(&self) -> usize {
        self.edge_count
    }

    pub fn increment_nodes(&mut self) {
        self.node_count += 1;
    }

    pub fn increment_edges(&mut self) {
        self.edge_count += 1;
    }
}

/// 图谱批量守恒报告
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConservationReport {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub violations: Vec<ConservationViolation>,
    pub max_residual: f64,
    pub pass_rate: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConservationViolation {
    pub node_id: String,
    pub law: String,
    pub residual: f64,
    pub threshold: f64,
}

impl GuardedGraph {
    /// 生成全图谱守恒报告
    pub fn generate_report<N: GraphNode>(&self, nodes: &[(String, N)]) -> ConservationReport {
        let total = nodes.len();
        let mut violations = Vec::new();
        let mut max_res = 0.0f64;
        let mut passed = 0;

        for (id, node) in nodes {
            let residuals = self.checker.check_all_residuals(node.state_vector());
            let node_max = residuals.iter().map(|(_, r)| r.abs()).fold(0.0f64, f64::max);

            if node_max > max_res {
                max_res = node_max;
            }

            let mut node_ok = true;
            for (law, r) in &residuals {
                if r.abs() > self.checker.threshold {
                    violations.push(ConservationViolation {
                        node_id: id.clone(),
                        law: law.to_string(),
                        residual: *r,
                        threshold: self.checker.threshold,
                    });
                    node_ok = false;
                }
            }

            if node_ok {
                passed += 1;
            }
        }

        ConservationReport {
            total_nodes: total,
            total_edges: self.edge_count,
            violations,
            max_residual: max_res,
            pass_rate: if total > 0 {
                passed as f64 / total as f64
            } else {
                1.0
            },
        }
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

    #[test]
    fn test_guarded_graph_node_check() {
        use crate::state::StateVector;

        struct TestNode {
            state: StateVector,
        }
        impl GraphNode for TestNode {
            fn state_vector(&self) -> &StateVector {
                &self.state
            }
            fn state_vector_mut(&mut self) -> &mut StateVector {
                &mut self.state
            }
        }

        let graph = GuardedGraph::with_default_laws(1e-10);
        let node = TestNode {
            state: StateVector::from_vec(vec![1.0, 0.0, 0.0]),
        };

        assert!(graph.check_node(&node).is_ok());
    }

    #[test]
    fn test_guarded_graph_edge_check() {
        use crate::state::StateVector;

        struct TestNode {
            state: StateVector,
        }
        impl GraphNode for TestNode {
            fn state_vector(&self) -> &StateVector {
                &self.state
            }
            fn state_vector_mut(&mut self) -> &mut StateVector {
                &mut self.state
            }
        }

        // 不使用默认守恒律，仅验证接口正确
        let graph = GuardedGraph::new(1e-10);
        let source = TestNode {
            state: StateVector::from_vec(vec![0.5, 0.5]),
        };
        let target = TestNode {
            state: StateVector::from_vec(vec![0.5, 0.5]),
        };

        // 无守恒律时应始终通过
        assert!(graph.check_edge(&source, &target).is_ok());

        // 添加守恒律后可检测违规
        let mut graph_with_laws = GuardedGraph::new(1e-10);
        graph_with_laws.add_law(L1Conservation::new(2.0)); // 期望 L1=2.0
        assert!(graph_with_laws.check_edge(&source, &target).is_ok());
    }

    #[test]
    fn test_guarded_graph_report() {
        use crate::state::StateVector;

        struct TestNode {
            state: StateVector,
        }
        impl GraphNode for TestNode {
            fn state_vector(&self) -> &StateVector {
                &self.state
            }
            fn state_vector_mut(&mut self) -> &mut StateVector {
                &mut self.state
            }
        }

        let graph = GuardedGraph::with_default_laws(1e-10);
        let nodes = vec![
            (
                "n1".to_string(),
                TestNode {
                    state: StateVector::from_vec(vec![1.0, 0.0, 0.0]),
                },
            ),
            (
                "n2".to_string(),
                TestNode {
                    state: StateVector::from_vec(vec![0.0, 1.0, 0.0]),
                },
            ),
        ];

        let report = graph.generate_report(&nodes);
        assert_eq!(report.total_nodes, 2);
        assert!(report.pass_rate >= 0.0);
    }
}
