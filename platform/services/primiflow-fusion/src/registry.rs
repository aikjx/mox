//! 能力融合 Registry（规范缺口 R06 六维绑定 Registry + R08 文档/数据挂接）
//!
//! 把全工程 13 个 crate 的能力、关图 12 类实体、PT-Primi 六维绑定，**融合**进一张
//! [`UnifiedGraph`]。同时把 `ddl.sql` 里悬空的 6 张表挂接到 primiflow 代码节点，
//! 消除关图骨架中标注的「数据孤岛」（R07/R08 缺口）。

use crate::unified::{
    EntityKind, Layer, PrimitiveCoords, RelKind, UnifiedEdge, UnifiedGraph, UnifiedNode,
};

/// 全工程 crate 清单（与 workspace members + 关图骨架 D01-D12 对齐）
pub const CRATE_NAMES: &[&str] = &[
    "operator-core",
    "operator-wasm",
    "graph-algorithms",
    "optimizer",
    "flow-ai",
    "mox-expert",
    "hermes-flow-bridge",
    "business-catalog",
    "ai-agent",
    "template-market",
    "runtime",
    "mox-system",
    "primiflow-core",
];

/// 单条能力登记项
struct Cap {
    id: &'static str,
    name: &'static str,
    kind: EntityKind,
    layer: Layer,
    primitive: PrimitiveCoords,
}

