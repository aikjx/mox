// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! ReAct 工具注册表与内置工具
//!
//! 工具协议：模型在回复中输出 `<tool_call>{"name":"...","arguments":{...}}</tool_call>`，
//! ReAct 循环解析后执行对应工具并把结果作为观察消息回喂模型。
//!
//! 内置工具：
//! - `calculate(expression)`：安全数学表达式求值（无 eval，自研递归下降解析器）
//! - `expert_lookup(expert_id)`：查询专家元信息
//! - `now()`：当前 UTC 时间

use serde_json::Value;
use std::sync::Arc;

/// 工具执行结果
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub ok: bool,
    pub output: String,
}

impl ToolResult {
    pub fn ok(output: impl Into<String>) -> Self {
        Self { ok: true, output: output.into() }
    }
    pub fn err(output: impl Into<String>) -> Self {
        Self { ok: false, output: output.into() }
    }
}

/// 工具抽象
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn run(&self, args: &Value) -> anyhow::Result<ToolResult>;
}

// ============================================================================
// calculate：安全数学表达式求值
// ============================================================================

/// 数学表达式求值工具（递归下降解析器：+ - * / ^ 括号 一元负号）
pub struct CalculateTool;

impl Tool for CalculateTool {
    fn name(&self) -> &str {
        "calculate"
    }
    fn description(&self) -> &str {
        "对数学表达式求值。arguments: {\"expression\": \"2+3*4\"}"
    }
    fn run(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let expr = args
            .get("expression")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("calculate 需要 expression 参数"))?;
        match eval_expr(expr) {
            Ok(v) => Ok(ToolResult::ok(format!("{}", v))),
            Err(e) => Ok(ToolResult::err(format!("表达式求值失败: {}", e))),
        }
    }
}

/// 解析并求值表达式，返回 f64
pub fn eval_expr(input: &str) -> anyhow::Result<f64> {
    let tokens = tokenize(input)?;
    let mut p = Parser { tokens, pos: 0 };
    let v = p.parse_expr()?;
    p.expect_end()?;
    Ok(v)
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
}

