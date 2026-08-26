use mox_kg_sdk::{Client, Node, Edge};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let g = Client::new();
    // a bit of everything: seed + bulk + upsert
    g.spark_seed_nodes(100).await.unwrap();
    g.spark_seed_edges(150).await.unwrap();
    let extra: Vec<Node> = (1000..1200).map(|i| Node {
        id: i, label: "Plus".into(), typ: "X".into(),
        community: 1, attrs: HashMap::new(),
    }).collect();
    g.spark_writer_bulk(extra, vec![] as Vec<Edge>).await.unwrap();
    let s = g.spark_stats().await.unwrap();
    assert_eq!(s.nodes_written, 200);
    assert_eq!(s.edges_written, 150);
    assert!(s.roundtrips == 0);
    println!("XJ-OK: graph-014_spark_stats_accumulate");
}