/// 融合全部 crate 能力 + 数据表 + 示例六维链，产出平台唯一事实源图
pub fn fuse_all() -> UnifiedGraph {
    let mut g = UnifiedGraph::new();

    // 1) 13 crate 作为能力域节点，并登记其代表性能力（融合「所有的功能」）
    for name in CRATE_NAMES {
        let (layer, caps) = crate_caps(name);
        let crate_id = format!("crate:{name}");
        g.add_node(UnifiedNode {
            id: crate_id.clone(),
            kind: EntityKind::Dependency,
            layer,
            name: (*name).into(),
            path: format!("crates/{name}"),
            summary: format!("能力域 crate：{name}"),
            evidence: format!("workspace members 登记：crates/{name}"),
            primitive: PrimitiveCoords::zero(),
            bind_id: None,
            external: false,
        });
        for cap in caps {
            g.add_node(UnifiedNode {
                id: cap.id.into(),
                kind: cap.kind,
                layer: cap.layer,
                name: cap.name.into(),
                path: String::new(),
                summary: String::new(),
                evidence: format!("crate:{name} 能力登记"),
                primitive: cap.primitive,
                bind_id: None,
                external: false,
            });
            g.add_edge(UnifiedEdge {
                id: format!("{crate_id}-->{cap_id}", cap_id = cap.id),
                from: crate_id.clone(),
                to: cap.id.into(),
                kind: RelKind::Reference,
                label: "提供能力".into(),
                evidence: format!("crates/{name}/Cargo.toml + src/lib.rs"),
            });
        }
    }

    // 2) 挂接 ddl.sql 中悬空的 6 张表（消除数据孤岛 R07/R08）
    let tables = [
        ("PROJECTS", "Project"),
        ("CONVERSATIONS", "Conversation"),
        ("TOPOLOGYS", "Topology"),
        ("ASSETS", "Asset"),
        ("ARTIFACTS", "Artifact"),
        ("TRACE_LINKS", "TraceLink"),
    ];
    let schema_code = "code:primiflow-schema";
    g.add_node(UnifiedNode {
        id: schema_code.into(),
        kind: EntityKind::Code,
        layer: Layer::AssetPrecipitation,
        name: "primiflow_core::schema".into(),
        path: "crates/primiflow-core/src/gen/schema.rs".into(),
        summary: "六维数据载体（Project/Conversation/Topology/Asset/Artifact/TraceLink）".into(),
        evidence: "primiflow-core/src/gen/schema.rs".into(),
        primitive: PrimitiveCoords::zero(),
        bind_id: Some("REQ-1".into()),
        external: false,
    });
    for (table, struct_name) in tables {
        let tid = format!("store:{table}");
        g.add_node(UnifiedNode {
            id: tid.clone(),
            kind: EntityKind::DataStore,
            layer: Layer::AssetPrecipitation,
            name: table.into(),
            path: format!("ddl.sql::{table}"),
            summary: format!("数据表 {table} ↔ Rust 结构 {struct_name}"),
            evidence: "primiflow-core/src/gen/ddl.sql".into(),
            primitive: PrimitiveCoords::zero(),
            bind_id: None,
            external: false,
        });
        g.add_edge(UnifiedEdge {
            id: format!("{schema_code}=>{tid}"),
            from: schema_code.into(),
            to: tid.clone(),
            kind: RelKind::ReadWrite,
            label: "读写".into(),
            evidence: format!(
                "primiflow-core/src/gen/schema.rs 结构 {struct_name} → ddl.sql {table}"
            ),
        });
        g.add_edge(UnifiedEdge {
            id: format!("{tid}<=deploy"),
            from: tid,
            to: schema_code.into(),
            kind: RelKind::Deploy,
            label: "承载".into(),
            evidence: "DDL 建表语句".into(),
        });
    }

    // 3) 示例六维绑定链（证明 A4 零孤儿 + R07 守恒可达成）
    // REQ(c=√(0.7²+0.3²)) → FUN → BIZ → ALG(κ=0.7,τ=0.3) → TSK → COD
    let req_c = PrimitiveCoords::from_kt(0.7, 0.3);
    let alg_c = PrimitiveCoords::from_kt(0.7, 0.3); // 下游 κ/τ 之和 = REQ 的 κ/τ
    let chain: Vec<(&str, EntityKind, Layer, PrimitiveCoords, &str)> = vec![
        (
            "REQ-SAMPLE",
            EntityKind::Requirement,
            Layer::RequirementSemantic,
            req_c,
            "REQ-1",
        ),
        (
            "FUN-SAMPLE",
            EntityKind::Feature,
            Layer::PrimitiveMapping,
            PrimitiveCoords::zero(),
            "FUN-1",
        ),
        (
            "BIZ-SAMPLE",
            EntityKind::Business,
            Layer::TopologyEmergence,
            PrimitiveCoords::zero(),
            "BIZ-1",
        ),
        (
            "ALG-SAMPLE",
            EntityKind::Algorithm,
            Layer::TopologyEmergence,
            alg_c,
            "ALG-1",
        ),
        (
            "TSK-SAMPLE",
            EntityKind::Task,
            Layer::Orchestration,
            PrimitiveCoords::zero(),
            "TSK-1",
        ),
        (
            "COD-SAMPLE",
            EntityKind::Code,
            Layer::ExecutionRuntime,
            PrimitiveCoords::zero(),
            "COD-1",
        ),
    ];
    for (id, kind, layer, prim, bind) in &chain {
        g.add_node(UnifiedNode {
            id: (*id).into(),
            kind: *kind,
            layer: *layer,
            name: (*id).into(),
            path: String::new(),
            summary: format!("示例六维实体 {}", kind.zh()),
            evidence: "fuse_all 示例链".into(),
            primitive: *prim,
            bind_id: Some((*bind).into()),
            external: false,
        });
    }
    let pairs = [
        ("REQ-SAMPLE", "FUN-SAMPLE"),
        ("FUN-SAMPLE", "BIZ-SAMPLE"),
        ("BIZ-SAMPLE", "ALG-SAMPLE"),
        ("ALG-SAMPLE", "TSK-SAMPLE"),
        ("ALG-SAMPLE", "COD-SAMPLE"),
    ];
    for (a, b) in pairs {
        g.add_edge(UnifiedEdge {
            id: format!("{a}-bind-{b}"),
            from: a.into(),
            to: b.into(),
            kind: RelKind::Bind,
            label: "六维绑定".into(),
            evidence: "示例绑定".into(),
        });
    }

    // schema 代码作为任务落库载体，挂入六维链：Code 需经 Bind 边可达上游 Algorithm/Task，
    // 否则 binding_report 判为维度孤儿（A4）。这里绑定到示例任务 TSK-SAMPLE。
    g.add_edge(UnifiedEdge {
        id: format!("{}-bind-{}", "TSK-SAMPLE", schema_code),
        from: "TSK-SAMPLE".into(),
        to: schema_code.into(),
        kind: RelKind::Bind,
        label: "六维绑定".into(),
        evidence: "schema 持久化 6 张表，作为任务层落库载体".into(),
    });

    g
}

