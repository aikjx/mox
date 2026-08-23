//! # 守恒律检查与残差监控
//!
//! 实现守恒律验证和残差计算，保证系统数学自洽性。
//!
//! 纯数学内核（ConservationLaw、L1/L2/Sum 守恒律、ConservationChecker、ResidualMonitor）
//! 已移至 `kernel.rs` 并基于 `VectorOps` trait 抽象（DIP：依赖倒置），
//! 本模块：
//! 1. 重导出 kernel 的纯类型；
//! 2. 提供 `verify` / 返回 `OperatorError` 的扩展方法；
//! 3. 保留原 `GraphNode`、`GuardedGraph`、`ConservationReport` 等依赖 `StateVector` 的高层 API。

use crate::kernel::VectorOps;
use crate::state::StateVector;
use crate::OperatorError;

// ===== 重导出 L6 纯内核守恒律系统 =====
pub use crate::kernel::{
    ConservationChecker, ConservationLaw, L1Conservation, L2Conservation, ResidualMonitor,
    SumConservation,
};

// ===== 扩展：为 kernel::ConservationLaw 补充返回 OperatorError 的 verify（非纯） =====

/// 为任何 `ConservationLaw` 实现者增加高级方法（需依赖 OperatorError，因此放入上层）。
pub trait ConservationLawExt: ConservationLaw {
    fn verify(&self, state: &StateVector, threshold: f64) -> Result<(), OperatorError> {
        let residual = self.check(state as &dyn VectorOps);
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
impl<L: ConservationLaw + ?Sized> ConservationLawExt for L {}

// ===== 扩展：让 ConservationChecker 返回 OperatorError =====

/// 高层包装：把 `check_all` 的错误从 `KernelError` 转换为 `OperatorError`。
pub trait ConservationCheckerExt {
    fn check_all_ops(&self, state: &StateVector) -> Result<(), OperatorError>;
    fn check_all_residuals_ops(&self, state: &StateVector) -> Vec<(&str, f64)>;
}

impl ConservationCheckerExt for ConservationChecker {
    fn check_all_ops(&self, state: &StateVector) -> Result<(), OperatorError> {
        let residuals = self.check_all_residuals(state as &dyn VectorOps);
        for (law, residual) in &residuals {
            if residual.abs() >= self.threshold() {
                return Err(OperatorError::ConservationViolation {
                    law: (*law).to_string(),
                    residual: *residual,
                    threshold: self.threshold(),
                });
            }
        }
        Ok(())
    }

    fn check_all_residuals_ops(&self, state: &StateVector) -> Vec<(&str, f64)> {
        self.check_all_residuals(state as &dyn VectorOps)
    }
}

// ===== GraphNode trait（绑定具体 StateVector，保留原公共 API） =====

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
    pub fn check_node<N: GraphNode>(&self, node: &N) -> Result<(), OperatorError> {
        self.checker.check_all_ops(node.state_vector())
    }

    /// 预检查两个节点合并后的守恒状态
    pub fn check_edge<N: GraphNode>(
        &self,
        source: &N,
        target: &N,
    ) -> Result<(), OperatorError> {
        let source_state = source.state_vector();
        let target_state = target.state_vector();
        let combined = source_state.combine(target_state)?;
        self.checker.check_all_ops(&combined)
    }

    /// 获取图谱级守恒残差全景
    pub fn check_all_residuals<N: GraphNode>(&self, nodes: &[N]) -> Vec<(String, f64)> {
        let mut all_residuals = Vec::new();
        for node in nodes {
            let residuals = self.checker.check_all_residuals_ops(node.state_vector());
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
            let residuals = self.checker.check_all_residuals_ops(node.state_vector());
            let node_max = residuals
                .iter()
                .map(|(_, r)| r.abs())
                .fold(0.0f64, f64::max);

            if node_max > max_res {
                max_res = node_max;
            }

            let mut node_ok = true;
            for (law, r) in &residuals {
                if r.abs() > self.checker.threshold() {
                    violations.push(ConservationViolation {
                        node_id: id.clone(),
                        law: (*law).to_string(),
                        residual: *r,
                        threshold: self.checker.threshold(),
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

// ===== 原 conservation 单元测试 =====

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use crate::state::StateVector;

    #[test]
    fn test_l1_conservation() {
        let law = L1Conservation::probability();
        let state = StateVector::from_vec(vec![0.25, 0.25, 0.25, 0.25]);
        assert!(law.is_satisfied(&state as &dyn VectorOps, 1e-10));

        let modified = StateVector::from_vec(vec![0.5, 0.25, 0.25, 0.25]);
        assert!(!law.is_satisfied(&modified as &dyn VectorOps, 1e-10));
        assert_relative_eq!(law.check(&modified as &dyn VectorOps), 0.25);
    }

    #[test]
    fn test_l2_conservation() {
        let law = L2Conservation::unit_energy();
        let state = StateVector::from_vec(vec![1.0, 0.0, 0.0]);
        assert!(law.is_satisfied(&state as &dyn VectorOps, 1e-10));

        let modified = StateVector::from_vec(vec![2.0, 0.0, 0.0]);
        assert!(!law.is_satisfied(&modified as &dyn VectorOps, 1e-10));
    }

    #[test]
    fn test_conservation_checker() {
        // 使用L2归一化检查能量守恒
        let mut checker = ConservationChecker::new(1e-10);
        checker.add_law(L2Conservation::unit_energy());
        let mut state = StateVector::from_vec(vec![0.5, 0.5]);
        state.normalize();
        assert!(checker.check_all_ops(&state).is_ok());
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

    // 验证 verify 返回 OperatorError
    #[test]
    fn test_conservation_law_ext_verify_returns_error() {
        let law = L1Conservation::probability();
        let bad = StateVector::from_vec(vec![2.0, 0.0, 0.0]);
        let result = law.verify(&bad, 1e-10);
        assert!(result.is_err());
        match result.unwrap_err() {
            OperatorError::ConservationViolation { law: l, residual, threshold } => {
                assert!(l.contains("L1"));
                assert!(residual > 0.5);
                assert_eq!(threshold, 1e-10);
            }
            other => panic!("预期 ConservationViolation，得到 {:?}", other),
        }
    }
}
