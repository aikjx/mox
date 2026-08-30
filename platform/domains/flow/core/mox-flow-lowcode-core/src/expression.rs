// Copyright (c) 2026 璇玑 RelGraph · 低代码核心 (Low-Code Core)
// Licensed under the MIT License.

//! 表达式求值器
//!
//! 支持简单的表达式语言，用于：
//! - 字段默认值计算
//! - 验证规则条件
//! - 字段联动逻辑
//! - 条件显示/隐藏
//!
//! 支持的语法：
//! - 字面量：字符串、数字、布尔、null
//! - 变量引用：field_name, object.field
//! - 算术运算：+ - * / %
//! - 比较运算：== != > >= < <=
//! - 逻辑运算：&& || !
//! - 三元表达式：condition ? a : b
//! - 字符串拼接
//! - 内置函数

use std::collections::HashMap;

use crate::error::{LowcodeError, LowcodeResult};
use crate::types::DataType;

/// 表达式求值器
pub struct ExpressionEvaluator;

impl ExpressionEvaluator {
    /// 求值表达式
    pub fn evaluate(expr: &str, context: &HashMap<String, DataType>) -> LowcodeResult<DataType> {
        let tokens = tokenize(expr)?;
        let (result, _) = parse_expression(&tokens, 0, context)?;
        Ok(result)
    }

    /// 求值为布尔值
    pub fn evaluate_bool(expr: &str, context: &HashMap<String, DataType>) -> LowcodeResult<bool> {
        let result = Self::evaluate(expr, context)?;
        Ok(is_truthy(&result))
    }

    /// 求值为字符串
    pub fn evaluate_string(
        expr: &str,
        context: &HashMap<String, DataType>,
    ) -> LowcodeResult<String> {
        let result = Self::evaluate(expr, context)?;
        Ok(to_string(&result))
    }
}

/// 判断真值
fn is_truthy(value: &DataType) -> bool {
    match value {
        DataType::Boolean(b) => *b,
        DataType::String(s) => !s.is_empty(),
        DataType::Integer(i) => *i != 0,
        DataType::Float(f) => *f != 0.0,
        DataType::Null => false,
        DataType::Array(arr) => !arr.is_empty(),
        DataType::Object(_) => true,
    }
}

/// 转换为字符串
fn to_string(value: &DataType) -> String {
    match value {
        DataType::String(s) => s.clone(),
        DataType::Integer(i) => i.to_string(),
        DataType::Float(f) => {
            if f.fract() == 0.0 {
                (*f as i64).to_string()
            } else {
                f.to_string()
            }
        }
        DataType::Boolean(b) => b.to_string(),
        DataType::Null => "".to_string(),
        DataType::Array(_) => "[Array]".to_string(),
        DataType::Object(_) => "[Object]".to_string(),
    }
}

/// Token 类型
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Str(String),
    Ident(String),
    True,
    False,
    Null,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    And,
    Or,
    Not,
    Question,
    Colon,
    LParen,
    RParen,
    Dot,
    Comma,
}

