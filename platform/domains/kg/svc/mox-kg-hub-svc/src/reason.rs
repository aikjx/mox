// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 智能推理层：在统一图上做影响面分析、需求溯源与图智能计算。
//!
//! 关键设计——`to_knowledge_graph` 桥接器：
//! `crates/graph-algorithms` 已实现 PageRank / 社区发现 / 激活传播 / 相似推荐，
//! 但它此前只作用于自己那张孤立的 AI 图。本模块把**归一后的全量企业知识图**
//! 投影为 `KnowledgeGraph`，从而让这些算法首次作用于企业全域信息，
//! 而不是重新实现一套算法（避免同能力二次实现造成语义漂移）。

use std::collections::{HashMap, HashSet, VecDeque};

use mox_kg_algo_core::{KnowledgeEdge, KnowledgeGraph, KnowledgeNode};
use mox_flow_fusion_svc::{EntityKind, RelKind, UnifiedGraph};
use serde::{Deserialize, Serialize};

use crate::ontology;

/// 把统一图投影为 graph-algorithms 的 `KnowledgeGraph`，解锁其全部图智能算法。
///
/// 边权按关系强度赋值：六维绑定 > 调用/数据流 > 一般引用。
pub fn to_knowledge_graph(graph: &UnifiedGraph) -> KnowledgeGraph {
    let mut kg = KnowledgeGraph::new();
    for (id, n) in &graph.nodes {
        kg.add_node(KnowledgeNode {
            id: id.clone(),
            label: n.name.clone(),
            node_type: format!("{:?}", n.kind).to_ascii_lowercase(),
            properties: serde_json::json!({
                "layer": n.layer.code(),
                "path": n.path,
                "summary": n.summary,
            }),
            embedding: None,
            activation: 0.0,
            metadata: HashMap::new(),
        });
    }
    for e in &graph.edges {
        // add_edge 在端点缺失时报错；统一图内边端点必然存在，失败则忽略该边而非中断全图
        let _ = kg.add_edge(KnowledgeEdge {
            source: e.from.clone(),
            target: e.to.clone(),
            weight: rel_weight(e.kind),
            relation_type: format!("{:?}", e.kind).to_ascii_lowercase(),
            properties: serde_json::json!({ "evidence": e.evidence }),
        });
    }
    kg
}

/// 关系强度权重
pub fn rel_weight(k: RelKind) -> f64 {
    match k {
        RelKind::Bind => 3.0,
        RelKind::Call | RelKind::DataFlow => 2.0,
        RelKind::ReadWrite | RelKind::Dependency | RelKind::Inheritance => 1.5,
        RelKind::Trigger | RelKind::Deploy | RelKind::Branch | RelKind::LoopBack => 1.2,
        RelKind::Reference | RelKind::ConfigRef => 1.0,
    }
}

/// 影响面分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactReport {
    pub origin: String,
    pub hops: usize,
    /// 受影响节点，按跳数升序
    pub affected: Vec<ImpactNode>,
    pub total: usize,
    /// 按实体类型汇总，便于评估变更风险构成
    pub by_kind: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactNode {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub layer: String,
    pub hop: usize,
    pub path: String,
}

/// 变更影响面：从给定节点沿**出边方向**逐跳扩散，回答"改它会波及谁"。
///
/// 方向性至关重要：影响面必须顺依赖方向传播，不能用无向 BFS，
/// 否则会把"我依赖的库"也算成"被我影响"，严重高估风险面。
pub fn impact(graph: &UnifiedGraph, origin: &str, hops: usize) -> ImpactReport {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in &graph.edges {
        adj.entry(&e.from).or_default().push(&e.to);
    }

    let mut affected: Vec<ImpactNode> = Vec::new();
    let mut by_kind: HashMap<String, usize> = HashMap::new();
    let mut seen: HashSet<&str> = HashSet::new();
    seen.insert(origin);
    let mut q: VecDeque<(&str, usize)> = VecDeque::new();
    q.push_back((origin, 0));

    while let Some((cur, h)) = q.pop_front() {
        if h >= hops {
            continue;
        }
        if let Some(neis) = adj.get(cur) {
            for nb in neis {
                if !seen.insert(nb) {
                    continue;
                }
                if let Some(n) = graph.node(nb) {
                    *by_kind.entry(n.kind.zh().to_string()).or_insert(0) += 1;
                    affected.push(ImpactNode {
                        id: (*nb).to_string(),
                        name: n.name.clone(),
                        kind: n.kind.zh().to_string(),
                        layer: n.layer.code().to_string(),
                        hop: h + 1,
                        path: n.path.clone(),
                    });
                }
                q.push_back((nb, h + 1));
            }
        }
    }

    affected.sort_by(|a, b| a.hop.cmp(&b.hop).then_with(|| a.id.cmp(&b.id)));
    ImpactReport {
        origin: origin.to_string(),
        hops,
        total: affected.len(),
        affected,
        by_kind,
    }
}

