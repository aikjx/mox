//! T2 回归验证：15 个 crate 的 CRATE_ID + CRATE_META 常量声明 + 唯一性契约。
//!
//! 运行方式（仓库根执行）：
//!   cargo test -p runtime --test _tmp_t2_crate_meta --workspace
//!   # 或直接：cargo test -p runtime --test _tmp_t2_crate_meta
//!
//! 设计说明：runtime 是唯一依赖全部 15 个 workspace crate 的聚合入口，
//! 因此把跨 crate 可见性的一致性测试放在 runtime/tests/ 下即可在
//! 一次集成测试里统一验证。

use std::collections::HashSet;

// 15 个 crate 逐一 extern 引入（用别名 c_ 前缀规避保留字歧义）
extern crate operator_core as c_operator_core;
extern crate operator_wasm as c_operator_wasm;
extern crate graph_algorithms as c_graph_algorithms;
extern crate optimizer as c_optimizer;
extern crate flow_ai as c_flow_ai;
extern crate xuanji_expert as c_xuanji_expert;
extern crate hermes_flow_bridge as c_hermes_flow_bridge;
extern crate business_catalog as c_business_catalog;
extern crate ai_agent as c_ai_agent;
extern crate template_market as c_template_market;
extern crate runtime as c_runtime;
extern crate xuanji_system as c_xuanji_system;
extern crate primiflow_core as c_primiflow_core;
extern crate primiflow_fusion as c_primiflow_fusion;
extern crate kg_hub as c_kg_hub;

const EXPECTED_COUNT: usize = 15;

fn is_uuid_v4(s: &str) -> bool {
    if s.len() != 36 {
        return false;
    }
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        match i {
            8 | 13 | 18 | 23 => {
                if b != b'-' {
                    return false;
                }
            }
            _ => {
                if !matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F') {
                    return false;
                }
            }
        }
    }
    true
}

/// 算法 / 业务类 crate（非基础设施非网关接入），capabilities 必须非空。
fn algorithm_or_business(token: &str) -> bool {
    matches!(
        token,
        "c_operator_core"
            | "c_graph_algorithms"
            | "c_optimizer"
            | "c_flow_ai"
            | "c_xuanji_expert"
            | "c_business_catalog"
            | "c_ai_agent"
            | "c_template_market"
            | "c_primiflow_core"
            | "c_primiflow_fusion"
            | "c_kg_hub"
    )
}

