//! 流程图驱动引擎 - AI综合处理核心
//!
//! 支持节点类型：
//! - Start/End: 流程控制
//! - LLM: AI大模型调用
//! - Browser: 浏览器自动化
//! - HTTP: HTTP请求
//! - Operator: 算子执行
//! - Condition: 条件分支
//! - Transform: 数据转换
//! - Script: 自定义脚本
//! - DataInput/DataOutput: 输入输出
//! - Parallel: 并行执行

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FlowError {
    #[error("节点不存在: {0}")]
    NodeNotFound(String),
    #[error("循环检测: {0}")]
    CycleDetected(String),
    #[error("执行失败: {0}")]
    ExecutionFailed(String),
    #[error("条件评估错误: {0}")]
    ConditionError(String),
    #[error("配置无效: {0}")]
    InvalidConfig(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNode {
    pub id: String,
    pub node_type: NodeType,
    pub name: String,
    pub config: serde_json::Value,
    pub position: Option<Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    Start,
    End,
    Task,
    Guard,
    Decision,
    Event,
    LLM,
    Browser,
    HttpRequest,
    Operator,
    Condition,
    Transform,
    Script,
    DataInput,
    DataOutput,
    Parallel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
    pub variables: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowExecutionResult {
    pub flow_id: String,
    pub flow_name: String,
    pub success: bool,
    pub node_results: Vec<NodeExecutionResult>,
    pub output: Option<serde_json::Value>,
    pub variables: HashMap<String, serde_json::Value>,
    pub execution_time_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionResult {
    pub node_id: String,
    pub node_name: String,
    pub node_type: String,
    pub status: String,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

pub struct FlowEngine {
    flows: HashMap<String, FlowDefinition>,
}

impl FlowEngine {
    pub fn new() -> Self {
        Self {
            flows: HashMap::new(),
        }
    }

    pub fn create_flow(&mut self, mut flow: FlowDefinition) -> Result<FlowDefinition, FlowError> {
        if flow.nodes.is_empty() {
            return Err(FlowError::InvalidConfig("流程图必须至少包含一个节点".into()));
        }
        if !flow.nodes.iter().any(|n| matches!(n.node_type, NodeType::Start)) {
            return Err(FlowError::InvalidConfig("流程图必须包含Start节点".into()));
        }
        let now = chrono::Utc::now();
        flow.created_at = now;
        flow.updated_at = now;
        self.flows.insert(flow.id.clone(), flow.clone());
        Ok(flow)
    }

    pub fn get_flow(&self, id: &str) -> Option<&FlowDefinition> {
        self.flows.get(id)
    }

    pub fn list_flows(&self) -> Vec<&FlowDefinition> {
        self.flows.values().collect()
    }

    pub fn delete_flow(&mut self, id: &str) -> bool {
        self.flows.remove(id).is_some()
    }

    /// 更新流程图：目标必须已存在，且更新后仍通过结构校验（Start/End/无环/边引用有效）。
    pub fn update_flow(&mut self, flow: FlowDefinition) -> Result<FlowDefinition, FlowError> {
        if !self.flows.contains_key(&flow.id) {
            return Err(FlowError::InvalidConfig(format!(
                "流程图不存在: {}（更新目标必须已创建）",
                flow.id
            )));
        }
        Self::validate_flow(&flow)?;
        let now = chrono::Utc::now();
        let mut updated = flow;
        updated.updated_at = now;
        self.flows.insert(updated.id.clone(), updated.clone());
        Ok(updated)
    }

    pub fn validate_flow(flow: &FlowDefinition) -> Result<(), FlowError> {
        // 检查Start和End节点
        let has_start = flow.nodes.iter().any(|n| matches!(n.node_type, NodeType::Start));
        let has_end = flow.nodes.iter().any(|n| matches!(n.node_type, NodeType::End));
        if !has_start { return Err(FlowError::InvalidConfig("缺少Start节点".into())); }
        if !has_end { return Err(FlowError::InvalidConfig("缺少End节点".into())); }

        // 检查循环
        let node_map: HashMap<&str, &FlowNode> = flow.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        for edge in &flow.edges {
            adj.entry(edge.source.clone()).or_default().push(edge.target.clone());
        }
        if detect_cycle(&adj) {
            return Err(FlowError::CycleDetected("流程图存在循环依赖".into()));
        }

        // 检查所有边的节点存在
        for edge in &flow.edges {
            if !node_map.contains_key(edge.source.as_str()) {
                return Err(FlowError::NodeNotFound(edge.source.clone()));
            }
            if !node_map.contains_key(edge.target.as_str()) {
                return Err(FlowError::NodeNotFound(edge.target.clone()));
            }
        }
        Ok(())
    }

    pub async fn execute_flow(
        &mut self,
        flow_id: &str,
        input: HashMap<String, serde_json::Value>,
    ) -> Result<FlowExecutionResult, FlowError> {
        let flow = self.flows.get(flow_id).cloned()
            .ok_or_else(|| FlowError::NodeNotFound(flow_id.into()))?;

        // 合并变量
        let mut variables = flow.variables.clone();
        variables.extend(input.clone());

        // 找Start节点
        let start_node = flow.nodes.iter()
            .find(|n| matches!(n.node_type, NodeType::Start))
            .ok_or_else(|| FlowError::InvalidConfig("缺少Start节点".into()))?;

        let start_time = Instant::now();
        let mut node_results = Vec::new();
        let mut current_node_id = start_node.id.clone();
        let mut max_steps = 1000;

        loop {
            if max_steps == 0 {
                return Err(FlowError::ExecutionFailed("执行步数超限，可能存在无限循环".into()));
            }
            max_steps -= 1;

            let node = flow.nodes.iter()
                .find(|n| n.id == current_node_id)
                .ok_or_else(|| FlowError::NodeNotFound(current_node_id.clone()))?
                .clone();

            let result = execute_node(&node, &variables).await;
            let is_error = result.error.is_some();
            let output_data = result.output.clone();
            let node_id_for_log = node.id.clone();
            let _node_name = node.name.clone();
            let _node_type_str = format!("{:?}", node.node_type);
            let _duration = result.duration_ms;

            node_results.push(result.clone());

            if is_error {
                return Ok(FlowExecutionResult {
                    flow_id: flow.id.clone(),
                    flow_name: flow.name.clone(),
                    success: false,
                    node_results,
                    output: None,
                    variables,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    error: result.error,
                });
            }

            // 更新变量
            if let Some(ref out) = output_data {
                variables.insert(format!("node_{}", node_id_for_log), out.clone());
                variables.insert("last_output".to_string(), out.clone());
            }

            // End节点
            if matches!(node.node_type, NodeType::End) {
                return Ok(FlowExecutionResult {
                    flow_id: flow.id.clone(),
                    flow_name: flow.name.clone(),
                    success: true,
                    node_results,
                    output: output_data,
                    variables,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    error: None,
                });
            }

            // 条件节点
            if matches!(node.node_type, NodeType::Condition) {
                let condition = node.config.get("condition")
                    .and_then(|c| c.as_str())
                    .unwrap_or("true");
                let should_take_true = evaluate_condition(condition, &variables);
                let condition_match = if should_take_true { "true" } else { "false" };

                let next_edge = flow.edges.iter()
                    .find(|e| e.source == node_id_for_log &&
                        (e.condition.as_deref() == Some(condition_match) || e.condition.is_none()))
                    .or_else(|| flow.edges.iter()
                        .find(|e| e.source == node_id_for_log))
                    .cloned();

                if let Some(edge) = next_edge {
                    current_node_id = edge.target;
                } else {
                    break;
                }
                continue;
            }

            // 普通节点：找下一个节点
            let next_edge = flow.edges.iter()
                .find(|e| e.source == node_id_for_log)
                .cloned();

            if let Some(edge) = next_edge {
                current_node_id = edge.target;
            } else {
                break;
            }
        }

        Ok(FlowExecutionResult {
            flow_id: flow.id.clone(),
            flow_name: flow.name.clone(),
            success: true,
            node_results,
            output: variables.get("last_output").cloned(),
            variables,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            error: None,
        })
    }
}

async fn execute_node(node: &FlowNode, variables: &HashMap<String, serde_json::Value>) -> NodeExecutionResult {
    let start = Instant::now();
    let input_data = resolve_template(node.config.get("input"), variables);
    
    match &node.node_type {
        NodeType::Start => {
            NodeExecutionResult {
                node_id: node.id.clone(),
                node_name: node.name.clone(),
                node_type: "start".into(),
                status: "success".into(),
                input: None,
                output: Some(serde_json::json!({"message": "Flow started", "variables_count": variables.len()})),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        NodeType::End => {
            NodeExecutionResult {
                node_id: node.id.clone(),
                node_name: node.name.clone(),
                node_type: "end".into(),
                status: "success".into(),
                input: input_data.clone(),
                output: Some(variables.get("last_output").cloned().unwrap_or(serde_json::json!({"status": "completed"}))),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        NodeType::DataInput => {
            let value = node.config.get("value").cloned()
                .or_else(|| input_data.clone())
                .unwrap_or(serde_json::Value::Null);
            NodeExecutionResult {
                node_id: node.id.clone(),
                node_name: node.name.clone(),
                node_type: "data_input".into(),
                status: "success".into(),
                input: None,
                output: Some(value),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        NodeType::DataOutput => {
            NodeExecutionResult {
                node_id: node.id.clone(),
                node_name: node.name.clone(),
                node_type: "data_output".into(),
                status: "success".into(),
                input: input_data.clone(),
                output: input_data,
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        NodeType::Transform => {
            let template = node.config.get("template")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            let result = apply_template(template, variables);
            NodeExecutionResult {
                node_id: node.id.clone(),
                node_name: node.name.clone(),
                node_type: "transform".into(),
                status: "success".into(),
                input: input_data,
                output: Some(serde_json::json!({"transformed": result})),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        NodeType::Condition => {
            let condition = node.config.get("condition")
                .and_then(|c| c.as_str())
                .unwrap_or("true");
            let result = evaluate_condition(condition, variables);
            NodeExecutionResult {
                node_id: node.id.clone(),
                node_name: node.name.clone(),
                node_type: "condition".into(),
                status: "success".into(),
                input: input_data,
                output: Some(serde_json::json!({"condition": condition, "result": result, "branch": if result { "true" } else { "false" }})),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        NodeType::HttpRequest => {
            // HTTP请求节点 - 通过事件发送给runtime处理
            // 这里只是框架，实际执行由runtime层注入
            let url = node.config.get("url").and_then(|u| u.as_str()).unwrap_or("");
            let method = node.config.get("method").and_then(|m| m.as_str()).unwrap_or("GET");
            NodeExecutionResult {
                node_id: node.id.clone(),
                node_name: node.name.clone(),
                node_type: "http_request".into(),
                status: "pending".into(),
                input: Some(serde_json::json!({"url": url, "method": method})),
                output: None,
                error: Some("HTTP请求需要在runtime层执行".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        NodeType::LLM => {
            let prompt = node.config.get("prompt").and_then(|p| p.as_str())
                .map(|p| apply_template(p, variables))
                .unwrap_or("".into());
            NodeExecutionResult {
                node_id: node.id.clone(),
                node_name: node.name.clone(),
                node_type: "llm".into(),
                status: "pending".into(),
                input: Some(serde_json::json!({"prompt": prompt})),
                output: None,
                error: Some("LLM调用需要在runtime层执行".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        NodeType::Browser => {
            let url = node.config.get("url").and_then(|u| u.as_str()).unwrap_or("");
            let action = node.config.get("action").and_then(|a| a.as_str()).unwrap_or("navigate");
            NodeExecutionResult {
                node_id: node.id.clone(),
                node_name: node.name.clone(),
                node_type: "browser".into(),
                status: "pending".into(),
                input: Some(serde_json::json!({"url": url, "action": action})),
                output: None,
                error: Some("浏览器操作需要在runtime层执行".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        NodeType::Operator => {
            let op_id = node.config.get("operator").and_then(|o| o.as_str()).unwrap_or("");
            NodeExecutionResult {
                node_id: node.id.clone(),
                node_name: node.name.clone(),
                node_type: "operator".into(),
                status: "pending".into(),
                input: Some(serde_json::json!({"operator_id": op_id})),
                output: None,
                error: Some("算子执行需要在runtime层执行".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        NodeType::Script => {
            let code = node.config.get("code").and_then(|c| c.as_str()).unwrap_or("");
            // 简化脚本执行 - 支持基本表达式
            let result = execute_script_sandbox(code, variables);
            match result {
                Ok(output) => NodeExecutionResult {
                    node_id: node.id.clone(),
                    node_name: node.name.clone(),
                    node_type: "script".into(),
                    status: "success".into(),
                    input: input_data,
                    output: Some(output),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                },
                Err(e) => NodeExecutionResult {
                    node_id: node.id.clone(),
                    node_name: node.name.clone(),
                    node_type: "script".into(),
                    status: "error".into(),
                    input: input_data,
                    output: None,
                    error: Some(e),
                    duration_ms: start.elapsed().as_millis() as u64,
                },
            }
        }
        NodeType::Parallel => {
            NodeExecutionResult {
                node_id: node.id.clone(),
                node_name: node.name.clone(),
                node_type: "parallel".into(),
                status: "success".into(),
                input: input_data,
                output: Some(serde_json::json!({"parallel": true, "branches": node.config.get("branches").cloned().unwrap_or_default()})),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        // 兜底：未显式处理的节点类型按"透传/算子"语义执行
        _ => {
            NodeExecutionResult {
                node_id: node.id.clone(),
                node_name: node.name.clone(),
                node_type: format!("{:?}", node.node_type).to_lowercase(),
                status: "success".into(),
                input: input_data,
                output: Some(serde_json::json!({"executed": node.node_type, "config": node.config})),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
    }
}

fn detect_cycle(adj: &HashMap<String, Vec<String>>) -> bool {
    let mut visited = std::collections::HashSet::new();
    let mut stack = std::collections::HashSet::new();
    
    for node in adj.keys() {
        if !visited.contains(node)
            && dfs_cycle(node, adj, &mut visited, &mut stack) {
                return true;
            }
    }
    false
}

fn dfs_cycle(
    node: &str,
    adj: &HashMap<String, Vec<String>>,
    visited: &mut std::collections::HashSet<String>,
    stack: &mut std::collections::HashSet<String>,
) -> bool {
    visited.insert(node.to_string());
    stack.insert(node.to_string());
    
    if let Some(neighbors) = adj.get(node) {
        for neighbor in neighbors {
            if stack.contains(neighbor) {
                return true;
            }
            if !visited.contains(neighbor)
                && dfs_cycle(neighbor, adj, visited, stack) {
                    return true;
                }
        }
    }
    stack.remove(node);
    false
}

fn resolve_template(config: Option<&serde_json::Value>, variables: &HashMap<String, serde_json::Value>) -> Option<serde_json::Value> {
    config.map(|c| {
        let s = serde_json::to_string(c).unwrap_or_default();
        let resolved = apply_template(&s, variables);
        serde_json::from_str(&resolved).unwrap_or_else(|_| c.clone())
    })
}

pub fn apply_template(template: &str, variables: &HashMap<String, serde_json::Value>) -> String {
    let mut result = template.to_string();
    for (key, value) in variables {
        let placeholder = format!("{{{{{}}}}}", key);
        let val_str = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            other => serde_json::to_string(other).unwrap_or_default(),
        };
        result = result.replace(&placeholder, &val_str);
    }
    result
}

pub fn evaluate_condition(condition: &str, variables: &HashMap<String, serde_json::Value>) -> bool {
    let resolved = apply_template(condition, variables);
    let lower = resolved.to_lowercase();
    
    // 布尔值
    if lower == "true" || lower == "yes" || lower == "1" { return true; }
    if lower == "false" || lower == "no" || lower == "0" { return false; }
    
    // 比较操作
    if let Some(parts) = parse_comparison(&resolved) {
        let left = evaluate_value(&parts.0, variables);
        let right = evaluate_value(&parts.2, variables);
        match parts.1.as_str() {
            "==" | "=" => left == right,
            "!=" | "<>" => left != right,
            ">" => parse_number(&left) > parse_number(&right),
            "<" => parse_number(&left) < parse_number(&right),
            ">=" => parse_number(&left) >= parse_number(&right),
            "<=" => parse_number(&left) <= parse_number(&right),
            _ => false,
        }
    } else {
        // 检查是否是存在性检查
        if resolved.contains("{{") { return true; } // 还有未解析变量，默认true
        !resolved.is_empty()
    }
}

fn parse_comparison(expr: &str) -> Option<(String, String, String)> {
    let operators = ["==", "!=", ">=", "<=", "<>", ">", "<", "="];
    for op in operators {
        if let Some(parts) = expr.split_once(op) {
            return Some((parts.0.trim().to_string(), op.to_string(), parts.1.trim().to_string()));
        }
    }
    None
}

fn evaluate_value(expr: &str, variables: &HashMap<String, serde_json::Value>) -> String {
    if expr.starts_with('"') && expr.ends_with('"') {
        return expr[1..expr.len()-1].to_string();
    }
    if expr.starts_with('\'') && expr.ends_with('\'') {
        return expr[1..expr.len()-1].to_string();
    }
    if let Some(val) = variables.get(expr) {
        return serde_json::to_string(val).unwrap_or(expr.to_string());
    }
    expr.to_string()
}

fn parse_number(s: &str) -> f64 {
    s.trim().parse::<f64>().unwrap_or(0.0)
}

fn execute_script_sandbox(code: &str, variables: &HashMap<String, serde_json::Value>) -> Result<serde_json::Value, String> {
    let resolved = apply_template(code, variables);
    
    // 简单脚本引擎 - 支持 print, 基本数学运算
    let lines: Vec<&str> = resolved.lines().map(|l| l.trim()).filter(|l| !l.is_empty() && !l.starts_with("//")).collect();
    let mut output = String::new();
    let mut local_vars = variables.clone();
    
    for line in lines {
        if line.starts_with("print(") && line.ends_with(')') {
            let expr = &line[6..line.len()-1];
            let val = evaluate_script_expr(expr, &local_vars)?;
            output.push_str(&format!("{}\n", val));
        } else if let Some(assignment) = line.split_once('=') {
            let var_name = assignment.0.trim();
            let expr = assignment.1.trim();
            let val = evaluate_script_expr(expr, &local_vars)?;
            local_vars.insert(var_name.to_string(), serde_json::Value::String(val));
        }
    }
    
    Ok(serde_json::json!({"output": output.trim(), "variables": local_vars}))
}

fn evaluate_script_expr(expr: &str, variables: &HashMap<String, serde_json::Value>) -> Result<String, String> {
    let trimmed = expr.trim();
    
    // 字符串字面量
    if (trimmed.starts_with('"') && trimmed.ends_with('"')) || 
       (trimmed.starts_with('\'') && trimmed.ends_with('\'')) {
        return Ok(trimmed[1..trimmed.len()-1].to_string());
    }
    
    // 数字
    if let Ok(n) = trimmed.parse::<f64>() {
        return Ok(if n.fract() == 0.0 { format!("{}", n as i64) } else { format!("{}", n) });
    }
    
    // 变量
    if let Some(val) = variables.get(trimmed) {
        return Ok(val.as_str().unwrap_or(&val.to_string()).to_string());
    }
    
    // 数学运算 (简单支持)
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

impl Default for FlowEngine {
    fn default() -> Self { Self::new() }
}

// 预置流程图模板
pub fn create_default_templates() -> Vec<FlowDefinition> {
    let now = chrono::Utc::now();
    vec![
        FlowDefinition {
            id: "template-chat".into(),
            name: "AI对话处理流程".into(),
            description: "接收用户消息，通过LLM处理并返回响应".into(),
            nodes: vec![
                FlowNode { id: "n1".into(), node_type: NodeType::Start, name: "开始".into(), config: serde_json::json!({}), position: None },
                FlowNode { id: "n2".into(), node_type: NodeType::LLM, name: "AI处理".into(), config: serde_json::json!({"prompt": "{{user_message}}", "model": "gpt-3.5-turbo"}), position: None },
                FlowNode { id: "n3".into(), node_type: NodeType::End, name: "结束".into(), config: serde_json::json!({}), position: None },
            ],
            edges: vec![
                FlowEdge { id: "e1".into(), source: "n1".into(), target: "n2".into(), condition: None },
                FlowEdge { id: "e2".into(), source: "n2".into(), target: "n3".into(), condition: None },
            ],
            variables: HashMap::new(),
            created_at: now,
            updated_at: now,
        },
        FlowDefinition {
            id: "template-web-search".into(),
            name: "网页搜索流程".into(),
            description: "自动搜索关键词并提取结果".into(),
            nodes: vec![
                FlowNode { id: "n1".into(), node_type: NodeType::Start, name: "开始".into(), config: serde_json::json!({}), position: None },
                FlowNode { id: "n2".into(), node_type: NodeType::Browser, name: "搜索".into(), config: serde_json::json!({"action": "search", "query": "{{keyword}}"}), position: None },
                FlowNode { id: "n3".into(), node_type: NodeType::Transform, name: "格式化".into(), config: serde_json::json!({"template": "搜索结果: {{last_output}}"}), position: None },
                FlowNode { id: "n4".into(), node_type: NodeType::End, name: "结束".into(), config: serde_json::json!({}), position: None },
            ],
            edges: vec![
                FlowEdge { id: "e1".into(), source: "n1".into(), target: "n2".into(), condition: None },
                FlowEdge { id: "e2".into(), source: "n2".into(), target: "n3".into(), condition: None },
                FlowEdge { id: "e3".into(), source: "n3".into(), target: "n4".into(), condition: None },
            ],
            variables: HashMap::new(),
            created_at: now,
            updated_at: now,
        },
        FlowDefinition {
            id: "template-data-pipeline".into(),
            name: "数据处理管道".into(),
            description: "从输入到输出的数据处理流程".into(),
            nodes: vec![
                FlowNode { id: "n1".into(), node_type: NodeType::Start, name: "开始".into(), config: serde_json::json!({}), position: None },
                FlowNode { id: "n2".into(), node_type: NodeType::DataInput, name: "输入数据".into(), config: serde_json::json!({"value": "{{input_data}}"}), position: None },
                FlowNode { id: "n3".into(), node_type: NodeType::Transform, name: "数据转换".into(), config: serde_json::json!({"template": "处理: {{node_n2}}"}), position: None },
                FlowNode { id: "n4".into(), node_type: NodeType::Condition, name: "条件检查".into(), config: serde_json::json!({"condition": "{{input_data}} != null"}), position: None },
                FlowNode { id: "n5".into(), node_type: NodeType::DataOutput, name: "输出".into(), config: serde_json::json!({}), position: None },
                FlowNode { id: "n6".into(), node_type: NodeType::End, name: "结束".into(), config: serde_json::json!({}), position: None },
            ],
            edges: vec![
                FlowEdge { id: "e1".into(), source: "n1".into(), target: "n2".into(), condition: None },
                FlowEdge { id: "e2".into(), source: "n2".into(), target: "n3".into(), condition: None },
                FlowEdge { id: "e3".into(), source: "n3".into(), target: "n4".into(), condition: None },
                FlowEdge { id: "e4".into(), source: "n4".into(), target: "n5".into(), condition: Some("true".into()) },
                FlowEdge { id: "e5".into(), source: "n5".into(), target: "n6".into(), condition: None },
            ],
            variables: HashMap::new(),
            created_at: now,
            updated_at: now,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use chrono::Utc;

    fn def_with(nodes: Vec<FlowNode>, edges: Vec<FlowEdge>, vars: HashMap<String, serde_json::Value>) -> FlowDefinition {
        FlowDefinition {
            id: "test-flow".into(),
            name: "测试流".into(),
            description: "单测".into(),
            nodes, edges, variables: vars,
            created_at: Utc::now(), updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_apply_template_substitutes_variables() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), serde_json::json!("世界"));
        vars.insert("n".to_string(), serde_json::json!(42));
        vars.insert("flag".to_string(), serde_json::json!(true));
        assert_eq!(apply_template("hello {{name}}", &vars), "hello 世界");
        assert_eq!(apply_template("num={{n}}", &vars), "num=42");
        assert_eq!(apply_template("flag={{flag}}", &vars), "flag=true");
        // 未提供变量保留占位
        assert_eq!(apply_template("x={{missing}}", &HashMap::new()), "x={{missing}}");
    }

    #[test]
    fn test_evaluate_condition_boolean_literals() {
        let v = HashMap::new();
        assert!(evaluate_condition("true", &v));
        assert!(evaluate_condition("1", &v));
        assert!(!evaluate_condition("false", &v));
        assert!(!evaluate_condition("0", &v));
        assert!(evaluate_condition("nonempty", &v));
        assert!(!evaluate_condition("", &v));
    }

    #[test]
    fn test_evaluate_condition_comparisons() {
        let mut v = HashMap::new();
        v.insert("a".to_string(), serde_json::json!(10));
        v.insert("name".to_string(), serde_json::json!("bob"));
        assert!(evaluate_condition("{{a}} > 5", &v));
        assert!(evaluate_condition("{{a}} <= 10", &v));
        assert!(evaluate_condition("{{a}} == 10", &v));
        assert!(evaluate_condition("{{a}} != 11", &v));
        assert!(evaluate_condition("{{name}} == \"bob\"", &v));
        assert!(!evaluate_condition("{{name}} == \"alice\"", &v));
    }

    #[test]
    fn test_create_flow_sets_timestamps_and_stores() {
        let mut engine = FlowEngine::new();
        let definition = def_with(
            vec![
                FlowNode { id: "s".into(), node_type: NodeType::Start, name: "S".into(), config: serde_json::json!({}), position: None },
                FlowNode { id: "e".into(), node_type: NodeType::End, name: "E".into(), config: serde_json::json!({}), position: None },
            ],
            vec![FlowEdge { id: "se".into(), source: "s".into(), target: "e".into(), condition: None }],
            HashMap::new());
        let created = engine.create_flow(definition).unwrap();
        assert!(!created.id.is_empty());
        assert!(engine.get_flow(&created.id).is_some());
        assert_eq!(engine.list_flows().len(), 1);
        // 时间戳已设置
        assert!(created.created_at <= created.updated_at);
    }

    #[test]
    fn test_create_flow_rejects_empty_and_missing_start() {
        let mut engine = FlowEngine::new();
        let empty = def_with(vec![], vec![], HashMap::new());
        assert!(engine.create_flow(empty).is_err());
        let no_start = def_with(
            vec![FlowNode { id: "e".into(), node_type: NodeType::End, name: "E".into(), config: serde_json::json!({}), position: None }],
            vec![], HashMap::new());
        assert!(engine.create_flow(no_start).is_err());
    }

    #[test]
    fn test_update_flow_requires_existing_and_validates() {
        let mut engine = FlowEngine::new();
        let def = def_with(
            vec![
                FlowNode { id: "s".into(), node_type: NodeType::Start, name: "S".into(), config: serde_json::json!({}), position: None },
                FlowNode { id: "e".into(), node_type: NodeType::End, name: "E".into(), config: serde_json::json!({}), position: None },
            ],
            vec![FlowEdge { id: "se".into(), source: "s".into(), target: "e".into(), condition: None }],
            HashMap::new());
        let created = engine.create_flow(def.clone()).unwrap();
        let id = created.id.clone();

        // 更新不存在的 id → 报错
        let mut ghost = def.clone();
        ghost.id = "ghost".into();
        assert!(engine.update_flow(ghost).is_err());

        // 有效更新 → 成功且 updated_at 刷新
        let mut updated = def;
        updated.id = id;
        updated.name = "改名后".into();
        let before = created.updated_at;
        let res = engine.update_flow(updated).unwrap();
        assert_eq!(res.name, "改名后");
        assert!(res.updated_at >= before);

        // 更新后破坏校验（去 Start）→ 报错且原数据保持
        let mut broken = engine.get_flow(&created.id).unwrap().clone();
        broken.nodes.retain(|n| n.node_type != NodeType::Start);
        assert!(engine.update_flow(broken).is_err());
        assert_eq!(engine.get_flow(&created.id).unwrap().name, "改名后");
    }

    #[test]
    fn test_validate_flow_ok_and_errors() {
        let ok = def_with(
            vec![
                FlowNode { id: "s".into(), node_type: NodeType::Start, name: "S".into(), config: serde_json::json!({}), position: None },
                FlowNode { id: "e".into(), node_type: NodeType::End, name: "E".into(), config: serde_json::json!({}), position: None },
            ],
            vec![FlowEdge { id: "se".into(), source: "s".into(), target: "e".into(), condition: None }],
            HashMap::new());
        assert!(FlowEngine::validate_flow(&ok).is_ok());

        let no_end = def_with(
            vec![FlowNode { id: "s".into(), node_type: NodeType::Start, name: "S".into(), config: serde_json::json!({}), position: None }],
            vec![], HashMap::new());
        assert!(FlowEngine::validate_flow(&no_end).is_err());

        // 悬空边
        let dangling = def_with(
            vec![
                FlowNode { id: "s".into(), node_type: NodeType::Start, name: "S".into(), config: serde_json::json!({}), position: None },
                FlowNode { id: "e".into(), node_type: NodeType::End, name: "E".into(), config: serde_json::json!({}), position: None },
            ],
            vec![FlowEdge { id: "x".into(), source: "s".into(), target: "missing".into(), condition: None }],
            HashMap::new());
        assert!(FlowEngine::validate_flow(&dangling).is_err());
    }

    #[tokio::test]
    async fn test_execute_flow_data_transform_condition_output() {
        let nodes = vec![
            FlowNode { id: "s".into(), node_type: NodeType::Start, name: "S".into(), config: serde_json::json!({}), position: None },
            FlowNode { id: "in".into(), node_type: NodeType::DataInput, name: "IN".into(), config: serde_json::json!({"value": "hello"}), position: None },
            FlowNode { id: "tr".into(), node_type: NodeType::Transform, name: "TR".into(), config: serde_json::json!({"template": "GOT:{{node_in}}"}), position: None },
            FlowNode { id: "c".into(), node_type: NodeType::Condition, name: "C".into(), config: serde_json::json!({"condition": "true"}), position: None },
            FlowNode { id: "out".into(), node_type: NodeType::DataOutput, name: "OUT".into(), config: serde_json::json!({}), position: None },
            FlowNode { id: "e".into(), node_type: NodeType::End, name: "E".into(), config: serde_json::json!({}), position: None },
        ];
        let edges = vec![
            FlowEdge { id: "a".into(), source: "s".into(), target: "in".into(), condition: None },
            FlowEdge { id: "b".into(), source: "in".into(), target: "tr".into(), condition: None },
            FlowEdge { id: "c".into(), source: "tr".into(), target: "c".into(), condition: None },
            FlowEdge { id: "d".into(), source: "c".into(), target: "out".into(), condition: Some("true".into()) },
            FlowEdge { id: "f".into(), source: "out".into(), target: "e".into(), condition: None },
        ];
        let mut variables = HashMap::new();
        variables.insert("input_data".to_string(), serde_json::json!("hello"));
        let def = def_with(nodes, edges, variables);

        let mut engine = FlowEngine::new();
        let created = engine.create_flow(def).unwrap();
        let mut input = HashMap::new();
        input.insert("input_data".to_string(), serde_json::json!("hello"));
        let result = engine.execute_flow(&created.id, input).await.unwrap();

        assert!(result.success);
        assert!(result.output.is_some());
        // 节点结果应覆盖同步节点
        let ids: Vec<&String> = result.node_results.iter().map(|n| &n.node_id).collect();
        assert!(ids.iter().any(|id| *id == "in"));
        assert!(ids.iter().any(|id| *id == "tr"));
        assert!(ids.iter().any(|id| *id == "out"));
        assert!(ids.iter().any(|id| *id == "e"));
    }

    #[tokio::test]
    async fn test_execute_flow_script_node_succeeds() {
        let nodes = vec![
            FlowNode { id: "s".into(), node_type: NodeType::Start, name: "S".into(), config: serde_json::json!({}), position: None },
            FlowNode { id: "sc".into(), node_type: NodeType::Script, name: "SC".into(),
                config: serde_json::json!({"code": "x = 1 + 2\nprint(x)"}), position: None },
            FlowNode { id: "e".into(), node_type: NodeType::End, name: "E".into(), config: serde_json::json!({}), position: None },
        ];
        let edges = vec![
            FlowEdge { id: "a".into(), source: "s".into(), target: "sc".into(), condition: None },
            FlowEdge { id: "b".into(), source: "sc".into(), target: "e".into(), condition: None },
        ];
        let def = def_with(nodes, edges, HashMap::new());
        let mut engine = FlowEngine::new();
        let created = engine.create_flow(def).unwrap();
        let result = engine.execute_flow(&created.id, HashMap::new()).await.unwrap();
        assert!(result.success);
        let ids: Vec<&String> = result.node_results.iter().map(|n| &n.node_id).collect();
        assert!(ids.iter().any(|id| *id == "sc"));
    }

    #[tokio::test]
    async fn test_execute_flow_missing_id_errors() {
        let mut engine = FlowEngine::new();
        let r = engine.execute_flow("nope", HashMap::new()).await;
        assert!(r.is_err());
    }

    #[test]
    fn test_create_default_templates_non_empty_and_valid() {
        let templates = create_default_templates();
        assert!(!templates.is_empty());
        for t in &templates {
            assert!(FlowEngine::validate_flow(t).is_ok(), "template {} should be valid", t.id);
        }
    }
}
