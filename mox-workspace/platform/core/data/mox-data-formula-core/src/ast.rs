//! 公式抽象语法树（AST）定义

use serde::{Deserialize, Serialize};

/// 公式值类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum FormulaValue {
    /// 数字
    Number(f64),
    /// 字符串
    String(String),
    /// 布尔值
    Boolean(bool),
    /// 空值
    Null,
}

/// 二元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    /// 加法
    Add,
    /// 减法
    Sub,
    /// 乘法
    Mul,
    /// 除法
    Div,
    /// 取模
    Mod,
    /// 幂运算
    Pow,
    /// 等于
    Eq,
    /// 不等于
    Ne,
    /// 大于
    Gt,
    /// 大于等于
    Ge,
    /// 小于
    Lt,
    /// 小于等于
    Le,
    /// 逻辑与
    And,
    /// 逻辑或
    Or,
}

/// 一元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    /// 取负
    Neg,
    /// 逻辑非
    Not,
}

/// AST 节点
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    /// 字面量值
    Literal(FormulaValue),
    /// 变量引用
    Variable(String),
    /// 一元运算
    Unary(UnaryOp, Box<Expr>),
    /// 二元运算
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    /// 函数调用
    Call {
        /// 函数名
        name: String,
        /// 参数列表
        args: Vec<Expr>,
    },
    /// 条件表达式（三元运算）
    If {
        /// 条件
        condition: Box<Expr>,
        /// 真值分支
        then_branch: Box<Expr>,
        /// 假值分支
        else_branch: Box<Expr>,
    },
}
