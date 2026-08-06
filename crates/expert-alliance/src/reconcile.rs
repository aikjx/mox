//! 归一化裁决：把七位专家的观点合并为可喂给 flow-ai 的图
//!
//! 铁律：裁决器只翻译约束为 flow-ai 能识别的边/规则，**不求解**。
//! 硬约束（Blocking / 资源上限）一律落地；冲突按维度优先级仲裁。

use crate::expert::{Constraint, ExpertOpinion};
use crate::ir::Dimension;
use flow_ai::model::{
    EdgeKind, ExpertRule, FlowEdge, FlowGraph, FlowNode, NodeKind, ResourcePool, Severity,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 裁决后的归一化计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciledPlan {
    pub graph: FlowGraph,
    /// 注入的合规规则
    pub rules: Vec<ExpertRule>,
    /// 资源池上限（已应用租户配额）
    pub pools: Vec<ResourcePool>,
    /// 裁决冲突日志（同优先级无法仲裁时升级为 Blocking）
    pub conflicts: Vec<ReconcileConflict>,
    /// 采纳的算力路由
    pub model_routes: Vec<(String, flow_ai::schedule::ModelTier)>,
    /// 各专家健康分
    pub scores: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileConflict {
    pub dimension_a: Dimension,
    pub dimension_b: Dimension,
    pub nodes: Vec<String>,
    pub resolution: String,
    pub escalated: bool,
}

/// 归一化裁决
pub fn reconcile(opinions: &[ExpertOpinion], base: &FlowGraph, tenant_pools: &[ResourcePool]) -> ReconciledPlan {
    let mut graph = base.clone();
    let mut rules: Vec<ExpertRule> = Vec::new();
    let conflicts: Vec<ReconcileConflict> = Vec::new();
    let mut model_routes: Vec<(String, flow_ai::schedule::ModelTier)> = Vec::new();
    let mut scores: Vec<(String, f64)> = Vec::new();

    // 记录各专家健康分
    for o in opinions {
        scores.push((o.expert.clone(), o.score));
    }

    // 按维度优先级升序处理：高优先级（大值）后处理，天然覆盖低优先级的软约束
    let mut ordered: Vec<&ExpertOpinion> = opinions.iter().collect();
    ordered.sort_by_key(|o| o.dimension.priority());

    let mut serialized = HashSet::new();

    for o in ordered {
        for c in &o.constraints {
            match c {
                Constraint::MustOrder(ref node_ref) => {
                    if graph.node(&node_ref.from).is_some() && graph.node(&node_ref.to).is_some() {
                        graph.add_edge(FlowEdge::seq(node_ref.from.clone(), node_ref.to.clone()));
                    }
                }
                Constraint::MustGuard(target, tags) => {
                    // 在 target 前插入 Guard 节点（若尚未存在）
                    let key = format!("__guard_{}_{}", tags.join("_"), target);
                    if graph.node(&key).is_none() {
                        let mut gnode = FlowNode::new(key.clone(), format!("{} 校验", tags.join("/")), NodeKind::Guard);
                        gnode.tags.extend(tags.iter().cloned());
                        gnode.duration_ms = 5;
                        graph.add_node(gnode);
                    }
                    // 重连：target 的前驱改连 Guard，Guard 连 target
                    let preds: Vec<String> = graph
                        .edges
                        .iter()
                        .filter(|e| e.to == *target && e.kind != EdgeKind::Exception)
                        .map(|e| e.from.clone())
                        .collect();
                    graph.edges.retain(|e| !(e.to == *target && e.kind != EdgeKind::Exception));
                    for p in preds {
                        graph.add_edge(FlowEdge::seq(p, key.clone()));
                    }
                    graph.add_edge(FlowEdge::seq(key.clone(), target.clone()));
                }
                Constraint::MustSerialize(ref node_ref) => {
                    let pair = if node_ref.from <= node_ref.to {
                        (node_ref.from.clone(), node_ref.to.clone())
                    } else {
                        (node_ref.to.clone(), node_ref.from.clone())
                    };
                    if !serialized.contains(&pair)
                        && graph.node(&pair.0).is_some()
                        && graph.node(&pair.1).is_some()
                    {
                        // 物化为 Mutex 硬约束边（flow-ai 不会剪除）
                        graph.edges.push(FlowEdge::mutex(pair.0.clone(), pair.1.clone()));
                        serialized.insert(pair);
                    }
                }
                Constraint::MustIsolate(target) => {
                    if let Some(n) = graph.node_mut(target) {
                        if !n.tags.iter().any(|t| t == "sandboxed") {
                            n.tags.push("sandboxed".into());
                        }
                    }
                }
                Constraint::MustAudit(target) => {
                    if let Some(n) = graph.node_mut(target) {
                        if !n.tags.iter().any(|t| t == "traced" || t == "audited") {
                            n.tags.push("traced".into());
                        }
                    }
                }
                Constraint::ResourceCap(pool, cap) => {
                    if let Some(p) = graph.pools.iter_mut().find(|p| p.name == *pool) {
                        p.capacity = (*cap).min(p.capacity);
                    } else {
                        graph.pools.push(ResourcePool { name: pool.clone(), capacity: *cap });
                    }
                }
                Constraint::Compliance(pid) => {
                    rules.push(ExpertRule {
                        id: pid.clone(),
                        description: format!("合规策略 {}", pid),
                        severity: Severity::Blocking,
                        resource_prefixes: vec![],
                        tool_kinds: vec![],
                        required_guard_tags: vec!["desensitize".into()],
                    });
                }
                Constraint::RouteModel(node, tier) => {
                    model_routes.push((node.clone(), *tier));
                }
            }
        }
    }

    // 合并租户显式配额池
    for tp in tenant_pools {
        if let Some(p) = graph.pools.iter_mut().find(|p| p.name == tp.name) {
            p.capacity = tp.capacity.min(p.capacity);
        } else {
            graph.pools.push(tp.clone());
        }
    }

    let pools = graph.pools.clone();
    ReconciledPlan {
        graph,
        rules,
        pools,
        conflicts,
        model_routes,
        scores,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expert::ExpertOpinion;
    use crate::ir::Dimension;
    use flow_ai::model::{Access, ToolKind};

    fn base() -> FlowGraph {
        let mut g = FlowGraph::new("t", "t");
        g.add_node(FlowNode::task("a", "读库", ToolKind::Database, 300)
            .with_access(Access::read("db:citizen_info"))
            .with_access(Access::write("var:citizen")));
        g.add_node(FlowNode::task("b", "外发", ToolKind::Http, 100)
            .with_access(Access::read("var:citizen")));

        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "a"));
        g.add_edge(FlowEdge::seq("a", "b"));
        g.add_edge(FlowEdge::seq("b", "e"));
        g
    }

    #[test]
    fn permission_guard_injected() {
        let g = base();
        let mut o = ExpertOpinion::empty("permission", Dimension::Permission);
        o.constraints.push(Constraint::MustGuard("a".into(), vec!["desensitize".into()]));
        let plan = reconcile(&[o], &g, &[]);
        assert!(plan.graph.nodes.iter().any(|n| n.kind == NodeKind::Guard));
        // a 现在被 guard 支配
        let gnode = plan.graph.nodes.iter().find(|n| n.kind == NodeKind::Guard).unwrap();
        assert!(plan.graph.edges.iter().any(|e| e.from == gnode.id && e.to == "a"));
    }

    #[test]
    fn mutex_is_hard_not_pruned() {
        let g = base();
        let mut o = ExpertOpinion::empty("resource", Dimension::Resource);
        o.constraints.push(Constraint::MustSerialize(crate::expert::NodeEdge { from: "a".into(), to: "b".into() }));
        let plan = reconcile(&[o], &g, &[]);
        assert!(plan.graph.edges.iter().any(|e| e.kind == EdgeKind::Mutex));
    }

    #[test]
    fn resource_cap_applied() {
        let g = base();
        let mut o = ExpertOpinion::empty("resource", Dimension::Resource);
        o.constraints.push(Constraint::ResourceCap("browser".into(), 1));
        let plan = reconcile(&[o], &g, &[]);
        // 原图无 browser 池 → 新建
        assert!(plan.graph.pools.iter().any(|p| p.name == "browser" && p.capacity == 1));
    }

    #[test]
    fn scores_recorded() {
        let g = base();
        let o = ExpertOpinion::empty("algo", Dimension::Algorithm);
        let plan = reconcile(&[o], &g, &[]);
        assert!(plan.scores.iter().any(|(e, _)| e == "algo"));
    }
}