/// 词法分析
fn tokenize(expr: &str) -> LowcodeResult<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // 跳过空白
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // 字符串
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
                return Err(LowcodeError::ExpressionError(
                    "unterminated string".to_string(),
                ));
            }
            tokens.push(Token::Str(s));
            i += 1;
            continue;
        }

        // 数字
        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let mut num_str = String::new();
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                num_str.push(chars[i]);
                i += 1;
            }
            let num: f64 = num_str
                .parse()
                .map_err(|_| LowcodeError::ExpressionError(format!("invalid number: {}", num_str)))?;
            tokens.push(Token::Number(num));
            continue;
        }

        // 标识符/关键字
        if c.is_alphabetic() || c == '_' {
            let mut ident = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                ident.push(chars[i]);
                i += 1;
            }
            match ident.as_str() {
                "true" => tokens.push(Token::True),
                "false" => tokens.push(Token::False),
                "null" => tokens.push(Token::Null),
                _ => tokens.push(Token::Ident(ident)),
            }
            continue;
        }

        // 操作符
        match c {
            '+' => tokens.push(Token::Plus),
            '-' => tokens.push(Token::Minus),
            '*' => tokens.push(Token::Star),
            '/' => tokens.push(Token::Slash),
            '%' => tokens.push(Token::Percent),
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            '.' => tokens.push(Token::Dot),
            ',' => tokens.push(Token::Comma),
            '?' => tokens.push(Token::Question),
            ':' => tokens.push(Token::Colon),
            '!' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::Ne);
                    i += 1;
                } else {
                    tokens.push(Token::Not);
                }
            }
            '=' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::Eq);
                    i += 1;
                } else {
                    return Err(LowcodeError::ExpressionError(
                        "unexpected '='".to_string(),
                    ));
                }
            }
            '>' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::Ge);
                    i += 1;
                } else {
                    tokens.push(Token::Gt);
                }
            }
            '<' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::Le);
                    i += 1;
                } else {
                    tokens.push(Token::Lt);
                }
            }
            '&' => {
                if i + 1 < chars.len() && chars[i + 1] == '&' {
                    tokens.push(Token::And);
                    i += 1;
                } else {
                    return Err(LowcodeError::ExpressionError(
                        "unexpected '&'".to_string(),
                    ));
                }
            }
            '|' => {
                if i + 1 < chars.len() && chars[i + 1] == '|' {
                    tokens.push(Token::Or);
                    i += 1;
                } else {
                    return Err(LowcodeError::ExpressionError(
                        "unexpected '|'".to_string(),
                    ));
                }
            }
            _ => {
                return Err(LowcodeError::ExpressionError(format!(
                    "unexpected character: {}",
                    c
                )));
            }
        }
        i += 1;
    }

    Ok(tokens)
}

/// 解析表达式（三元表达式优先级最低）
fn parse_expression(
    tokens: &[Token],
    pos: usize,
    context: &HashMap<String, DataType>,
) -> LowcodeResult<(DataType, usize)> {
    let (left, mut pos) = parse_or(tokens, pos, context)?;

    // 三元表达式
    if pos < tokens.len() && tokens[pos] == Token::Question {
        pos += 1;
        let (true_val, new_pos) = parse_expression(tokens, pos, context)?;
        pos = new_pos;
        if pos >= tokens.len() || tokens[pos] != Token::Colon {
            return Err(LowcodeError::ExpressionError(
                "expected ':' in ternary expression".to_string(),
            ));
        }
        pos += 1;
        let (false_val, new_pos) = parse_expression(tokens, pos, context)?;
        pos = new_pos;

        let result = if is_truthy(&left) { true_val } else { false_val };
        Ok((result, pos))
    } else {
        Ok((left, pos))
    }
}

fn parse_or(
    tokens: &[Token],
    pos: usize,
    context: &HashMap<String, DataType>,
) -> LowcodeResult<(DataType, usize)> {
    let (mut left, mut pos) = parse_and(tokens, pos, context)?;

    while pos < tokens.len() && tokens[pos] == Token::Or {
        pos += 1;
        let (right, new_pos) = parse_and(tokens, pos, context)?;
        pos = new_pos;
        left = DataType::Boolean(is_truthy(&left) || is_truthy(&right));
    }

    Ok((left, pos))
}

fn parse_and(
    tokens: &[Token],
    pos: usize,
    context: &HashMap<String, DataType>,
) -> LowcodeResult<(DataType, usize)> {
    let (mut left, mut pos) = parse_equality(tokens, pos, context)?;

    while pos < tokens.len() && tokens[pos] == Token::And {
        pos += 1;
        let (right, new_pos) = parse_equality(tokens, pos, context)?;
        pos = new_pos;
        left = DataType::Boolean(is_truthy(&left) && is_truthy(&right));
    }

    Ok((left, pos))
}

fn parse_equality(
    tokens: &[Token],
    pos: usize,
    context: &HashMap<String, DataType>,
) -> LowcodeResult<(DataType, usize)> {
    let (left, mut pos) = parse_comparison(tokens, pos, context)?;

    if pos < tokens.len() {
        match &tokens[pos] {
            Token::Eq => {
                pos += 1;
                let (right, new_pos) = parse_comparison(tokens, pos, context)?;
                pos = new_pos;
                Ok((DataType::Boolean(values_equal(&left, &right)), pos))
            }
            Token::Ne => {
                pos += 1;
                let (right, new_pos) = parse_comparison(tokens, pos, context)?;
                pos = new_pos;
                Ok((DataType::Boolean(!values_equal(&left, &right)), pos))
            }
            _ => Ok((left, pos)),
        }
    } else {
        Ok((left, pos))
    }
}

