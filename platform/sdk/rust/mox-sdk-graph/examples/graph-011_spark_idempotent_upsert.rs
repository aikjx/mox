use mox_sdk_graph::{Client, Node};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let g = Client::new();
    let batch: Vec<Node> = (0..50).map(|i| Node {
        id: i, label: "N".into(), typ: "T".into(),
        community: 0, attrs: HashMap::new(),
    }).collect();
    let (app1, sk1) = g.spark_upsert(batch.clone()).await.unwrap();
    assert_eq!(app1, 50);
    assert_eq!(sk1, 0);
    // Second pass → everything equal, all skipped = idempotent
    let (app2, sk2) = g.spark_upsert(batch).await.unwrap();
    assert_eq!(app2, 0);
    assert_eq!(sk2, 50);
    println!("XJ-OK: graph-011_spark_idempotent_upsert");
}
