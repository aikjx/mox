// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 算子流水线执行引擎
//!
//! 把多个算子串成有向流水线，逐步执行并：
//! - 累计资源使用（`ResourceUsage`）
//! - 逐阶段计算守恒残差（`ConservationChecker`）并监控收敛
//! - 可选「严格守恒闸门」：任一阶段残差超阈即中断流水线并返回错误
//!
//! 这是"算子内核与执行"（D01）的统一运行能力，弥补了
//! `Operator::execute()` 中 `residual` 写死为 0 的桩——引擎在流水线层级
//! 真正落地守恒律校验，而非仅对单个算子做一次性调用。

use std::sync::Arc;
use std::time::Instant;

use crate::conservation::{ConservationChecker, ResidualMonitor};
use crate::operator::Operator;
use crate::resource::ResourceUsage;
use crate::state::StateVector;
use crate::{ExecutionContext, ExecutionResult, SystemConfig};

/// 单阶段执行记录
#[derive(Debug, Clone)]
pub struct StageResult {
    /// 算子名称
    pub operator_name: String,
    /// 阶段输出态（失败为 None）
    pub output: Option<StateVector>,
    /// 该阶段守恒残差（取所有守恒律的最大绝对值）
    pub residual: f64,
    /// 该阶段资源消耗
    pub resources_used: ResourceUsage,
    /// 该阶段耗时（毫秒）
    pub execution_time_ms: u64,
    /// 阶段级错误（若有）
    pub error: Option<String>,
}

/// 流水线整体执行结果
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// 各阶段记录
    pub stages: Vec<StageResult>,
    /// 最终输出态
    pub final_state: Option<StateVector>,
    /// 各阶段最大守恒残差
    pub total_residual: f64,
    /// 累计资源消耗
    pub total_resources: ResourceUsage,
    /// 总耗时（毫秒）
    pub total_time_ms: u64,
    /// 残差是否落入阈值窗口（收敛）
    pub converged: bool,
    /// 是否全部成功
    pub success: bool,
    /// 顶层错误（若有）
    pub error: Option<String>,
}

/// 算子流水线执行引擎
pub struct OperatorPipeline {
    stages: Vec<Arc<dyn Operator>>,
    checker: ConservationChecker,
    /// 守恒违反即刻中断
    strict: bool,
    /// 收敛判定窗口（最近 N 个残差均低于阈值）
    convergence_window: usize,
}

