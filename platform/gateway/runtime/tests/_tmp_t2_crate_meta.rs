//! T2 回归验证（新契约版）：16 个 crate 的 CRATE_ID / ENGINE_NAME / CRATE_META 常量声明 + 唯一性契约 + UUIDv5 格式 + AIS 分层合规。
//!
//! 运行方式（仓库根执行）：
//!   cargo test -p runtime --test _tmp_t2_crate_meta

use std::collections::HashSet;

// 16 个 crate 逐一 extern 引入（用别名 c_ 前缀规避保留字歧义；共 16 = 14 services + runtime + mox-common-meta）
extern crate ai_agent as c_ai_agent;
extern crate business_catalog as c_business_catalog;
extern crate flow_ai as c_flow_ai;
extern crate graph_algorithms as c_graph_algorithms;
extern crate hermes_flow_bridge as c_hermes_flow_bridge;
extern crate kg_hub as c_kg_hub;
extern crate operator_core as c_operator_core;
extern crate operator_wasm as c_operator_wasm;
extern crate optimizer as c_optimizer;
extern crate primiflow_core as c_primiflow_core;
extern crate primiflow_fusion as c_primiflow_fusion;
extern crate runtime as c_runtime;
extern crate template_market as c_template_market;
extern crate mox_common_meta as c_mox_common_meta;
extern crate mox_expert as c_mox_expert;
extern crate mox_system as c_mox_system;

const EXPECTED_COUNT: usize = 16;

/// UUIDv5 格式校验：36 字符、'-' 在位置 8/13/18/23、版本字节 parts[2][0..1] == '5'（UUID v5）
fn is_uuid_v5(s: &str) -> bool {
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
                if !b.is_ascii_hexdigit() {
                    return false;
                }
            }
        }
    }
    // Version: 14th char (0-indexed 14) should be '5'
    // Actually it's parts[2][0..1] = byte at index 14 (positions: 0-7=part1, 8='-', 9-12=part2, 13='-', 14=version '5')
    &s[14..15] == "5"
}

