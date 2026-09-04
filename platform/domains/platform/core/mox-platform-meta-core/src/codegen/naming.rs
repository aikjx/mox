// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 命名归一化与类型映射（codegen 内部公共件）。
//!
//! `field_type_to_string` 与 `repo.rs` 写入侧保持同一映射（测试锁定，防漂移）。

use crate::model::FieldType;

/// FieldType 枚举 → 元数据字符串（与 `repo.rs` 写入侧同源映射）。
#[must_use]
pub fn field_type_to_string(t: &FieldType) -> String {
    match t {
        FieldType::String => "string",
        FieldType::Int => "integer",
        FieldType::Decimal => "decimal",
        FieldType::Boolean => "boolean",
        FieldType::DateTime => "datetime",
        FieldType::Enum => "enum",
        FieldType::Text => "text",
        FieldType::Json => "json",
    }
    .to_string()
}

/// 元数据字符串/常见别名 → FieldType（`field_type_to_string` 的规范逆映射）。
#[must_use]
pub fn field_type_from_str(s: &str) -> Option<FieldType> {
    match s.trim().to_lowercase().as_str() {
        "string" | "str" | "varchar" => Some(FieldType::String),
        "integer" | "int" => Some(FieldType::Int),
        "decimal" | "number" | "float" | "double" => Some(FieldType::Decimal),
        "boolean" | "bool" => Some(FieldType::Boolean),
        "datetime" | "date" | "time" => Some(FieldType::DateTime),
        "enum" | "select" => Some(FieldType::Enum),
        "text" | "longtext" | "richtext" => Some(FieldType::Text),
        "json" | "object" => Some(FieldType::Json),
        _ => None,
    }
}

/// 元数据字段类型字符串是否受支持。
#[must_use]
pub fn is_known_field_type(t: &str) -> bool {
    matches!(
        t,
        "string" | "integer" | "decimal" | "boolean" | "datetime" | "enum" | "text" | "json"
    )
}

/// 是否合法 snake_case 标识符（小写字母/数字/下划线，字母或下划线开头）。
#[must_use]
pub fn is_valid_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// `project_code` → `ProjectCode`。
#[must_use]
pub fn to_pascal(s: &str) -> String {
    s.split('_')
        .filter(|p| !p.is_empty())
        .map(capitalize)
        .collect()
}

/// `project_code` → `projectCode`。
#[must_use]
pub fn to_camel(s: &str) -> String {
    let pascal = to_pascal(s);
    let mut chars = pascal.chars();
    match chars.next() {
        Some(c) => c.to_ascii_lowercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// 简单确定性英文复数：y→ies / s,x,z,ch,sh→es / 其余+s。
#[must_use]
pub fn pluralize(s: &str) -> String {
    if s.ends_with('y') && s.len() > 1 && !is_vowel(s.as_bytes()[s.len() - 2]) {
        format!("{}ies", &s[..s.len() - 1])
    } else if s.ends_with("s")
        || s.ends_with("x")
        || s.ends_with("z")
        || s.ends_with("ch")
        || s.ends_with("sh")
    {
        format!("{s}es")
    } else {
        format!("{s}s")
    }
}

fn is_vowel(b: u8) -> bool {
    matches!(b, b'a' | b'e' | b'i' | b'o' | b'u')
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// 元数据类型 → TypeScript 类型。
#[must_use]
pub fn ts_type(field_type: &str) -> &'static str {
    match field_type {
        "integer" | "decimal" => "number",
        "boolean" => "boolean",
        "json" => "Record<string, unknown>",
        _ => "string",
    }
}

/// 元数据类型 → Rust 类型。
#[must_use]
pub fn rust_type(field_type: &str) -> &'static str {
    match field_type {
        "integer" => "i64",
        "decimal" => "f64",
        "boolean" => "bool",
        "json" => "serde_json::Value",
        _ => "String",
    }
}

/// 元数据类型 → SQL 列类型（SQLite 口径）。
#[must_use]
pub fn sql_type(field_type: &str) -> &'static str {
    match field_type {
        "integer" => "INTEGER",
        "decimal" => "REAL",
        "boolean" => "INTEGER",
        "json" => "TEXT",
        _ => "TEXT",
    }
}

/// 转义双引号字符串字面量（TS/Vue/JSON 通用）。
#[must_use]
pub fn esc_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naming_conversions() {
        assert_eq!(to_pascal("project_code"), "ProjectCode");
        assert_eq!(to_pascal("a"), "A");
        assert_eq!(to_pascal(""), "");
        assert_eq!(to_camel("project_code"), "projectCode");
        assert_eq!(to_camel("a_b_c"), "aBC");
        assert_eq!(pluralize("project"), "projects");
        assert_eq!(pluralize("category"), "categories");
        assert_eq!(pluralize("box"), "boxes");
        assert_eq!(pluralize("batch"), "batches");
        assert_eq!(pluralize("day"), "days");
    }

    #[test]
    fn test_type_mappings() {
        assert_eq!(ts_type("integer"), "number");
        assert_eq!(ts_type("decimal"), "number");
        assert_eq!(ts_type("enum"), "string");
        assert_eq!(ts_type("json"), "Record<string, unknown>");
        assert_eq!(rust_type("integer"), "i64");
        assert_eq!(rust_type("decimal"), "f64");
        assert_eq!(rust_type("boolean"), "bool");
        assert_eq!(sql_type("boolean"), "INTEGER");
        assert_eq!(sql_type("decimal"), "REAL");
    }

    #[test]
    fn test_field_type_inverse_mapping() {
        for t in [
            FieldType::String,
            FieldType::Int,
            FieldType::Decimal,
            FieldType::Boolean,
            FieldType::DateTime,
            FieldType::Enum,
            FieldType::Text,
            FieldType::Json,
        ] {
            let s = field_type_to_string(&t);
            assert_eq!(field_type_from_str(&s), Some(t), "inverse of `{s}`");
        }
        assert!(field_type_from_str("money").is_none());
        assert_eq!(field_type_from_str("Int"), Some(FieldType::Int));
    }

    #[test]
    fn test_ident_validation_and_escape() {
        assert!(is_valid_ident("project_code"));
        assert!(is_valid_ident("_x1"));
        assert!(!is_valid_ident("Project"));
        assert!(!is_valid_ident("1abc"));
        assert!(!is_valid_ident("bad-name"));
        assert!(!is_valid_ident(""));
        assert_eq!(esc_str("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
    }

    #[test]
    fn test_repo_field_type_mapping_alignment() {
        // codegen 的映射必须覆盖 repo.rs 写入侧全部 FieldType（含 is_known 反向校验）。
        for t in [
            FieldType::String,
            FieldType::Int,
            FieldType::Decimal,
            FieldType::Boolean,
            FieldType::DateTime,
            FieldType::Enum,
            FieldType::Text,
            FieldType::Json,
        ] {
            assert!(is_known_field_type(&field_type_to_string(&t)));
        }
    }
}
