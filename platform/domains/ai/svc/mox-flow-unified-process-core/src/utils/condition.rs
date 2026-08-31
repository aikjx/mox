// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 条件表达式求值
//!
//! 统一两套引擎的条件求值逻辑：
//! - flow_engine::evaluate_condition（{{var}} 语法）
//! - workflow_engine::eval_condition（${var} 语法，功能更完整）
//!
//! 支持的操作：
//! - 布尔字面量：true/false/yes/no/1/0
//! - 比较操作：==/!=/>/</>=/<=
//! - 字符串比较（带引号）
//! - 变量引用：{{var}} 或 ${var}

use std::collections::HashMap;

use crate::utils::template::apply_template;

/// 求值条件表达式
///
/// # 支持的语法
///
/// 布尔字面量：
/// - `true` / `yes` / `1` → true
/// - `false` / `no` / `0` → false
///
/// 比较操作：
/// - `==` / `=` / `!=` / `<>` → 相等/不等比较
/// - `>` / `<` / `>=` / `<=` → 数值比较
///
/// 变量引用：
/// - `{{var}}` 或 `${var}` → 从 variables 中取值
///
/// # 示例
/// ```ignore
/// let mut v = HashMap::new();
/// v.insert("a".to_string(), serde_json::json!(10));
/// v.insert("name".to_string(), serde_json::json!("bob"));
///
/// assert!(evaluate_condition("true", &v));
/// assert!(evaluate_condition("{{a}} > 5", &v));
/// assert!(evaluate_condition("{{name}} == \"bob\"", &v));
/// ```
pub fn evaluate_condition(
    condition: &str,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<bool, String> {
    let resolved = apply_template(condition, variables);
    let lower = resolved.trim().to_lowercase();

    // 布尔字面量
    if lower == "true" || lower == "yes" || lower == "1" {
        return Ok(true);
    }
    if lower == "false" || lower == "no" || lower == "0" {
        return Ok(false);
    }

    // 比较操作
    if let Some((left, op, right)) = parse_comparison(&resolved) {
        let left_val = evaluate_value(&left, variables);
        let right_val = evaluate_value(&right, variables);

        return Ok(match op.as_str() {
            "==" | "=" => left_val == right_val,
            "!=" | "<>" => left_val != right_val,
            ">" => parse_number(&left_val) > parse_number(&right_val),
            "<" => parse_number(&left_val) < parse_number(&right_val),
            ">=" => parse_number(&left_val) >= parse_number(&right_val),
            "<=" => parse_number(&left_val) <= parse_number(&right_val),
            _ => false,
        });
    }

    // 非空字符串视为 true（fail-open for unknown）
    // 但如果还有未解析的变量占位符，返回 false 更安全
    if resolved.contains("{{") || resolved.contains("${") {
        return Ok(false);
    }

    Ok(!resolved.trim().is_empty())
}

/// 解析比较表达式
fn parse_comparison(expr: &str) -> Option<(String, String, String)> {
    let operators = ["==", "!=", ">=", "<=", "<>", ">", "<", "="];
    for op in operators {
        if let Some(parts) = expr.split_once(op) {
            return Some((
                parts.0.trim().to_string(),
                op.to_string(),
                parts.1.trim().to_string(),
            ));
        }
    }
    None
}

/// 求值单个值（字符串字面量 / 变量 / 数字）
fn evaluate_value(expr: &str, variables: &HashMap<String, serde_json::Value>) -> String {
    let trimmed = expr.trim();

    // 字符串字面量（双引号）
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        return trimmed[1..trimmed.len() - 1].to_string();
    }
    // 字符串字面量（单引号）
    if trimmed.starts_with('\'') && trimmed.ends_with('\'') {
        return trimmed[1..trimmed.len() - 1].to_string();
    }

    // 变量（变量名可能已被模板替换，所以先检查 variables）
    if let Some(val) = variables.get(trimmed) {
        return serde_json::to_string(val).unwrap_or(trimmed.to_string());
    }

    trimmed.to_string()
}

/// 解析数字（失败返回 0.0）
fn parse_number(s: &str) -> f64 {
    s.trim().parse::<f64>().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boolean_literals() {
        let v = HashMap::new();
        assert!(evaluate_condition("true", &v).unwrap());
        assert!(evaluate_condition("yes", &v).unwrap());
        assert!(evaluate_condition("1", &v).unwrap());
        assert!(!evaluate_condition("false", &v).unwrap());
        assert!(!evaluate_condition("no", &v).unwrap());
        assert!(!evaluate_condition("0", &v).unwrap());
    }

    #[test]
    fn test_numeric_comparisons() {
        let mut v = HashMap::new();
        v.insert("a".to_string(), serde_json::json!(10));

        assert!(evaluate_condition("{{a}} > 5", &v).unwrap());
        assert!(evaluate_condition("{{a}} < 20", &v).unwrap());
        assert!(evaluate_condition("{{a}} >= 10", &v).unwrap());
        assert!(evaluate_condition("{{a}} <= 10", &v).unwrap());
        assert!(evaluate_condition("{{a}} == 10", &v).unwrap());
        assert!(evaluate_condition("{{a}} != 11", &v).unwrap());
    }

    #[test]
    fn test_string_comparisons() {
        let mut v = HashMap::new();
        v.insert("name".to_string(), serde_json::json!("bob"));

        assert!(evaluate_condition("{{name}} == \"bob\"", &v).unwrap());
        assert!(!evaluate_condition("{{name}} == \"alice\"", &v).unwrap());
    }

    #[test]
    fn test_unresolved_variables_return_false() {
        let v = HashMap::new();
        assert!(!evaluate_condition("{{missing}}", &v).unwrap());
    }

    #[test]
    fn test_empty_string_is_false() {
        let v = HashMap::new();
        assert!(!evaluate_condition("", &v).unwrap());
    }
}
