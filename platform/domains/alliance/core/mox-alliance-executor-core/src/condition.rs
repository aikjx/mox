// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 条件表达式引擎（Dynamic 路由专用）
//!
//! 支持的表达式语法：
//! - 比较：output.success == true, output.score > 0.8
//! - 逻辑：A && B, A || B, !A
//! - 组合：(A || B) && C

use serde_json::Value;

/// 条件表达式
#[derive(Debug, Clone)]
pub enum Condition {
    /// 比较操作
    Compare(CompareOp),
    /// 逻辑与
    And(Box<Condition>, Box<Condition>),
    /// 逻辑或
    Or(Box<Condition>, Box<Condition>),
    /// 逻辑非
    Not(Box<Condition>),
}

/// 比较操作
#[derive(Debug, Clone)]
pub struct CompareOp {
    pub left: Operand,
    pub operator: Operator,
    pub right: Operand,
}

/// 操作数
#[derive(Debug, Clone)]
pub enum Operand {
    /// 输出字段引用，如 output.score
    Field(String),
    /// 字面量值
    Literal(Value),
}

/// 比较运算符
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    Eq,
    NotEq,
    GreaterThan,
    GreaterThanOrEq,
    LessThan,
    LessThanOrEq,
}

impl Condition {
    /// 评估条件
    pub fn evaluate(&self, context: &Value) -> bool {
        match self {
            Condition::Compare(op) => op.evaluate(context),
            Condition::And(a, b) => a.evaluate(context) && b.evaluate(context),
            Condition::Or(a, b) => a.evaluate(context) || b.evaluate(context),
            Condition::Not(a) => !a.evaluate(context),
        }
    }
}

impl CompareOp {
    fn evaluate(&self, context: &Value) -> bool {
        let left_val = self.left.resolve(context);
        let right_val = self.right.resolve(context);

        match self.operator {
            Operator::Eq => left_val == right_val,
            Operator::NotEq => left_val != right_val,
            Operator::GreaterThan => compare_numbers(&left_val, &right_val).map(|c| c > 0).unwrap_or(false),
            Operator::GreaterThanOrEq => compare_numbers(&left_val, &right_val).map(|c| c >= 0).unwrap_or(false),
            Operator::LessThan => compare_numbers(&left_val, &right_val).map(|c| c < 0).unwrap_or(false),
            Operator::LessThanOrEq => compare_numbers(&left_val, &right_val).map(|c| c <= 0).unwrap_or(false),
        }
    }
}

impl Operand {
    fn resolve(&self, context: &Value) -> Value {
        match self {
            Operand::Field(path) => {
                // 简化实现：output.score -> context["score"]
                context.get(path.replace("output.", "")).cloned().unwrap_or(Value::Null)
            }
            Operand::Literal(v) => v.clone(),
        }
    }
}

/// 比较两个 JSON 值
fn compare_numbers(a: &Value, b: &Value) -> Result<i32, ()> {
    let a_f = a.as_f64().ok_or(())?;
    let b_f = b.as_f64().ok_or(())?;
    Ok(a_f.partial_cmp(&b_f).unwrap_or(std::cmp::Ordering::Equal) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_equality() {
        let cond = Condition::Compare(CompareOp {
            left: Operand::Field("output.success".to_string()),
            operator: Operator::Eq,
            right: Operand::Literal(json!(true)),
        });

        let ctx = json!({ "success": true });
        assert!(cond.evaluate(&ctx));

        let ctx2 = json!({ "success": false });
        assert!(!cond.evaluate(&ctx2));
    }

    #[test]
    fn test_greater_than() {
        let cond = Condition::Compare(CompareOp {
            left: Operand::Field("output.score".to_string()),
            operator: Operator::GreaterThan,
            right: Operand::Literal(json!(0.8)),
        });

        let ctx = json!({ "score": 0.9 });
        assert!(cond.evaluate(&ctx));

        let ctx2 = json!({ "score": 0.5 });
        assert!(!cond.evaluate(&ctx2));
    }

    #[test]
    fn test_logical_and() {
        let cond = Condition::And(
            Box::new(Condition::Compare(CompareOp {
                left: Operand::Field("output.success".to_string()),
                operator: Operator::Eq,
                right: Operand::Literal(json!(true)),
            })),
            Box::new(Condition::Compare(CompareOp {
                left: Operand::Field("output.score".to_string()),
                operator: Operator::GreaterThan,
                right: Operand::Literal(json!(0.8)),
            })),
        );

        let ctx = json!({ "success": true, "score": 0.9 });
        assert!(cond.evaluate(&ctx));

        let ctx2 = json!({ "success": true, "score": 0.5 });
        assert!(!cond.evaluate(&ctx2));
    }
}
