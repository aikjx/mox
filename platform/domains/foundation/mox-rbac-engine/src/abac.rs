// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! ABAC 条件表达式求值器
//!
//! 支持基于属性的访问控制（Attribute-Based Access Control）。
//! 条件表达式语法类似简单的布尔表达式，支持：
//!
//! - 属性访问：`subject.department`、`resource.owner`、`env.time_hour`
//! - 比较操作：`==`、`!=`、`>`、`<`、`>=`、`<=`
//! - 逻辑操作：`&&`、`||`、`!`
//! - 字符串/数字/布尔字面量
//! - 括号分组
//! - `in` 操作符：`subject.role in ["admin", "editor"]`
//! - `contains` 操作符：`resource.tags contains "confidential"`
//!
//! # 示例
//!
//! ```text
//! subject.department == resource.owner_department
//! subject.age >= 18 && resource.classification != "top_secret"
//! !subject.is_guest && env.time_hour >= 9 && env.time_hour <= 18
//! subject.roles in ["admin", "safety_approver"]
//! ```

use crate::error::RbacError;
use crate::types::{AttributeValue, EvaluationContext};

// ── 求值器 ──────────────────────────────────────────────────────────────────

/// ABAC 条件求值器
pub struct ConditionEvaluator;

impl ConditionEvaluator {
    /// 求值条件表达式
    ///
    /// # Arguments
    /// * `expression` - 条件表达式字符串
    /// * `ctx` - 评估上下文（包含 subject/resource/environment 属性）
    ///
    /// # Returns
    /// * `Ok(true)` - 条件满足
    /// * `Ok(false)` - 条件不满足
    /// * `Err` - 表达式解析或求值错误
    pub fn evaluate(expression: &str, ctx: &EvaluationContext) -> Result<bool, RbacError> {
        let tokens = tokenize(expression)?;
        let mut parser = Parser::new(tokens, ctx);
        let result = parser.parse_expression()?;
        parser.expect_end()?;
        Ok(result)
    }

    /// 验证表达式语法是否正确（不求值属性）
    ///
    /// 仅检查语法结构（token 解析 + 运算符/括号匹配），
    /// 不检查属性是否存在。属性不存在属于运行时错误而非语法错误。
    pub fn validate(expression: &str) -> Result<(), RbacError> {
        let tokens = tokenize(expression)?;

        // 基础语法检查：括号匹配
        let mut paren_depth = 0;
        let mut bracket_depth = 0;
        for tok in &tokens {
            match tok {
                Token::LParen => paren_depth += 1,
                Token::RParen => {
                    paren_depth -= 1;
                    if paren_depth < 0 {
                        return Err(RbacError::ConditionParseError {
                            expression: expression.into(),
                            detail: "unexpected ')'".into(),
                        });
                    }
                }
                Token::LBracket => bracket_depth += 1,
                Token::RBracket => {
                    bracket_depth -= 1;
                    if bracket_depth < 0 {
                        return Err(RbacError::ConditionParseError {
                            expression: expression.into(),
                            detail: "unexpected ']'".into(),
                        });
                    }
                }
                _ => {}
            }
        }

        if paren_depth != 0 {
            return Err(RbacError::ConditionParseError {
                expression: expression.into(),
                detail: "unclosed parenthesis".into(),
            });
        }
        if bracket_depth != 0 {
            return Err(RbacError::ConditionParseError {
                expression: expression.into(),
                detail: "unclosed bracket".into(),
            });
        }

        // 检查 token 序列是否有明显的结构问题
        // （简化版：确保不以操作符开头或结尾）
        if tokens.is_empty() {
            return Err(RbacError::ConditionParseError {
                expression: expression.into(),
                detail: "empty expression".into(),
            });
        }

        // 第一个 token 不能是二元操作符
        match tokens.first() {
            Some(Token::Eq)
            | Some(Token::Neq)
            | Some(Token::Gt)
            | Some(Token::Lt)
            | Some(Token::Gte)
            | Some(Token::Lte)
            | Some(Token::And)
            | Some(Token::Or)
            | Some(Token::In)
            | Some(Token::Contains)
            | Some(Token::RParen)
            | Some(Token::RBracket)
            | Some(Token::Comma)
            | Some(Token::Dot) => {
                return Err(RbacError::ConditionParseError {
                    expression: expression.into(),
                    detail: "expression cannot start with operator".into(),
                });
            }
            _ => {}
        }

        // 最后一个 token 不能是二元/一元操作符
        match tokens.last() {
            Some(Token::Eq)
            | Some(Token::Neq)
            | Some(Token::Gt)
            | Some(Token::Lt)
            | Some(Token::Gte)
            | Some(Token::Lte)
            | Some(Token::And)
            | Some(Token::Or)
            | Some(Token::Not)
            | Some(Token::In)
            | Some(Token::Contains)
            | Some(Token::LParen)
            | Some(Token::LBracket)
            | Some(Token::Comma)
            | Some(Token::Dot) => {
                return Err(RbacError::ConditionParseError {
                    expression: expression.into(),
                    detail: "expression cannot end with operator".into(),
                });
            }
            _ => {}
        }

        Ok(())
    }
}

