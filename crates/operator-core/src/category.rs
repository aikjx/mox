//! # 范畴论组合子
//!
//! 实现公理4：插件满足范畴论态射规则
//! 实现算子的范畴论组合，满足结合律、单位律

use std::sync::Arc;

use crate::operator::Operator;
use crate::resource::ResourceCost;
use crate::state::StateVector;
use crate::types::{TypeCheck, TypeIdentifier};
use crate::{ExecutionContext, ExecutionResult, OperatorError, OperatorMetadata, Result};

/// 组合算子：g ∘ f，先执行f，再执行g
pub struct ComposedOperator {
    first: Arc<dyn Operator>,
    second: Arc<dyn Operator>,
}

impl ComposedOperator {
    pub fn new(first: Arc<dyn Operator>, second: Arc<dyn Operator>) -> Self {
        // 类型检查：f的输出类型必须等于g的输入类型
        let f_type = first.type_pair();
        let g_type = second.type_pair();
        assert!(
            f_type.can_compose(&g_type),
            "类型不匹配：无法组合 {} 和 {}",
            f_type,
            g_type
        );
        Self { first, second }
    }
}

impl Operator for ComposedOperator {
    fn metadata(&self) -> OperatorMetadata {
        let f_meta = self.first.metadata();
        let g_meta = self.second.metadata();
        OperatorMetadata {
            id: crate::generate_operator_id(),
            name: format!("{}∘{}", g_meta.name, f_meta.name),
            version: "1.0.0".to_string(),
            description: format!("组合算子：先 {}，再 {}", f_meta.name, g_meta.name),
            input_type: f_meta.input_type,
            output_type: g_meta.output_type,
            resource_cost: f_meta.resource_cost + g_meta.resource_cost,
            author: "System".to_string(),
            tags: vec!["composed".to_string()],
        }
    }

    fn apply(&self, input: &StateVector, ctx: &mut ExecutionContext) -> Result<StateVector> {
        let intermediate = self.first.apply(input, ctx)?;
        self.second.apply(&intermediate, ctx)
    }
}

impl TypeCheck for ComposedOperator {
    fn input_type(&self) -> TypeIdentifier {
        self.first.input_type()
    }

    fn output_type(&self) -> TypeIdentifier {
        self.second.output_type()
    }
}

/// 算子工作流：算子序列
pub struct Workflow {
    operators: Vec<Arc<dyn Operator>>,
    name: String,
}

impl Workflow {
    pub fn new(name: &str) -> Self {
        Self {
            operators: Vec::new(),
            name: name.to_string(),
        }
    }

    /// 添加算子到工作流末尾
    pub fn then<O: Operator + 'static>(mut self, op: O) -> Result<Self> {
        if let Some(last) = self.operators.last() {
            let last_type = last.type_pair();
            let new_type = op.type_pair();
            if !last_type.can_compose(&new_type) {
                return Err(OperatorError::CompositionError(format!(
                    "类型不匹配：{} 无法接在 {} 后面",
                    new_type, last_type
                )));
            }
        }
        self.operators.push(Arc::new(op));
        Ok(self)
    }

    /// 添加算子到工作流开头
    pub fn compose_before<O: Operator + 'static>(mut self, op: O) -> Result<Self> {
        if let Some(first) = self.operators.first() {
            let new_type = op.type_pair();
            let first_type = first.type_pair();
            if !new_type.can_compose(&first_type) {
                return Err(OperatorError::CompositionError(format!(
                    "类型不匹配：{} 无法接在 {} 前面",
                    new_type, first_type
                )));
            }
        }
        self.operators.insert(0, Arc::new(op));
        Ok(self)
    }

    pub fn len(&self) -> usize {
        self.operators.len()
    }

    pub fn is_empty(&self) -> bool {
        self.operators.is_empty()
    }
}

impl Operator for Workflow {
    fn metadata(&self) -> OperatorMetadata {
        let total_cost = self
            .operators
            .iter()
            .map(|op| op.resource_cost())
            .fold(ResourceCost::zero(), |a, b| a + b);
        OperatorMetadata {
            id: crate::generate_operator_id(),
            name: self.name.clone(),
            version: "1.0.0".to_string(),
            description: format!("工作流，包含 {} 个算子", self.operators.len()),
            input_type: self
                .operators
                .first()
                .map(|op| op.metadata().input_type)
                .unwrap_or_else(|| "Any".to_string()),
            output_type: self
                .operators
                .last()
                .map(|op| op.metadata().output_type)
                .unwrap_or_else(|| "Any".to_string()),
            resource_cost: total_cost,
            author: "User".to_string(),
            tags: vec!["workflow".to_string()],
        }
    }

    fn apply(&self, input: &StateVector, ctx: &mut ExecutionContext) -> Result<StateVector> {
        let mut current = input.clone();
        for op in &self.operators {
            current = op.apply(&current, ctx)?;
        }
        Ok(current)
    }

