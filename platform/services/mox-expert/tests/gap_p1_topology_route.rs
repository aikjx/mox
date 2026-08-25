//! 缺口 P1.4 —— topology route 算法专项测试
//!
//! 目的：针对六维实体关系拓扑网的**路由/最短路径算法**做专项验证：
//!  1) Dijkstra 最短路径在「多跳更优」场景下正确选择低成本路径；
//!  2) 归档（archived）实体被排除在路径之外，算法自动绕行/收敛；
//!  3) `route()` 快路径阈值语义：强命中走快路径，弱命中/无命中回退；
//!  4) `search()` 排序与召回（多技能竞争时返回最优匹配）；
//!  5) `ingest_flow()` 从流程图自动构建 节点↔工具 绑定关系。

use flow_ai::model::{FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};
use flow_ai::schedule::route_models;
use flow_ai::schedule::ModelTier;
use flow_ai::topology::{Entity, EntityKind, Relation, RelationKind, TopologyGraph};

/// 构造确定性代价的小图：所有实体 cost_ms=0（执行开销贡献为 0），
/// 边代价仅由 `base_cost / strength` 决定，便于精确断言 Dijkstra 选路。
fn cost_graph() -> TopologyGraph {
    let mut g = TopologyGraph::new();
    g.add_entity(Entity::new("A", EntityKind::Tool, "A"));
    g.add_entity(Entity::new("B", EntityKind::Tool, "B"));
    g.add_entity(Entity::new("C", EntityKind::Tool, "C"));
    // A→B 弱关联（强度 0.1）→ 代价高（0.5/0.1 = 5.0）
    g.add_relation(Relation::new("A", "B", RelationKind::Implements, 0.1));
    // A→C、C→B 强关联（强度 1.0）→ 各 0.5，合计 1.0 < 5.0
    g.add_relation(Relation::new("A", "C", RelationKind::Implements, 1.0));
    g.add_relation(Relation::new("C", "B", RelationKind::Implements, 1.0));
    g
}

#[test]
fn dijkstra_prefers_cheaper_multi_hop_path() {
    let g = cost_graph();
    let (path, cost) = g.shortest_path("A", "B").expect("应存在路径");
    // 多跳 A→C→B（1.0）优于直连 A→B（5.0）
    assert_eq!(
        path,
        vec!["A".to_string(), "C".to_string(), "B".to_string()]
    );
    assert!(
        (cost - 1.0).abs() < 1e-9,
        "最短路径代价应为 1.0，实际 {}",
        cost
    );
}

#[test]
fn shortest_path_avoids_archived_entity() {
    let mut g = cost_graph();
    // 归档中间节点 C → A→C 边被排除，仅剩直连 A→B
    g.entity_mut("C").unwrap().archived = true;
    let (path, cost) = g.shortest_path("A", "B").expect("直连路径应仍存在");
    assert_eq!(path, vec!["A".to_string(), "B".to_string()]);
    assert!(
        (cost - 5.0).abs() < 1e-9,
        "绕行后代价应回到直连 5.0，实际 {}",
        cost
    );
}

#[test]
fn shortest_path_none_when_target_unreachable() {
    let mut g = cost_graph();
    // 归档 B（目标）→ 无路径
    g.entity_mut("B").unwrap().archived = true;
    assert!(g.shortest_path("A", "B").is_none(), "目标归档后应无路径");
}

