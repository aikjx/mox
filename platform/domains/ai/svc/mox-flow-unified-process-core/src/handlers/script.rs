// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 脚本节点处理器
//!
//! 提供基础脚本执行能力（简化版沙箱）。
//! 生产环境建议替换为 WASM 或进程隔离的沙箱实现。

use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;

use crate::error::FlowResult;
use crate::executor::{ExecutionContext, NodeHandler};
use crate::types::*;
use crate::utils::template::apply_template;

pub struct ScriptHandler;

#[async_trait]
impl NodeHandler for ScriptHandler {
    fn kind(&self) -> UnifiedNodeKind {
        UnifiedNodeKind::Script
    }

    async fn execute(
        &self,
        node: &UnifiedFlowNode,
        context: &ExecutionContext<'_>,
    ) -> FlowResult<UnifiedNodeResult> {
        let start = Instant::now();

        let (language, code) = match &node.config {
            UnifiedNodeConfig::Script { language, code } => (language.clone(), code.clone()),
            _ => {
                return Ok(UnifiedNodeResult::failed(
                    node,
                    "Script 节点配置类型不匹配".into(),
                    start.elapsed().as_millis() as u64,
                ))
            }
        };

        // 模板替换：解析 {{var}} 变量引用
        let resolved_code = apply_template(&code, context.variables);

        // 简化脚本执行（仅支持基本表达式和 print）
        match execute_simple_script(&resolved_code, context.variables) {
            Ok(output) => Ok(UnifiedNodeResult::success(
                node,
                output,
                start.elapsed().as_millis() as u64,
            )),
            Err(e) => Ok(UnifiedNodeResult::failed(
                node,
                e,
                start.elapsed().as_millis() as u64,
            )),
        }
    }

    fn name(&self) -> &'static str {
        "builtin_script"
    }
}

/// 执行简化脚本（支持 print、变量赋值、基本数学运算）
fn execute_simple_script(
    code: &str,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let lines: Vec<&str> = code
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .collect();

    let mut output = String::new();
    let mut local_vars = variables.clone();

    for line in lines {
        if line.starts_with("print(") && line.ends_with(')') {
            let expr = &line[6..line.len() - 1];
            let val = evaluate_expr(expr, &local_vars)?;
            output.push_str(&format!("{}\n", val));
        } else if let Some(assignment) = line.split_once('=') {
            let var_name = assignment.0.trim();
            let expr = assignment.1.trim();
            let val = evaluate_expr(expr, &local_vars)?;
            local_vars.insert(
                var_name.to_string(),
                serde_json::Value::String(val),
            );
        }
        // 忽略其他行（简化版）
    }

    Ok(serde_json::json!({
        "output": output.trim(),
        "variables": local_vars,
    }))
}

fn evaluate_expr(
    expr: &str,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<String, String> {
    let trimmed = expr.trim();

    // 字符串字面量
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        return Ok(trimmed[1..trimmed.len() - 1].to_string());
    }

    // 数字
    if let Ok(n) = trimmed.parse::<f64>() {
        return Ok(if n.fract() == 0.0 {
            format!("{}", n as i64)
        } else {
            format!("{}", n)
        });
    }

    // 变量
    if let Some(val) = variables.get(trimmed) {
        return Ok(val.as_str().unwrap_or(&val.to_string()).to_string());
    }

    // 简单数学运算
    if expr.contains('+') || expr.contains('-') || expr.contains('*') || expr.contains('/') {
        if let Some(result) = simple_math(expr) {
            return Ok(format!("{}", result));
        }
    }

    Ok(expr.to_string())
}

fn simple_math(expr: &str) -> Option<f64> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    for c in expr.chars() {
        if c == '+' || c == '-' || c == '*' || c == '/' {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            tokens.push(c.to_string());
        } else if !c.is_whitespace() {
            current.push(c);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    if tokens.len() >= 3 {
        if let (Some(a), Some(b)) = (
            tokens.first().and_then(|t| t.parse::<f64>().ok()),
            tokens.get(2).and_then(|t| t.parse::<f64>().ok()),
        ) {
            let op = tokens.get(1).map(|s| s.as_str()).unwrap_or("+");
            return Some(match op {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                "/" if b != 0.0 => a / b,
                _ => return None,
            });
        }
    }
    None
}
