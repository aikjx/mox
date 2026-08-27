// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_kg_sdk::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    let (nodes, edges) = g.spark_inc_roundtrip(2000, 3000).await.unwrap();
    assert!(nodes >= 2000);
    assert!(edges >= 3000);
    let stats = g.spark_stats().await.unwrap();
    assert_eq!(stats.roundtrips, 1);
    println!("XJ-OK: graph-012_spark_roundtrip_2k_3k");
}
