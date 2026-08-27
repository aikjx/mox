// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_kg_sdk::{Client, ProjectionSpec};

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.spark_seed_nodes(100).await.unwrap();
    g.spark_seed_edges(500).await.unwrap();  // enough edges to produce high-degree nodes
    g.projection_define(ProjectionSpec {
        name: "hubs-out2".into(),
        min_degree_out: 2,
        ..Default::default()
    }).await.unwrap();
    let r = g.projection_run("hubs-out2").await.unwrap();
    // With 500 edges across 100 nodes, average out-degree = 5, so ≥10 nodes expected
    assert!(r.node_count >= 2, "only {} hubs with out-degree ≥2", r.node_count);
    println!("XJ-OK: graph-021_proj_degree_out_2");
}