fn parse_comparison(
    tokens: &[Token],
    pos: usize,
    context: &HashMap<String, DataType>,
) -> LowcodeResult<(DataType, usize)> {
    let (left, mut pos) = parse_additive(tokens, pos, context)?;

    if pos < tokens.len() {
        let op = match &tokens[pos] {
            Token::Gt => Some(Token::Gt),
            Token::Ge => Some(Token::Ge),
            Token::Lt => Some(Token::Lt),
            Token::Le => Some(Token::Le),
            _ => None,
        };

        if let Some(op) = op {
            pos += 1;
            let (right, new_pos) = parse_additive(tokens, pos, context)?;
            pos = new_pos;
            let result = compare_values(&left, &right, &op)?;
            return Ok((DataType::Boolean(result), pos));
        }
    }

    Ok((left, pos))
}

fn parse_additive(
    tokens: &[Token],
    pos: usize,
    context: &HashMap<String, DataType>,
) -> LowcodeResult<(DataType, usize)> {
    let (mut left, mut pos) = parse_multiplicative(tokens, pos, context)?;

    while pos < tokens.len() {
        match &tokens[pos] {
            Token::Plus => {
                pos += 1;
                let (right, new_pos) = parse_multiplicative(tokens, pos, context)?;
                pos = new_pos;
                left = add_values(&left, &right)?;
            }
            Token::Minus => {
                pos += 1;
                let (right, new_pos) = parse_multiplicative(tokens, pos, context)?;
                pos = new_pos;
                left = subtract_values(&left, &right)?;
            }
            _ => break,
        }
    }

    Ok((left, pos))
}

fn parse_multiplicative(
    tokens: &[Token],
    pos: usize,
    context: &HashMap<String, DataType>,
) -> LowcodeResult<(DataType, usize)> {
    let (mut left, mut pos) = parse_unary(tokens, pos, context)?;

    while pos < tokens.len() {
        match &tokens[pos] {
            Token::Star => {
                pos += 1;
                let (right, new_pos) = parse_unary(tokens, pos, context)?;
                pos = new_pos;
                left = multiply_values(&left, &right)?;
            }
            Token::Slash => {
                pos += 1;
                let (right, new_pos) = parse_unary(tokens, pos, context)?;
                pos = new_pos;
                left = divide_values(&left, &right)?;
            }
            Token::Percent => {
                pos += 1;
                let (right, new_pos) = parse_unary(tokens, pos, context)?;
                pos = new_pos;
                left = modulo_values(&left, &right)?;
            }
            _ => break,
        }
    }

    Ok((left, pos))
}

fn parse_unary(
    tokens: &[Token],
    pos: usize,
    context: &HashMap<String, DataType>,
) -> LowcodeResult<(DataType, usize)> {
    if pos >= tokens.len() {
        return Err(LowcodeError::ExpressionError(
            "unexpected end of expression".to_string(),
        ));
    }

    match &tokens[pos] {
        Token::Minus => {
            let (val, new_pos) = parse_unary(tokens, pos + 1, context)?;
            match val {
                DataType::Integer(i) => Ok((DataType::Integer(-i), new_pos)),
                DataType::Float(f) => Ok((DataType::Float(-f), new_pos)),
                _ => Err(LowcodeError::ExpressionError(
                    "cannot negate non-numeric value".to_string(),
                )),
            }
        }
        Token::Not => {
            let (val, new_pos) = parse_unary(tokens, pos + 1, context)?;
            Ok((DataType::Boolean(!is_truthy(&val)), new_pos))
        }
        _ => parse_primary(tokens, pos, context),
    }
}

