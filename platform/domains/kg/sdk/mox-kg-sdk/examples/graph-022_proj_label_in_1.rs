// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_kg_sdk::{Client, ProjectionSpec};

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.spark_seed_nodes(80).await.unwrap();
    // Labels are one of User / Product / Order / Account (20 each)
    g.projection_define(ProjectionSpec {
        name: "accounts-in".into(),
        node_labels: vec!["Account".into()],
        ..Default::default()
    }).await.unwrap();
    let r = g.projection_run("accounts-in").await.unwrap();
    assert_eq!(r.node_count, 20); // exactly one quarter
    // Verify all samples are indeed labeled Account
    let nodes = g.list_nodes().await.unwrap();
    for sid in &r.sample_node_ids {
        let n = nodes.iter().find(|x| &x.id == sid).unwrap();
        assert_eq!(n.label, "Account");
    }
    println!("XJ-OK: graph-022_proj_label_in_1");
}
