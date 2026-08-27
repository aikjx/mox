// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_kg_sdk::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    let (n, e) = g.spark_inc_roundtrip(5000, 8000).await.unwrap();
    assert!(n >= 5000);
    assert!(e >= 8000);
    // list endpoints report the same counts
    let all_nodes = g.list_nodes().await.unwrap();
    assert_eq!(all_nodes.len() as u64, n);
    let all_edges = g.list_edges().await.unwrap();
    assert_eq!(all_edges.len() as u64, e);
    println!("XJ-OK: graph-013_spark_roundtrip_5k_8k");
}