fn tokenize(input: &str) -> anyhow::Result<Vec<Tok>> {
    let mut out = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }
            '+' => {
                out.push(Tok::Plus);
                chars.next();
            }
            '-' => {
                out.push(Tok::Minus);
                chars.next();
            }
            '*' => {
                out.push(Tok::Star);
                chars.next();
            }
            '/' => {
                out.push(Tok::Slash);
                chars.next();
            }
            '^' => {
                out.push(Tok::Caret);
                chars.next();
            }
            '(' => {
                out.push(Tok::LParen);
                chars.next();
            }
            ')' => {
                out.push(Tok::RParen);
                chars.next();
            }
            '0'..='9' | '.' => {
                let mut num = String::new();
                while let Some(&d) = chars.peek() {
                    if d.is_ascii_digit() || d == '.' {
                        num.push(d);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let v: f64 = num
                    .parse()
                    .map_err(|_| anyhow::anyhow!("非法数字: {}", num))?;
                out.push(Tok::Num(v));
            }
            other => {
                return Err(anyhow::anyhow!("非法字符: {}", other));
            }
        }
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
    fn expect_end(&self) -> anyhow::Result<()> {
        if self.pos == self.tokens.len() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("表达式末尾存在多余 token"))
        }
    }

    // expr := term (('+'|'-') term)*
    fn parse_expr(&mut self) -> anyhow::Result<f64> {
        let mut left = self.parse_term()?;
        loop {
            match self.peek() {
                Some(Tok::Plus) => {
                    self.next();
                    let right = self.parse_term()?;
                    left += right;
                }
                Some(Tok::Minus) => {
                    self.next();
                    let right = self.parse_term()?;
                    left -= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    // term := factor (('*'|'/') factor)*
    fn parse_term(&mut self) -> anyhow::Result<f64> {
        let mut left = self.parse_factor()?;
        loop {
            match self.peek() {
                Some(Tok::Star) => {
                    self.next();
                    let right = self.parse_factor()?;
                    left *= right;
                }
                Some(Tok::Slash) => {
                    self.next();
                    let right = self.parse_factor()?;
                    if right.abs() < f64::EPSILON {
                        return Err(anyhow::anyhow!("除以零"));
                    }
                    left /= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    // factor := ('+'|'-') factor | atom ('^' factor)?
    fn parse_factor(&mut self) -> anyhow::Result<f64> {
        if let Some(Tok::Minus) = self.peek() {
            self.next();
            return Ok(-self.parse_factor()?);
        }
        if let Some(Tok::Plus) = self.peek() {
            self.next();
            return self.parse_factor();
        }
        let base = self.parse_atom()?;
        if let Some(Tok::Caret) = self.peek() {
            self.next();
            let exp = self.parse_factor()?;
            return Ok(base.powf(exp));
        }
        Ok(base)
    }

    fn parse_atom(&mut self) -> anyhow::Result<f64> {
        match self.next() {
            Some(Tok::Num(v)) => Ok(v),
            Some(Tok::LParen) => {
                let v = self.parse_expr()?;
                match self.next() {
                    Some(Tok::RParen) => Ok(v),
                    _ => Err(anyhow::anyhow!("缺少右括号")),
                }
            }
            _ => Err(anyhow::anyhow!("非法表达式")),
        }
    }
}

// ============================================================================
// expert_lookup：专家元信息查询
// ============================================================================

/// 专家信息查询工具
pub struct ExpertLookupTool {
    /// 查询回调：输入 expert_id 返回 (id, name, domain, capabilities) 或 None
    lookup: Arc<dyn Fn(&str) -> Option<(String, String, String, Vec<String>)> + Send + Sync>,
}

impl ExpertLookupTool {
    pub fn new<F>(lookup: F) -> Self
    where
        F: Fn(&str) -> Option<(String, String, String, Vec<String>)> + Send + Sync + 'static,
    {
        Self { lookup: Arc::new(lookup) }
    }
}

impl Tool for ExpertLookupTool {
    fn name(&self) -> &str {
        "expert_lookup"
    }
    fn description(&self) -> &str {
        "查询专家元信息（id/name/domain/capabilities）。arguments: {\"expert_id\": \"...\"}"
    }
    fn run(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let id = args
            .get("expert_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("expert_lookup 需要 expert_id 参数"))?;
        match (self.lookup)(id) {
            Some((id, name, domain, caps)) => Ok(ToolResult::ok(format!(
                "专家 {}: name={}, domain={}, capabilities={:?}",
                id, name, domain, caps
            ))),
            None => Ok(ToolResult::err(format!("未找到专家 {}", id))),
        }
    }
}

// ============================================================================
// now：当前时间
// ============================================================================

/// 当前 UTC 时间工具
pub struct NowTool;

impl Tool for NowTool {
    fn name(&self) -> &str {
        "now"
    }
    fn description(&self) -> &str {
        "获取当前 UTC 时间。无需参数。"
    }
    fn run(&self, _args: &Value) -> anyhow::Result<ToolResult> {
        let now = chrono::Utc::now();
        Ok(ToolResult::ok(now.to_rfc3339()))
    }
}

// ============================================================================
// ToolRegistry
// ============================================================================

/// 工具注册表：按名称查找工具
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// 内置默认工具集（calculate / now），可再追加
    pub fn with_builtins() -> Self {
        let mut r = Self::new();
        r.register(Arc::new(CalculateTool));
        r.register(Arc::new(NowTool));
        r
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn find(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .cloned()
    }

    /// 供系统提示词使用的工具说明列表
    pub fn tool_descriptions(&self) -> String {
        self.tools
            .iter()
            .map(|t| format!("- `{}`：{}", t.name(), t.description()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_basic() {
        assert_eq!(eval_expr("2+3*4").unwrap(), 14.0);
        assert_eq!(eval_expr("(2+3)*4").unwrap(), 20.0);
        assert_eq!(eval_expr("10/4").unwrap(), 2.5);
        assert_eq!(eval_expr("2^10").unwrap(), 1024.0);
        assert_eq!(eval_expr("-3+5").unwrap(), 2.0);
        assert_eq!(eval_expr("1.5*2").unwrap(), 3.0);
    }

    #[test]
    fn eval_errors() {
        assert!(eval_expr("1/0").is_err());
        assert!(eval_expr("2+").is_err());
        assert!(eval_expr("(2+3").is_err());
        assert!(eval_expr("2+@").is_err());
    }

    #[test]
    fn calculate_tool_ok_and_err() {
        let t = CalculateTool;
        let r = t
            .run(&serde_json::json!({"expression": "6*7"}))
            .unwrap();
        assert!(r.ok);
        assert_eq!(r.output, "42");
        let r = t
            .run(&serde_json::json!({"expression": "1/0"}))
            .unwrap();
        assert!(!r.ok);
    }

    #[test]
    fn registry_finds_tool() {
        let reg = ToolRegistry::with_builtins();
        assert!(reg.find("calculate").is_some());
        assert!(reg.find("now").is_some());
        assert!(reg.find("nope").is_none());
        assert!(!reg.is_empty());
        assert!(reg.tool_descriptions().contains("calculate"));
    }
}
