// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_kg_sdk::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.spark_seed_nodes(20).await.unwrap();
    g.spark_seed_edges(600).await.unwrap();
    let p1 = g.spark_reader_edges_paged(1, 250).await.unwrap();
    assert_eq!(p1.items.len(), 250);
    assert_eq!(p1.total, 600);
    let p3 = g.spark_reader_edges_paged(3, 250).await.unwrap();
    assert_eq!(p3.items.len(), 100);
    println!("XJ-OK: graph-009_spark_reader_paged_edges");
}