fn parse_primary(
    tokens: &[Token],
    pos: usize,
    context: &HashMap<String, DataType>,
) -> LowcodeResult<(DataType, usize)> {
    if pos >= tokens.len() {
        return Err(LowcodeError::ExpressionError(
            "unexpected end of expression".to_string(),
        ));
    }

    match &tokens[pos] {
        Token::Number(n) => {
            if n.fract() == 0.0 && *n <= i64::MAX as f64 && *n >= i64::MIN as f64 {
                Ok((DataType::Integer(*n as i64), pos + 1))
            } else {
                Ok((DataType::Float(*n), pos + 1))
            }
        }
        Token::Str(s) => Ok((DataType::String(s.clone()), pos + 1)),
        Token::True => Ok((DataType::Boolean(true), pos + 1)),
        Token::False => Ok((DataType::Boolean(false), pos + 1)),
        Token::Null => Ok((DataType::Null, pos + 1)),
        Token::LParen => {
            let (val, new_pos) = parse_expression(tokens, pos + 1, context)?;
            if new_pos >= tokens.len() || tokens[new_pos] != Token::RParen {
                return Err(LowcodeError::ExpressionError(
                    "expected ')'".to_string(),
                ));
            }
            Ok((val, new_pos + 1))
        }
        Token::Ident(name) => {
            // 检查是否是函数调用
            let mut next_pos = pos + 1;
            if next_pos < tokens.len() && tokens[next_pos] == Token::LParen {
                // 函数调用
                next_pos += 1;
                let mut args = Vec::new();
                if next_pos < tokens.len() && tokens[next_pos] != Token::RParen {
                    let (arg, new_pos) = parse_expression(tokens, next_pos, context)?;
                    args.push(arg);
                    next_pos = new_pos;
                    while next_pos < tokens.len() && tokens[next_pos] == Token::Comma {
                        next_pos += 1;
                        let (arg, new_pos) = parse_expression(tokens, next_pos, context)?;
                        args.push(arg);
                        next_pos = new_pos;
                    }
                }
                if next_pos >= tokens.len() || tokens[next_pos] != Token::RParen {
                    return Err(LowcodeError::ExpressionError(
                        "expected ')' in function call".to_string(),
                    ));
                }
                next_pos += 1;

                let result = call_builtin_function(name, &args)?;
                return Ok((result, next_pos));
            }

            // 普通变量
            let value = context
                .get(name)
                .cloned()
                .unwrap_or(DataType::Null);

            // 支持点访问：a.b.c
            let mut current = value;
            let mut pos = pos + 1;
            while pos < tokens.len() && tokens[pos] == Token::Dot {
                pos += 1;
                if pos >= tokens.len() {
                    return Err(LowcodeError::ExpressionError(
                        "expected identifier after '.'".to_string(),
                    ));
                }
                match &tokens[pos] {
                    Token::Ident(prop) => {
                        current = match &current {
                            DataType::Object(obj) => {
                                obj.get(prop).cloned().unwrap_or(DataType::Null)
                            }
                            _ => DataType::Null,
                        };
                        pos += 1;
                    }
                    _ => {
                        return Err(LowcodeError::ExpressionError(
                            "expected identifier after '.'".to_string(),
                        ));
                    }
                }
            }

            Ok((current, pos))
        }
        _ => Err(LowcodeError::ExpressionError(format!(
            "unexpected token at position {}",
            pos
        ))),
    }
}

// ---------- 值操作 ----------

fn values_equal(a: &DataType, b: &DataType) -> bool {
    match (a, b) {
        (DataType::Null, DataType::Null) => true,
        (DataType::Boolean(x), DataType::Boolean(y)) => x == y,
        (DataType::String(x), DataType::String(y)) => x == y,
        (DataType::Integer(x), DataType::Integer(y)) => x == y,
        (DataType::Float(x), DataType::Float(y)) => x == y,
        (DataType::Integer(x), DataType::Float(y)) => *x as f64 == *y,
        (DataType::Float(x), DataType::Integer(y)) => *x == *y as f64,
        _ => false,
    }
}

fn compare_values(a: &DataType, b: &DataType, op: &Token) -> LowcodeResult<bool> {
    let a_num = a.as_float().ok_or_else(|| {
        LowcodeError::ExpressionError("cannot compare non-numeric values".to_string())
    })?;
    let b_num = b.as_float().ok_or_else(|| {
        LowcodeError::ExpressionError("cannot compare non-numeric values".to_string())
    })?;

    Ok(match op {
        Token::Gt => a_num > b_num,
        Token::Ge => a_num >= b_num,
        Token::Lt => a_num < b_num,
        Token::Le => a_num <= b_num,
        _ => false,
    })
}

