// Copyright (c) 2026 璇玑 RelGraph · 流程算法归一化核心 (Unified Process & Algorithm Core)
// Licensed under the MIT License.

//! 决策表（Decision Table）
//!
//! 业务规则的表格化表达，支持：
//! - 多条件组合（输入列）
//! - 多输出列
//! - 命中策略：first-match / all-match / unique-match
//! - 优先级排序

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::error::{ProcessError, ProcessResult};
use crate::types::ProcessContext;

/// 决策表命中策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitPolicy {
    /// 命中第一条（按优先级）
    First,
    /// 命中所有满足的
    All,
    /// 必须唯一命中
    Unique,
    /// 命中优先级最高的一条
    Priority,
    /// 无命中则走默认
    Default,
}

/// 决策表列类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    /// 输入（条件）
    Input,
    /// 输出（结论）
    Output,
}

/// 决策表列定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionColumn {
    /// 列名
    pub name: String,
    /// 列类型
    pub column_type: ColumnType,
    /// 数据类型
    pub data_type: String,
    /// 描述
    pub description: Option<String>,
}

/// 决策表行（规则）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRow {
    /// 行 ID
    pub id: String,
    /// 描述
    pub description: Option<String>,
    /// 条件值：column_name -> value
    pub conditions: HashMap<String, Value>,
    /// 输出值：column_name -> value
    pub outputs: HashMap<String, Value>,
    /// 优先级（数值越大越优先）
    pub priority: u32,
    /// 是否启用
    pub enabled: bool,
}

/// 决策表定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTable {
    /// 决策表 ID
    pub id: String,
    /// 决策表名称
    pub name: String,
    /// 描述
    pub description: Option<String>,
    /// 列定义
    pub columns: Vec<DecisionColumn>,
    /// 规则行
    pub rows: Vec<DecisionRow>,
    /// 命中策略
    pub hit_policy: HitPolicy,
    /// 默认输出
    pub default_outputs: HashMap<String, Value>,
}

impl DecisionTable {
    /// 创建决策表
    pub fn new(name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: None,
            columns: Vec::new(),
            rows: Vec::new(),
            hit_policy: HitPolicy::First,
            default_outputs: HashMap::new(),
        }
    }

    /// 添加输入列
    pub fn add_input(&mut self, name: &str, data_type: &str) {
        self.columns.push(DecisionColumn {
            name: name.to_string(),
            column_type: ColumnType::Input,
            data_type: data_type.to_string(),
            description: None,
        });
    }

    /// 添加输出列
    pub fn add_output(&mut self, name: &str, data_type: &str) {
        self.columns.push(DecisionColumn {
            name: name.to_string(),
            column_type: ColumnType::Output,
            data_type: data_type.to_string(),
            description: None,
        });
    }

    /// 添加规则行
    pub fn add_row(
        &mut self,
        conditions: HashMap<String, Value>,
        outputs: HashMap<String, Value>,
        priority: u32,
    ) -> String {
        let row = DecisionRow {
            id: uuid::Uuid::new_v4().to_string(),
            description: None,
            conditions,
            outputs,
            priority,
            enabled: true,
        };
        let id = row.id.clone();
        self.rows.push(row);
        // 按优先级降序排序
        self.rows.sort_by(|a, b| b.priority.cmp(&a.priority));
        id
    }

    /// 评估决策表
    pub fn evaluate(
        &self,
        context: &mut ProcessContext,
        inputs: &HashMap<String, Value>,
    ) -> ProcessResult<Vec<HashMap<String, Value>>> {
        let mut matched: Vec<&DecisionRow> = Vec::new();

        for row in &self.rows {
            if !row.enabled {
                continue;
            }

            if self.row_matches(row, inputs) {
                matched.push(row);
            }
        }

        let results = match self.hit_policy {
            HitPolicy::First => {
                if let Some(row) = matched.first() {
                    vec![row.outputs.clone()]
                } else {
                    vec![self.default_outputs.clone()]
                }
            }
            HitPolicy::All => {
                if matched.is_empty() {
                    vec![self.default_outputs.clone()]
                } else {
                    matched.iter().map(|r| r.outputs.clone()).collect()
                }
            }
            HitPolicy::Unique => {
                if matched.len() > 1 {
                    return Err(ProcessError::RuleError(
                        "unique hit policy: multiple rules matched".to_string(),
                    ));
                }
                if let Some(row) = matched.first() {
                    vec![row.outputs.clone()]
                } else {
                    vec![self.default_outputs.clone()]
                }
            }
            HitPolicy::Priority => {
                // 已经按优先级排序了，取第一个
                if let Some(row) = matched.first() {
                    vec![row.outputs.clone()]
                } else {
                    vec![self.default_outputs.clone()]
                }
            }
            HitPolicy::Default => {
                if matched.is_empty() {
                    vec![self.default_outputs.clone()]
                } else {
                    matched.iter().map(|r| r.outputs.clone()).collect()
                }
            }
        };

        context.log(
            crate::types::LogLevel::Info,
            &format!(
                "decision table '{}': {} rows matched, {} results",
                self.name,
                matched.len(),
                results.len()
            ),
        );

        Ok(results)
    }

    /// 检查行是否匹配输入
    fn row_matches(&self, row: &DecisionRow, inputs: &HashMap<String, Value>) -> bool {
        for (col_name, cond_value) in &row.conditions {
            let input_val = inputs.get(col_name).unwrap_or(&Value::Null);

            // 支持 "-" 表示任意匹配
            if cond_value.is_string() && cond_value.as_str().unwrap_or("") == "-" {
                continue;
            }

            if !compare_values(input_val, cond_value) {
                return false;
            }
        }
        true
    }
}

