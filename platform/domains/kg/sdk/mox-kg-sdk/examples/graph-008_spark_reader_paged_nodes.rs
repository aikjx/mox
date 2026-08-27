// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_kg_sdk::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.spark_seed_nodes(120).await.unwrap();
    let page = g.spark_reader_nodes_paged(1, 50).await.unwrap();
    assert_eq!(page.items.len(), 50);
    assert_eq!(page.total, 120);
    assert_eq!(page.page_size, 50);
    let p2 = g.spark_reader_nodes_paged(3, 50).await.unwrap();
    assert_eq!(p2.items.len(), 20); // remainder
    println!("XJ-OK: graph-008_spark_reader_paged_nodes");
}