/// 复刻 route() 快路径/回退语义（带技能模板）
fn skill_topology() -> TopologyGraph {
    let mut g = TopologyGraph::new();
    g.add_entity(
        Entity::new("skill:report", EntityKind::Skill, "月度报表生成").with_keywords([
            "报表",
            "月度",
            "月度报表",
            "生成",
        ]),
    );
    g.add_entity(Entity::new("flow:n1", EntityKind::FlowNode, "读取Excel").with_cost(300));
    g.add_entity(Entity::new("flow:n2", EntityKind::FlowNode, "汇总输出").with_cost(100));
    g.add_entity(Entity::new("tool:file", EntityKind::Tool, "File"));
    g.add_entity(Entity::new("mem:last", EntityKind::Memory, "上次执行记录"));
    g.add_relation(Relation::new(
        "skill:report",
        "flow:n1",
        RelationKind::Implements,
        1.0,
    ));
    g.add_relation(Relation::new(
        "flow:n1",
        "flow:n2",
        RelationKind::Implements,
        1.0,
    ));
    g.add_relation(Relation::new(
        "flow:n1",
        "tool:file",
        RelationKind::Binds,
        0.9,
    ));
    g.add_relation(Relation::new(
        "skill:report",
        "mem:last",
        RelationKind::Recalls,
        0.7,
    ));
    g
}

#[test]
fn route_takes_fast_path_on_strong_hit() {
    let g = skill_topology();
    let plan = g.route("生成月度报表", 0.1);
    assert!(plan.fast_path, "强命中应走快路径：{}", plan.rationale);
    assert!(plan.path.contains(&"flow:n1".to_string()));
    assert!(plan.entry.as_ref().unwrap().entity_id == "skill:report");
}

#[test]
fn route_falls_back_when_threshold_too_high() {
    let g = skill_topology();
    // 阈值远高于实际得分 → 不算快路径，需完整推理兜底
    let plan = g.route("生成月度报表", 100.0);
    assert!(!plan.fast_path, "超高阈值不应走快路径");
    assert!(plan.entry.is_some());
}

#[test]
fn route_falls_back_when_no_match() {
    let g = skill_topology();
    let plan = g.route("给我讲个冷笑话", 0.1);
    assert!(!plan.fast_path);
    assert!(plan.entry.is_none(), "无匹配实体应回退且 entry 为空");
}

#[test]
fn search_ranks_best_skill_first() {
    let mut g = skill_topology();
    // 增加一个弱相关技能，验证强相关者仍排第一
    g.add_entity(
        Entity::new("skill:news", EntityKind::Skill, "每日新闻摘要")
            .with_keywords(["新闻", "摘要", "每日"]),
    );
    let hits = g.search("帮我生成月度报表", 5);
    assert!(!hits.is_empty());
    assert_eq!(hits[0].entity_id, "skill:report", "最相关技能应排第一");
}

#[test]
fn ingest_flow_builds_node_tool_bindings() {
    let mut f = FlowGraph::new("f1", "测试流程");
    f.add_node(FlowNode::task("a", "浏览器抓取", ToolKind::Browser, 100));
    f.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    f.add_edge(FlowEdge::seq("s", "a"));
    let mut g = TopologyGraph::new();
    g.ingest_flow(&f);
    assert!(g.entity("flow:f1:a").is_some(), "应导入流程节点实体");
    assert!(g.entity("tool:browser").is_some(), "应导入浏览器工具实体");
    assert!(
        g.relations
            .iter()
            .any(|r| r.kind == RelationKind::Binds && r.to == "tool:browser"),
        "应建立 节点→工具 绑定关系"
    );
}

#[test]
fn route_models_assigns_tiers_by_semantics() {
    let mut g = FlowGraph::new("rm", "模型路由");
    g.add_node(FlowNode::task("heavy", "代码生成工程", ToolKind::Llm, 500));
    g.add_node(FlowNode::task("light", "意图分类", ToolKind::Llm, 50));
    g.add_node(FlowNode::task("std", "常规业务推理", ToolKind::Llm, 300));
    let routes = route_models(&g);
    assert_eq!(routes.len(), 3);
    let tier_of = |id: &str| routes.iter().find(|r| r.node_id == id).unwrap().model_tier;
    assert_eq!(tier_of("heavy"), ModelTier::Heavy, "代码生成应为重型");
    assert_eq!(tier_of("light"), ModelTier::Light, "短类任务应为轻量");
    assert_eq!(tier_of("std"), ModelTier::Standard, "常规推理应为标准");
}
