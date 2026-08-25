use mox_sdk_graph::{Client, ProjectionSpec};

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.spark_seed_nodes(20).await.unwrap();
    // Tag first 5 nodes with email+phone → match attrs_out projection
    for id in 0..5 {
        g.node_set_attrs(id, vec![
            ("email".into(), format!("u{}@x", id)),
            ("phone".into(), format!("+86-100{}", id)),
        ]).await.unwrap();
    }
    g.projection_define(ProjectionSpec {
        name: "reachable-out".into(),
        attrs_out: vec!["email".into(), "phone".into()],
        ..Default::default()
    }).await.unwrap();
    let r = g.projection_run("reachable-out").await.unwrap();
    assert_eq!(r.node_count, 5);
    println!("XJ-OK: graph-019_proj_attr_out");
}
