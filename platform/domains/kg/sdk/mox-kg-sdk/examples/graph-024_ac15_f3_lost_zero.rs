// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_kg_sdk::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    // 50 nodes × 2 zero-valued attrs each = 100 zero count
    let (count, report) = g.ac15_f3_lost_zero(50).await.unwrap();
    assert_eq!(count, 100);
    assert_eq!(report.lost_zero_count, 100);
    assert_eq!(report.fault_tag, "f3");
    println!("XJ-OK: graph-024_ac15_f3_lost_zero");
}