// ── Tokenizer ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    StringLit(String),
    IntLit(i64),
    BoolLit(bool),
    Eq,    // ==
    Neq,   // !=
    Gt,    // >
    Lt,    // <
    Gte,   // >=
    Lte,   // <=
    And,   // &&
    Or,    // ||
    Not,   // !
    LParen, // (
    RParen, // )
    LBracket, // [
    RBracket, // ]
    Comma, // ,
    Dot,   // .
    In,    // in
    Contains, // contains
}

fn tokenize(input: &str) -> Result<Vec<Token>, RbacError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // 跳过空白
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // 字符串字面量
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                    match chars[i] {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        '\\' => s.push('\\'),
                        '"' => s.push('"'),
                        '\'' => s.push('\''),
                        other => s.push(other),
                    }
                } else {
                    s.push(chars[i]);
                }
                i += 1;
            }
            if i >= chars.len() {
                return Err(RbacError::ConditionParseError {
                    expression: input.into(),
                    detail: "unterminated string literal".into(),
                });
            }
            i += 1; // 跳过结束引号
            tokens.push(Token::StringLit(s));
            continue;
        }

        // 数字字面量
        if c.is_ascii_digit() || (c == '-' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let mut num_str = String::new();
            if c == '-' {
                num_str.push(c);
                i += 1;
            }
            while i < chars.len() && chars[i].is_ascii_digit() {
                num_str.push(chars[i]);
                i += 1;
            }
            let num: i64 = num_str.parse().map_err(|_| RbacError::ConditionParseError {
                expression: input.into(),
                detail: format!("invalid number: {num_str}"),
            })?;
            tokens.push(Token::IntLit(num));
            continue;
        }

        // 标识符 / 关键字
        if c.is_ascii_alphabetic() || c == '_' {
            let mut ident = String::new();
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                ident.push(chars[i]);
                i += 1;
            }

            match ident.as_str() {
                "true" => tokens.push(Token::BoolLit(true)),
                "false" => tokens.push(Token::BoolLit(false)),
                "in" => tokens.push(Token::In),
                "contains" => tokens.push(Token::Contains),
                _ => tokens.push(Token::Ident(ident)),
            }
            continue;
        }

        // 多字符操作符
        match c {
            '=' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::Eq);
                    i += 2;
                } else {
                    return Err(RbacError::ConditionParseError {
                        expression: input.into(),
                        detail: format!("unexpected character '=' at position {i}"),
                    });
                }
            }
            '!' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::Neq);
                    i += 2;
                } else {
                    tokens.push(Token::Not);
                    i += 1;
                }
            }
            '>' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::Gte);
                    i += 2;
                } else {
                    tokens.push(Token::Gt);
                    i += 1;
                }
            }
            '<' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::Lte);
                    i += 2;
                } else {
                    tokens.push(Token::Lt);
                    i += 1;
                }
            }
            '&' => {
                if i + 1 < chars.len() && chars[i + 1] == '&' {
                    tokens.push(Token::And);
                    i += 2;
                } else {
                    return Err(RbacError::ConditionParseError {
                        expression: input.into(),
                        detail: format!("unexpected character '&' at position {i}"),
                    });
                }
            }
            '|' => {
                if i + 1 < chars.len() && chars[i + 1] == '|' {
                    tokens.push(Token::Or);
                    i += 2;
                } else {
                    return Err(RbacError::ConditionParseError {
                        expression: input.into(),
                        detail: format!("unexpected character '|' at position {i}"),
                    });
                }
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            '[' => {
                tokens.push(Token::LBracket);
                i += 1;
            }
            ']' => {
                tokens.push(Token::RBracket);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            '.' => {
                tokens.push(Token::Dot);
                i += 1;
            }
            _ => {
                return Err(RbacError::ConditionParseError {
                    expression: input.into(),
                    detail: format!("unexpected character '{}' at position {}", c, i),
                });
            }
        }
    }

    Ok(tokens)
}

