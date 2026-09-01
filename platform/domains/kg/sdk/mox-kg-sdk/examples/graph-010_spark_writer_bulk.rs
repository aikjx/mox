// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_kg_sdk::{Client, Node, Edge};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let g = Client::new();
    let nodes: Vec<Node> = (0..1000).map(|i| Node {
        id: i, label: "Bulk".into(), typ: "Item".into(),
        community: (i % 5), attrs: HashMap::new(),
    }).collect();
    let edges: Vec<Edge> = (0..3000).map(|i| Edge {
        id: i, src: (i % 1000), dst: ((i * 3 + 7) % 1000),
        label: "LINKS".into(), weight: 1.0,
    }).collect();
    let (n, e) = g.spark_writer_bulk(nodes, edges).await.unwrap();
    assert_eq!(n, 1000);
    assert_eq!(e, 3000);
    println!("XJ-OK: graph-010_spark_writer_bulk");
}
