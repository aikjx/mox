// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Step 3：跨回合累积「会话执行流程图」。
//!
//! 设计：每个 Hermes 会话维护一张 FlowGraph，工具调用按发生顺序累积成节点+边。
//! 后台任务周期性把图推给 mox-expert 服务做 optimize + verify（异步，不阻塞中间件）。

use crate::normalize::{dependency_edges, to_flow_node, ToolCall};
use mox_ai_flow_sdk::model::{FlowEdge, FlowGraph, FlowNode};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

struct SessionFlow {
    graph: FlowGraph,
    order: Vec<String>,
    /// 当前回合已入图的节点（用于判定同回合并发 → 无依赖边）
    current_turn: u32,
    current_turn_nodes: Vec<FlowNode>,
}

impl SessionFlow {
    fn new(session_id: &str) -> Self {
        Self {
            graph: FlowGraph::new(session_id, "hermes-session"),
            order: Vec::new(),
            current_turn: 0,
            current_turn_nodes: Vec::new(),
        }
    }

    fn record(&mut self, call: &ToolCall) {
        let node: FlowNode = to_flow_node(call);
        // 回合切换：把上一回合并发节点清空，本回合节点开始累积
        if call.turn != self.current_turn {
            self.current_turn = call.turn;
            self.current_turn_nodes.clear();
        }
        // 仅与「本回合并发节点」算依赖（跨回合在 node.id 含前序 id 时已在 dependency_edges 处理）
        let edges: Vec<FlowEdge> = dependency_edges(&self.current_turn_nodes, &node);
        self.graph.add_node(node.clone());
        for e in edges {
            self.graph.add_edge(e);
        }
        self.current_turn_nodes.push(node);
        self.order.push(call.tool_name.clone());
    }
}

/// 进程内会话流程图仓储（插件持有单一全局实例）。
#[derive(Clone)]
pub struct Recorder {
    sessions: Arc<Mutex<HashMap<String, SessionFlow>>>,
}

impl Default for Recorder {
    fn default() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Recorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次工具调用到指定会话的流程图。
    pub fn record(&self, session_id: &str, call: &ToolCall) {
        let mut map = self.sessions.lock().unwrap();
        let sf = map
            .entry(session_id.to_string())
            .or_insert_with(|| SessionFlow::new(session_id));
        sf.record(call);
    }

    /// 取出会话流程图快照（用于后台 optimize 推送）。
    pub fn snapshot(&self, session_id: &str) -> Option<FlowGraph> {
        let map = self.sessions.lock().unwrap();
        map.get(session_id).map(|sf| sf.graph.clone())
    }

    pub fn session_count(&self) -> usize {
        self.sessions.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn call(name: &str, turn: u32) -> ToolCall {
        ToolCall {
            tool_name: name.into(),
            args: json!({}),
            turn,
        }
    }

    #[test]
    fn accumulates_nodes_into_session_graph() {
        let r = Recorder::new();
        r.record("s1", &call("web1", 1));
        r.record("s1", &call("web2", 1));
        r.record("s1", &call("db.read", 2));
        let g = r.snapshot("s1").expect("session exists");
        assert_eq!(g.nodes.len(), 3, "应累积 3 个节点");
        assert_eq!(r.session_count(), 1);
    }

    #[test]
    fn separate_sessions_are_isolated() {
        let r = Recorder::new();
        r.record("sA", &call("web1", 1));
        r.record("sB", &call("web2", 1));
        assert_eq!(r.snapshot("sA").unwrap().nodes.len(), 1);
        assert_eq!(r.snapshot("sB").unwrap().nodes.len(), 1);
    }
}
