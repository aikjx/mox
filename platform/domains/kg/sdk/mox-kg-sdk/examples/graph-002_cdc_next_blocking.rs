// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_kg_sdk::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.cdc_new_consumer("t", "reader").await.unwrap();
    g.cdc_write_records(5, "row").await.unwrap();
    for i in 0..5 {
        let r = g.cdc_next_blocking("reader").await.unwrap();
        assert_eq!(r.offset, i);
    }
    println!("XJ-OK: graph-002_cdc_next_blocking");
}
