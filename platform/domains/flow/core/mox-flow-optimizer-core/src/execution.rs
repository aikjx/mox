//! Reusable bounded workflow execution. The caller owns authentication, configuration and audit persistence.
use mox_flow_operator_core::{
    category::Workflow,
    operator::{FunctionOperator, IdentityOperator, LinearOperator, Operator},
    state::StateVector,
    ExecutionContext,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ExecuteRequest {
    pub workflow: Vec<String>,
    pub input: Vec<f64>,
    pub parameters: Option<HashMap<String, f64>>,
}

#[derive(Debug, Serialize)]
pub struct ExecuteResponse {
    pub success: bool,
    pub output: Option<Vec<f64>>,
    pub execution_time_ms: u64,
    pub logs: Vec<String>,
    pub error: Option<String>,
    pub metrics: Option<ExecutionMetrics>,
}

#[derive(Debug, Serialize)]
pub struct ExecutionMetrics {
    pub input_norm: f64,
    pub output_norm: f64,
    pub l1_residual: f64,
}

#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    pub max_dimension: usize,
    pub max_steps: usize,
    pub max_allocation_bytes: usize,
    pub max_cpu: u64,
    pub max_memory: u64,
}
impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_dimension: 1024,
            max_steps: 128,
            max_allocation_bytes: 64 * 1024 * 1024,
            max_cpu: 1_000_000_000_000,
            max_memory: 1_000_000_000_000,
        }
    }
}
fn rejected(message: String, start: std::time::Instant) -> ExecuteResponse {
    ExecuteResponse {
        success: false,
        output: None,
        execution_time_ms: start.elapsed().as_millis() as u64,
        logs: vec![],
        error: Some(message),
        metrics: None,
    }
}

