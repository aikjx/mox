// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 算子商城：DSL 转换
//!
//! 打通「商城资产 → 内核可执行」的转换链路：
//!
//! 1. **流程图 JSON → FlowDefinition DSL**
//!    商城包内的结构化流程图（`nodes/edges`）投影为内核
//!    `mox_ai_agent_svc::flow_engine::FlowDefinition`（节点类型归一化、坐标与备注进入 config）。
//! 2. **FlowDefinition DSL → BusinessWorkflow 自动生成**
//!    把 FlowDefinition 映射为 `mox_ai_agent_svc::types::BusinessWorkflow`：
//!    Start/End/Condition/Parallel 语义保留，LLM→AiTask、Script→Script，
//!    其余业务节点→Operator；条件分支自动推导 true/false 路径。
//! 3. **前端预览**
//!    `GET /api/market/:id/dsl/preview` 返回自包含 HTML 页面，
//!    同时展示生成的 DSL JSON、Workflow JSON 与可读代码（供前端 iframe/新窗口预览）。
//!
//! 转换保持幂等：同一包多次转换结果一致（时间戳字段除外）。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::collections::HashMap;

use crate::market::{load_package, MarketState, OperatorPackage};
use crate::market_migration::now_rfc3339;
use mox_api_protocol::{ApiResponse, api_ok, api_error, api_ok_empty};

// 复用内核类型
use mox_ai_agent_svc::flow_engine::{
    FlowDefinition, FlowEdge as AFlowEdge, FlowNode as AFlowNode, NodeType, Position,
};
use mox_ai_agent_svc::{
    BusinessWorkflow, MergeStrategy, NodePosition, WorkflowEdge, WorkflowNode, WorkflowNodeConfig,
    WorkflowNodeType,
};

// ========== 1) 流程图 JSON → FlowDefinition ==========

/// 把商城节点类型字符串归一化为内核 NodeType
pub fn map_node_type(s: &str) -> NodeType {
    match s.to_lowercase().as_str() {
        "start" => NodeType::Start,
        "end" => NodeType::End,
        "decision" | "condition" | "guard" => NodeType::Decision,
        "parallel" | "branch" | "fork" => NodeType::Parallel,
        "llm" | "ai" => NodeType::LLM,
        "browser" => NodeType::Browser,
        "http" | "http_request" | "api" | "rest" => NodeType::HttpRequest,
        "operator" | "op" => NodeType::Operator,
        "script" => NodeType::Script,
        "transform" | "mapping" => NodeType::Transform,
        "io" | "input" | "data_input" => NodeType::DataInput,
        "output" | "data_output" => NodeType::DataOutput,
        "event" => NodeType::Event,
        _ => NodeType::Task,
    }
}

fn parse_ts(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

/// 流程图 JSON（nodes/edges）→ FlowDefinition DSL。
/// `requirement` 会写入 `variables["requirement"]`，便于执行层消费。
pub fn flowchart_to_definition(
    id: &str,
    name: &str,
    requirement: &str,
    nodes: &[crate::market::FlowNode],
    edges: &[crate::market::FlowEdge],
) -> FlowDefinition {
    let now = now_rfc3339();
    let a_nodes: Vec<AFlowNode> = nodes
        .iter()
        .map(|n| {
            let node_type = map_node_type(&n.node_type);
            let config = serde_json::json!({
                "note": n.note,
                "label": n.label,
                "x": n.x,
                "y": n.y,
            });
            AFlowNode {
                id: n.id.clone(),
                node_type,
                name: if n.label.is_empty() {
                    n.id.clone()
                } else {
                    n.label.clone()
                },
                config,
                position: Some(Position { x: n.x, y: n.y }),
            }
        })
        .collect();
    let a_edges: Vec<AFlowEdge> = edges
        .iter()
        .map(|e| AFlowEdge {
            id: e.id.clone(),
            source: e.source.clone(),
            target: e.target.clone(),
            condition: if e.label.is_empty() {
                None
            } else {
                Some(e.label.clone())
            },
        })
        .collect();
    let mut variables = HashMap::new();
    variables.insert(
        "requirement".to_string(),
        serde_json::Value::String(requirement.to_string()),
    );
    FlowDefinition {
        id: id.to_string(),
        name: name.to_string(),
        description: requirement.to_string(),
        nodes: a_nodes,
        edges: a_edges,
        variables,
        created_at: parse_ts(&now),
        updated_at: parse_ts(&now),
    }
}

/// 算子包 → FlowDefinition DSL
pub fn package_to_flow_definition(pkg: &OperatorPackage) -> FlowDefinition {
    flowchart_to_definition(&pkg.id, &pkg.name, &pkg.requirement, &pkg.nodes, &pkg.edges)
}

// ========== 2) FlowDefinition → BusinessWorkflow ==========

fn outgoing_edges<'a>(fd: &'a FlowDefinition, node_id: &str) -> Vec<&'a AFlowEdge> {
    fd.edges.iter().filter(|e| e.source == node_id).collect()
}

