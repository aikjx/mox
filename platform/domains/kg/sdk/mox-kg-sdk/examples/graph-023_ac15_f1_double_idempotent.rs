// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_kg_sdk::{Client, Node};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let g = Client::new();
    let batch: Vec<Node> = (0..25).map(|i| Node {
        id: i, label: "L".into(), typ: "T".into(),
        community: 0, attrs: HashMap::new(),
    }).collect();
    let (skips, report) = g.ac15_f1_double_idempotent(batch).await.unwrap();
    assert_eq!(skips, 25); // second pass is all equal = 25 skips
    assert_eq!(report.fault_tag, "f1");
    // report.idempotent_verified: skip_total(25) > 0 and applied_total == skip_total
    // Actually applied_total = first pass applied 25 (applied a=25) + second pass applied 0 = 25
    // skip_total = first pass skip 0 + second pass skip 25 = 25
    // So 25 == 25 → idempotent_verified=true
    assert!(report.idempotent_verified, "not idempotent: report={:?}", report);
    println!("XJ-OK: graph-023_ac15_f1_double_idempotent");
}
