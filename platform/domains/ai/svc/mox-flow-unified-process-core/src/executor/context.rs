// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 执行上下文
//!
//! 节点执行时的环境信息，包括变量、前序输出、trace 等。

use std::collections::HashMap;

use crate::extension::ExtensionRegistry;
use crate::types::UnifiedFlowGraph;

/// 执行上下文 —— 传递给每个 NodeHandler 的环境信息
pub struct ExecutionContext<'a> {
    /// 当前变量状态
    pub variables: &'a HashMap<String, serde_json::Value>,
    /// 各节点的输出（key: node_id）
    pub previous_outputs: &'a HashMap<String, serde_json::Value>,
    /// 流程图 ID
    pub flow_id: &'a str,
    /// 流程图名称
    pub flow_name: &'a str,
    /// 执行追踪 ID
    pub trace_id: &'a str,
    /// 扩展注册表（可获取其他扩展能力）
    pub extensions: &'a ExtensionRegistry,
}

impl<'a> ExecutionContext<'a> {
    pub fn new(
        graph: &'a UnifiedFlowGraph,
        variables: &'a HashMap<String, serde_json::Value>,
        previous_outputs: &'a HashMap<String, serde_json::Value>,
        trace_id: &'a str,
        extensions: &'a ExtensionRegistry,
    ) -> Self {
        Self {
            variables,
            previous_outputs,
            flow_id: &graph.id,
            flow_name: &graph.name,
            trace_id,
            extensions,
        }
    }

    /// 获取变量值
    pub fn get_var(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name)
    }

    /// 获取某个节点的输出
    pub fn get_node_output(&self, node_id: &str) -> Option<&serde_json::Value> {
        self.previous_outputs.get(node_id)
    }

    /// 获取 last_output（上一个节点的输出）
    pub fn last_output(&self) -> Option<&serde_json::Value> {
        self.variables.get("last_output")
    }
}
