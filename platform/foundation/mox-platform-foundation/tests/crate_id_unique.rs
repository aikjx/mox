// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

// tests/crate_id_unique.rs - T2 Step 3 RED-2: check 16 CRATE_ID are all unique
use std::collections::HashSet;
use mox_platform_foundation::all_crate_metas;

#[test]
fn test_crate_ids_all_unique() {
    let metas = all_crate_metas();
    assert_eq!(metas.len(), 16, "precondition: must have 16 entries");
    let ids: Vec<&str> = metas.iter().map(|m| m.id).collect();
    let unique: HashSet<&str> = ids.iter().copied().collect();
    assert_eq!(
        unique.len(),
        ids.len(),
        "CRATE_IDs must be all unique. Got {} unique / {} total",
        unique.len(),
        ids.len()
    );
}

#[test]
fn test_crate_ids_well_formed_uuid() {
    let metas = all_crate_metas();
    for m in &metas {
        // UUIDv4/v5 format: 8-4-4-4-12 hex chars
        let parts: Vec<&str> = m.id.split('-').collect();
        assert_eq!(
            parts.len(),
            5,
            "id for {} should have 5 dash-separated parts: {}",
            m.name,
            m.id
        );
        assert_eq!(parts[0].len(), 8, "id[0] should be 8 chars for {}", m.name);
        assert_eq!(parts[1].len(), 4, "id[1] should be 4 chars for {}", m.name);
        assert_eq!(parts[2].len(), 4, "id[2] should be 4 chars for {}", m.name);
        assert_eq!(parts[3].len(), 4, "id[3] should be 4 chars for {}", m.name);
        assert_eq!(
            parts[4].len(),
            12,
            "id[4] should be 12 chars for {}",
            m.name
        );
        // v5 marker: parts[2][0..1] should be '5'
        assert_eq!(
            &parts[2][0..1],
            "5",
            "id should be UUIDv5 (version byte 5) for {}",
            m.name
        );
    }
}
