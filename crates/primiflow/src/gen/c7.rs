//! 代码骨架 · 由关联图谱自动生成（primiflow::assoc::primiflow_seed）
//! 溯源链路: R2 → F2 → B3 → A2 → T9 → C7
//! 数据设计: S3(Topology)
//! 说明: 画布状态 + 编辑后重算 ℛ̂（用户改流程图 → 系统跟着变）。
//! 规格: primiflow/SPEC.md（§3 客户旅程 / §9 风险缓解）

use flow_ai::model::{FlowEdge, FlowGraph, FlowNode};
use flow_ai::primitive::{PrimitiveState, ResourceBudget};
use crate::gen::c2::Scheduler;
use crate::gen::c2::RegularizeOutput;

/// 画布编辑操作
#[derive(Debug, Clone)]
pub enum CanvasOp {
    /// 新增节点
    AddNode(FlowNode),
    /// 删除节点（连同其边）
    RemoveNode(String),
    /// 新增边
    AddEdge(FlowEdge),
    /// 删除边
    RemoveEdge(String, String),
}

/// 画布状态：用户可编辑的 DAG + 版本历史 + 重算 ℛ̂
#[derive(Debug)]
pub struct CanvasState {
    pub graph: FlowGraph,
    /// 每次编辑前的快照（用于撤销 / 溯源回放）
    history: Vec<FlowGraph>,
    /// 当前正则化状态（重算后刷新）
    pub last_regularize: Option<RegularizeOutput>,
    scheduler: Scheduler,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self::new()
    }
}

impl CanvasState {
    pub fn new() -> Self {
        Self {
            graph: FlowGraph::new("canvas", "编辑画布"),
            history: Vec::new(),
            last_regularize: None,
            scheduler: Scheduler::new(),
        }
    }

    /// 以已有拓扑初始化画布
    pub fn from_graph(graph: FlowGraph) -> Self {
        Self {
            graph,
            history: Vec::new(),
            last_regularize: None,
            scheduler: Scheduler::new(),
        }
    }

    /// 应用一次编辑操作（先快照，再变更）
    pub fn edit_canvas(&mut self, op: CanvasOp) {
        self.history.push(self.graph.clone());
        match op {
            CanvasOp::AddNode(n) => {
                self.graph.add_node(n);
            }
            CanvasOp::RemoveNode(id) => {
                self.graph.nodes.retain(|n| n.id != id);
                self.graph.edges.retain(|e| e.from != id && e.to != id);
            }
            CanvasOp::AddEdge(e) => {
                self.graph.add_edge(e);
            }
            CanvasOp::RemoveEdge(from, to) => {
                self.graph.edges.retain(|e| !(e.from == from && e.to == to));
            }
        }
    }

    /// 撤销最近一次编辑
    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.history.pop() {
            self.graph = prev;
            true
        } else {
            false
        }
    }

    pub fn revision(&self) -> usize {
        self.history.len()
    }

    /// 编辑后重算 ℛ̂：返回合规拓扑与调整后 κ‑τ（SPEC §3 "改完系统跟着变"）
    pub fn recompute(&mut self, state: PrimitiveState, budget: ResourceBudget) -> &RegularizeOutput {
        let out = self.scheduler.regularize(self.graph.clone(), state, budget);
        self.last_regularize = Some(out);
        self.last_regularize.as_ref().unwrap()
    }

    /// 当前画布是否仍为合法 DAG
    pub fn is_acyclic(&self) -> bool {
        self.graph.topo_order().is_ok()
    }

    /// 导出当前画布为 JSON（画布即源码，导出即工程）
    pub fn export_json(&self) -> String {
        serde_json::to_string_pretty(&self.graph).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_ai::model::{NodeKind, ToolKind};
    use flow_ai::primitive::DeliveryPolicy;

    #[test]
    fn edit_adds_node_and_history() {
        let mut c = CanvasState::new();
        c.edit_canvas(CanvasOp::AddNode(FlowNode::task("a", "A", ToolKind::Http, 300)));
        assert_eq!(c.graph.nodes.len(), 1);
        assert_eq!(c.revision(), 1);
        assert!(c.undo());
        assert_eq!(c.graph.nodes.len(), 0);
    }

    #[test]
    fn remove_node_clears_its_edges() {
        let mut c = CanvasState::new();
        c.edit_canvas(CanvasOp::AddNode(FlowNode::task("a", "A", ToolKind::Http, 300)));
        c.edit_canvas(CanvasOp::AddNode(FlowNode::task("b", "B", ToolKind::Compute, 200)));
        c.edit_canvas(CanvasOp::AddEdge(FlowEdge::seq("a", "b")));
        assert_eq!(c.graph.edges.len(), 1);
        c.edit_canvas(CanvasOp::RemoveNode("a".into()));
        assert_eq!(c.graph.edges.len(), 0, "删节点应连带删除其边");
    }

    #[test]
    fn recompute_refreshes_state() {
        let mut c = CanvasState::new();
        c.edit_canvas(CanvasOp::AddNode(FlowNode::task("a", "A", ToolKind::Http, 300)));
        let state = PrimitiveState::from_policy(10.0, DeliveryPolicy::Balanced, 0.0);
        let out = c.recompute(state, ResourceBudget { total_ms: 10_000, per_pool: Default::default() });
        assert!(out.delta.abs() < 1e-6, "重算后守恒残差应≈0");
    }

    #[test]
    fn cycle_is_detected() {
        let mut c = CanvasState::new();
        c.edit_canvas(CanvasOp::AddNode(FlowNode::new("a", "A", NodeKind::Task)));
        c.edit_canvas(CanvasOp::AddNode(FlowNode::new("b", "B", NodeKind::Task)));
        c.edit_canvas(CanvasOp::AddEdge(FlowEdge::seq("a", "b")));
        c.edit_canvas(CanvasOp::AddEdge(FlowEdge::seq("b", "a")));
        assert!(!c.is_acyclic());
    }
}