#[test]
fn sixteen_crates_have_valid_crate_id_engine_name_and_crate_meta() {
    // --------- operator-core (L6Kernel) ---------
    {
        assert_eq!(c_operator_core::ENGINE_NAME, "mox::operator_core");
        assert!(
            is_uuid_v5(c_operator_core::CRATE_ID),
            "operator-core CRATE_ID 不是 UUIDv5: {}",
            c_operator_core::CRATE_ID
        );
        let m = &c_operator_core::CRATE_META;
        assert_eq!(m.id, c_operator_core::CRATE_ID);
        assert_eq!(m.name, "operator-core");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(m.layer, mox_common_meta::AisLayer::L6Kernel));
    }
    // --------- operator-wasm (L4Services) ---------
    {
        assert_eq!(c_operator_wasm::ENGINE_NAME, "mox::operator_wasm");
        assert!(is_uuid_v5(c_operator_wasm::CRATE_ID));
        let m = &c_operator_wasm::CRATE_META;
        assert_eq!(m.id, c_operator_wasm::CRATE_ID);
        assert_eq!(m.name, "operator-wasm");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(m.layer, mox_common_meta::AisLayer::L4Services));
    }
    // --------- graph-algorithms (L4Services) ---------
    {
        assert_eq!(c_graph_algorithms::ENGINE_NAME, "mox::graph_algorithms");
        assert!(is_uuid_v5(c_graph_algorithms::CRATE_ID));
        let m = &c_graph_algorithms::CRATE_META;
        assert_eq!(m.id, c_graph_algorithms::CRATE_ID);
        assert_eq!(m.name, "graph-algorithms");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(m.layer, mox_common_meta::AisLayer::L4Services));
    }
    // --------- optimizer (L4Services) ---------
    {
        assert_eq!(c_optimizer::ENGINE_NAME, "mox::optimizer");
        assert!(is_uuid_v5(c_optimizer::CRATE_ID));
        let m = &c_optimizer::CRATE_META;
        assert_eq!(m.id, c_optimizer::CRATE_ID);
        assert_eq!(m.name, "optimizer");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(m.layer, mox_common_meta::AisLayer::L4Services));
    }
    // --------- flow-ai (L4Services) ---------
    {
        assert_eq!(c_flow_ai::ENGINE_NAME, "mox::flow_ai");
        assert!(is_uuid_v5(c_flow_ai::CRATE_ID));
        let m = &c_flow_ai::CRATE_META;
        assert_eq!(m.id, c_flow_ai::CRATE_ID);
        assert_eq!(m.name, "flow-ai");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(m.layer, mox_common_meta::AisLayer::L4Services));
    }
    // --------- mox-expert (L4Services) ---------
    {
        assert_eq!(c_mox_expert::ENGINE_NAME, "mox::mox_expert");
        assert!(is_uuid_v5(c_mox_expert::CRATE_ID));
        let m = &c_mox_expert::CRATE_META;
        assert_eq!(m.id, c_mox_expert::CRATE_ID);
        assert_eq!(m.name, "mox-expert");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(m.layer, mox_common_meta::AisLayer::L4Services));
    }
    // --------- hermes-flow-bridge (L4Services) ---------
    {
        assert_eq!(
            c_hermes_flow_bridge::ENGINE_NAME,
            "mox::hermes_flow_bridge"
        );
        assert!(is_uuid_v5(c_hermes_flow_bridge::CRATE_ID));
        let m = &c_hermes_flow_bridge::CRATE_META;
        assert_eq!(m.id, c_hermes_flow_bridge::CRATE_ID);
        assert_eq!(m.name, "hermes-flow-bridge");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(m.layer, mox_common_meta::AisLayer::L4Services));
    }
    // --------- business-catalog (L4Services) ---------
    {
        assert_eq!(c_business_catalog::ENGINE_NAME, "mox::business_catalog");
        assert!(is_uuid_v5(c_business_catalog::CRATE_ID));
        let m = &c_business_catalog::CRATE_META;
        assert_eq!(m.id, c_business_catalog::CRATE_ID);
        assert_eq!(m.name, "business-catalog");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(m.layer, mox_common_meta::AisLayer::L4Services));
    }
    // --------- ai-agent (L4Services) ---------
    {
        assert_eq!(c_ai_agent::ENGINE_NAME, "mox::ai_agent");
        assert!(is_uuid_v5(c_ai_agent::CRATE_ID));
        let m = &c_ai_agent::CRATE_META;
        assert_eq!(m.id, c_ai_agent::CRATE_ID);
        assert_eq!(m.name, "ai-agent");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(m.layer, mox_common_meta::AisLayer::L4Services));
    }
    // --------- template-market (L4Services) ---------
    {
        assert_eq!(c_template_market::ENGINE_NAME, "mox::template_market");
        assert!(is_uuid_v5(c_template_market::CRATE_ID));
        let m = &c_template_market::CRATE_META;
        assert_eq!(m.id, c_template_market::CRATE_ID);
        assert_eq!(m.name, "template-market");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(m.layer, mox_common_meta::AisLayer::L4Services));
    }
    // --------- runtime (L3Orchestration) ---------
    {
        assert_eq!(c_runtime::ENGINE_NAME, "mox::runtime");
        assert!(is_uuid_v5(c_runtime::CRATE_ID));
        let m = &c_runtime::CRATE_META;
        assert_eq!(m.id, c_runtime::CRATE_ID);
        assert_eq!(m.name, "runtime");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(
            m.layer,
            mox_common_meta::AisLayer::L3Orchestration
        ));
    }
    // --------- mox-system (L7Infrastructure) ---------
    {
        assert_eq!(c_mox_system::ENGINE_NAME, "mox::mox_system");
        assert!(is_uuid_v5(c_mox_system::CRATE_ID));
        let m = &c_mox_system::CRATE_META;
        assert_eq!(m.id, c_mox_system::CRATE_ID);
        assert_eq!(m.name, "mox-system");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(
            m.layer,
            mox_common_meta::AisLayer::L7Infrastructure
        ));
    }
    // --------- primiflow-core (L4Services) ---------
    {
        assert_eq!(c_primiflow_core::ENGINE_NAME, "mox::primiflow_core");
        assert!(is_uuid_v5(c_primiflow_core::CRATE_ID));
        let m = &c_primiflow_core::CRATE_META;
        assert_eq!(m.id, c_primiflow_core::CRATE_ID);
        assert_eq!(m.name, "primiflow-core");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(m.layer, mox_common_meta::AisLayer::L4Services));
    }
    // --------- primiflow-fusion (L4Services) ---------
    {
        assert_eq!(c_primiflow_fusion::ENGINE_NAME, "mox::primiflow_fusion");
        assert!(is_uuid_v5(c_primiflow_fusion::CRATE_ID));
        let m = &c_primiflow_fusion::CRATE_META;
        assert_eq!(m.id, c_primiflow_fusion::CRATE_ID);
        assert_eq!(m.name, "primiflow-fusion");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(m.layer, mox_common_meta::AisLayer::L4Services));
    }
    // --------- kg-hub (L4Services) ---------
    {
        assert_eq!(c_kg_hub::ENGINE_NAME, "mox::kg_hub");
        assert!(is_uuid_v5(c_kg_hub::CRATE_ID));
        let m = &c_kg_hub::CRATE_META;
        assert_eq!(m.id, c_kg_hub::CRATE_ID);
        assert_eq!(m.name, "kg-hub");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(m.layer, mox_common_meta::AisLayer::L4Services));
    }
    // --------- mox-common-meta (L5Domain) ---------
    {
        assert_eq!(
            c_mox_common_meta::ENGINE_NAME,
            "mox::mox_common_meta"
        );
        assert!(is_uuid_v5(c_mox_common_meta::CRATE_ID));
        let m = &c_mox_common_meta::CRATE_META;
        assert_eq!(m.id, c_mox_common_meta::CRATE_ID);
        assert_eq!(m.name, "mox-common-meta");
        assert_eq!(m.owner, "mox-core");
        assert!(matches!(m.layer, mox_common_meta::AisLayer::L5Domain));
    }

    // ---- CRATE_ID 全局唯一性 ----
    let ids: [&str; EXPECTED_COUNT] = [
        c_operator_core::CRATE_ID,
        c_operator_wasm::CRATE_ID,
        c_graph_algorithms::CRATE_ID,
        c_optimizer::CRATE_ID,
        c_flow_ai::CRATE_ID,
        c_mox_expert::CRATE_ID,
        c_hermes_flow_bridge::CRATE_ID,
        c_business_catalog::CRATE_ID,
        c_ai_agent::CRATE_ID,
        c_template_market::CRATE_ID,
        c_runtime::CRATE_ID,
        c_mox_system::CRATE_ID,
        c_primiflow_core::CRATE_ID,
        c_primiflow_fusion::CRATE_ID,
        c_kg_hub::CRATE_ID,
        c_mox_common_meta::CRATE_ID,
    ];
    let set: HashSet<&str> = ids.iter().copied().collect();
    assert_eq!(
        set.len(),
        EXPECTED_COUNT,
        "CRATE_ID 唯一性失败：{}/{} 个唯一值",
        set.len(),
        EXPECTED_COUNT
    );

    // ---- ENGINE_NAME 全局唯一性 ----
    let engines: [&str; EXPECTED_COUNT] = [
        c_operator_core::ENGINE_NAME,
        c_operator_wasm::ENGINE_NAME,
        c_graph_algorithms::ENGINE_NAME,
        c_optimizer::ENGINE_NAME,
        c_flow_ai::ENGINE_NAME,
        c_mox_expert::ENGINE_NAME,
        c_hermes_flow_bridge::ENGINE_NAME,
        c_business_catalog::ENGINE_NAME,
        c_ai_agent::ENGINE_NAME,
        c_template_market::ENGINE_NAME,
        c_runtime::ENGINE_NAME,
        c_mox_system::ENGINE_NAME,
        c_primiflow_core::ENGINE_NAME,
        c_primiflow_fusion::ENGINE_NAME,
        c_kg_hub::ENGINE_NAME,
        c_mox_common_meta::ENGINE_NAME,
    ];
    let eset: HashSet<&str> = engines.iter().copied().collect();
    assert_eq!(
        eset.len(),
        EXPECTED_COUNT,
        "ENGINE_NAME 唯一性失败：{}/{} 个唯一值",
        eset.len(),
        EXPECTED_COUNT
    );

    // ---- L4Services 数量校验：12 个 L4Services ----
    let layers: [mox_common_meta::AisLayer; EXPECTED_COUNT] = [
        c_operator_core::CRATE_META.layer,
        c_operator_wasm::CRATE_META.layer,
        c_graph_algorithms::CRATE_META.layer,
        c_optimizer::CRATE_META.layer,
        c_flow_ai::CRATE_META.layer,
        c_mox_expert::CRATE_META.layer,
        c_hermes_flow_bridge::CRATE_META.layer,
        c_business_catalog::CRATE_META.layer,
        c_ai_agent::CRATE_META.layer,
        c_template_market::CRATE_META.layer,
        c_runtime::CRATE_META.layer,
        c_mox_system::CRATE_META.layer,
        c_primiflow_core::CRATE_META.layer,
        c_primiflow_fusion::CRATE_META.layer,
        c_kg_hub::CRATE_META.layer,
        c_mox_common_meta::CRATE_META.layer,
    ];
    let l4_count = layers
        .iter()
        .filter(|l| matches!(l, mox_common_meta::AisLayer::L4Services))
        .count();
    assert_eq!(
        l4_count, 12,
        "L4Services 应有 12 个（11 L4 + mox-expert），实际 {}",
        l4_count
    );
    let l6_kernel_count = layers
        .iter()
        .filter(|l| matches!(l, mox_common_meta::AisLayer::L6Kernel))
        .count();
    assert_eq!(
        l6_kernel_count, 1,
        "L6Kernel 应有 1 个 (operator-core)，实际 {}",
        l6_kernel_count
    );
    let l7_count = layers
        .iter()
        .filter(|l| matches!(l, mox_common_meta::AisLayer::L7Infrastructure))
        .count();
    assert_eq!(
        l7_count, 1,
        "L7Infrastructure 应有 1 个 (mox-system)，实际 {}",
        l7_count
    );
    let l5_count = layers
        .iter()
        .filter(|l| matches!(l, mox_common_meta::AisLayer::L5Domain))
        .count();
    assert_eq!(
        l5_count, 1,
        "L5Domain 应有 1 个 (mox-common-meta)，实际 {}",
        l5_count
    );
    let l3_count = layers
        .iter()
        .filter(|l| matches!(l, mox_common_meta::AisLayer::L3Orchestration))
        .count();
    assert_eq!(
        l3_count, 1,
        "L3Orchestration 应有 1 个 (runtime)，实际 {}",
        l3_count
    );
}
