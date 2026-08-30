// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_ai_flow_svc::topology::{Entity, EntityKind, Relation, RelationKind, TopologyGraph};

/// 跨业务共享的六维关系网：注入 Skill / Rule / Memory / Model 实体与关系
pub fn build_topology() -> TopologyGraph {
    let mut topo = TopologyGraph::new();
    topo.add_entity(
        Entity::new("model:hermes3", EntityKind::Model, "Hermes3 重模型")
            .with_cost(800)
            .with_keywords(["流程图", "代码", "重推理"]),
    );
    topo.add_entity(
        Entity::new("model:light", EntityKind::Model, "轻量模型")
            .with_cost(120)
            .with_keywords(["分类", "意图", "摘要"]),
    );
    for t in [
        "database", "browser", "file", "http", "shell", "llm", "compute", "guard",
    ] {
        topo.add_entity(
            Entity::new(format!("tool:{}", t), EntityKind::Tool, t.to_string()).with_keywords([t]),
        );
    }
    topo.add_entity(
        Entity::new("skill:desensitize", EntityKind::Skill, "脱敏模板")
            .with_keywords(["脱敏", "pii", "政务"])
            .with_cost(50),
    );
    topo.add_entity(
        Entity::new("skill:intent-route", EntityKind::Skill, "意图路由模板")
            .with_keywords(["意图", "分类", "路由", "客服"])
            .with_cost(120),
    );
    topo.add_entity(
        Entity::new("skill:etl-map", EntityKind::Skill, "ETL字段映射模板")
            .with_keywords(["etl", "映射", "抽取"])
            .with_cost(250),
    );
    topo.add_entity(
        Entity::new("skill:db-pull", EntityKind::Skill, "数据库拉取模板")
            .with_keywords(["数据库", "拉取", "对账"])
            .with_cost(300),
    );
    topo.add_entity(
        Entity::new("mem:kb_vec", EntityKind::Memory, "知识库向量")
            .with_keywords(["知识", "检索", "客服"]),
    );
    topo.add_entity(
        Entity::new("rule:pii", EntityKind::Rule, "PII 必须脱敏")
            .with_keywords(["pii", "脱敏", "政务"]),
    );
    topo.add_entity(
        Entity::new("flownode:start", EntityKind::FlowNode, "开始节点")
            .with_keywords(["流程", "节点", "start"]),
    );
    topo.add_entity(
        Entity::new("flownode:end", EntityKind::FlowNode, "结束节点")
            .with_keywords(["流程", "节点", "end"]),
    );
    topo.add_relation(Relation::new(
        "model:hermes3",
        "flow:gov-pii:ic",
        RelationKind::Serves,
        0.9,
    ));
    topo.add_relation(Relation::new(
        "model:light",
        "flow:bot:ic",
        RelationKind::Serves,
        0.95,
    ));
    topo.add_relation(Relation::new(
        "skill:desensitize",
        "flow:gov-pii:guard",
        RelationKind::Implements,
        1.0,
    ));
    topo.add_relation(Relation::new(
        "skill:intent-route",
        "flow:bot:ic",
        RelationKind::Implements,
        1.0,
    ));
    topo.add_relation(Relation::new(
        "skill:etl-map",
        "flow:etl:map",
        RelationKind::Implements,
        1.0,
    ));
    topo.add_relation(Relation::new(
        "skill:db-pull",
        "flow:finance:pull_a",
        RelationKind::Implements,
        1.0,
    ));
    topo.add_relation(Relation::new(
        "skill:intent-route",
        "mem:kb_vec",
        RelationKind::Recalls,
        0.8,
    ));
    topo.add_relation(Relation::new(
        "rule:pii",
        "flow:gov-pii:db",
        RelationKind::Constrains,
        1.0,
    ));
    topo
}
