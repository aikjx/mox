// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 模板变量替换
//!
//! 统一三套引擎的模板语法：
//! - flow_engine: {{var}} 语法
//! - workflow_engine: ${var} 语法
//! - flow_svc: （无内置模板）
//!
//! 统一后使用 {{var}} 为主语法，同时兼容 ${var}。

use std::collections::HashMap;

/// 模板变量替换
///
/// 支持语法：
/// - `{{variable_name}}` - 标准语法
/// - `${variable_name}` - 兼容语法
///
/// 未定义的变量保留占位符（不替换）。
///
/// # 示例
/// ```
/// use std::collections::HashMap;
/// use mox_flow_unified_process_core::utils::template::apply_template;
///
/// let mut vars = HashMap::new();
/// vars.insert("name".to_string(), serde_json::json!("世界"));
/// vars.insert("n".to_string(), serde_json::json!(42));
///
/// assert_eq!(apply_template("hello {{name}}", &vars), "hello 世界");
/// assert_eq!(apply_template("num={{n}}", &vars), "num=42");
/// ```
pub fn apply_template(template: &str, variables: &HashMap<String, serde_json::Value>) -> String {
    let mut result = template.to_string();

    // 先替换 {{var}} 语法
    for (key, value) in variables {
        let placeholder = format!("{{{{{}}}}}", key);
        let val_str = value_to_string(value);
        result = result.replace(&placeholder, &val_str);
    }

    // 再替换 ${var} 语法（兼容）
    for (key, value) in variables {
        let placeholder = format!("${{{}}}", key);
        let val_str = value_to_string(value);
        result = result.replace(&placeholder, &val_str);
    }

    result
}

/// 将 serde_json::Value 转为字符串（用于模板替换）
fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// 从配置值中解析模板变量（配置可能是 JSON 值）
pub fn resolve_template(
    config: Option<&serde_json::Value>,
    variables: &HashMap<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    config.map(|c| {
        let s = serde_json::to_string(c).unwrap_or_default();
        let resolved = apply_template(&s, variables);
        serde_json::from_str(&resolved).unwrap_or_else(|_| c.clone())
    })
}

/// 提取表达式中的变量名（{{var}} 格式）
pub fn extract_var_names(expression: &str) -> Vec<String> {
    let mut names = Vec::new();
    let bytes = expression.as_bytes();
    let mut i = 0;

    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            // 找到 {{
            let start = i + 2;
            let mut end = start;
            while end + 1 < bytes.len() && !(bytes[end] == b'}' && bytes[end + 1] == b'}') {
                end += 1;
            }
            if end + 1 < bytes.len() {
                let name = &expression[start..end];
                if !name.is_empty() {
                    names.push(name.to_string());
                }
                i = end + 2;
                continue;
            }
        }
        i += 1;
    }

    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_template_substitutes_variables() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), serde_json::json!("世界"));
        vars.insert("n".to_string(), serde_json::json!(42));
        vars.insert("flag".to_string(), serde_json::json!(true));

        assert_eq!(apply_template("hello {{name}}", &vars), "hello 世界");
        assert_eq!(apply_template("num={{n}}", &vars), "num=42");
        assert_eq!(apply_template("flag={{flag}}", &vars), "flag=true");
    }

    #[test]
    fn test_apply_template_compatible_dollar_syntax() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), serde_json::json!("world"));

        assert_eq!(apply_template("hello ${name}", &vars), "hello world");
    }

    #[test]
    fn test_apply_template_missing_variable_keeps_placeholder() {
        let vars = HashMap::new();
        assert_eq!(
            apply_template("x={{missing}}", &vars),
            "x={{missing}}"
        );
    }

    #[test]
    fn test_extract_var_names() {
        let expr = "hello {{name}}, age={{age}}";
        let names = extract_var_names(expr);
        assert_eq!(names, vec!["name", "age"]);
    }

    #[test]
    fn test_extract_var_names_empty() {
        let names = extract_var_names("no variables here");
        assert!(names.is_empty());
    }
}
