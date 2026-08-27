// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_kg_sdk::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.cdc_new_consumer("t", "bulk").await.unwrap();
    let buffered = g.cdc_write_records(100_000, "ev100k").await.unwrap();
    assert!(buffered >= 100_000);
    // Consume first few and advance offset, confirm offset monotonic
    for i in 0..10 {
        let r = g.cdc_next_blocking("bulk").await.unwrap();
        assert_eq!(r.offset, i);
    }
    println!("XJ-OK: graph-004_cdc_100k_via_writer");
}
