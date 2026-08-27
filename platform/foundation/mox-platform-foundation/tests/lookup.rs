// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

// tests/lookup.rs - T2 RED step 1: assert all_crate_metas().len() == 16 and 16 ENGINE_NAME lookup
use mox_platform_foundation::{all_crate_metas, lookup_meta_by_engine};

const EXPECTED_ENGINE_NAMES: &[&str] = &[
    "mox::ai_agent",
    "mox::business_catalog",
    "mox::mox_ai_flow_svc",
    "mox::graph_algorithms",
    "mox::hermes_flow_bridge",
    "mox::kg_hub",
    "mox::operator_core",
    "mox::operator_wasm",
    "mox::optimizer",
    "mox::primiflow_core",
    "mox::primiflow_fusion",
    "mox::template_market",
    "mox::mox_expert",
    "mox::mox_system",
    "mox::mox_platform_orchestrator_svc",
    "mox::mox_common_meta",
];

#[test]
fn test_all_crate_metas_len_16() {
    assert_eq!(
        all_crate_metas().len(),
        16,
        "all_crate_metas must return exactly 16 entries"
    );
}

#[test]
fn test_all_engine_names_unique_and_lookup() {
    let metas = all_crate_metas();
    let mut engines: Vec<String> = metas.iter().map(|m| m.engine_name()).collect();
    engines.sort();
    let dedup_len = {
        let mut c = engines.clone();
        c.dedup();
        c.len()
    };
    assert_eq!(
        dedup_len,
        EXPECTED_ENGINE_NAMES.len(),
        "ENGINE_NAMEs must be all unique"
    );
    for expected in EXPECTED_ENGINE_NAMES {
        let found = lookup_meta_by_engine(expected);
        assert!(
            found.is_some(),
            "lookup_meta_by_engine({}) should return Some",
            expected
        );
    }
}