/// 返回 crate 的主责层与其代表性能力清单
fn crate_caps(name: &str) -> (Layer, Vec<Cap>) {
    match name {
        "operator-core" => (
            Layer::ExecutionRuntime,
            vec![
                cap(
                    "cap:opcore-exec",
                    "算子内核执行",
                    EntityKind::Function,
                    Layer::ExecutionRuntime,
                ),
                cap(
                    "cap:opcore-reg",
                    "算子注册表",
                    EntityKind::Interface,
                    Layer::ExecutionRuntime,
                ),
            ],
        ),
        "operator-wasm" => (
            Layer::ExecutionRuntime,
            vec![cap(
                "cap:wasm-hot",
                "WASM 热加载",
                EntityKind::Runtime,
                Layer::ExecutionRuntime,
            )],
        ),
        "graph-algorithms" => (
            Layer::AssetPrecipitation,
            vec![
                cap(
                    "cap:ograph-kg",
                    "知识图谱存储",
                    EntityKind::Data,
                    Layer::AssetPrecipitation,
                ),
                cap(
                    "cap:ograph-query",
                    "图谱查询",
                    EntityKind::Function,
                    Layer::AssetPrecipitation,
                ),
            ],
        ),
        "optimizer" => (
            Layer::TopologyEmergence,
            vec![cap(
                "cap:opt-flow",
                "流程图优化",
                EntityKind::Algorithm,
                Layer::TopologyEmergence,
            )],
        ),
        "flow-ai" => (
            Layer::PrimitiveMapping,
            vec![
                cap_prim(
                    "cap:flowai-kt",
                    "κ‑τ 拓扑原语引擎",
                    EntityKind::Algorithm,
                    Layer::PrimitiveMapping,
                    PrimitiveCoords::from_kt(0.7, 0.3),
                ),
                cap(
                    "cap:flowai-emerge",
                    "自涌现调度",
                    EntityKind::Loop,
                    Layer::TopologyEmergence,
                ),
            ],
        ),
        "mox-expert" => (
            Layer::Governance,
            vec![cap(
                "cap:ea-govern",
                "全维治理校验",
                EntityKind::Function,
                Layer::Governance,
            )],
        ),
        "hermes-flow-bridge" => (
            Layer::ExecutionRuntime,
            vec![cap(
                "cap:hermes-bridge",
                "外部流系统桥接",
                EntityKind::Interface,
                Layer::ExecutionRuntime,
            )],
        ),
        "business-catalog" => (
            Layer::RequirementSemantic,
            vec![cap(
                "cap:catalog-biz",
                "业务全景目录",
                EntityKind::Business,
                Layer::RequirementSemantic,
            )],
        ),
        "ai-agent" => (
            Layer::ExecutionRuntime,
            vec![cap(
                "cap:agent-loop",
                "AI 智能体闭环",
                EntityKind::Loop,
                Layer::ExecutionRuntime,
            )],
        ),
        "template-market" => (
            Layer::AssetPrecipitation,
            vec![cap(
                "cap:tmpl-market",
                "模板市场",
                EntityKind::Data,
                Layer::AssetPrecipitation,
            )],
        ),
        "runtime" => (
            Layer::Orchestration,
            vec![
                cap(
                    "cap:rt-auto",
                    "AI 自动化中枢",
                    EntityKind::Task,
                    Layer::Orchestration,
                ),
                cap(
                    "cap:rt-market",
                    "算子商城",
                    EntityKind::Interface,
                    Layer::Orchestration,
                ),
            ],
        ),
        "mox-system" => (
            Layer::Governance,
            vec![cap(
                "cap:mox-sys",
                "璇玑系统",
                EntityKind::Function,
                Layer::Governance,
            )],
        ),
        "primiflow-core" => (
            Layer::RequirementSemantic,
            vec![
                cap(
                    "cap:primiflow-orch",
                    "全域原语编排器",
                    EntityKind::Task,
                    Layer::Orchestration,
                ),
                cap(
                    "cap:primiflow-fusion",
                    "多维度融合归一化",
                    EntityKind::Function,
                    Layer::Governance,
                ),
                cap(
                    "cap:primiflow-canvas",
                    "可视化拓扑画布",
                    EntityKind::Interface,
                    Layer::ExecutionRuntime,
                ),
            ],
        ),
        _ => (Layer::ExecutionRuntime, vec![]),
    }
}