fn add_values(a: &DataType, b: &DataType) -> LowcodeResult<DataType> {
    match (a, b) {
        (DataType::String(s), _) => Ok(DataType::String(format!("{}{}", s, to_string(b)))),
        (_, DataType::String(s)) => Ok(DataType::String(format!("{}{}", to_string(a), s))),
        (DataType::Integer(x), DataType::Integer(y)) => Ok(DataType::Integer(x + y)),
        (DataType::Float(x), DataType::Float(y)) => Ok(DataType::Float(x + y)),
        (DataType::Integer(x), DataType::Float(y)) => Ok(DataType::Float(*x as f64 + y)),
        (DataType::Float(x), DataType::Integer(y)) => Ok(DataType::Float(x + *y as f64)),
        _ => Err(LowcodeError::ExpressionError(
            "cannot add these types".to_string(),
        )),
    }
}

fn subtract_values(a: &DataType, b: &DataType) -> LowcodeResult<DataType> {
    match (a, b) {
        (DataType::Integer(x), DataType::Integer(y)) => Ok(DataType::Integer(x - y)),
        _ => {
            let a = a.as_float().ok_or_else(|| {
                LowcodeError::ExpressionError("cannot subtract non-numeric values".to_string())
            })?;
            let b = b.as_float().ok_or_else(|| {
                LowcodeError::ExpressionError("cannot subtract non-numeric values".to_string())
            })?;
            Ok(DataType::Float(a - b))
        }
    }
}

fn multiply_values(a: &DataType, b: &DataType) -> LowcodeResult<DataType> {
    match (a, b) {
        (DataType::Integer(x), DataType::Integer(y)) => Ok(DataType::Integer(x * y)),
        _ => {
            let a = a.as_float().ok_or_else(|| {
                LowcodeError::ExpressionError("cannot multiply non-numeric values".to_string())
            })?;
            let b = b.as_float().ok_or_else(|| {
                LowcodeError::ExpressionError("cannot multiply non-numeric values".to_string())
            })?;
            Ok(DataType::Float(a * b))
        }
    }
}

fn divide_values(a: &DataType, b: &DataType) -> LowcodeResult<DataType> {
    let a = a.as_float().ok_or_else(|| {
        LowcodeError::ExpressionError("cannot divide non-numeric values".to_string())
    })?;
    let b = b.as_float().ok_or_else(|| {
        LowcodeError::ExpressionError("cannot divide non-numeric values".to_string())
    })?;
    if b == 0.0 {
        return Err(LowcodeError::ExpressionError(
            "division by zero".to_string(),
        ));
    }
    Ok(DataType::Float(a / b))
}

fn modulo_values(a: &DataType, b: &DataType) -> LowcodeResult<DataType> {
    let a = a.as_integer().ok_or_else(|| {
        LowcodeError::ExpressionError("modulo requires integer values".to_string())
    })?;
    let b = b.as_integer().ok_or_else(|| {
        LowcodeError::ExpressionError("modulo requires integer values".to_string())
    })?;
    if b == 0 {
        return Err(LowcodeError::ExpressionError(
            "modulo by zero".to_string(),
        ));
    }
    Ok(DataType::Integer(a % b))
}

// ---------- 内置函数 ----------

