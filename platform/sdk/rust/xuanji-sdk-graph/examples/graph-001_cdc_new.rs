use xuanji_sdk_graph::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    let cons = g.cdc_new_consumer("graph.nodes", "cons-v1").await.unwrap();
    assert_eq!(cons.topic, "graph.nodes");
    assert_eq!(cons.id, "cons-v1");
    assert_eq!(cons.offset, 0);
    println!("XJ-OK: graph-001_cdc_new");
}