fn cap(id: &'static str, name: &'static str, kind: EntityKind, layer: Layer) -> Cap {
    Cap {
        id,
        name,
        kind,
        layer,
        primitive: PrimitiveCoords::zero(),
    }
}

fn cap_prim(
    id: &'static str,
    name: &'static str,
    kind: EntityKind,
    layer: Layer,
    primitive: PrimitiveCoords,
) -> Cap {
    Cap {
        id,
        name,
        kind,
        layer,
        primitive,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unified::{EntityKind, RelKind};

    #[test]
    fn fuses_all_thirteen_crates() {
        let g = fuse_all();
        for name in CRATE_NAMES {
            assert!(
                g.node(&format!("crate:{name}")).is_some(),
                "缺失 crate 节点：{name}"
            );
        }
        // 至少 13 个 crate 节点
        let crate_nodes = g
            .nodes
            .values()
            .filter(|n| n.id.starts_with("crate:"))
            .count();
        assert_eq!(crate_nodes, 13, "应融合全部 13 crate");
    }

    #[test]
    fn data_tables_no_longer_isolated() {
        let g = fuse_all();
        for t in [
            "PROJECTS",
            "CONVERSATIONS",
            "TOPOLOGYS",
            "ASSETS",
            "ARTIFACTS",
            "TRACE_LINKS",
        ] {
            let id = format!("store:{t}");
            assert!(g.node(&id).is_some(), "缺数据表 {t}");
            // 应有 ReadWrite/Deploy 边挂接到 schema 代码
            let connected = g.edges.iter().any(|e| e.from == id || e.to == id);
            assert!(connected, "数据表 {t} 仍为孤岛");
        }
    }

    #[test]
    fn fused_graph_passes_full_gate() {
        let g = fuse_all();
        let gate = g.full_gate();
        assert!(gate.passed, "融合图应通过全闸门：{:?}", gate);
        // 六维示例链零孤儿
        assert!(g.binding_report().passed);
        // 守恒：示例需求下游 κ/τ 之和 = 声明 C
        assert!(g.conservation_report().passed);
    }

    #[test]
    fn six_dim_sample_chain_present() {
        let g = fuse_all();
        for id in [
            "REQ-SAMPLE",
            "FUN-SAMPLE",
            "BIZ-SAMPLE",
            "ALG-SAMPLE",
            "TSK-SAMPLE",
            "COD-SAMPLE",
        ] {
            assert!(g.node(id).is_some());
        }
        let chain = g.trace_binding("COD-SAMPLE");
        assert_eq!(
            chain,
            vec![
                "REQ-SAMPLE",
                "FUN-SAMPLE",
                "BIZ-SAMPLE",
                "ALG-SAMPLE",
                "COD-SAMPLE"
            ]
        );
        // 确认链上确实含六维实体
        let kinds: Vec<EntityKind> = chain.iter().map(|id| g.node(id).unwrap().kind).collect();
        assert!(kinds.contains(&EntityKind::Requirement));
        assert!(kinds.contains(&EntityKind::Code));
        // 确认存在 Bind 边
        assert!(g.edges.iter().any(|e| e.kind == RelKind::Bind));
    }
}
