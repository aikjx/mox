// tests/lookup.rs - T2 RED step 1: assert all_crate_metas().len() == 16 and 16 ENGINE_NAME lookup
use xuanji_common_meta::{all_crate_metas, lookup_meta_by_engine};

const EXPECTED_ENGINE_NAMES: &[&str] = &[
    "xuanji::ai_agent",
    "xuanji::business_catalog",
    "xuanji::flow_ai",
    "xuanji::graph_algorithms",
    "xuanji::hermes_flow_bridge",
    "xuanji::kg_hub",
    "xuanji::operator_core",
    "xuanji::operator_wasm",
    "xuanji::optimizer",
    "xuanji::primiflow_core",
    "xuanji::primiflow_fusion",
    "xuanji::template_market",
    "xuanji::xuanji_expert",
    "xuanji::xuanji_system",
    "xuanji::runtime",
    "xuanji::xuanji_common_meta",
];

#[test]
fn test_all_crate_metas_len_16() {
    assert_eq!(all_crate_metas().len(), 16, "all_crate_metas must return exactly 16 entries");
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
    assert_eq!(dedup_len, EXPECTED_ENGINE_NAMES.len(), "ENGINE_NAMEs must be all unique");
    for expected in EXPECTED_ENGINE_NAMES {
        let found = lookup_meta_by_engine(expected);
        assert!(found.is_some(), "lookup_meta_by_engine({}) should return Some", expected);
    }
}
