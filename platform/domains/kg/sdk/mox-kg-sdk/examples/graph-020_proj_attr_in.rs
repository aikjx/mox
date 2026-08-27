// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_kg_sdk::{Client, ProjectionSpec};

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.spark_seed_nodes(30).await.unwrap();
    // attrs_in filter — nodes that carry "age" AND "address"
    for id in 10..20 {
        g.node_set_attrs(id, vec![
            ("age".into(), format!("{}", 20 + id)),
            ("address".into(), format!("Addr-{}", id)),
        ]).await.unwrap();
    }
    g.projection_define(ProjectionSpec {
        name: "with-profile".into(),
        attrs_in: vec!["age".into(), "address".into()],
        ..Default::default()
    }).await.unwrap();
    let r = g.projection_run("with-profile").await.unwrap();
    assert_eq!(r.node_count, 10);
    println!("XJ-OK: graph-020_proj_attr_in");
}
