// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_kg_sdk::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.cdc_new_consumer("t", "dedup-consumer").await.unwrap();
    let after = g.cdc_dedup_bump("dedup-consumer", 42).await.unwrap();
    assert_eq!(after, 42);
    let after2 = g.cdc_dedup_bump("dedup-consumer", 8).await.unwrap();
    assert_eq!(after2, 50);
    let cons = g.cdc_get_consumer("dedup-consumer").await.unwrap();
    assert_eq!(cons.dedup_count, 50);
    println!("XJ-OK: graph-005_cdc_dedup_stats");
}
