use mox_sdk_graph::{Client, ProjectionSpec};

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