impl Default for OperatorPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl OperatorPipeline {
    /// 新建空流水线（不挂守恒律，residual 恒为 0，仅做资源累计与串联）
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            checker: ConservationChecker::new(1e-10),
            strict: false,
            convergence_window: 1,
        }
    }

    /// 追加算子（消费式，便于链式构造 `new().then(a).then(b)`）
    pub fn then(mut self, op: Arc<dyn Operator>) -> Self {
        self.stages.push(op);
        self
    }

    /// 追加算子（可变式，便于循环构建）
    pub fn add(&mut self, op: Arc<dyn Operator>) -> &mut Self {
        self.stages.push(op);
        self
    }

    /// 挂载守恒律检查器（逐阶段计算残差）
    pub fn with_conservation(mut self, checker: ConservationChecker) -> Self {
        self.checker = checker;
        self
    }

    /// 便捷：挂载概率/能量守恒律（输入须为归一化分布才有意义）
    pub fn with_probability_conservation(self) -> Self {
        self.with_conservation(ConservationChecker::with_default_laws(1e-10))
    }

    /// 开启严格守恒闸门：任一阶段残差超阈即中断并返回错误
    pub fn strict(mut self, on: bool) -> Self {
        self.strict = on;
        self
    }

    /// 设置收敛判定窗口大小
    pub fn with_convergence_window(mut self, window: usize) -> Self {
        self.convergence_window = window.max(1);
        self
    }

    /// 流水线阶段数
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    /// 是否为空流水线
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// 执行整条流水线
    ///
    /// - `input`：初始输入态
    /// - `config`：系统配置（取 `residual_threshold` 与 `enable_conservation_check`）
    ///
    /// 返回 `PipelineResult`；任一阶段算子自身 `success=false` 或（严格模式下）
    /// 守恒残差超阈，都会以 `success=false` 提前结束（已执行阶段仍记录）。
    pub fn run(
        &self,
        input: &StateVector,
        config: &SystemConfig,
    ) -> Result<PipelineResult, crate::OperatorError> {
        let mut ctx = ExecutionContext {
            config: config.clone(),
            trace_id: crate::generate_operator_id(),
            resources: ResourceUsage::zero(),
            metadata: Default::default(),
        };
        let mut monitor = ResidualMonitor::new(config.residual_threshold);

        let mut current = input.clone();
        let mut stage_results: Vec<StageResult> = Vec::with_capacity(self.stages.len());
        let start = Instant::now();
        let mut total_resources = ResourceUsage::zero();
        let mut max_residual = 0.0f64;

        for (i, op) in self.stages.iter().enumerate() {
            let stage_start = Instant::now();
            let res: ExecutionResult = op.execute(&current, &mut ctx)?;
            let stage_time = stage_start.elapsed().as_millis() as u64;

            // 算子自身失败：回填阶段记录并中断
            if !res.success {
                stage_results.push(StageResult {
                    operator_name: op.metadata().name,
                    output: res.output_state.clone(),
                    residual: 0.0,
                    resources_used: res.resources_used,
                    execution_time_ms: stage_time,
                    error: res.error.clone(),
                });
                total_resources = total_resources + res.resources_used;
                return Ok(PipelineResult {
                    stages: stage_results,
                    final_state: res.output_state,
                    total_residual: max_residual,
                    total_resources,
                    total_time_ms: start.elapsed().as_millis() as u64,
                    converged: false,
                    success: false,
                    error: res
                        .error
                        .or_else(|| Some(format!("第 {} 阶段算子执行失败", i))),
                });
            }

            let out_state = res
                .output_state
                .expect("success 为 true 时 execute 必返回 output_state");

            // 真实守恒残差：对输出态套用已挂载的守恒律，取最大绝对值
            let residual = if config.enable_conservation_check {
                self.checker
                    .check_all_residuals(&out_state)
                    .iter()
                    .map(|(_, r)| r.abs())
                    .fold(0.0f64, f64::max)
            } else {
                0.0
            };
            monitor.record(residual);
            if residual > max_residual {
                max_residual = residual;
            }

            // 严格守恒闸门：残差超阈即中断
            if self.strict && residual > config.residual_threshold {
                stage_results.push(StageResult {
                    operator_name: op.metadata().name,
                    output: Some(out_state.clone()),
                    residual,
                    resources_used: res.resources_used,
                    execution_time_ms: stage_time,
                    error: None,
                });
                total_resources = total_resources + res.resources_used;
                return Ok(PipelineResult {
                    stages: stage_results,
                    final_state: Some(out_state),
                    total_residual: max_residual,
                    total_resources,
                    total_time_ms: start.elapsed().as_millis() as u64,
                    converged: false,
                    success: false,
                    error: Some(format!(
                        "第 {} 阶段守恒残差 {:.3e} 超阈值 {:.3e}",
                        i, residual, config.residual_threshold
                    )),
                });
            }

            total_resources = total_resources + res.resources_used;
            stage_results.push(StageResult {
                operator_name: op.metadata().name,
                output: Some(out_state.clone()),
                residual,
                resources_used: res.resources_used,
                execution_time_ms: stage_time,
                error: None,
            });
            current = out_state;
        }

        Ok(PipelineResult {
            stages: stage_results,
            final_state: Some(current.clone()),
            total_residual: max_residual,
            total_resources,
            total_time_ms: start.elapsed().as_millis() as u64,
            converged: monitor.is_converged(self.convergence_window),
            success: true,
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::{FunctionOperator, IdentityOperator, LinearOperator};
    use crate::state::StateVector;
    use std::sync::Arc;

    fn scale_op(factor: f64) -> Arc<dyn Operator> {
        Arc::new(FunctionOperator::new("scale", move |s, _ctx| {
            Ok(s.scale(factor))
        }))
    }

    #[test]
    fn empty_pipeline_passthrough() {
        let pipe = OperatorPipeline::new();
        let cfg = SystemConfig::default();
        let input = StateVector::from_vec(vec![5.0, 7.0]);
        let result = pipe.run(&input, &cfg).unwrap();
        assert!(result.success);
        assert_eq!(result.stages.len(), 0);
        assert_eq!(result.final_state.unwrap()[1], 7.0);
        assert!(!result.converged); // 无残差记录
    }

    #[test]
    fn pipeline_chains_and_scales() {
        let pipe = OperatorPipeline::new()
            .then(Arc::new(IdentityOperator::new(3)))
            .then(Arc::new(LinearOperator::identity(3)))
            .then(scale_op(2.0));
        let cfg = SystemConfig::default();
        let input = StateVector::from_vec(vec![1.0, 2.0, 3.0]);
        let result = pipe.run(&input, &cfg).unwrap();

        assert!(result.success);
        assert_eq!(pipe.len(), 3);
        assert_eq!(result.stages.len(), 3);
        let final_state = result.final_state.unwrap();
        assert!((final_state[0] - 2.0).abs() < 1e-9);
        assert!((final_state[1] - 4.0).abs() < 1e-9);
        assert!((final_state[2] - 6.0).abs() < 1e-9);
    }

    #[test]
    fn pipeline_respects_probability_conservation() {
        // 单位向量：L2 范数=1、L1 和=1，满足默认概率/能量守恒律
        let pipe = OperatorPipeline::new()
            .then(Arc::new(IdentityOperator::new(3)))
            .with_probability_conservation();
        let cfg = SystemConfig::default();
        let input = StateVector::from_vec(vec![1.0, 0.0, 0.0]);
        let result = pipe.run(&input, &cfg).unwrap();

        assert!(result.success);
        assert!(
            result.total_residual < 1e-9,
            "residual={}",
            result.total_residual
        );
        assert!(result.converged);
    }

    #[test]
    fn strict_gating_blocks_conservation_violation() {
        // [2,0,0] 的 L2 范数=2，违反单位能量守恒律 → 严格闸门应中断
        let pipe = OperatorPipeline::new()
            .then(Arc::new(IdentityOperator::new(3)))
            .with_probability_conservation()
            .strict(true);
        let cfg = SystemConfig::default();
        let input = StateVector::from_vec(vec![2.0, 0.0, 0.0]);
        let result = pipe.run(&input, &cfg).unwrap();

        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("守恒残差"));
    }

    #[test]
    fn non_strict_reports_residual_without_failing() {
        // 非严格模式下，残差被记录但不中断
        let pipe = OperatorPipeline::new()
            .then(Arc::new(IdentityOperator::new(3)))
            .with_probability_conservation();
        let cfg = SystemConfig::default();
        let input = StateVector::from_vec(vec![2.0, 0.0, 0.0]);
        let result = pipe.run(&input, &cfg).unwrap();

        assert!(result.success);
        assert!(result.total_residual > 1e-9, "应记录非零残差");
    }
}
