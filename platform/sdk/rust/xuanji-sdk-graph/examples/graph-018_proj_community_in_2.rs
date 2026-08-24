use xuanji_sdk_graph::{Client, ProjectionSpec};

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.spark_seed_nodes(210).await.unwrap();
    g.projection_define(ProjectionSpec {
        name: "comm-2".into(),
        community: Some(2),
        ..Default::default()
    }).await.unwrap();
    let r = g.projection_run("comm-2").await.unwrap();
    assert!(r.node_count >= 25 && r.node_count <= 35, "got {}", r.node_count); // 210/7 = 30
    // sample ids should match community
    let nodes = g.list_nodes().await.unwrap();
    for sid in &r.sample_node_ids {
        let n = nodes.iter().find(|x| &x.id == sid).unwrap();
        assert_eq!(n.community, 2);
    }
    println!("XJ-OK: graph-018_proj_community_in_2");
}
