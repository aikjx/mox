// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 归一化裁决：把七位专家的观点合并为可喂给 flow-ai 的图
//!
//! 铁律：裁决器只翻译约束为 flow-ai 能识别的边/规则，**不求解**。
//! 硬约束（Blocking / 资源上限）一律落地；冲突按维度优先级仲裁。

use crate::expert::{Constraint, ExpertOpinion};
use crate::ir::Dimension;
use mox_ai_flow_svc::model::{
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
    pub model_routes: Vec<(String, mox_ai_flow_svc::schedule::ModelTier)>,
    /// 各专家健康分
    pub scores: Vec<(String, f64)>,
    /// 采纳的优化建议（经裁决器确认、未与硬约束冲突的 Suggestion）。
    /// 此前 `suggestions` 仅由各专家产出后停留在 `ExpertOpinion` 中，裁决器从不消费，
    /// 导致"并行化/缓存/降档"等优化建议永远无法落到最终计划。此处显式采纳并暴露给流水线。
    pub adopted_suggestions: Vec<crate::expert::Suggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileConflict {
    pub dimension_a: Dimension,
    pub dimension_b: Dimension,
    pub nodes: Vec<String>,
    pub resolution: String,
    pub escalated: bool,
}

impl ReconcileConflict {
    /// 同级维度（优先级相同）约束冲突，无法按优先级仲裁 → 升级为 Blocking
    pub fn escalated_same_priority(
        dimension_a: Dimension,
        dimension_b: Dimension,
        nodes: Vec<String>,
        resolution: String,
    ) -> Self {
        Self {
            dimension_a,
            dimension_b,
            nodes,
            resolution,
            escalated: true,
        }
    }

    /// 语义相反约束（如强制串行 vs 建议并行），记录但交由求解器兜底
    pub fn semantic(
        dimension_a: Dimension,
        dimension_b: Dimension,
        nodes: Vec<String>,
        resolution: String,
    ) -> Self {
        Self {
            dimension_a,
            dimension_b,
            nodes,
            resolution,
            escalated: false,
        }
    }
}

/// 约束类别，用于冲突检测时按"语义"而非"枚举变体"归类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConstraintKind {
    Serialize, // 强制串行（MustSerialize）
    Guard,     // 脱敏/鉴权前置（MustGuard）
    Isolate,   // 沙箱隔离（MustIsolate）
    Other,
}

impl ConstraintKind {
    fn from_constraint(c: &Constraint) -> Self {
        match c {
            Constraint::MustSerialize(_) => ConstraintKind::Serialize,
            Constraint::MustGuard(_, _) => ConstraintKind::Guard,
            Constraint::MustIsolate(_) => ConstraintKind::Isolate,
            _ => ConstraintKind::Other,
        }
    }
}

