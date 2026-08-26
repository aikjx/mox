use mox_kg_sdk::{Client, Node};
use std::collections::HashMap;

fn node(id: i64) -> Option<Node> {
    Some(Node {
        id, label: "P".into(), typ: "Person".into(),
        community: 0, attrs: HashMap::new(),
    })
}

#[tokio::main]
async fn main() {
    let g = Client::new();
    // Batch: 10 valid, 10 None (invalid gaps) → partial writes 10 land
    let mut batch: Vec<Option<Node>> = Vec::new();
    for i in 0..20i64 {
        if i % 2 == 0 {
            batch.push(node(1000 + i));
        } else {
            batch.push(None);
        }
    }
    let (written, report) = g.ac15_f6_partial(batch).await.unwrap();
    assert_eq!(written, 10);
    assert_eq!(report.partial_writes, 10);
    assert_eq!(report.fault_tag, "f6");
    println!("XJ-OK: graph-025_ac15_f6_partial");
}
