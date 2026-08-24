//! 算子流水线执行引擎 —— 端到端集成测试
//!
//! 覆盖：串联执行 + 资源累计、概率守恒校验、严格闸门中断、非严格残差记录。

use std::sync::Arc;

use operator_core::engine::OperatorPipeline;
use operator_core::operator::{FunctionOperator, IdentityOperator, LinearOperator};
use operator_core::state::StateVector;
use operator_core::SystemConfig;

#[test]
fn pipeline_chains_operators_and_tracks_resources() {
    let pipe = OperatorPipeline::new()
        .then(Arc::new(IdentityOperator::new(4)))
        .then(Arc::new(LinearOperator::identity(4)))
        .then(Arc::new(FunctionOperator::new("double", |s, _ctx| {
            Ok(s.scale(2.0))
        })));

    let cfg = SystemConfig::default();
    let input = StateVector::from_vec(vec![1.0, 2.0, 3.0, 4.0]);
    let result = pipe.run(&input, &cfg).unwrap();

    assert!(result.success);
    assert_eq!(pipe.len(), 3);
    assert_eq!(result.stages.len(), 3);

    let final_state = result.final_state.expect("成功流水线必有终态");
    assert!((final_state[0] - 2.0).abs() < 1e-9);
    assert!((final_state[1] - 4.0).abs() < 1e-9);
    assert!((final_state[2] - 6.0).abs() < 1e-9);
    assert!((final_state[3] - 8.0).abs() < 1e-9);
}

#[test]
fn pipeline_probability_conservation_ok_on_unit_vector() {
    let pipe = OperatorPipeline::new()
        .then(Arc::new(IdentityOperator::new(3)))
        .with_probability_conservation();

    let cfg = SystemConfig::default();
    // 单位向量满足默认概率(L1=1)/能量(L2=1)守恒律
    let input = StateVector::from_vec(vec![1.0, 0.0, 0.0]);
    let result = pipe.run(&input, &cfg).unwrap();

    assert!(result.success);
    assert!(
        result.total_residual < 1e-9,
        "归一化输入下守恒残差应≈0，实际={}",
        result.total_residual
    );
    assert!(result.converged);
}

#[test]
fn pipeline_strict_gating_blocks_violation() {
    let pipe = OperatorPipeline::new()
        .then(Arc::new(IdentityOperator::new(3)))
        .with_probability_conservation()
        .strict(true);

    let cfg = SystemConfig::default();
    // [2,0,0] L2 范数=2 → 违反单位能量守恒律 → 严格闸门应中断
    let input = StateVector::from_vec(vec![2.0, 0.0, 0.0]);
    let result = pipe.run(&input, &cfg).unwrap();

    assert!(!result.success);
    let err = result.error.expect("严格闸门失败应有错误");
    assert!(
        err.contains("守恒残差"),
        "错误应为守恒残差中断，实际: {err}"
    );
}

#[test]
fn pipeline_non_strict_records_residual_without_failing() {
    let pipe = OperatorPipeline::new()
        .then(Arc::new(IdentityOperator::new(3)))
        .with_probability_conservation();

    let cfg = SystemConfig::default();
    let input = StateVector::from_vec(vec![2.0, 0.0, 0.0]);
    let result = pipe.run(&input, &cfg).unwrap();

    // 非严格模式：成功，但残差被真实记录
    assert!(result.success);
    assert!(result.total_residual > 1e-9, "非严格模式应记录非零守恒残差");
}

#[test]
fn pipeline_propagates_operator_failure() {
    let failing = Arc::new(FunctionOperator::new("boom", |_s, _ctx| {
        Err(operator_core::OperatorError::ExecutionError(
            "intentional failure".to_string(),
        ))
    }));

    let pipe = OperatorPipeline::new()
        .then(Arc::new(IdentityOperator::new(2)))
        .then(failing);

    let cfg = SystemConfig::default();
    let input = StateVector::from_vec(vec![1.0, 1.0]);
    let result = pipe.run(&input, &cfg).unwrap();

    assert!(!result.success);
    assert_eq!(result.stages.len(), 2); // 失败阶段也被记录
    assert!(result.stages[1].error.is_some());
}
