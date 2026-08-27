// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_kg_sdk::{Client, ProjectionSpec};

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.spark_seed_nodes(70).await.unwrap(); // types: Person, Item, Event, Org
    g.projection_define(ProjectionSpec {
        name: "persons-out".into(),
        node_labels: vec![],
        edge_labels: vec![],
        attrs_out: vec![],
        attrs_in: vec![],
        min_degree_out: 0,
        community: None,
        type_out: Some("Person".into()),
        type_in: None,
    }).await.unwrap();
    let r = g.projection_run("persons-out").await.unwrap();
    assert_eq!(r.spec_name, "persons-out");
    // 70 nodes / 4 types ≈ 17~18 expected for Person
    assert!(r.node_count > 0 && r.node_count <= 70);
    println!("XJ-OK: graph-015_proj_type_out_1");
}