// ── Parser / Evaluator ──────────────────────────────────────────────────────

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    ctx: &'a EvaluationContext,
}

impl<'a> Parser<'a> {
    fn new(tokens: Vec<Token>, ctx: &'a EvaluationContext) -> Self {
        Self { tokens, pos: 0, ctx }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos);
        self.pos += 1;
        tok
    }

    fn expect_end(&self) -> Result<(), RbacError> {
        if self.pos < self.tokens.len() {
            Err(RbacError::ConditionParseError {
                expression: "<expression>".into(),
                detail: format!(
                    "unexpected token {:?} at position {}",
                    self.tokens[self.pos], self.pos
                ),
            })
        } else {
            Ok(())
        }
    }

    // 表达式 = or_expr
    fn parse_expression(&mut self) -> Result<bool, RbacError> {
        self.parse_or()
    }

    // or_expr = and_expr ( "||" and_expr )*
    fn parse_or(&mut self) -> Result<bool, RbacError> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Some(Token::Or)) {
            self.next(); // consume ||
            let right = self.parse_and()?;
            left = left || right;
        }
        Ok(left)
    }

    // and_expr = not_expr ( "&&" not_expr )*
    fn parse_and(&mut self) -> Result<bool, RbacError> {
        let mut left = self.parse_not()?;
        while matches!(self.peek(), Some(Token::And)) {
            self.next(); // consume &&
            let right = self.parse_not()?;
            left = left && right;
        }
        Ok(left)
    }

    // not_expr = "!" not_expr | comparison
    fn parse_not(&mut self) -> Result<bool, RbacError> {
        if matches!(self.peek(), Some(Token::Not)) {
            self.next(); // consume !
            let val = self.parse_not()?;
            Ok(!val)
        } else {
            self.parse_comparison()
        }
    }

    // comparison = primary ( cmp_op primary | "in" list | "contains" primary )?
    fn parse_comparison(&mut self) -> Result<bool, RbacError> {
        // 检查是否是括号表达式
        if matches!(self.peek(), Some(Token::LParen)) {
            self.next(); // consume (
            let val = self.parse_expression()?;
            match self.next() {
                Some(Token::RParen) => {}
                _ => {
                    return Err(RbacError::ConditionParseError {
                        expression: "<expression>".into(),
                        detail: "expected ')'".into(),
                    });
                }
            }
            return Ok(val);
        }

        let left = self.parse_value()?;

        // 检查比较操作符
        match self.peek() {
            Some(Token::Eq) => {
                self.next();
                let right = self.parse_value()?;
                Ok(value_eq(&left, &right))
            }
            Some(Token::Neq) => {
                self.next();
                let right = self.parse_value()?;
                Ok(!value_eq(&left, &right))
            }
            Some(Token::Gt) => {
                self.next();
                let right = self.parse_value()?;
                value_cmp(&left, &right, |a, b| a > b)
            }
            Some(Token::Lt) => {
                self.next();
                let right = self.parse_value()?;
                value_cmp(&left, &right, |a, b| a < b)
            }
            Some(Token::Gte) => {
                self.next();
                let right = self.parse_value()?;
                value_cmp(&left, &right, |a, b| a >= b)
            }
            Some(Token::Lte) => {
                self.next();
                let right = self.parse_value()?;
                value_cmp(&left, &right, |a, b| a <= b)
            }
            Some(Token::In) => {
                self.next();
                let list = self.parse_list()?;
                Ok(list_contains(&list, &left))
            }
            Some(Token::Contains) => {
                self.next();
                let right = self.parse_value()?;
                Ok(value_contains(&left, &right))
            }
            _ => {
                // 没有比较操作符，如果是布尔值直接返回
                if let AttributeValue::Bool(b) = left {
                    Ok(b)
                } else {
                    Err(RbacError::ConditionParseError {
                        expression: "<expression>".into(),
                        detail: format!("expected comparison operator, got {:?}", self.peek()),
                    })
                }
            }
        }
    }

    // 值 = 属性路径 | 字面量
    fn parse_value(&mut self) -> Result<AttributeValue, RbacError> {
        match self.peek() {
            Some(Token::StringLit(s)) => {
                let s = s.clone();
                self.next();
                Ok(AttributeValue::String(s))
            }
            Some(Token::IntLit(n)) => {
                let n = *n;
                self.next();
                Ok(AttributeValue::Int(n))
            }
            Some(Token::BoolLit(b)) => {
                let b = *b;
                self.next();
                Ok(AttributeValue::Bool(b))
            }
            Some(Token::Ident(_)) => self.parse_attribute_path(),
            Some(Token::LBracket) => {
                let list = self.parse_list()?;
                Ok(AttributeValue::List(list))
            }
            other => Err(RbacError::ConditionParseError {
                expression: "<expression>".into(),
                detail: format!("unexpected token in value: {:?}", other),
            }),
        }
    }

    // 属性路径 = ident ( "." ident )*
    // 支持 subject.xxx, resource.xxx, env.xxx
    fn parse_attribute_path(&mut self) -> Result<AttributeValue, RbacError> {
        let mut path_parts = Vec::new();

        // 第一个标识符
        match self.next() {
            Some(Token::Ident(name)) => path_parts.push(name.clone()),
            other => {
                return Err(RbacError::ConditionParseError {
                    expression: "<expression>".into(),
                    detail: format!("expected identifier, got {:?}", other),
                });
            }
        }

        // 后续的 .ident
        while matches!(self.peek(), Some(Token::Dot)) {
            self.next(); // consume .
            match self.next() {
                Some(Token::Ident(name)) => path_parts.push(name.clone()),
                other => {
                    return Err(RbacError::ConditionParseError {
                        expression: "<expression>".into(),
                        detail: format!("expected identifier after '.', got {:?}", other),
                    });
                }
            }
        }

        // 解析属性路径
        resolve_attribute(&path_parts, self.ctx)
    }

    // 列表 = "[" value ( "," value )* "]"
    fn parse_list(&mut self) -> Result<Vec<String>, RbacError> {
        match self.next() {
            Some(Token::LBracket) => {}
            other => {
                return Err(RbacError::ConditionParseError {
                    expression: "<expression>".into(),
                    detail: format!("expected '[', got {:?}", other),
                });
            }
        }

        let mut items = Vec::new();

        // 空列表
        if matches!(self.peek(), Some(Token::RBracket)) {
            self.next();
            return Ok(items);
        }

        // 第一个元素
        items.push(self.parse_list_value()?);

        // 后续元素
        while matches!(self.peek(), Some(Token::Comma)) {
            self.next(); // consume ,
            items.push(self.parse_list_value()?);
        }

        match self.next() {
            Some(Token::RBracket) => {}
            other => {
                return Err(RbacError::ConditionParseError {
                    expression: "<expression>".into(),
                    detail: format!("expected ']', got {:?}", other),
                });
            }
        }

        Ok(items)
    }

    fn parse_list_value(&mut self) -> Result<String, RbacError> {
        match self.next() {
            Some(Token::StringLit(s)) => Ok(s.clone()),
            Some(Token::Ident(s)) => Ok(s.clone()),
            other => Err(RbacError::ConditionParseError {
                expression: "<expression>".into(),
                detail: format!("expected string in list, got {:?}", other),
            }),
        }
    }
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/// 解析属性路径并返回属性值
fn resolve_attribute(
    path: &[String],
    ctx: &EvaluationContext,
) -> Result<AttributeValue, RbacError> {
    if path.is_empty() {
        return Err(RbacError::ConditionEvalError {
            expression: "<expression>".into(),
            detail: "empty attribute path".into(),
        });
    }

    let namespace = path[0].as_str();
    let attr_name = path.get(1).ok_or_else(|| RbacError::ConditionEvalError {
        expression: "<expression>".into(),
        detail: format!("attribute path '{namespace}' missing property name"),
    })?;

    match namespace {
        "subject" => {
            // 内置属性：id
            if attr_name == "id" {
                return Ok(AttributeValue::String(ctx.subject.id.clone()));
            }
            // 内置属性：roles（列表）
            if attr_name == "roles" {
                return Ok(AttributeValue::List(ctx.subject.roles.clone()));
            }
            ctx.subject.attributes.get(attr_name.as_str()).cloned().ok_or_else(|| {
                RbacError::ConditionEvalError {
                    expression: "<expression>".into(),
                    detail: format!("attribute 'subject.{attr_name}' not found"),
                }
            })
        }
        "resource" => {
            // 内置属性：path
            if attr_name == "path" {
                return Ok(AttributeValue::String(ctx.resource.path.clone()));
            }
            // 内置属性：tenant
            if attr_name == "tenant" {
                return Ok(ctx
                    .resource
                    .tenant
                    .as_ref()
                    .map(|t| AttributeValue::String(t.clone()))
                    .unwrap_or(AttributeValue::String(String::new())));
            }
            ctx.resource.attributes.get(attr_name.as_str()).cloned().ok_or_else(|| {
                RbacError::ConditionEvalError {
                    expression: "<expression>".into(),
                    detail: format!("attribute 'resource.{attr_name}' not found"),
                }
            })
        }
        "env" | "environment" => {
            ctx.environment.get(attr_name.as_str()).cloned().ok_or_else(|| {
                RbacError::ConditionEvalError {
                    expression: "<expression>".into(),
                    detail: format!("attribute 'env.{attr_name}' not found"),
                }
            })
        }
        other => Err(RbacError::ConditionEvalError {
            expression: "<expression>".into(),
            detail: format!("unknown attribute namespace: '{other}'"),
        }),
    }
}

