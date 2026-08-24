//! # 算子Trait定义
//!
//! 实现公理1：万物皆算子
//! 所有操作都抽象为算子，支持组合、张量积、对偶等运算

use std::sync::Arc;
use std::time::Instant;

use crate::category::ComposedOperator;
use crate::resource::{ResourceCost, ResourceUsage};
use crate::state::StateVector;
use crate::types::{builtin, TypeCheck, TypeIdentifier};
use crate::{ExecutionContext, ExecutionResult, OperatorMetadata, Result};

/// 核心算子Trait
///
/// 所有算子必须实现此Trait，保证：
/// 1. 类型安全：明确的输入输出类型
/// 2. 可组合：通过Category组合子自动组合
/// 3. 资源感知：报告资源消耗
/// 4. 可观测：执行日志和残差监控
pub trait Operator: Send + Sync + TypeCheck {
    /// 获取算子元数据
    fn metadata(&self) -> OperatorMetadata;

    /// 执行算子
    fn apply(&self, input: &StateVector, ctx: &mut ExecutionContext) -> Result<StateVector>;

    /// 获取算子资源消耗模型
    fn resource_cost(&self) -> ResourceCost {
        self.metadata().resource_cost
    }

    /// 执行算子并返回完整结果（包含监控信息）
    fn execute(&self, input: &StateVector, ctx: &mut ExecutionContext) -> Result<ExecutionResult> {
        let start = Instant::now();
        let initial_resources = ctx.resources;

        if ctx.config.enable_type_check {
            let expected_input = self.input_type();
            let actual_input =
                TypeIdentifier::new(input.metadata["type"].as_str().unwrap_or("StateVector"));
            if !expected_input.matches(&actual_input)
                && !expected_input.matches(&builtin::any_type())
            {
                return Ok(ExecutionResult {
                    success: false,
                    output_state: None,
                    residual: f64::INFINITY,
                    resources_used: ResourceUsage::zero(),
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    logs: vec![format!(
                        "类型不匹配: 期望 {}, 实际 {}",
                        expected_input, actual_input
                    )],
                    error: Some(format!(
                        "类型不匹配: 期望 {}, 实际 {}",
                        expected_input, actual_input
                    )),
                });
            }
            tracing::debug!("算子 {} 输入类型检查通过", self.metadata().name);
        }

        let mut logs = Vec::new();
        let output = match self.apply(input, ctx) {
            Ok(output) => output,
            Err(e) => {
                return Ok(ExecutionResult {
                    success: false,
                    output_state: None,
                    residual: f64::INFINITY,
                    resources_used: ctx.resources - initial_resources,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    logs,
                    error: Some(e.to_string()),
                });
            }
        };

        let residual = 0.0;
        let execution_time = start.elapsed().as_millis() as u64;
        let resources_used = ctx.resources - initial_resources;

        logs.push(format!(
            "算子 {} 执行完成，耗时 {}ms",
            self.metadata().name,
            execution_time
        ));

        Ok(ExecutionResult {
            success: true,
            output_state: Some(output),
            residual,
            resources_used,
            execution_time_ms: execution_time,
            logs,
            error: None,
        })
    }

    /// 算子组合：self ∘ other，即先执行other，再执行self
    fn compose<O: Operator + 'static>(self, other: O) -> ComposedOperator
    where
        Self: Sized + 'static,
    {
        ComposedOperator::new(Arc::new(other), Arc::new(self))
    }

    /// 转换为Arc<dyn Operator>以便于组合
    fn into_arc(self) -> Arc<dyn Operator>
    where
        Self: Sized + 'static,
    {
        Arc::new(self)
    }
}

/// 恒等算子
pub struct IdentityOperator {
    pub dimension: usize,
}

impl IdentityOperator {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

impl Operator for IdentityOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata {
            id: crate::generate_operator_id(),
            name: "Identity".to_string(),
            version: "1.0.0".to_string(),
            description: "恒等算子，输出等于输入".to_string(),
            input_type: builtin::state_vector_type(),
            output_type: builtin::state_vector_type(),
            resource_cost: ResourceCost::minimal(),
            author: "System".to_string(),
            tags: vec!["core".to_string(), "identity".to_string()],
        }
    }

    fn apply(&self, input: &StateVector, _ctx: &mut ExecutionContext) -> Result<StateVector> {
        Ok(input.clone())
    }
}

impl TypeCheck for IdentityOperator {
    fn input_type(&self) -> TypeIdentifier {
        builtin::state_vector_type()
    }

    fn output_type(&self) -> TypeIdentifier {
        builtin::state_vector_type()
    }
}

/// 线性变换算子：y = Mx + b
pub struct LinearOperator {
    matrix: nalgebra::DMatrix<f64>,
    bias: Option<nalgebra::DVector<f64>>,
}