fn compare_values(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            if let (Some(xi), Some(yi)) = (x.as_i64(), y.as_i64()) {
                xi == yi
            } else {
                let xf = x.as_f64().unwrap_or(0.0);
                let yf = y.as_f64().unwrap_or(0.0);
                (xf - yf).abs() < f64::EPSILON
            }
        }
        (Value::String(x), Value::String(y)) => {
            // 支持通配符 *
            if y.contains('*') {
                wildcard_match(x, y)
            } else {
                x == y
            }
        }
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

fn wildcard_match(text: &str, pattern: &str) -> bool {
    let pattern = pattern.replace('*', ".*");
    // 简单实现：按 * 分割后逐一匹配
    let parts: Vec<&str> = pattern.split(".*").collect();
    if parts.len() == 1 {
        return text == parts[0];
    }

    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue; // 开头或结尾的 *
        }
        let remaining = &text[pos..];
        if let Some(idx) = remaining.find(*part) {
            if i == 0 && idx != 0 {
                return false; // 第一个非空部分必须从开头匹配
            }
            pos += idx + part.len();
        } else {
            return false;
        }
    }

    // 最后一个部分为空（以*结尾）或已经匹配到末尾
    parts.last().map(|s| s.is_empty()).unwrap_or(false) || pos == text.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_decision_table() {
        let mut dt = DecisionTable::new("loan_approval");
        dt.add_input("age", "integer");
        dt.add_input("income", "integer");
        dt.add_output("approved", "boolean");
        dt.add_output("amount", "number");

        let mut cond1 = HashMap::new();
        cond1.insert("age".to_string(), json!(30));
        cond1.insert("income".to_string(), json!(10000));
        let mut out1 = HashMap::new();
        out1.insert("approved".to_string(), json!(true));
        out1.insert("amount".to_string(), json!(50000));
        dt.add_row(cond1, out1, 100);

        let mut cond2 = HashMap::new();
        cond2.insert("age".to_string(), json!(25));
        cond2.insert("income".to_string(), json!(5000));
        let mut out2 = HashMap::new();
        out2.insert("approved".to_string(), json!(false));
        out2.insert("amount".to_string(), json!(0));
        dt.add_row(cond2, out2, 50);

        let mut context = ProcessContext::new();
        let mut inputs = HashMap::new();
        inputs.insert("age".to_string(), json!(30));
        inputs.insert("income".to_string(), json!(10000));

        let results = dt.evaluate(&mut context, &inputs).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["approved"], json!(true));
        assert_eq!(results[0]["amount"], json!(50000));
    }

    #[test]
    fn test_default_output() {
        let mut dt = DecisionTable::new("test");
        dt.add_input("x", "integer");
        dt.add_output("result", "string");

        let mut default = HashMap::new();
        default.insert("result".to_string(), json!("default"));
        dt.default_outputs = default;

        let mut context = ProcessContext::new();
        let mut inputs = HashMap::new();
        inputs.insert("x".to_string(), json!(999));

        let results = dt.evaluate(&mut context, &inputs).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["result"], json!("default"));
    }

    #[test]
    fn test_wildcard_match() {
        assert!(wildcard_match("hello world", "hello*"));
        assert!(wildcard_match("hello world", "*world"));
        assert!(wildcard_match("hello world", "*lo*wor*"));
        assert!(!wildcard_match("hello world", "abc*"));
    }

    #[test]
    fn test_all_hit_policy() {
        let mut dt = DecisionTable::new("test");
        dt.hit_policy = HitPolicy::All;
        dt.add_input("score", "integer");
        dt.add_output("tag", "string");

        let mut cond1 = HashMap::new();
        cond1.insert("score".to_string(), json!(90));
        let mut out1 = HashMap::new();
        out1.insert("tag".to_string(), json!("high"));
        dt.add_row(cond1, out1, 100);

        let mut cond2 = HashMap::new();
        cond2.insert("score".to_string(), json!(90));
        let mut out2 = HashMap::new();
        out2.insert("tag".to_string(), json!("excellent"));
        dt.add_row(cond2, out2, 50);

        let mut context = ProcessContext::new();
        let mut inputs = HashMap::new();
        inputs.insert("score".to_string(), json!(90));

        let results = dt.evaluate(&mut context, &inputs).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_unique_hit_policy_conflict() {
        let mut dt = DecisionTable::new("test");
        dt.hit_policy = HitPolicy::Unique;
        dt.add_input("x", "integer");
        dt.add_output("y", "integer");

        let mut cond = HashMap::new();
        cond.insert("x".to_string(), json!(1));
        let mut out = HashMap::new();
        out.insert("y".to_string(), json!(10));
        dt.add_row(cond.clone(), out, 100);

        let mut out2 = HashMap::new();
        out2.insert("y".to_string(), json!(20));
        dt.add_row(cond, out2, 50);

        let mut context = ProcessContext::new();
        let mut inputs = HashMap::new();
        inputs.insert("x".to_string(), json!(1));

        let result = dt.evaluate(&mut context, &inputs);
        assert!(result.is_err());
    }
}