#[test]
fn fifteen_crates_have_valid_crate_id_and_crate_meta() {
    // 15 份结构完全同构但类型独立的断言块（每个 CrateMeta 由各自 crate 自声明）。

    // ---- operator-core ----
    {
        assert_ne!(c_operator_core::CRATE_ID, "", "operator-core CRATE_ID 不能为空");
        let m = &c_operator_core::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "operator-core uuid={} 非 36-char UUID", m.uuid);
        assert!(!m.ais_layers.is_empty(), "operator-core ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "operator-core owner_project={}", m.owner_project);
        assert!(!m.capabilities.is_empty(), "operator-core capabilities 为空");
    }
    // ---- operator-wasm ----
    {
        assert_ne!(c_operator_wasm::CRATE_ID, "", "operator-wasm CRATE_ID 不能为空");
        let m = &c_operator_wasm::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "operator-wasm uuid={} 非 36-char UUID", m.uuid);
        assert!(!m.ais_layers.is_empty(), "operator-wasm ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "operator-wasm owner_project={}", m.owner_project);
    }
    // ---- graph-algorithms ----
    {
        assert_ne!(c_graph_algorithms::CRATE_ID, "", "graph-algorithms CRATE_ID 不能为空");
        let m = &c_graph_algorithms::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "graph-algorithms uuid={}", m.uuid);
        assert!(!m.ais_layers.is_empty(), "graph-algorithms ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "graph-algorithms owner_project={}", m.owner_project);
        assert!(!m.capabilities.is_empty(), "graph-algorithms capabilities 为空");
    }
    // ---- optimizer ----
    {
        assert_ne!(c_optimizer::CRATE_ID, "", "optimizer CRATE_ID 不能为空");
        let m = &c_optimizer::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "optimizer uuid={}", m.uuid);
        assert!(!m.ais_layers.is_empty(), "optimizer ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "optimizer owner_project={}", m.owner_project);
        assert!(!m.capabilities.is_empty(), "optimizer capabilities 为空");
    }
    // ---- flow-ai ----
    {
        assert_ne!(c_flow_ai::CRATE_ID, "", "flow-ai CRATE_ID 不能为空");
        let m = &c_flow_ai::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "flow-ai uuid={}", m.uuid);
        assert!(!m.ais_layers.is_empty(), "flow-ai ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "flow-ai owner_project={}", m.owner_project);
        assert!(!m.capabilities.is_empty(), "flow-ai capabilities 为空");
    }
    // ---- xuanji-expert ----
    {
        assert_ne!(c_xuanji_expert::CRATE_ID, "", "xuanji-expert CRATE_ID 不能为空");
        let m = &c_xuanji_expert::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "xuanji-expert uuid={}", m.uuid);
        assert!(!m.ais_layers.is_empty(), "xuanji-expert ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "xuanji-expert owner_project={}", m.owner_project);
        assert!(!m.capabilities.is_empty(), "xuanji-expert capabilities 为空");
    }
    // ---- hermes-flow-bridge ----
    {
        assert_ne!(c_hermes_flow_bridge::CRATE_ID, "", "hermes-flow-bridge CRATE_ID 不能为空");
        let m = &c_hermes_flow_bridge::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "hermes-flow-bridge uuid={}", m.uuid);
        assert!(!m.ais_layers.is_empty(), "hermes-flow-bridge ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "hermes-flow-bridge owner_project={}", m.owner_project);
    }
    // ---- business-catalog ----
    {
        assert_ne!(c_business_catalog::CRATE_ID, "", "business-catalog CRATE_ID 不能为空");
        let m = &c_business_catalog::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "business-catalog uuid={}", m.uuid);
        assert!(!m.ais_layers.is_empty(), "business-catalog ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "business-catalog owner_project={}", m.owner_project);
        assert!(!m.capabilities.is_empty(), "business-catalog capabilities 为空");
    }
    // ---- ai-agent ----
    {
        assert_ne!(c_ai_agent::CRATE_ID, "", "ai-agent CRATE_ID 不能为空");
        let m = &c_ai_agent::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "ai-agent uuid={}", m.uuid);
        assert!(!m.ais_layers.is_empty(), "ai-agent ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "ai-agent owner_project={}", m.owner_project);
        assert!(!m.capabilities.is_empty(), "ai-agent capabilities 为空");
    }
    // ---- template-market ----
    {
        assert_ne!(c_template_market::CRATE_ID, "", "template-market CRATE_ID 不能为空");
        let m = &c_template_market::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "template-market uuid={}", m.uuid);
        assert!(!m.ais_layers.is_empty(), "template-market ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "template-market owner_project={}", m.owner_project);
        assert!(!m.capabilities.is_empty(), "template-market capabilities 为空");
    }
    // ---- runtime ----
    {
        assert_ne!(c_runtime::CRATE_ID, "", "runtime CRATE_ID 不能为空");
        let m = &c_runtime::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "runtime uuid={}", m.uuid);
        assert!(!m.ais_layers.is_empty(), "runtime ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "runtime owner_project={}", m.owner_project);
    }
    // ---- xuanji-system ----
    {
        assert_ne!(c_xuanji_system::CRATE_ID, "", "xuanji-system CRATE_ID 不能为空");
        let m = &c_xuanji_system::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "xuanji-system uuid={}", m.uuid);
        assert!(!m.ais_layers.is_empty(), "xuanji-system ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "xuanji-system owner_project={}", m.owner_project);
    }
    // ---- primiflow-core ----
    {
        assert_ne!(c_primiflow_core::CRATE_ID, "", "primiflow-core CRATE_ID 不能为空");
        let m = &c_primiflow_core::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "primiflow-core uuid={}", m.uuid);
        assert!(!m.ais_layers.is_empty(), "primiflow-core ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "primiflow-core owner_project={}", m.owner_project);
        assert!(!m.capabilities.is_empty(), "primiflow-core capabilities 为空");
    }
    // ---- primiflow-fusion ----
    {
        assert_ne!(c_primiflow_fusion::CRATE_ID, "", "primiflow-fusion CRATE_ID 不能为空");
        let m = &c_primiflow_fusion::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "primiflow-fusion uuid={}", m.uuid);
        assert!(!m.ais_layers.is_empty(), "primiflow-fusion ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "primiflow-fusion owner_project={}", m.owner_project);
        assert!(!m.capabilities.is_empty(), "primiflow-fusion capabilities 为空");
    }
    // ---- kg-hub ----
    {
        assert_ne!(c_kg_hub::CRATE_ID, "", "kg-hub CRATE_ID 不能为空");
        let m = &c_kg_hub::CRATE_META;
        assert!(is_uuid_v4(m.uuid), "kg-hub uuid={}", m.uuid);
        assert!(!m.ais_layers.is_empty(), "kg-hub ais_layers 为空");
        assert!(m.owner_project.starts_with("proj-"), "kg-hub owner_project={}", m.owner_project);
        assert!(!m.capabilities.is_empty(), "kg-hub capabilities 为空");
    }

    // ---- UUID 唯一性 ----
    let uuids: [&str; EXPECTED_COUNT] = [
        c_operator_core::CRATE_META.uuid,
        c_operator_wasm::CRATE_META.uuid,
        c_graph_algorithms::CRATE_META.uuid,
        c_optimizer::CRATE_META.uuid,
        c_flow_ai::CRATE_META.uuid,
        c_xuanji_expert::CRATE_META.uuid,
        c_hermes_flow_bridge::CRATE_META.uuid,
        c_business_catalog::CRATE_META.uuid,
        c_ai_agent::CRATE_META.uuid,
        c_template_market::CRATE_META.uuid,
        c_runtime::CRATE_META.uuid,
        c_xuanji_system::CRATE_META.uuid,
        c_primiflow_core::CRATE_META.uuid,
        c_primiflow_fusion::CRATE_META.uuid,
        c_kg_hub::CRATE_META.uuid,
    ];
    let set: HashSet<&str> = uuids.iter().copied().collect();
    assert_eq!(
        set.len(),
        EXPECTED_COUNT,
        "CRATE_META.uuid 唯一性失败：{}/{} 个唯一值",
        set.len(),
        EXPECTED_COUNT
    );

    // ---- 算法/业务 crate 的 capabilities 契约校验 ----
    let caps_list: [&[&str]; EXPECTED_COUNT] = [
        c_operator_core::CRATE_META.capabilities,
        c_operator_wasm::CRATE_META.capabilities,
        c_graph_algorithms::CRATE_META.capabilities,
        c_optimizer::CRATE_META.capabilities,
        c_flow_ai::CRATE_META.capabilities,
        c_xuanji_expert::CRATE_META.capabilities,
        c_hermes_flow_bridge::CRATE_META.capabilities,
        c_business_catalog::CRATE_META.capabilities,
        c_ai_agent::CRATE_META.capabilities,
        c_template_market::CRATE_META.capabilities,
        c_runtime::CRATE_META.capabilities,
        c_xuanji_system::CRATE_META.capabilities,
        c_primiflow_core::CRATE_META.capabilities,
        c_primiflow_fusion::CRATE_META.capabilities,
        c_kg_hub::CRATE_META.capabilities,
    ];
    let tokens: [&str; EXPECTED_COUNT] = [
        "c_operator_core",
        "c_operator_wasm",
        "c_graph_algorithms",
        "c_optimizer",
        "c_flow_ai",
        "c_xuanji_expert",
        "c_hermes_flow_bridge",
        "c_business_catalog",
        "c_ai_agent",
        "c_template_market",
        "c_runtime",
        "c_xuanji_system",
        "c_primiflow_core",
        "c_primiflow_fusion",
        "c_kg_hub",
    ];
    for (token, caps) in tokens.iter().zip(caps_list.iter()) {
        if algorithm_or_business(token) {
            assert!(
                !caps.is_empty(),
                "token={} 属于算法/业务类，但 CRATE_META.capabilities 为空（违反 T2 契约）",
                token
            );
        }
    }
}