/// 节点类型 → 工作流节点类型与配置
fn to_workflow_node(
    fd: &FlowDefinition,
    node: &AFlowNode,
) -> (WorkflowNodeType, WorkflowNodeConfig) {
    let outs = outgoing_edges(fd, &node.id);
    match node.node_type {
        NodeType::Start => (WorkflowNodeType::Start, WorkflowNodeConfig::Start),
        NodeType::End => (WorkflowNodeType::End, WorkflowNodeConfig::End),
        NodeType::Decision | NodeType::Guard => {
            let mut true_path = String::new();
            let mut false_path = String::new();
            let mut expression = String::new();
            for (i, e) in outs.iter().enumerate() {
                match i {
                    0 => {
                        true_path = e.target.clone();
                        expression = e.condition.clone().unwrap_or_else(|| "true".to_string());
                    }
                    1 => false_path = e.target.clone(),
                    _ => {}
                }
            }
            (
                WorkflowNodeType::Condition,
                WorkflowNodeConfig::Condition {
                    expression,
                    true_path,
                    false_path,
                },
            )
        }
        NodeType::Parallel => (
            WorkflowNodeType::Parallel,
            WorkflowNodeConfig::Parallel {
                branches: outs.iter().map(|e| e.target.clone()).collect(),
                merge_strategy: MergeStrategy::AllComplete,
            },
        ),
        NodeType::LLM => {
            let prompt = node
                .config
                .get("note")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (
                WorkflowNodeType::AiTask,
                WorkflowNodeConfig::AiTask {
                    task_type: "llm".to_string(),
                    prompt,
                },
            )
        }
        NodeType::Script | NodeType::Transform => {
            let code = node
                .config
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (
                WorkflowNodeType::Script,
                WorkflowNodeConfig::Script {
                    language: "javascript".to_string(),
                    code,
                },
            )
        }
        NodeType::Browser => (
            WorkflowNodeType::PluginCall,
            WorkflowNodeConfig::PluginCall {
                plugin_id: "browser_automation".to_string(),
                method: "navigate".to_string(),
                parameters: node.config.clone(),
            },
        ),
        _ => {
            // Operator / Task / Event / HttpRequest / DataInput / DataOutput
            let operator_id = match node.node_type {
                NodeType::HttpRequest => "http_request".to_string(),
                NodeType::DataInput => "data_input".to_string(),
                NodeType::DataOutput => "data_output".to_string(),
                NodeType::Event => "event".to_string(),
                NodeType::Operator => node
                    .config
                    .get("operator_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| node.id.clone()),
                _ => node.id.clone(),
            };
            (
                WorkflowNodeType::Operator,
                WorkflowNodeConfig::Operator {
                    operator_id,
                    parameters: node
                        .config
                        .as_object()
                        .map(|m| m.clone().into_iter().collect())
                        .unwrap_or_default(),
                },
            )
        }
    }
}

/// FlowDefinition DSL → BusinessWorkflow 自动生成
pub fn flow_definition_to_business_workflow(fd: &FlowDefinition) -> BusinessWorkflow {
    let mut wf_nodes = Vec::new();
    let mut start_node_id = String::new();
    for node in &fd.nodes {
        let (nt, config) = to_workflow_node(fd, node);
        if matches!(nt, WorkflowNodeType::Start) && start_node_id.is_empty() {
            start_node_id = node.id.clone();
        }
        wf_nodes.push(WorkflowNode {
            id: node.id.clone(),
            node_type: nt,
            name: node.name.clone(),
            config,
            position: node
                .position
                .clone()
                .map(|p| NodePosition { x: p.x, y: p.y }),
        });
    }
    if start_node_id.is_empty() {
        // 无 Start 节点：取入度为 0 的首个节点，退而求其次取第一个
        let has_incoming: Vec<&str> = fd.edges.iter().map(|e| e.target.as_str()).collect();
        start_node_id = fd
            .nodes
            .iter()
            .find(|n| !has_incoming.contains(&n.id.as_str()))
            .or_else(|| fd.nodes.first())
            .map(|n| n.id.clone())
            .unwrap_or_default();
    }
    let edges: Vec<WorkflowEdge> = fd
        .edges
        .iter()
        .map(|e| WorkflowEdge {
            id: e.id.clone(),
            source: e.source.clone(),
            target: e.target.clone(),
            condition: e.condition.clone(),
        })
        .collect();
    BusinessWorkflow {
        id: fd.id.clone(),
        name: fd.name.clone(),
        description: fd.description.clone(),
        nodes: wf_nodes,
        edges,
        variables: fd.variables.clone(),
        start_node_id,
        created_at: chrono::Utc::now(),
    }
}

/// 算子包 → BusinessWorkflow（完整链路）
pub fn package_to_business_workflow(pkg: &OperatorPackage) -> BusinessWorkflow {
    flow_definition_to_business_workflow(&package_to_flow_definition(pkg))
}

// ========== 3) 代码生成（前端预览用）==========

/// 生成可读的 JS 风格工作流定义代码（供前端预览展示）
pub fn generate_workflow_code(wf: &BusinessWorkflow) -> String {
    let mut out = String::new();
    out.push_str(&format!("// 自动生成: BusinessWorkflow (id: {})\n", wf.id));
    out.push_str(&format!(
        "const workflow = {{\n  id: {:?},\n  name: {:?},\n  start: {:?},\n  steps: [\n",
        wf.id, wf.name, wf.start_node_id
    ));
    for n in &wf.nodes {
        let t = serde_json::to_string(&n.node_type).unwrap_or_default();
        let brief = match &n.config {
            WorkflowNodeConfig::Condition { expression, .. } => {
                format!(", expression: {:?}", expression)
            }
            WorkflowNodeConfig::Operator { operator_id, .. } => {
                format!(", operator: {:?}", operator_id)
            }
            WorkflowNodeConfig::AiTask { task_type, .. } => format!(", task: {:?}", task_type),
            _ => String::new(),
        };
        out.push_str(&format!(
            "    {{ id: {:?}, type: {}, name: {:?}{} }},\n",
            n.id, t, n.name, brief
        ));
    }
    out.push_str("  ],\n  transitions: [\n");
    for e in &wf.edges {
        let cond = e.condition.as_deref().unwrap_or("");
        out.push_str(&format!(
            "    {{ from: {:?}, to: {:?}, when: {:?} }},\n",
            e.source, e.target, cond
        ));
    }
    out.push_str("  ],\n};\n");
    out
}

// ========== 4) 预览 HTML ==========

/// 自包含预览页：展示 DSL JSON + Workflow JSON + 生成代码
pub fn preview_html(dsl: &FlowDefinition, wf: &BusinessWorkflow, code: &str) -> String {
    let dsl_json = serde_json::to_string_pretty(dsl).unwrap_or_default();
    let wf_json = serde_json::to_string_pretty(wf).unwrap_or_default();
    let dsl_esc = html_escape(&dsl_json);
    let wf_esc = html_escape(&wf_json);
    let code_esc = html_escape(code);
    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<title>算子包 DSL 预览 — {id}</title>
<style>
  body {{ font-family: "Segoe UI", "Microsoft YaHei", sans-serif; margin: 24px; background: #0f172a; color: #e2e8f0; }}
  h1 {{ font-size: 18px; }}
  .tabs {{ display: flex; gap: 8px; margin: 16px 0; }}
  .tabs button {{ background: #1e293b; color: #e2e8f0; border: 1px solid #334155; border-radius: 6px; padding: 6px 14px; cursor: pointer; }}
  .tabs button.active {{ background: #2563eb; border-color: #2563eb; }}
  pre {{ background: #1e293b; border: 1px solid #334155; border-radius: 8px; padding: 16px; overflow: auto; max-height: 70vh; font-size: 12.5px; line-height: 1.5; }}
  .meta {{ color: #94a3b8; font-size: 13px; }}
</style>
</head>
<body>
<h1>算子包 DSL / Workflow 预览</h1>
<div class="meta">id: {id} · name: {name} · 由商城 /dsl/preview 端点生成</div>
<div class="tabs">
  <button onclick="show('dsl')" id="tb-dsl" class="active">FlowDefinition DSL</button>
  <button onclick="show('wf')" id="tb-wf">BusinessWorkflow JSON</button>
  <button onclick="show('code')" id="tb-code">生成代码</button>
</div>
<pre id="pane-dsl">{dsl_esc}</pre>
<pre id="pane-wf" style="display:none">{wf_esc}</pre>
<pre id="pane-code" style="display:none">{code_esc}</pre>
<script>
function show(k) {{
  ['dsl','wf','code'].forEach(x => {{
    document.getElementById('pane-'+x).style.display = x===k ? '' : 'none';
    document.getElementById('tb-'+x).classList.toggle('active', x===k);
  }});
}}
</script>
</body>
</html>"#,
        id = html_escape(&dsl.id),
        name = html_escape(&dsl.name),
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ========== 请求体 ==========

/// POST /:id/convert 请求体：任意流程图 JSON
#[derive(Debug, Deserialize)]
pub struct ConvertRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub requirement: String,
    #[serde(default)]
    pub nodes: Vec<crate::market::FlowNode>,
    #[serde(default)]
    pub edges: Vec<crate::market::FlowEdge>,
}

// ========== 路由 ==========

/// DSL 转换路由：挂载到 /api/market 下
pub fn dsl_routes() -> Router<MarketState> {
    Router::new()
        .route("/:id/dsl", get(dsl_handler))
        .route("/:id/workflow", get(workflow_handler))
        .route("/:id/dsl/preview", get(dsl_preview_handler))
        .route("/:id/convert", post(convert_handler))
}

/// GET /:id/dsl —— 返回 FlowDefinition DSL
async fn dsl_handler(
    State(_s): State<MarketState>,
    Path(id): Path<String>,
) -> ApiResponse<serde_json::Value> {
    match load_package(&id) {
        Ok(pkg) => {
            let dsl = package_to_flow_definition(&pkg);
            api_ok(serde_json::json!({ "success": true, "dsl": dsl, "schema_version": "2026.1" }))
        }
        Err(e) => api_error(500, e),
    }
}

/// GET /:id/workflow —— 返回自动生成的 BusinessWorkflow + 代码
async fn workflow_handler(
    State(_s): State<MarketState>,
    Path(id): Path<String>,
) -> ApiResponse<serde_json::Value> {
    match load_package(&id) {
        Ok(pkg) => {
            let wf = package_to_business_workflow(&pkg);
            let code = generate_workflow_code(&wf);
            api_ok(serde_json::json!({ "success": true, "workflow": wf, "code": code }))
        }
        Err(e) => api_error(500, e),
    }
}

/// GET /:id/dsl/preview —— 前端预览页（HTML）
async fn dsl_preview_handler(
    State(_s): State<MarketState>,
    Path(id): Path<String>,
) -> (StatusCode, Html<String>) {
    match load_package(&id) {
        Ok(pkg) => {
            let dsl = package_to_flow_definition(&pkg);
            let wf = package_to_business_workflow(&pkg);
            let code = generate_workflow_code(&wf);
            (StatusCode::OK, Html(preview_html(&dsl, &wf, &code)))
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            Html(format!("<h1>算子包不存在</h1><p>{}</p>", html_escape(&e))),
        ),
    }
}

/// POST /:id/convert —— 编辑器内实时转换（流程图 JSON → DSL + Workflow + 代码）
async fn convert_handler(
    State(_s): State<MarketState>,
    Path(id): Path<String>,
    Json(req): Json<ConvertRequest>,
) -> ApiResponse<serde_json::Value> {
    let name = if req.name.is_empty() {
        format!("convert-{}", id)
    } else {
        req.name.clone()
    };
    let dsl = flowchart_to_definition(&id, &name, &req.requirement, &req.nodes, &req.edges);
    let wf = flow_definition_to_business_workflow(&dsl);
    let code = generate_workflow_code(&wf);
    api_ok(serde_json::json!({
        "success": true,
        "dsl": dsl,
        "workflow": wf,
        "code": code,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::{FlowEdge, FlowNode};

    fn sample_pkg() -> OperatorPackage {
        OperatorPackage {
            id: "flow-1".to_string(),
            name: "订单处理".to_string(),
            category: "业务".to_string(),
            author: "tester".to_string(),
            version: "1.0.0".to_string(),
            summary: "测试".to_string(),
            requirement: "处理订单全流程".to_string(),
            nodes: vec![
                FlowNode {
                    id: "n1".into(),
                    label: "开始".into(),
                    node_type: "start".into(),
                    x: 0.0,
                    y: 0.0,
                    note: "".into(),
                },
                FlowNode {
                    id: "n2".into(),
                    label: "校验".into(),
                    node_type: "decision".into(),
                    x: 0.0,
                    y: 100.0,
                    note: "检查库存".into(),
                },
                FlowNode {
                    id: "n3".into(),
                    label: "发货".into(),
                    node_type: "process".into(),
                    x: 0.0,
                    y: 200.0,
                    note: "".into(),
                },
                FlowNode {
                    id: "n4".into(),
                    label: "结束".into(),
                    node_type: "end".into(),
                    x: 0.0,
                    y: 300.0,
                    note: "".into(),
                },
            ],
            edges: vec![
                FlowEdge {
                    id: "e1".into(),
                    source: "n1".into(),
                    target: "n2".into(),
                    label: "".into(),
                },
                FlowEdge {
                    id: "e2".into(),
                    source: "n2".into(),
                    target: "n3".into(),
                    label: "有货".into(),
                },
                FlowEdge {
                    id: "e3".into(),
                    source: "n2".into(),
                    target: "n4".into(),
                    label: "缺货".into(),
                },
                FlowEdge {
                    id: "e4".into(),
                    source: "n3".into(),
                    target: "n4".into(),
                    label: "".into(),
                },
            ],
            features: vec![],
            tags: vec![],
            created_at: now_rfc3339(),
            updated_at: now_rfc3339(),
            clone_count: 0,
            forked_from: None,
            tenant: "default".to_string(),
            tenant_id: "default".to_string(),
            created_by: "tester".to_string(),
            permissions: vec![],
            ..Default::default()
        }
    }

    #[test]
    fn convert_chain_preserves_semantics() {
        let pkg = sample_pkg();
        let dsl = package_to_flow_definition(&pkg);
        assert_eq!(dsl.nodes.len(), 4);
        assert_eq!(dsl.edges.len(), 4);
        assert!(dsl
            .nodes
            .iter()
            .any(|n| matches!(n.node_type, NodeType::Start)));
        assert!(dsl
            .nodes
            .iter()
            .any(|n| matches!(n.node_type, NodeType::Decision)));
        assert_eq!(dsl.variables["requirement"], "处理订单全流程");

        let wf = flow_definition_to_business_workflow(&dsl);
        assert_eq!(wf.start_node_id, "n1");
        assert_eq!(wf.nodes.len(), 4);
        assert_eq!(wf.edges.len(), 4);
        // 决策节点生成 Condition 配置，带 true/false 路径
        let cond = wf
            .nodes
            .iter()
            .find(|n| matches!(n.node_type, WorkflowNodeType::Condition))
            .expect("应有 Condition 节点");
        match &cond.config {
            WorkflowNodeConfig::Condition {
                expression,
                true_path,
                false_path,
            } => {
                assert_eq!(expression, "有货");
                assert_eq!(true_path, "n3");
                assert_eq!(false_path, "n4");
            }
            other => panic!("期望 Condition 配置，实际 {:?}", other),
        }
        let code = generate_workflow_code(&wf);
        assert!(code.contains("workflow"));
        assert!(code.contains("start"));
    }
}
