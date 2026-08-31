//! 公式执行运行时
//!
//! 执行编译后的公式，支持变量上下文和函数注册

use std::collections::HashMap;
use crate::ast::{Expr, FormulaValue};
use crate::error::FormulaResult;
use crate::compiler::CompiledFormula;

/// 执行上下文：变量环境
#[derive(Debug, Clone, Default)]
pub struct EvalContext {
    /// 变量映射
    variables: HashMap<String, FormulaValue>,
}

impl EvalContext {
    /// 创建空上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置变量
    pub fn set_var(&mut self, name: impl Into<String>, value: FormulaValue) {
        self.variables.insert(name.into(), value);
    }

    /// 获取变量
    pub fn get_var(&self, name: &str) -> Option<&FormulaValue> {
        self.variables.get(name)
    }
}

/// 直接计算表达式（解释执行）
///
/// # Arguments
/// * `expr` - AST 表达式
/// * `ctx` - 执行上下文
///
/// # Returns
/// 计算结果值
pub fn evaluate(expr: &Expr, ctx: &EvalContext) -> FormulaResult<FormulaValue> {
    // TODO: 实现完整的表达式求值
    let _ = ctx;
    match expr {
        Expr::Literal(v) => Ok(v.clone()),
        _ => Ok(FormulaValue::Null),
    }
}

/// 执行编译后的公式
///
/// # Arguments
/// * `formula` - 编译后的公式
/// * `ctx` - 执行上下文
///
/// # Returns
/// 计算结果值
pub fn execute(formula: &CompiledFormula, ctx: &EvalContext) -> FormulaResult<FormulaValue> {
    evaluate(&formula.expr, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_literal() {
        let ctx = EvalContext::new();
        let expr = Expr::Literal(FormulaValue::Number(3.14));
        let result = evaluate(&expr, &ctx).unwrap();
        assert_eq!(result, FormulaValue::Number(3.14));
    }

    #[test]
    fn test_context_set_get() {
        let mut ctx = EvalContext::new();
        ctx.set_var("x", FormulaValue::Number(10.0));
        assert_eq!(ctx.get_var("x"), Some(&FormulaValue::Number(10.0)));
        assert_eq!(ctx.get_var("y"), None);
    }
}
