//! 公式编译器
//!
//! 将 AST 编译为字节码或直接可执行的指令序列

use crate::ast::Expr;
use crate::error::FormulaResult;

/// 编译后的公式
#[derive(Debug, Clone)]
pub struct CompiledFormula {
    /// 原始表达式（占位，后续替换为字节码）
    pub expr: Expr,
}

/// 将 AST 编译为可执行形式
///
/// # Arguments
/// * `expr` - AST 表达式
///
/// # Returns
/// 编译后的公式
pub fn compile(expr: Expr) -> FormulaResult<CompiledFormula> {
    // TODO: 实现完整的编译器
    // 当前为占位实现，直接包装 AST
    Ok(CompiledFormula { expr })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, FormulaValue};

    #[test]
    fn test_compile_literal() {
        let expr = Expr::Literal(FormulaValue::Number(42.0));
        let result = compile(expr);
        assert!(result.is_ok());
    }
}