/// 值相等比较
fn value_eq(a: &AttributeValue, b: &AttributeValue) -> bool {
    match (a, b) {
        (AttributeValue::String(x), AttributeValue::String(y)) => x == y,
        (AttributeValue::Int(x), AttributeValue::Int(y)) => x == y,
        (AttributeValue::Bool(x), AttributeValue::Bool(y)) => x == y,
        (AttributeValue::String(x), AttributeValue::Int(y)) => x.parse::<i64>().ok() == Some(*y),
        (AttributeValue::Int(x), AttributeValue::String(y)) => y.parse::<i64>().ok() == Some(*x),
        _ => false,
    }
}

/// 值比较（数值）
fn value_cmp<F>(a: &AttributeValue, b: &AttributeValue, cmp: F) -> Result<bool, RbacError>
where
    F: Fn(i64, i64) -> bool,
{
    let ai = to_int(a).ok_or_else(|| RbacError::ConditionEvalError {
        expression: "<expression>".into(),
        detail: format!("cannot compare non-numeric value: {:?}", a),
    })?;
    let bi = to_int(b).ok_or_else(|| RbacError::ConditionEvalError {
        expression: "<expression>".into(),
        detail: format!("cannot compare non-numeric value: {:?}", b),
    })?;
    Ok(cmp(ai, bi))
}