fn call_builtin_function(name: &str, args: &[DataType]) -> LowcodeResult<DataType> {
    match name {
        "len" | "length" => {
            if args.is_empty() {
                return Err(LowcodeError::ExpressionError(
                    "len() requires 1 argument".to_string(),
                ));
            }
            match &args[0] {
                DataType::String(s) => Ok(DataType::Integer(s.len() as i64)),
                DataType::Array(arr) => Ok(DataType::Integer(arr.len() as i64)),
                _ => Ok(DataType::Integer(0)),
            }
        }
        "lower" => {
            let s = args.first().map(to_string).unwrap_or_default();
            Ok(DataType::String(s.to_lowercase()))
        }
        "upper" => {
            let s = args.first().map(to_string).unwrap_or_default();
            Ok(DataType::String(s.to_uppercase()))
        }
        "trim" => {
            let s = args.first().map(to_string).unwrap_or_default();
            Ok(DataType::String(s.trim().to_string()))
        }
        "substring" => {
            if args.len() < 2 {
                return Err(LowcodeError::ExpressionError(
                    "substring() requires at least 2 arguments".to_string(),
                ));
            }
            let s = to_string(&args[0]);
            let start = args[1].as_integer().unwrap_or(0) as usize;
            let end = args
                .get(2)
                .and_then(|a| a.as_integer())
                .unwrap_or(s.len() as i64) as usize;
            let result = s
                .get(start..end.min(s.len()))
                .unwrap_or("")
                .to_string();
            Ok(DataType::String(result))
        }
        "concat" => {
            let result: String = args.iter().map(to_string).collect();
            Ok(DataType::String(result))
        }
        "isEmpty" | "empty" => {
            let empty = match args.first() {
                Some(DataType::String(s)) => s.is_empty(),
                Some(DataType::Array(arr)) => arr.is_empty(),
                Some(DataType::Null) => true,
                _ => false,
            };
            Ok(DataType::Boolean(empty))
        }
        "isNumber" => Ok(DataType::Boolean(
            args.first()
                .map(|v| v.as_float().is_some())
                .unwrap_or(false),
        )),
        "round" => {
            let n = args.first().and_then(|v| v.as_float()).unwrap_or(0.0);
            Ok(DataType::Integer(n.round() as i64))
        }
        "floor" => {
            let n = args.first().and_then(|v| v.as_float()).unwrap_or(0.0);
            Ok(DataType::Integer(n.floor() as i64))
        }
        "ceil" => {
            let n = args.first().and_then(|v| v.as_float()).unwrap_or(0.0);
            Ok(DataType::Integer(n.ceil() as i64))
        }
        "abs" => {
            let n = args.first().and_then(|v| v.as_float()).unwrap_or(0.0);
            Ok(DataType::Float(n.abs()))
        }
        "if" => {
            if args.len() < 3 {
                return Err(LowcodeError::ExpressionError(
                    "if() requires 3 arguments".to_string(),
                ));
            }
            if is_truthy(&args[0]) {
                Ok(args[1].clone())
            } else {
                Ok(args[2].clone())
            }
        }
        _ => Err(LowcodeError::ExpressionError(format!(
            "unknown function: {}",
            name
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> HashMap<String, DataType> {
        let mut c = HashMap::new();
        c.insert("name".to_string(), DataType::String("Alice".to_string()));
        c.insert("age".to_string(), DataType::Integer(25));
        c.insert("score".to_string(), DataType::Float(95.5));
        c.insert("active".to_string(), DataType::Boolean(true));
        c.insert("count".to_string(), DataType::Integer(10));
        c
    }

    #[test]
    fn test_literals() {
        let c = HashMap::new();
        assert_eq!(
            ExpressionEvaluator::evaluate("42", &c).unwrap(),
            DataType::Integer(42)
        );
        assert_eq!(
            ExpressionEvaluator::evaluate("3.14", &c).unwrap(),
            DataType::Float(3.14)
        );
        assert_eq!(
            ExpressionEvaluator::evaluate("\"hello\"", &c).unwrap(),
            DataType::String("hello".to_string())
        );
        assert_eq!(
            ExpressionEvaluator::evaluate("true", &c).unwrap(),
            DataType::Boolean(true)
        );
        assert_eq!(
            ExpressionEvaluator::evaluate("null", &c).unwrap(),
            DataType::Null
        );
    }

    #[test]
    fn test_variables() {
        let c = ctx();
        assert_eq!(
            ExpressionEvaluator::evaluate("name", &c).unwrap(),
            DataType::String("Alice".to_string())
        );
        assert_eq!(
            ExpressionEvaluator::evaluate("age", &c).unwrap(),
            DataType::Integer(25)
        );
        assert_eq!(
            ExpressionEvaluator::evaluate("nonexist", &c).unwrap(),
            DataType::Null
        );
    }

    #[test]
    fn test_arithmetic() {
        let c = ctx();
        assert_eq!(
            ExpressionEvaluator::evaluate("age + 5", &c).unwrap(),
            DataType::Integer(30)
        );
        assert_eq!(
            ExpressionEvaluator::evaluate("age * 2", &c).unwrap(),
            DataType::Integer(50)
        );
        assert_eq!(
            ExpressionEvaluator::evaluate("count - 3", &c).unwrap(),
            DataType::Integer(7)
        );
        assert_eq!(
            ExpressionEvaluator::evaluate("count / 2", &c).unwrap(),
            DataType::Float(5.0)
        );
        assert_eq!(
            ExpressionEvaluator::evaluate("count % 3", &c).unwrap(),
            DataType::Integer(1)
        );
    }

    #[test]
    fn test_comparison() {
        let c = ctx();
        assert!(ExpressionEvaluator::evaluate_bool("age > 20", &c).unwrap());
        assert!(!ExpressionEvaluator::evaluate_bool("age < 20", &c).unwrap());
        assert!(ExpressionEvaluator::evaluate_bool("age >= 25", &c).unwrap());
        assert!(ExpressionEvaluator::evaluate_bool("age <= 25", &c).unwrap());
        assert!(ExpressionEvaluator::evaluate_bool("age == 25", &c).unwrap());
        assert!(ExpressionEvaluator::evaluate_bool("age != 30", &c).unwrap());
    }

    #[test]
    fn test_logical() {
        let c = ctx();
        assert!(ExpressionEvaluator::evaluate_bool("active && age > 20", &c).unwrap());
        assert!(ExpressionEvaluator::evaluate_bool("active || age > 100", &c).unwrap());
        assert!(!ExpressionEvaluator::evaluate_bool("!active", &c).unwrap());
        assert!(ExpressionEvaluator::evaluate_bool("!false", &c).unwrap());
    }

    #[test]
    fn test_ternary() {
        let c = ctx();
        assert_eq!(
            ExpressionEvaluator::evaluate("age > 18 ? 'adult' : 'minor'", &c).unwrap(),
            DataType::String("adult".to_string())
        );
        assert_eq!(
            ExpressionEvaluator::evaluate("age > 30 ? 'adult' : 'young'", &c).unwrap(),
            DataType::String("young".to_string())
        );
    }

    #[test]
    fn test_string_concat() {
        let c = ctx();
        assert_eq!(
            ExpressionEvaluator::evaluate("\"Hello, \" + name", &c).unwrap(),
            DataType::String("Hello, Alice".to_string())
        );
    }

    #[test]
    fn test_parentheses() {
        let c = ctx();
        assert_eq!(
            ExpressionEvaluator::evaluate("(age + 5) * 2", &c).unwrap(),
            DataType::Integer(60)
        );
    }

    #[test]
    fn test_builtin_functions() {
        let c = ctx();
        assert_eq!(
            ExpressionEvaluator::evaluate("len(name)", &c).unwrap(),
            DataType::Integer(5)
        );
        assert_eq!(
            ExpressionEvaluator::evaluate("upper(name)", &c).unwrap(),
            DataType::String("ALICE".to_string())
        );
        assert_eq!(
            ExpressionEvaluator::evaluate("lower(name)", &c).unwrap(),
            DataType::String("alice".to_string())
        );
        assert_eq!(
            ExpressionEvaluator::evaluate("round(score)", &c).unwrap(),
            DataType::Integer(96)
        );
        assert!(ExpressionEvaluator::evaluate_bool("isEmpty('')", &c).unwrap());
        assert!(!ExpressionEvaluator::evaluate_bool("isEmpty(name)", &c).unwrap());
    }

    #[test]
    fn test_if_function() {
        let c = ctx();
        assert_eq!(
            ExpressionEvaluator::evaluate("if(active, 'yes', 'no')", &c).unwrap(),
            DataType::String("yes".to_string())
        );
    }

    #[test]
    fn test_negation() {
        let c = ctx();
        assert_eq!(
            ExpressionEvaluator::evaluate("-age", &c).unwrap(),
            DataType::Integer(-25)
        );
    }

    #[test]
    fn test_substring() {
        let c = ctx();
        assert_eq!(
            ExpressionEvaluator::evaluate("substring(name, 0, 3)", &c).unwrap(),
            DataType::String("Ali".to_string())
        );
    }

    #[test]
    fn test_division_by_zero() {
        let c = HashMap::new();
        let result = ExpressionEvaluator::evaluate("10 / 0", &c);
        assert!(result.is_err());
    }
}
