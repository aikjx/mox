//! 公式引擎错误类型

use thiserror::Error;

/// 公式错误
#[derive(Debug, Error)]
pub enum FormulaError {
    /// 语法错误
    #[error("语法错误: {0}")]
    SyntaxError(String),

    /// 编译错误
    #[error("编译错误: {0}")]
    CompileError(String),

    /// 运行时错误
    #[error("运行时错误: {0}")]
    RuntimeError(String),

    /// 未定义变量
    #[error("未定义变量: {0}")]
    UndefinedVariable(String),

    /// 未定义函数
    #[error("未定义函数: {0}")]
    UndefinedFunction(String),

    /// 类型不匹配
    #[error("类型不匹配: {0}")]
    TypeError(String),

    /// 除零错误
    #[error("除零错误")]
    DivisionByZero,
}

pub type FormulaResult<T> = Result<T, FormulaError>;