/// 需求溯源结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceReport {
    pub target: String,
    /// 命中的需求根（可能多个）
    pub requirements: Vec<String>,
    /// 溯源链（自 target 反向到需求根）
    pub chains: Vec<Vec<String>>,
    pub grounded: bool,
}

/// 需求溯源：从任意实体沿**入边方向**回溯到 `Requirement` 根，
/// 回答"这段代码为什么存在"。`grounded == false` 即为偏离信号（GR-E6）。
pub fn trace_to_requirement(graph: &UnifiedGraph, target: &str) -> TraceReport {
    let mut radj: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in &graph.edges {
        radj.entry(&e.to).or_default().push(&e.from);
    }

    let mut requirements: Vec<String> = Vec::new();
    let mut chains: Vec<Vec<String>> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    seen.insert(target);
    // BFS 同时记录父指针以重建链路
    let mut parent: HashMap<&str, &str> = HashMap::new();
    let mut q: VecDeque<&str> = VecDeque::new();
    q.push_back(target);

    while let Some(cur) = q.pop_front() {
        if let Some(n) = graph.node(cur) {
            if n.kind == EntityKind::Requirement && cur != target {
                requirements.push(cur.to_string());
                // 重建链
                let mut chain = vec![cur.to_string()];
                let mut p = cur;
                while let Some(up) = parent.get(p) {
                    chain.push((*up).to_string());
                    p = up;
                }
                chain.reverse();
                chains.push(chain);
                continue;
            }
        }
        if let Some(ups) = radj.get(cur) {
            for up in ups {
                if seen.insert(up) {
                    parent.insert(up, cur);
                    q.push_back(up);
                }
            }
        }
    }

    requirements.sort();
    requirements.dedup();
    TraceReport {
        grounded: !requirements.is_empty(),
        target: target.to_string(),
        requirements,
        chains,
    }
}

/// 知识热点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hotspot {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub score: f64,
}

/// 知识热点识别：PageRank 复用 graph-algorithms 实现。
/// 高分节点即企业知识的"枢纽"，变更风险最高、最值得优先治理。
pub fn hotspots(graph: &UnifiedGraph, top: usize) -> Vec<Hotspot> {
    let kg = to_knowledge_graph(graph);
    let pr = kg.pagerank(30);
    let mut v: Vec<Hotspot> = pr
        .into_iter()
        .filter_map(|(id, score)| {
            let n = graph.node(&id)?;
            Some(Hotspot {
                id,
                name: n.name.clone(),
                kind: n.kind.zh().to_string(),
                score,
            })
        })
        .collect();
    v.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    v.truncate(top.max(1));
    v
}

/// 孤立知识：无任何关联边的节点——企业知识库中"沉睡"的资产。
pub fn isolated(graph: &UnifiedGraph) -> Vec<String> {
    let mut linked: HashSet<&str> = HashSet::new();
    for e in &graph.edges {
        linked.insert(&e.from);
        linked.insert(&e.to);
    }
    let mut v: Vec<String> = graph
        .nodes
        .keys()
        .filter(|id| !linked.contains(id.as_str()))
        .cloned()
        .collect();
    v.sort();
    v
}

