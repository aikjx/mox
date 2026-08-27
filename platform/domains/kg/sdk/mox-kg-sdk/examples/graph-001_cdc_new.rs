// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_kg_sdk::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    let cons = g.cdc_new_consumer("graph.nodes", "cons-v1").await.unwrap();
    assert_eq!(cons.topic, "graph.nodes");
    assert_eq!(cons.id, "cons-v1");
    assert_eq!(cons.offset, 0);
    println!("XJ-OK: graph-001_cdc_new");
}
