// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_kg_sdk::{Client, ProjectionSpec};

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.spark_seed_nodes(140).await.unwrap(); // communities 0..=6
    g.projection_define(ProjectionSpec {
        name: "comm-1".into(),
        community: Some(1),
        ..Default::default()
    }).await.unwrap();
    let r = g.projection_run("comm-1").await.unwrap();
    // 140 nodes / 7 communities = 20 expected
    assert!(r.node_count >= 15 && r.node_count <= 25);
    println!("XJ-OK: graph-017_proj_community_in_1");
}