/// 归一化裁决
pub fn reconcile(
    opinions: &[ExpertOpinion],
    base: &FlowGraph,
    tenant_pools: &[ResourcePool],
) -> ReconciledPlan {
    let mut graph = base.clone();
    let mut rules: Vec<ExpertRule> = Vec::new();
    let mut conflicts: Vec<ReconcileConflict> = Vec::new();
    let mut model_routes: Vec<(String, mox_ai_flow_svc::schedule::ModelTier)> = Vec::new();
    let mut scores: Vec<(String, f64)> = Vec::new();

    // 记录各专家健康分
    for o in opinions {
        scores.push((o.expert.clone(), o.score));
    }

    // —— 冲突预扫描（P2 修复：原 conflicts 永久为空）——
    // 收集每个节点被哪些维度施加了哪类约束，以及全局是否存在并行化建议
    let mut per_node: std::collections::HashMap<String, Vec<(Dimension, ConstraintKind)>> =
        std::collections::HashMap::new();
    let mut has_parallelize_suggestion = false;
    let mut serialize_nodes: Vec<String> = Vec::new();
    // P1：收集专家产出的全部 Suggestion，供裁决器采纳
    let mut all_suggestions: Vec<crate::expert::Suggestion> = Vec::new();
    for o in opinions {
        for c in &o.constraints {
            let kind = ConstraintKind::from_constraint(c);
            let nodes = c.nodes();
            if kind == ConstraintKind::Serialize {
                serialize_nodes.extend(nodes.iter().cloned());
            }
            for n in nodes {
                per_node.entry(n).or_default().push((o.dimension, kind));
            }
        }
        for s in &o.suggestions {
            if matches!(s, crate::expert::Suggestion::Parallelize) {
                has_parallelize_suggestion = true;
            }
            all_suggestions.push(s.clone());
        }
    }
    // 冲突检测：
    //  - escalated（升级 Blocking）：同节点、不同维度、同优先级、**且约束类别相同**（如两维度都要求 MustGuard
    //    却参数互斥）—— 此时无法按优先级仲裁，须人工/安全审批。互补类别（如 MustGuard + MustIsolate）属正交互补，不升级。
    //  - semantic（仅记录溯源）：互补类别共存，或强制串行 vs 并行化建议。
    for (node, dims) in &per_node {
        for i in 0..dims.len() {
            for j in (i + 1)..dims.len() {
                let (da, ka) = dims[i];
                let (db, kb) = dims[j];
                if da != db && da.priority() == db.priority() && ka == kb {
                    conflicts.push(ReconcileConflict::escalated_same_priority(
                        da,
                        db,
                        vec![node.clone()],
                        format!(
                            "维度 {:?} 与 {:?} 优先级相同({})，对同一节点施加同类约束({:?})无法按优先级仲裁，升级为 Blocking 阻断",
                            da, db, da.priority(), ka
                        ),
                    ));
                } else if da != db && ka != kb {
                    // 互补类别（如 Guard + Isolate）共存：记录但不升级
                    conflicts.push(ReconcileConflict::semantic(
                        da,
                        db,
                        vec![node.clone()],
                        format!(
                            "维度 {:?}({:?}) 与 {:?}({:?}) 对同一节点施加互补约束，正交互补，交由求解器合并",
                            da, ka, db, kb
                        ),
                    ));
                }
            }
        }
    }
    // 语义相反：存在强制串行(MustSerialize) 与 全局并行化(Parallelize)建议 → 记录 semantic 冲突
    if has_parallelize_suggestion && !serialize_nodes.is_empty() {
        conflicts.push(ReconcileConflict::semantic(
            Dimension::Resource,
            Dimension::Algorithm,
            serialize_nodes.clone(),
            "检测到强制串行约束与并行化建议语义相反，交由 flow-ai 求解器权衡".to_string(),
        ));
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
                        let mut gnode = FlowNode::new(
                            key.clone(),
                            format!("{} 校验", tags.join("/")),
                            NodeKind::Guard,
                        );
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
                    graph
                        .edges
                        .retain(|e| !(e.to == *target && e.kind != EdgeKind::Exception));
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
                        graph
                            .edges
                            .push(FlowEdge::mutex(pair.0.clone(), pair.1.clone()));
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
                        graph.pools.push(ResourcePool {
                            name: pool.clone(),
                            capacity: *cap,
                        });
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
    // P1：采纳建议。与硬串行约束(MustSerialize)语义相反的 Parallelize 不采纳（已记入 semantic 冲突）；
    // 其余（Cache/Offload/Merge 等）一律采纳，交由流水线落地。
    let adopted_suggestions: Vec<crate::expert::Suggestion> = if serialize_nodes.is_empty() {
        all_suggestions
    } else {
        all_suggestions
            .into_iter()
            .filter(|s| !matches!(s, crate::expert::Suggestion::Parallelize))
            .collect()
    };
    ReconciledPlan {
        graph,
        rules,
        pools,
        conflicts,
        model_routes,
        scores,
        adopted_suggestions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expert::ExpertOpinion;
    use crate::ir::Dimension;
    use mox_ai_flow_svc::model::{Access, ToolKind};

    fn base() -> FlowGraph {
        let mut g = FlowGraph::new("t", "t");
        g.add_node(
            FlowNode::task("a", "读库", ToolKind::Database, 300)
                .with_access(Access::read("db:citizen_info"))
                .with_access(Access::write("var:citizen")),
        );
        g.add_node(
            FlowNode::task("b", "外发", ToolKind::Http, 100)
                .with_access(Access::read("var:citizen")),
        );

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
        o.constraints.push(Constraint::MustGuard(
            "a".into(),
            vec!["desensitize".into()],
        ));
        let plan = reconcile(&[o], &g, &[]);
        assert!(plan.graph.nodes.iter().any(|n| n.kind == NodeKind::Guard));
        // a 现在被 guard 支配
        let gnode = plan
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == NodeKind::Guard)
            .unwrap();
        assert!(plan
            .graph
            .edges
            .iter()
            .any(|e| e.from == gnode.id && e.to == "a"));
    }

    #[test]
    fn mutex_is_hard_not_pruned() {
        let g = base();
        let mut o = ExpertOpinion::empty("resource", Dimension::Resource);
        o.constraints
            .push(Constraint::MustSerialize(crate::expert::NodeEdge {
                from: "a".into(),
                to: "b".into(),
            }));
        let plan = reconcile(&[o], &g, &[]);
        assert!(plan.graph.edges.iter().any(|e| e.kind == EdgeKind::Mutex));
    }

    #[test]
    fn resource_cap_applied() {
        let g = base();
        let mut o = ExpertOpinion::empty("resource", Dimension::Resource);
        o.constraints
            .push(Constraint::ResourceCap("browser".into(), 1));
        let plan = reconcile(&[o], &g, &[]);
        // 原图无 browser 池 → 新建
        assert!(plan
            .graph
            .pools
            .iter()
            .any(|p| p.name == "browser" && p.capacity == 1));
    }

    #[test]
    fn scores_recorded() {
        let g = base();
        let o = ExpertOpinion::empty("algo", Dimension::Algorithm);
        let plan = reconcile(&[o], &g, &[]);
        assert!(plan.scores.iter().any(|(e, _)| e == "algo"));
    }

    #[test]
    fn same_priority_conflict_escalates() {
        // P2 验收：Permission(7) 与 Security(7) 对同一节点施加**同类别**约束（都 MustGuard，参数互斥）
        // → 无法按优先级仲裁 → 升级 Blocking
        let g = base();
        let mut p = ExpertOpinion::empty("permission", Dimension::Permission);
        p.constraints.push(Constraint::MustGuard(
            "a".into(),
            vec!["desensitize".into()],
        ));
        let mut s = ExpertOpinion::empty("security", Dimension::Security);
        // 安全维度对同节点也要求 MustGuard（不同脱敏策略）→ 同类别冲突
        s.constraints
            .push(Constraint::MustGuard("a".into(), vec!["sandbox".into()]));
        let plan = reconcile(&[p, s], &g, &[]);
        assert!(
            plan.conflicts.iter().any(|c| c.escalated
                && c.dimension_a == Dimension::Permission
                && c.dimension_b == Dimension::Security),
            "同级维度同类别约束冲突应升级为 escalated Blocking"
        );
    }

    #[test]
    fn complementary_constraints_not_escalated() {
        // P2 验收：Permission(MustGuard) 与 Security(MustIsolate) 同节点属正交互补 → 记录 semantic，不升级
        let g = base();
        let mut p = ExpertOpinion::empty("permission", Dimension::Permission);
        p.constraints.push(Constraint::MustGuard(
            "a".into(),
            vec!["desensitize".into()],
        ));
        let mut s = ExpertOpinion::empty("security", Dimension::Security);
        s.constraints.push(Constraint::MustIsolate("a".into()));
        let plan = reconcile(&[p, s], &g, &[]);
        assert!(
            !plan.conflicts.iter().any(|c| c.escalated),
            "互补约束不应升级 Blocking"
        );
        assert!(
            plan.conflicts.iter().any(|c| !c.escalated
                && c.dimension_a == Dimension::Permission
                && c.dimension_b == Dimension::Security),
            "互补约束应记录为 semantic 溯源"
        );
    }

    #[test]
    fn serialize_vs_parallelize_recorded() {
        // P2 验收：MustSerialize（强制串行）与 Parallelize 建议语义相反 → 记录 semantic 冲突
        let g = base();
        let mut r = ExpertOpinion::empty("resource", Dimension::Resource);
        r.constraints
            .push(Constraint::MustSerialize(crate::expert::NodeEdge {
                from: "a".into(),
                to: "b".into(),
            }));
        let mut algo = ExpertOpinion::empty("algorithm", Dimension::Algorithm);
        algo.suggestions
            .push(crate::expert::Suggestion::Parallelize);
        let plan = reconcile(&[r, algo], &g, &[]);
        assert!(
            plan.conflicts.iter().any(|c| !c.escalated
                && c.dimension_a == Dimension::Resource
                && c.dimension_b == Dimension::Algorithm),
            "串行 vs 并行应记录为 semantic 冲突"
        );
    }

    #[test]
    fn no_false_conflict_for_distinct_nodes() {
        // 不同节点各自约束，不应误报升级冲突
        let g = base();
        let mut p = ExpertOpinion::empty("permission", Dimension::Permission);
        p.constraints.push(Constraint::MustGuard(
            "a".into(),
            vec!["desensitize".into()],
        ));
        let mut s = ExpertOpinion::empty("security", Dimension::Security);
        s.constraints.push(Constraint::MustIsolate("b".into()));
        let plan = reconcile(&[p, s], &g, &[]);
        assert!(!plan.conflicts.iter().any(|c| c.escalated));
    }

    #[test]
    fn non_conflicting_suggestions_adopted() {
        // P1 验收：与硬约束不冲突的建议（Cache/Merge）应被裁决器采纳进 ReconciledPlan
        let g = base();
        let mut algo = ExpertOpinion::empty("algorithm", Dimension::Algorithm);
        algo.suggestions.push(crate::expert::Suggestion::Cache);
        let mut biz = ExpertOpinion::empty("business", Dimension::Business);
        biz.suggestions.push(crate::expert::Suggestion::Merge);
        let plan = reconcile(&[algo, biz], &g, &[]);
        assert!(
            plan.adopted_suggestions
                .contains(&crate::expert::Suggestion::Cache),
            "Cache 建议应被采纳"
        );
        assert!(
            plan.adopted_suggestions
                .contains(&crate::expert::Suggestion::Merge),
            "Merge 建议应被采纳"
        );
    }

    #[test]
    fn parallelize_not_adopted_when_serialize_conflict() {
        // P1 验收：存在 MustSerialize 硬串行约束时，语义相反的 Parallelize 建议不应被采纳
        let g = base();
        let mut r = ExpertOpinion::empty("resource", Dimension::Resource);
        r.constraints
            .push(Constraint::MustSerialize(crate::expert::NodeEdge {
                from: "a".into(),
                to: "b".into(),
            }));
        let mut algo = ExpertOpinion::empty("algorithm", Dimension::Algorithm);
        algo.suggestions
            .push(crate::expert::Suggestion::Parallelize);
        algo.suggestions.push(crate::expert::Suggestion::Cache);
        let plan = reconcile(&[r, algo], &g, &[]);
        assert!(
            !plan
                .adopted_suggestions
                .iter()
                .any(|s| matches!(s, crate::expert::Suggestion::Parallelize)),
            "与串行约束冲突的 Parallelize 不应被采纳"
        );
        assert!(
            plan.adopted_suggestions
                .contains(&crate::expert::Suggestion::Cache),
            "无冲突的 Cache 仍应被采纳"
        );
    }
}