impl LinearOperator {
    pub fn new(matrix: nalgebra::DMatrix<f64>) -> Self {
        Self { matrix, bias: None }
    }

    pub fn with_bias(matrix: nalgebra::DMatrix<f64>, bias: nalgebra::DVector<f64>) -> Self {
        Self {
            matrix,
            bias: Some(bias),
        }
    }

    pub fn identity(dimension: usize) -> Self {
        Self::new(nalgebra::DMatrix::identity(dimension, dimension))
    }
}

impl Operator for LinearOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata {
            id: crate::generate_operator_id(),
            name: "LinearTransform".to_string(),
            version: "1.0.0".to_string(),
            description: "线性变换算子 y = Mx + b".to_string(),
            input_type: builtin::state_vector_type(),
            output_type: builtin::state_vector_type(),
            resource_cost: ResourceCost::new(
                (self.matrix.nrows() * self.matrix.ncols()) as u64,
                (self.matrix.nrows() * self.matrix.ncols() * 8) as u64,
            ),
            author: "System".to_string(),
            tags: vec!["core".to_string(), "linear".to_string()],
        }
    }

    fn apply(&self, input: &StateVector, _ctx: &mut ExecutionContext) -> Result<StateVector> {
        // 安全加固：nalgebra 矩阵乘法对维度不匹配会直接 panic（进程崩溃），需在乘前显式校验。
        if self.matrix.ncols() != input.dimension {
            return Err(crate::OperatorError::ExecutionError(format!(
                "线性算子维度不匹配: 矩阵列数 {} != 输入维度 {}",
                self.matrix.ncols(),
                input.dimension
            )));
        }
        let mut result = self.matrix.clone() * &input.data;
        if let Some(bias) = &self.bias {
            result += bias;
        }
        Ok(StateVector {
            data: result,
            dimension: self.matrix.nrows(),
            timestamp: input.timestamp,
            metadata: input.metadata.clone(),
        })
    }
}

impl TypeCheck for LinearOperator {
    fn input_type(&self) -> TypeIdentifier {
        builtin::state_vector_type()
    }

    fn output_type(&self) -> TypeIdentifier {
        builtin::state_vector_type()
    }
}

/// 函数算子：包装任意函数作为算子
pub struct FunctionOperator<F>
where
    F: Fn(&StateVector, &mut ExecutionContext) -> Result<StateVector> + Send + Sync,
{
    meta: OperatorMetadata,
    func: F,
}

impl<F> FunctionOperator<F>
where
    F: Fn(&StateVector, &mut ExecutionContext) -> Result<StateVector> + Send + Sync,
{
    pub fn new(name: &str, func: F) -> Self {
        Self {
            meta: OperatorMetadata {
                id: crate::generate_operator_id(),
                name: name.to_string(),
                version: "1.0.0".to_string(),
                description: format!("函数算子: {}", name),
                input_type: builtin::state_vector_type(),
                output_type: builtin::state_vector_type(),
                resource_cost: ResourceCost::default(),
                author: "User".to_string(),
                tags: vec!["function".to_string()],
            },
            func,
        }
    }
}

impl<F> Operator for FunctionOperator<F>
where
    F: Fn(&StateVector, &mut ExecutionContext) -> Result<StateVector> + Send + Sync,
{
    fn metadata(&self) -> OperatorMetadata {
        self.meta.clone()
    }

    fn apply(&self, input: &StateVector, ctx: &mut ExecutionContext) -> Result<StateVector> {
        (self.func)(input, ctx)
    }
}

impl<F> TypeCheck for FunctionOperator<F>
where
    F: Fn(&StateVector, &mut ExecutionContext) -> Result<StateVector> + Send + Sync,
{
    fn input_type(&self) -> TypeIdentifier {
        builtin::state_vector_type()
    }

    fn output_type(&self) -> TypeIdentifier {
        builtin::state_vector_type()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_identity_operator() {
        let op = IdentityOperator::new(3);
        let input = StateVector::from_vec(vec![1.0, 2.0, 3.0]);
        let mut ctx = ExecutionContext::default();
        let output = op.apply(&input, &mut ctx).unwrap();
        assert_relative_eq!(output[0], 1.0);
        assert_relative_eq!(output[1], 2.0);
        assert_relative_eq!(output[2], 3.0);
    }

    #[test]
    fn test_linear_operator() {
        // 2x2矩阵 [[1,2],[3,4]]
        let matrix = nalgebra::DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let op = LinearOperator::new(matrix);
        let input = StateVector::from_vec(vec![1.0, 1.0]);
        let mut ctx = ExecutionContext::default();
        let output = op.apply(&input, &mut ctx).unwrap();
        assert_relative_eq!(output[0], 3.0); // 1*1 + 2*1
        assert_relative_eq!(output[1], 7.0); // 3*1 + 4*1
    }
}
