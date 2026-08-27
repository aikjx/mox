// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_kg_sdk::{Client, ProjectionSpec};

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.spark_seed_nodes(100).await.unwrap();
    g.projection_define(ProjectionSpec {
        name: "orgs-only".into(),
        type_out: Some("Org".into()),
        ..Default::default()
    }).await.unwrap();
    let r = g.projection_run("orgs-only").await.unwrap();
    assert_eq!(r.spec_name, "orgs-only");
    assert!(r.node_count > 0);
    println!("XJ-OK: graph-016_proj_type_out_2");
}