/// 六维完备性：统计每类六维实体的数量，缺维即链路不完整。
pub fn six_dim_coverage(graph: &UnifiedGraph) -> HashMap<String, usize> {
    let mut m: HashMap<String, usize> = HashMap::new();
    for k in ontology::SIX_DIM_ORDER {
        m.insert(k.zh().to_string(), 0);
    }
    for n in graph.nodes.values() {
        if ontology::SIX_DIM_ORDER.contains(&n.kind) {
            *m.entry(n.kind.zh().to_string()).or_insert(0) += 1;
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_flow_fusion_svc::{Layer, PrimitiveCoords, UnifiedEdge, UnifiedNode};

    fn n(id: &str, kind: EntityKind) -> UnifiedNode {
        UnifiedNode {
            id: id.into(),
            kind,
            layer: ontology::default_layer(kind),
            name: id.into(),
            path: id.into(),
            summary: String::new(),
            evidence: id.into(),
            primitive: PrimitiveCoords::zero(),
            bind_id: None,
            external: false,
        }
    }

    fn e(from: &str, to: &str, kind: RelKind) -> UnifiedEdge {
        UnifiedEdge {
            id: format!("{from}->{to}"),
            from: from.into(),
            to: to.into(),
            kind,
            label: String::new(),
            evidence: "t".into(),
        }
    }

    /// REQ → COD → FUNC，另有一个游离的 Doc
    fn sample() -> UnifiedGraph {
        let mut g = UnifiedGraph::new();
        g.add_node(n("REQ", EntityKind::Requirement));
        g.add_node(n("COD", EntityKind::Code));
        g.add_node(n("FUNC", EntityKind::Function));
        g.add_node(n("LIB", EntityKind::Dependency));
        g.add_node(n("ORPHAN", EntityKind::Doc));
        g.add_edge(e("REQ", "COD", RelKind::Bind));
        g.add_edge(e("COD", "FUNC", RelKind::Call));
        g.add_edge(e("COD", "LIB", RelKind::Dependency));
        g
    }

    #[test]
    fn impact_follows_edge_direction_only() {
        let g = sample();
        let r = impact(&g, "COD", 3);
        let ids: HashSet<&str> = r.affected.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains("FUNC"));
        assert!(ids.contains("LIB"));
        // REQ 是 COD 的上游，绝不能出现在影响面里
        assert!(!ids.contains("REQ"), "影响面不得逆依赖方向传播");
        assert_eq!(r.total, 2);
    }

    #[test]
    fn impact_respects_hop_limit() {
        let g = sample();
        let r = impact(&g, "REQ", 1);
        assert_eq!(r.total, 1);
        assert_eq!(r.affected[0].id, "COD");
        let r2 = impact(&g, "REQ", 2);
        assert_eq!(r2.total, 3);
    }

    #[test]
    fn trace_reaches_requirement_root() {
        let g = sample();
        let t = trace_to_requirement(&g, "FUNC");
        assert!(t.grounded);
        assert_eq!(t.requirements, vec!["REQ".to_string()]);
        // 链路应为 FUNC → COD → REQ 的反向重建
        let chain = &t.chains[0];
        assert_eq!(chain.last().unwrap(), "REQ");
        assert!(chain.contains(&"COD".to_string()));
    }

    #[test]
    fn ungrounded_node_is_detected_as_deviation() {
        let g = sample();
        let t = trace_to_requirement(&g, "ORPHAN");
        assert!(!t.grounded, "无需求溯源的节点必须被识别为偏离");
        assert!(t.requirements.is_empty());
    }

    #[test]
    fn isolated_finds_only_unlinked() {
        let g = sample();
        assert_eq!(isolated(&g), vec!["ORPHAN".to_string()]);
    }

    #[test]
    fn bridge_preserves_node_and_edge_counts() {
        let g = sample();
        let kg = to_knowledge_graph(&g);
        assert_eq!(kg.node_count(), g.nodes.len());
        assert_eq!(kg.edge_count(), g.edges.len());
    }

    #[test]
    fn hotspots_rank_hub_node_first() {
        let g = sample();
        let h = hotspots(&g, 3);
        assert!(!h.is_empty());
        // 所有分数非负且降序
        for w in h.windows(2) {
            assert!(w[0].score >= w[1].score, "热点必须降序");
        }
    }

    #[test]
    fn bind_weight_is_strongest() {
        assert!(rel_weight(RelKind::Bind) > rel_weight(RelKind::Call));
        assert!(rel_weight(RelKind::Call) > rel_weight(RelKind::Reference));
    }

    #[test]
    fn six_dim_coverage_counts_all_six_keys() {
        let g = sample();
        let c = six_dim_coverage(&g);
        assert_eq!(c.len(), 6, "六维统计必须始终含 6 个键，缺维显式为 0");
        assert_eq!(c["需求"], 1);
        assert_eq!(c["代码"], 1);
        assert_eq!(c["算法"], 0);
    }

    #[test]
    fn impact_on_unknown_node_is_empty_not_panic() {
        let g = sample();
        let r = impact(&g, "NOPE", 3);
        assert_eq!(r.total, 0);
        let t = trace_to_requirement(&g, "NOPE");
        assert!(!t.grounded);
    }

    #[test]
    fn layers_projected_into_knowledge_graph_properties() {
        let g = sample();
        let kg = to_knowledge_graph(&g);
        let node = kg.get_node("REQ").expect("REQ exists");
        assert_eq!(node.properties["layer"], Layer::RequirementSemantic.code());
    }
}
