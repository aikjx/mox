//! 草莓多平台 · 端到端集成测试
//!
//! 验证完整链路（库层面，不依赖未完工的 runtime HTTP 层）：
//!   对话需求 → ai-agent 编译蓝图 → flow-ai 生成后端+DB+前端代码
//!            → template-market 落盘为可复用系统模板 → 重新加载
//!
//! 这是"草莓多"核心价值的实证：一句话即可生成一个完整系统的蓝图与全栈代码骨架，
//! 并能作为"系统模板"上传到市场、供他人引用下载复用。

use ai_agent::flow_engine::NodeType;
use ai_agent::requirement_compiler::{RequirementCompiler, SystemBlueprint};
use flow_ai::codegen::{generate, generate_full_stack};
use flow_ai::model::{Access, FlowEdge, FlowGraph, FlowNode, NodeKind};
use flow_ai::{conflict, dataflow, schedule};
use template_market::{Domain, SystemTemplate, TemplateMarket};
use serde_json::json;

/// ai-agent 的 NodeType → flow-ai 的 NodeKind 映射（两库枚举独立，需显式转换）
fn map_node_kind(t: &NodeType) -> NodeKind {
    match t {
        NodeType::Start => NodeKind::Start,
        NodeType::End => NodeKind::End,
        NodeType::LLM => NodeKind::Task,
        NodeType::Browser => NodeKind::Task,
        NodeType::HttpRequest => NodeKind::Task,
        NodeType::Operator => NodeKind::Task,
        NodeType::Condition => NodeKind::Decision,
        NodeType::Transform => NodeKind::Guard,
        NodeType::Script => NodeKind::Task,
        NodeType::DataInput => NodeKind::Task,
        NodeType::DataOutput => NodeKind::Task,
        NodeType::Parallel => NodeKind::ParallelFork,
        NodeType::Task => NodeKind::Task,
        NodeType::Guard => NodeKind::Guard,
        NodeType::Decision => NodeKind::Decision,
        NodeType::Event => NodeKind::Task,
    }
}

/// 把对话蓝图转成 flow-ai 可用的 FlowGraph（复用蓝图里已生成的 flow 定义）
fn blueprint_to_flowgraph(bp: &SystemBlueprint) -> FlowGraph {
    let mut g = FlowGraph::new(&bp.id, &bp.name);
    for n in &bp.flow.nodes {
        let mut node = FlowNode::new(&n.id, &n.name, map_node_kind(&n.node_type));
        // 每个功能节点对其涉及的实体产生 db: 写访问 → 驱动 DDL 生成
        if let Some(obj) = n.config.as_object() {
            if let Some(serde_json::Value::Array(items)) = obj.get("entities") {
                for item in items {
                    if let Some(s) = item.as_str() {
                        let table = s.to_lowercase();
                        node = node.with_access(Access::write(&format!("db:{}.id", table)));
                    }
                }
            }
        }
        if n.node_type == ai_agent::flow_engine::NodeType::Start
            || n.node_type == ai_agent::flow_engine::NodeType::End
        {
            // 起点/终点不加库访问
        }
        g.add_node(node);
    }
    for e in &bp.flow.edges {
        g.add_edge(FlowEdge::seq(&e.source, &e.target));
    }
    g
}

#[tokio::test]
async fn e2e_dialogue_to_fullstack_code_and_market_template() {
    // 1) 对话需求 → 系统蓝图
    let mut rc = RequirementCompiler::new();
    let bp = rc
        .compile(
            "我要做一个商城：有商品，购物车，下单，支付",
            "草莓多商城",
            vec![Domain::Mall.as_str()],
        )
        .unwrap();
    assert!(bp.features.len() >= 4, "应抽取出 ≥4 个功能点");
    assert!(bp.flow.nodes.len() >= 6, "流程图应含 Start + 功能 + End");

    // 2) 蓝图 → flow-ai FlowGraph → 全栈代码（后端 + DB + 前端）
    let g = blueprint_to_flowgraph(&bp);
    let plan = dataflow::analyze(&g);
    let sc = schedule::schedule(&g, &plan.dependencies);
    let cf = conflict::detect(&g, &plan.layers);
    let bundle = generate_full_stack(&g, &plan, &sc, &cf);
    assert!(!bundle.rejected, "生成不应被否决: {:?}", bundle.reject_reasons);

    let files: Vec<&String> = bundle.files.iter().map(|f| &f.path).collect();
    // 后端骨架
    assert!(files.iter().any(|p| p.ends_with("main.py")), "缺少后端入口");
    // 数据库 DDL（草莓多新增）
    let schema = bundle.file("generated/schema.sql").expect("缺 schema.sql");
    assert!(schema.content.contains("CREATE TABLE IF NOT EXISTS"), "缺建表语句");
    // 前端 Vue（草莓多新增）
    let vue = bundle.file("generated/App.vue").expect("缺 App.vue");
    assert!(vue.content.contains("<template>"), "缺前端模板");

    // generate 与 generate_full_stack 等价
    let bundle2 = generate(&g, &plan, &sc, &cf);
    assert_eq!(bundle.files.len(), bundle2.files.len());

    // 3) 把蓝图 + 代码包落盘为"系统模板"上传到市场
    let mut artifacts = std::collections::BTreeMap::new();
    for f in &bundle.files {
        artifacts.insert(f.path.clone(), f.content.clone());
    }
    let tpl = SystemTemplate::new(
        &bp.name,
        &bp.description,
        vec![Domain::Mall],
        json!({ "features": bp.features.len(), "entities": bp.entities.keys().collect::<Vec<_>>() }),
    )
    .with_artifacts(artifacts);

    let dir = std::env::temp_dir().join(format!("caomei_e2e_{}", uuid::Uuid::new_v4()));
    let market = TemplateMarket::open(&dir).unwrap();
    market.publish(&tpl).unwrap();

    // 4) 从市场重新加载（"引用下载"），验证可复用
    let loaded = market.load(&tpl.id).unwrap();
    assert_eq!(loaded.name, "草莓多商城");
    assert!(loaded.artifacts.contains_key("generated/schema.sql"));
    assert!(loaded.artifacts.contains_key("generated/App.vue"));
    assert_eq!(loaded.reuse_count, 1, "加载即视为一次复用");

    // 5) 派生模板（"引用下载后快速开发"）：基于商城模板二开一个"生鲜商城"
    let forked = loaded.fork("生鲜商城", "在商城基础上增加冷链物流");
    assert_eq!(forked.derived_from.as_deref(), Some(tpl.id.as_str()));
    assert!(forked.artifacts.contains_key("generated/App.vue"));
    market.publish(&forked).unwrap();

    // 6) 市场按域检索应包含两个商城模板
    let malls = market.list(Some(&Domain::Mall), None).unwrap();
    assert_eq!(malls.len(), 2, "市场应含 2 个商城模板");

    // 7) 持续学习：评分沉淀
    market.rate(&forked.id, 5.0).unwrap();
    let ranked = market.ranked(Some(&Domain::Mall)).unwrap();
    assert_eq!(ranked[0].id, forked.id, "高评分模板应排前");

    // 清理
    let _ = std::fs::remove_dir_all(&dir);
}