    fn execute(&self, input: &StateVector, ctx: &mut ExecutionContext) -> Result<ExecutionResult> {
        let start = std::time::Instant::now();
        let initial_resources = ctx.resources;
        let mut all_logs = Vec::new();
        let mut current = input.clone();

        for (i, op) in self.operators.iter().enumerate() {
            tracing::info!("执行工作流第 {}/{} 步: {}", i + 1, self.operators.len(), op.metadata().name);
            let result = op.execute(&current, ctx)?;
            if !result.success {
                return Ok(ExecutionResult {
                    success: false,
                    output_state: None,
                    residual: result.residual,
                    resources_used: ctx.resources - initial_resources,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    logs: all_logs,
                    error: result.error,
                });
            }
            all_logs.extend(result.logs);
            current = result.output_state.unwrap();
        }

        Ok(ExecutionResult {
            success: true,
            output_state: Some(current),
            residual: 0.0,
            resources_used: ctx.resources - initial_resources,
            execution_time_ms: start.elapsed().as_millis() as u64,
            logs: all_logs,
            error: None,
        })
    }
}

impl TypeCheck for Workflow {
    fn input_type(&self) -> TypeIdentifier {
        self.operators
            .first()
            .map(|op| op.input_type())
            .unwrap_or_else(|| TypeIdentifier::new("Any"))
    }

    fn output_type(&self) -> TypeIdentifier {
        self.operators
            .last()
            .map(|op| op.output_type())
            .unwrap_or_else(|| TypeIdentifier::new("Any"))
    }
}

/// 张量积算子：并行执行两个算子
pub struct TensorProductOperator {
    op1: Arc<dyn Operator>,
    op2: Arc<dyn Operator>,
}

impl TensorProductOperator {
    pub fn new(op1: Arc<dyn Operator>, op2: Arc<dyn Operator>) -> Self {
        Self { op1, op2 }
    }
}

impl Operator for TensorProductOperator {
    fn metadata(&self) -> OperatorMetadata {
        let m1 = self.op1.metadata();
        let m2 = self.op2.metadata();
        OperatorMetadata {
            id: crate::generate_operator_id(),
            name: format!("{}⊗{}", m1.name, m2.name),
            version: "1.0.0".to_string(),
            description: "张量积算子，并行执行两个算子".to_string(),
            input_type: format!("{}×{}", m1.input_type, m2.input_type),
            output_type: format!("{}×{}", m1.output_type, m2.output_type),
            resource_cost: m1.resource_cost + m2.resource_cost,
            author: "System".to_string(),
            tags: vec!["tensor".to_string(), "parallel".to_string()],
        }
    }

    fn apply(&self, input: &StateVector, ctx: &mut ExecutionContext) -> Result<StateVector> {
        // 简化实现：将输入向量分为两半，分别执行后拼接
        let n = input.dimension / 2;
        let mut input1 = StateVector::new(n);
        let mut input2 = StateVector::new(input.dimension - n);
        for i in 0..n {
            input1[i] = input[i];
        }
        for i in n..input.dimension {
            input2[i - n] = input[i];
        }

        let output1 = self.op1.apply(&input1, ctx)?;
        let output2 = self.op2.apply(&input2, ctx)?;

        let mut result = StateVector::new(output1.dimension + output2.dimension);
        for i in 0..output1.dimension {
            result[i] = output1[i];
        }
        for i in 0..output2.dimension {
            result[output1.dimension + i] = output2[i];
        }
        Ok(result)
    }
}

impl TypeCheck for TensorProductOperator {
    fn input_type(&self) -> TypeIdentifier {
        TypeIdentifier::new("TensorProduct")
    }

    fn output_type(&self) -> TypeIdentifier {
        TypeIdentifier::new("TensorProduct")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::{IdentityOperator, LinearOperator};
    use approx::assert_relative_eq;

    #[test]
    fn test_composed_operator() {
        let op1 = LinearOperator::new(nalgebra::DMatrix::identity(3, 3));
        let op2 = IdentityOperator::new(3);
        let composed = op1.compose(op2);

        let input = StateVector::from_vec(vec![1.0, 2.0, 3.0]);
        let mut ctx = ExecutionContext::default();
        let output = composed.apply(&input, &mut ctx).unwrap();
        assert_relative_eq!(output[0], 1.0);
        assert_relative_eq!(output[1], 2.0);
        assert_relative_eq!(output[2], 3.0);
    }

    #[test]
    fn test_workflow() {
        let workflow = Workflow::new("test")
            .then(IdentityOperator::new(2)).unwrap()
            .then(LinearOperator::new(nalgebra::DMatrix::from_diagonal_element(2, 2, 2.0))).unwrap();

        let input = StateVector::from_vec(vec![1.0, 2.0]);
        let mut ctx = ExecutionContext::default();
        let output = workflow.apply(&input, &mut ctx).unwrap();
        assert_relative_eq!(output[0], 2.0);
        assert_relative_eq!(output[1], 4.0);
    }

    #[test]
    fn test_category_laws() {
        // 测试单位律：id ∘ f = f = f ∘ id
        let matrix = nalgebra::DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let input = StateVector::from_vec(vec![1.0, 1.0]);
        let mut ctx = ExecutionContext::default();

        let f = LinearOperator::new(matrix.clone());
        let f_result = f.apply(&input, &mut ctx).unwrap();
        let id_f = IdentityOperator::new(2).compose(LinearOperator::new(matrix.clone()));
        let f_id = LinearOperator::new(matrix).compose(IdentityOperator::new(2));

        let id_f_result = id_f.apply(&input, &mut ctx).unwrap();
        let f_id_result = f_id.apply(&input, &mut ctx).unwrap();

        for i in 0..2 {
            assert_relative_eq!(f_result[i], id_f_result[i]);
            assert_relative_eq!(f_result[i], f_id_result[i]);
        }
    }
}