/// 尝试将属性值转为整数
fn to_int(v: &AttributeValue) -> Option<i64> {
    match v {
        AttributeValue::Int(i) => Some(*i),
        AttributeValue::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// 列表包含检查
fn list_contains(list: &[String], value: &AttributeValue) -> bool {
    match value {
        AttributeValue::String(s) => list.iter().any(|item| item == s),
        AttributeValue::Int(i) => list.iter().any(|item| item.parse::<i64>().ok() == Some(*i)),
        _ => false,
    }
}

/// contains 操作符：检查左值是否包含右值
fn value_contains(container: &AttributeValue, element: &AttributeValue) -> bool {
    match (container, element) {
        (AttributeValue::List(list), AttributeValue::String(s)) => {
            list.iter().any(|item| item == s)
        }
        (AttributeValue::String(s), AttributeValue::String(sub)) => s.contains(sub),
        _ => false,
    }
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Action, EvaluationContext, Resource, Subject};

    fn make_ctx() -> EvaluationContext {
        let subject = Subject::new("user:alice", vec!["editor".into()])
            .with_attr("department", "engineering")
            .with_attr("age", 30i64)
            .with_attr("is_guest", false)
            .with_attr("roles_list", vec!["editor".into(), "viewer".into()]);

        let resource = Resource::new("db:prod/citizen")
            .with_attr("owner_department", "engineering")
            .with_attr("classification", "confidential")
            .with_attr("tags", vec!["confidential".into(), "pii".into()]);

        EvaluationContext::new(subject, resource, Action::Read)
            .with_env("time_hour", 14i64)
            .with_env("is_workday", true)
    }

    #[test]
    fn eval_equality_string() {
        let ctx = make_ctx();
        assert!(ConditionEvaluator::evaluate(
            "subject.department == resource.owner_department",
            &ctx
        )
        .unwrap());
        assert!(!ConditionEvaluator::evaluate(
            "subject.department == \"sales\"",
            &ctx
        )
        .unwrap());
    }

    #[test]
    fn eval_inequality() {
        let ctx = make_ctx();
        assert!(ConditionEvaluator::evaluate(
            "resource.classification != \"public\"",
            &ctx
        )
        .unwrap());
    }

    #[test]
    fn eval_numeric_comparisons() {
        let ctx = make_ctx();
        assert!(ConditionEvaluator::evaluate("subject.age >= 18", &ctx).unwrap());
        assert!(ConditionEvaluator::evaluate("subject.age < 40", &ctx).unwrap());
        assert!(ConditionEvaluator::evaluate("subject.age == 30", &ctx).unwrap());
        assert!(!ConditionEvaluator::evaluate("subject.age > 50", &ctx).unwrap());
        assert!(ConditionEvaluator::evaluate("env.time_hour <= 18", &ctx).unwrap());
    }

    #[test]
    fn eval_logical_and() {
        let ctx = make_ctx();
        assert!(
            ConditionEvaluator::evaluate("subject.age >= 18 && env.time_hour <= 18", &ctx)
                .unwrap()
        );
        assert!(!ConditionEvaluator::evaluate(
            "subject.age >= 50 && env.time_hour <= 18",
            &ctx
        )
        .unwrap());
    }

    #[test]
    fn eval_logical_or() {
        let ctx = make_ctx();
        assert!(ConditionEvaluator::evaluate(
            "subject.age >= 50 || env.time_hour <= 18",
            &ctx
        )
        .unwrap());
        assert!(!ConditionEvaluator::evaluate(
            "subject.age >= 50 || env.time_hour >= 20",
            &ctx
        )
        .unwrap());
    }

    #[test]
    fn eval_logical_not() {
        let ctx = make_ctx();
        // subject.is_guest = false, so !false = true
        assert!(ConditionEvaluator::evaluate("!subject.is_guest", &ctx).unwrap());
        assert!(ConditionEvaluator::evaluate("!subject.is_guest == true", &ctx).unwrap());
        // double negation
        assert!(!ConditionEvaluator::evaluate("!!false", &ctx).unwrap());
    }

    #[test]
    fn eval_parentheses() {
        let ctx = make_ctx();
        assert!(ConditionEvaluator::evaluate(
            "(subject.age >= 18 && env.time_hour <= 18) || subject.is_guest == false",
            &ctx
        )
        .unwrap());
    }

    #[test]
    fn eval_in_operator() {
        let ctx = make_ctx();
        assert!(ConditionEvaluator::evaluate(
            "subject.department in [\"engineering\", \"sales\"]",
            &ctx
        )
        .unwrap());
        assert!(!ConditionEvaluator::evaluate(
            "subject.department in [\"hr\", \"finance\"]",
            &ctx
        )
        .unwrap());
    }

    #[test]
    fn eval_contains_operator_list() {
        let ctx = make_ctx();
        assert!(
            ConditionEvaluator::evaluate("resource.tags contains \"pii\"", &ctx).unwrap()
        );
        assert!(!ConditionEvaluator::evaluate(
            "resource.tags contains \"public\"",
            &ctx
        )
        .unwrap());
    }

    #[test]
    fn eval_contains_operator_string() {
        let ctx = make_ctx();
        // 字符串 contains
        assert!(
            ConditionEvaluator::evaluate("resource.classification contains \"fident\"", &ctx)
                .unwrap()
        );
    }

    #[test]
    fn eval_bool_literal() {
        let ctx = make_ctx();
        assert!(ConditionEvaluator::evaluate("true", &ctx).unwrap());
        assert!(!ConditionEvaluator::evaluate("false", &ctx).unwrap());
    }

    #[test]
    fn eval_complex_expression() {
        let ctx = make_ctx();
        let expr = "subject.department == resource.owner_department && subject.age >= 18 && env.time_hour >= 9 && env.time_hour <= 18";
        assert!(ConditionEvaluator::evaluate(expr, &ctx).unwrap());
    }

    #[test]
    fn validate_valid_expression() {
        assert!(ConditionEvaluator::validate("subject.age == resource.level").is_ok());
        assert!(ConditionEvaluator::validate("env.x > 5 && env.y < 10").is_ok());
        assert!(ConditionEvaluator::validate("!(subject.active == true)").is_ok());
        assert!(ConditionEvaluator::validate("subject.role in [\"a\", \"b\"]").is_ok());
        assert!(ConditionEvaluator::validate("resource.tags contains \"vip\"").is_ok());
    }

    #[test]
    fn validate_invalid_expression() {
        assert!(ConditionEvaluator::validate("a.b ==").is_err());
        assert!(ConditionEvaluator::validate("a & b").is_err()); // 单个 &
        assert!(ConditionEvaluator::validate("(a == b").is_err()); // 缺少右括号
    }

    #[test]
    fn eval_unknown_attribute_error() {
        let ctx = make_ctx();
        let result = ConditionEvaluator::evaluate("subject.nonexistent == \"x\"", &ctx);
        assert!(result.is_err());
        match result.unwrap_err() {
            RbacError::ConditionEvalError { .. } => {}
            _ => panic!("expected ConditionEvalError"),
        }
    }

    #[test]
    fn eval_unknown_namespace_error() {
        let ctx = make_ctx();
        let result = ConditionEvaluator::evaluate("unknown.field == \"x\"", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn eval_negative_number() {
        let ctx = make_ctx();
        let result = ConditionEvaluator::evaluate("subject.age > -1", &ctx);
        assert!(result.unwrap());
    }

    #[test]
    fn eval_empty_list() {
        let ctx = make_ctx();
        assert!(!ConditionEvaluator::evaluate(
            "subject.department in []",
            &ctx
        )
        .unwrap());
    }

    #[test]
    fn eval_single_item_list() {
        let ctx = make_ctx();
        assert!(ConditionEvaluator::evaluate(
            "subject.department in [\"engineering\"]",
            &ctx
        )
        .unwrap());
    }

    #[test]
    fn eval_operator_precedence() {
        // && 优先级高于 ||
        let ctx = make_ctx();
        // false || true && true = true
        let expr = "subject.age > 100 || subject.age < 50 && env.time_hour > 0";
        assert!(ConditionEvaluator::evaluate(expr, &ctx).unwrap());
    }

    #[test]
    fn tokenizer_handles_strings_with_escapes() {
        let tokens = tokenize("\"hello\\nworld\"").unwrap();
        assert_eq!(tokens.len(), 1);
        match &tokens[0] {
            Token::StringLit(s) => assert_eq!(s, "hello\nworld"),
            _ => panic!("expected string literal"),
        }
    }
}