pub fn execute(req: ExecuteRequest, limits: &ExecutionLimits) -> ExecuteResponse {
    let start = std::time::Instant::now();
    if req.workflow.is_empty() || req.workflow.len() > limits.max_steps {
        return rejected(format!("工作流步骤数必须在 1..={} 范围内", limits.max_steps), start);
    }
    // Reject dimensions before copying input or constructing any operator state.
    if req.input.is_empty() || req.input.len() > limits.max_dimension {
        return rejected(
            format!("输入维度必须在 1..={} 范围内", limits.max_dimension),
            start,
        );
    }
    if req.input.iter().any(|v| !v.is_finite())
        || req.parameters.as_ref().is_some_and(|p| p.values().any(|v| !v.is_finite()))
    {
        return rejected("输入和参数必须为有限数值".into(), start);
    }
    let linear_count = req.workflow.iter().filter(|id| id.as_str() == "linear").count();
    let allocation = req
        .input
        .len()
        .checked_mul(req.input.len())
        .and_then(|n| n.checked_mul(8))
        .and_then(|n| n.checked_mul(linear_count));
    if allocation.map_or(true, |bytes| {
        bytes > limits.max_allocation_bytes || bytes as u64 > limits.max_memory
    }) {
        return rejected("线性算子矩阵分配超过内存预算".into(), start);
    }
    let mut ctx = ExecutionContext::default();
    let input = StateVector::from_vec(req.input);
    let input_norm = input.norm();
    if !input_norm.is_finite() {
        return rejected("输入范数超出可计算范围".into(), start);
    }
    let params = req.parameters.unwrap_or_default();

    // 公理5（资源约束优化）接线：构造算子 DAG 做调度预检。
    // 每个算子按请求顺序建立串行依赖；预检通过后才进入真实执行。
    // 配额默认宽松（10^12 cycles / 10^12 B），可通过环境变量收紧：
    //   OUS_EXEC_MAX_CPU / OUS_EXEC_MAX_MEM（企业部署建议显式配置）。
    let mut dag_ops: Vec<std::sync::Arc<dyn Operator>> = Vec::new();
    for op_id in &req.workflow {
        let arc: std::sync::Arc<dyn Operator> = match op_id.as_str() {
            "identity" => std::sync::Arc::new(IdentityOperator::new(input.dimension)),
            "linear" => {
                let scale = params.get("scale").copied().unwrap_or(2.0);
                let n = input.dimension;
                std::sync::Arc::new(LinearOperator::new(nalgebra::DMatrix::from_diagonal_element(
                    n, n, scale,
                )))
            },
            "normalize" => {
                std::sync::Arc::new(FunctionOperator::new("normalize", |s: &StateVector, _ctx| {
                    let mut s = s.clone();
                    s.normalize();
                    Ok(s)
                }))
            },
            "normalize_l1" => std::sync::Arc::new(FunctionOperator::new(
                "normalize_l1",
                |s: &StateVector, _ctx| {
                    let mut s = s.clone();
                    s.normalize_probability();
                    Ok(s)
                },
            )),
            "relu" => {
                std::sync::Arc::new(FunctionOperator::new("relu", |s: &StateVector, _ctx| {
                    let mut result = s.clone();
                    for i in 0..result.dimension {
                        result[i] = result[i].max(0.0);
                    }
                    Ok(result)
                }))
            },
            "sigmoid" => {
                std::sync::Arc::new(FunctionOperator::new("sigmoid", |s: &StateVector, _ctx| {
                    let mut result = s.clone();
                    for i in 0..result.dimension {
                        result[i] = 1.0 / (1.0 + (-result[i]).exp());
                    }
                    Ok(result)
                }))
            },
            "tanh" => {
                std::sync::Arc::new(FunctionOperator::new("tanh", |s: &StateVector, _ctx| {
                    let mut result = s.clone();
                    for i in 0..result.dimension {
                        result[i] = result[i].tanh();
                    }
                    Ok(result)
                }))
            },
            "softmax" => {
                std::sync::Arc::new(FunctionOperator::new("softmax", |s: &StateVector, _ctx| {
                    let mut result = s.clone();
                    let max_val =
                        (0..result.dimension).map(|i| result[i]).fold(f64::NEG_INFINITY, f64::max);
                    let sum_exp: f64 =
                        (0..result.dimension).map(|i| (result[i] - max_val).exp()).sum();
                    for i in 0..result.dimension {
                        result[i] = (result[i] - max_val).exp() / sum_exp;
                    }
                    Ok(result)
                }))
            },
            "scale" => {
                let factor = params.get("factor").copied().unwrap_or(1.0);
                std::sync::Arc::new(FunctionOperator::new("scale", move |s: &StateVector, _ctx| {
                    let mut result = s.clone();
                    for i in 0..result.dimension {
                        result[i] *= factor;
                    }
                    Ok(result)
                }))
            },
            _ => {
                return ExecuteResponse {
                    success: false,
                    output: None,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    logs: vec![],
                    error: Some(format!("未知算子: {}", op_id)),
                    metrics: None,
                };
            },
        };
        dag_ops.push(arc);
    }

    // 构建串行 DAG 并执行公理5 资源约束预检（拓扑有效 + 配额内）
    let mut dag = crate::OperatorDag::new();
    for (i, op) in dag_ops.iter().enumerate() {
        let step_id = format!("step-{i}");
        dag.add_operator(&step_id, op.clone());
        if i > 0 {
            if let Err(e) = dag.add_dependency(&format!("step-{}", i - 1), &step_id) {
                return ExecuteResponse {
                    success: false,
                    output: None,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    logs: vec![],
                    error: Some(e),
                    metrics: None,
                };
            }
        }
    }
    if let Err(e) = dag.topological_order() {
        return ExecuteResponse {
            success: false,
            output: None,
            execution_time_ms: start.elapsed().as_millis() as u64,
            logs: vec![],
            error: Some(format!("调度预检失败（DAG 含环）: {}", e)),
            metrics: None,
        };
    }
    let max_cpu = limits.max_cpu;
    let max_mem = limits.max_memory;
    let scheduler = crate::ResourceOptimizer::new(max_cpu, max_mem);
    if !scheduler.check_resources(&dag_ops) {
        let cost = dag.estimated_resource_cost();
        return ExecuteResponse {
            success: false, output: None,
            execution_time_ms: start.elapsed().as_millis() as u64,
            logs: vec![], error: Some(format!(
                "公理5 资源约束预检失败：估算 CPU={} cycles / MEM={} B 超出配额 CPU={} / MEM={}（可用 OUS_EXEC_MAX_CPU/OUS_EXEC_MAX_MEM 调优）",
                cost.cpu_cycles, cost.memory_bytes, max_cpu, max_mem
            )), metrics: None,
        };
    }
    let est_ms = dag.estimated_execution_time();
    let est_cost = dag.estimated_resource_cost();
    let mut all_logs =
        vec![format!(
        "[scheduler] 公理5 预检通过: 关键路径={:?} 预估执行时间={}ms 资源成本=CPU {} / MEM {} B",
        dag.critical_path(), est_ms, est_cost.cpu_cycles, est_cost.memory_bytes
    )];

    let mut workflow = Workflow::new("ai-workflow");
    for op in dag_ops {
        match workflow.then_shared(op) {
            Ok(next) => workflow = next,
            Err(error) => return rejected(error.to_string(), start),
        }
    }

    match workflow.execute(&input, &mut ctx) {
        Ok(result) => {
            let output_norm = result.output_state.as_ref().map(|s| s.norm()).unwrap_or(0.0);
            let l1_residual = result
                .output_state
                .as_ref()
                .map(|_| (input_norm - output_norm).abs())
                .unwrap_or(0.0);
            if !output_norm.is_finite() || !l1_residual.is_finite() {
                return rejected("执行结果超出可计算范围".into(), start);
            }
            all_logs.extend(result.logs.clone());

            ExecuteResponse {
                success: result.success,
                output: result.output_state.map(|s| s.to_vec()),
                execution_time_ms: result.execution_time_ms,
                logs: all_logs,
                error: result.error,
                metrics: Some(ExecutionMetrics { input_norm, output_norm, l1_residual }),
            }
        },
        Err(e) => ExecuteResponse {
            success: false,
            output: None,
            execution_time_ms: start.elapsed().as_millis() as u64,
            logs: all_logs,
            error: Some(e.to_string()),
            metrics: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(steps: &[&str], input: Vec<f64>) -> ExecuteRequest {
        ExecuteRequest {
            workflow: steps.iter().map(|s| (*s).into()).collect(),
            input,
            parameters: None,
        }
    }
    #[test]
    fn repeated_operator_names_are_distinct_steps() {
        let response =
            execute(request(&["linear", "linear"], vec![1.0, 2.0]), &ExecutionLimits::default());
        assert!(response.success, "{:?}", response.error);
        assert_eq!(response.output.unwrap(), vec![4.0, 8.0]);
    }
    #[test]
    fn invalid_dimensions_are_rejected_before_matrix_budget() {
        let limits = ExecutionLimits { max_dimension: 2, max_allocation_bytes: 0, ..Default::default() };
        for input in [vec![], vec![1.0; 3]] {
            let response = execute(request(&["linear"], input), &limits);
            assert!(!response.success);
            assert!(response.error.unwrap().contains("输入维度"));
        }
        assert!(execute(request(&["identity"], vec![1.0; 2]), &limits).success);
    }
    #[test]
    fn matrix_budget_is_checked_before_operator_construction() {
        let limits = ExecutionLimits { max_allocation_bytes: 8, ..Default::default() };
        let response = execute(request(&["linear"], vec![1.0; 1024]), &limits);
        assert!(!response.success);
        assert!(response.error.unwrap().contains("矩阵分配"));
    }
    #[test]
    fn rejects_nonfinite_inputs_and_unbounded_steps() {
        assert!(!execute(request(&["relu"], vec![f64::NAN]), &ExecutionLimits::default()).success);
        let limits = ExecutionLimits { max_steps: 1, ..Default::default() };
        assert!(!execute(request(&["identity", "identity"], vec![1.0]), &limits).success);
        assert!(!execute(request(&[], vec![1.0]), &limits).success);
    }
    #[test]
    fn unknown_operators_and_insufficient_resources_fail_explicitly() {
        assert!(!execute(request(&["missing"], vec![1.0]), &ExecutionLimits::default()).success);
        let limits = ExecutionLimits { max_memory: 0, ..Default::default() };
        assert!(!execute(request(&["linear"], vec![1.0]), &limits).success);
    }
}
